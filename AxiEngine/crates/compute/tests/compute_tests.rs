//! Golden Tests suite for the compute facade.

#[cfg(feature = "mock")]
use bytemuck::Zeroable;
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
        fatigue_capacity: 255,
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
    assert!(engine.unwrap().handle().is_some());
}

#[cfg(feature = "cuda-native")]
#[test]
fn test_cuda_native_backend_lifecycle_and_dispatch() {
    let engine_res = ShardEngine::new(BackendPreference::Cuda { device_id: 0 });
    let mut engine = match engine_res {
        Ok(eng) => eng,
        Err(ComputeError::BackendUnavailable { .. }) => {
            // GPU is unavailable or device lost, skip test
            println!("CUDA GPU not available, skipping lifecycle test.");
            return;
        }
        Err(e) => panic!("Unexpected CUDA initialization error: {:?}", e),
    };

    assert_eq!(engine.backend_kind(), BackendKind::Cuda);
    assert_eq!(engine.state(), LifecycleState::Created);

    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    // Staged allocation
    engine.alloc_shard(spec).unwrap();
    assert_eq!(engine.state(), LifecycleState::Allocated);
    let handle = engine.handle();
    assert!(handle.is_some());
    assert_eq!(handle.unwrap().kind(), BackendKind::Cuda);

    let state_size = layout::calculate_state_blob_size(spec.padded_n as usize);
    let axons_size = compute_api::expected_axons_blob_size(spec.total_axons).unwrap();

    let mut voltages = vec![-70_000i32; 64];
    voltages[0] = -60_000; // will spike
    let flags = vec![0u8; 64];
    let mut soma_to_axon = vec![0xFFFFFFFFu32; 64];
    soma_to_axon[0] = 0;
    let offsets = layout::compute_state_offsets(64);

    let variant_0 = layout::VariantParameters {
        threshold: -50_000,
        rest_potential: -70_000,
        leak_shift: 4,
        homeostasis_penalty: 1000,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 200,
        gsop_depression: 150,
        homeostasis_decay: 100,
        refractory_period: 5,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [100, 90, 80, 70, 60, 50, 40, 30],
        ahp_amplitude: 10_000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 128,
        d2_affinity: 64,
        heartbeat_m: 0,
    };
    let variant_table = [variant_0; layout::VARIANT_LUT_LEN];

    let mut dendrite_targets = vec![0u32; 64 * 128];
    let mut dendrite_weights = vec![0i32; 64 * 128];
    dendrite_targets[0] = types::PackedTarget::pack(0, 0).0;
    dendrite_weights[0] = 15_000 << 16;

    let mut test_state = vec![0u8; state_size];
    test_state[offsets.off_voltage..offsets.off_voltage + 256]
        .copy_from_slice(bytemuck::cast_slice(&voltages));
    test_state[offsets.off_flags..offsets.off_flags + 64].copy_from_slice(&flags);
    test_state[offsets.off_s2a..offsets.off_s2a + 256]
        .copy_from_slice(bytemuck::cast_slice(&soma_to_axon));
    test_state[offsets.off_targets..offsets.off_targets + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_targets));
    test_state[offsets.off_weights..offsets.off_weights + 64 * 128 * 4]
        .copy_from_slice(bytemuck::cast_slice(&dendrite_weights));

    let header = layout::AxonsFileHeader::new(2);
    let mut test_axons = vec![0u8; axons_size];
    test_axons[..16].copy_from_slice(bytemuck::bytes_of(&header));
    let heads_init = vec![layout::BurstHeads8::empty(types::AXON_SENTINEL); 2];
    test_axons[16..16 + 2 * 32].copy_from_slice(bytemuck::cast_slice(&heads_init));

    let upload = compute_api::ShardUpload {
        state_blob: &test_state,
        axons_blob: &test_axons,
        variant_table: &variant_table,
    };

    // Staged upload
    engine.upload_shard(upload).unwrap();
    assert_eq!(engine.state(), LifecycleState::Running);

    let input_bitmask = vec![0x00000001u32];
    let mapped_soma_ids = vec![0u32];
    let mut output_spikes = vec![0u32; 2];
    let mut output_spike_counts = vec![0u32; 1];

    let cmd = compute_api::DayBatchCmd {
        tick_base: 1,
        sync_batch_ticks: 1,
        v_seg: 2,
        virtual_offset: 100,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        max_spikes_per_tick: 2,
        num_outputs: mapped_soma_ids.len() as u32,
        dopamine: 50,
        input_bitmask: Some(&input_bitmask),
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let result = engine.run_day_batch(cmd).unwrap();
    assert_eq!(result.ticks_executed, 1);
    assert!(result.generated_spikes_count > 0);
    assert!(output_spike_counts[0] > 0);
    assert_eq!(output_spikes[0], 0);

    // Verify debug snapshot
    let mut state_dest = vec![0u8; state_size];
    let mut axons_dest = vec![0u8; axons_size];
    engine
        .debug_snapshot(compute_api::ShardSnapshotMut {
            state_blob: &mut state_dest,
            axons_blob: &mut axons_dest,
        })
        .unwrap();

    assert_ne!(state_dest, test_state);

    // Verify teardown
    engine.teardown().unwrap();
    assert_eq!(engine.state(), LifecycleState::TornDown);
}

#[test]
#[cfg(feature = "mock")]
fn test_maintenance_lifecycle() {
    let mut engine = ShardEngine::new(BackendPreference::Mock).unwrap();
    let spec = compute_api::ShardAllocSpec {
        padded_n: 64,
        total_axons: 2,
        total_ghosts: 0,
        virtual_offset: 100,
    };

    engine.alloc_shard(spec).unwrap();
    let state_size = layout::calculate_state_blob_size(64);
    let axons_size = compute_api::expected_axons_blob_size(2).unwrap();

    let state_blob = vec![0u8; state_size];
    let axons_blob = vec![0u8; axons_size];

    let variant_table = [layout::VariantParameters::zeroed(); layout::VARIANT_LUT_LEN];
    let upload = compute_api::ShardUpload {
        state_blob: &state_blob,
        axons_blob: &axons_blob,
        variant_table: &variant_table,
    };
    engine.upload_shard(upload).unwrap();
    assert_eq!(engine.state(), LifecycleState::Running);

    // 1. Enter maintenance
    engine.enter_maintenance().unwrap();
    assert_eq!(engine.state(), LifecycleState::Maintenance);

    // 2. Reject Day Batch in maintenance
    let mut out_counts = [0u32; 1];
    let mut out_spikes = [0u32; 10];
    let cmd = compute_api::DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: 1,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: 10,
        num_outputs: 0,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0],
        mapped_soma_ids: &[],
        output_spikes: &mut out_spikes,
        output_spike_counts: &mut out_counts,
    };
    assert!(engine.run_day_batch(cmd).is_err());

    // 3. Export / Import in maintenance
    let mut state_exp = vec![0u8; state_size];
    let mut axons_exp = vec![0u8; axons_size];
    let maint_export = compute_api::BackendMaintenanceMut {
        state_blob: &mut state_exp,
        axons_blob: &mut axons_exp,
    };
    engine.export_maintenance_state(maint_export).unwrap();

    let maint_import = compute_api::BackendMaintenanceRef {
        state_blob: &state_exp,
        axons_blob: &axons_exp,
    };
    engine.import_maintenance_state(maint_import).unwrap();

    // 4. Exit maintenance
    engine.exit_maintenance().unwrap();
    assert_eq!(engine.state(), LifecycleState::Running);
}
