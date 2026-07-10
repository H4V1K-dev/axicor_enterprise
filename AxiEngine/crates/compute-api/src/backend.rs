//! Hardware abstraction layer (HAL) trait definition for compute execution backends.

use crate::capabilities::BackendCapabilities;
use crate::dto::{
    BackendMaintenanceMut, BackendMaintenanceRef, BatchResult, DayBatchCmd, ShardAllocSpec,
    ShardSnapshotMut, ShardUpload,
};
use crate::error::ComputeApiError;
use crate::handle::VramHandle;
use crate::kind::BackendKind;

/// Hardware-independent contract implemented by simulation compute backends.
pub trait ComputeBackend {
    /// Returns the backend kind identifier.
    fn kind(&self) -> BackendKind;

    /// Returns the execution capability descriptor for this backend.
    fn capabilities(&self) -> BackendCapabilities;

    /// Allocates VRAM resources for a simulation shard according to the specification.
    fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError>;

    /// Uploads initial binary state and axon tables into allocated VRAM.
    fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError>;

    /// Executes a day batch of simulation ticks synchronously.
    fn run_day_batch(
        &mut self,
        handle: VramHandle,
        cmd: DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError>;

    /// Deallocates VRAM resources associated with the specified handle.
    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError>;

    /// Tears down the backend instance and releases all associated hardware contexts.
    fn teardown(&mut self) -> Result<(), ComputeApiError>;

    /// Diagnostic method for full-state VRAM snapshot extraction in test harnesses.
    ///
    /// Default implementation returns `Err(ComputeApiError::UnsupportedFeature)`.
    fn debug_snapshot(
        &mut self,
        _handle: VramHandle,
        _snapshot: ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }

    /// Exports the current simulation state from VRAM to host memory buffers for maintenance.
    ///
    /// Default implementation returns `Err(ComputeApiError::UnsupportedFeature)`.
    fn export_maintenance_state(
        &mut self,
        _handle: VramHandle,
        _maintenance: BackendMaintenanceMut<'_>,
    ) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }

    /// Imports the updated simulation state from host memory buffers back into VRAM.
    ///
    /// Default implementation returns `Err(ComputeApiError::UnsupportedFeature)`.
    fn import_maintenance_state(
        &mut self,
        _handle: VramHandle,
        _maintenance: BackendMaintenanceRef<'_>,
    ) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }
}
