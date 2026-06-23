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

/// Converts milliseconds to ticks given a specific tick duration in microseconds.
#[inline]
pub fn ms_to_ticks(ms: f32, tick_duration_us: u32) -> Tick {
    let us = ms * 1000.0;
    ((us / tick_duration_us as f32) + 0.5) as Tick
}

/// Converts microseconds to ticks given a specific tick duration in microseconds.
#[inline]
pub fn us_to_ticks(us: u32, tick_duration_us: u32) -> Tick {
    (us / tick_duration_us) as Tick
}

/// Converts ticks to milliseconds given a specific tick duration in microseconds.
#[inline]
pub fn ticks_to_ms(ticks: Tick, tick_duration_us: u32) -> f32 {
    ticks as f32 * tick_duration_us as f32 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversions() {
        // ms_to_ticks: 5.0 ms with tick_duration 100us -> 50 ticks
        assert_eq!(ms_to_ticks(5.0, 100), 50);

        // us_to_ticks: 500 us with tick_duration 100us -> 5 ticks
        assert_eq!(us_to_ticks(500, 100), 5);

        // ticks_to_ms: 50 ticks with tick_duration 100us -> 5.0 ms
        assert_eq!(ticks_to_ms(50, 100), 5.0);
    }
}

