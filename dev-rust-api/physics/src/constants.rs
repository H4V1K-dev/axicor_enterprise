//! Fundamental algebraic and physics constants for the Axicor engine.

/// Maximum synapse weight limit in the high-precision Mass Domain.
///
/// Under invariant `INV-PHYS-010`, setting this limit to `2,140,000,000` leaves a safety
/// headroom of approximately 7.4 million units before `i32::MAX`. This prevents signed
/// integer overflow in Rust during intermediate weight modifications (e.g., `weight + delta`)
/// before the branchless clamping operator is applied.
pub const MAX_WEIGHT_LIMIT: i32 = 2_140_000_000;

/// Resolution of the phase accumulator (16-bit resolution) for the DDS Heartbeat.
///
/// Used in Ahead-Of-Time (AOT) calculation of the heartbeat phase increment:
/// `65536 / period_ticks`.
pub const DDS_PHASE_RESOLUTION: u32 = 65536;

/// A prime number multiplier used in the DDS Heartbeat phase accumulator.
///
/// Multiplies the thread index (`tid * 104729`) to achieve Spatial Scattering, preventing
/// synchronous spiking of neighboring threads within a single GPU warp and distributing
/// compute load evenly across cores.
pub const DDS_PRIME_SCATTER: u32 = 104729;

/// Bitwise arithmetic shift count (`>> 16`) to convert synapse weight from Mass Domain to Charge Domain.
///
/// In the Integer Physics paradigm, this replaces FPU division with a high-performance bitwise
/// shift, translating the cumulative structural synapse weight into discrete membrane voltage
/// units (microvolts) and modeling biological "Silent Synapses" for values below the threshold.
pub const MASS_TO_CHARGE_SHIFT: u32 = 16;

/// Fixed-point scaling bitwise shift (`>> 7`, equivalent to division by 128) for STDP plasticity calculations.
///
/// Used for fixed-point arithmetic scaling in GSOP dopamine modulation, STDP learning curves, and
/// synapse inertia calculations, replacing float division in the Integer Physics paradigm.
pub const GSOP_FIXED_POINT_SHIFT: u32 = 7;

/// Bitwise shift count (`>> 28`) to map weights to the synapse inertia rank.
///
/// Compresses the entire weight range (`0` to `2.14B`) into a 3-bit rank index (`0..7`)
/// at zero cost (O(1)), replacing floating-point log-scaling or branch-based clamping
/// for constant-time lookups in the `inertia_curve` array.
pub const INERTIA_RANK_SHIFT: u32 = 28;

/// Fixed-point scaling bitwise shift (`>> 8`, equivalent to division by 256) for homeostasis leak calculations.
///
/// Normalizes the `adaptive_leak_gain` factor against threshold offsets during GLIF leakage
/// calculation, replacing division in the Integer Physics fixed-point arithmetic system.
pub const ADAPTIVE_LEAK_SHIFT: u32 = 8;
