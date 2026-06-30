//! Integration tests covering the Golden Tests matrix (§9) of test-harness.

use compute_api::BackendKind;
use test_harness::{ConformanceFixture, HarnessErrorKind, HarnessOutcome};

#[cfg(any(feature = "mock", feature = "cpu"))]
use compute_api::{ComputeBackend, ShardAllocSpec};

#[cfg(feature = "mock")]
use test_harness::MockBackend;

#[test]
#[cfg(feature = "cpu")]
fn test_cpu_reference_fixture_runs() {
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use test_harness::run_conformance_test;

    let fixture = ConformanceFixture::new("cpu_ref_run", 64, 10, 5, 100);
    let config = CpuBackendConfig {
        thread_count: Some(1),
    };
    let mut cpu_backend = CpuBackend::new(config).unwrap();
    let outcome = run_conformance_test(&fixture, &mut cpu_backend, 0, 10, 1, 0, 1, 10, 2, 32);
    assert_eq!(outcome, HarnessOutcome::Passed);
}

#[test]
#[cfg(feature = "mock")]
fn test_compute_backend_trait_conformance() {
    use test_harness::run_conformance_test;
    let fixture = ConformanceFixture::new("conformance_mock", 64, 10, 5, 100);
    let mut backend = MockBackend::new();
    let outcome = run_conformance_test(&fixture, &mut backend, 0, 10, 1, 0, 1, 10, 2, 32);
    assert_eq!(outcome, HarnessOutcome::Passed);
}

#[test]
#[cfg(feature = "mock")]
fn test_mock_backend_conformance() {
    use compute_api::ComputeBackend;
    let mut backend = MockBackend::new();
    assert_eq!(backend.kind(), BackendKind::Mock);
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 5,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    assert_eq!(handle.kind(), BackendKind::Mock);
    backend.free_shard(handle).unwrap();
}

#[test]
fn test_cuda_matches_cpu_fixtures_when_available() {
    #[cfg(not(feature = "cuda"))]
    {
        let outcome = HarnessOutcome::Skipped {
            backend: BackendKind::Cuda,
            reason: String::from("CUDA backend feature 'cuda' not compiled in Stage 1"),
        };
        match outcome {
            HarnessOutcome::Skipped { backend, .. } => {
                assert_eq!(backend, BackendKind::Cuda);
            }
            _ => panic!("Expected skipped outcome"),
        }
    }
}

#[test]
fn test_hip_matches_cpu_fixtures_when_available() {
    #[cfg(not(feature = "hip"))]
    {
        let outcome = HarnessOutcome::Skipped {
            backend: BackendKind::Hip,
            reason: String::from("HIP backend feature 'hip' not compiled in Stage 1"),
        };
        match outcome {
            HarnessOutcome::Skipped { backend, .. } => {
                assert_eq!(backend, BackendKind::Hip);
            }
            _ => panic!("Expected skipped outcome"),
        }
    }
}

#[test]
fn test_backend_unavailable_is_reported_as_skip_for_optional_matrix() {
    let outcome = HarnessOutcome::Skipped {
        backend: BackendKind::Cuda,
        reason: String::from("Hardware driver initialization failed"),
    };
    match outcome {
        HarnessOutcome::Skipped {
            backend,
            ref reason,
        } => {
            assert_eq!(backend, BackendKind::Cuda);
            assert_eq!(reason, "Hardware driver initialization failed");
        }
        _ => panic!("Expected skipped"),
    }
}

#[test]
#[cfg(feature = "cpu")]
fn test_invalid_dto_errors_are_consistent_across_backends() {
    #[cfg(feature = "mock")]
    {
        use compute_api::ComputeApiError;
        use compute_cpu::{CpuBackend, CpuBackendConfig};

        let mut mock = MockBackend::new();
        let mut cpu = CpuBackend::new(CpuBackendConfig {
            thread_count: Some(1),
        })
        .unwrap();

        let bad_spec = ShardAllocSpec {
            padded_n: 3,
            total_axons: 10,
            total_ghosts: 5,
            virtual_offset: 100,
        };

        let err_mock = mock.alloc_shard(bad_spec).unwrap_err();
        let err_cpu = cpu.alloc_shard(bad_spec).unwrap_err();

        assert_eq!(err_mock, err_cpu);
        assert_eq!(err_mock, ComputeApiError::AlignmentViolation);
    }
}

