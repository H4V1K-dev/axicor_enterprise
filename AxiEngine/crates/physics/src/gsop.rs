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

/// Applies GSOP synaptic plasticity to update a single synaptic weight while enforcing Dale's Law.
///
/// # Arguments
/// * `weight` - Current synaptic mass (`i32`).
/// * `is_active` - Whether the target dendrite segment contacted an active axonal tail.
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
/// Implements branchless sign extraction, delta selection, and clamping adhering strictly to `INV-PHYS-001`.
#[allow(clippy::too_many_arguments)] // Takes raw scalar parameters matching physical specs without layout DTOs.
pub fn apply_gsop_plasticity(
    weight: i32,
    is_active: bool,
    gsop_potentiation: i32,
    gsop_depression: i32,
    dopamine: i32,
    d1_affinity: i32,
    d2_affinity: i32,
    burst_count: u32,
    inertia_curve: &[i32; 8],
) -> i32 {
    // 1. Extract biological sign (+1 or -1) and absolute weight branchlessly (INV-PHYS-001, INV-PHYS-004)
    let sign: i32 = 1 - ((weight >> 31) & 2);
    let abs_w = weight.unsigned_abs();

    // 2. Branchless inertia rank lookup
    let rank = inertia_rank(abs_w);
    let inertia = inertia_curve[rank] as i64;

    // 3. Dopamine neuromodulation
    let pot_mod = (dopamine as i64 * d1_affinity as i64) / 128;
    let dep_mod = (dopamine as i64 * d2_affinity as i64) / 128;

    let final_pot = (gsop_potentiation as i64 + pot_mod).max(0);
    let final_dep = (gsop_depression as i64 - dep_mod).max(0);

    let burst_mult = (burst_count as i64).max(1);

    // 4. Delta impulse calculation with branchless mask selection
    let delta_pot = (final_pot * inertia * burst_mult) / 128;
    let delta_dep = (final_dep * inertia * burst_mult) / 128;

    let active_mask = 0i64.wrapping_sub(is_active as i64);
    let delta = (delta_pot & active_mask) | ((-delta_dep) & !active_mask);

    // 5. Apply delta and clamp absolute mass (Mass Floor Guard & Headroom Guard)
    let new_abs_raw = abs_w as i64 + delta;
    let new_abs = new_abs_raw.clamp(MIN_WEIGHT_LIMIT as i64, MAX_WEIGHT_LIMIT as i64) as u32;

    // 6. Restore biological sign branchlessly (Dale's Law Preservation / INV-PHYS-007)
    (new_abs as i32) * sign
}
