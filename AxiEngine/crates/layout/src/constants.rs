//! Fundamental limits, memory alignment quants, and binary magic identifiers for AxiEngine layout contracts.

/// Maximum number of dendrite slots (columns) per neuron soma.
pub const MAX_DENDRITES: usize = 128;

/// Maximum number of 3D segments in geometry per axon.
pub const MAX_SEGMENTS_PER_AXON: usize = 256;

/// Processor/GPU cache line alignment quantum in bytes (64 bytes).
pub const CACHE_LINE_BYTES: usize = 64;

/// NVIDIA CUDA execution warp size (32 threads).
pub const CUDA_WARP_LANES: usize = 32;

/// AMD ROCm/HIP execution wavefront size (64 threads).
pub const HIP_WAVE_LANES: usize = 64;

/// Neutral alignment quantum for neuron count `padded_n` (64 bytes).
pub const PADDED_N_ALIGNMENT: usize = 64;

/// Number of available neuron parameter profile variations in the LUT table.
pub const VARIANT_LUT_LEN: usize = 16;

/// Binary magic identifier for state dump files (`.state`).
pub const STATE_MAGIC: [u8; 4] = *b"AXST";

/// Binary magic identifier for axon burst files (`.axons`).
pub const AXONS_MAGIC: [u8; 4] = *b"AXAX";

/// Binary magic identifier for path geometry files (`.paths`).
pub const PATHS_MAGIC: [u8; 4] = *b"AXPT";

/// Format version identifier for state dump files (`.state`).
pub const STATE_FILE_VERSION: u32 = 1;

/// Format version identifier for axon burst files (`.axons`).
pub const AXONS_FILE_VERSION: u32 = 1;

/// Format version identifier for path geometry files (`.paths`).
pub const PATHS_FILE_VERSION: u32 = 1;
