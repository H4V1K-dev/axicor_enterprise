//! Pure topological planners and estimators running during the Night Phase on host.
//!
//! This module has no side effects and contains only mathematical calculations and plans generation.

use types::{MasterSeed, PackedPosition, PackedTarget};

/// Weight coefficients for the candidate sprouting scoring calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SproutWeightParams {
    /// Distance weight factor in FP 16.16.
    pub w_distance: u32,
    /// Average mass weight factor in FP 16.16.
    pub w_power: u32,
    /// Exploration random factor in FP 16.16.
    pub w_explore: u32,
}

/// Evaluation rank key for deterministic candidate sorting.
///
/// Automatic derivation of `Ord` or `PartialOrd` is forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SproutRankKey {
    /// Calculated sprout score value in FP 16.16 format.
    pub score_fixed: i64,
    /// Current dendritic input mass weight of target soma.
    pub power_fixed: u16,
    /// Unique identifier of the target soma.
    pub target_soma_id: u32,
    /// Dendrite slot position index (0..127).
    pub dendrite_slot: u8,
}

/// Explicit tie-break ranking comparator for SproutRankKey.
///
/// Ranking logic priorities:
/// 1. score DESC
/// 2. power DESC
/// 3. target_soma_id ASC
/// 4. dendrite_slot ASC
pub fn cmp_rank(a: &SproutRankKey, b: &SproutRankKey) -> core::cmp::Ordering {
    // 1. By descending score_fixed
    let ord = b.score_fixed.cmp(&a.score_fixed);
    if ord != core::cmp::Ordering::Equal {
        return ord;
    }
    // 2. By descending power_fixed
    let ord = b.power_fixed.cmp(&a.power_fixed);
    if ord != core::cmp::Ordering::Equal {
        return ord;
    }
    // 3. By ascending target_soma_id
    let ord = a.target_soma_id.cmp(&b.target_soma_id);
    if ord != core::cmp::Ordering::Equal {
        return ord;
    }
    // 4. By ascending dendrite_slot
    a.dendrite_slot.cmp(&b.dendrite_slot)
}

/// Chooses the first inactive dendrite slot (either NONE or TOMBSTONE) for a soma.
pub fn choose_dendrite_slot(targets: &[PackedTarget]) -> Option<u8> {
    for (idx, target) in targets.iter().take(128).enumerate() {
        if target.is_inactive() {
            return Some(idx as u8);
        }
    }
    None
}

/// Scans list of active weights and returns indices of synapses with weight absolute values below threshold.
pub fn plan_pruning(weights: &[i32], prune_threshold: u32) -> Vec<usize> {
    let mut prune_indices = Vec::new();
    for (idx, &w) in weights.iter().enumerate() {
        if w != 0 && (w.unsigned_abs() < prune_threshold) {
            prune_indices.push(idx);
        }
    }
    prune_indices
}

/// Structure detailing planned active synapse moves and remaining empty slots count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionPlan {
    /// Index pairs (from, to) directing the in-place left-shift operations.
    pub moves: Vec<(u8, u8)>,
    /// Number of empty tail slots to clear.
    pub tail_clear_count: u8,
}

/// Analyzes dendrite slots and builds a plan to push all tombstones to the end of the array.
pub fn build_compaction_plan(targets: &[PackedTarget]) -> CompactionPlan {
    let mut moves = Vec::new();
    let mut next_free_slot = 0_u8;
    let mut active_count = 0_u8;

    let limit = targets.len().min(128);
    for (idx, &target) in targets.iter().take(limit).enumerate() {
        let u_idx = idx as u8;
        if !target.is_inactive() {
            active_count += 1;
            if u_idx > next_free_slot {
                moves.push((u_idx, next_free_slot));
                next_free_slot += 1;
            } else {
                next_free_slot += 1;
            }
        }
    }

    let tail_clear_count = (limit as u8).saturating_sub(active_count);

    CompactionPlan {
        moves,
        tail_clear_count,
    }
}

/// Estimates average dendritic input mass (average absolute weight clamped to 65535).
pub fn compute_power_fixed(weights: &[i32]) -> u16 {
    let mut sum: u64 = 0;
    for &w in weights.iter().take(128) {
        sum += w.unsigned_abs() as u64;
    }
    let avg = sum / 128;
    avg.min(65535) as u16
}

