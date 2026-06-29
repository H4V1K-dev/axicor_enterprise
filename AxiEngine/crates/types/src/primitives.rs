//! Fundamental scalar integer aliases and domain newtypes.

/// Discrete simulation tick counter (monotonic time).
pub type Tick = u64;

/// Neuron soma membrane potential in microvolts (uV).
pub type Voltage = i32;

/// Synaptic weight in the Mass Domain.
/// INVARIANT: Strictly i32 to ensure signed Dale's Law mathematics.
pub type Weight = i32;

/// Propagation head position (axon segment index).
/// When inactive, contains AXON_SENTINEL (0x80000000).
pub type AxonHead = u32;

/// Segment index within an axon (generalized top-level outer container).
/// NOTE: Inside `PackedTarget`, segment offset is strictly limited to 8 bits (0..255).
pub type SegmentIndex = u32;

/// Neuron profile (variant) identifier within a shard (0..15).
pub type VariantId = u8;

/// Discrete coordinate of voxel grid (0..1023).
pub type VoxelCoord = u32;
