//! Fundamental mathematical and physical constants for AxiEngine physics.

/// Bit shift applied when transitioning from Mass Domain (`i32` synaptic weight)
/// to Charge Domain (`i32` electrical current `i_in = weight >> 16`).
pub const MASS_TO_CHARGE_SHIFT: u32 = 16;

/// Minimum absolute weight limit for a live synapse (Mass Floor Guard).
///
/// Ensures that synaptic depression never drops a live synapse's absolute weight
/// to 0, thereby preserving its biological sign (Dale's Law).
pub const MIN_WEIGHT_LIMIT: i32 = 1;

/// Maximum absolute weight limit for a synapse (Headroom Guard).
///
/// Prevents integer overflow before clamping by leaving an overflow buffer up to `i32::MAX`.
pub const MAX_WEIGHT_LIMIT: i32 = 2_140_000_000;

/// Bit shift for branchless O(1) calculation of inertia rank index (`abs_weight >> 28`).
pub const INERTIA_RANK_SHIFT: u32 = 28;

/// Upper boundary of inertia rank index (range `0..7`).
pub const MAX_INERTIA_RANK: usize = 7;

/// Probability scale and hash bucket modulus of Stochastic Heartbeat (16-bit limit).
pub const HEARTBEAT_PHASE_MOD: u64 = 65_536;

/// Bitwise mask for extracting 16-bit Stochastic Heartbeat hash value (`& 0xFFFF`).
pub const HEARTBEAT_PHASE_MASK: u64 = 0xFFFF;

/// Spatial pseudorandom multiplier prime for decorrelating spontaneous heartbeat phases by `tid`.
pub const HEARTBEAT_SCATTER_PRIME: u64 = 104_729;

/// Maximum phase step value for Stochastic Heartbeat (16-bit limit).
pub const MAX_HEARTBEAT_M: u32 = 65_535;

/// Recovery rate subtracted from dendrite fatigue timer on each tick (units per tick).
pub const FATIGUE_RECOVERY_RATE: u8 = 1;

/// Fatigue cost added to dendrite timer upon receiving a presynaptic active tail hit.
pub const FATIGUE_SPIKE_COST: u8 = 50;
