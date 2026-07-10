//! In-process Day -> Night -> Day vertical slice integration test.

#![cfg(feature = "full-chain-probe")]

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalShardComputeInput};
use runtime::{LocalRuntime, LocalRuntimeConfig};
use types::MasterSeed;
use weaver_daemon::WeaverJobRequest;

fn create_test_engine_and_bundle() -> (compute::ShardEngine, boot::LocalShardBootBundle, PathBuf) {
    let neuron_types = vec![config::NeuronType {
        name: "TypeA".to_string(),
        membrane: config::MembraneParams {
            threshold: 1000,
            rest_potential: -70,
            leak_shift: 1,
            ahp_amplitude: 5,
        },
        timing: config::TimingParams {
            refractory_period: 2,
            fatigue_capacity: 255,
        },
        signal: config::SignalParams {
            signal_propagation_length: 10,
        },
        homeostasis: config::HomeostasisParams {
            homeostasis_penalty: 0,
            homeostasis_decay: 10,
        },
        adaptive_leak: config::AdaptiveLeakParams {
            adaptive_leak_min_shift: 0,
            adaptive_leak_gain: 0,
            adaptive_mode: 0,
        },
        dopamine: config::DopamineParams {
            d1_affinity: 0,
            d2_affinity: 0,
        },
        gsop: config::GsopParams {
            gsop_potentiation: 1,
            gsop_depression: 1,
            initial_synapse_weight: 100,
            is_inhibitory: false,
            inertia_curve: vec![1, 1, 1, 1, 1, 1, 1, 1],
        },
        growth: config::GrowthParams {
            steering_fov_deg: 45.0,
            steering_radius_um: 10.0,
            steering_weight_inertia: 0.5,
            steering_weight_sensor: 0.5,
            steering_weight_jitter: 0.1,
            dendrite_radius_um: 5.0,
            growth_vertical_bias: 0.0,
            type_affinity: 1.0,
            dendrite_whitelist: vec![],
            sprouting_weight_distance: 1.0,
            sprouting_weight_power: 1.0,
            sprouting_weight_explore: 1.0,
            sprouting_weight_type: 1.0,
        },
        spontaneous: config::SpontaneousParams {
            spontaneous_firing_period_ticks: 0,
        },
    }];
    let layers = vec![config::LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.2,
        composition: vec![config::NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let shard_config = config::ShardConfig {
        meta: None,
        dimensions: config::ShardDimensions {
            w: 20,
            d: 20,
            h: 20,
        },
        settings: config::ShardSettings {
            ghost_capacity: 1024,
            prune_threshold: 0,
            max_sprouts: 8,
            night_interval_ticks: 100,
            save_checkpoints_interval_ticks: 1000,
        },
        layers,
        neuron_types,
        sockets: None,
        ports: None,
    };
    let input = LocalShardBakeInput {
        shard_config: &shard_config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("local_vertical_slice_{}.axic", rand));
    {
        let mut f = File::create(&temp).unwrap();
        f.write_all(&packed).unwrap();
    }

    let compute_input = LocalShardComputeInput {
        archive_path: temp.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let (engine, bundle) = bootstrap_local_shard_engine(&compute_input).unwrap();
    (engine, bundle, temp)
}

#[test]
fn test_in_proc_day_night_day_slice() {
    let (engine, bundle, path) = create_test_engine_and_bundle();

    let config = LocalRuntimeConfig {
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 4,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1],
    };
    let padded_n = bundle.spec.padded_n;
    let total_axons = bundle.spec.total_axons;
    let total_ghosts = bundle.spec.total_ghosts;

    use boot::LocalRuntimeBootExt;
    let mut runtime = LocalRuntime::from_boot_bundle(engine, config, bundle).unwrap();

    // Verify paths are loaded and non-empty (> 16 bytes header)
    let paths_len = runtime.working_state().unwrap().paths_blob.len();
    assert!(
        paths_len > 16,
        "paths_blob is empty or too small: {}",
        paths_len
    );

    // 1. Day batch 1
    let report1 = runtime.run_empty_batch().expect("Day batch 1 failed");
    assert_eq!(report1.ticks_executed, 2);
    assert_eq!(runtime.state(), runtime::RuntimeState::Running);
    assert_eq!(runtime.engine_state(), compute::LifecycleState::Running);

    // 2. Execute Night Phase
    let job = WeaverJobRequest {
        shard_id: 0,
        zone_hash: 0x11223344,
        night_epoch: 1,
        master_seed: [0u8; 32],
        prune_threshold: 5,
        max_sprouts: 8,
        w_distance: 100,
        w_power: 50,
        w_explore: 10,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let report_night = runtime
        .run_night_phase(&job, None, padded_n, total_axons, total_ghosts)
        .expect("Night phase execution failed");

    // Report sprouted_count should match job.max_sprouts
    assert_eq!(report_night.sprouted_count, 8);
    assert_eq!(runtime.engine_state(), compute::LifecycleState::Running);

    // 3. Day batch 2
    let report2 = runtime.run_empty_batch().expect("Day batch 2 failed");
    assert_eq!(report2.ticks_executed, 2);
    assert_eq!(runtime.state(), runtime::RuntimeState::Running);
    assert_eq!(runtime.engine_state(), compute::LifecycleState::Running);

    // 4. Fail-closed test on invalid parameters (size mismatch)
    let err_res = runtime.run_night_phase(
        &job,
        None,
        0, // Mismatched padded_n (must trigger error)
        total_axons,
        total_ghosts,
    );
    assert!(err_res.is_err());
    // In fail-closed behavior, the engine must remain in Maintenance/not return to Running
    assert_eq!(runtime.engine_state(), compute::LifecycleState::Maintenance);
    // The runtime state must transition to Faulted
    assert_eq!(runtime.state(), runtime::RuntimeState::Faulted);

    // Attempting a day batch must refuse to run and return Err
    let day_after_fail = runtime.run_empty_batch();
    assert!(day_after_fail.is_err());

    let _ = remove_file(path);
}

#[test]
fn test_night_without_working_state_fails() {
    let (engine, _bundle, path) = create_test_engine_and_bundle();
    let config = LocalRuntimeConfig {
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 4,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1],
    };

    // Construct via new (working is None)
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    let job = WeaverJobRequest {
        shard_id: 0,
        zone_hash: 0x11223344,
        night_epoch: 1,
        master_seed: [0u8; 32],
        prune_threshold: 5,
        max_sprouts: 8,
        w_distance: 100,
        w_power: 50,
        w_explore: 10,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    let err_res = runtime.run_night_phase(&job, None, 1600, 100, 10);
    assert!(err_res.is_err());

    // Engine must NOT be in Maintenance because the check happened BEFORE enter_maintenance
    assert_eq!(runtime.engine_state(), compute::LifecycleState::Running);

    let _ = remove_file(path);
}

#[test]
fn test_durability_and_attach_scenarios() {
    let (engine, bundle, path) = create_test_engine_and_bundle();
    let config = LocalRuntimeConfig {
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 4,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1],
    };

    let padded_n = bundle.spec.padded_n;
    let total_axons = bundle.spec.total_axons;
    let total_ghosts = bundle.spec.total_ghosts;

    use boot::LocalRuntimeBootExt;
    let mut runtime = LocalRuntime::from_boot_bundle(engine, config, bundle).unwrap();

    // 1. Paths non-empty from bake/boot
    let initial_paths = runtime.working_state().unwrap().paths_blob.clone();
    assert!(initial_paths.len() > 16, "paths_blob is too small");
    assert!(
        initial_paths.iter().any(|&b| b != 0),
        "paths_blob is completely zeroed"
    );

    // 2. After Night with growth_context: None: paths_blob is byte-identical (not wiped)
    let job = WeaverJobRequest {
        shard_id: 0,
        zone_hash: 0x11223344,
        night_epoch: 1,
        master_seed: [0u8; 32],
        prune_threshold: 5,
        max_sprouts: 8,
        w_distance: 100,
        w_power: 50,
        w_explore: 10,
        initial_synapse_weight: 100,
        has_growth_context: false,
    };

    runtime
        .run_night_phase(&job, None, padded_n, total_axons, total_ghosts)
        .unwrap();
    let paths_after_no_growth = runtime.working_state().unwrap().paths_blob.clone();
    assert_eq!(
        initial_paths, paths_after_no_growth,
        "paths_blob was mutated or wiped under None growth context"
    );

    // 3. After Night with growth_context: Some(minimal): paths attached path exercised
    let growth_ctx = weaver_daemon::WeaverGrowthContext {
        target_somas: vec![0; 4],
        attraction_radius: 5,
    };

    let job_with_growth = WeaverJobRequest {
        has_growth_context: true,
        ..job
    };

    runtime
        .run_night_phase(
            &job_with_growth,
            Some(&growth_ctx),
            padded_n,
            total_axons,
            total_ghosts,
        )
        .unwrap();
    let paths_after_growth = runtime.working_state().unwrap().paths_blob.clone();
    assert_eq!(
        paths_after_no_growth, paths_after_growth,
        "paths_blob was unexpectedly modified or zeroed"
    );

    let _ = remove_file(path);
}
