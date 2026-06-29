//! Hardware backend execution capability descriptor.

/// Describes the execution parameters and hardware limits of a compute backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    /// Execution lane or warp width (e.g. 32 for CUDA, 64 for HIP).
    pub lane_count: u32,
    /// Whether asynchronous batch execution is supported.
    pub supports_async: bool,
    /// Whether electrophysiology debug telemetry probes are supported.
    pub supports_ephys: bool,
    /// Maximum number of simulation ticks per day batch.
    pub max_batch_ticks: u32,
    /// Required memory alignment quantum in bytes.
    pub alignment_bytes: usize,
    /// Whether host memory buffers must be pinned (page-locked) for DMA transfers.
    pub pinned_host_required: bool,
}
