use std::sync::RwLock;
use slotmap::{SlotMap, DefaultKey, Key, KeyData};
use compute_api::{GpuBackend, VramHandle, ShardLayout, DayBatchCmd, BatchResult, OutputFrame, TelemetryFrame, GhostPatch, ComputeApiError};
use layout::VariantParameters;
use crate::memory::ShardCpuResources;

/// CPU compute backend implementing the `GpuBackend` interface.
///
/// Under INV-COMPUTE-CPU-001, this backend encapsulates shard allocations in an internal
/// thread-safe registry (`SlotMap` inside `RwLock`), preventing cross-shard access and ensuring
/// absolute memory isolation between shards.
pub struct CpuBackend {
    pub(crate) resources: RwLock<SlotMap<DefaultKey, ShardCpuResources>>,
}

impl CpuBackend {
    /// Initializes a new CPU-based computing backend.
    pub fn new() -> Result<Self, ComputeApiError> {
        Ok(Self {
            resources: RwLock::new(SlotMap::new()),
        })
    }

    /// Downloads the entire raw memory state of the shard.
    pub fn download_raw_state(&self, handle: &VramHandle) -> Result<Vec<u8>, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;
        Ok(resource.as_slice().to_vec())
    }
}

pub(crate) fn key_to_handle(key: DefaultKey) -> VramHandle {
    VramHandle(key.data().as_ffi())
}

pub(crate) fn handle_to_key(handle: &VramHandle) -> DefaultKey {
    KeyData::from_ffi(handle.0).into()
}

impl GpuBackend for CpuBackend {
    /// Allocates a 64-byte aligned flat state buffer in host RAM for a simulation shard.
    ///
    /// Under INV-COMPUTE-CPU-002, the shard layout neuron count `padded_n` must be a multiple of 64
    /// to ensure alignment with CPU cache lines, preventing cache false-sharing and enabling SIMD.
    fn alloc_shard(&self, layout: &ShardLayout) -> Result<VramHandle, ComputeApiError> {
        // Padded neuron count must be a multiple of 64
        if layout.padded_n % 64 != 0 {
            return Err(ComputeApiError::InvalidLayout);
        }

        // Compute state offsets and the total monolithic size needed
        let offsets = layout::compute_state_offsets(layout.padded_n as usize);

        // Allocate the resources with 64-byte alignment
        let resource = ShardCpuResources::new(offsets.total_size, layout.clone())?;

        let mut guard = self.resources.write().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = guard.insert(resource);
        Ok(key_to_handle(key))
    }

    fn upload_state(&self, handle: &VramHandle, state: &[u8]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let mut guard = self.resources.write().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get_mut(key).ok_or(ComputeApiError::InvalidHandle)?;
        let slice = resource.as_mut_slice();
        let len = state.len().min(slice.len());
        slice[..len].copy_from_slice(&state[..len]);
        Ok(())
    }

    fn upload_variants(&self, handle: &VramHandle, _variants: &[VariantParameters]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Variant upload logic
        Ok(())
    }

    fn run_day_batch(&self, handle: &VramHandle, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // E-062: Validate spike_counts slice length
        if cmd.spike_counts.len() as u32 != cmd.sync_batch_ticks {
            return Err(ComputeApiError::InvalidLayout);
        }

        // SAFETY: The orchestrator guarantees exclusive thread access to this shard's VRAM handle
        // during execution of the run_day_batch HFT epoch, preventing data races.
        let (voltage, flags, threshold_offset, timers) = unsafe { resource.extract_soa_slices() };

        use rayon::prelude::*;

        for _tick in 0..cmd.sync_batch_ticks {
            // Iterate in parallel over chunks of 16 neurons to prevent false sharing.
            //
            // Under INV-COMPUTE-CPU-006, chunk size of 16 elements (64 bytes for i32/u32) aligns
            // perfectly with L1/L2 cache lines, preventing MESI cache invalidations.
            voltage.par_chunks_mut(16)
                .zip(flags.par_chunks_mut(16))
                .zip(threshold_offset.par_chunks_mut(16))
                .zip(timers.par_chunks_mut(16))
                .for_each(|(((v_chunk, f_chunk), to_chunk), t_chunk)| {
                    // GLIF/GSOP physical integration step (physics calculation stub)
                    // Will be fully emulated in subsequent iterations.
                    let _ = (v_chunk, f_chunk, to_chunk, t_chunk);
                });
        }

        Ok(BatchResult {
            ticks_processed: cmd.sync_batch_ticks,
            is_warmup: false,
        })
    }

    fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Download output frame
        Ok(OutputFrame {
            data: vec![],
            num_outputs: 0,
            sync_batch_ticks: 0,
        })
    }

    fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Download telemetry
        Ok(TelemetryFrame {
            active_soma_ids: vec![],
            total_spikes: 0,
        })
    }

    fn patch_ghosts(&self, handle: &VramHandle, _patches: &[GhostPatch]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Patch ghosts
        Ok(())
    }

    fn run_sort_and_prune(&self, handle: &VramHandle, _prune_threshold: i16) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Run sort and prune
        Ok(())
    }

    fn free(&self, handle: VramHandle) {
        let key = handle_to_key(&handle);
        if let Ok(mut guard) = self.resources.write() {
            guard.remove(key);
        }
    }
}