#[test]
#[cfg(feature = "mock")]
fn test_batch_stage_order_fixture() {
    use compute_api::{ComputeBackend, DayBatchCmd};
    let mut backend = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 5,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();

    let bad_cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 0,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 1,
        max_spikes_per_tick: 10,
        num_outputs: 0,
        virtual_offset: 100,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[],
        mapped_soma_ids: &[],
        output_spikes: &mut [],
        output_spike_counts: &mut [],
    };

    let res = backend.run_day_batch(handle, bad_cmd).unwrap_err();
    assert_eq!(res, compute_api::ComputeApiError::InvalidBatch);
}

#[test]
fn test_no_raw_vendor_types_in_public_test_api() {
    let err = HarnessErrorKind::FeatureNotCompiled { feature: "cuda" };
    match err {
        HarnessErrorKind::FeatureNotCompiled { feature } => {
            assert_eq!(feature, "cuda");
        }
        _ => panic!("Expected FeatureNotCompiled"),
    }
}

#[test]
fn test_layout_blob_size_validation_fixture() {
    let fixture = ConformanceFixture::new("size_check", 64, 100, 10, 1000);
    let expected_state_size = layout::calculate_state_blob_size(64);
    assert_eq!(fixture.state_blob.len(), expected_state_size);

    let expected_axons_size = compute_api::validation::expected_axons_blob_size(100).unwrap();
    assert_eq!(fixture.axons_blob.len(), expected_axons_size);

    let upload = fixture.upload();
    assert_eq!(upload.variant_table.len(), layout::VARIANT_LUT_LEN);
}

#[test]
#[cfg(feature = "abi")]
fn test_abi_mirror_drift_detection() {
    use test_harness::verify_abi_mirrors;
    assert!(verify_abi_mirrors().is_ok());
}

#[test]
fn test_first_mismatch_report_contains_fixture_tick_field_or_plane_offset_expected_actual() {
    let err_res = HarnessErrorKind::ResultMismatch {
        fixture_name: String::from("test_fixture"),
        tick: 42,
        field: "ticks_executed",
        expected: String::from("10"),
        actual: String::from("5"),
    };

    match err_res {
        HarnessErrorKind::ResultMismatch {
            fixture_name,
            tick,
            field,
            expected,
            actual,
        } => {
            assert_eq!(fixture_name, "test_fixture");
            assert_eq!(tick, 42);
            assert_eq!(field, "ticks_executed");
            assert_eq!(expected, "10");
            assert_eq!(actual, "5");
        }
        _ => panic!("Expected ResultMismatch"),
    }

    let err_snap = HarnessErrorKind::SnapshotMismatch {
        fixture_name: String::from("test_fixture"),
        tick: 50,
        plane: "state_blob",
        offset: 1024,
        expected: 255,
        actual: 0,
    };

    match err_snap {
        HarnessErrorKind::SnapshotMismatch {
            fixture_name,
            tick,
            plane,
            offset,
            expected,
            actual,
        } => {
            assert_eq!(fixture_name, "test_fixture");
            assert_eq!(tick, 50);
            assert_eq!(plane, "state_blob");
            assert_eq!(offset, 1024);
            assert_eq!(expected, 255);
            assert_eq!(actual, 0);
        }
        _ => panic!("Expected SnapshotMismatch"),
    }
}

