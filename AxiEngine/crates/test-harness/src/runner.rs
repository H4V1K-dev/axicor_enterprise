//! Crate test runners for conformance and differential testing of backends.

use crate::fixture::ConformanceFixture;
use crate::outcome::{HarnessErrorKind, HarnessOutcome};
use compute_api::{ComputeBackend, ShardSnapshotMut};

#[cfg(feature = "cpu")]
use crate::compare::{compare_output_spikes, compare_results, compare_snapshots};

/// Runs a differential execution test for a target backend against the CPU reference backend.
///
/// Returns `HarnessOutcome::Passed` if everything matches, or `HarnessOutcome::Failed` on mismatch.
#[allow(clippy::too_many_arguments)]
pub fn run_differential_test<B>(
    fixture: &ConformanceFixture,
    target_backend: &mut B,
    tick_base: u64,
    ticks: u32,
    v_seg: u32,
    dopamine: i16,
    input_words: u32,
    max_spikes: u32,
    num_outputs: u32,
    num_virtual_axons: u32,
) -> HarnessOutcome
where
    B: ComputeBackend + ?Sized,
{
    match run_differential_test_impl(
        fixture,
        target_backend,
        tick_base,
        ticks,
        v_seg,
        dopamine,
        input_words,
        max_spikes,
        num_outputs,
        num_virtual_axons,
    ) {
        Ok(()) => HarnessOutcome::Passed,
        Err(e) => HarnessOutcome::Failed(e),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_differential_test_impl<B>(
    fixture: &ConformanceFixture,
    target_backend: &mut B,
    tick_base: u64,
    ticks: u32,
    v_seg: u32,
    dopamine: i16,
    input_words: u32,
    max_spikes: u32,
    num_outputs: u32,
    num_virtual_axons: u32,
) -> Result<(), HarnessErrorKind>
where
    B: ComputeBackend + ?Sized,
{
    #[cfg(not(feature = "cpu"))]
    {
        let _ = fixture;
        let _ = target_backend;
        let _ = tick_base;
        let _ = ticks;
        let _ = v_seg;
        let _ = dopamine;
        let _ = input_words;
        let _ = max_spikes;
        let _ = num_outputs;
        let _ = num_virtual_axons;
        Err(HarnessErrorKind::FeatureNotCompiled { feature: "cpu" })
    }

    #[cfg(feature = "cpu")]
    {
        use compute_cpu::{CpuBackend, CpuBackendConfig};

        // Initialize CPU reference backend
        let config = CpuBackendConfig {
            thread_count: Some(1),
        };
        let mut cpu_backend = CpuBackend::new(config).map_err(HarnessErrorKind::BackendError)?;

        // Allocate shard on both
        let cpu_handle = cpu_backend
            .alloc_shard(fixture.spec)
            .map_err(HarnessErrorKind::BackendError)?;
        let target_handle = target_backend
            .alloc_shard(fixture.spec)
            .map_err(HarnessErrorKind::BackendError)?;

        // Upload on both
        cpu_backend
            .upload_shard(cpu_handle, fixture.upload())
            .map_err(HarnessErrorKind::BackendError)?;
        target_backend
            .upload_shard(target_handle, fixture.upload())
            .map_err(HarnessErrorKind::BackendError)?;

        // Allocate separate command buffers for CPU and Target
        let mut cpu_bufs = fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);
        let mut target_bufs =
            fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);

        let cpu_cmd = fixture.build_cmd(
            tick_base,
            ticks,
            v_seg,
            dopamine,
            input_words,
            max_spikes,
            num_outputs,
            num_virtual_axons,
            &mut cpu_bufs,
        );
        let target_cmd = fixture.build_cmd(
            tick_base,
            ticks,
            v_seg,
            dopamine,
            input_words,
            max_spikes,
            num_outputs,
            num_virtual_axons,
            &mut target_bufs,
        );

        // Run batch on both
        let cpu_result = cpu_backend
            .run_day_batch(cpu_handle, cpu_cmd)
            .map_err(HarnessErrorKind::BackendError)?;
        let target_result = target_backend
            .run_day_batch(target_handle, target_cmd)
            .map_err(HarnessErrorKind::BackendError)?;

        // Compare results (BatchResult)
        compare_results(&fixture.name, tick_base, &cpu_result, &target_result)?;

        // Compare output spikes/counts per-tick
        compare_output_spikes(
            &fixture.name,
            tick_base,
            ticks,
            max_spikes,
            &cpu_bufs.output_spikes,
            &cpu_bufs.output_spike_counts,
            &target_bufs.output_spikes,
            &target_bufs.output_spike_counts,
        )?;

        // Take debug snapshots on both using standard host buffers
        let mut cpu_state_snapshot = vec![0u8; fixture.state_blob.len()];
        let mut cpu_axons_snapshot = vec![0u8; fixture.axons_blob.len()];
        let cpu_snap = ShardSnapshotMut {
            state_blob: &mut cpu_state_snapshot,
            axons_blob: &mut cpu_axons_snapshot,
        };
        cpu_backend
            .debug_snapshot(cpu_handle, cpu_snap)
            .map_err(HarnessErrorKind::BackendError)?;

        let mut target_state_snapshot = vec![0u8; fixture.state_blob.len()];
        let mut target_axons_snapshot = vec![0u8; fixture.axons_blob.len()];
        let target_snap = ShardSnapshotMut {
            state_blob: &mut target_state_snapshot,
            axons_blob: &mut target_axons_snapshot,
        };
        target_backend
            .debug_snapshot(target_handle, target_snap)
            .map_err(HarnessErrorKind::BackendError)?;

        // Compare snapshots byte-by-byte
        compare_snapshots(
            &fixture.name,
            tick_base + ticks as u64,
            &cpu_state_snapshot,
            &cpu_axons_snapshot,
            &target_state_snapshot,
            &target_axons_snapshot,
        )?;

        // Free resources on both
        cpu_backend
            .free_shard(cpu_handle)
            .map_err(HarnessErrorKind::BackendError)?;
        target_backend
            .free_shard(target_handle)
            .map_err(HarnessErrorKind::BackendError)?;

        Ok(())
    }
}

