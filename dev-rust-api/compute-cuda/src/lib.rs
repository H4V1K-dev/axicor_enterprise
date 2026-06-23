pub mod ffi;
pub mod backend;

#[cfg(feature = "mock-gpu")]
pub mod mock;

pub use backend::CudaBackend;

#[cfg(test)]
mod tests {
    use super::*;
    use compute_api::{ComputeApiError, DayBatchCmd, GhostPatch, GpuBackend, ShardLayout, VramHandle};
    use std::sync::atomic::Ordering;

    #[test]
    fn test_cuda_device_presence() {
        let backend = CudaBackend::new(0);
        // On mock builds this must succeed, on native builds it depends on GPU status
        #[cfg(feature = "mock-gpu")]
        {
            assert!(backend.is_ok());
            let backend = backend.unwrap();
            assert_eq!(backend.device_id, 0);
        }
    }

    #[test]
    fn test_cuda_alloc_free() {
        let backend = CudaBackend::new(0).unwrap();
        let layout = ShardLayout {
            padded_n: 128,
            total_axons: 100,
            total_ghosts: 10,
        };

        let handle = backend.alloc_shard(&layout).unwrap();
        assert_ne!(handle.0, compute_api::INVALID_VRAM_HANDLE);

        // Verify SoA alignment boundary (64-byte alignment check)
        {
            let registry = backend.resources.read().unwrap();
            let key = slotmap::KeyData::from_ffi(handle.0).into();
            let res = registry.get(key).unwrap();
            
            assert_eq!(res.vram_ptrs.soma_voltage as usize % 64, 0);
            assert_eq!(res.vram_ptrs.flags as usize % 64, 0);
            assert_eq!(res.vram_ptrs.threshold_offset as usize % 64, 0);
            assert_eq!(res.vram_ptrs.timers as usize % 64, 0);
            assert_eq!(res.vram_ptrs.soma_to_axon as usize % 64, 0);
            assert_eq!(res.vram_ptrs.dendrite_targets as usize % 64, 0);
            assert_eq!(res.vram_ptrs.dendrite_weights as usize % 64, 0);
            assert_eq!(res.vram_ptrs.dendrite_timers as usize % 64, 0);
        }

        backend.free(handle);
    }

    #[test]
    fn test_cuda_pinned_allocation() {
        let backend = CudaBackend::new(0).unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };

        let handle = backend.alloc_shard(&layout).unwrap();
        {
            let registry = backend.resources.read().unwrap();
            let key = slotmap::KeyData::from_ffi(handle.0).into();
            let res = registry.get(key).unwrap();
            
            assert!(!res.pinned_output_ptr.is_null());
            assert!(!res.pinned_telemetry_ids_ptr.is_null());
            assert!(!res.pinned_telemetry_count_ptr.is_null());
        }
        backend.free(handle);
    }

    #[test]
    fn test_cuda_invalid_handle_checks() {
        let backend = CudaBackend::new(0).unwrap();
        let bad_handle = VramHandle(99999);
        
        use compute_api::GpuBackend;
        assert_eq!(backend.upload_state(&bad_handle, &[]), Err(ComputeApiError::InvalidHandle));
        assert_eq!(backend.upload_variants(&bad_handle, &[]), Err(ComputeApiError::InvalidHandle));
        
        let spike_counts = vec![0];
        let mapped_somas = vec![];
        let cmd = DayBatchCmd {
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
        assert_eq!(backend.run_day_batch(&bad_handle, &cmd), Err(ComputeApiError::InvalidHandle));
        assert_eq!(backend.download_output(&bad_handle).map(|_| ()), Err(ComputeApiError::InvalidHandle));
        assert_eq!(backend.download_telemetry(&bad_handle).map(|_| ()), Err(ComputeApiError::InvalidHandle));
        assert_eq!(backend.patch_ghosts(&bad_handle, &[]), Err(ComputeApiError::InvalidHandle));
        assert_eq!(backend.run_sort_and_prune(&bad_handle, 0), Err(ComputeApiError::InvalidHandle));
    }

    #[test]
    fn test_cuda_compute_capability_check() {
        #[cfg(feature = "mock-gpu")]
        {
            use mock::COMPUTE_CAPABILITY_MAJOR;
            
            // Set compute capability below Pascal (6.1)
            COMPUTE_CAPABILITY_MAJOR.with(|f| f.store(5, Ordering::Relaxed));
            let res = CudaBackend::new(0);
            COMPUTE_CAPABILITY_MAJOR.with(|f| f.store(7, Ordering::Relaxed)); // restore default
            
            assert_eq!(res.err(), Some(ComputeApiError::VendorError(801)));
        }
    }

    #[test]
    fn test_cuda_launch_parameter_error() {
        #[cfg(feature = "mock-gpu")]
        {
            use compute_api::GpuBackend;
            let backend = CudaBackend::new(0).unwrap();
            let layout = ShardLayout {
                padded_n: 99968, // Specially-handled size in mock to trigger resource error
                total_axons: 100,
                total_ghosts: 10,
            };
            let handle = backend.alloc_shard(&layout).unwrap();
            
            let spike_counts = vec![0];
            let mapped_somas = vec![];
            let cmd = DayBatchCmd {
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
            
            let res = backend.run_day_batch(&handle, &cmd);
            assert_eq!(res.err(), Some(ComputeApiError::InvalidLayout));
            backend.free(handle);
        }
    }

    #[test]
    fn test_cuda_slice_mismatch() {
        use compute_api::GpuBackend;
        let backend = CudaBackend::new(0).unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let handle = backend.alloc_shard(&layout).unwrap();

        // 1. spike_counts length mismatch (sync_batch_ticks = 2, spike_counts = 1)
        let spike_counts = vec![0];
        let mapped_somas = vec![];
        let cmd = DayBatchCmd {
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
        
        let res = backend.run_day_batch(&handle, &cmd);
        assert_eq!(res, Err(ComputeApiError::InvalidLayout));
        backend.free(handle);
    }

    #[test]
    fn test_cuda_ghost_capacity_overflow() {
        use compute_api::GpuBackend;
        let backend = CudaBackend::new(0).unwrap();
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 5,
        };
        let handle = backend.alloc_shard(&layout).unwrap();

        // Valid: dst_ghost < 5
        let patch_ok = GhostPatch::Add { src_axon: 10, dst_ghost: 4 };
        assert!(backend.patch_ghosts(&handle, &[patch_ok]).is_ok());

        // Invalid: dst_ghost >= 5
        let patch_fail = GhostPatch::Add { src_axon: 10, dst_ghost: 5 };
        assert_eq!(backend.patch_ghosts(&handle, &[patch_fail]), Err(ComputeApiError::CapacityExceeded));
        
        backend.free(handle);
    }

    #[test]
    fn test_cuda_pinned_ram_exhaustion() {
        #[cfg(feature = "mock-gpu")]
        {
            use compute_api::GpuBackend;
            use mock::FAIL_PINNED_ALLOC;

            let backend = CudaBackend::new(0).unwrap();
            let layout = ShardLayout {
                padded_n: 64,
                total_axons: 100,
                total_ghosts: 10,
            };

            FAIL_PINNED_ALLOC.with(|f| f.store(true, Ordering::Relaxed));
            let res = backend.alloc_shard(&layout);
            FAIL_PINNED_ALLOC.with(|f| f.store(false, Ordering::Relaxed));

            assert_eq!(res.err(), Some(ComputeApiError::OutOfMemory));
        }
    }

    #[test]
    fn test_cuda_compile_assertions() {
        // Assert sizes and alignments for ShardVramPtrs (INV-CROSS-007 verification)
        assert_eq!(std::mem::size_of::<layout::ShardVramPtrs>(), 80);
        assert_eq!(std::mem::align_of::<layout::ShardVramPtrs>(), 8);

        // Check that CUDA source files contain core_math.h (INV-COMPUTE-CUDA-003 validation)
        let cu_path = std::path::Path::new("src/cuda/physics.cu");
        if cu_path.exists() {
            let content = std::fs::read_to_string(cu_path).unwrap();
            assert!(
                content.contains("#include \"core_math.h\""),
                "physics.cu must include core_math.h"
            );
        }
    }
}