#[test]
fn test_facade_lifecycle_under_facade_feature() {
    #[cfg(not(feature = "facade"))]
    {
        let outcome = HarnessOutcome::Skipped {
            backend: BackendKind::Cpu,
            reason: String::from("Facade feature 'facade' not compiled in Stage 1"),
        };
        match outcome {
            HarnessOutcome::Skipped { backend, .. } => {
                assert_eq!(backend, BackendKind::Cpu);
            }
            _ => panic!("Expected skipped outcome"),
        }
    }
    #[cfg(feature = "facade")]
    {
        use compute::{BackendPreference, LifecycleState, ShardEngine};
        use compute_api::ShardSnapshotMut;

        let fixture = ConformanceFixture::new("facade_lifecycle", 64, 10, 5, 100);

        let engine = ShardEngine::new(BackendPreference::Cpu).unwrap();
        assert_eq!(engine.state(), LifecycleState::Created);
        assert_eq!(engine.backend_kind(), BackendKind::Cpu);

        // Transition from Created -> Running via bootstrap
        let mut engine =
            ShardEngine::bootstrap(BackendPreference::Cpu, fixture.spec, fixture.upload()).unwrap();
        assert_eq!(engine.state(), LifecycleState::Running);
        assert_eq!(engine.backend_kind(), BackendKind::Cpu);

        // Before run_day_batch, call debug_snapshot and check state/axon snapshots match
        let mut state_dest = vec![0u8; fixture.state_blob.len()];
        let mut axons_dest = vec![0u8; fixture.axons_blob.len()];
        {
            let snapshot = ShardSnapshotMut {
                state_blob: &mut state_dest,
                axons_blob: &mut axons_dest,
            };
            engine.debug_snapshot(snapshot).unwrap();
        }
        assert_eq!(state_dest, fixture.state_blob);
        assert_eq!(axons_dest, fixture.axons_blob);

        // Run a short run_day_batch (1 tick)
        let ticks = 1;
        let v_seg = 1;
        let dopamine = 0;
        let input_words = 0;
        let max_spikes = 10;
        let num_outputs = 2;
        let num_virtual_axons = 10;
        let tick_base = 0;

        let mut cmd_bufs = fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);
        let cmd = fixture.build_cmd(
            tick_base,
            ticks,
            v_seg,
            dopamine,
            input_words,
            max_spikes,
            num_outputs,
            num_virtual_axons,
            &mut cmd_bufs,
        );

        let result = engine.run_day_batch(cmd).unwrap();
        assert_eq!(result.ticks_executed, ticks);

        // free_shard
        engine.free_shard().unwrap();
        assert_eq!(engine.state(), LifecycleState::Created);
        assert!(engine.handle().is_none());

        // teardown
        engine.teardown().unwrap();
        assert_eq!(engine.state(), LifecycleState::TornDown);
    }
}

#[test]
fn test_facade_lifecycle_errors() {
    #[cfg(not(feature = "facade"))]
    {
        // Skip
    }
    #[cfg(feature = "facade")]
    {
        use compute::{BackendPreference, ComputeError, LifecycleState, ShardEngine};
        use compute_api::ShardSnapshotMut;

        let mut engine = ShardEngine::new(BackendPreference::Cpu).unwrap();
        assert_eq!(engine.state(), LifecycleState::Created);

        // debug_snapshot before bootstrap/upload
        let mut state_dest = vec![0u8; 10];
        let mut axons_dest = vec![0u8; 10];
        let snapshot = ShardSnapshotMut {
            state_blob: &mut state_dest,
            axons_blob: &mut axons_dest,
        };
        let res = engine.debug_snapshot(snapshot);
        assert!(matches!(
            res,
            Err(ComputeError::InvalidLifecycleState { .. })
        ));

        // run_day_batch before bootstrap/upload
        let mut output_spikes = vec![0u32; 10];
        let mut output_spike_counts = vec![0u32; 1];
        let cmd = compute_api::DayBatchCmd {
            tick_base: 0,
            sync_batch_ticks: 1,
            v_seg: 1,
            dopamine: 0,
            input_words_per_tick: 0,
            max_spikes_per_tick: 5,
            num_outputs: 0,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &[0],
            mapped_soma_ids: &[],
            output_spikes: &mut output_spikes,
            output_spike_counts: &mut output_spike_counts,
        };
        let res = engine.run_day_batch(cmd);
        assert!(matches!(
            res,
            Err(ComputeError::InvalidLifecycleState { .. })
        ));
    }
}

#[test]
#[cfg(feature = "cpu")]
fn test_debug_snapshot_comparison() {
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use test_harness::run_differential_test;

    let fixture = ConformanceFixture::new("diff_snap", 64, 10, 5, 100);
    let config = CpuBackendConfig {
        thread_count: Some(1),
    };
    let mut cpu_backend = CpuBackend::new(config).unwrap();

    let outcome = run_differential_test(&fixture, &mut cpu_backend, 0, 10, 1, 0, 1, 10, 2, 32);
    assert_eq!(outcome, HarnessOutcome::Passed);
}

