//! Private VRAM resource registry for compute-cuda.

use compute_api::{
    validation, BackendKind, ComputeApiError, ShardAllocSpec, ShardUpload, VramHandle,
};

#[cfg(feature = "native")]
use core::num::NonZeroU64;

#[cfg(feature = "native")]
use crate::native;

/// GPU scratch memory manager for non-allocating production batch paths.
pub struct CudaScratch {
    pub d_i_in: *mut i32,
    pub i_in_capacity: usize,

    pub d_input_bitmask: *mut u32,
    pub input_bitmask_capacity: usize,

    pub d_incoming_spikes: *mut u32,
    pub incoming_spikes_capacity: usize,

    pub d_mapped_soma_ids: *mut u32,
    pub mapped_soma_ids_capacity: usize,

    pub d_output_spikes: *mut u32,
    pub output_spikes_capacity: usize,

    pub d_output_spike_counts: *mut u32,
    pub output_spike_counts_capacity: usize,

    // Total counters
    pub d_output_count: *mut u32,
    pub d_generated_spikes_count: *mut u32,
    pub d_output_spikes_written: *mut u32,
    pub d_dropped_spikes_count: *mut u32,

    // Tick-level scalar counters
    pub d_tick_generated: *mut u32,
    pub d_tick_written: *mut u32,
    pub d_tick_dropped: *mut u32,
}

impl Default for CudaScratch {
    fn default() -> Self {
        Self {
            d_i_in: std::ptr::null_mut(),
            i_in_capacity: 0,
            d_input_bitmask: std::ptr::null_mut(),
            input_bitmask_capacity: 0,
            d_incoming_spikes: std::ptr::null_mut(),
            incoming_spikes_capacity: 0,
            d_mapped_soma_ids: std::ptr::null_mut(),
            mapped_soma_ids_capacity: 0,
            d_output_spikes: std::ptr::null_mut(),
            output_spikes_capacity: 0,
            d_output_spike_counts: std::ptr::null_mut(),
            output_spike_counts_capacity: 0,
            d_output_count: std::ptr::null_mut(),
            d_generated_spikes_count: std::ptr::null_mut(),
            d_output_spikes_written: std::ptr::null_mut(),
            d_dropped_spikes_count: std::ptr::null_mut(),
            d_tick_generated: std::ptr::null_mut(),
            d_tick_written: std::ptr::null_mut(),
            d_tick_dropped: std::ptr::null_mut(),
        }
    }
}

impl Drop for CudaScratch {
    fn drop(&mut self) {
        #[cfg(feature = "native")]
        {
            unsafe {
                if !self.d_i_in.is_null() {
                    let _ = native::axi_cuda_free(self.d_i_in as *mut u8);
                }
                if !self.d_input_bitmask.is_null() {
                    let _ = native::axi_cuda_free(self.d_input_bitmask as *mut u8);
                }
                if !self.d_incoming_spikes.is_null() {
                    let _ = native::axi_cuda_free(self.d_incoming_spikes as *mut u8);
                }
                if !self.d_mapped_soma_ids.is_null() {
                    let _ = native::axi_cuda_free(self.d_mapped_soma_ids as *mut u8);
                }
                if !self.d_output_spikes.is_null() {
                    let _ = native::axi_cuda_free(self.d_output_spikes as *mut u8);
                }
                if !self.d_output_spike_counts.is_null() {
                    let _ = native::axi_cuda_free(self.d_output_spike_counts as *mut u8);
                }
                if !self.d_output_count.is_null() {
                    let _ = native::axi_cuda_free(self.d_output_count as *mut u8);
                }
                if !self.d_generated_spikes_count.is_null() {
                    let _ = native::axi_cuda_free(self.d_generated_spikes_count as *mut u8);
                }
                if !self.d_output_spikes_written.is_null() {
                    let _ = native::axi_cuda_free(self.d_output_spikes_written as *mut u8);
                }
                if !self.d_dropped_spikes_count.is_null() {
                    let _ = native::axi_cuda_free(self.d_dropped_spikes_count as *mut u8);
                }
                if !self.d_tick_generated.is_null() {
                    let _ = native::axi_cuda_free(self.d_tick_generated as *mut u8);
                }
                if !self.d_tick_written.is_null() {
                    let _ = native::axi_cuda_free(self.d_tick_written as *mut u8);
                }
                if !self.d_tick_dropped.is_null() {
                    let _ = native::axi_cuda_free(self.d_tick_dropped as *mut u8);
                }
            }
        }
    }
}

