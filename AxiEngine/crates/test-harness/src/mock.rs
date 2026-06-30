//! Conformance Mock compute backend implementation.

use compute_api::{
    BackendCapabilities, BackendKind, BatchResult, ComputeApiError, ComputeBackend, DayBatchCmd,
    ShardAllocSpec, ShardSnapshotMut, ShardUpload, VramHandle,
};
use core::num::NonZeroU64;

struct MockResource {
    spec: ShardAllocSpec,
    state: Vec<u8>,
    axons: Vec<u8>,
    uploaded: bool,
}

enum ResourceSlot {
    Empty,
    Occupied {
        generation: u32,
        resource: MockResource,
    },
    Freed {
        generation: u32,
    },
}

/// A lightweight mock backend for testing trait conformance and error mapping.
#[derive(Default)]
pub struct MockBackend {
    slots: Vec<ResourceSlot>,
}

impl MockBackend {
    /// Constructs a new `MockBackend` instance.
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn get_resource_mut(
        &mut self,
        handle: VramHandle,
    ) -> Result<&mut MockResource, ComputeApiError> {
        if handle.kind() != BackendKind::Mock {
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
}

impl ComputeBackend for MockBackend {
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

    fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError> {
        compute_api::validation::validate_alloc_spec(&spec)?;

        let resource = MockResource {
            spec,
            state: vec![0u8; layout::calculate_state_blob_size(spec.padded_n as usize)],
            axons: vec![0u8; compute_api::validation::expected_axons_blob_size(spec.total_axons)?],
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
            resource,
        };

        let raw_id =
            NonZeroU64::new((slot_idx as u64) + 1).ok_or(ComputeApiError::InvalidHandle)?;
        Ok(VramHandle::from_raw_parts(
            BackendKind::Mock,
            raw_id,
            generation,
        ))
    }

    fn upload_shard(
        &mut self,
        handle: VramHandle,
        upload: ShardUpload<'_>,
    ) -> Result<(), ComputeApiError> {
        let resource = self.get_resource_mut(handle)?;
        compute_api::validation::validate_upload(&resource.spec, &upload)?;
        resource.state.copy_from_slice(upload.state_blob);
        resource.axons.copy_from_slice(upload.axons_blob);
        resource.uploaded = true;
        Ok(())
    }

    fn run_day_batch(
        &mut self,
        handle: VramHandle,
        cmd: DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError> {
        let _resource = self.get_resource_mut(handle)?;
        compute_api::validation::validate_day_batch_cmd(&cmd)?;

        // Simulating minimal deterministic behavior: echo incoming spikes to output
        let ticks = cmd.sync_batch_ticks as usize;
        let max_spikes = cmd.max_spikes_per_tick as usize;
        let mut total_spikes = 0;

        for t in 0..ticks {
            let count = cmd.incoming_spike_counts[t];
            cmd.output_spike_counts[t] = count;
            total_spikes += count;
            let start = t * max_spikes;
            if let Some(in_spikes) = cmd.incoming_spikes {
                let end = start + count as usize;
                cmd.output_spikes[start..end].copy_from_slice(&in_spikes[start..end]);
            }
        }

        Ok(BatchResult {
            ticks_executed: cmd.sync_batch_ticks,
            generated_spikes_count: total_spikes,
            output_spikes_written: total_spikes,
            dropped_spikes_count: 0,
            execution_time_us: 123,
        })
    }

    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        if handle.kind() != BackendKind::Mock {
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

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
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

    fn debug_snapshot(
        &mut self,
        handle: VramHandle,
        snapshot: ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        let resource = self.get_resource_mut(handle)?;
        if !resource.uploaded {
            return Err(ComputeApiError::InvalidDebugProbeBounds);
        }
        compute_api::validation::validate_snapshot_buffers(&resource.spec, &snapshot)?;
        snapshot.state_blob.copy_from_slice(&resource.state);
        snapshot.axons_blob.copy_from_slice(&resource.axons);
        Ok(())
    }
}
