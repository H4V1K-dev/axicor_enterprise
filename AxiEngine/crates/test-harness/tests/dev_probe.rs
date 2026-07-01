#![cfg(feature = "baker-probe")]

use baker::{bake_local_shard, LocalShardBakeInput};
use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
use compute_cpu::{CpuBackend, CpuBackendConfig};
use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use layout::{align_to_padded_n, compute_state_offsets, StateFileHeader};
use types::{MasterSeed, AXON_SENTINEL};

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
            synapse_refractory_period: 2,
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

fn make_probe_test_config() -> ShardConfig {
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
    make_basic_test_config(10, 10, 10, layers, neuron_types)
}

#[test]
fn test_dev_probe_baker_to_cpu_roundtrip() {
    // 1. Bake the shard using the baker
    let config = make_probe_test_config();
    let baker_input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&baker_input).expect("Baking failed");
    assert!(report.total_somas > 0);
    assert_eq!(report.total_axons, report.total_somas);

    let padded_n = align_to_padded_n(report.total_somas as usize);

    // 2. Initialize CPU backend
    let backend_config = CpuBackendConfig {
        thread_count: Some(1),
    };
    let mut cpu_backend = CpuBackend::new(backend_config).expect("Failed to create CpuBackend");

    // 3. Allocate resources
    let spec = ShardAllocSpec {
        padded_n: padded_n as u32,
        total_axons: report.total_axons,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = cpu_backend.alloc_shard(spec).expect("Allocation failed");

    // 4. Upload artifacts (modify voltage of soma 0 to verify it leaks/drifts)
    let mut modified_state_blob = artifacts.state_blob.clone();
    let offsets = compute_state_offsets(padded_n);
    // Write -50 safely into the first 4 bytes of the voltage slice using to_le_bytes
    modified_state_blob[offsets.off_voltage..offsets.off_voltage + 4]
        .copy_from_slice(&(-50i32).to_le_bytes());

    let upload = ShardUpload {
        state_blob: &modified_state_blob,
        axons_blob: &artifacts.axons_blob,
        variant_table: &artifacts.variant_table,
    };
    cpu_backend
        .upload_shard(handle, upload)
        .expect("Upload failed");

    // 5. Run a simulation day batch (5 ticks)
    let ticks = 5;
    let max_spikes = 10;

    let incoming_spike_counts = vec![0u32; ticks as usize];
    let mapped_soma_ids = vec![0u32]; // monitor soma 0
    let mut output_spikes = vec![0u32; (ticks * max_spikes) as usize];
    let mut output_spike_counts = vec![0u32; ticks as usize];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: ticks,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: max_spikes,
        num_outputs: mapped_soma_ids.len() as u32,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: None,
        incoming_spike_counts: &incoming_spike_counts,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    let result = cpu_backend
        .run_day_batch(handle, cmd)
        .expect("Simulation run failed");
    assert_eq!(result.ticks_executed, ticks);

    // 6. Extract snapshot for verification
    let mut state_snapshot = vec![0u8; artifacts.state_blob.len()];
    let mut axons_snapshot = vec![0u8; artifacts.axons_blob.len()];

    let snapshot = ShardSnapshotMut {
        state_blob: &mut state_snapshot,
        axons_blob: &mut axons_snapshot,
    };
    cpu_backend
        .debug_snapshot(handle, snapshot)
        .expect("Debug snapshot failed");

    // Assert: size match
    assert_eq!(state_snapshot.len(), artifacts.state_blob.len());
    assert_eq!(axons_snapshot.len(), artifacts.axons_blob.len());

    // Assert: State header is correct
    let state_header: StateFileHeader = bytemuck::pod_read_unaligned(&state_snapshot[0..16]);
    assert_eq!(state_header.padded_n, padded_n as u32);
    assert_eq!(state_header.total_axons, report.total_axons);

    // Assert: Voltages changed *after* simulation relative to the uploaded modified voltages
    let offsets = compute_state_offsets(padded_n);
    let mut changed_after_sim = false;
    for i in 0..report.total_somas as usize {
        let off = offsets.off_voltage + i * 4;
        let v_mod = i32::from_le_bytes([
            modified_state_blob[off],
            modified_state_blob[off + 1],
            modified_state_blob[off + 2],
            modified_state_blob[off + 3],
        ]);
        let v_post = i32::from_le_bytes([
            state_snapshot[off],
            state_snapshot[off + 1],
            state_snapshot[off + 2],
            state_snapshot[off + 3],
        ]);
        if v_mod != v_post {
            changed_after_sim = true;
        }
        if i == 0 {
            // Verify specifically that soma 0 voltage drifted from the manually set value of -50
            assert_ne!(
                v_post, -50,
                "Soma 0 voltage should have drifted from the manually initialized -50 due to physical leak calculations"
            );
        }
    }
    assert!(
        changed_after_sim,
        "Expected voltages to change/drift during simulation ticks relative to uploaded modified state"
    );

    // Assert: Soma flags type_id is valid
    let flags_slice =
        &state_snapshot[offsets.off_flags..offsets.off_flags + report.total_somas as usize];
    for &flag in flags_slice {
        let type_id = (flag & 0xF0) >> 4;
        assert!(
            type_id < config.neuron_types.len() as u8,
            "type_id {} must be valid",
            type_id
        );
    }

    // Assert: Axon heads check (unaligned safe)
    let heads_slice = &axons_snapshot[16..];
    for chunk in heads_slice.chunks_exact(4) {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        // Val must be either AXON_SENTINEL or valid propagation head/segment positions (less than AXON_SENTINEL)
        assert!(val <= AXON_SENTINEL, "Axon head value {} is corrupt", val);
    }

    // 7. Clean up
    cpu_backend.free_shard(handle).expect("Free shard failed");
    cpu_backend.teardown().expect("Teardown failed");
}

