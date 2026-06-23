//! GLIF soma membrane dynamics and homeostasis threshold adaptation functions.

/// Computes the GLIF membrane potential integration and leakage for a single tick.
///
/// # INV-PHYS-001 Zero Warp Divergence
/// This calculation contains no branching (`if`/`else` statements) to prevent thread
/// divergence on SIMT/GPU hardware.
///
/// # INV-PHYS-005 Zero-Float
/// This function executes entirely using integer arithmetic, preventing the platform-specific
/// discrepancies of floating-point computations (IEEE 754) in hot simulation cycles.
///
/// # Preconditions
/// - `leak_shift` must be less than 32 to avoid arithmetic shift overflow panics/undefined behavior.
///
/// # Mathematical Model
/// 1. Integrate input current: `v_integrated = voltage + input_current` (with wrapping math to prevent overflow panic under `INV-PHYS-004`).
/// 2. Compute potential difference from rest: `diff = v_integrated - rest_potential`
/// 3. Apply exponential leak via arithmetic shift: `v_integrated - (diff >> leak_shift)`
#[inline]
pub const fn compute_glif(
    voltage: i32,
    rest_potential: i32,
    leak_shift: u32,
    input_current: i32,
) -> i32 {
    let v_integrated = voltage.wrapping_add(input_current);
    let diff = v_integrated.wrapping_sub(rest_potential);
    v_integrated.wrapping_sub(diff >> leak_shift)
}

/// Legacy division-based GLIF calculation.
/// Uses integer division by `leak_rate` instead of bit shift.
#[inline]
pub const fn compute_glif_div(
    voltage: i32,
    rest_potential: i32,
    leak_rate: i32,
    input_current: i32,
) -> i32 {
    if leak_rate <= 0 {
        voltage + input_current
    } else {
        let leak = (voltage - rest_potential) / leak_rate;
        voltage - leak + input_current
    }
}

/// Updates the homeostatic threshold offset based on decay and spiking activity.
///
/// # INV-PHYS-001 Zero Warp Divergence
/// All bounds checks are implemented branchless via bitwise algebra.
///
/// # INV-PHYS-005 Zero-Float
/// Calculated entirely with integer math.
///
/// # Branchless Clamp Explanation
/// The clamping operator `clamped = decayed & !(decayed >> 31)` performs a branchless `max(0, decayed)` clamp:
/// - If `decayed` is positive, its sign bit (bit 31) is `0`. Shifting arithmetically by 31 yields `0x00000000`.
///   Its negation is `0xFFFFFFFF` (all ones). `decayed & 0xFFFFFFFF` returns `decayed` unchanged.
/// - If `decayed` is negative, its sign bit is `1`. Shifting arithmetically by 31 extends the sign bit,
///   yielding `0xFFFFFFFF` (all ones). Its negation is `0x00000000`. `decayed & 0x00000000` returns `0`.
#[inline]
pub const fn update_homeostasis(
    offset: i32,
    decay: u16,
    is_spiking: bool,
    penalty: i32,
) -> i32 {
    let decayed = offset.wrapping_sub(decay as i32);
    let clamped = decayed & !(decayed >> 31);
    clamped.wrapping_add((is_spiking as i32).wrapping_mul(penalty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glif_leak_positive_offset() {
        // Voltage = 20000, rest_potential = 10000, input_current = 0, leak_shift = 1.
        // v_integrated = 20000
        // diff = 10000
        // diff >> 1 = 5000
        // result = 20000 - 5000 = 15000.
        // Verify leak pulls the potential down towards rest potential.
        let val = compute_glif(20000, 10000, 1, 0);
        assert_eq!(val, 15000);
    }

    #[test]
    fn test_glif_leak_negative_offset() {
        // Voltage = 0, rest_potential = 10000, input_current = 0, leak_shift = 2.
        // v_integrated = 0
        // diff = -10000
        // diff >> 2 = -2500
        // result = 0 - (-2500) = 2500.
        // Verify leak pulls the potential up towards rest potential.
        let val = compute_glif(0, 10000, 2, 0);
        assert_eq!(val, 2500);
    }

    #[test]
    fn test_glif_at_rest_no_change() {
        // Voltage = rest_potential, no current.
        // Voltage = 10000, rest_potential = 10000, input_current = 0, leak_shift = 3.
        // v_integrated = 10000
        // diff = 0
        // diff >> 3 = 0
        // result = 10000.
        let val = compute_glif(10000, 10000, 3, 0);
        assert_eq!(val, 10000);
    }

    #[test]
    fn test_homeostasis_branchless_clamp() {
        // Offset = 50, decay = 100, is_spiking = false, penalty = 200.
        // decayed = -50
        // clamped = -50 & !(0xFFFFFFFF) = -50 & 0x0 = 0.
        // result = 0 + 0 = 0.
        let val = update_homeostasis(50, 100, false, 200);
        assert_eq!(val, 0);
    }

    #[test]
    fn test_homeostasis_spike_adds_penalty() {
        // Offset = 50, decay = 10, is_spiking = true, penalty = 200.
        // decayed = 40
        // clamped = 40 & !(0x0) = 40 & 0xFFFFFFFF = 40.
        // result = 40 + 200 = 240.
        let val = update_homeostasis(50, 10, true, 200);
        assert_eq!(val, 240);
    }

    #[test]
    fn test_glif_extreme_overflow_safety() {
        // INV-PHYS-004: Ensure no panic on signed overflow of voltages or currents
        let _ = compute_glif(i32::MAX, 10000, 5, i32::MAX);
        let _ = compute_glif(i32::MIN, -10000, 5, i32::MIN);
    }

    #[test]
    fn test_glif_leak_shift_boundaries() {
        // leak_shift = 0: instantaneous relaxation to rest potential
        let val_zero_shift = compute_glif(20000, 10000, 0, 0);
        assert_eq!(val_zero_shift, 10000);

        // leak_shift = 30: minimal leak shift. Arithmetic shift on a negative diff (-5)
        // should yield -1.
        // v_integrated = 9995, rest = 10000. diff = -5.
        // diff >> 30 = -1. result = 9995 - (-1) = 9996.
        let val_max_shift = compute_glif(9995, 10000, 30, 0);
        assert_eq!(val_max_shift, 9996);
    }

    #[test]
    fn test_glif_div_compatibility() {
        // voltage=100, rest=-70, leak=2, input=0 -> new_v = 15
        assert_eq!(compute_glif_div(100, -70, 2, 0), 15);
        // voltage=-100, rest=-70, leak=2, input=0 -> new_v = -85
        assert_eq!(compute_glif_div(-100, -70, 2, 0), -85);
        // voltage=-70, rest=-70, leak=2, input=50 -> new_v = -20
        assert_eq!(compute_glif_div(-70, -70, 2, 50), -20);
        // leak_rate <= 0 handling
        assert_eq!(compute_glif_div(100, -70, 0, 10), 110);
    }
}
