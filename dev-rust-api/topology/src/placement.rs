use rand::seq::SliceRandom;
use types::PackedPosition;

/// Local helper to distribute `layer_budget` neurons across neuron types proportionally to their share.
///
/// Ensures the total allocated slots matches `layer_budget` exactly by using a cumulative/rounding approach
/// and assigning any remaining slots to the last type variant in the composition.
fn build_type_pool(composition: &[config::NeuronTypeDistribution], layer_budget: usize) -> Vec<u8> {
    if composition.is_empty() || layer_budget == 0 {
        return Vec::new();
    }
    
    let mut pool = Vec::with_capacity(layer_budget);
    let mut allocated = 0;
    let mut cum_share = 0.0;

    for (i, dist) in composition.iter().enumerate() {
        if i == composition.len() - 1 {
            let count = layer_budget.saturating_sub(allocated);
            for _ in 0..count {
                pool.push(i as u8);
            }
        } else {
            cum_share += dist.share;
            let target_cum = (layer_budget as f32 * cum_share).round() as usize;
            let count = target_cum.saturating_sub(allocated);
            for _ in 0..count {
                pool.push(i as u8);
            }
            allocated += count;
        }
    }
    pool
}

/// Stochastic placement of neuron somas within a 3D voxel grid.
///
/// # Invariants
/// - **INV-TOPO-001**: Voxel density constraint.
///   A single voxel in the 3D grid can contain at most one neuron soma. This allows using
///   voxel coordinates as an O(1) unique identifier for fast spatial lookup and collision-free hashing.
///
/// # Arguments
/// * `bounds` - Physical dimension bounds of the shard (max_x, max_y, max_z) where max_z is the height.
/// * `anatomy` - Anatomical layer structures and compositions.
/// * `rng` - Deterministic ChaCha8 random number generator.
///
/// # Errors
/// Returns `TopologyError::PlacementCollision` if the requested neuron budget for a layer exceeds
/// the physical volume (number of voxels) of that layer.
pub fn place_somas(
    bounds: (u32, u32, u32),
    layers: &[config::LayerConfig],
    rng: &mut rand_chacha::ChaCha8Rng,
) -> Result<Vec<types::PackedPosition>, crate::error::TopologyError> {
    let (max_x, max_y, max_z) = bounds;
    let mut positions = Vec::new();
    let mut current_z_pct = 0.0;

    for layer in layers {
        // Calculate physical layer boundaries along the Z axis avoiding floating-point accumulation drift
        let z_start = (current_z_pct * max_z as f32).floor() as u32;
        let z_end = ((current_z_pct + layer.height_pct) * max_z as f32).floor() as u32;
        current_z_pct += layer.height_pct;

        let layer_height = (z_end - z_start).max(1);
        let layer_volume = max_x as u64 * max_y as u64 * layer_height as u64;

        // Bottom-up density allocation budget
        let layer_budget = (layer_volume as f32 * layer.density).floor() as usize;
        if layer_budget == 0 {
            continue;
        }
        if layer_budget > layer_volume as usize {
            return Err(crate::error::TopologyError::PlacementCollision {
                density: layer.density,
                layer: layer.name.clone(),
            });
        }

        // Generate deterministic voxel pool for this layer and shuffle
        let mut pool: Vec<u32> = (0..layer_volume as u32).collect();
        pool.shuffle(rng);

        // Build neuron type index allocation pool
        let type_pool = build_type_pool(&layer.composition, layer_budget);

        // Unpack flat shuffled index to lx, ly, lz voxel coordinates and pack into PackedPosition
        for type_id in type_pool {
            let flat_idx = pool.pop().unwrap();

            let lz = z_start + (flat_idx / (max_x * max_y));
            let rem = flat_idx % (max_x * max_y);
            let ly = rem / max_x;
            let lx = rem % max_x;

            positions.push(PackedPosition::pack_raw(lx, ly, lz, type_id));
        }
    }

    // Z-Sort: Sort positions by Z coordinate to optimize L2 cache locality during spatial queries
    positions.sort_by_key(|p| p.z());
    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{LayerConfig, NeuronTypeDistribution};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn make_test_layers() -> Vec<LayerConfig> {
        vec![
            LayerConfig {
                name: "L1".to_string(),
                height_pct: 0.2,
                density: 0.1,
                composition: vec![
                    NeuronTypeDistribution {
                        type_name: "L1_Exc".to_string(),
                        share: 0.8,
                    },
                    NeuronTypeDistribution {
                        type_name: "L1_Inh".to_string(),
                        share: 0.2,
                    },
                ],
            },
            LayerConfig {
                name: "L2".to_string(),
                height_pct: 0.8,
                density: 0.05,
                composition: vec![
                    NeuronTypeDistribution {
                        type_name: "L2_Exc".to_string(),
                        share: 1.0,
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_build_type_pool_proportions() {
        let composition = vec![
            NeuronTypeDistribution {
                type_name: "T0".to_string(),
                share: 0.6,
            },
            NeuronTypeDistribution {
                type_name: "T1".to_string(),
                share: 0.4,
            },
        ];
        
        let pool = build_type_pool(&composition, 10);
        let count_0 = pool.iter().filter(|&&x| x == 0).count();
        let count_1 = pool.iter().filter(|&&x| x == 1).count();
        
        assert_eq!(pool.len(), 10);
        assert_eq!(count_0, 6);
        assert_eq!(count_1, 4);
    }

    #[test]
    fn test_stochastic_placement_density_and_z_sort() {
        let layers = make_test_layers();
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        
        // Bounds of grid (x, y, z)
        let bounds = (10, 10, 100);
        let positions = place_somas(bounds, &layers, &mut rng).unwrap();
        
        // Verify Z-sorting
        for i in 1..positions.len() {
            assert!(positions[i].z() >= positions[i - 1].z());
        }

        // Verify INV-TOPO-001: No duplicate voxel coordinates
        let mut voxel_set = std::collections::HashSet::new();
        for pos in &positions {
            let key = (pos.x(), pos.y(), pos.z());
            assert!(voxel_set.insert(key), "INV-TOPO-001 violated: Duplicate voxel at {:?}", key);
        }
    }

    #[test]
    fn test_placement_collision_error() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let bounds = (10, 10, 10);
        
        // High density that will exceed volume
        let layers = vec![
            LayerConfig {
                name: "Overdense".to_string(),
                height_pct: 1.0,
                density: 1.5, // 150% density is impossible
                composition: vec![
                    NeuronTypeDistribution {
                        type_name: "Exc".to_string(),
                        share: 1.0,
                    },
                ],
            },
        ];

        let result = place_somas(bounds, &layers, &mut rng);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::TopologyError::PlacementCollision { density, layer } => {
                assert_eq!(layer, "Overdense");
                assert_eq!(density, 1.5);
            }
            other => panic!("Expected PlacementCollision, got {:?}", other),
        }
    }
}
