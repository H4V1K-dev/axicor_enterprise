//! Generic Synaptic Optimization Protocol (GSOP) synaptic plasticity algorithms.

use crate::constants::{
    INERTIA_RANK_SHIFT, MASS_TO_CHARGE_SHIFT, MAX_INERTIA_RANK, MAX_WEIGHT_LIMIT, MIN_WEIGHT_LIMIT,
};

/// Converts a synaptic weight from Mass Domain (`i32`) to Charge Domain (`i32`) via bit shift (`weight >> 16`).
#[inline]
pub fn weight_to_charge(weight: i32) -> i32 {
    weight >> MASS_TO_CHARGE_SHIFT
}

/// Calculates the O(1) inertia rank index (`0..=7`) from absolute synaptic mass.
#[inline]
pub fn inertia_rank(abs_weight: u32) -> usize {
    ((abs_weight >> INERTIA_RANK_SHIFT) as usize).min(MAX_INERTIA_RANK)
}

/// Recovers dendritic fatigue by subtracting `FATIGUE_RECOVERY_RATE` per tick.
#[inline]
pub fn recover_fatigue(fatigue: u8) -> u8 {
    fatigue.saturating_sub(crate::constants::FATIGUE_RECOVERY_RATE)
}

/// Calculates available dendritic capacity `(capacity - fatigue)` for weight attenuation.
#[inline]
pub fn fatigue_available(fatigue: u8, capacity: u8) -> u8 {
    let cap = capacity.max(1);
    let fat = fatigue.min(cap);
    cap - fat
}

/// Attenuates synaptic weight based on dendritic fatigue ratio `available / capacity`.
#[inline]
pub fn apply_synaptic_fatigue(weight: i32, fatigue: u8, capacity: u8) -> i32 {
    let cap = capacity.max(1) as i64;
    let avail = fatigue_available(fatigue, capacity) as i64;
    ((weight as i64 * avail) / cap) as i32
}

/// Increments dendritic fatigue upon receiving a presynaptic active tail hit.
#[inline]
pub fn fatigue_after_spike(fatigue: u8, capacity: u8) -> u8 {
    let cap = capacity.max(1);
    fatigue
        .saturating_add(crate::constants::FATIGUE_SPIKE_COST)
        .min(cap)
}

/// Applies All-to-All Spatial STDP synaptic plasticity with dendritic fatigue penalty.
///
/// ### Competitive Depression
/// If a post-synaptic spike occurs but there is no causal LTP contribution (i.e., no axonal head
/// is within the causal active tail window for the dendritic segment), the synapse is depressed
/// by a flat base depression (`base_ltd`) representing competitive LTD. This prevents inactive/unmatched
/// synapses from remaining flat, aligning behavior with competitive learning intent. If a causal
/// LTP contribution is present, the standard STDP path is preserved, and LTD is determined solely
/// by anti-causal cooling.
///
/// # Arguments
/// * `weight` - Current synaptic mass (`i32`).
/// * `heads` - Axonal head segment positions (`&[u32; 8]`).
/// * `seg_idx` - Target dendrite segment index (`u32`).
/// * `signal_propagation_length` - Axon active tail propagation length (`u32`).
/// * `fatigue` - Current dendritic fatigue level (`u8`).
/// * `fatigue_capacity` - Maximum fatigue capacity (`u8`).
/// * `gsop_potentiation` - Base potentiation pulse amount (LTP).
/// * `gsop_depression` - Base depression pulse amount (LTD).
/// * `dopamine` - Global dopamine neuromodulation level.
/// * `d1_affinity` - Profile D1 receptor affinity (amplifies LTP).
/// * `d2_affinity` - Profile D2 receptor affinity (suppresses LTD under reward).
/// * `burst_count` - Axonal burst count multiplier.
/// * `inertia_curve` - Array of 8 inertia multipliers corresponding to ranks 0..7.
///
/// # Returns
/// Updated synaptic mass (`i32`) strictly maintaining biological sign and clamped to `[MIN_WEIGHT_LIMIT, MAX_WEIGHT_LIMIT]`.
#[allow(clippy::too_many_arguments)]
pub fn apply_gsop_plasticity(
    weight: i32,
    heads: &[u32; 8],
    seg_idx: u32,
    signal_propagation_length: u32,
    fatigue: u8,
    fatigue_capacity: u8,
    gsop_potentiation: i32,
    gsop_depression: i32,
    dopamine: i32,
    d1_affinity: i32,
    d2_affinity: i32,
    burst_count: u32,
    inertia_curve: &[i32; 8],
) -> i32 {
    let sign: i32 = 1 - ((weight >> 31) & 2);
    let abs_w = weight.unsigned_abs();

    let rank = inertia_rank(abs_w);
    let inertia = inertia_curve[rank] as i64;

    let pot_mod = (dopamine as i64 * d1_affinity as i64) / 128;
    let dep_mod = (dopamine as i64 * d2_affinity as i64) / 128;

    let final_pot = (gsop_potentiation as i64 + pot_mod).max(0);
    let final_dep = (gsop_depression as i64 - dep_mod).max(0);

    let burst_mult = (burst_count as i64).max(1);
    let prop = signal_propagation_length as u64;

    let mut total_ltp: i64 = 0;
    let mut total_ltd: i64 = 0;
    let mut has_causal_hit = false;

    if prop > 0 {
        for &head in heads {
            if head == types::AXON_SENTINEL {
                continue;
            }
            let head_u64 = head as u64;
            let seg_u64 = seg_idx as u64;

            // Causal LTP: spike passed segment (head >= seg_idx)
            let dist_ltp = head_u64.wrapping_sub(seg_u64);
            if dist_ltp < prop {
                has_causal_hit = true;
                let cooling = prop - dist_ltp;
                let base_ltp = (final_pot * inertia * burst_mult) / 128;
                total_ltp += (base_ltp * cooling as i64) / prop as i64;
            }

            // Anti-causal LTD: spike approaching segment (seg_idx >= head)
            let dist_ltd = seg_u64.wrapping_sub(head_u64);
            if dist_ltd < prop {
                let cooling = prop - dist_ltd;
                let base_ltd = (final_dep * inertia * burst_mult) / 128;
                total_ltd += (base_ltd * cooling as i64) / prop as i64;
            }
        }
    }

    // Fatigue penalty
    let base_ltd = (final_dep * inertia * burst_mult) / 128;
    let cap = fatigue_capacity.max(1) as i64;
    let fat = (fatigue.min(fatigue_capacity)) as i64;
    let fatigue_penalty = (fat * base_ltd) / cap;

    let ltd_contrib = if prop > 0 {
        if has_causal_hit {
            total_ltd
        } else {
            base_ltd
        }
    } else {
        0
    };

    let net_delta = total_ltp - ltd_contrib - fatigue_penalty;

    let new_abs_raw = abs_w as i64 + net_delta;
    let new_abs = new_abs_raw.clamp(MIN_WEIGHT_LIMIT as i64, MAX_WEIGHT_LIMIT as i64) as u32;

    (new_abs as i32) * sign
}