impl CudaScratch {
    /// Lazy allocation / expansion of the GPU buffers.
    #[cfg(feature = "native")]
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_capacity(
        &mut self,
        i_in_len: usize,
        input_bitmask_len: usize,
        incoming_spikes_len: usize,
        mapped_soma_ids_len: usize,
        output_spikes_len: usize,
        output_spike_counts_len: usize,
    ) -> Result<(), ComputeApiError> {
        if i_in_len > self.i_in_capacity {
            if !self.d_i_in.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_i_in as *mut u8);
                }
                self.d_i_in = std::ptr::null_mut();
                self.i_in_capacity = 0;
            }
            let mut ptr = std::ptr::null_mut();
            let size = i_in_len * std::mem::size_of::<i32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_i_in = ptr as *mut i32;
            self.i_in_capacity = i_in_len;
        }

        if input_bitmask_len > self.input_bitmask_capacity {
            if !self.d_input_bitmask.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_input_bitmask as *mut u8);
                }
                self.d_input_bitmask = std::ptr::null_mut();
                self.input_bitmask_capacity = 0;
            }
            if input_bitmask_len > 0 {
                let mut ptr = std::ptr::null_mut();
                let size = input_bitmask_len * std::mem::size_of::<u32>();
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                self.d_input_bitmask = ptr as *mut u32;
                self.input_bitmask_capacity = input_bitmask_len;
            }
        }

        if incoming_spikes_len > self.incoming_spikes_capacity {
            if !self.d_incoming_spikes.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_incoming_spikes as *mut u8);
                }
                self.d_incoming_spikes = std::ptr::null_mut();
                self.incoming_spikes_capacity = 0;
            }
            if incoming_spikes_len > 0 {
                let mut ptr = std::ptr::null_mut();
                let size = incoming_spikes_len * std::mem::size_of::<u32>();
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                self.d_incoming_spikes = ptr as *mut u32;
                self.incoming_spikes_capacity = incoming_spikes_len;
            }
        }

        if mapped_soma_ids_len > self.mapped_soma_ids_capacity {
            if !self.d_mapped_soma_ids.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_mapped_soma_ids as *mut u8);
                }
                self.d_mapped_soma_ids = std::ptr::null_mut();
                self.mapped_soma_ids_capacity = 0;
            }
            if mapped_soma_ids_len > 0 {
                let mut ptr = std::ptr::null_mut();
                let size = mapped_soma_ids_len * std::mem::size_of::<u32>();
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                self.d_mapped_soma_ids = ptr as *mut u32;
                self.mapped_soma_ids_capacity = mapped_soma_ids_len;
            }
        }

        if output_spikes_len > self.output_spikes_capacity {
            if !self.d_output_spikes.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_output_spikes as *mut u8);
                }
                self.d_output_spikes = std::ptr::null_mut();
                self.output_spikes_capacity = 0;
            }
            if output_spikes_len > 0 {
                let mut ptr = std::ptr::null_mut();
                let size = output_spikes_len * std::mem::size_of::<u32>();
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                self.d_output_spikes = ptr as *mut u32;
                self.output_spikes_capacity = output_spikes_len;
            }
        }

        if output_spike_counts_len > self.output_spike_counts_capacity {
            if !self.d_output_spike_counts.is_null() {
                unsafe {
                    native::axi_cuda_free(self.d_output_spike_counts as *mut u8);
                }
                self.d_output_spike_counts = std::ptr::null_mut();
                self.output_spike_counts_capacity = 0;
            }
            if output_spike_counts_len > 0 {
                let mut ptr = std::ptr::null_mut();
                let size = output_spike_counts_len * std::mem::size_of::<u32>();
                let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
                if res != 0 {
                    return Err(native::map_cuda_error(res));
                }
                self.d_output_spike_counts = ptr as *mut u32;
                self.output_spike_counts_capacity = output_spike_counts_len;
            }
        }

        if self.d_output_count.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_output_count = ptr as *mut u32;
        }

        if self.d_generated_spikes_count.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_generated_spikes_count = ptr as *mut u32;
        }

        if self.d_output_spikes_written.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_output_spikes_written = ptr as *mut u32;
        }

        if self.d_dropped_spikes_count.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_dropped_spikes_count = ptr as *mut u32;
        }

        if self.d_tick_generated.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_tick_generated = ptr as *mut u32;
        }

        if self.d_tick_written.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_tick_written = ptr as *mut u32;
        }

        if self.d_tick_dropped.is_null() {
            let mut ptr = std::ptr::null_mut();
            let size = std::mem::size_of::<u32>();
            let res = unsafe { native::axi_cuda_alloc_bytes(size, &mut ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            self.d_tick_dropped = ptr as *mut u32;
        }

        Ok(())
    }
}

