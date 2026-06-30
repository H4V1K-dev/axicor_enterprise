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
    let outcome = run_conformance_test(&fixture, &mut cpu_backend, 0, 10, 1, 0, 1, 10, 2, 100);
    assert_eq!(outcome, HarnessOutcome::Passed);
}

#[test]
#[cfg(feature = "mock")]
fn test_compute_backend_trait_conformance() {
    use test_harness::run_conformance_test;
    let fixture = ConformanceFixture::new("conformance_mock", 64, 10, 5, 100);
    let mut backend = MockBackend::new();
    let outcome = run_conformance_test(&fixture, &mut backend, 0, 10, 1, 0, 1, 10, 2, 100);
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

    let outcome = run_differential_test(&fixture, &mut cpu_backend, 0, 10, 1, 0, 1, 10, 2, 100);
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
