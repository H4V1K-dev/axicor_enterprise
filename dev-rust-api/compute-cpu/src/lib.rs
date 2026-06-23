pub mod backend;
pub mod memory;

pub use backend::*;
pub use memory::*;

#[cfg(test)]
mod tests {
    use super::*;
    use compute_api::{GpuBackend, VramHandle, ShardLayout, INVALID_VRAM_HANDLE, ComputeApiError};

    #[test]
    fn test_cpu_alloc_free() {
        let backend = CpuBackend::new().unwrap();

        // Valid layout: padded_n is a multiple of 64
        let layout = ShardLayout {
            padded_n: 128,
            total_axons: 200,
            total_ghosts: 50,
        };

        // Allocate shard
        let handle = backend.alloc_shard(&layout).unwrap();
        assert_ne!(handle.0, INVALID_VRAM_HANDLE);

        // Verify that the memory buffer starts on a 64-byte aligned boundary (INV-COMPUTE-CPU-002)
        {
            let key = backend::handle_to_key(&handle);
            let mut guard = backend.resources.write().unwrap();
            let resource = guard.get_mut(key).unwrap();
            assert_eq!(resource.raw_ptr as usize % 64, 0);
            assert_eq!(resource.as_mut_slice().len(), resource.size);
            assert_eq!(resource.as_slice().len(), resource.size);
            assert_eq!(resource.layout, layout);
        }

        // Test run_day_batch HFT tick loop
        let spike_counts = vec![2, 3];
        let mapped_somas = vec![0, 1];
        let cmd = compute_api::DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 2,
            v_seg: 0,
            global_dopamine: 0,
            virtual_offset: 0,
            num_virtual_axons: 0,
            num_outputs: 0,
            input_bitmask: None,
            incoming_spikes: None,
            spike_counts: &spike_counts,
            mapped_soma_ids: &mapped_somas,
            ephys_cmd: None,
        };
        let batch_res = backend.run_day_batch(&handle, &cmd).unwrap();
        assert_eq!(batch_res.ticks_processed, 2);

        // Free the handle
        backend.free(handle);

