//! Deterministic local axon growth path geometry algorithms.

use crate::dto::{
    AxonGrowthInput, AxonGrowthStopReason, AxonSegment, GrownAxonPath, LocalGrowthResult,
};
use crate::error::TopologyError;
use std::collections::HashSet;
use types::PackedPosition;

/// Deterministically mixes salt values using wrapping FNV-1a hash formula.
#[inline(always)]
fn deterministic_mix_salt(soma_id: u32, step_index: usize, candidate_index: usize) -> u64 {
    let mut hash_val: u64 = 0xcbf2_9ce4_8422_2325;
    hash_val = (hash_val ^ (soma_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (step_index as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (candidate_index as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val
}

/// Deterministically grows local axon paths within a single shard.
pub fn grow_local_axons(input: &AxonGrowthInput) -> Result<LocalGrowthResult, TopologyError> {
    let config = input.config;
    let topology = input.topology;
    let seed = input.seed;

    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;

    // Build a lookup set of all placed soma coordinates for obstacle checking.
    let all_somas_coords: HashSet<(u32, u32, u32)> = topology
        .somas
        .iter()
        .map(|s| {
            (
                s.position.x() as u32,
                s.position.y() as u32,
                s.position.z() as u32,
            )
        })
        .collect();

    let mut axons = Vec::with_capacity(topology.somas.len());

    // Moore neighborhood (26 candidates) lexicographically ordered by (dz, dy, dx) from -1 to 1.
    let mut candidates = Vec::with_capacity(26);
    let mut candidate_idx = 0;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dz == 0 && dy == 0 && dx == 0 {
                    continue;
                }
                candidates.push((dx, dy, dz, candidate_idx));
                candidate_idx += 1;
            }
        }
    }

    let max_segments = (layout::MAX_SEGMENTS_PER_AXON - 1).min(types::MAX_SEGMENT_OFFSET as usize);

    for soma in &topology.somas {
        let variant_idx = soma.variant_id as usize;
        if variant_idx >= config.neuron_types.len() {
            return Err(TopologyError::UnknownNeuronType {
                variant_id: soma.variant_id,
            });
        }
        let source_type = &config.neuron_types[variant_idx];
        let growth = &source_type.growth;

        // Position coordinates extraction
        let sx = soma.position.x() as u32;
        let sy = soma.position.y() as u32;
        let sz = soma.position.z() as u32;

        // Boundary check for the source position itself.
        if sx >= shard_w || sy >= shard_d || sz >= shard_h {
            axons.push(GrownAxonPath {
                soma_id: soma.soma_id,
                segments: Vec::new(),
                stop_reason: AxonGrowthStopReason::SourceOutOfBounds,
            });
            continue;
        }

        // Q16 fixed-point conversion
        let inertia_q = (growth.steering_weight_inertia as f64 * 65536.0) as i64;
        let vertical_q = (growth.growth_vertical_bias as f64 * 65536.0) as i64;
        let jitter_q = (growth.steering_weight_jitter as f64 * 65536.0) as i64;

        let mut segments = Vec::new();
        let mut curr_x = sx;
        let mut curr_y = sy;
        let mut curr_z = sz;
        let mut d_prev = (0i32, 0i32, 0i32);

        // Keep track of coordinates visited by this axon path.
        // Voxel of the source soma is considered visited from step 0.
        let mut visited = HashSet::new();
        visited.insert((curr_x, curr_y, curr_z));

        let mut stop_reason = AxonGrowthStopReason::Blocked;

        for step_index in 1..=max_segments {
            let mut candidates_with_score = Vec::with_capacity(26);

            for &(dx, dy, dz, candidate_index) in &candidates {
                let salt = deterministic_mix_salt(soma.soma_id, step_index, candidate_index);
                let jitter_unit = (seed.random_u64(salt) >> 48) as i64;

                let dot = d_prev.0 * dx + d_prev.1 * dy + d_prev.2 * dz;
                let score_q = inertia_q * dot as i64
                    + vertical_q * dz as i64
                    + (jitter_q * jitter_unit) / 65535;

                candidates_with_score.push((dx, dy, dz, candidate_index, score_q));
            }

            // Sort: score_q DESC, then candidate_index ASC
            candidates_with_score.sort_by(|a, b| match b.4.cmp(&a.4) {
                std::cmp::Ordering::Equal => a.3.cmp(&b.3),
                ord => ord,
            });

            let mut step_taken = false;
            for &(dx, dy, dz, _, _) in &candidates_with_score {
                let next_x = curr_x as i32 + dx;
                let next_y = curr_y as i32 + dy;
                let next_z = curr_z as i32 + dz;

                // Boundary verification
                if next_x < 0
                    || next_x >= shard_w as i32
                    || next_y < 0
                    || next_y >= shard_d as i32
                    || next_z < 0
                    || next_z >= shard_h as i32
                {
                    stop_reason = AxonGrowthStopReason::BoundaryReached;
                    break;
                }

                let nx = next_x as u32;
                let ny = next_y as u32;
                let nz = next_z as u32;

                // Obstacle and self-intersection checks
                if visited.contains(&(nx, ny, nz)) {
                    continue;
                }

                // Somas of other neurons check
                if all_somas_coords.contains(&(nx, ny, nz)) {
                    continue;
                }

                // Successful move
                curr_x = nx;
                curr_y = ny;
                curr_z = nz;
                d_prev = (dx, dy, dz);
                visited.insert((curr_x, curr_y, curr_z));

                let packed_pos = PackedPosition::try_new(curr_x, curr_y, curr_z, soma.variant_id)
                    .map_err(|_| TopologyError::VoxelBoundsOverflow {
                    x: curr_x,
                    y: curr_y,
                    z: curr_z,
                })?;

                segments.push(AxonSegment {
                    position: packed_pos,
                    segment_offset: step_index as u8,
                });

                step_taken = true;
                break;
            }

            if stop_reason == AxonGrowthStopReason::BoundaryReached {
                break;
            }

            if !step_taken {
                stop_reason = AxonGrowthStopReason::Blocked;
                break;
            }

            if step_index == max_segments {
                stop_reason = AxonGrowthStopReason::MaxLengthReached;
            }
        }

        axons.push(GrownAxonPath {
            soma_id: soma.soma_id,
            segments,
            stop_reason,
        });
    }

    Ok(LocalGrowthResult { axons })
}
