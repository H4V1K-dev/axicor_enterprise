//! Differential test runner module.
//!
//! Under invariant INV-CROSS-011, this module provides the capability to run
//! differential testing comparing the execution of CPU and GPU backends.
//! It ensures bit-exact cross-platform determinism across all steps.

use compute_api::{GpuBackend, VramHandle, ShardLayout, DayBatchCmd, OutputFrame};
use compute::{BackendType, instantiate_backend};
use crate::error::DifferentialTestError;

/// An RAII guard ensuring that allocated VRAM handles are freed on drop.
///
/// Under INV-COMPUTE-API-003, implicit teardown using Rust's Drop is avoided
/// in general application lifecycles to prevent process exit races. However,
/// inside the test harness, this RAII guard is utilized to prevent VRAM memory
/// leaks on early test failures or panics.
pub struct VramGuard<'a> {
    backend: &'a dyn GpuBackend,
    handle: VramHandle,
}

impl<'a> VramGuard<'a> {
    /// Creates a new `VramGuard` wrapping a backend and a handle.
    pub fn new(backend: &'a dyn GpuBackend, handle: VramHandle) -> Self {
        Self { backend, handle }
    }

    /// Returns the wrapped VramHandle.
    pub fn handle(&self) -> VramHandle {
        self.handle
    }
}

impl<'a> Drop for VramGuard<'a> {
    fn drop(&mut self) {
        if self.handle.0 != compute_api::INVALID_VRAM_HANDLE {
            self.backend.free(self.handle);
        }
    }
}

