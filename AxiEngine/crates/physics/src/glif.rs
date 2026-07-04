//! Generalized Leaky Integrate-and-Fire (GLIF) membrane dynamics and Stochastic Heartbeat.

use crate::constants::{HEARTBEAT_PHASE_MASK, HEARTBEAT_SCATTER_PRIME, MAX_HEARTBEAT_M};

/// Evaluates whether a biological GLIF membrane potential crosses the effective spiking threshold.
///
/// Effective threshold is defined as `v_th + thresh_offset`.
#[inline]
pub fn is_glif_spike(voltage_new: i32, v_th: i32, thresh_offset: i32) -> bool {
    let v_th_eff = v_th.wrapping_add(thresh_offset);
    voltage_new >= v_th_eff
}

/// Computes a fast 64-bit deterministic integer hash for spatial/temporal pseudorandom generation.
#[inline]
pub fn stochastic_hash(current_tick: u64, tid: u32) -> u64 {
    let mut h = current_tick.wrapping_add((tid as u64).wrapping_mul(HEARTBEAT_SCATTER_PRIME));
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    h
}

/// Evaluates spontaneous Stochastic Heartbeat spike detection.
///
/// # Arguments
/// * `current_tick` - Current simulation tick (`u64`).
/// * `heartbeat_m` - Compiled Stochastic Heartbeat rate parameter (`u32`).
/// * `tid` - Unique thread/neuron ID (`u32`).
///
/// # Returns
/// - `true` every tick if `heartbeat_m == MAX_HEARTBEAT_M` (`period_ticks == 1`).
/// - `false` if `heartbeat_m == 0`.
/// - `(rnd < heartbeat_m)` otherwise, where `rnd` is calculated via deterministic integer hash (`stochastic_hash`).
///
/// Implements branchless boolean evaluation adhering strictly to `INV-PHYS-001`.
#[inline]
pub fn heartbeat_spike(current_tick: u64, heartbeat_m: u32, tid: u32) -> bool {
    let is_max = heartbeat_m == MAX_HEARTBEAT_M;
    let is_zero = heartbeat_m == 0;
    let rnd = stochastic_hash(current_tick, tid) & HEARTBEAT_PHASE_MASK;
    let is_hit = rnd < (heartbeat_m as u64);

    is_max || (!is_zero && is_hit)
}

/// Computes adaptive GLIF membrane potential integration and exponential leak decay.
///
/// Implements 100% panic-free arithmetic using 64-bit intermediate differences and wrapping arithmetic (INV-PHYS-004).
///
/// # Arguments
/// * `voltage` - Current membrane potential (`i32`).
/// * `i_in` - Total incoming electrical charge current (`i32`).
/// * `rest_potential` - Resting membrane potential ($V_{\text{rest}}$, `i32`).
/// * `thresh_offset` - Current adaptive homeostasis threshold offset (`i32`).
/// * `leak_shift` - Base exponential leak bit-shift (`i32`).
/// * `adaptive_leak_gain` - Adaptive leak scaling gain (`i32`).
/// * `adaptive_leak_min_shift` - Minimum allowed leak shift guard (`i32`).
/// * `adaptive_mode` - Adaptive leak enable flag (`0` or `1`).
///
/// # Returns
/// Updated membrane potential `voltage_new`.
#[allow(clippy::too_many_arguments)] // Takes raw scalar parameters matching physical specs without layout DTOs.
pub fn update_glif_voltage(
    voltage: i32,
    i_in: i32,
    rest_potential: i32,
    thresh_offset: i32,
    leak_shift: i32,
    adaptive_leak_gain: i32,
    adaptive_leak_min_shift: i32,
    adaptive_mode: i32,
) -> i32 {
    let adaptive_sub =
        ((thresh_offset as i64 * adaptive_leak_gain as i64) / 256) * adaptive_mode as i64;
    let current_shift = (leak_shift as i64 - adaptive_sub).max(adaptive_leak_min_shift as i64);
    let shift = current_shift.clamp(0, 63) as u32;

    let v_diff = (voltage as i64) - (rest_potential as i64);
    let delta_v_leak = (v_diff >> shift) as i32;

    voltage.wrapping_add(i_in).wrapping_sub(delta_v_leak)
}

/// Applies branchless homeostasis decay to the threshold offset every simulation tick.
///
/// Ensures non-negative clamping using bitwise operations (`decayed & !(decayed >> 31)`).
#[inline]
pub fn homeostasis_decay(thresh_offset: i32, homeostasis_decay_amount: i32) -> i32 {
    let decayed = thresh_offset.wrapping_sub(homeostasis_decay_amount);
    decayed & !(decayed >> 31)
}
