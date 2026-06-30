//! Scaffold implementation of CudaBackend for AxiEngine Layer 3.

use std::marker::PhantomData;
use std::rc::Rc;

use compute_api::{BackendCapabilities, BackendKind, ComputeApiError, ComputeBackend};

#[cfg(feature = "native")]
mod native;

mod resource;

use resource::ResourceRegistry;

/// Configuration parameters for the CudaBackend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CudaBackendConfig {
    /// Target NVIDIA GPU device index.
    pub device_id: u32,
}

/// A CUDA-accelerated compute backend.
///
/// Thread-affine: statically restricted to a single OS thread.
pub struct CudaBackend {
    _config: CudaBackendConfig,
    registry: ResourceRegistry,
    // Statically prevent Send and Sync
    _marker: PhantomData<Rc<()>>,
}

impl CudaBackend {
    /// Creates a new instance of the CUDA compute backend.
    ///
    /// # Errors
    /// Returns `ComputeApiError::UnsupportedBackend` in Stage 1A when native drivers/features are absent.
    pub fn new(config: CudaBackendConfig) -> Result<Self, ComputeApiError> {
        #[cfg(not(feature = "native"))]
        {
            let _ = config;
            Err(ComputeApiError::UnsupportedBackend)
        }
        #[cfg(feature = "native")]
        {
            let res = unsafe { native::axi_cuda_probe_device(config.device_id) };
            if res != 0 {
                return Err(ComputeApiError::UnsupportedBackend);
            }
            Ok(Self {
                _config: config,
                registry: ResourceRegistry::default(),
                _marker: PhantomData,
            })
        }
    }

