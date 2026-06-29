//! AOT and pre-bake parameter derivation functions.

use crate::constants::MAX_HEARTBEAT_M;
use crate::error::PhysicsError;

/// Calculates discrete axonal segment velocity `v_seg` (number of segments traversed per tick).
///
/// Implements pre-bake derivation strictly isolated to configuration/baking phase.
///
/// # Arguments
/// * `signal_speed_m_s` - Axonal propagation speed in meters per second.
/// * `tick_duration_us` - Simulation tick duration in microseconds.
/// * `voxel_size_um` - Spatial voxel grid resolution in micrometers.
/// * `segment_length_voxels` - Length of a single axonal segment measured in voxels.
///
/// # Errors
/// Returns [`PhysicsError`] if input parameters are non-positive, if the derived velocity has a
/// non-negligible fractional component (`epsilon > 1e-5`), or if `v_seg` falls outside `1..=255`.
pub fn compute_v_seg(
    signal_speed_m_s: f32,
    tick_duration_us: u32,
    voxel_size_um: f32,
    segment_length_voxels: u32,
) -> Result<u32, PhysicsError> {
    if signal_speed_m_s <= 0.0
        || voxel_size_um <= 0.0
        || tick_duration_us == 0
        || segment_length_voxels == 0
    {
        return Err(PhysicsError::InvalidPhysicalParameters);
    }

    let signal_speed_um_tick = signal_speed_m_s * (tick_duration_us as f32);
    let segment_length_um = voxel_size_um * (segment_length_voxels as f32);
    let v_seg_float = signal_speed_um_tick / segment_length_um;

    // Validate exact integer requirement
    let rounded = v_seg_float + 0.5;
    let rounded_floor = libm_floor(rounded);
    let diff = if v_seg_float >= rounded_floor {
        v_seg_float - rounded_floor
    } else {
        rounded_floor - v_seg_float
    };

    if diff > 1e-5 {
        return Err(PhysicsError::NonIntegerSegmentVelocity);
    }

    let v_seg = rounded_floor as u32;
    if !(1..=255).contains(&v_seg) {
        return Err(PhysicsError::SegmentVelocityOutOfBounds);
    }

    Ok(v_seg)
}

/// Helper function to perform floor without standard library dependencies.
fn libm_floor(x: f32) -> f32 {
    let i = x as i32;
    let i_f = i as f32;
    if x < i_f {
        i_f - 1.0
    } else {
        i_f
    }
}

/// Compiles the Direct Digital Synthesis (DDS) phase step parameter `heartbeat_m`.
///
/// # Arguments
/// * `period_ticks` - Desired spontaneous spiking period in simulation ticks.
///
/// # Details
/// - Returns `0` if `period_ticks == 0` or `period_ticks > 65536` (disabling heartbeat).
/// - Returns [`MAX_HEARTBEAT_M`] (65535) if `period_ticks == 1` (spiking every tick).
/// - Returns `min(65536 / period_ticks, 65535)` for `2 <= period_ticks <= 65536`.
pub fn compile_dds_heartbeat(period_ticks: u64) -> u32 {
    if period_ticks == 0 || period_ticks > 65536 {
        0
    } else if period_ticks == 1 {
        MAX_HEARTBEAT_M
    } else {
        let m = 65536 / period_ticks;
        if m > MAX_HEARTBEAT_M as u64 {
            MAX_HEARTBEAT_M
        } else {
            m as u32
        }
    }
}
