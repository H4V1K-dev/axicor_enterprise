use compute_api::{GpuBackend, VramHandle, ShardLayout, DayBatchCmd, BatchResult, OutputFrame, TelemetryFrame, GhostPatch, ComputeApiError};
use layout::VariantParameters;

/// Orchestrator for a compute shard using a stateless facade pattern.
///
/// Under INV-COMPUTE-001, this structure encapsulates the dynamic backend implementation
/// within a `Box<dyn GpuBackend>`, allowing dynamic platform dispatch.
///
/// Under INV-COMPUTE-004, the facade is strictly stateless and must not store counters,
/// timers, or debug states. It only holds the dynamic backend dispatch box, the allocated
/// memory handle, the layout, and a teardown tracking flag.
pub struct ShardEngine {
    backend: Box<dyn GpuBackend>,
    handle: VramHandle,
    layout: ShardLayout,
    is_teared_down: bool,
}

impl ShardEngine {
    /// Initializes a new `ShardEngine`, allocating the memory handle for the given layout.
    pub fn new(backend: Box<dyn GpuBackend>, layout: ShardLayout) -> Result<Self, ComputeApiError> {
        let handle = backend.alloc_shard(&layout)?;
        Ok(Self {
            backend,
            handle,
            layout,
            is_teared_down: false,
        })
    }

    /// Uploads the initial state of the shard to the accelerator.
    pub fn upload_state(&self, state: &[u8]) -> Result<(), ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.upload_state(&self.handle, state)
    }

    /// Uploads 64-byte neuron profiles into Constant Memory (L1 Cache).
    pub fn upload_variants(&self, variants: &[VariantParameters]) -> Result<(), ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.upload_variants(&self.handle, variants)
    }

    /// Runs a simulation batch epoch (Day Phase).
    ///
    /// Under INV-COMPUTE-005, the dynamic dispatch call occurs once per batch (zero-cost dispatch)
    /// to preserve instruction cache locality and performance on the hot path.
    pub fn run_day_batch(&self, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.run_day_batch(&self.handle, cmd)
    }

    /// Downloads the calculated output frame of motor commands.
    pub fn download_output(&self) -> Result<OutputFrame, ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.download_output(&self.handle)
    }

    /// Downloads the active neuron telemetry frame.
    pub fn download_telemetry(&self) -> Result<TelemetryFrame, ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.download_telemetry(&self.handle)
    }

    /// Performs hot patching of inter-shard connection paths in VRAM.
    pub fn patch_ghosts(&self, patches: &[GhostPatch]) -> Result<(), ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.patch_ghosts(&self.handle, patches)
    }

    /// Initiates segmented radix sort to defragment dendritic slots.
    pub fn run_sort_and_prune(&self, prune_threshold: i16) -> Result<(), ComputeApiError> {
        debug_assert!(!self.is_teared_down);
        self.backend.run_sort_and_prune(&self.handle, prune_threshold)
    }

    /// Explicitly tears down the engine context, freeing the memory handle.
    ///
    /// E-067: Multiple teardown calls are safely ignored and return `Ok(())`.
    pub fn teardown(&mut self) -> Result<(), ComputeApiError> {
        if self.is_teared_down {
            return Ok(());
        }
        self.backend.free(self.handle);
        self.is_teared_down = true;
        Ok(())
    }

    /// Checks if the engine context has been torn down.
    pub fn is_teared_down(&self) -> bool {
        self.is_teared_down
    }

    /// Returns the allocated memory handle.
    pub fn handle(&self) -> VramHandle {
        self.handle
    }

    /// Returns the geometric layout of the shard.
    pub fn layout(&self) -> &ShardLayout {
        &self.layout
    }
}

impl Drop for ShardEngine {
    /// Cleans up allocated VRAM resources if explicit teardown was not called.
    ///
    /// Under R-023, this fallback catches and suppresses any C-FFI / driver-level panics
    /// (silent drop) to prevent a double-panic abort when the process aborts or GPU driver is killed.
    fn drop(&mut self) {
        if !self.is_teared_down {
            let backend = &*self.backend;
            let handle = self.handle;
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                backend.free(handle);
            }));
            self.is_teared_down = true;
        }
    }
}
