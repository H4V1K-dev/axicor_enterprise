//! Type aliases representing domain primitives for the Axicor engine.

/// A discrete simulation tick.
///
/// Used as a monotonic temporal quantum to index simulation progress and coordinate events
/// across the distributed simulation cluster.
pub type Tick = u64;

/// Synaptic weight in the mass domain.
///
/// Mass Domain representation limited to ±2.14B to prevent overflow before branchless clamp
/// in the Dale's Law calculations within the hot execution loops.
pub type Weight = i32;

/// Membrane potential of a soma (represented in microvolts).
///
/// Using raw microvolts prevents floating-point inaccuracies and allows uniform
/// integer-based membrane dynamics computation.
pub type Voltage = i32;

/// A contiguous zero-based index used by the GPU to reference SoA (Structure of Arrays) state.
///
/// Essential for coalesced memory access patterns on compute devices.
pub type DenseIndex = u32;

/// Index of an axon segment serving as the active signal front (head).
///
/// Set to `AXON_SENTINEL` (0x80000000) when the axon is quiescent to prevent
/// out-of-bounds execution and false triggers in hot physics loops.
pub type AxonHead = u32;

/// Zero-based local index of a segment within an individual axon's spatial path.
///
/// Constrained by the layout specification to a range of 0..255.
pub type SegmentIndex = u32;

/// Identifier for a neuron's behavioral variant profile in the look-up table (LUT).
///
/// Strictly bounded to 0..15 to allow safe, branchless indexing in GPU memory.
pub type VariantId = u8;

/// A discrete coordinate within the uniform spatial voxel grid.
///
/// Used for spatial hashing and proximity calculations.
pub type VoxelCoord = u32;

/// Absolute spatial unit measured in micrometers (1.0 = 1 μm).
///
/// # WARNING
/// AOT and configuration only. Float math is fatal in hot loops.
pub type Microns = f32;

/// Normalized coordinate or ratio bounded within [0.0, 1.0].
///
/// # WARNING
/// AOT and configuration only. Float math is fatal in hot loops.
pub type Fraction = f32;
