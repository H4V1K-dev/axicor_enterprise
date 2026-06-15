//! Ahead-of-Time (AOT) compiler calculations and physical validations.
//!
//! This module is the only component in the Layer 0 physics engine where `f32` type
//! operations and `Result` types are permitted, since AOT validations do not execute
//! in hot high-frequency execution loops.

/// Ahead-of-Time calculation of the DDS heartbeat phase increment.
///
/// Returns `65536 / period_ticks` or `0` if `period_ticks` is 0 to prevent division by zero.
#[inline]
pub const fn compile_dds_heartbeat(period_ticks: u32) -> u32 {
    if period_ticks == 0 {
        0
    } else {
        65536 / period_ticks
    }
}

/// Safe derivation of the discrete signal speed `v_seg` (segments per tick).
///
/// # Invariants
/// - **INV-CONFIG-003**: `v_seg` must be strictly an integer.
/// - **INV-CROSS-005**: `v_seg` must not exceed `255`.
///
/// # Errors
/// Returns an error if the calculated speed step has a fractional part greater than `1e-5`,
/// protecting the integer-based execution engine from FPU drift (`E-087`), or if `v_seg > 255`.
pub fn compute_v_seg(
    speed_m_s: f32,
    tick_us: u32,
    voxel_um: f32,
    seg_voxels: u32,
) -> Result<u32, &'static str> {
    if seg_voxels == 0 {
        return Err("Segment voxels count must be greater than zero");
    }
    if voxel_um <= 0.0 {
        return Err("Voxel size must be greater than zero");
    }

    let segment_length_um = voxel_um * (seg_voxels as f32);
    let speed_um_tick = speed_m_s * (tick_us as f32);
    let v_seg_f32 = speed_um_tick / segment_length_um;

    // Helper functions for f32 round/abs under #![no_std]
    let v_seg_rounded = ((v_seg_f32 + 0.5) as u32) as f32;
    let diff = v_seg_f32 - v_seg_rounded;
    let diff_abs = f32::from_bits(diff.to_bits() & 0x7FFFFFFF);

    if diff_abs > 1e-5 {
        return Err("v_seg is not an integer: Integer Physics constraint violated");
    }

    let v_seg = v_seg_rounded as u32;
    if v_seg > 255 {
        return Err("v_seg exceeds maximum allowed value of 255");
    }

    Ok(v_seg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_dds_heartbeat() {
        assert_eq!(compile_dds_heartbeat(0), 0);
        assert_eq!(compile_dds_heartbeat(100), 655);
        assert_eq!(compile_dds_heartbeat(65536), 1);
    }

    #[test]
    fn test_compute_v_seg_success() {
        // speed = 2.0 m/s, tick = 10us -> speed_um_tick = 20.0 um
        // voxel = 10.0 um, seg_voxels = 1 -> segment_length_um = 10.0 um
        // v_seg = 20.0 / 10.0 = 2 (exact integer)
        let v_seg = compute_v_seg(2.0, 10, 10.0, 1);
        assert_eq!(v_seg, Ok(2));
    }

    #[test]
    fn test_compute_v_seg_exceeds_bounds() {
        // test_compute_v_seg_exceeds_bounds: Проверить возврат ошибки при дробной скорости
        // (например, при скорости 1.23, тике 1000 и сегменте 20.0).
        // speed = 1.23, tick = 1000, segment = 20.0 (voxel = 20.0, seg_voxels = 1)
        // speed_um_tick = 1.23 * 1000 = 1230.
        // segment_length_um = 20.0.
        // 1230 / 20.0 = 61.5 (fractional!)
        let res = compute_v_seg(1.23, 1000, 20.0, 1);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "v_seg is not an integer: Integer Physics constraint violated");

        // Test exceed limit (> 255)
        // speed = 300.0, tick = 1000, segment = 1.0 (voxel = 1.0, seg_voxels = 1)
        // v_seg = 300_000 / 1.0 = 300000 (exceeds 255)
        let res_large = compute_v_seg(300.0, 1000, 1.0, 1);
        assert!(res_large.is_err());
        assert_eq!(res_large.unwrap_err(), "v_seg exceeds maximum allowed value of 255");
    }

    #[test]
    fn test_dds_phase_u64_overflow() {
        // INV-CROSS-006: Intermediate 64-bit multiplication prevents overflow panic in debug mode
        let current_tick: u32 = u32::MAX;
        let heartbeat_m: u32 = 104729; // DDS_PRIME_SCATTER
        let phase = ((current_tick as u64) * (heartbeat_m as u64)) & 0xFFFF;
        assert_eq!(phase, 26343);
    }
}