/// Internal VRAM resource allocated for a single simulation shard on the GPU.
pub struct CudaResource {
    pub spec: ShardAllocSpec,
    #[allow(dead_code)]
    pub state_ptr: *mut u8,
    #[allow(dead_code)]
    pub axons_ptr: *mut u8,
    #[allow(dead_code)]
    pub state_size: usize,
    #[allow(dead_code)]
    pub axons_size: usize,
    pub uploaded: bool,
    pub variant_table: [layout::VariantParameters; layout::VARIANT_LUT_LEN],
    pub scratch: CudaScratch,
}

impl Drop for CudaResource {
    fn drop(&mut self) {
        #[cfg(feature = "native")]
        {
            if !self.state_ptr.is_null() {
                unsafe {
                    let _ = native::axi_cuda_free(self.state_ptr);
                }
            }
            if !self.axons_ptr.is_null() {
                unsafe {
                    let _ = native::axi_cuda_free(self.axons_ptr);
                }
            }
        }
    }
}

#[cfg_attr(not(feature = "native"), allow(dead_code))]
pub enum ResourceSlot {
    Empty,
    Occupied {
        generation: u32,
        resource: Box<CudaResource>,
    },
    Freed {
        generation: u32,
    },
}

/// Internal registry for managing device buffers allocated on the GPU.
#[derive(Default)]
pub struct ResourceRegistry {
    slots: Vec<ResourceSlot>,
}

impl ResourceRegistry {
    /// Allocates device VRAM blocks and registers the shard resource.
    pub fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
        validation::validate_alloc_spec(&spec)?;
        let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_size = validation::expected_axons_blob_size(spec.total_axons)?;