/// Runs conformance trait-level tests on a single backend.
#[allow(clippy::too_many_arguments)]
pub fn run_conformance_test<B>(
    fixture: &ConformanceFixture,
    backend: &mut B,
    tick_base: u64,
    ticks: u32,
    v_seg: u32,
    dopamine: i16,
    input_words: u32,
    max_spikes: u32,
    num_outputs: u32,
    num_virtual_axons: u32,
) -> HarnessOutcome
where
    B: ComputeBackend + ?Sized,
{
    match run_conformance_test_impl(
        fixture,
        backend,
        tick_base,
        ticks,
        v_seg,
        dopamine,
        input_words,
        max_spikes,
        num_outputs,
        num_virtual_axons,
    ) {
        Ok(()) => HarnessOutcome::Passed,
        Err(e) => HarnessOutcome::Failed(e),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_conformance_test_impl<B>(
    fixture: &ConformanceFixture,
    backend: &mut B,
    tick_base: u64,
    ticks: u32,
    v_seg: u32,
    dopamine: i16,
    input_words: u32,
    max_spikes: u32,
    num_outputs: u32,
    num_virtual_axons: u32,
) -> Result<(), HarnessErrorKind>
where
    B: ComputeBackend + ?Sized,
{
    // Alloc
    let handle = backend
        .alloc_shard(fixture.spec)
        .map_err(HarnessErrorKind::BackendError)?;

    // Upload
    backend
        .upload_shard(handle, fixture.upload())
        .map_err(HarnessErrorKind::BackendError)?;

    // Run
    let mut bufs = fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);
    let cmd = fixture.build_cmd(
        tick_base,
        ticks,
        v_seg,
        dopamine,
        input_words,
        max_spikes,
        num_outputs,
        num_virtual_axons,
        &mut bufs,
    );
    let result = backend
        .run_day_batch(handle, cmd)
        .map_err(HarnessErrorKind::BackendError)?;

    if result.ticks_executed != ticks {
        return Err(HarnessErrorKind::ResultMismatch {
            fixture_name: fixture.name.clone(),
            tick: tick_base,
            field: "ticks_executed",
            expected: ticks.to_string(),
            actual: result.ticks_executed.to_string(),
        });
    }

    // Snapshot using standard host buffers
    let mut state_snapshot = vec![0u8; fixture.state_blob.len()];
    let mut axons_snapshot = vec![0u8; fixture.axons_blob.len()];
    let snap = ShardSnapshotMut {
        state_blob: &mut state_snapshot,
        axons_blob: &mut axons_snapshot,
    };
    backend
        .debug_snapshot(handle, snap)
        .map_err(HarnessErrorKind::BackendError)?;

    // Free
    backend
        .free_shard(handle)
        .map_err(HarnessErrorKind::BackendError)?;

    Ok(())
}