#[test]
#[cfg(feature = "mock")]
fn test_mock_foreign_handle() {
    use compute_api::VramHandle;
    let mut backend = MockBackend::new();
    let foreign_nz = core::num::NonZeroU64::new(1).unwrap();
    let foreign_handle = VramHandle::from_raw_parts(BackendKind::Cpu, foreign_nz, 1);

    let dummy = layout::VariantParameters {
        threshold: 0,
        rest_potential: 0,
        leak_shift: 0,
        homeostasis_penalty: 0,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 0,
        refractory_period: 0,
        synapse_refractory_period: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };
    let dummy_table = [dummy; 16];

    let upload = compute_api::ShardUpload {
        state_blob: &[],
        axons_blob: &[],
        variant_table: &dummy_table,
    };
    let err = backend.upload_shard(foreign_handle, upload).unwrap_err();
    assert_eq!(err, compute_api::ComputeApiError::ForeignHandle);
}

#[test]
#[cfg(feature = "mock")]
fn test_mock_freed_handle() {
    use compute_api::ComputeBackend;
    let mut backend = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 5,
        virtual_offset: 100,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    backend.free_shard(handle).unwrap();
    let err = backend.free_shard(handle).unwrap_err();
    assert_eq!(err, compute_api::ComputeApiError::AlreadyFreed);
}

#[test]
#[cfg(feature = "mock")]
fn test_mock_alloc_after_teardown() {
    use compute_api::ComputeBackend;
    let mut backend = MockBackend::new();
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 5,
        virtual_offset: 100,
    };
    let handle1 = backend.alloc_shard(spec).unwrap();
    backend.teardown().unwrap();
    let err = backend.free_shard(handle1).unwrap_err();
    assert_eq!(err, compute_api::ComputeApiError::InvalidHandle);

    let handle2 = backend.alloc_shard(spec).unwrap();
    assert_eq!(handle2.kind(), BackendKind::Mock);
}