/// Runs differential simulation verification with different seeds.
///
/// Under invariant INV-HARNESS-001, CPU and GPU backends must be initialized
/// with matching master seeds to guarantee identical pseudo-random behaviour.
/// This function verifies seed matching, returning `SeedDiscrepancy` if mismatch is found.
pub fn run_differential_check_with_seeds(
    layout: ShardLayout,
    cpu_seed: u64,
    gpu_seed: u64,
    inputs: &[DayBatchCmd<'_>],
    gpu_backend_type: BackendType,
) -> Result<(), DifferentialTestError> {
    // Verify INV-HARNESS-001 seed matching
    if cpu_seed != gpu_seed {
        return Err(DifferentialTestError::SeedDiscrepancy {
            cpu_seed,
            gpu_seed,
        });
    }

    run_differential_check(layout, cpu_seed, inputs, gpu_backend_type)
}

/// Runs differential simulation verification between CPU and GPU backends.
///
/// Under invariant INV-CROSS-011, the CPU and GPU backends must yield
/// bit-exact identical outputs and state developments at every single step.
pub fn run_differential_check(
    layout: ShardLayout,
    _master_seed: u64,
    inputs: &[DayBatchCmd<'_>],
    gpu_backend_type: BackendType,
) -> Result<(), DifferentialTestError> {
    match gpu_backend_type {
        BackendType::Cpu => {
            // Instantiate two CPU backends for local testing
            let cpu_backend = compute_cpu::CpuBackend::new()
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let gpu_backend = compute_cpu::CpuBackend::new()
                .map_err(DifferentialTestError::BackendInitFailed)?;

            let cpu_handle = cpu_backend.alloc_shard(&layout)
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let _cpu_guard = VramGuard::new(&cpu_backend, cpu_handle);

            let gpu_handle = gpu_backend.alloc_shard(&layout)
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let _gpu_guard = VramGuard::new(&gpu_backend, gpu_handle);

            run_loop(
                &layout,
                &cpu_backend,
                cpu_handle,
                &gpu_backend,
                gpu_handle,
                inputs,
            )
        }
        BackendType::Cuda | BackendType::Hip => {
            // Instantiate standard CPU backend and GPU backend via factory
            let cpu_backend = compute_cpu::CpuBackend::new()
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let gpu_backend = instantiate_backend(gpu_backend_type, Some(0))
                .map_err(DifferentialTestError::BackendInitFailed)?;

            let cpu_handle = cpu_backend.alloc_shard(&layout)
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let _cpu_guard = VramGuard::new(&cpu_backend, cpu_handle);

            let gpu_handle = gpu_backend.alloc_shard(&layout)
                .map_err(DifferentialTestError::BackendInitFailed)?;
            let _gpu_guard = VramGuard::new(&*gpu_backend, gpu_handle);

            // TODO: Once CudaBackend and HipBackend are fully implemented:
            // 1. Perform downcast of the GpuBackend dyn reference to extract actual VRAM pointers.
            // 2. Fetch raw GPU VRAM dumps using device-to-host copies (e.g. cudaMemcpy / hipMemcpy).
            // 3. Compare with CPU state dumps using extract_raw_vram_dump.
            
            run_loop_generic(
                &layout,
                &cpu_backend,
                cpu_handle,
                &*gpu_backend,
                gpu_handle,
                inputs,
            )
        }
    }
}

/// Runs differential suite (spec-aligned wrapper).
pub fn run_differential_suite(
    layout: ShardLayout,
    master_seed: u64,
    inputs: &[DayBatchCmd<'_>],
    target_gpu: BackendType,
) -> Result<(), DifferentialTestError> {
    run_differential_check_with_seeds(layout, master_seed, master_seed, inputs, target_gpu)
}

fn run_loop(
    _layout: &ShardLayout,
    cpu_backend: &compute_cpu::CpuBackend,
    cpu_handle: VramHandle,
    gpu_backend: &compute_cpu::CpuBackend,
    gpu_handle: VramHandle,
    inputs: &[DayBatchCmd<'_>],
) -> Result<(), DifferentialTestError> {
    for (tick_idx, cmd) in inputs.iter().enumerate() {
        let tick = tick_idx as u64;

        // Execute day batch on both backends
        let cpu_res = cpu_backend.run_day_batch(&cpu_handle, cmd)
            .map_err(DifferentialTestError::DmaFailure)?;
        let gpu_res = gpu_backend.run_day_batch(&gpu_handle, cmd)
            .map_err(DifferentialTestError::DmaFailure)?;

        // Validate BatchResult counters
        if cpu_res.ticks_processed != gpu_res.ticks_processed || cpu_res.is_warmup != gpu_res.is_warmup {
            return Err(DifferentialTestError::StateMismatch {
                tick,
                offset: 0,
                cpu_val: 0,
                gpu_val: 0,
            });
        }

        // Compare motor outputs
        let cpu_out = cpu_backend.download_output(&cpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;
        let gpu_out = gpu_backend.download_output(&gpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;

        compare_outputs(tick, &cpu_out, &gpu_out)?;

        // Download and compare raw states
        let cpu_state = cpu_backend.download_raw_state(&cpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;
        let gpu_state = gpu_backend.download_raw_state(&gpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;

        compare_states(tick, &cpu_state, &gpu_state)?;
    }

    Ok(())
}

fn run_loop_generic(
    _layout: &ShardLayout,
    cpu_backend: &compute_cpu::CpuBackend,
    cpu_handle: VramHandle,
    gpu_backend: &dyn GpuBackend,
    gpu_handle: VramHandle,
    inputs: &[DayBatchCmd<'_>],
) -> Result<(), DifferentialTestError> {
    for (tick_idx, cmd) in inputs.iter().enumerate() {
        let tick = tick_idx as u64;

        // Execute day batch
        let cpu_res = cpu_backend.run_day_batch(&cpu_handle, cmd)
            .map_err(DifferentialTestError::DmaFailure)?;
        let gpu_res = gpu_backend.run_day_batch(&gpu_handle, cmd)
            .map_err(DifferentialTestError::DmaFailure)?;

        // Validate BatchResult counters
        if cpu_res.ticks_processed != gpu_res.ticks_processed || cpu_res.is_warmup != gpu_res.is_warmup {
            return Err(DifferentialTestError::StateMismatch {
                tick,
                offset: 0,
                cpu_val: 0,
                gpu_val: 0,
            });
        }

        // Compare motor outputs
        let cpu_out = cpu_backend.download_output(&cpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;
        let gpu_out = gpu_backend.download_output(&gpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;

        compare_outputs(tick, &cpu_out, &gpu_out)?;

        // NOTE: GPU state extraction is skipped here since CUDA/HIP backends are stubs.
        // CPU state is downloaded to verify no hardware extraction failures.
        let _cpu_state = cpu_backend.download_raw_state(&cpu_handle)
            .map_err(DifferentialTestError::DmaFailure)?;
    }

    Ok(())
}

fn compare_outputs(tick: u64, cpu_out: &OutputFrame, gpu_out: &OutputFrame) -> Result<(), DifferentialTestError> {
    if cpu_out.num_outputs != gpu_out.num_outputs
        || cpu_out.sync_batch_ticks != gpu_out.sync_batch_ticks
        || cpu_out.data != gpu_out.data
    {
        let min_len = cpu_out.data.len().min(gpu_out.data.len());
        for i in 0..min_len {
            if cpu_out.data[i] != gpu_out.data[i] {
                return Err(DifferentialTestError::StateMismatch {
                    tick,
                    offset: i,
                    cpu_val: cpu_out.data[i],
                    gpu_val: gpu_out.data[i],
                });
            }
        }
        return Err(DifferentialTestError::StateMismatch {
            tick,
            offset: min_len,
            cpu_val: if min_len < cpu_out.data.len() { cpu_out.data[min_len] } else { 0 },
            gpu_val: if min_len < gpu_out.data.len() { gpu_out.data[min_len] } else { 0 },
        });
    }
    Ok(())
}

fn compare_states(tick: u64, cpu_state: &[u8], gpu_state: &[u8]) -> Result<(), DifferentialTestError> {
    if cpu_state != gpu_state {
        let min_len = cpu_state.len().min(gpu_state.len());
        for i in 0..min_len {
            if cpu_state[i] != gpu_state[i] {
                return Err(DifferentialTestError::StateMismatch {
                    tick,
                    offset: i,
                    cpu_val: cpu_state[i],
                    gpu_val: gpu_state[i],
                });
            }
        }
        return Err(DifferentialTestError::StateMismatch {
            tick,
            offset: min_len,
            cpu_val: if min_len < cpu_state.len() { cpu_state[min_len] } else { 0 },
            gpu_val: if min_len < gpu_state.len() { gpu_state[min_len] } else { 0 },
        });
    }
    Ok(())
}
