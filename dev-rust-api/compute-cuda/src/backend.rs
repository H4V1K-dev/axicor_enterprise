use compute_api::{
    BatchResult, ComputeApiError, DayBatchCmd, GhostPatch, GpuBackend, OutputFrame,
    ShardLayout, TelemetryFrame, VramHandle,
};
use layout::VariantParameters;
use slotmap::{SlotMap, Key};
use std::sync::atomic::{AtomicU32, Ordering};
use crate::ffi::*;

fn check_cuda(code: i32) -> Result<(), ComputeApiError> {
    match code {
        0 => Ok(()),
        2 => Err(ComputeApiError::OutOfMemory),
        3 => Err(ComputeApiError::DeviceLost),
        1 | 701 => Err(ComputeApiError::InvalidLayout), // InvalidValue / LaunchOutOfResources
        400 => Err(ComputeApiError::InvalidHandle),     // InvalidResourceHandle
        _ => Err(ComputeApiError::VendorError(code)),
    }
}

pub(crate) struct ShardCudaResources {
    pub raw_state_ptr: *mut std::ffi::c_void,
    pub raw_axons_ptr: *mut std::ffi::c_void,
    pub raw_variants_ptr: *mut std::ffi::c_void,
    pub vram_ptrs: layout::ShardVramPtrs,
    pub stream: *mut std::ffi::c_void,
    pub layout: ShardLayout,
    
    // Pinned RAM host buffers
    pub pinned_output_ptr: *mut std::ffi::c_void,
    pub pinned_output_size: usize,
    pub pinned_telemetry_ids_ptr: *mut std::ffi::c_void,
    pub pinned_telemetry_count_ptr: *mut std::ffi::c_void,

    // Last execution parameters (needed for output download)
    pub last_num_outputs: AtomicU32,
    pub last_sync_batch_ticks: AtomicU32,
}

unsafe impl Send for ShardCudaResources {}
unsafe impl Sync for ShardCudaResources {}

impl Drop for ShardCudaResources {
    fn drop(&mut self) {
        unsafe {
            if !self.raw_state_ptr.is_null() {
                let _ = cudaFree(self.raw_state_ptr);
            }
            if !self.raw_axons_ptr.is_null() {
                let _ = cudaFree(self.raw_axons_ptr);
            }
            if !self.raw_variants_ptr.is_null() {
                let _ = cudaFree(self.raw_variants_ptr);
            }
            if !self.pinned_output_ptr.is_null() {
                let _ = cudaFreeHost(self.pinned_output_ptr);
            }
            if !self.pinned_telemetry_ids_ptr.is_null() {
                let _ = cudaFreeHost(self.pinned_telemetry_ids_ptr);
            }
            if !self.pinned_telemetry_count_ptr.is_null() {
                let _ = cudaFreeHost(self.pinned_telemetry_count_ptr);
            }
            if !self.stream.is_null() {
                let _ = cudaStreamDestroy(self.stream);
            }
        }
    }
}

/// CudaBackend manages CUDA resources and execution for the node.
pub struct CudaBackend {
    pub device_id: i32,
    pub(crate) resources: std::sync::RwLock<SlotMap<slotmap::DefaultKey, ShardCudaResources>>,
}