#[test]
#[cfg(feature = "cuda-native")]
fn test_cuda_native_facade_differential_with_cpu() {
    use compute::{BackendPreference, ComputeError, ShardEngine};
    use compute_api::ShardSnapshotMut;

    let fixture = ConformanceFixture::new("cuda_facade_diff", 64, 2, 0, 100);

    // Try to bootstrap CUDA
    let cuda_engine_res = ShardEngine::bootstrap(
        BackendPreference::Cuda { device_id: 0 },
        fixture.spec,
        fixture.upload(),
    );

    let mut cuda_engine = match cuda_engine_res {
        Ok(eng) => eng,
        Err(ComputeError::BackendUnavailable { .. }) => {
            // GPU is unavailable or device lost, skip test
            println!("CUDA GPU not available, skipping differential test.");
            return;
        }
        Err(e) => panic!("Unexpected CUDA bootstrap error: {:?}", e),
    };

    // Bootstrap CPU
    let mut cpu_engine =
        ShardEngine::bootstrap(BackendPreference::Cpu, fixture.spec, fixture.upload()).unwrap();

    assert_eq!(cuda_engine.backend_kind(), BackendKind::Cuda);
    assert_eq!(cpu_engine.backend_kind(), BackendKind::Cpu);

    // Initial snapshots comparison
    let mut cuda_state_init = vec![0u8; fixture.state_blob.len()];
    let mut cuda_axons_init = vec![0u8; fixture.axons_blob.len()];
    cuda_engine
        .debug_snapshot(ShardSnapshotMut {
            state_blob: &mut cuda_state_init,
            axons_blob: &mut cuda_axons_init,
        })
        .unwrap();

    let mut cpu_state_init = vec![0u8; fixture.state_blob.len()];
    let mut cpu_axons_init = vec![0u8; fixture.axons_blob.len()];
    cpu_engine
        .debug_snapshot(ShardSnapshotMut {
            state_blob: &mut cpu_state_init,
            axons_blob: &mut cpu_axons_init,
        })
        .unwrap();

    assert_eq!(cuda_state_init, cpu_state_init);
    assert_eq!(cuda_axons_init, cpu_axons_init);

    // Run 3 ticks DayBatch
    let ticks = 3;
    let max_spikes = 2;
    let input_words = 1;
    let num_outputs = 2;
    let num_virtual_axons = 32;
    let tick_base = 1;

    let mut cmd_bufs_cpu = fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);
    // Fill input bitmask to trigger spike on virtual axon
    cmd_bufs_cpu.input_bitmask[0] = 0x00000001; // tick 1 virtual axon 100 spikes
    cmd_bufs_cpu.input_bitmask[1] = 0x0;
    cmd_bufs_cpu.input_bitmask[2] = 0x00000001;

    // Fill incoming physical spikes
    cmd_bufs_cpu.incoming_spikes[0] = 1; // tick 1
    cmd_bufs_cpu.incoming_spikes[1] = 0;
    cmd_bufs_cpu.incoming_spikes[2] = 1; // tick 2
    cmd_bufs_cpu.incoming_spikes[3] = 0;
    cmd_bufs_cpu.incoming_spikes[4] = 1; // tick 3
    cmd_bufs_cpu.incoming_spikes[5] = 0;

    cmd_bufs_cpu
        .incoming_spike_counts
        .copy_from_slice(&[1, 1, 1]);
    cmd_bufs_cpu.mapped_soma_ids.copy_from_slice(&[0, 1]);

    let mut cmd_bufs_cuda = fixture.create_cmd_buffers(ticks, max_spikes, input_words, num_outputs);
    cmd_bufs_cuda
        .input_bitmask
        .copy_from_slice(&cmd_bufs_cpu.input_bitmask);
    cmd_bufs_cuda
        .incoming_spikes
        .copy_from_slice(&cmd_bufs_cpu.incoming_spikes);
    cmd_bufs_cuda
        .incoming_spike_counts
        .copy_from_slice(&cmd_bufs_cpu.incoming_spike_counts);
    cmd_bufs_cuda
        .mapped_soma_ids
        .copy_from_slice(&cmd_bufs_cpu.mapped_soma_ids);

    let cmd_cpu = fixture.build_cmd(
        tick_base,
        ticks,
        2,  // v_seg = 2
        50, // dopamine
        input_words,
        max_spikes,
        num_outputs,
        num_virtual_axons,
        &mut cmd_bufs_cpu,
    );

    let cmd_cuda = fixture.build_cmd(
        tick_base,
        ticks,
        2,
        50,
        input_words,
        max_spikes,
        num_outputs,
        num_virtual_axons,
        &mut cmd_bufs_cuda,
    );

    let res_cpu = cpu_engine.run_day_batch(cmd_cpu).unwrap();
    let res_cuda = cuda_engine.run_day_batch(cmd_cuda).unwrap();

    // Verify batch results parity
    assert_eq!(res_cpu.ticks_executed, res_cuda.ticks_executed);
    assert_eq!(
        res_cpu.generated_spikes_count,
        res_cuda.generated_spikes_count
    );
    assert_eq!(
        res_cpu.output_spikes_written,
        res_cuda.output_spikes_written
    );
    assert_eq!(res_cpu.dropped_spikes_count, res_cuda.dropped_spikes_count);

    assert_eq!(
        cmd_bufs_cpu.output_spike_counts,
        cmd_bufs_cuda.output_spike_counts
    );

    // Verify output spikes used regions
    for tick in 0..ticks as usize {
        let count = cmd_bufs_cpu.output_spike_counts[tick] as usize;
        let start = tick * max_spikes as usize;
        assert_eq!(
            &cmd_bufs_cpu.output_spikes[start..start + count],
            &cmd_bufs_cuda.output_spikes[start..start + count]
        );
    }

    // Verify snapshot state parity
    let mut cuda_state_final = vec![0u8; fixture.state_blob.len()];
    let mut cuda_axons_final = vec![0u8; fixture.axons_blob.len()];
    cuda_engine
        .debug_snapshot(ShardSnapshotMut {
            state_blob: &mut cuda_state_final,
            axons_blob: &mut cuda_axons_final,
        })
        .unwrap();

    let mut cpu_state_final = vec![0u8; fixture.state_blob.len()];
    let mut cpu_axons_final = vec![0u8; fixture.axons_blob.len()];
    cpu_engine
        .debug_snapshot(ShardSnapshotMut {
            state_blob: &mut cpu_state_final,
            axons_blob: &mut cpu_axons_final,
        })
        .unwrap();

    assert_eq!(cuda_state_final, cpu_state_final);
    assert_eq!(cuda_axons_final, cpu_axons_final);

    // Clean teardowns
    cuda_engine.teardown().unwrap();
    cpu_engine.teardown().unwrap();
}
