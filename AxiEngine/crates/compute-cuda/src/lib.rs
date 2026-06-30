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

    /// Diagnostic/test utility wrapper that delegates to private `inject_and_propagate_axons_tick_native`.
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
        self.inject_and_propagate_axons_tick_native(
            handle,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
        )
    }

    /// Diagnostic/test utility wrapper for active-tail input-current probe.
    #[cfg(feature = "native")]
    pub fn compute_input_current_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        out_i_in: &mut [i32],
    ) -> Result<(), ComputeApiError> {
        self.compute_input_current_native(handle, out_i_in)
    }

    /// Diagnostic/test utility wrapper for GLIF membrane updates.
    #[cfg(feature = "native")]
    pub fn apply_glif_membrane_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        i_in: &[i32],
    ) -> Result<(), ComputeApiError> {
        self.apply_glif_membrane_native(handle, i_in)
    }

    /// Diagnostic/test utility that runs compute_input_current_probe and apply_glif_membrane_probe in one tick.
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

        self.compute_input_current_native(handle, &mut i_in)?;
        self.apply_glif_membrane_native(handle, &i_in)?;

        Ok(())
    }

    /// Diagnostic/test utility wrapper that runs variant-aware compute_input_current_probe
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
        self.apply_glif_final_spike_tick_native(
            handle,
            current_tick,
            v_seg,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
        )
    }

    /// Diagnostic/test utility wrapper that executes the complete single-tick pipeline without GSOP plasticity.
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
        self.run_single_tick_no_gsop_native(
            handle,
            current_tick,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
        )
    }

    /// Diagnostic/test utility wrapper that executes the complete single-tick pipeline including GSOP plasticity.
    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)]
    pub fn run_single_tick_with_gsop_probe_for_test(
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
        dopamine: i32,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        self.run_single_tick_with_gsop_native(
            handle,
            current_tick,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
            dopamine,
        )
    }

    /// Diagnostic/test utility wrapper that executes the GSOP plasticity optimization protocol.
    #[cfg(feature = "native")]
    pub fn apply_gsop_plasticity_probe_for_test(
        &mut self,
        handle: compute_api::VramHandle,
        dopamine: i32,
    ) -> Result<(), ComputeApiError> {
        self.apply_gsop_plasticity_native(handle, dopamine)
    }

    /// Private native helpers implementing CUDA execution steps.
    #[cfg(feature = "native")]
    fn inject_and_propagate_axons_tick_native(
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

    #[cfg(feature = "native")]
    fn compute_input_current_native(
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

    #[cfg(feature = "native")]
    fn apply_glif_membrane_native(
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

    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)] // Mirroring CPU and FFI discrete interface constraints
    fn apply_glif_final_spike_tick_native(
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

        // 1. Compute input currents via native helper
        self.compute_input_current_native(handle, &mut i_in)?;

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

    #[cfg(feature = "native")]
    fn apply_gsop_plasticity_native(
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

    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)] // Requires discrete arguments matching single tick execution interface
    fn run_single_tick_no_gsop_native(
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

        self.inject_and_propagate_axons_tick_native(
            handle,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
        )?;

        self.apply_glif_final_spike_tick_native(
            handle,
            current_tick,
            v_seg,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
        )
    }

    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)] // Requires discrete arguments matching single tick execution interface
    fn run_single_tick_with_gsop_native(
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
        dopamine: i32,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        let result = self.run_single_tick_no_gsop_native(
            handle,
            current_tick,
            v_seg,
            cmd_virtual_offset,
            num_virtual_axons,
            input_bitmask,
            incoming_spikes,
            mapped_soma_ids,
            max_spikes_per_tick,
            output_spikes,
            output_spike_counts,
        )?;

        self.apply_gsop_plasticity_native(handle, dopamine)?;

        Ok(result)
    }

    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)] // Requires discrete arguments matching single tick execution interface
    fn run_single_tick_with_gsop_production(
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
        dopamine: i32,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        let resource = self.registry.get_resource_mut(handle)?;

        let i_in_len = resource.spec.padded_n as usize;
        let input_bitmask_len = input_bitmask.map(|m| m.len()).unwrap_or(0);
        let incoming_spikes_len = incoming_spikes.map(|s| s.len()).unwrap_or(0);
        let mapped_soma_ids_len = mapped_soma_ids.len();
        let output_spikes_len = max_spikes_per_tick as usize;

        let offsets = layout::compute_state_offsets(i_in_len);
        if offsets.off_targets > u32::MAX as usize
            || offsets.off_weights > u32::MAX as usize
            || offsets.off_flags > u32::MAX as usize
            || offsets.off_voltage > u32::MAX as usize
            || offsets.off_thresh > u32::MAX as usize
            || offsets.off_timers > u32::MAX as usize
            || offsets.off_s2a > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        if input_bitmask_len > u32::MAX as usize
            || incoming_spikes_len > u32::MAX as usize
            || mapped_soma_ids_len > u32::MAX as usize
        {
            return Err(ComputeApiError::CapacityExceeded);
        }

        resource.scratch.ensure_capacity(
            i_in_len,
            input_bitmask_len,
            incoming_spikes_len,
            mapped_soma_ids_len,
            output_spikes_len,
        )?;

        if let Some(mask) = input_bitmask {
            if !mask.is_empty() {
                let size = std::mem::size_of_val(mask);
                let res = unsafe {
                    native::axi_cuda_copy_h2d(
                        resource.scratch.d_input_bitmask as *mut u8,
                        mask.as_ptr() as *const u8,
                        size,
                    )
                };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        if let Some(spikes) = incoming_spikes {
            if !spikes.is_empty() {
                let size = std::mem::size_of_val(spikes);
                let res = unsafe {
                    native::axi_cuda_copy_h2d(
                        resource.scratch.d_incoming_spikes as *mut u8,
                        spikes.as_ptr() as *const u8,
                        size,
                    )
                };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
            }
        }

        if !mapped_soma_ids.is_empty() {
            let size = std::mem::size_of_val(mapped_soma_ids);
            let res = unsafe {
                native::axi_cuda_copy_h2d(
                    resource.scratch.d_mapped_soma_ids as *mut u8,
                    mapped_soma_ids.as_ptr() as *const u8,
                    size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
        }

        let input_words_len = input_bitmask_len as u32;
        let spikes_count = incoming_spikes_len as u32;
        let res = unsafe {
            native::axi_cuda_inject_and_propagate_axons_tick(
                resource.axons_ptr,
                resource.spec.total_axons,
                v_seg,
                resource.spec.virtual_offset,
                cmd_virtual_offset,
                num_virtual_axons,
                if input_words_len > 0 {
                    resource.scratch.d_input_bitmask
                } else {
                    std::ptr::null()
                },
                input_words_len,
                if spikes_count > 0 {
                    resource.scratch.d_incoming_spikes
                } else {
                    std::ptr::null()
                },
                spikes_count,
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

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
            native::axi_cuda_compute_input_current_production(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_targets as u32,
                offsets.off_weights as u32,
                offsets.off_flags as u32,
                resource.scratch.d_i_in,
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        let res = unsafe {
            native::axi_cuda_apply_glif_final_spike_production(
                resource.state_ptr,
                resource.axons_ptr,
                resource.spec.padded_n,
                resource.spec.total_axons,
                offsets.off_voltage as u32,
                offsets.off_flags as u32,
                offsets.off_thresh as u32,
                offsets.off_timers as u32,
                offsets.off_s2a as u32,
                resource.scratch.d_i_in,
                current_tick,
                v_seg,
                if !mapped_soma_ids.is_empty() {
                    resource.scratch.d_mapped_soma_ids
                } else {
                    std::ptr::null()
                },
                mapped_soma_ids.len() as u32,
                max_spikes_per_tick,
                if max_spikes_per_tick > 0 {
                    resource.scratch.d_output_spikes
                } else {
                    std::ptr::null_mut()
                },
                resource.scratch.d_output_count,
                resource.scratch.d_generated_spikes_count,
                resource.scratch.d_dropped_spikes_count,
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        let res = unsafe {
            native::axi_cuda_apply_gsop_plasticity_production(
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

        let mut h_output_count = 0u32;
        let mut h_generated_spikes_count = 0u32;
        let mut h_dropped_spikes_count = 0u32;

        let res = unsafe {
            native::axi_cuda_copy_d2h(
                &mut h_output_count as *mut u32 as *mut u8,
                resource.scratch.d_output_count as *const u8,
                std::mem::size_of::<u32>(),
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        let res = unsafe {
            native::axi_cuda_copy_d2h(
                &mut h_generated_spikes_count as *mut u32 as *mut u8,
                resource.scratch.d_generated_spikes_count as *const u8,
                std::mem::size_of::<u32>(),
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        let res = unsafe {
            native::axi_cuda_copy_d2h(
                &mut h_dropped_spikes_count as *mut u32 as *mut u8,
                resource.scratch.d_dropped_spikes_count as *const u8,
                std::mem::size_of::<u32>(),
            )
        };
        if res != 0 {
            return Err(native::map_cuda_error(res));
        }

        output_spike_counts[0] = h_output_count;

        if h_output_count > 0 && max_spikes_per_tick > 0 {
            let copy_len = (h_output_count as usize).min(max_spikes_per_tick as usize);
            let res = unsafe {
                native::axi_cuda_copy_d2h(
                    output_spikes.as_mut_ptr() as *mut u8,
                    resource.scratch.d_output_spikes as *const u8,
                    copy_len * std::mem::size_of::<u32>(),
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
        }

        Ok(compute_api::BatchResult {
            ticks_executed: 1,
            generated_spikes_count: h_generated_spikes_count,
            output_spikes_written: h_output_count,
            dropped_spikes_count: h_dropped_spikes_count,
            execution_time_us: 0,
        })
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

    #[cfg(feature = "native")]
    fn run_day_batch(
        &mut self,
        handle: compute_api::VramHandle,
        cmd: compute_api::DayBatchCmd<'_>,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
        let uploaded = {
            let resource = self.registry.get_resource_mut(handle)?;
            resource.uploaded
        };
        if !uploaded {
            return Err(ComputeApiError::InvalidBatch);
        }
        compute_api::validation::validate_day_batch_cmd(&cmd)?;

        let mut generated_spikes_count: u32 = 0;
        let mut output_spikes_written: u32 = 0;
        let mut dropped_spikes_count: u32 = 0;

        // Reset output_spike_counts to zero for all ticks in this batch
        for count in cmd
            .output_spike_counts
            .iter_mut()
            .take(cmd.sync_batch_ticks as usize)
        {
            *count = 0;
        }

        let max_spikes_per_tick = cmd.max_spikes_per_tick as usize;
        let input_words_per_tick = cmd.input_words_per_tick as usize;

        // Output spikes buffer as mutable slice
        let output_spikes = cmd.output_spikes;

        for tick_idx in 0..cmd.sync_batch_ticks as usize {
            let current_tick = cmd.tick_base + tick_idx as u64;

            // Take tick slice for input_bitmask
            let tick_bitmask = if let Some(bitmask) = cmd.input_bitmask {
                let start_w = tick_idx * input_words_per_tick;
                let end_w = start_w + input_words_per_tick;
                if end_w <= bitmask.len() {
                    Some(&bitmask[start_w..end_w])
                } else {
                    None
                }
            } else {
                None
            };

            // Take incoming spikes slice
            let tick_incoming = if let Some(spikes) = cmd.incoming_spikes {
                let counts = cmd.incoming_spike_counts;
                if tick_idx < counts.len() {
                    let count = (counts[tick_idx] as usize).min(max_spikes_per_tick);
                    let start_s = tick_idx * max_spikes_per_tick;
                    if start_s + count <= spikes.len() {
                        Some(&spikes[start_s..start_s + count])
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Setup output slices for this tick directly from cmd buffers without allocations
            let dest_start = tick_idx * max_spikes_per_tick;
            let tick_output_spikes =
                &mut output_spikes[dest_start..dest_start + max_spikes_per_tick];
            let tick_output_counts = &mut cmd.output_spike_counts[tick_idx..tick_idx + 1];

            // Run single tick pipeline via production scratch path
            let tick_res = self.run_single_tick_with_gsop_production(
                handle,
                current_tick,
                cmd.v_seg,
                cmd.virtual_offset,
                cmd.num_virtual_axons,
                tick_bitmask,
                tick_incoming,
                cmd.mapped_soma_ids,
                cmd.max_spikes_per_tick,
                tick_output_spikes,
                tick_output_counts,
                cmd.dopamine as i32,
            )?;

            generated_spikes_count =
                generated_spikes_count.saturating_add(tick_res.generated_spikes_count);
            output_spikes_written =
                output_spikes_written.saturating_add(tick_res.output_spikes_written);
            dropped_spikes_count =
                dropped_spikes_count.saturating_add(tick_res.dropped_spikes_count);
        }

        Ok(compute_api::BatchResult {
            ticks_executed: cmd.sync_batch_ticks,
            generated_spikes_count,
            output_spikes_written,
            dropped_spikes_count,
            execution_time_us: 0,
        })
    }

    #[cfg(not(feature = "native"))]
    fn run_day_batch(
        &mut self,
        _handle: compute_api::VramHandle,
        _cmd: compute_api::DayBatchCmd<'_>,
    ) -> Result<compute_api::BatchResult, ComputeApiError> {
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
