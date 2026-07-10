//! Layer 3 CPU compute backend implementation for `AxiEngine`.
//!
//! This crate implements the [`ComputeBackend`] trait for host processors using an isolated [`rayon::ThreadPool`].

mod aligned_mem;
mod config;
mod resource;
mod simulation;

pub use config::CpuBackendConfig;

use compute_api::{
    validation, BackendCapabilities, BackendKind, BackendMaintenanceMut, BackendMaintenanceRef,
    BatchResult, ComputeApiError, ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut,
    ShardUpload, VramHandle,
};
use resource::ResourceRegistry;
use std::sync::Mutex;

/// CPU-based simulation compute backend implementing [`ComputeBackend`].
pub struct CpuBackend {
    config: CpuBackendConfig,
    pool: rayon::ThreadPool,
    registry: Mutex<ResourceRegistry>,
}

impl CpuBackend {
    /// Constructs a new [`CpuBackend`] instance with the provided configuration.
    ///
    /// # Errors
    /// Returns [`ComputeApiError::BackendNotInitialized`] if thread pool construction fails.
    pub fn new(config: CpuBackendConfig) -> Result<Self, ComputeApiError> {
        let mut builder = rayon::ThreadPoolBuilder::new();
        if let Some(count) = config.thread_count {
            if count > 0 {
                builder = builder.num_threads(count);
            }
        }
        let pool = builder
            .build()
            .map_err(|_| ComputeApiError::BackendNotInitialized)?;

        Ok(Self {
            config,
            pool,
            registry: Mutex::new(ResourceRegistry::default()),
        })
    }

    /// Returns a reference to the backend configuration.
    pub fn config(&self) -> &CpuBackendConfig {
        &self.config
    }
}

impl ComputeBackend for CpuBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Cpu
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            lane_count: 1,
            supports_async: false,
            supports_ephys: false,
            max_batch_ticks: 1000,
            alignment_bytes: 64,
            pinned_host_required: false,
        }
    }

    fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        registry.alloc_shard(spec)
    }

    fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        registry.upload_shard(handle, upload)
    }

    fn run_day_batch(
        &mut self,
        handle: VramHandle,
        cmd: DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = registry.get_resource_mut(handle)?;
        simulation::run_day_batch(resource, cmd, &self.pool)
    }

    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        registry.free_shard(handle)
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        registry.teardown()
    }

    fn debug_snapshot(
        &mut self,
        handle: VramHandle,
        snapshot: ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::InvalidDebugProbeBounds);
        }

        validation::validate_snapshot_buffers(&resource.spec, &snapshot)?;

        snapshot
            .state_blob
            .copy_from_slice(resource.state_blob.as_slice());
        snapshot
            .axons_blob
            .copy_from_slice(resource.axons_blob.as_slice());

        Ok(())
    }

    fn export_maintenance_state(
        &mut self,
        handle: VramHandle,
        maintenance: BackendMaintenanceMut<'_>,
    ) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::InvalidDebugProbeBounds);
        }

        validation::validate_maintenance_export(&resource.spec, &maintenance)?;

        maintenance
            .state_blob
            .copy_from_slice(resource.state_blob.as_slice());
        maintenance
            .axons_blob
            .copy_from_slice(resource.axons_blob.as_slice());

        Ok(())
    }

    fn import_maintenance_state(
        &mut self,
        handle: VramHandle,
        maintenance: BackendMaintenanceRef<'_>,
    ) -> Result<(), ComputeApiError> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::InvalidDebugProbeBounds);
        }

        validation::validate_maintenance_import(&resource.spec, &maintenance)?;

        resource
            .state_blob
            .as_slice_mut()
            .copy_from_slice(maintenance.state_blob);
        resource
            .axons_blob
            .as_slice_mut()
            .copy_from_slice(maintenance.axons_blob);

        Ok(())
    }
}