    /// Returns static capabilities of the CUDA execution backend.
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            lane_count: 32,
            supports_async: true,
            supports_ephys: false,
            max_batch_ticks: 1000,
            alignment_bytes: 64,
            pinned_host_required: true,
        }
    }

    /// Advances an axonal propagation head on the GPU.
    #[cfg(feature = "native")]
    pub fn cuda_propagate_head_for_test(head: u32, v_seg: u32) -> Result<u32, ComputeApiError> {
        let mut out = 0u32;
        let res = unsafe { native::axi_cuda_propagate_head(head, v_seg, &mut out) };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(out)
    }

    /// Evaluates active tail contact on the GPU.
    #[cfg(feature = "native")]
    pub fn cuda_active_tail_hit_for_test(
        head: u32,
        seg_idx: u32,
        propagation_length: u32,
    ) -> Result<bool, ComputeApiError> {
        let mut out = 0u8;
        let res = unsafe {
            native::axi_cuda_active_tail_hit(head, seg_idx, propagation_length, &mut out)
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(out != 0)
    }

    /// Propagation test utility to advance axons on the GPU.
    #[cfg(feature = "native")]
    pub fn propagate_uploaded_axons_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        v_seg: u32,
    ) -> Result<(), ComputeApiError> {
        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        let res = unsafe {
            native::axi_cuda_propagate_uploaded_axons(
                resource.axons_ptr,
                resource.spec.total_axons,
                v_seg,
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(())
    }

    /// Single-tick injection and propagation test utility.
    #[cfg(feature = "native")]
    pub fn inject_and_propagate_axons_tick_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        v_seg: u32,
        cmd_virtual_offset: u32,
        num_virtual_axons: u32,
        input_bitmask: Option<&[u32]>,
        incoming_spikes: Option<&[u32]>,
    ) -> Result<(), ComputeApiError> {
        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if let Some(mask) = input_bitmask {
            if mask.len() > u32::MAX as usize {
                return Err(ComputeApiError::CapacityExceeded);
            }
            let required_words = num_virtual_axons.div_ceil(32) as usize;
            if mask.len() < required_words {
                return Err(ComputeApiError::InvalidBatch);
            }
        }
        if let Some(spikes) = incoming_spikes {
            if spikes.len() > u32::MAX as usize {
                return Err(ComputeApiError::CapacityExceeded);
            }
        }

        let mut input_words_len = 0u32;
        let mut d_bitmask = std::ptr::null_mut();
        if let Some(mask) = input_bitmask {
            if !mask.is_empty() {
                input_words_len = mask.len() as u32;
                let size = std::mem::size_of_val(mask);
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut d_bitmask) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                let res = unsafe {
                    native::axi_cuda_copy_h2d(d_bitmask, mask.as_ptr() as *const u8, size)
                };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                    }
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        let mut d_spikes = std::ptr::null_mut();
        let mut spikes_count = 0u32;
        if let Some(spikes) = incoming_spikes {
            if !spikes.is_empty() {
                spikes_count = spikes.len() as u32;
                let size = std::mem::size_of_val(spikes);
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut d_spikes) };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                    }
                    return Err(native::map_cuda_error(res));
                }
                let res = unsafe {
                    native::axi_cuda_copy_h2d(d_spikes, spikes.as_ptr() as *const u8, size)
                };
                if res != 0 {
                    unsafe {
                        let _ = native::axi_cuda_free(d_bitmask);
                        let _ = native::axi_cuda_free(d_spikes);
                    }
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        let res = unsafe {
            native::axi_cuda_inject_and_propagate_axons_tick(
                resource.axons_ptr,
                resource.spec.total_axons,
                v_seg,
                resource.spec.virtual_offset,
                cmd_virtual_offset,
                num_virtual_axons,
                d_bitmask as *const u32,
                input_words_len,
                d_spikes as *const u32,
                spikes_count,
            )
        };

        unsafe {
            let _ = native::axi_cuda_free(d_bitmask);
            let _ = native::axi_cuda_free(d_spikes);
        }

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }
        Ok(())
    }

    /// Native-only test utility for active-tail input-current probe.
    #[cfg(feature = "native")]
    pub fn compute_input_current_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        out_i_in: &mut [i32],
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if out_i_in.len() > u32::MAX as usize {
            return Err(ComputeApiError::CapacityExceeded);
        }

        if out_i_in.len() < resource.spec.padded_n as usize {
            return Err(ComputeApiError::InvalidBatch);
        }

        let offsets = layout::compute_state_offsets(resource.spec.padded_n as usize);
        if offsets.off_targets > u32::MAX as usize
            || offsets.off_weights > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        // Upload variant_table to constant memory
        let upload_res = unsafe {
            native::axi_cuda_upload_variant_table(
                resource.variant_table.as_ptr() as *const u8,
                resource.variant_table.len() * std::mem::size_of::<layout::VariantParameters>(),
            )
        };
        if upload_res != 0 {
            return Err(native::map_cuda_error(upload_res));
        }

        let res = unsafe {
            native::axi_cuda_compute_input_current_probe(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_targets as u32,
                offsets.off_weights as u32,
                offsets.off_flags as u32,
                out_i_in.as_mut_ptr(),
                out_i_in.len() as u32,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        Ok(())
    }

    /// Native-only test utility for GLIF membrane updates.
    #[cfg(feature = "native")]
    pub fn apply_glif_membrane_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        i_in: &[i32],
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        if i_in.len() > u32::MAX as usize {
            return Err(ComputeApiError::CapacityExceeded);
        }

        if i_in.len() < resource.spec.padded_n as usize {
            return Err(ComputeApiError::InvalidBatch);
        }

        let offsets = layout::compute_state_offsets(resource.spec.padded_n as usize);
        if offsets.off_voltage > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
            || offsets.off_thresh > u32::MAX as usize
            || offsets.off_timers > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        // Re-upload this resource's variant_table to device constant memory
        let variant_bytes = resource.variant_table.as_ptr() as *const u8;
        let variant_size = std::mem::size_of_val(&resource.variant_table);
        let upload_res =
            unsafe { native::axi_cuda_upload_variant_table(variant_bytes, variant_size) };
        if upload_res != 0 {
            return Err(native::map_cuda_error(upload_res));
        }

        let res = unsafe {
            native::axi_cuda_apply_glif_membrane_probe(
                resource.state_ptr,
                resource.spec.padded_n,
                offsets.off_voltage as u32,
                offsets.off_flags as u32,
                offsets.off_thresh as u32,
                offsets.off_timers as u32,
                i_in.as_ptr(),
                i_in.len() as u32,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        Ok(())
    }

    /// Native-only test utility that runs compute_input_current_probe and apply_glif_membrane_probe in one tick.
    #[cfg(feature = "native")]
    pub fn run_current_glif_tick_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }

        let padded_n = resource.spec.padded_n as usize;
        let mut i_in = vec![0i32; padded_n];

        self.compute_input_current_probe_for_test(handle, &mut i_in)?;
        self.apply_glif_membrane_probe_for_test(handle, &i_in)?;

        Ok(())
    }

    /// Native-only test utility that runs variant-aware compute_input_current_probe
    /// and apply_glif_final_spike_probe in one tick.
    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)] // Takes discrete parameter slices to precisely mimic CPU-matching test interface
    pub fn run_current_glif_final_tick_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        current_tick: u64,
        v_seg: u32,
        mapped_soma_ids: &[u32],
        max_spikes_per_tick: u32,
        output_spikes: &mut [u32],
        output_spike_counts: &mut [u32],
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        let padded_n = {
            let resource = self.registry.get_resource_mut(handle)?;
            if !resource.uploaded {
                return Err(ComputeApiError::BackendNotInitialized);
            }
            resource.spec.padded_n as usize
        };

        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }

        if output_spike_counts.is_empty() {
            return Err(ComputeApiError::InvalidBatch);
        }

        if output_spikes.len() < max_spikes_per_tick as usize {
            return Err(ComputeApiError::InvalidBatch);
        }

        let mut i_in = vec![0i32; padded_n];

        // 1. Compute input currents via variant-aware probe
        self.compute_input_current_probe_for_test(handle, &mut i_in)?;

        // Now we can safely borrow resource for the rest of the method
        let resource = self.registry.get_resource_mut(handle)?;

        // 2. Re-upload this resource's variant_table to device constant memory
        let variant_bytes = resource.variant_table.as_ptr() as *const u8;
        let variant_size = std::mem::size_of_val(&resource.variant_table);
        let upload_res =
            unsafe { native::axi_cuda_upload_variant_table(variant_bytes, variant_size) };
        if upload_res != 0 {
            return Err(native::map_cuda_error(upload_res));
        }

        let offsets = layout::compute_state_offsets(padded_n);
        if offsets.off_voltage > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
            || offsets.off_thresh > u32::MAX as usize
            || offsets.off_timers > u32::MAX as usize
            || offsets.off_s2a > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        let mut output_count = 0u32;
        let mut generated_spikes_count = 0u32;
        let mut dropped_spikes_count = 0u32;

        let res = unsafe {
            native::axi_cuda_apply_glif_final_spike_probe(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_voltage as u32,
                offsets.off_flags as u32,
                offsets.off_thresh as u32,
                offsets.off_timers as u32,
                offsets.off_s2a as u32,
                i_in.as_ptr(),
                i_in.len() as u32,
                current_tick,
                v_seg,
                mapped_soma_ids.as_ptr(),
                mapped_soma_ids.len() as u32,
                max_spikes_per_tick,
                output_spikes.as_mut_ptr(),
                &mut output_count,
                &mut generated_spikes_count,
                &mut dropped_spikes_count,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        output_spike_counts[0] = output_count;

        Ok(compute_api::BatchResult {
            ticks_executed: 1,
            generated_spikes_count,
            output_spikes_written: output_count,
            dropped_spikes_count,
            execution_time_us: 0,
        })
    }

    /// Native-only test utility that executes the complete single-tick pipeline without GSOP plasticity:
    /// 1. Axon spike propagation and virtual/incoming spikes injection.
    /// 2. Input currents calculations.
    /// 3. GLIF voltage updates, DDS heartbeat, axon pushes, and output spikes emission.
    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)]
    pub fn run_single_tick_no_gsop_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        current_tick: u64,
        v_seg: u32,
        cmd_virtual_offset: u32,
        num_virtual_axons: u32,
        input_bitmask: Option<&[u32]>,
        incoming_spikes: Option<&[u32]>,
        mapped_soma_ids: &[u32],
        max_spikes_per_tick: u32,
        output_spikes: &mut [u32],
        output_spike_counts: &mut [u32],
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        if !(1..=255).contains(&v_seg) {
            return Err(ComputeApiError::InvalidBatch);
        }
        if output_spike_counts.is_empty() {
            return Err(ComputeApiError::InvalidBatch);
        }
        if output_spikes.len() < max_spikes_per_tick as usize {
            return Err(ComputeApiError::InvalidBatch);
        }
        if let Some(mask) = input_bitmask {
            if mask.len() > u32::MAX as usize {
                return Err(ComputeApiError::CapacityExceeded);
            }
            let required_words = num_virtual_axons.div_ceil(32) as usize;
            if mask.len() < required_words {
                return Err(ComputeApiError::InvalidBatch);
            }
        }
        if let Some(spikes) = incoming_spikes {
            if spikes.len() > max_spikes_per_tick as usize {
                return Err(ComputeApiError::InvalidBatch);
            }
        }

        self.inject_and_propagate_axons_tick_for_test(
            handle,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
        )?;

        self.run_current_glif_final_tick_probe_for_test(
            handle,
            current_tick,
            v_seg,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
        )
    }

    /// Native-only test utility that executes the GSOP plasticity optimization protocol
    /// directly on the GPU state for all synapses targeting currently active soma cells.
    #[cfg(feature = "native")]
    pub fn apply_gsop_plasticity_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        dopamine: i32,
    ) -> Result<(), ComputeApiError> {
        let padded_n = {
            let resource = self.registry.get_resource_mut(handle)?;
            if !resource.uploaded {
                return Err(ComputeApiError::BackendNotInitialized);
            }
            resource.spec.padded_n as usize
        };

        let resource = self.registry.get_resource_mut(handle)?;
        let variant_bytes = resource.variant_table.as_ptr() as *const u8;
        let variant_size = std::mem::size_of_val(&resource.variant_table);
        let upload_res =
            unsafe { native::axi_cuda_upload_variant_table(variant_bytes, variant_size) };
        if upload_res != 0 {
            return Err(native::map_cuda_error(upload_res));
        }

        let offsets = layout::compute_state_offsets(padded_n);
        if offsets.off_targets > u32::MAX as usize
            || offsets.off_weights > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        let res = unsafe {
            native::axi_cuda_apply_gsop_plasticity_probe(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_targets as u32,
                offsets.off_weights as u32,
                offsets.off_flags as u32,
                dopamine,
            )
        };

        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        Ok(())
    }
}

