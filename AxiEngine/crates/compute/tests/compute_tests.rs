//! Golden Tests suite for the compute facade.

#[cfg(any(feature = "cpu", feature = "mock"))]
use compute::LifecycleState;
use compute::{BackendPreference, ComputeError, ShardEngine};
use compute_api::BackendKind;
#[cfg(feature = "mock")]
use compute_api::{DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};

#[test]
fn test_shard_engine_not_send_sync() {
    // Compile-time assertion that ShardEngine is Thread-Affine (neither Send nor Sync).
    static_assertions::assert_not_impl_any!(ShardEngine: Send, Sync);
}

#[test]
fn test_no_raw_pointers_in_public_api() {
    let _new_fn: fn(BackendPreference) -> Result<ShardEngine, ComputeError> = ShardEngine::new;
    let _alloc_fn: fn(&mut ShardEngine, compute_api::ShardAllocSpec) -> Result<(), ComputeError> =
        ShardEngine::alloc_shard;
}

#[cfg(feature = "cpu")]
#[test]
fn test_cpu_default_path() {
    let engine = ShardEngine::new(BackendPreference::Cpu);
    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert_eq!(engine.backend_kind(), BackendKind::Cpu);
    assert_eq!(engine.state(), LifecycleState::Created);
    assert!(engine.handle().is_none());
}

#[cfg(feature = "cpu")]
#[test]
fn test_auto_backend_selection_priority() {
    let engine = ShardEngine::new(BackendPreference::Auto);
    assert!(engine.is_ok());
    let engine = engine.unwrap();
    #[cfg(feature = "cuda-native")]
    {
        let kind = engine.backend_kind();
        assert!(matches!(kind, BackendKind::Cuda | BackendKind::Cpu));
    }
    #[cfg(not(feature = "cuda-native"))]
    {
        assert_eq!(engine.backend_kind(), BackendKind::Cpu);
    }
}

#[cfg(not(feature = "cpu"))]
#[test]
fn test_auto_backend_selection_no_cpu() {
    let engine = ShardEngine::new(BackendPreference::Auto);
    assert!(matches!(engine, Err(ComputeError::NoBackendAvailable)));
}

#[test]
fn test_explicit_backend_error_policy() {
    #[cfg(not(feature = "cuda"))]
    {
        let engine = ShardEngine::new(BackendPreference::Cuda { device_id: 0 });
        assert!(matches!(
            engine,
            Err(ComputeError::FeatureNotCompiled { feature: "cuda" })
        ));
    }
    #[cfg(feature = "cuda")]
    {
        let engine = ShardEngine::new(BackendPreference::Cuda { device_id: 0 });
        #[cfg(feature = "cuda-native")]
        {
            match engine {
                Err(e) => {
                    assert!(matches!(
                        e,
                        ComputeError::BackendUnavailable {
                            backend: BackendKind::Cuda,
                            ..
                        }
                    ));
                }
                Ok(engine) => {
                    assert_eq!(engine.backend_kind(), BackendKind::Cuda);
                }
            }
        }
        #[cfg(not(feature = "cuda-native"))]
        {
            assert!(matches!(
                engine,
                Err(ComputeError::BackendUnavailable {
                    backend: BackendKind::Cuda,
                    ..
                })
            ));
        }
    }

    #[cfg(not(feature = "hip"))]
    {
        let engine = ShardEngine::new(BackendPreference::Hip { device_id: 0 });
        assert!(matches!(
            engine,
            Err(ComputeError::FeatureNotCompiled { feature: "hip" })
        ));
    }
    #[cfg(feature = "hip")]
    {
        let engine = ShardEngine::new(BackendPreference::Hip { device_id: 0 });
        assert!(matches!(
            engine,
            Err(ComputeError::BackendUnavailable {
                backend: BackendKind::Hip,
                ..
            })
        ));
    }

    #[cfg(not(feature = "mock"))]
    {
        let engine = ShardEngine::new(BackendPreference::Mock);
        assert!(matches!(
            engine,
            Err(ComputeError::FeatureNotCompiled { feature: "mock" })
        ));
    }
}

#[cfg(feature = "mock")]
fn make_dummy_variant_params() -> layout::VariantParameters {
    layout::VariantParameters {
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
    }
}