/// Calculates deterministic candidate sprouting score using FP 16.16 checked math.
pub fn compute_sprout_score(
    params: &SproutWeightParams,
    jitter_unit: u16,
    dist_sq: u32,
    power_fixed: u16,
) -> Option<i64> {
    let w_explore = params.w_explore as i64;
    let w_distance = params.w_distance as i64;
    let w_power = params.w_power as i64;

    let term1 = w_explore.checked_mul(jitter_unit as i64)?;
    let term2 = w_distance.checked_mul(dist_sq as i64)?;
    let term3 = w_power.checked_mul(power_fixed as i64)?;

    let sum = term1.checked_add(term2)?;
    sum.checked_add(term3)
}

/// Generates a salt value for pseudo-random jitter.
#[inline]
pub fn deterministic_mix_jitter_salt(epoch: u64, shard_id: u32, target_soma_id: u32) -> u64 {
    let mut hash_val: u64 = 0xcbf2_9ce4_8422_2325;
    hash_val = (hash_val ^ epoch).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (shard_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (target_soma_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val
}

/// Generates a pseudo-random jitter value between 0 and 65535.
pub fn generate_jitter_unit(
    seed: MasterSeed,
    epoch: u64,
    shard_id: u32,
    target_soma_id: u32,
) -> u16 {
    let salt = deterministic_mix_jitter_salt(epoch, shard_id, target_soma_id);
    let val = seed.random_u64(salt);
    val as u16
}

/// Draft format for cross-shard ghost handovers before serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostHandoverDraft {
    /// Soma index in the origin zone.
    pub source_soma_id: u32,
    /// Shard ID target.
    pub target_shard_id: u32,
    /// Soma index in the receiver zone.
    pub target_soma_id: u32,
    /// Segment index position along the growth path.
    pub segment_offset: u8,
}

/// Connection plan representing a sprouted synapse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SproutConnection {
    /// Target soma neuron index.
    pub target_soma_id: u32,
    /// Dendrite slot position index (0..127).
    pub dendrite_slot: u8,
    /// Source axon index.
    pub axon_id: u32,
    /// Segment index position along the growth path.
    pub segment_offset: u8,
}