impl ComputeBackend for CudaBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Cuda
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    fn alloc_shard(
        &mut self,
        spec: compute_api::ShardAllocSpec,
    ) -> Result<compute_api::VramHandle, ComputeApiError> {
        self.registry.alloc_shard(spec)
    }

    fn upload_shard(
        &mut self,
        handle: compute_api::VramHandle,
        upload: compute_api::ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        self.registry.upload_shard(handle, upload)
    }

    fn run_day_batch(
        &mut self,
        _handle: compute_api::VramHandle,
        _cmd: compute_api::DayBatchCmd<'_>,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        // Stage 1D: run_day_batch implementation is deferred.
        Err(ComputeApiError::UnsupportedFeature)
    }

    fn free_shard(&mut self, handle: compute_api::VramHandle) -> Result<(), ComputeApiError> {
        self.registry.free_shard(handle)
    }

    fn debug_snapshot(
        &mut self,
        handle: compute_api::VramHandle,
        snapshot: compute_api::ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        compute_api::validation::validate_snapshot_buffers(&resource.spec, &snapshot)?;

        #[cfg(feature = "native")]
        {
            let res = unsafe {
                native::axi_cuda_copy_d2h(
                    snapshot.state_blob.as_mut_ptr(),
                    resource.state_ptr,
                    resource.state_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }

            let res = unsafe {
                native::axi_cuda_copy_d2h(
                    snapshot.axons_blob.as_mut_ptr(),
                    resource.axons_ptr,
                    resource.axons_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            Ok(())
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = snapshot;
            Err(ComputeApiError::UnsupportedBackend)
        }
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        self.registry.teardown()
    }
}

#[cfg(test)]
mod tests;
