//! ShardEngine implementation, handling state transitions and dispatch.

use crate::error::ComputeError;
use crate::lifecycle::LifecycleState;
use crate::preference::BackendPreference;
use compute_api::{
    BackendCapabilities, BackendKind, BackendMaintenanceMut, BackendMaintenanceRef, BatchResult,
    ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload, VramHandle,
};

/// The primary executor facade representing a simulation execution shard.
pub struct ShardEngine {
    backend: Box<dyn ComputeBackend>,
    handle: Option<VramHandle>,
    state: LifecycleState,
    capabilities: BackendCapabilities,
    import_poisoned: bool,
    _marker: std::marker::PhantomData<std::rc::Rc<()>>,
}

#[cfg(feature = "cpu")]
fn try_create_cpu() -> Result<Box<dyn ComputeBackend>, ComputeError> {
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    let backend = CpuBackend::new(CpuBackendConfig::default())?;
    Ok(Box::new(backend))
}

#[cfg(not(feature = "cpu"))]
fn try_create_cpu() -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Err(ComputeError::FeatureNotCompiled { feature: "cpu" })
}

#[cfg(feature = "cuda")]
fn try_create_cuda(device_id: u32) -> Result<Box<dyn ComputeBackend>, ComputeError> {
    use compute_cuda::{CudaBackend, CudaBackendConfig};
    let config = CudaBackendConfig { device_id };
    match CudaBackend::new(config) {
        Ok(backend) => Ok(Box::new(backend)),
        Err(compute_api::ComputeApiError::UnsupportedBackend)
        | Err(compute_api::ComputeApiError::BackendNotInitialized) => {
            Err(ComputeError::BackendUnavailable {
                backend: BackendKind::Cuda,
                reason: "CUDA native provider is not available".to_string(),
            })
        }
        Err(e) => Err(ComputeError::ApiError(e)),
    }
}

#[cfg(not(feature = "cuda"))]
fn try_create_cuda(_device_id: u32) -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Err(ComputeError::FeatureNotCompiled { feature: "cuda" })
}

#[cfg(feature = "hip")]
fn try_create_hip(_device_id: u32) -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Err(ComputeError::BackendUnavailable {
        backend: BackendKind::Hip,
        reason: "HIP backend is not available in Stage 1".to_string(),
    })
}

#[cfg(not(feature = "hip"))]
fn try_create_hip(_device_id: u32) -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Err(ComputeError::FeatureNotCompiled { feature: "hip" })
}

#[cfg(feature = "mock")]
fn try_create_mock() -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Ok(Box::new(crate::mock::MockBackend::new()))
}

#[cfg(not(feature = "mock"))]
fn try_create_mock() -> Result<Box<dyn ComputeBackend>, ComputeError> {
    Err(ComputeError::FeatureNotCompiled { feature: "mock" })
}

fn try_auto() -> Result<Box<dyn ComputeBackend>, ComputeError> {
    // CUDA first
    match try_create_cuda(0) {
        Ok(b) => return Ok(b),
        Err(ComputeError::BackendUnavailable { .. })
        | Err(ComputeError::FeatureNotCompiled { .. }) => {}
        Err(e) => return Err(e),
    }

    // HIP second
    match try_create_hip(0) {
        Ok(b) => return Ok(b),
        Err(ComputeError::BackendUnavailable { .. })
        | Err(ComputeError::FeatureNotCompiled { .. }) => {}
        Err(e) => return Err(e),
    }

    // CPU third
    match try_create_cpu() {
        Ok(b) => return Ok(b),
        Err(ComputeError::BackendUnavailable { .. })
        | Err(ComputeError::FeatureNotCompiled { .. }) => {}
        Err(e) => return Err(e),
    }

    Err(ComputeError::NoBackendAvailable)
}

