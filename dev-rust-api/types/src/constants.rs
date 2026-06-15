//! Static constants for memory layouts, limits, and safety sentinels.

// =============================================================================
// Memory & Layout Limits
// =============================================================================

/// Fundamental maximum limit of dendrites attached to a single soma.
///
/// Used to maintain the 1166-byte layout invariant and cache-line alignment.
pub const MAX_DENDRITES: usize = 128;

/// Hardware warp size of execution threads on target compute architectures.
///
/// Serves as the base unit for thread and memory-alignment calculations.
pub const WARP_SIZE: usize = 32;

/// Maximum number of segments allowed in a single axon's path.
///
/// Bounded by the 8-bit segment offset representation in packed targets.
pub const MAX_SEGMENTS_PER_AXON: usize = 256;

/// Bitmask to isolate the 24-bit axon identifier from a packed target representation.
pub const TARGET_AXON_MASK: u32 = 0x00FFFFFF;

/// Shift count to extract the 8-bit segment offset from a packed target representation.
pub const TARGET_SEG_SHIFT: u32 = 24;

/// Active C-ABI version of shared memory interfaces for IPC synchronization.
pub const SHM_VERSION: u8 = 4;

// =============================================================================
// Sentinels & Guards
// =============================================================================

/// Marker representing an inactive or quiescent axon head.
///
/// Placed to avoid temporal overflows and out-of-bounds propagation.
pub const AXON_SENTINEL: u32 = 0x80000000;

/// Safety boundary to prevent fast-moving signals from overstepping the sentinel.
///
/// Axon heads with index below this value are safe from garbage-collection reclamation.
pub const SENTINEL_DANGER_THRESHOLD: u32 = 0x70000000;

/// Empty pixel indicator in mapped soma identification buffers.
///
/// Signals an immediate early-exit in I/O compute kernels.
pub const EMPTY_PIXEL: u32 = 0xFFFF_FFFF;
