//! Internal RAM resource registry for managing allocated simulation shards.

use crate::aligned_mem::AlignedBuffer;
use compute_api::{
    validation, BackendKind, ComputeApiError, ShardAllocSpec, ShardUpload, VramHandle,
};
use core::num::NonZeroU64;

/// Internal host RAM resource allocated for a single simulation shard.
pub struct HostResource {
    pub spec: ShardAllocSpec,
    pub state_blob: AlignedBuffer,
    pub axons_blob: AlignedBuffer,
    pub variant_table: [layout::VariantParameters; layout::VARIANT_LUT_LEN],
    pub uploaded: bool,
}

enum ResourceSlot {
    Empty,
    Occupied {
        generation: u32,
        resource: Box<HostResource>,
    },
    Freed {
        generation: u32,
    },
}

/// Internal vector-based resource registry with generation counter validation.
#[derive(Default)]
pub struct ResourceRegistry {
    slots: Vec<ResourceSlot>,
}

impl ResourceRegistry {
    /// Allocates physical 64B-aligned host RAM buffers and registers the shard resource.
    pub fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
        validation::validate_alloc_spec(&spec)?;

        let state_blob_size = layout::calculate_state_blob_size(spec.padded_n as usize);
        let axons_blob_size = validation::expected_axons_blob_size(spec.total_axons)?;

        let state_blob =
            AlignedBuffer::new(state_blob_size).map_err(|_| ComputeApiError::OutOfMemory)?;
        let axons_blob =
            AlignedBuffer::new(axons_blob_size).map_err(|_| ComputeApiError::OutOfMemory)?;

        let const_zero_variant = layout::VariantParameters {
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
            synapse_refractory_period: 0,
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

        let resource = HostResource {
            spec,
            state_blob,
            axons_blob,
            variant_table: [const_zero_variant; layout::VARIANT_LUT_LEN],
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
            BackendKind::Cpu,
            raw_id,
            generation,
        ))
    }

    /// Validates handle and returns a mutable reference to the occupied host resource.
    pub fn get_resource_mut(
        &mut self,
        handle: VramHandle,
    ) -> Result<&mut HostResource, ComputeApiError> {
        if handle.kind() != BackendKind::Cpu {
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

    /// Uploads state and axon blobs and copies the variant table into backend-owned memory.
    pub fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        let resource = self.get_resource_mut(handle)?;
        validation::validate_upload(&resource.spec, &upload)?;

        resource
            .state_blob
            .as_slice_mut()
            .copy_from_slice(upload.state_blob);
        resource
            .axons_blob
            .as_slice_mut()
            .copy_from_slice(upload.axons_blob);
        resource.variant_table.copy_from_slice(upload.variant_table);
        resource.uploaded = true;

        Ok(())
    }

    /// Frees the resource associated with the provided handle.
    pub fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        if handle.kind() != BackendKind::Cpu {
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
                    self.slots[slot_idx] = ResourceSlot::Freed { generation: gen };
                    Ok(())
                }
            }
        }
    }

    /// Clears all allocated resources and increments generation counters for all slots to invalidate outstanding handles.
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