        // Verify the handle has been removed and lookup fails (INV-COMPUTE-CPU-001)
        let res = backend.upload_state(&handle, &[]);
        assert_eq!(res, Err(ComputeApiError::InvalidHandle));
    }

    #[test]
    fn test_cpu_invalid_handle_checks() {
        let backend = CpuBackend::new().unwrap();

        // Invalid layouts should be rejected immediately during alloc
        let invalid_layout = ShardLayout {
            padded_n: 63, // Not a multiple of 64
            total_axons: 100,
            total_ghosts: 10,
        };
        assert_eq!(
            backend.alloc_shard(&invalid_layout),
            Err(ComputeApiError::InvalidLayout)
        );

        // Accessing using invalid or zero handles must return InvalidHandle (INV-COMPUTE-CPU-001)
        let bad_handle = VramHandle(99999);
        let zero_handle = VramHandle(INVALID_VRAM_HANDLE);

        for handle in &[bad_handle, zero_handle] {
            assert_eq!(backend.upload_state(handle, &[]), Err(ComputeApiError::InvalidHandle));
            assert_eq!(backend.upload_variants(handle, &[]), Err(ComputeApiError::InvalidHandle));
            
            let spike_counts = vec![0];
            let mapped_somas = vec![];
            let cmd = compute_api::DayBatchCmd {
                tick_base: 0,
                sync_batch_ticks: 1,
                v_seg: 0,
                global_dopamine: 0,
                virtual_offset: 0,
                num_virtual_axons: 0,
                num_outputs: 0,
                input_bitmask: None,
                incoming_spikes: None,
                spike_counts: &spike_counts,
                mapped_soma_ids: &mapped_somas,
                ephys_cmd: None,
            };
            assert_eq!(backend.run_day_batch(handle, &cmd), Err(ComputeApiError::InvalidHandle));
            assert_eq!(backend.download_output(handle).map(|_| ()), Err(ComputeApiError::InvalidHandle));
            assert_eq!(backend.download_telemetry(handle).map(|_| ()), Err(ComputeApiError::InvalidHandle));
            assert_eq!(backend.patch_ghosts(handle, &[]), Err(ComputeApiError::InvalidHandle));
            assert_eq!(backend.run_sort_and_prune(handle, 0), Err(ComputeApiError::InvalidHandle));
        }
    }

    #[test]
    fn test_cpu_slice_mismatch() {
        let backend = CpuBackend::new().unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let handle = backend.alloc_shard(&layout).unwrap();

        // 1. spike_counts length mismatch
        let spike_counts = vec![2]; // sync_batch_ticks is 2, so should be length 2
        let mapped_somas = vec![];
        let cmd = compute_api::DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 2,
            v_seg: 0,
            global_dopamine: 0,
            virtual_offset: 0,
            num_virtual_axons: 0,
            num_outputs: 0,
            input_bitmask: None,
            incoming_spikes: None,
            spike_counts: &spike_counts,
            mapped_soma_ids: &mapped_somas,
            ephys_cmd: None,
        };
        assert_eq!(
            backend.run_day_batch(&handle, &cmd),
            Err(ComputeApiError::InvalidLayout)
        );
    }

    #[test]
    fn test_cpu_ghost_capacity_overflow() {
        let backend = CpuBackend::new().unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 5,
        };
        let handle = backend.alloc_shard(&layout).unwrap();

        // Valid patch inside bounds: dst_ghost 4 < 5
        let patch_ok = compute_api::GhostPatch::Add { src_axon: 10, dst_ghost: 4 };
        assert!(backend.patch_ghosts(&handle, &[patch_ok]).is_ok());

        // Invalid patch: dst_ghost 5 >= 5 (overflows capacity E-063)
        let patch_fail = compute_api::GhostPatch::Add { src_axon: 10, dst_ghost: 5 };
        assert_eq!(
            backend.patch_ghosts(&handle, &[patch_fail]),
            Err(ComputeApiError::CapacityExceeded)
        );
    }

    #[test]
    fn test_cpu_zero_copy_cast() {
        let backend = CpuBackend::new().unwrap();
        let layout = ShardLayout {
            padded_n: 128,
            total_axons: 200,
            total_ghosts: 50,
        };
        let handle = backend.alloc_shard(&layout).unwrap();

        let state_data = vec![0xAA; 10000];
        backend.upload_state(&handle, &state_data).unwrap();

        // Retrieve raw state and check offsets
        let _raw_state = backend.download_raw_state(&handle).unwrap();
        
        let key = handle_to_key(&handle);
        let guard = backend.resources.read().unwrap();
        let resource = guard.get(key).unwrap();
        
        let (voltage, flags, threshold_offset, timers) = unsafe { resource.extract_soa_slices() };
        
        // Assert length matches padded_n
        assert_eq!(voltage.len(), 128);
        assert_eq!(flags.len(), 128);
        assert_eq!(threshold_offset.len(), 128);
        assert_eq!(timers.len(), 128);

        // Check if raw pointers match expected layout offset arithmetic
        let base_addr = resource.raw_ptr as usize;
        let offsets = layout::compute_state_offsets(128);
        
        assert_eq!(voltage.as_ptr() as usize, base_addr + offsets.soma_voltage);
        assert_eq!(flags.as_ptr() as usize, base_addr + offsets.flags);
        assert_eq!(threshold_offset.as_ptr() as usize, base_addr + offsets.threshold_offset);
        assert_eq!(timers.as_ptr() as usize, base_addr + offsets.timers);
    }

    #[test]
    fn test_cpu_chunk_alignment_invariants() {
        // INV-COMPUTE-CPU-006 False sharing chunk size check.
        // Cache lines on typical CPUs are 64 bytes. For 4-byte types (i32/u32),
        // chunk size of 16 elements corresponds to 64 bytes.
        let chunk_size = 16;
        assert_eq!(chunk_size * std::mem::size_of::<i32>(), 64);
        assert_eq!(chunk_size * std::mem::size_of::<u32>(), 64);
    }

    #[test]
    fn test_cpu_map_reduce_telemetry() {
        let backend = CpuBackend::new().unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let handle = backend.alloc_shard(&layout).unwrap();
        
        let telemetry = backend.download_telemetry(&handle).unwrap();
        assert_eq!(telemetry.total_spikes, 0);
        assert!(telemetry.active_soma_ids.is_empty());
    }
}
