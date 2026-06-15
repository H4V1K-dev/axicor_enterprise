use types::{PackedPosition, PackedTarget};
use crate::types::{AxonSegment, SpatialGrid, NewSynapse};

impl SpatialGrid {
    /// Builds and initializes the spatial grid by sorting segments.
    pub fn build(mut segments: Vec<AxonSegment>) -> Self {
        segments.sort_by_key(|s| s.pos);
        Self { segments }
    }

    /// Queries the spatial grid for segments within the specified voxel radius.
    ///
    /// # Arguments
    /// * `center` - Center coordinate of the query.
    /// * `radius` - Maximum Manhattan distance search radius in voxels.
    /// * `callback` - Callback function invoked for each matching segment.
    pub fn find_in_radius<F>(&self, center: PackedPosition, radius: i32, mut callback: F)
    where
        F: FnMut(&AxonSegment),
    {
        let cx = center.x() as i32;
        let cy = center.y() as i32;
        let cz = center.z() as i32;

        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    let lx = cx + dx;
                    let ly = cy + dy;
                    let lz = cz + dz;

                    // Bound checks for voxel dimensions: X/Y inside 0..1024, Z inside 0..256
                    if lx >= 0 && lx < 1024 && ly >= 0 && ly < 1024 && lz >= 0 && lz < 256 {
                        // Binary search for all 16 potential neuron type masks
                        for t in 0..16 {
                            let key = PackedPosition::pack_raw(lx as u32, ly as u32, lz as u32, t as u8).0;
                            if let Ok(idx) = self.segments.binary_search_by_key(&key, |s| s.pos) {
                                // Scan left to find the first matching element in the sorted slice
                                let mut left = idx;
                                while left > 0 && self.segments[left - 1].pos == key {
                                    left -= 1;
                                }
                                // Scan right and execute callback for all matches
                                let mut right = left;
                                while right < self.segments.len() && self.segments[right].pos == key {
                                    callback(&self.segments[right]);
                                    right += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Finds the first empty dendrite slot for a given soma.
///
/// Uses SoA mapping format: `soma_idx + slot * padded_n`.
/// Empty targets are represented by `0` or `0xFFFF_FFFF`.
fn find_first_empty_slot(soma_idx: usize, existing_targets: &[u32], padded_n: usize) -> Option<usize> {
    for slot in 0..layout::MAX_DENDRITES {
        let idx = soma_idx + slot * padded_n;
        if idx < existing_targets.len() {
            let val = existing_targets[idx];
            if val == 0 || val == types::EMPTY_PIXEL {
                return Some(slot);
            }
        }
    }
    None
}

/// Checks if a connection to the specified `axon_id` already exists for the given soma.
fn is_duplicate_axon(axon_id: u32, soma_idx: usize, existing_targets: &[u32], padded_n: usize) -> bool {
    for slot in 0..layout::MAX_DENDRITES {
        let idx = soma_idx + slot * padded_n;
        if idx < existing_targets.len() {
            let val = existing_targets[idx];
            if val != 0 && val != types::EMPTY_PIXEL {
                let target = PackedTarget(val);
                if target.axon_id() == axon_id {
                    return true;
                }
            }
        }
    }
    false
}

/// Sprouts new synapses for active somas during Night Phase consolidation.
///
/// # Invariants
/// - **INV-TOPO-004**: Memory Density.
///   Cycles through dendrite slots sequentially, placing new targets in the first empty
///   slot to avoid gaps that would break early exit optimizations in VRAM processing.
/// - **INV-TOPO-005**: DoA Protection.
///   Ensures that newly created synapses do not have initial weights below the shifted
///   pruning threshold by assigning a recovery capital if necessary.
/// - **INV-TOPO-006**: Dale's Law.
///   Initial weight sign is determined strictly by whether the source axon is inhibitory
///   rather than the target dendrite type.
/// - **INV-TOPO-007**: Unique Connections.
///   A soma can form at most one connection to any unique `axon_id` to maximize synaptic target diversity.
pub fn sprout_connections(
    active_somas: &[usize],
    existing_targets: &[u32],
    padded_n: usize,
    grid: &SpatialGrid,
    blueprints: &config::BlueprintsConfig,
    prune_threshold: i16,
    soma_positions: &[types::PackedPosition],
) -> Vec<NewSynapse> {
    let mut new_synapses = Vec::with_capacity(active_somas.len());

    for &soma_idx in active_somas {
        // INV-TOPO-004: Find first empty slot in SoA layout
        let slot_idx = match find_first_empty_slot(soma_idx, existing_targets, padded_n) {
            Some(slot) => slot,
            None => continue, // Limit of 128 connections exceeded, cancel sprouting
        };

        if soma_idx >= soma_positions.len() {
            continue;
        }
        let my_pos = soma_positions[soma_idx];

        let mut best_candidate = None;
        let mut best_dist_sq = i32::MAX;

        // Search within Manhattan distance of 1 (27 neighboring voxels)
        grid.find_in_radius(my_pos, 1, |segment| {
            // Self-connection guard (cannot connect to own axon)
            if segment.axon_id == soma_idx as u32 {
                return;
            }

            // INV-TOPO-007: Rule of Uniqueness
            if is_duplicate_axon(segment.axon_id, soma_idx, existing_targets, padded_n) {
                return;
            }

            let seg_pos = PackedPosition(segment.pos);
            let dx = my_pos.x() as i32 - seg_pos.x() as i32;
            let dy = my_pos.y() as i32 - seg_pos.y() as i32;
            let dz = my_pos.z() as i32 - seg_pos.z() as i32;
            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_candidate = Some(*segment);
            }
        });

        if let Some(candidate) = best_candidate {
            if candidate.type_idx >= blueprints.neuron_types.len() {
                continue;
            }
            let target_type_cfg = &blueprints.neuron_types[candidate.type_idx];

            // INV-TOPO-005: DoA (Dead on Arrival) Protection
            // Read initial synapse weight (derived from gsop_potentiation) and shift by 16 bits
            let initial_synapse_weight = target_type_cfg.gsop.gsop_potentiation;
            let mut start_w = (initial_synapse_weight as i32) << 16;
            let prune_i32 = (prune_threshold as i32) << 16;

            if start_w <= prune_i32 {
                start_w = prune_i32 + (100 << 16);
            }

            // INV-TOPO-006: Dale's Law (Sign dictated strictly by the source axon type)
            let sign = if target_type_cfg.gsop.is_inhibitory { -1 } else { 1 };
            let final_weight = start_w * sign;

            let target_packed = PackedTarget::pack(candidate.axon_id, 0).0;

            new_synapses.push(NewSynapse {
                soma_idx,
                slot_idx,
                target_packed,
                weight: final_weight,
            });
        }
    }

    new_synapses
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{BlueprintsConfig, NeuronType, GsopParams};
    
    // Minimal mock for GsopParams
    fn mock_gsop(is_inhibitory: bool, potentiation: u16) -> GsopParams {
        GsopParams {
            gsop_potentiation: potentiation,
            gsop_depression: 5,
            is_inhibitory,
            inertia_curve: vec![0; 8],
        }
    }

    fn mock_neuron_type(name: &str, is_inhibitory: bool, potentiation: u16) -> NeuronType {
        // We use dummy values for unrelated fields
        let membrane = config::MembraneParams {
            threshold: 20000,
            rest_potential: -70000,
            leak_shift: 4,
        };
        let timings = config::TimingParams {
            refractory_period: 5,
            synapse_refractory_period: 10,
        };
        let signal = config::SignalParams {
            signal_propagation_length: 8,
        };
        let homeostasis = config::HomeostasisParams {
            homeostasis_penalty: 1500,
            homeostasis_decay: 990,
        };
        let adaptive_leak = config::AdaptiveLeakParams {
            adaptive_leak_min_shift: -5,
            adaptive_leak_gain: 2,
            adaptive_mode: 1,
        };
        let dopamine = config::DopamineParams {
            d1_affinity: 80,
            d2_affinity: 20,
        };
        let spontaneous = config::SpontaneousParams {
            spontaneous_firing_period_ticks: 10000,
        };

        NeuronType {
            name: name.to_string(),
            membrane,
            timings,
            signal,
            homeostasis,
            adaptive_leak,
            dopamine,
            gsop: mock_gsop(is_inhibitory, potentiation),
            spontaneous,
        }
    }

    #[test]
    fn test_spatial_grid_build_and_query() {
        let segments = vec![
            AxonSegment { axon_id: 10, type_idx: 0, pos: PackedPosition::pack_raw(5, 5, 5, 0).0 },
            AxonSegment { axon_id: 11, type_idx: 0, pos: PackedPosition::pack_raw(10, 10, 10, 1).0 },
            AxonSegment { axon_id: 12, type_idx: 0, pos: PackedPosition::pack_raw(6, 5, 5, 2).0 },
        ];

        let grid = SpatialGrid::build(segments);
        
        // Verify sorted ordering
        for i in 1..grid.segments.len() {
            assert!(grid.segments[i].pos >= grid.segments[i - 1].pos);
        }

        // Query in radius around (5, 5, 5)
        let mut results = Vec::new();
        grid.find_in_radius(PackedPosition::pack_raw(5, 5, 5, 0), 1, |seg| {
            results.push(seg.axon_id);
        });

        assert_eq!(results.len(), 2);
        assert!(results.contains(&10));
        assert!(results.contains(&12));
        assert!(!results.contains(&11)); // (10,10,10) is out of radius
    }

    #[test]
    fn test_find_first_empty_slot_and_duplicate() {
        let padded_n = 10;
        let mut existing_targets = vec![0; padded_n * 128];
        
        // Populate slots
        // slot 0 for soma 1 has target axon 5 (packed target value)
        existing_targets[1 + 0 * padded_n] = PackedTarget::pack(5, 0).0;
        // slot 1 for soma 1 is empty (0)
        // slot 2 for soma 1 has EMPTY_PIXEL
        existing_targets[1 + 2 * padded_n] = types::EMPTY_PIXEL;
        
        assert_eq!(find_first_empty_slot(1, &existing_targets, padded_n), Some(1));
        assert_eq!(find_first_empty_slot(2, &existing_targets, padded_n), Some(0)); // All empty for soma 2

        assert!(is_duplicate_axon(5, 1, &existing_targets, padded_n));
        assert!(!is_duplicate_axon(6, 1, &existing_targets, padded_n));
    }

    #[test]
    fn test_sprout_connections_full() {
        let blueprints = BlueprintsConfig {
            neuron_types: vec![
                mock_neuron_type("Exc", false, 15), // 15 potentiation
                mock_neuron_type("Inh", true, 5),  // 5 potentiation (will trigger DoA protection)
            ],
        };

        let soma_positions = vec![
            PackedPosition::pack_raw(10, 10, 10, 0), // soma 0
            PackedPosition::pack_raw(20, 20, 20, 0), // soma 1
        ];

        let segments = vec![
            // Segment near soma 0 (distance 1 in X), type Excitatory (index 0)
            AxonSegment { axon_id: 100, type_idx: 0, pos: PackedPosition::pack_raw(11, 10, 10, 0).0 },
            // Segment near soma 1 (distance 1 in Y), type Inhibitory (index 1)
            AxonSegment { axon_id: 200, type_idx: 1, pos: PackedPosition::pack_raw(20, 21, 20, 1).0 },
        ];

        let grid = SpatialGrid::build(segments);
        
        let padded_n = 2;
        let existing_targets = vec![0; padded_n * 128];
        
        // Active somas: 0 and 1
        let synapses = sprout_connections(
            &[0, 1],
            &existing_targets,
            padded_n,
            &grid,
            &blueprints,
            10, // prune_threshold
            &soma_positions,
        );

        assert_eq!(synapses.len(), 2);

        // Synapse 0 (Excitatory candidate 100, weight 15 shifted)
        assert_eq!(synapses[0].soma_idx, 0);
        assert_eq!(synapses[0].slot_idx, 0);
        assert_eq!(PackedTarget(synapses[0].target_packed).axon_id(), 100);
        assert_eq!(synapses[0].weight, 15 << 16);

        // Synapse 1 (Inhibitory candidate 200, initial 5 << 16 <= 10 << 16 threshold -> DoA protected weight = (10 + 100) << 16 * -1)
        assert_eq!(synapses[1].soma_idx, 1);
        assert_eq!(synapses[1].slot_idx, 0);
        assert_eq!(PackedTarget(synapses[1].target_packed).axon_id(), 200);
        assert_eq!(synapses[1].weight, -110 << 16);
    }
}
