//! Integration test matrix for runtime Stage A.

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalShardComputeInput};
use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use types::MasterSeed;

use runtime::{LocalRuntime, LocalRuntimeConfig, RuntimeBatchInput, RuntimeError, RuntimeState};

fn make_dummy_neuron_type(name: &str) -> NeuronType {
    NeuronType {
        name: name.to_string(),
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
    }
}

fn make_basic_test_config(
    width: u32,
    depth: u32,
    height: u32,
    layers: Vec<LayerConfig>,
    neuron_types: Vec<NeuronType>,
) -> ShardConfig {
    ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: width,
            d: depth,
            h: height,
        },
        settings: ShardSettings {
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
    }
}

fn make_baker_test_setup() -> ShardConfig {
    let neuron_types = vec![
        make_dummy_neuron_type("TypeA"),
        make_dummy_neuron_type("TypeB"),
    ];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 0.2,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    make_basic_test_config(20, 20, 20, layers, neuron_types)
}

fn get_temp_axic_path() -> PathBuf {
    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("runtime_test_{}.axic", rand));
    temp
}

fn create_test_engine_and_path() -> (compute::ShardEngine, PathBuf) {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = pack_local_shard_artifacts(&artifacts).unwrap();

    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let compute_input = LocalShardComputeInput {
        archive_path: path.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let (engine, _) = bootstrap_local_shard_engine(&compute_input).unwrap();
    (engine, path)
}

fn make_test_runtime_config() -> LocalRuntimeConfig {
    LocalRuntimeConfig {
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 4,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_words_per_tick: 1,
        mapped_soma_ids: vec![0, 1],
        plasticity_enabled: true,
    }
}

#[test]
fn test_runtime_stage_a_create_with_running_engine() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let runtime = LocalRuntime::new(engine, config);
    assert!(runtime.is_ok());
    let runtime = runtime.unwrap();
    assert_eq!(runtime.state(), RuntimeState::Running);
    assert_eq!(runtime.stats().current_tick, 0);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_invalid_initial_engine_state() {
    // An engine in Created state
    let engine = compute::ShardEngine::new(compute::BackendPreference::Cpu).unwrap();
    let config = make_test_runtime_config();
    let runtime = LocalRuntime::new(engine, config);
    assert!(matches!(
        runtime,
        Err(RuntimeError::InvalidEngineLifecycle {
            actual: compute::LifecycleState::Created
        })
    ));
}

#[test]
fn test_runtime_stage_a_reject_batch_after_shutdown() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();
    runtime.shutdown().unwrap();
    assert_eq!(runtime.state(), RuntimeState::Stopped);

    let input = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0, 0],
    };
    let res = runtime.run_batch(input);
    assert!(matches!(
        res,
        Err(RuntimeError::InvalidState {
            from: RuntimeState::Stopped,
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_tick_advancement() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();
    assert_eq!(runtime.stats().current_tick, 0);

    let report = runtime.run_empty_batch().unwrap();
    assert_eq!(report.ticks_executed, 2);
    assert_eq!(runtime.stats().current_tick, 2);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_stats_accumulation() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    runtime.run_empty_batch().unwrap();
    runtime.run_empty_batch().unwrap();

    let stats = runtime.stats();
    assert_eq!(stats.batches_executed, 2);
    assert_eq!(stats.ticks_executed, 4);
    assert_eq!(stats.current_tick, 4);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_empty_batch_creation() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    let report = runtime.run_empty_batch();
    assert!(report.is_ok());
    let report = report.unwrap();
    assert_eq!(report.output_spikes.len(), 8); // 2 ticks * 4 max_spikes_per_tick
    assert_eq!(report.output_spike_counts.len(), 2);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_invalid_input_lengths() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    // Mismatched incoming_spike_counts length (expected 2, found 3)
    let input = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[0, 0, 0],
    };
    let res = runtime.run_batch(input);
    assert!(matches!(
        res,
        Err(RuntimeError::InvalidInputDimensions {
            field: "incoming_spike_counts",
            ..
        })
    ));

    // Element in incoming_spike_counts exceeds max_spikes_per_tick (value 5 > 4)
    let input2 = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: Some(&[0; 8]),
        incoming_spike_counts: &[5, 0],
    };
    let res2 = runtime.run_batch(input2);
    assert!(matches!(
        res2,
        Err(RuntimeError::InvalidInputDimensions {
            field: "incoming_spike_counts value",
            ..
        })
    ));

    // Mismatched incoming_spikes buffer capacity (expected >= 8, found 7)
    let input3 = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: Some(&[0; 7]),
        incoming_spike_counts: &[1, 0],
    };
    let res3 = runtime.run_batch(input3);
    assert!(matches!(
        res3,
        Err(RuntimeError::InvalidInputDimensions {
            field: "incoming_spikes",
            ..
        })
    ));

    // Mismatched input_bitmask length (expected 2 ticks * 1 word = 2, found 1)
    let input4 = RuntimeBatchInput {
        input_bitmask: Some(&[0]),
        incoming_spikes: None,
        incoming_spike_counts: &[0, 0],
    };
    let res4 = runtime.run_batch(input4);
    assert!(matches!(
        res4,
        Err(RuntimeError::InvalidInputDimensions {
            field: "input_bitmask",
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_incoming_spikes_none_with_nonzero_count() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    let input = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &[1, 0],
    };
    let res = runtime.run_batch(input);
    assert!(matches!(
        res,
        Err(RuntimeError::InvalidInputDimensions {
            field: "incoming_spike_counts (when incoming_spikes is None)",
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_shutdown_idempotency() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    assert!(runtime.shutdown().is_ok());
    assert_eq!(runtime.state(), RuntimeState::Stopped);

    // Second call is no-op and returns Ok
    assert!(runtime.shutdown().is_ok());
    assert_eq!(runtime.state(), RuntimeState::Stopped);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_stage_a_no_forbidden_production_dependencies() {
    let mut cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_path.push("Cargo.toml");
    let cargo_content = std::fs::read_to_string(cargo_path).unwrap();
    // Verify we do not import boot, baker, vfs, config, topology, ipc, net, wire, protocol, transport in dependencies
    // (They are allowed in dev-dependencies though)
    let prod_deps_start = cargo_content.find("[dependencies]").unwrap();
    let dev_deps_start = cargo_content.find("[dev-dependencies]").unwrap();
    let prod_deps_section = &cargo_content[prod_deps_start..dev_deps_start];

    assert!(!prod_deps_section.contains("boot"));
    assert!(!prod_deps_section.contains("baker"));
    assert!(!prod_deps_section.contains("vfs"));
    assert!(!prod_deps_section.contains("config"));
    assert!(!prod_deps_section.contains("topology"));
    assert!(!prod_deps_section.contains("ipc"));
    assert!(!prod_deps_section.contains("net"));
    assert!(!prod_deps_section.contains("wire"));
    assert!(!prod_deps_section.contains("protocol"));
    assert!(!prod_deps_section.contains("transport"));
}

#[test]
fn test_runtime_stage_a_compute_error_to_faulted() {
    let (engine, path) = create_test_engine_and_path();

    let config = LocalRuntimeConfig {
        sync_batch_ticks: 2,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 4,
        virtual_offset: 0,
        num_virtual_axons: 10,
        input_words_per_tick: 0,
        mapped_soma_ids: vec![0, 1],
        plasticity_enabled: true,
    };

    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    let input = RuntimeBatchInput {
        input_bitmask: Some(&[]),
        incoming_spikes: None,
        incoming_spike_counts: &[0, 0],
    };

    let res = runtime.run_batch(input);
    assert!(res.is_err(), "Expected compute validation error, got Ok");

    assert_eq!(runtime.state(), RuntimeState::Faulted);
    assert_eq!(runtime.stats().compute_errors, 1);

    let res2 = runtime.run_batch(input);
    assert!(matches!(
        res2,
        Err(RuntimeError::InvalidState {
            from: RuntimeState::Faulted,
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_runtime_run_batch_with_ticks() {
    let (engine, path) = create_test_engine_and_path();
    let config = make_test_runtime_config();
    let mut runtime = LocalRuntime::new(engine, config).unwrap();

    // Check run_empty_batch_with_ticks with 5 ticks (different from sync_batch_ticks = 2)
    let res = runtime.run_empty_batch_with_ticks(5);
    assert!(res.is_ok());
    let report = res.unwrap();
    assert_eq!(report.ticks_executed, 5);
    assert_eq!(runtime.stats().current_tick, 5);

    // Check run_batch_with_ticks with 3 ticks
    let counts = vec![0, 0, 0];
    let input = RuntimeBatchInput {
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &counts,
    };
    let res2 = runtime.run_batch_with_ticks(3, input);
    assert!(res2.is_ok());
    let report2 = res2.unwrap();
    assert_eq!(report2.ticks_executed, 3);
    assert_eq!(runtime.stats().current_tick, 8);

    let _ = remove_file(path);
}

#[test]
fn test_runtime_prune_threshold_validation() {
    use runtime::local::prune_threshold_for_night;

    // -1 err
    let err_res = prune_threshold_for_night(-1);
    assert!(err_res.is_err());

    // 0 ok
    let ok_res = prune_threshold_for_night(0);
    assert_eq!(ok_res.unwrap(), 0);

    // 10 ok
    let ok_res2 = prune_threshold_for_night(10);
    assert_eq!(ok_res2.unwrap(), 10);
}
