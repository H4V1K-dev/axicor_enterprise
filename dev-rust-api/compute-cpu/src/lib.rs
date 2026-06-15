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
}
