//! Private VRAM resource registry for compute-cuda.

use compute_api::{
    validation, BackendKind, ComputeApiError, ShardAllocSpec, ShardUpload, VramHandle,
};

#[cfg(feature = "native")]
use core::num::NonZeroU64;

#[cfg(feature = "native")]
use crate::native;

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

            let resource = CudaResource {
                spec,
                state_ptr,
                axons_ptr,
                state_size,
                axons_size,
                uploaded: false,
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