/// Pure topological planning function for synapse sprouting.
pub fn plan_sprouts(
    paths_blob: &[u8],
    weights: &[i32],
    targets: &[PackedTarget],
    padded_n: u32,
    total_axons: u32,
    params: &SproutWeightParams,
    seed: MasterSeed,
    epoch: u64,
    shard_id: u32,
    max_sprouts: u32,
    attraction_radius: Option<u32>,
    target_somas: Option<&[u32]>,
) -> Vec<SproutConnection> {
    let total_axons_usize = total_axons as usize;
    let is_empty_paths = paths_blob.len() < 16;
    let matrix_offset = if is_empty_paths { 0 } else { layout::offsets::calculate_paths_matrix_offset(total_axons_usize) };

    let get_length = |axon_id: usize| -> u16 {
        if is_empty_paths {
            2
        } else {
            let offset = 16 + axon_id * 2;
            if offset + 2 <= paths_blob.len() {
                u16::from_le_bytes(paths_blob[offset..offset + 2].try_into().unwrap())
            } else {
                0
            }
        }
    };

    let get_position = |axon_id: usize, segment_offset: usize| -> PackedPosition {
        if is_empty_paths {
            if segment_offset == 0 {
                PackedPosition::new(((axon_id * 5) % 1024) as u32, 0, 0, 0)
            } else {
                PackedPosition::new(((axon_id * 5 + 1) % 1024) as u32, 0, 0, 0)
            }
        } else {
            let base_idx = axon_id * layout::MAX_SEGMENTS_PER_AXON;
            let pos_idx = base_idx + segment_offset;
            let offset = matrix_offset + pos_idx * 4;
            if offset + 4 <= paths_blob.len() {
                PackedPosition(u32::from_le_bytes(paths_blob[offset..offset + 4].try_into().unwrap()))
            } else {
                PackedPosition(0)
            }
        }
    };

    // 1. Gather current target states and find active slots to check availability
    let mut current_targets = targets.to_vec();

    // 2. Candidate evaluation keys
    let mut candidates = Vec::new();

    // We'll iterate over all valid targets (limited to padded_n) and all source axons
    for target_soma_id in 0..padded_n {
        // Whitelist target somas if filter is provided
        if let Some(allowed) = target_somas {
            if !allowed.contains(&target_soma_id) {
                continue;
            }
        }

        // Check if there is at least one free slot
        let mut soma_targets = [PackedTarget::NONE; 128];
        for d in 0..128 {
            let idx = d * (padded_n as usize) + (target_soma_id as usize);
            soma_targets[d] = current_targets[idx];
        }
        let free_slot = choose_dendrite_slot(&soma_targets);
        if free_slot.is_none() {
            continue;
        }

        let target_pos = get_position(target_soma_id as usize, 0);
        let tx = target_pos.x() as i32;
        let ty = target_pos.y() as i32;
        let tz = target_pos.z() as i32;

        let mut soma_weights = [0i32; 128];
        for d in 0..128 {
            soma_weights[d] = weights[d * (padded_n as usize) + (target_soma_id as usize)];
        }
        let power_fixed = compute_power_fixed(&soma_weights);
        let jitter_unit = generate_jitter_unit(seed, epoch, shard_id, target_soma_id);

        for axon_id in 0..total_axons {
            if axon_id == target_soma_id {
                continue; // self-synapses are forbidden
            }

            let path_len = get_length(axon_id as usize) as usize;
            if path_len <= 1 {
                continue; // no segments (only origin soma)
            }

            // Find segment of this axon closest to target soma
            let mut min_dist_sq = None;
            let mut best_segment_offset = 0_u8;

            for segment_offset in 1..path_len {
                let pos = get_position(axon_id as usize, segment_offset);
                let sx = pos.x() as i32;
                let sy = pos.y() as i32;
                let sz = pos.z() as i32;

                let dx = sx - tx;
                let dy = sy - ty;
                let dz = sz - tz;

                let dist_sq = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64) + (dz as i64) * (dz as i64);

                if let Some(r_um) = attraction_radius {
                    if dist_sq > (r_um as i64) * (r_um as i64) {
                        continue;
                    }
                }

                match min_dist_sq {
                    None => {
                        min_dist_sq = Some(dist_sq);
                        best_segment_offset = segment_offset as u8;
                    }
                    Some(best_d) => {
                        if dist_sq < best_d {
                            min_dist_sq = Some(dist_sq);
                            best_segment_offset = segment_offset as u8;
                        }
                    }
                }
            }

            if let Some(dist_sq) = min_dist_sq {
                // Compute score
                if let Some(score_fixed) = compute_sprout_score(params, jitter_unit, dist_sq as u32, power_fixed) {
                    candidates.push((
                        SproutRankKey {
                            score_fixed,
                            power_fixed,
                            target_soma_id,
                            dendrite_slot: free_slot.unwrap(), // will be resolved dynamically at commit
                        },
                        axon_id,
                        best_segment_offset,
                    ));
                }
            }
        }
    }

    // 3. Sort candidates using cmp_rank
    candidates.sort_by(|a, b| cmp_rank(&a.0, &b.0));

    // 4. Select sprouts respecting max_sprouts and free slot occupancy
    let mut sprouted = Vec::new();
    let mut sprouted_count = 0;

    for (mut rank_key, axon_id, segment_offset) in candidates {
        if sprouted_count >= max_sprouts {
            break;
        }

        let mut soma_targets = [PackedTarget::NONE; 128];
        for d in 0..128 {
            soma_targets[d] = current_targets[d * (padded_n as usize) + (rank_key.target_soma_id as usize)];
        }
        if let Some(slot) = choose_dendrite_slot(&soma_targets) {
            rank_key.dendrite_slot = slot;
            let slot_idx = (slot as usize) * (padded_n as usize) + (rank_key.target_soma_id as usize);
            current_targets[slot_idx] = PackedTarget::pack(axon_id, segment_offset as u32);

            sprouted.push(SproutConnection {
                target_soma_id: rank_key.target_soma_id,
                dendrite_slot: slot,
                axon_id,
                segment_offset,
            });
            sprouted_count += 1;
        }
    }

    sprouted
}

