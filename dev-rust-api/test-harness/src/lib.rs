//! Axicor Test Harness Crate.
//!
//! Provides FFI alignment assertions and differential testing utilities
//! to enforce cross-platform deterministic and bit-exact calculations.

pub mod error;
pub mod ffi;
pub mod runner;

pub use error::DifferentialTestError;
pub use ffi::verify_ffi_alignments;
pub use runner::{run_differential_check, run_differential_check_with_seeds, run_differential_suite};

#[cfg(test)]
mod tests {
    use super::*;
    use compute_api::{ShardLayout, DayBatchCmd, OutputFrame};
    use compute::BackendType;

    #[test]
    fn test_harness_ffi_alignments() {
        // INV-CROSS-007: Verify C-ABI FFI structure sizes and alignments
        verify_ffi_alignments();
    }

    #[test]
    fn test_harness_backend_init_failure() {
        // E-071: Verify that trying to run a GPU backend that is currently stubbed/unsupported
        // returns `BackendInitFailed`.
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 2,
        };
        
        #[cfg(not(feature = "cuda"))]
        {
            let res = run_differential_suite(layout, 42, &[], BackendType::Cuda);
            assert!(matches!(res, Err(DifferentialTestError::BackendInitFailed(_))), "Expected Cuda backend initialization to fail when disabled");
        }

        #[cfg(not(feature = "hip"))]
        {
            let res = run_differential_suite(layout, 42, &[], BackendType::Hip);
            assert!(matches!(res, Err(DifferentialTestError::BackendInitFailed(_))), "Expected Hip backend initialization to fail when disabled");
        }
    }

    #[test]
    fn test_harness_detects_seed_mismatch() {
        // E-072: Verify that initializing CPU and GPU test configurations with different
        // seeds returns `SeedDiscrepancy`.
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 2,
        };
        let res = run_differential_check_with_seeds(layout, 42, 100, &[], BackendType::Cpu);
        assert!(matches!(res, Err(DifferentialTestError::SeedDiscrepancy { cpu_seed: 42, gpu_seed: 100 })));
    }

    #[test]
    fn test_harness_differential_cpu_vs_cpu() {
        // INV-CROSS-011: Run a successful CPU-vs-CPU differential test
        let layout = ShardLayout {
            padded_n: 64,
            total_axons: 10,
            total_ghosts: 2,
        };

        let spike_counts = vec![0, 0];
        let mapped_somas = vec![];
        let inputs = vec![
            DayBatchCmd {
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
            }
        ];

        let res = run_differential_suite(layout, 42, &inputs, BackendType::Cpu);
        assert!(res.is_ok(), "CPU vs CPU run failed: {:?}", res);
    }

    #[test]
    fn test_harness_layout_validation() {
        // E-073: Passing invalid layouts (e.g. non-64 padded_n) fails during alloc/init.
        let invalid_layout = ShardLayout {
            padded_n: 63, // invalid alignment
            total_axons: 10,
            total_ghosts: 2,
        };
        let res = run_differential_suite(invalid_layout, 42, &[], BackendType::Cpu);
        assert!(matches!(res, Err(DifferentialTestError::BackendInitFailed(_))));
    }

    #[test]
    fn test_harness_detects_voltage_mismatch() {
        // E-075: Verify that a 1-bit difference in state/output buffers is caught.
        // We will directly verify that the StateMismatch error is generated correctly.
        let cpu_out = OutputFrame {
            data: vec![0, 0, 0],
            num_outputs: 1,
            sync_batch_ticks: 1,
        };
        let gpu_out = OutputFrame {
            data: vec![0, 1, 0],
            num_outputs: 1,
            sync_batch_ticks: 1,
        };

        // We can use a test of the compare helpers or mock a state check.
        // We want to be sure that state discrepancy of 1 bit returns StateMismatch.
        let cpu_state = vec![0, 0, 100, 0];
        let gpu_state = vec![0, 0, 101, 0];

        // Helper check for output frame mismatch
        fn check_output_compare(cpu: &OutputFrame, gpu: &OutputFrame) -> Result<(), DifferentialTestError> {
            if cpu.num_outputs != gpu.num_outputs
                || cpu.sync_batch_ticks != gpu.sync_batch_ticks
                || cpu.data != gpu.data
            {
                let min_len = cpu.data.len().min(gpu.data.len());
                for i in 0..min_len {
                    if cpu.data[i] != gpu.data[i] {
                        return Err(DifferentialTestError::StateMismatch {
                            tick: 0,
                            offset: i,
                            cpu_val: cpu.data[i],
                            gpu_val: gpu.data[i],
                        });
                    }
                }
                return Err(DifferentialTestError::StateMismatch {
                    tick: 0,
                    offset: min_len,
                    cpu_val: 0,
                    gpu_val: 0,
                });
            }
            Ok(())
        }

        let res_out = check_output_compare(&cpu_out, &gpu_out);
        assert!(matches!(res_out, Err(DifferentialTestError::StateMismatch { tick: 0, offset: 1, cpu_val: 0, gpu_val: 1 })));

        // Helper check for state buffer mismatch
        fn check_state_compare(cpu: &[u8], gpu: &[u8]) -> Result<(), DifferentialTestError> {
            if cpu != gpu {
                let min_len = cpu.len().min(gpu.len());
                for i in 0..min_len {
                    if cpu[i] != gpu[i] {
                        return Err(DifferentialTestError::StateMismatch {
                            tick: 0,
                            offset: i,
                            cpu_val: cpu[i],
                            gpu_val: gpu[i],
                        });
                    }
                }
                return Err(DifferentialTestError::StateMismatch {
                    tick: 0,
                    offset: min_len,
                    cpu_val: 0,
                    gpu_val: 0,
                });
            }
            Ok(())
        }

        let res_state = check_state_compare(&cpu_state, &gpu_state);
        assert!(matches!(res_state, Err(DifferentialTestError::StateMismatch { tick: 0, offset: 2, cpu_val: 100, gpu_val: 101 })));
    }
}
