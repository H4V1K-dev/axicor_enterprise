use crate::error::ComputeApiError;
use crate::types::{VramHandle, ShardLayout, DayBatchCmd, BatchResult, OutputFrame, TelemetryFrame, GhostPatch};
use layout::VariantParameters;

/// Hardware Abstraction Layer (HAL) interface for compute engines (NVIDIA CUDA, AMD HIP, CPU fallback).
///
/// Under INV-COMPUTE-API-001, this trait must remain object-safe to allow dynamic dispatch using
/// `Box<dyn GpuBackend>` or `Arc<dyn GpuBackend>`. Thus, no method may introduce generic parameters.
pub trait GpuBackend: Send + Sync {
    /// Allocates memory on the accelerator device for a single simulation shard.
    ///
    /// Under INV-COMPUTE-API-006, this allocates unique segments and returns a [`VramHandle`].
    fn alloc_shard(&self, layout: &ShardLayout) -> Result<VramHandle, ComputeApiError>;

    /// Uploads Shard State into the device memory via Zero-Copy DMA transfer.
    fn upload_state(&self, handle: &VramHandle, state: &[u8]) -> Result<(), ComputeApiError>;

    /// Uploads GLIF/GSOP behavior variant parameters into Constant Memory / cache lines on the device.
    fn upload_variants(&self, handle: &VramHandle, variants: &[VariantParameters]) -> Result<(), ComputeApiError>;

    /// Asynchronously runs the hot execution cycle (Day Phase) for the given batch commands.
    ///
    /// Under INV-COMPUTE-API-004, the lifecycle of memory references passed inside the [`DayBatchCmd`]
    /// is statically checked by the compiler to prevent Use-After-Free during DMA copies.
    fn run_day_batch(&self, handle: &VramHandle, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError>;

    /// Asynchronously downloads motor commands (Soma Readout) from VRAM to host memory.
    fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError>;

    /// Downloads activity telemetry containing recorded spikes from accelerator memory.
    fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError>;

    /// Mutates inter-shard connections inside VRAM in O(1) time without triggering memory reallocation.
    fn patch_ghosts(&self, handle: &VramHandle, patches: &[GhostPatch]) -> Result<(), ComputeApiError>;

    /// Runs Segmented Radix Sort in VRAM to evict empty slots to the end of the routing array.
    fn run_sort_and_prune(&self, handle: &VramHandle, prune_threshold: i16) -> Result<(), ComputeApiError>;

    /// Explicitly frees the allocated resources identified by the VramHandle.
    ///
    /// Under INV-COMPUTE-API-003 (R-015), implicit cleanup using Rust's `Drop` trait is prohibited
    /// to avoid C-ABI teardown races at process exit. Hence, cleanup must be explicitly triggered.
    fn free(&self, handle: VramHandle);
}