        #[cfg(feature = "native")]
        {
            let mut state_ptr = core::ptr::null_mut();
            let mut axons_ptr = core::ptr::null_mut();

            let res = unsafe { native::axi_cuda_alloc_bytes(state_size, &mut state_ptr) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }
            let res = unsafe { native::axi_cuda_alloc_bytes(axons_size, &mut axons_ptr) };
            if res != 0 {
                unsafe {
                    let _ = native::axi_cuda_free(state_ptr);
                }
                return Err(native::map_cuda_error(res));
            }

            let zero_param = layout::VariantParameters {
                threshold: 0,
                rest_potential: 0,
                leak_shift: 0,
                homeostasis_penalty: 0,
                spontaneous_firing_period_ticks: 0,
                initial_synapse_weight: 0,
                gsop_potentiation: 0,
                gsop_depression: 0,
                homeostasis_decay: 0,
                refractory_period: 0,
                fatigue_capacity: 255,
                signal_propagation_length: 0,
                is_inhibitory: 0,
                inertia_curve: [0; 8],
                ahp_amplitude: 0,
                _pad1: [0; 6],
                adaptive_leak_min_shift: 0,
                adaptive_leak_gain: 0,
                adaptive_mode: 0,
                _leak_pad: [0; 3],
                d1_affinity: 0,
                d2_affinity: 0,
                heartbeat_m: 0,
            };

            let resource = CudaResource {
                spec,
                state_ptr,
                axons_ptr,
                state_size,
                axons_size,
                uploaded: false,
                variant_table: [zero_param; layout::VARIANT_LUT_LEN],
                scratch: CudaScratch::default(),
            };

            let mut found_idx = None;
            for (idx, slot) in self.slots.iter().enumerate() {
                if matches!(slot, ResourceSlot::Empty | ResourceSlot::Freed { .. }) {
                    found_idx = Some(idx);
                    break;
                }
            }

            let slot_idx = match found_idx {
                Some(idx) => idx,
                None => {
                    self.slots.push(ResourceSlot::Empty);
                    self.slots.len() - 1
                }
            };

            let generation = match &self.slots[slot_idx] {
                ResourceSlot::Freed { generation } => generation.wrapping_add(1),
                _ => 1,
            };

            self.slots[slot_idx] = ResourceSlot::Occupied {
                generation,
                resource: Box::new(resource),
            };

            let raw_id =
                NonZeroU64::new((slot_idx as u64) + 1).ok_or(ComputeApiError::InvalidHandle)?;
            Ok(VramHandle::from_raw_parts(
                BackendKind::Cuda,
                raw_id,
                generation,
            ))
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = spec;
            let _ = state_size;
            let _ = axons_size;
            Err(ComputeApiError::UnsupportedBackend)
        }
    }

    /// Validates handle and returns a mutable reference to the resource.
    pub fn get_resource_mut(
        &mut self,
        handle: VramHandle,
    ) -> Result<&mut CudaResource, ComputeApiError> {
        if handle.kind() != BackendKind::Cuda {
            return Err(ComputeApiError::ForeignHandle);
        }

        let id = handle.id().get() as usize;
        if id == 0 || id > self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        let slot_idx = id - 1;
        match &mut self.slots[slot_idx] {
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
            ResourceSlot::Freed { generation } => {
                if handle.generation() == *generation {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Occupied {
                generation,
                resource,
            } => {
                if handle.generation() != *generation {
                    Err(ComputeApiError::InvalidHandle)
                } else {
                    Ok(resource)
                }
            }
        }
    }

    /// Uploads state, axons and variant table to the device.
    pub fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        #[cfg(feature = "native")]
        {
            let resource = self.get_resource_mut(handle)?;
            validation::validate_upload(&resource.spec, &upload)?;

            let res = unsafe {
                native::axi_cuda_copy_h2d(
                    resource.state_ptr,
                    upload.state_blob.as_ptr(),
                    resource.state_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }

            let res = unsafe {
                native::axi_cuda_copy_h2d(
                    resource.axons_ptr,
                    upload.axons_blob.as_ptr(),
                    resource.axons_size,
                )
            };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }

            let variant_bytes = upload.variant_table.as_ptr() as *const u8;
            let variant_size =
                upload.variant_table.len() * std::mem::size_of::<layout::VariantParameters>();
            let res = unsafe { native::axi_cuda_upload_variant_table(variant_bytes, variant_size) };
            if res != 0 {
                return Err(native::map_cuda_error(res));
            }

            resource.variant_table.copy_from_slice(upload.variant_table);
            resource.uploaded = true;
            Ok(())
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = handle;
            let _ = upload;
            Err(ComputeApiError::UnsupportedBackend)
        }
    }

    /// Invalidate and free the resource.
    pub fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        if handle.kind() != BackendKind::Cuda {
            return Err(ComputeApiError::ForeignHandle);
        }

        let id = handle.id().get() as usize;
        if id == 0 || id > self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        let slot_idx = id - 1;
        match &self.slots[slot_idx] {
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
            ResourceSlot::Freed { generation } => {
                if handle.generation() == *generation {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Occupied { generation, .. } => {
                if handle.generation() != *generation {
                    Err(ComputeApiError::InvalidHandle)
                } else {
                    let gen = *generation;
                    // Replacing slot with Freed triggers Drop on CudaResource, freeing device memory
                    self.slots[slot_idx] = ResourceSlot::Freed { generation: gen };
                    Ok(())
                }
            }
        }
    }

    /// Clears and invalidates all slots.
    pub fn teardown(&mut self) -> Result<(), ComputeApiError> {
        for slot in self.slots.iter_mut() {
            let next_gen = match slot {
                ResourceSlot::Occupied { generation, .. } => generation.wrapping_add(1),
                ResourceSlot::Freed { generation } => generation.wrapping_add(1),
                ResourceSlot::Empty => 1,
            };
            *slot = ResourceSlot::Freed {
                generation: next_gen,
            };
        }
        Ok(())
    }
}