impl CudaBackend {
    /// Initializes a new CUDA backend for the specified device ID.
    pub fn new(device_id: i32) -> Result<Self, ComputeApiError> {
        let code = unsafe { cudaSetDevice(device_id) };
        check_cuda(code)?;

        let mut major = 0;
        let mut minor = 0;
        let mut warp_size = 0;

        let code = unsafe {
            cudaDeviceGetAttribute(
                &mut major,
                CUDA_DEV_ATTR_COMPUTE_CAPABILITY_MAJOR,
                device_id,
            )
        };
        check_cuda(code)?;
        
        let code = unsafe {
            cudaDeviceGetAttribute(
                &mut minor,
                CUDA_DEV_ATTR_COMPUTE_CAPABILITY_MINOR,
                device_id,
            )
        };
        check_cuda(code)?;
        
        let code = unsafe {
            cudaDeviceGetAttribute(&mut warp_size, CUDA_DEV_ATTR_WARP_SIZE, device_id)
        };
        check_cuda(code)?;

        if major < 6 || (major == 6 && minor < 1) {
            // E-053: Compute capability less than Pascal (6.1)
            return Err(ComputeApiError::VendorError(801)); // cudaErrorNotSupported
        }

        if warp_size != 32 {
            return Err(ComputeApiError::VendorError(999));
        }

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

        let offsets = layout::compute_state_offsets(layout.padded_n as usize);

        // 1. Allocate state blob in VRAM
        let mut raw_state_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let code = unsafe { cudaMalloc(&mut raw_state_ptr, offsets.total_size) };
        check_cuda(code)?;

        // Zero-fill state blob
        let code = unsafe {
            cudaMemsetAsync(
                raw_state_ptr,
                0,
                offsets.total_size,
                std::ptr::null_mut(),
            )
        };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
            }
            return Err(e);
        }

        // 2. Allocate axon heads in VRAM
        let mut raw_axons_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let axons_size =
            layout.total_axons as usize * std::mem::size_of::<layout::BurstHeads8>();
        let code = unsafe { cudaMalloc(&mut raw_axons_ptr, axons_size) };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
            }
            return Err(e);
        }

        // Initialize axon heads with AXON_SENTINEL (0x80000000)
        let host_axons = vec![0x80000000u32; layout.total_axons as usize * 8];
        let code = unsafe {
            cudaMemcpyAsync(
                raw_axons_ptr,
                host_axons.as_ptr() as *const std::ffi::c_void,
                axons_size,
                CUDA_MEMCPY_HOST_TO_DEVICE,
                std::ptr::null_mut(),
            )
        };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
            }
            return Err(e);
        }

        // 3. Allocate variant params in VRAM (16 variants x 64 bytes)
        let mut raw_variants_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let variants_size = 16 * std::mem::size_of::<VariantParameters>();
        let code = unsafe { cudaMalloc(&mut raw_variants_ptr, variants_size) };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
            }
            return Err(e);
        }

        // 4. Create non-blocking stream
        let mut stream: *mut std::ffi::c_void = std::ptr::null_mut();
        let code = unsafe { cudaStreamCreate(&mut stream) };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
                let _ = cudaFree(raw_variants_ptr);
            }
            return Err(e);
        }

        // 5. Allocate Pinned RAM host buffers
        let pinned_output_size = layout.padded_n as usize * 1024;
        let mut pinned_output_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let code = unsafe { cudaMallocHost(&mut pinned_output_ptr, pinned_output_size) };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
                let _ = cudaFree(raw_variants_ptr);
                let _ = cudaStreamDestroy(stream);
            }
            return Err(e);
        }

        let mut pinned_telemetry_ids_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let telemetry_ids_size = layout.padded_n as usize * std::mem::size_of::<u32>();
        let code = unsafe {
            cudaMallocHost(&mut pinned_telemetry_ids_ptr, telemetry_ids_size)
        };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
                let _ = cudaFree(raw_variants_ptr);
                let _ = cudaStreamDestroy(stream);
                let _ = cudaFreeHost(pinned_output_ptr);
            }
            return Err(e);
        }

        let mut pinned_telemetry_count_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let code = unsafe {
            cudaMallocHost(&mut pinned_telemetry_count_ptr, std::mem::size_of::<u32>())
        };
        if let Err(e) = check_cuda(code) {
            unsafe {
                let _ = cudaFree(raw_state_ptr);
                let _ = cudaFree(raw_axons_ptr);
                let _ = cudaFree(raw_variants_ptr);
                let _ = cudaStreamDestroy(stream);
                let _ = cudaFreeHost(pinned_output_ptr);
                let _ = cudaFreeHost(pinned_telemetry_ids_ptr);
            }
            return Err(e);
        }

        // Establish ShardVramPtrs pointers
        let base = raw_state_ptr as *mut u8;
        let vram_ptrs = unsafe {
            layout::ShardVramPtrs {
                soma_voltage: base.add(offsets.soma_voltage) as *mut i32,
                flags: base.add(offsets.flags) as *mut u8,
                threshold_offset: base.add(offsets.threshold_offset) as *mut i32,
                timers: base.add(offsets.timers) as *mut u8,
                soma_to_axon: base.add(offsets.soma_to_axon) as *mut u32,
                dendrite_targets: base.add(offsets.dendrite_targets) as *mut u32,
                dendrite_weights: base.add(offsets.dendrite_weights) as *mut i32,
                dendrite_timers: base.add(offsets.dendrite_timers) as *mut u8,
                axon_heads: raw_axons_ptr as *mut layout::BurstHeads8,
                variant_params: raw_variants_ptr as *const VariantParameters,
            }
        };

        let res = ShardCudaResources {
            raw_state_ptr,
            raw_axons_ptr,
            raw_variants_ptr,
            vram_ptrs,
            stream,
            layout: layout.clone(),
            pinned_output_ptr,
            pinned_output_size,
            pinned_telemetry_ids_ptr,
            pinned_telemetry_count_ptr,
            last_num_outputs: AtomicU32::new(0),
            last_sync_batch_ticks: AtomicU32::new(0),
        };

        let mut registry = self.resources.write().unwrap();
        let key = registry.insert(res);
        Ok(VramHandle(key.data().as_ffi()))
    }

    fn upload_state(&self, handle: &VramHandle, state: &[u8]) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        let offsets = layout::compute_state_offsets(res.layout.padded_n as usize);
        let upload_size = state.len().min(offsets.total_size);

        let code = unsafe {
            cudaMemcpyAsync(
                res.raw_state_ptr,
                state.as_ptr() as *const std::ffi::c_void,
                upload_size,
                CUDA_MEMCPY_HOST_TO_DEVICE,
                res.stream,
            )
        };
        check_cuda(code)?;

        let code = unsafe { cudaStreamSynchronize(res.stream) };
        check_cuda(code)?;

        Ok(())
    }

    fn upload_variants(
        &self,
        handle: &VramHandle,
        variants: &[VariantParameters],
    ) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        let upload_size =
            variants.len().min(16) * std::mem::size_of::<VariantParameters>();
        let code = unsafe {
            cudaMemcpyAsync(
                res.raw_variants_ptr,
                variants.as_ptr() as *const std::ffi::c_void,
                upload_size,
                CUDA_MEMCPY_HOST_TO_DEVICE,
                res.stream,
            )
        };
        check_cuda(code)?;

        let code = unsafe { cudaStreamSynchronize(res.stream) };
        check_cuda(code)?;

        Ok(())
    }

    fn run_day_batch(
        &self,
        handle: &VramHandle,
        cmd: &DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // E-050: Slice Length Mismatch
        if cmd.spike_counts.len() as u32 != cmd.sync_batch_ticks {
            return Err(ComputeApiError::InvalidLayout);
        }

        // E-052: Ephys target count checks
        if let Some(ref ephys) = cmd.ephys_cmd {
            if ephys.count > compute_api::MAX_EPHYS_TARGETS {
                return Err(ComputeApiError::InvalidLayout);
            }
        }

        // Validate bitmask length
        if let Some(mask) = cmd.input_bitmask {
            let required_bits = (cmd.num_virtual_axons * cmd.sync_batch_ticks) as usize;
            if mask.len() * 32 < required_bits {
                return Err(ComputeApiError::InvalidLayout);
            }
        }

        // Cache last sizes for output download
        res.last_num_outputs.store(cmd.num_outputs, Ordering::Relaxed);
        res.last_sync_batch_ticks.store(cmd.sync_batch_ticks, Ordering::Relaxed);

        // Execute Day Phase step loop
        for tick_idx in 0..cmd.sync_batch_ticks {
            let current_tick = cmd.tick_base + tick_idx;

            let code = unsafe {
                launch_update_neurons(
                    res.vram_ptrs,
                    res.layout.padded_n,
                    current_tick,
                    cmd.v_seg,
                    res.stream,
                )
            };
            check_cuda(code)?;

            let code = unsafe {
                launch_propagate_axons(
                    res.vram_ptrs,
                    res.layout.padded_n,
                    cmd.v_seg,
                    res.stream,
                )
            };
            check_cuda(code)?;

            let code = unsafe {
                launch_apply_gsop(
                    res.vram_ptrs,
                    res.layout.padded_n,
                    cmd.v_seg,
                    res.stream,
                )
            };
            check_cuda(code)?;
        }

        let code = unsafe { cudaStreamSynchronize(res.stream) };
        check_cuda(code)?;

        Ok(BatchResult {
            ticks_processed: cmd.sync_batch_ticks,
            is_warmup: false,
        })
    }

    fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        let num_outputs = res.last_num_outputs.load(Ordering::Relaxed);
        let sync_batch_ticks = res.last_sync_batch_ticks.load(Ordering::Relaxed);

        if num_outputs == 0 || sync_batch_ticks == 0 {
            return Ok(OutputFrame {
                data: vec![],
                num_outputs: 0,
                sync_batch_ticks: 0,
            });
        }

        let total_bytes = (num_outputs * sync_batch_ticks) as usize;
        if total_bytes > res.pinned_output_size {
            return Err(ComputeApiError::OutOfMemory);
        }

        // DMA D2H Copy using Pinned RAM (INV-COMPUTE-CUDA-006)
        let copy_size = total_bytes.min(res.layout.padded_n as usize);
        if copy_size > 0 {
            let code = unsafe {
                cudaMemcpyAsync(
                    res.pinned_output_ptr,
                    res.vram_ptrs.flags as *const std::ffi::c_void,
                    copy_size,
                    CUDA_MEMCPY_DEVICE_TO_HOST,
                    res.stream,
                )
            };
            check_cuda(code)?;
            
            let code = unsafe { cudaStreamSynchronize(res.stream) };
            check_cuda(code)?;
        }

        let mut data = vec![0u8; total_bytes];
        unsafe {
            std::ptr::copy_nonoverlapping(
                res.pinned_output_ptr as *const u8,
                data.as_mut_ptr(),
                copy_size,
            );
        }

        Ok(OutputFrame {
            data,
            num_outputs,
            sync_batch_ticks,
        })
    }

    fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Download flags array to count spikes
        let code = unsafe {
            cudaMemcpyAsync(
                res.pinned_telemetry_ids_ptr,
                res.vram_ptrs.flags as *const std::ffi::c_void,
                res.layout.padded_n as usize,
                CUDA_MEMCPY_DEVICE_TO_HOST,
                res.stream,
            )
        };
        check_cuda(code)?;

        let code = unsafe { cudaStreamSynchronize(res.stream) };
        check_cuda(code)?;

        let flags_slice = unsafe {
            std::slice::from_raw_parts(
                res.pinned_telemetry_ids_ptr as *const u8,
                res.layout.padded_n as usize,
            )
        };
        
        let mut active_soma_ids = Vec::new();
        for (i, &flag) in flags_slice.iter().enumerate() {
            if (flag & 0x01) != 0 {
                active_soma_ids.push(i as u32);
            }
        }
        let total_spikes = active_soma_ids.len() as u64;

        Ok(TelemetryFrame {
            active_soma_ids,
            total_spikes: total_spikes as u32,
        })
    }

    fn patch_ghosts(
        &self,
        handle: &VramHandle,
        patches: &[GhostPatch],
    ) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // INV-COMPUTE-API-006, E-051: Check ghost capacity bounds
        for patch in patches {
            match patch {
                GhostPatch::Add { dst_ghost, .. } | GhostPatch::Prune { dst_ghost } => {
                    if *dst_ghost >= res.layout.total_ghosts {
                        return Err(ComputeApiError::CapacityExceeded);
                    }
                }
            }
        }

        Ok(())
    }

    fn run_sort_and_prune(
        &self,
        handle: &VramHandle,
        _prune_threshold: i16,
    ) -> Result<(), ComputeApiError> {
        let registry = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let _res = registry.get(key).ok_or(ComputeApiError::InvalidHandle)?;
        
        Ok(())
    }

    fn free(&self, handle: VramHandle) {
        let key = slotmap::KeyData::from_ffi(handle.0).into();
        let mut registry = self.resources.write().unwrap();
        registry.remove(key);
    }
}
