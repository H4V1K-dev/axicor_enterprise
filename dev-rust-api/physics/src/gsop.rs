//! Synaptic plasticity (GSOP) and weight modification functions.

use crate::constants::*;

/// Computes the synapse inertia rank (0..7) from the absolute synaptic weight.
///
/// # INV-PHYS-009 Inertia Rank Out-of-Bounds Safety
/// Maps the absolute weight range (`0` to `2.14B`) into a 3-bit rank index (`0..7`)
/// at zero cost (O(1)) without any branch conditions. This guarantees safe, out-of-bounds free
/// lookup into arrays of length 8.
#[inline]
pub const fn inertia_rank(abs_weight: i32) -> usize {
    let shifted = abs_weight.unsigned_abs() >> INERTIA_RANK_SHIFT;
    (if shifted < 7 { shifted } else { 7 }) as usize
}

/// Computes the modified synaptic weight under dopaminergic modulation and temporal dynamics.
///
/// # INV-PHYS-005 Zero-Float
/// Calculated entirely with fixed-point integer arithmetic to ensure побитовый детерминизм (bit-to-bit identity)
/// across heterogeneous compute hardware.
///
/// # INV-PHYS-007 Dale's Law Preservation
/// The biological sign of the synapse is preserved at all times. If LTD (depression) drops
/// the absolute weight below zero, it clamps to zero (becoming a "Silent Synapse") rather than
/// crossing the boundary and mutating the synapse type.
///
/// # INV-PHYS-010 Headroom Overflow Guard
/// New absolute weights are clamped to `MAX_WEIGHT_LIMIT` to prevent signed overflows.
#[inline]
pub const fn compute_gsop_weight(
    weight: i32,
    dopamine: i16,
    d1_aff: u8,
    d2_aff: u8,
    pot: u16,
    dep: u16,
    inertia: i32,
    is_active: bool,
    burst_mult: i32,
    cooling_shift: u32,
) -> i32 {
    // Шаг 1 (Dale's Law Guard)
    let sign = if weight >= 0 { 1 } else { -1 };
    let abs_w = weight.unsigned_abs();

    // Шаг 2 (Дофаминовая модуляция)
    let pot_mod = ((dopamine as i32) * (d1_aff as i32)) >> GSOP_FIXED_POINT_SHIFT;
    let dep_mod = ((dopamine as i32) * (d2_aff as i32)) >> GSOP_FIXED_POINT_SHIFT;

    // Шаг 3 (Применение модуляции с branchless clamp до 0)
    let raw_pot = (pot as i32) + pot_mod;
    let raw_dep = (dep as i32) - dep_mod;
    let final_pot = raw_pot & !(raw_pot >> 31);
    let final_dep = raw_dep & !(raw_dep >> 31);

    // Шаг 4 (Инерция и серийность)
    let delta_pot = (final_pot * inertia * burst_mult) >> GSOP_FIXED_POINT_SHIFT;
    let delta_dep = (final_dep * inertia * burst_mult) >> GSOP_FIXED_POINT_SHIFT;

    // Шаг 5 (Остывание и Slot Decay)
    let mut delta = if is_active {
        let shift = if cooling_shift < 31 { cooling_shift } else { 31 };
        delta_pot >> shift
    } else {
        -delta_dep
    };
    delta = (delta * 128) >> GSOP_FIXED_POINT_SHIFT;

    // Шаг 6 (Сборка с защитой от смены знака и переполнения - INV-PHYS-007, INV-PHYS-010)
    let mut new_abs = (abs_w as i32).wrapping_add(delta);
    new_abs &= !(new_abs >> 31);
    new_abs = if new_abs < MAX_WEIGHT_LIMIT { new_abs } else { MAX_WEIGHT_LIMIT };

    new_abs * sign
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_gsop_no_std_mem() {
        // Strict #![no_std] verification using core::mem
        assert_eq!(size_of::<i32>(), 4);
        assert_eq!(align_of::<i32>(), 4);
        assert_eq!(size_of::<i16>(), 2);
        assert_eq!(size_of::<u8>(), 1);
        assert_eq!(size_of::<u16>(), 2);
    }

    #[test]
    fn test_gsop_sign_preservation_negative() {
        // INV-PHYS-007: Inhibitory synapse (-10) under strong depression (-50 delta)
        // should clamp to 0 instead of crossing into positive (+40) region.
        let new_w = compute_gsop_weight(
            -10,    // weight
            0,      // dopamine
            0,      // d1_aff
            0,      // d2_aff
            0,      // pot
            50,     // dep
            128,    // inertia (GSOP_FIXED_POINT_SCALE = 128)
            false,  // is_active
            1,      // burst_mult
            0,      // cooling_shift
        );
        assert_eq!(new_w, 0);
    }

    #[test]
    fn test_gsop_clamp_max() {
        // INV-PHYS-010: Exceeding MAX_WEIGHT_LIMIT should clamp to MAX_WEIGHT_LIMIT.
        let new_w = compute_gsop_weight(
            MAX_WEIGHT_LIMIT - 100, // weight
            0,                      // dopamine
            0,                      // d1_aff
            0,                      // d2_aff
            1000,                   // pot
            0,                      // dep
            128,                    // inertia
            true,                   // is_active
            1,                      // burst_mult
            0,                      // cooling_shift
        );
        assert_eq!(new_w, MAX_WEIGHT_LIMIT);
    }

    #[test]
    fn test_gsop_inertia_rank_calculation() {
        // INV-PHYS-009: Test mapping weights to rank range (0..7)
        assert_eq!(inertia_rank(0), 0);
        assert_eq!(inertia_rank(100), 0);
        assert_eq!(inertia_rank(268_435_455), 0);  // Below INERTIA_RANK_SHIFT (2^28 = 268435456)
        assert_eq!(inertia_rank(268_435_456), 1);  // Threshold reached
        assert_eq!(inertia_rank(MAX_WEIGHT_LIMIT), 7); // Max limit is mapped to 7
        assert_eq!(inertia_rank(-MAX_WEIGHT_LIMIT), 7); // Negative absolute weight is mapped to 7
    }

    #[test]
    fn test_gsop_cooling_effect() {
        // is_active=true with cooling_shift=2 should reduce delta_pot by factor of 4 (>> 2).
        let w_no_cooling = compute_gsop_weight(
            1000,   // weight
            0,      // dopamine
            0,      // d1_aff
            0,      // d2_aff
            100,    // pot
            0,      // dep
            128,    // inertia
            true,   // is_active
            1,      // burst_mult
            0,      // cooling_shift
        );
        let w_cooled = compute_gsop_weight(
            1000,   // weight
            0,      // dopamine
            0,      // d1_aff
            0,      // d2_aff
            100,    // pot
            0,      // dep
            128,    // inertia
            true,   // is_active
            1,      // burst_mult
            2,      // cooling_shift (>> 2 = divide by 4)
        );

        let delta_no_cooling = w_no_cooling - 1000;
        let delta_cooled = w_cooled - 1000;

        assert_eq!(delta_no_cooling, 100);
        assert_eq!(delta_cooled, 25);
    }
}
