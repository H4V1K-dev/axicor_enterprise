//! Deterministic soma placement core algorithms.

use crate::dto::{PlacedSoma, SingleShardTopology, SingleShardTopologyInput};
use crate::error::TopologyError;
use std::collections::HashSet;

/// Structure to hold layer-specific placement properties before resolution.
struct LayerSlice {
    layer_index: usize,
    z_start: u32,
    layer_capacity: u64,
    neuron_counts: Vec<NeuronAllocation>,
}

struct NeuronAllocation {
    variant_id: u8,
    allocated: usize,
}

/// Helper structure for the Hamilton / Largest-Remainder distribution algorithm.
struct HamiltonCandidate {
    variant_id: u8,
    allocated: usize,
    fractional: f64,
}

/// Deterministic 64-bit mixer to combine salt values.
#[inline(always)]
fn deterministic_mix_salt(layer_index: usize, variant_id: u8, local_ordinal: usize) -> u64 {
    let mut hash_val: u64 = 0xcbf2_9ce4_8422_2325;

    // Hash layer_index
    hash_val = (hash_val ^ (layer_index as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    // Hash variant_id
    hash_val = (hash_val ^ (variant_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    // Hash local_ordinal
    hash_val = (hash_val ^ (local_ordinal as u64)).wrapping_mul(0x0000_0100_0000_01B3);

    hash_val
}

/// Deterministically generates the spatial topology (soma placement) of a single shard.
pub fn generate_single_shard_topology(
    input: &SingleShardTopologyInput,
) -> Result<SingleShardTopology, TopologyError> {
    let config = input.config;
    let seed = input.seed;
    let dimensions = &config.dimensions;
    let shard_h = dimensions.h;
    let shard_w = dimensions.w as u64;
    let shard_d = dimensions.d as u64;

    // 1. Z-slicing of layers
    let mut slices = Vec::with_capacity(config.layers.len());
    let mut cumulative_before = 0.0_f64;

    for (layer_index, layer) in config.layers.iter().enumerate() {
        let height_pct = layer.height_pct as f64;
        let cumulative_after = cumulative_before + height_pct;

        let z_start = (cumulative_before * shard_h as f64).floor() as u32;
        let mut z_end = (cumulative_after * shard_h as f64).floor() as u32;

        // Force the last layer to end exactly at shard_h
        if layer_index == config.layers.len() - 1 {
            z_end = shard_h;
        }

        let layer_h_voxels = z_end
            .checked_sub(z_start)
            .ok_or(TopologyError::CapacityOverflow)?;

        if layer_h_voxels == 0 && layer.density > 0.0 {
            return Err(TopologyError::LayerGeometryError {
                layer_index,
                msg: format!(
                    "Layer '{}' has 0 Z-voxels height but positive density={}",
                    layer.name, layer.density
                ),
            });
        }

        // 2. Capacity calculations using checked arithmetic
        let layer_capacity = shard_w
            .checked_mul(shard_d)
            .and_then(|val| val.checked_mul(layer_h_voxels as u64))
            .ok_or(TopologyError::CapacityOverflow)?;

        // 3. Compute soma target count in layer
        let density = layer.density as f64;
        let layer_soma_count_f64 = (layer_capacity as f64 * density).floor();
        if layer_soma_count_f64 < 0.0 || layer_soma_count_f64 > usize::MAX as f64 {
            return Err(TopologyError::CapacityOverflow);
        }
        let layer_soma_count = layer_soma_count_f64 as usize;

        // 4. Largest-Remainder (Hamilton) distribution
        let mut candidates = Vec::with_capacity(layer.composition.len());
        let mut sum_allocated = 0_usize;

        for comp in &layer.composition {
            // Retrieve variant_id from ShardConfig.neuron_types
            let variant_id = config
                .neuron_types
                .iter()
                .position(|nt| nt.name == comp.type_name)
                .ok_or_else(|| TopologyError::LayerGeometryError {
                    layer_index,
                    msg: format!(
                        "Neuron type '{}' specified in layer composition not found in neuron_types",
                        comp.type_name
                    ),
                })? as u8;

            let ideal = layer_soma_count as f64 * comp.share as f64;
            let base = ideal.floor() as usize;
            let fractional = ideal - ideal.floor();

            candidates.push(HamiltonCandidate {
                variant_id,
                allocated: base,
                fractional,
            });

            sum_allocated = sum_allocated
                .checked_add(base)
                .ok_or(TopologyError::CapacityOverflow)?;
        }

        let left = layer_soma_count
            .checked_sub(sum_allocated)
            .ok_or(TopologyError::CapacityOverflow)?;

        // Sort candidates: descending by fractional remainder, tie-break by smaller variant_id first (ascending)
        candidates.sort_by(|a, b| match b.fractional.total_cmp(&a.fractional) {
            std::cmp::Ordering::Equal => a.variant_id.cmp(&b.variant_id),
            other => other,
        });

        // Distribute remainder
        for cand in candidates.iter_mut().take(left) {
            cand.allocated = cand
                .allocated
                .checked_add(1)
                .ok_or(TopologyError::CapacityOverflow)?;
        }

        // Verify total distributed matches expected count
        let final_sum: usize = candidates.iter().map(|c| c.allocated).sum();
        if final_sum != layer_soma_count {
            return Err(TopologyError::CompositionMismatch {
                layer_index,
                expected: layer_soma_count,
                actual: final_sum,
            });
        }

        // Map candidates to layer allocations
        let neuron_counts = candidates
            .into_iter()
            .map(|c| NeuronAllocation {
                variant_id: c.variant_id,
                allocated: c.allocated,
            })
            .collect();

        slices.push(LayerSlice {
            layer_index,
            z_start,
            layer_capacity,
            neuron_counts,
        });

        cumulative_before = cumulative_after;
    }

    // 5. Placed Somas generation following stable soma_id sorting hierarchy
    let mut somas = Vec::new();
    let mut next_soma_id = 0_u32;

    for slice in slices {
        let mut layer_occupied = HashSet::new();

        // Sort allocations by variant_id to guarantee stable ID assignments
        let mut neuron_counts = slice.neuron_counts;
        neuron_counts.sort_by_key(|a| a.variant_id);

        for alloc in neuron_counts {
            let variant_id = alloc.variant_id;
            let allocated = alloc.allocated;

            for local_ordinal in 0..allocated {
                let salt = deterministic_mix_salt(slice.layer_index, variant_id, local_ordinal);
                let start = seed.random_u64(salt) % slice.layer_capacity;

                let mut placed = false;
                for attempt in 0..slice.layer_capacity {
                    let flat_idx = (start
                        .checked_add(attempt)
                        .ok_or(TopologyError::CapacityOverflow)?)
                        % slice.layer_capacity;

                    if !layer_occupied.contains(&flat_idx) {
                        let z = slice
                            .z_start
                            .checked_add((flat_idx / (shard_w * shard_d)) as u32)
                            .ok_or(TopologyError::CapacityOverflow)?;
                        let rem = flat_idx % (shard_w * shard_d);
                        let y = (rem / shard_w) as u32;
                        let x = (rem % shard_w) as u32;

                        let position = types::PackedPosition::try_new(x, y, z, variant_id)
                            .map_err(|_| TopologyError::VoxelBoundsOverflow { x, y, z })?;

                        somas.push(PlacedSoma {
                            soma_id: next_soma_id,
                            variant_id,
                            position,
                        });

                        layer_occupied.insert(flat_idx);
                        next_soma_id = next_soma_id
                            .checked_add(1)
                            .ok_or(TopologyError::CapacityOverflow)?;
                        placed = true;
                        break;
                    }
                }

                if !placed {
                    return Err(TopologyError::LayerCapacityExceeded {
                        layer_index: slice.layer_index,
                        max_capacity: slice.layer_capacity as usize,
                    });
                }
            }
        }
    }

    Ok(SingleShardTopology { somas })
}
