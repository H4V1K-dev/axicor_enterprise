//! Fundamental limits, sentinel values, and binary masks for AxiEngine.

/// Propagation head inactivity marker (0x80000000). Owner: types.
pub const AXON_SENTINEL: u32 = 0x8000_0000;

/// Hard marker for inactive/pruned synaptic dendrite slot (Pruned Tombstone, 0xFFFFFFFF). Owner: types.
pub const EMPTY_PIXEL: u32 = 0xFFFF_FFFF;

/// Bitmask to extract Axon_ID (24 bits) from raw packed target. Owner: types.
pub const TARGET_AXON_MASK: u32 = 0x00FF_FFFF;

/// Bit shift to extract Segment_Offset from raw packed target. Owner: types.
pub const TARGET_SEG_SHIFT: u32 = 24;

/// Default simulation seed string. Owner: types.
pub const DEFAULT_MASTER_SEED: &str = "AXICOR";

/// Hardware limit for neuron profile variant indices (4 bits, 0..15). Owner: types.
pub const MAX_TYPE_ID: u8 = 15;

/// Hardware limit for X coordinate in `PackedPosition` (10 bits, 0..1023). Owner: types.
pub const MAX_VOXEL_X: u32 = 1023;

/// Hardware limit for Y coordinate in `PackedPosition` (10 bits, 0..1023). Owner: types.
pub const MAX_VOXEL_Y: u32 = 1023;

/// Hardware limit for Z coordinate in `PackedPosition` (8 bits, 0..255). Owner: types.
pub const MAX_VOXEL_Z: u32 = 255;

/// Maximum axon ID considering +1 offset and `EMPTY_PIXEL` reservation (16_777_213 / 0x00FF_FFFD). Owner: types.
pub const MAX_AXON_ID: u32 = 16_777_213;

/// Maximum segment offset in `PackedTarget` (8 bits, 0..255). Owner: types.
pub const MAX_SEGMENT_OFFSET: u32 = 255;