impl ShardEngine {
    /// Initializes the ShardEngine context and selects the backend.
    pub fn new(pref: BackendPreference) -> Result<Self, ComputeError> {
        let backend = match pref {
            BackendPreference::Auto => try_auto()?,
            BackendPreference::Cpu => try_create_cpu()?,
            BackendPreference::Cuda { device_id } => try_create_cuda(device_id)?,
            BackendPreference::Hip { device_id } => try_create_hip(device_id)?,
            BackendPreference::Mock => try_create_mock()?,
        };

        let capabilities = backend.capabilities();

        Ok(Self {
            backend,
            handle: None,
            state: LifecycleState::Created,
            capabilities,
            import_poisoned: false,
            _marker: std::marker::PhantomData,
        })
    }

    /// Allocates VRAM resources for a simulation shard.
    pub fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Created {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Created",
            });
        }
        let handle = self.backend.alloc_shard(spec)?;
        self.handle = Some(handle);
        self.state = LifecycleState::Allocated;
        Ok(())
    }

    /// Uploads initial binary state and axon tables into allocated VRAM.
    pub fn upload_shard(&mut self, upload: ShardUpload<'_>) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Allocated {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Allocated",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        self.backend.upload_shard(handle, upload)?;
        self.state = LifecycleState::Running;
        Ok(())
    }

    /// Executes a day batch of simulation ticks synchronously.
    pub fn run_day_batch(&mut self, cmd: DayBatchCmd<'_>) -> Result<BatchResult, ComputeError> {
        if self.state != LifecycleState::Running {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Running",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        let res = self.backend.run_day_batch(handle, cmd)?;
        Ok(res)
    }

    /// Delegates diagnostic full-state VRAM snapshot extraction.
    pub fn debug_snapshot(&mut self, snapshot: ShardSnapshotMut<'_>) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Running {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Running",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        self.backend.debug_snapshot(handle, snapshot)?;
        Ok(())
    }

    /// Puts the compute engine into Maintenance mode for Night Phase operations.
    pub fn enter_maintenance(&mut self) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Running {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Running",
            });
        }
        self.state = LifecycleState::Maintenance;
        Ok(())
    }

    /// Exports the current simulation state from VRAM to host memory buffers for maintenance.
    pub fn export_maintenance_state(
        &mut self,
        maintenance: BackendMaintenanceMut<'_>,
    ) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Maintenance {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Maintenance",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        self.backend.export_maintenance_state(handle, maintenance)?;
        Ok(())
    }

    /// Imports the updated simulation state from host memory buffers back into VRAM.
    pub fn import_maintenance_state(
        &mut self,
        maintenance: BackendMaintenanceRef<'_>,
    ) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Maintenance {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Maintenance",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        if let Err(e) = self.backend.import_maintenance_state(handle, maintenance) {
            self.import_poisoned = true;
            return Err(ComputeError::ApiError(e));
        }
        Ok(())
    }

    /// Exits maintenance mode and resumes simulation execution.
    pub fn exit_maintenance(&mut self) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Maintenance {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Maintenance",
            });
        }
        if self.import_poisoned {
            return Err(ComputeError::ImportPoisoned);
        }
        self.state = LifecycleState::Running;
        Ok(())
    }

    /// Frees resources allocated for this shard.
    pub fn free_shard(&mut self) -> Result<(), ComputeError> {
        if self.state != LifecycleState::Allocated
            && self.state != LifecycleState::Running
            && self.state != LifecycleState::Maintenance
        {
            return Err(ComputeError::InvalidLifecycleState {
                current: self.state,
                expected: "Allocated, Running, or Maintenance",
            });
        }
        let handle = self.handle.ok_or(ComputeError::InvalidLifecycleState {
            current: self.state,
            expected: "Some(VramHandle)",
        })?;
        self.backend.free_shard(handle)?;
        self.handle = None;
        self.import_poisoned = false; // Reset poison flag on free
        self.state = LifecycleState::Created;
        Ok(())
    }

    /// Explicitly tears down the backend instance. Idempotent.
    pub fn teardown(&mut self) -> Result<(), ComputeError> {
        if self.state == LifecycleState::TornDown {
            return Ok(());
        }
        self.backend.teardown()?;
        self.handle = None;
        self.import_poisoned = false; // Reset poison flag on teardown
        self.state = LifecycleState::TornDown;
        Ok(())
    }

    /// Helper bootstrapper constructing, allocating, and uploading in one step.
    pub fn bootstrap(
        pref: BackendPreference,
        spec: ShardAllocSpec,
        upload: ShardUpload<'_>,
    ) -> Result<Self, ComputeError> {
        let mut engine = Self::new(pref)?;
        engine.alloc_shard(spec)?;
        engine.upload_shard(upload)?;
        Ok(engine)
    }

    /// Returns the active backend kind.
    pub fn backend_kind(&self) -> BackendKind {
        self.backend.kind()
    }

    /// Returns capabilities of the selected backend.
    pub fn capabilities(&self) -> BackendCapabilities {
        self.capabilities.clone()
    }

    /// Returns the current VRAM allocation handle, if active.
    pub fn handle(&self) -> Option<VramHandle> {
        self.handle
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> LifecycleState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compute_api::{
        BackendCapabilities, BackendKind, BackendMaintenanceMut, BackendMaintenanceRef,
        BatchResult, ComputeApiError, ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardUpload,
        VramHandle,
    };
    use std::num::NonZeroU64;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct FailingBackend {
        fail_free: bool,
        fail_teardown: bool,
        fail_import: bool,
        dispatch_count: Arc<AtomicU32>,
    }

    impl ComputeBackend for FailingBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Mock
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

        fn alloc_shard(&mut self, _spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
            Ok(VramHandle::from_raw_parts(
                BackendKind::Mock,
                NonZeroU64::new(1).unwrap(),
                1,
            ))
        }

        fn upload_shard(
            &mut self,
            _handle: VramHandle,
            _upload: ShardUpload<'_>,
        ) -> Result<(), ComputeApiError> {
            Ok(())
        }

        fn run_day_batch(
            &mut self,
            _handle: VramHandle,
            _cmd: DayBatchCmd<'_>,
        ) -> Result<BatchResult, ComputeApiError> {
            self.dispatch_count.fetch_add(1, Ordering::SeqCst);
            Ok(BatchResult {
                ticks_executed: 1,
                generated_spikes_count: 0,
                output_spikes_written: 0,
                dropped_spikes_count: 0,
                execution_time_us: 1,
            })
        }

        fn free_shard(&mut self, _handle: VramHandle) -> Result<(), ComputeApiError> {
            if self.fail_free {
                Err(ComputeApiError::DeviceLost)
            } else {
                Ok(())
            }
        }

        fn teardown(&mut self) -> Result<(), ComputeApiError> {
            if self.fail_teardown {
                Err(ComputeApiError::DeviceLost)
            } else {
                Ok(())
            }
        }

        fn export_maintenance_state(
            &mut self,
            _handle: VramHandle,
            _maintenance: BackendMaintenanceMut<'_>,
        ) -> Result<(), ComputeApiError> {
            Ok(())
        }

        fn import_maintenance_state(
            &mut self,
            _handle: VramHandle,
            _maintenance: BackendMaintenanceRef<'_>,
        ) -> Result<(), ComputeApiError> {
            if self.fail_import {
                Err(ComputeApiError::DeviceLost)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_free_shard_preserves_state_on_error() {
        let dispatch = Arc::new(AtomicU32::new(0));
        let backend = Box::new(FailingBackend {
            fail_free: true,
            fail_teardown: false,
            fail_import: false,
            dispatch_count: dispatch,
        });

        let mut engine = ShardEngine {
            backend,
            handle: Some(VramHandle::from_raw_parts(
                BackendKind::Mock,
                NonZeroU64::new(42).unwrap(),
                1,
            )),
            state: LifecycleState::Running,
            capabilities: BackendCapabilities {
                lane_count: 1,
                supports_async: false,
                supports_ephys: false,
                max_batch_ticks: 1000,
                alignment_bytes: 64,
                pinned_host_required: false,
            },
            import_poisoned: false,
            _marker: std::marker::PhantomData,
        };

        let res = engine.free_shard();
        assert!(res.is_err());
        assert_eq!(engine.state(), LifecycleState::Running);
        assert!(engine.handle().is_some());
    }

    #[test]
    fn test_teardown_preserves_state_on_error() {
        let dispatch = Arc::new(AtomicU32::new(0));
        let backend = Box::new(FailingBackend {
            fail_free: false,
            fail_teardown: true,
            fail_import: false,
            dispatch_count: dispatch,
        });

        let mut engine = ShardEngine {
            backend,
            handle: Some(VramHandle::from_raw_parts(
                BackendKind::Mock,
                NonZeroU64::new(42).unwrap(),
                1,
            )),
            state: LifecycleState::Allocated,
            capabilities: BackendCapabilities {
                lane_count: 1,
                supports_async: false,
                supports_ephys: false,
                max_batch_ticks: 1000,
                alignment_bytes: 64,
                pinned_host_required: false,
            },
            import_poisoned: false,
            _marker: std::marker::PhantomData,
        };

        let res = engine.teardown();
        assert!(res.is_err());
        assert_eq!(engine.state(), LifecycleState::Allocated);
        assert!(engine.handle().is_some());
    }

    #[test]
    fn test_single_dispatch_count() {
        let dispatch = Arc::new(AtomicU32::new(0));
        let backend = Box::new(FailingBackend {
            fail_free: false,
            fail_teardown: false,
            fail_import: false,
            dispatch_count: dispatch.clone(),
        });

        let mut engine = ShardEngine {
            backend,
            handle: Some(VramHandle::from_raw_parts(
                BackendKind::Mock,
                NonZeroU64::new(42).unwrap(),
                1,
            )),
            state: LifecycleState::Running,
            capabilities: BackendCapabilities {
                lane_count: 1,
                supports_async: false,
                supports_ephys: false,
                max_batch_ticks: 1000,
                alignment_bytes: 64,
                pinned_host_required: false,
            },
            import_poisoned: false,
            _marker: std::marker::PhantomData,
        };

        let mut output_spikes = [0u32; 10];
        let mut output_spike_counts = [0u32; 1];
        let cmd = DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 1,
            v_seg: 1,
            dopamine: 0,
            input_words_per_tick: 0,
            max_spikes_per_tick: 1,
            num_outputs: 0,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &[0],
            mapped_soma_ids: &[],
            output_spikes: &mut output_spikes,
            output_spike_counts: &mut output_spike_counts,
        };

        engine.run_day_batch(cmd).unwrap();
        assert_eq!(dispatch.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_maintenance_import_poisoning() {
        let dispatch = Arc::new(AtomicU32::new(0));
        let backend = Box::new(FailingBackend {
            fail_free: false,
            fail_teardown: false,
            fail_import: true,
            dispatch_count: dispatch,
        });

        let mut engine = ShardEngine {
            backend,
            handle: Some(VramHandle::from_raw_parts(
                BackendKind::Mock,
                NonZeroU64::new(42).unwrap(),
                1,
            )),
            state: LifecycleState::Maintenance,
            capabilities: BackendCapabilities {
                lane_count: 1,
                supports_async: false,
                supports_ephys: false,
                max_batch_ticks: 1000,
                alignment_bytes: 64,
                pinned_host_required: false,
            },
            import_poisoned: false,
            _marker: std::marker::PhantomData,
        };

        let mut state_buf = vec![0u8; 10];
        let mut axons_buf = vec![0u8; 10];
        let maint_ref = BackendMaintenanceRef {
            state_blob: &mut state_buf,
            axons_blob: &mut axons_buf,
        };

        assert!(engine.import_maintenance_state(maint_ref).is_err());
        assert!(engine.import_poisoned);
        assert_eq!(engine.exit_maintenance(), Err(ComputeError::ImportPoisoned));

        // free_shard should reset import_poisoned flag
        let res = engine.free_shard();
        assert!(res.is_ok());
        assert!(!engine.import_poisoned);
    }
}
