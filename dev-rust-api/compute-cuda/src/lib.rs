use compute_api::{ComputeApiError, GpuBackend, VramHandle, ShardLayout, DayBatchCmd, BatchResult, OutputFrame, TelemetryFrame, GhostPatch};
use layout::VariantParameters;
use slotmap::{SlotMap, Key};

unsafe extern "C" {
    fn cudaSetDevice(device: i32) -> i32;
    fn cudaStreamCreate(pStream: *mut *mut std::ffi::c_void) -> i32;
    fn cudaStreamDestroy(stream: *mut std::ffi::c_void) -> i32;
    
    fn launch_update_neurons(vram: layout::ShardVramPtrs, padded_n: u32, current_tick: u32, v_seg: u32, stream: *mut std::ffi::c_void) -> i32;
    fn launch_propagate_axons(vram: layout::ShardVramPtrs, padded_n: u32, v_seg: u32, stream: *mut std::ffi::c_void) -> i32;
    fn launch_apply_gsop(vram: layout::ShardVramPtrs, padded_n: u32, v_seg: u32, stream: *mut std::ffi::c_void) -> i32;
}

fn check_cuda(code: i32) -> Result<(), ComputeApiError> {
    match code {
        0 => Ok(()),
        2 => Err(ComputeApiError::OutOfMemory),
        3 => Err(ComputeApiError::DeviceLost),
        _ => Err(ComputeApiError::VendorError(code)),
    }
}

#[allow(dead_code)]
pub(crate) struct ShardCudaResources {
    pub vram_ptrs: layout::ShardVramPtrs,
    pub stream: *mut std::ffi::c_void,
    pub layout: ShardLayout,
}

unsafe impl Send for ShardCudaResources {}
unsafe impl Sync for ShardCudaResources {}

/// CudaBackend manages CUDA resources and execution for the node.
/// 
/// Invariants:
/// - INV-COMPUTE-CUDA-001 (Unique Context): CudaBackend owns a thread-safe registry of resources.
/// - INV-COMPUTE-CUDA-004 (Stream Context Isolation): Each shard receives its own CUDA stream, preventing serialization of computations across shards on the same GPU.
pub struct CudaBackend {
    pub device_id: i32,
    resources: std::sync::RwLock<SlotMap<slotmap::DefaultKey, ShardCudaResources>>,
}

impl CudaBackend {
    pub fn new(device_id: i32) -> Result<Self, ComputeApiError> {
        let code = unsafe { cudaSetDevice(device_id) };
        check_cuda(code)?;
        Ok(Self {
            device_id,
            resources: std::sync::RwLock::new(SlotMap::new()),
        })
    }
}

impl GpuBackend for CudaBackend {
    fn alloc_shard(&self, layout: &ShardLayout) -> Result<VramHandle, ComputeApiError> {
        if layout.padded_n % 64 != 0 {
            return Err(ComputeApiError::InvalidLayout);
        }
        
        let mut stream: *mut std::ffi::c_void = std::ptr::null_mut();
        let code = unsafe { cudaStreamCreate(&mut stream) };
        check_cuda(code)?;
        
        let vram_ptrs = unsafe { std::mem::zeroed() };
        
        let res = ShardCudaResources {
            vram_ptrs,
            stream,
            layout: layout.clone(),
        };
        
        let mut registry = self.resources.write().unwrap();
        let key = registry.insert(res);
        Ok(VramHandle(key.data().as_ffi()))
    }

    fn upload_state(&self, handle: &VramHandle, _state: &[u8]) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(())
    }

    fn upload_variants(&self, handle: &VramHandle, _variants: &[VariantParameters]) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(())
    }

    fn run_day_batch(&self, handle: &VramHandle, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        unsafe {
            check_cuda(launch_update_neurons(res.vram_ptrs, res.layout.padded_n, cmd.tick_base, cmd.v_seg, res.stream))?;
            check_cuda(launch_propagate_axons(res.vram_ptrs, res.layout.padded_n, cmd.v_seg, res.stream))?;
            check_cuda(launch_apply_gsop(res.vram_ptrs, res.layout.padded_n, cmd.v_seg, res.stream))?;
        }

        Ok(BatchResult { ticks_processed: 0, is_warmup: false })
    }

    fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(OutputFrame { data: Vec::new(), num_outputs: 0, sync_batch_ticks: 0 })
    }

    fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(TelemetryFrame { active_soma_ids: Vec::new(), total_spikes: 0 })
    }

    fn patch_ghosts(&self, handle: &VramHandle, _patches: &[GhostPatch]) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(())
    }

    fn run_sort_and_prune(&self, handle: &VramHandle, _prune_threshold: i16) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().unwrap();
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        if !registry.contains_key(key) {
            return Err(ComputeApiError::InvalidHandle);
        }
        Ok(())
    }

    fn free(&self, handle: VramHandle) {
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let mut registry = self.resources.write().unwrap();
        if let Some(res) = registry.remove(key) {
            unsafe {
                let _ = cudaStreamDestroy(res.stream);
            }
        }
    }
}