#[test]
#[ignore]
fn test_benchmark_performance_100k_neurons() {
    // 1. Create a configuration designed to generate exactly 100k neurons
    // Volume = 50 * 40 * 50 = 100,000 voxels. Density = 1.0 => 100,000 neurons.
    let mut type_a = make_dummy_neuron_type("TypeA");
    type_a.spontaneous.spontaneous_firing_period_ticks = 100;

    let neuron_types = vec![type_a];
    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: 1.0,
        composition: vec![NeuronTypeDistribution {
            type_name: "TypeA".to_string(),
            share: 1.0,
        }],
    }];
    let config = make_basic_test_config(50, 40, 50, layers, neuron_types);

    let baker_input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    println!("Starting 100k neurons AOT compile (baker)...");
    let baker_start = std::time::Instant::now();
    let (artifacts, report) = bake_local_shard(&baker_input).expect("Baking failed");
    println!(
        "Baking completed in {:.4} seconds.",
        baker_start.elapsed().as_secs_f64()
    );
    assert_eq!(
        report.total_somas, 100000,
        "Expected exactly 100k neurons generated"
    );

    let padded_n = align_to_padded_n(report.total_somas as usize);

    // 2. Initialize CpuBackend
    let backend_config = CpuBackendConfig { thread_count: None };
    let mut cpu_backend = CpuBackend::new(backend_config).expect("Failed to create CpuBackend");

    let spec = ShardAllocSpec {
        padded_n: padded_n as u32,
        total_axons: report.total_axons,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = cpu_backend.alloc_shard(spec).expect("Allocation failed");

    let upload = ShardUpload {
        state_blob: &artifacts.state_blob,
        axons_blob: &artifacts.axons_blob,
        variant_table: &artifacts.variant_table,
    };
    cpu_backend
        .upload_shard(handle, upload)
        .expect("Upload failed");

    // 3. Prepare execution command (100 ticks is sufficient for 100k network benchmark)
    // Inject 10 randomized noise spikes per tick
    let ticks = 100;
    let max_spikes = 10;

    let mut incoming_spikes = vec![0u32; (ticks * max_spikes) as usize];
    let incoming_spike_counts = vec![10u32; ticks as usize];

    let mut seed = 12345u32;
    for val in incoming_spikes.iter_mut() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        *val = seed % 100_000; // random target axon ID
    }

    let mapped_soma_ids = vec![0u32];
    let mut output_spikes = vec![0u32; (ticks * max_spikes) as usize];
    let mut output_spike_counts = vec![0u32; ticks as usize];

    let cmd = DayBatchCmd {
        tick_base: 0,
        sync_batch_ticks: ticks,
        v_seg: 1,
        dopamine: 0,
        input_words_per_tick: 0,
        max_spikes_per_tick: max_spikes,
        num_outputs: mapped_soma_ids.len() as u32,
        virtual_offset: 0,
        num_virtual_axons: 0,
        input_bitmask: None,
        incoming_spikes: Some(&incoming_spikes),
        incoming_spike_counts: &incoming_spike_counts,
        mapped_soma_ids: &mapped_soma_ids,
        output_spikes: &mut output_spikes,
        output_spike_counts: &mut output_spike_counts,
    };

    // 4. Measure execution time
    let start = std::time::Instant::now();
    let result = cpu_backend
        .run_day_batch(handle, cmd)
        .expect("Simulation run failed");
    let elapsed = start.elapsed();

    let tps = (ticks as f64) / elapsed.as_secs_f64();
    let tick_time_ms = (elapsed.as_millis() as f64) / (ticks as f64);
    let syn_updates_per_sec = (report.total_somas as f64) * 128.0 * tps; // 128 synapses per neuron

    println!("\n========================================================");
    println!("  AXIENGINE CPU BENCHMARK (100,000 Neurons, 128 Dendrites)");
    println!("========================================================");
    println!("  Total Ticks Simulated : {}", ticks);
    println!(
        "  Total Elapsed Time     : {:.4} seconds",
        elapsed.as_secs_f64()
    );
    println!(
        "  Average Tick Latency   : {:.2} milliseconds",
        tick_time_ms
    );
    println!("  Ticks Per Second (TPS) : {:.2}", tps);
    println!(
        "  Synaptic Updates/sec   : {:.2} M/s",
        syn_updates_per_sec / 1_000_000.0
    );
    println!(
        "  Spikes Generated       : {}",
        result.generated_spikes_count
    );
    println!("========================================================\n");

    cpu_backend.free_shard(handle).expect("Free shard failed");
    cpu_backend.teardown().expect("Teardown failed");
}
