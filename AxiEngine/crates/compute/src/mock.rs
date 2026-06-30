//! Mock backend implementation for testing lifecycle state changes and validations.

use compute_api::{
    expected_axons_blob_size, validate_alloc_spec, validate_day_batch_cmd,
    validate_snapshot_buffers, validate_upload, BackendCapabilities, BackendKind, BatchResult,
    ComputeApiError, ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload,
    VramHandle,
};
use std::num::NonZeroU64;

/// A mock compute backend implementing the core HAL traits.
pub struct MockBackend {
    slots: Vec<ResourceSlot>,
    is_teardown: bool,
}

#[allow(dead_code)]
enum ResourceSlot {
    Empty,
    Occupied {
        generation: u32,
        spec: ShardAllocSpec,
        state_blob: Vec<u8>,
        axons_blob: Vec<u8>,
    },
    Freed {
        generation: u32,
    },
}

impl MockBackend {
    /// Creates a new instance of the mock backend.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            is_teardown: false,
        }
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
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
        if self.is_teardown {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        validate_alloc_spec(&spec)?;

        let id = self.slots.len() as u32;
        let generation = 1;
        self.slots.push(ResourceSlot::Occupied {
            generation,
            spec,
            state_blob: vec![0; layout::calculate_state_blob_size(spec.padded_n as usize)],
            axons_blob: vec![0; expected_axons_blob_size(spec.total_axons)?],
        });

        let raw_id = NonZeroU64::new((id + 1) as u64).unwrap();
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
        if self.is_teardown {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        if handle.kind() != BackendKind::Mock {
            return Err(ComputeApiError::ForeignHandle);
        }

        let slot_idx = (handle.id().get() - 1) as usize;
        if slot_idx >= self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        match &mut self.slots[slot_idx] {
            ResourceSlot::Occupied {
                generation,
                spec,
                state_blob,
                axons_blob,
            } => {
                if *generation != handle.generation() {
                    return Err(ComputeApiError::InvalidHandle);
                }
                validate_upload(spec, &upload)?;
                state_blob.copy_from_slice(upload.state_blob);
                axons_blob.copy_from_slice(upload.axons_blob);
                Ok(())
            }
            ResourceSlot::Freed { generation } => {
                if *generation == handle.generation() {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
        }
    }

    fn run_day_batch(
        &mut self,
        handle: VramHandle,
        cmd: DayBatchCmd<'_>,
    ) -> Result<BatchResult, ComputeApiError> {
        if self.is_teardown {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        if handle.kind() != BackendKind::Mock {
            return Err(ComputeApiError::ForeignHandle);
        }

        let slot_idx = (handle.id().get() - 1) as usize;
        if slot_idx >= self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        match &self.slots[slot_idx] {
            ResourceSlot::Occupied { generation, .. } => {
                if *generation != handle.generation() {
                    return Err(ComputeApiError::InvalidHandle);
                }
                validate_day_batch_cmd(&cmd)?;

                let batch_ticks = cmd.sync_batch_ticks as usize;
                let max_spikes = cmd.max_spikes_per_tick as usize;
                let total_needed = batch_ticks * max_spikes;
                if cmd.output_spikes.len() < total_needed {
                    return Err(ComputeApiError::CapacityExceeded);
                }

                for val in &mut cmd.output_spikes[..total_needed] {
                    *val = 0xFFFF_FFFF;
                }
                for count in &mut cmd.output_spike_counts[..batch_ticks] {
                    *count = 0;
                }

                Ok(BatchResult {
                    ticks_executed: cmd.sync_batch_ticks,
                    generated_spikes_count: 0,
                    output_spikes_written: 0,
                    dropped_spikes_count: 0,
                    execution_time_us: 10,
                })
            }
            ResourceSlot::Freed { generation } => {
                if *generation == handle.generation() {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
        }
    }

    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError> {
        if self.is_teardown {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        if handle.kind() != BackendKind::Mock {
            return Err(ComputeApiError::ForeignHandle);
        }

        let slot_idx = (handle.id().get() - 1) as usize;
        if slot_idx >= self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        match &mut self.slots[slot_idx] {
            ResourceSlot::Occupied { generation, .. } => {
                if *generation != handle.generation() {
                    return Err(ComputeApiError::InvalidHandle);
                }
                let gen = *generation;
                self.slots[slot_idx] = ResourceSlot::Freed { generation: gen };
                Ok(())
            }
            ResourceSlot::Freed { generation } => {
                if *generation == handle.generation() {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
        }
    }

    fn teardown(&mut self) -> Result<(), ComputeApiError> {
        self.is_teardown = true;
        for slot in &mut self.slots {
            if let ResourceSlot::Occupied { generation, .. } = slot {
                let gen = *generation;
                *slot = ResourceSlot::Freed { generation: gen };
            }
        }
        Ok(())
    }

    fn debug_snapshot(
        &mut self,
        handle: VramHandle,
        snapshot: ShardSnapshotMut<'_>,
    ) -> Result<(), ComputeApiError> {
        if self.is_teardown {
            return Err(ComputeApiError::BackendNotInitialized);
        }
        if handle.kind() != BackendKind::Mock {
            return Err(ComputeApiError::ForeignHandle);
        }

        let slot_idx = (handle.id().get() - 1) as usize;
        if slot_idx >= self.slots.len() {
            return Err(ComputeApiError::InvalidHandle);
        }

        match &self.slots[slot_idx] {
            ResourceSlot::Occupied {
                generation,
                spec,
                state_blob,
                axons_blob,
            } => {
                if *generation != handle.generation() {
                    return Err(ComputeApiError::InvalidHandle);
                }
                validate_snapshot_buffers(spec, &snapshot)?;
                snapshot.state_blob.copy_from_slice(state_blob);
                snapshot.axons_blob.copy_from_slice(axons_blob);
                Ok(())
            }
            ResourceSlot::Freed { generation } => {
                if *generation == handle.generation() {
                    Err(ComputeApiError::AlreadyFreed)
                } else {
                    Err(ComputeApiError::InvalidHandle)
                }
            }
            ResourceSlot::Empty => Err(ComputeApiError::InvalidHandle),
        }
    }
}