#[cfg(feature = "mock")]
#[test]
fn test_mock_backend_lifecycle_and_dispatch() {
    let mut engine = ShardEngine::new(BackendPreference::Mock).unwrap();
    assert_eq!(engine.backend_kind(), BackendKind::Mock);
    assert_eq!(engine.state(), LifecycleState::Created);

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };

    // Staged allocation
    engine.alloc_shard(spec).unwrap();
    assert_eq!(engine.state(), LifecycleState::Allocated);
    let handle = engine.handle();
    assert!(handle.is_some());
    assert_eq!(handle.unwrap().kind(), BackendKind::Mock);

    // Initial dummy data blobs matching specification
    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::expected_axons_blob_size(spec.total_axons).unwrap();
    let state_blob = vec![0u8; state_size];
    let axons_blob = vec![0u8; axons_size];

    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &[make_dummy_variant_params(); layout::VARIANT_LUT_LEN],
    };

    // Staged upload
    engine.upload_shard(upload).unwrap();
    assert_eq!(engine.state(), LifecycleState::Running);

    // Run batch tick simulation
    let incoming_spike_counts = [2u32, 1u32];
    let incoming_spikes = [0u32; 10]; // dummy spikes
    let mut output_spikes = [0xFFFF_FFFFu32; 10];
    let mut output_spike_counts = [0u32; 2];
    let mapped_soma_ids = [0u32, 1u32];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 5,
        num_outputs: 2,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let result = engine.run_day_batch(cmd).unwrap();
    assert_eq!(result.ticks_executed, 2);

    // Verify debug snapshot probe
    let mut state_dest = vec![0u8; state_size];
    let mut axons_dest = vec![0u8; axons_size];
    let snapshot = ShardSnapshotMut {
        state_blob: &mut state_dest,
        axons_blob: &mut axons_dest,
    };
    engine.debug_snapshot(snapshot).unwrap();

    // Idempotent teardown
    engine.teardown().unwrap();
    assert_eq!(engine.state(), LifecycleState::TornDown);
    assert!(engine.handle().is_none());

    // Idempotency check
    engine.teardown().unwrap();
}

#[cfg(feature = "mock")]
#[test]
fn test_invalid_lifecycle_transition_rejected() {
    let mut engine = ShardEngine::new(BackendPreference::Mock).unwrap();

    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };

    // Cannot upload or run before alloc
    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::expected_axons_blob_size(spec.total_axons).unwrap();
    let state_blob = vec![0u8; state_size];
    let axons_blob = vec![0u8; axons_size];
    let variant_table = [make_dummy_variant_params(); layout::VARIANT_LUT_LEN];
    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &variant_table,
    };

    let res = engine.upload_shard(upload);
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 1,
        num_outputs: 2,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &[0, 1],
        output_spikes: &mut [0xFFFF_FFFF],
        output_spike_counts: &mut [0],
    };
    let res = engine.run_day_batch(cmd);
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));

    // alloc now
    engine.alloc_shard(spec).unwrap();
    assert_eq!(engine.state(), LifecycleState::Allocated);

    // cannot run batch or snapshot before upload
    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 1,
        num_outputs: 2,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &[0, 1],
        output_spikes: &mut [0xFFFF_FFFF],
        output_spike_counts: &mut [0],
    };
    let res = engine.run_day_batch(cmd);
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));

    let mut state_dest = vec![0u8; state_size];
    let mut axons_dest = vec![0u8; axons_size];
    let snapshot = ShardSnapshotMut {
        state_blob: &mut state_dest,
        axons_blob: &mut axons_dest,
    };
    let res = engine.debug_snapshot(snapshot);
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));

    // free_shard transitions back to Created
    engine.free_shard().unwrap();
    assert_eq!(engine.state(), LifecycleState::Created);
    assert!(engine.handle().is_none());

    // free_shard on Created returns InvalidLifecycleState
    let res = engine.free_shard();
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));

    // teardown transitions to TornDown
    engine.teardown().unwrap();
    assert_eq!(engine.state(), LifecycleState::TornDown);

    // free_shard on TornDown returns InvalidLifecycleState
    let res = engine.free_shard();
    assert!(matches!(
        res,
        Err(ComputeError::InvalidLifecycleState { .. })
    ));
}

#[cfg(feature = "mock")]
#[test]
fn test_bootstrap_success_path() {
    let spec = ShardAllocSpec {
        padded_n: 64,
        total_axons: 10,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::expected_axons_blob_size(spec.total_axons).unwrap();
    let state_blob = vec![0u8; state_size];
    let axons_blob = vec![0u8; axons_size];
    let upload = ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &[make_dummy_variant_params(); layout::VARIANT_LUT_LEN],
    };

    let engine = ShardEngine::bootstrap(BackendPreference::Mock, spec, upload);
    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert_eq!(engine.state(), LifecycleState::Running);
    assert!(engine.handle().is_some());
}
