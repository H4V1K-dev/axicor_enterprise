pub mod error;
pub mod traits;
pub mod types;

pub use error::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use layout::VariantParameters;

    struct MockBackend;

    impl GpuBackend for MockBackend {
        fn alloc_shard(&self, layout: &ShardLayout) -> Result<VramHandle, ComputeApiError> {
            // INV-CROSS-010: padded_n must be a multiple of 64
            if layout.padded_n % 64 != 0 {
                return Err(ComputeApiError::InvalidLayout);
            }
            Ok(VramHandle(1))
        }

        fn upload_state(&self, handle: &VramHandle, _state: &[u8]) -> Result<(), ComputeApiError> {
            // INV-COMPUTE-API-006: reject invalid handles
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(())
        }

        fn upload_variants(&self, handle: &VramHandle, _variants: &[VariantParameters]) -> Result<(), ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(())
        }

        fn run_day_batch(&self, handle: &VramHandle, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }

            // E-050: Slice Length Mismatch
            if cmd.spike_counts.len() != cmd.sync_batch_ticks as usize {
                return Err(ComputeApiError::InvalidLayout);
            }

            if let Some(mask) = cmd.input_bitmask {
                let required_bits = (cmd.num_virtual_axons * cmd.sync_batch_ticks) as usize;
                if mask.len() * 8 < required_bits {
                    return Err(ComputeApiError::InvalidLayout);
                }
            }

            // INV-COMPUTE-API-007, E-052: Ephys Targets Overflow
            if let Some(ref ephys) = cmd.ephys_cmd {
                if ephys.count > MAX_EPHYS_TARGETS {
                    return Err(ComputeApiError::InvalidLayout);
                }
            }

            Ok(BatchResult {
                ticks_processed: cmd.sync_batch_ticks,
                is_warmup: false,
            })
        }

        fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(OutputFrame {
                data: vec![0; 16],
                num_outputs: 8,
                sync_batch_ticks: 2,
            })
        }

        fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(TelemetryFrame {
                active_soma_ids: vec![],
                total_spikes: 0,
            })
        }

        fn patch_ghosts(&self, handle: &VramHandle, _patches: &[GhostPatch]) -> Result<(), ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(())
        }

        fn run_sort_and_prune(&self, handle: &VramHandle, _prune_threshold: i16) -> Result<(), ComputeApiError> {
            if handle.0 == INVALID_VRAM_HANDLE {
                return Err(ComputeApiError::InvalidHandle);
            }
            Ok(())
        }

        fn free(&self, _handle: VramHandle) {
            // Explicit teardown lifecycle check under INV-COMPUTE-API-003
        }
    }

    #[test]
    fn test_gpu_backend_object_safety() {
        // INV-COMPUTE-API-001
        fn check_object_safety(_b: Box<dyn GpuBackend>) {}
        let backend = Box::new(MockBackend);
        check_object_safety(backend);
    }

    #[test]
    fn test_vram_handle_opaqueness() {
        // INV-COMPUTE-API-002
        assert_eq!(std::mem::size_of::<VramHandle>(), 8);
        assert_eq!(std::mem::align_of::<VramHandle>(), 8);
    }

    #[test]
    fn test_day_batch_cmd_lifetimes() {
        // INV-COMPUTE-API-004
        let input_bitmask = vec![0b10101010];
        let incoming_spikes = vec![0b01010101];
        let spike_counts = vec![2, 3];
        let mapped_soma_ids = vec![1, 2];

        let cmd = DayBatchCmd {
            tick_base: 100,
            sync_batch_ticks: 2,
            v_seg: 0,
            global_dopamine: 15,
            virtual_offset: 10,
            num_virtual_axons: 4,
            num_outputs: 2,
            input_bitmask: Some(&input_bitmask),
            incoming_spikes: Some(&incoming_spikes),
            spike_counts: &spike_counts,
            mapped_soma_ids: &mapped_soma_ids,
            ephys_cmd: None,
        };

        assert_eq!(cmd.tick_base, 100);
        assert_eq!(cmd.sync_batch_ticks, 2);
    }

    #[test]
    fn test_compute_api_error_traits() {
        // INV-COMPUTE-API-005
        let err = ComputeApiError::OutOfMemory;
        let cloned = err.clone();
        assert_eq!(err, cloned);

        fn check_send_sync<T: Send + Sync>() {}
        check_send_sync::<ComputeApiError>();
    }

    #[test]
    fn test_vram_handle_isolation() {
        // INV-COMPUTE-API-006
        let backend = MockBackend;
        let invalid_handle = VramHandle(INVALID_VRAM_HANDLE);
        let res = backend.upload_state(&invalid_handle, &[]);
        assert_eq!(res, Err(ComputeApiError::InvalidHandle));
    }

    #[test]
    fn test_ephys_bounds_protection() {
        // INV-COMPUTE-API-007, E-052
        let backend = MockBackend;
        let handle = VramHandle(1);
        let spike_counts = vec![0];
        let mapped_soma_ids = vec![];
        let ephys = EphysCmd {
            tids_d: std::ptr::null(),
            uvs_d: std::ptr::null(),
            trace_d: std::ptr::null_mut(),
            count: MAX_EPHYS_TARGETS + 1,
            max_ticks: 100,
            current_tick: 0,
        };

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
            mapped_soma_ids: &mapped_soma_ids,
            ephys_cmd: Some(ephys),
        };

        let res = backend.run_day_batch(&handle, &cmd);
        assert_eq!(res, Err(ComputeApiError::InvalidLayout));
    }

    #[test]
    fn test_slice_length_mismatch() {
        // E-050
        let backend = MockBackend;
        let handle = VramHandle(1);
        let spike_counts = vec![0];
        let mapped_soma_ids = vec![];

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
            mapped_soma_ids: &mapped_soma_ids,
            ephys_cmd: None,
        };

        let res = backend.run_day_batch(&handle, &cmd);
        assert_eq!(res, Err(ComputeApiError::InvalidLayout));
    }

    #[test]
    fn test_invalid_layout_alignment() {
        // INV-CROSS-010
        let backend = MockBackend;

        let bad_layout = ShardLayout {
            padded_n: 63,
            total_axons: 100,
            total_ghosts: 10,
        };
        let res = backend.alloc_shard(&bad_layout);
        assert_eq!(res, Err(ComputeApiError::InvalidLayout));

        let good_layout = ShardLayout {
            padded_n: 64,
            total_axons: 100,
            total_ghosts: 10,
        };
        let res = backend.alloc_shard(&good_layout);
        assert!(res.is_ok());
    }
}
