//! Local engine response map integration test.
//!
//! Evaluates the transition between no-response, transient, sustained, and runaway behaviors.
//!
//! Execution:
//! cargo test -p test-harness --features response-map --test response_map -- --ignored --nocapture

#![cfg(feature = "response-map")]

use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalShardComputeInput};
use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use runtime::{LocalRuntime, LocalRuntimeConfig, RuntimeBatchInput};
use types::MasterSeed;

fn get_workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // AxiEngine
    path
}

fn make_neuron_type(
    name: &str,
    is_inhibitory: bool,
    threshold: i32,
    initial_synapse_weight: u16,
) -> NeuronType {
    NeuronType {
        name: name.to_string(),
        membrane: config::MembraneParams {
            threshold,
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
            initial_synapse_weight,
            is_inhibitory,
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

fn make_shard_config(
    density: f64,
    inhibitory_share: f64,
    threshold: i32,
    initial_synapse_weight: u16,
) -> ShardConfig {
    let mut neuron_types = Vec::new();
    let mut composition = Vec::new();

    if inhibitory_share > 0.0 {
        let excitatory_share = 1.0 - inhibitory_share;
        neuron_types.push(make_neuron_type(
            "Excitatory",
            false,
            threshold,
            initial_synapse_weight,
        ));
        neuron_types.push(make_neuron_type(
            "Inhibitory",
            true,
            threshold,
            initial_synapse_weight,
        ));

        composition.push(NeuronTypeDistribution {
            type_name: "Excitatory".to_string(),
            share: excitatory_share as f32,
        });
        composition.push(NeuronTypeDistribution {
            type_name: "Inhibitory".to_string(),
            share: inhibitory_share as f32,
        });
    } else {
        neuron_types.push(make_neuron_type(
            "Excitatory",
            false,
            threshold,
            initial_synapse_weight,
        ));
        composition.push(NeuronTypeDistribution {
            type_name: "Excitatory".to_string(),
            share: 1.0,
        });
    }

    let layers = vec![LayerConfig {
        name: "L1".to_string(),
        height_pct: 1.0,
        density: density as f32,
        composition,
    }];

    ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: 20,
            d: 20,
            h: 20,
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

#[test]
#[ignore]
fn test_response_map() {
    let workspace = get_workspace_root();
    let artifacts_dir = workspace.join("artifacts");
    create_dir_all(&artifacts_dir).unwrap();
    let examples_dir = artifacts_dir.join("response_map_examples");
    create_dir_all(&examples_dir).unwrap();

    let csv_path = artifacts_dir.join("response_map_summary.csv");
    let mut csv_file = File::create(&csv_path).unwrap();
    writeln!(
        csv_file,
        "experiment,density,inhibitory_share,threshold,initial_synapse_weight,stimulus_input_count,total_generated_spikes,total_output_spikes_written,total_dropped_spikes,dropped_ratio,output_saturation,nonzero_output_ticks,first_output_tick,last_output_tick,response_duration_ticks,peak_output_per_tick,mean_output_per_nonzero_tick,active_after_stimulus_ticks,wall_time_us,status"
    )
    .unwrap();

    let total_batches = 15;
    let ticks_per_batch = 10;
    let total_ticks = total_batches * ticks_per_batch;

    let mut examples_written = std::collections::HashSet::new();

    // ----------------------------------------------------
    // Experiment A: Current Excitatory Cliff
    // ----------------------------------------------------
    let thresholds_a = [10, 25, 50, 100];
    let weights_a = [250, 500, 1000, 2000];
    let inputs = 1..=7; // stimulus_input_count: 1..8

    for &threshold in &thresholds_a {
        for &weight in &weights_a {
            let density = 0.2;
            let inhibitory_share = 0.0;

            let shard_config = make_shard_config(density, inhibitory_share, threshold, weight);
            let baker_input = LocalShardBakeInput {
                shard_config: &shard_config,
                master_seed: MasterSeed(42),
                voxel_size_um: 1.0,
            };
            let (artifacts, report) = bake_local_shard(&baker_input).expect("Baking failed");
            let axic_data = pack_local_shard_artifacts(&artifacts).expect("Packaging failed");

            for stimulus_input_count in inputs.clone() {
                run_one_combination(
                    "A",
                    density,
                    inhibitory_share,
                    threshold,
                    weight,
                    stimulus_input_count,
                    &axic_data,
                    report.total_somas,
                    total_batches,
                    ticks_per_batch,
                    total_ticks,
                    &mut csv_file,
                    &examples_dir,
                    &mut examples_written,
                );
            }
        }
    }

    // ----------------------------------------------------
    // Experiment B: E/I Balance Probe
    // ----------------------------------------------------
    let threshold_b = 50;
    let weights_b = [500, 1000];
    let densities_b = [0.05, 0.1, 0.2];
    let inhibitory_shares_b = [0.0, 0.2, 0.3, 0.4];

    for &density in &densities_b {
        for &inhibitory_share in &inhibitory_shares_b {
            for &weight in &weights_b {
                let shard_config =
                    make_shard_config(density, inhibitory_share, threshold_b, weight);
                let baker_input = LocalShardBakeInput {
                    shard_config: &shard_config,
                    master_seed: MasterSeed(42),
                    voxel_size_um: 1.0,
                };
                let (artifacts, report) = bake_local_shard(&baker_input).expect("Baking failed");
                let axic_data = pack_local_shard_artifacts(&artifacts).expect("Packaging failed");

                for stimulus_input_count in inputs.clone() {
                    run_one_combination(
                        "B",
                        density,
                        inhibitory_share,
                        threshold_b,
                        weight,
                        stimulus_input_count,
                        &axic_data,
                        report.total_somas,
                        total_batches,
                        ticks_per_batch,
                        total_ticks,
                        &mut csv_file,
                        &examples_dir,
                        &mut examples_written,
                    );
                }
            }
        }
    }

    println!(
        "Response map sweep successfully completed. CSV output saved to response_map_summary.csv."
    );
}

#[allow(clippy::too_many_arguments)]
fn run_one_combination(
    experiment: &str,
    density: f64,
    inhibitory_share: f64,
    threshold: i32,
    weight: u16,
    stimulus_input_count: u32,
    axic_data: &[u8],
    total_somas: u32,
    total_batches: usize,
    ticks_per_batch: usize,
    total_ticks: usize,
    csv_file: &mut File,
    examples_dir: &PathBuf,
    examples_written: &mut std::collections::HashSet<String>,
) {
    let temp_axic_path = std::env::temp_dir().join(format!(
        "response_map_{}_{}_{}_{}_{}_{}.axic",
        experiment, density, inhibitory_share, threshold, weight, stimulus_input_count
    ));
    {
        let mut f = File::create(&temp_axic_path).unwrap();
        f.write_all(axic_data).unwrap();
    }

    // 1. Initialize engine and runtime
    let boot_input = LocalShardComputeInput {
        archive_path: temp_axic_path.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let (engine, _boot_bundle) =
        bootstrap_local_shard_engine(&boot_input).expect("Bootstrap failed");

    let mapped_somas: Vec<u32> = (0..total_somas).collect();
    let runtime_config = LocalRuntimeConfig {
        sync_batch_ticks: ticks_per_batch as u32,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 2000,
        virtual_offset: 0,
        num_virtual_axons: 32,
        input_words_per_tick: 1,
        mapped_soma_ids: mapped_somas,
    };
    let mut runtime =
        LocalRuntime::new(engine, runtime_config).expect("Failed to create LocalRuntime");

    // 2. Build the stimulus bitmask: single pulse on tick 0
    let mut full_bitmask = vec![0u32; total_ticks];
    // First N bits are active at tick 0
    let input_mask = (1u32 << stimulus_input_count) - 1;
    full_bitmask[0] = input_mask;

    // 3. Run batches
    let start_time = Instant::now();

    let mut total_generated = 0u64;
    let mut total_written = 0u64;
    let mut total_dropped = 0u64;
    let mut flat_output_spike_counts = Vec::new();
    let mut detailed_batches_csv = Vec::new();

    detailed_batches_csv.push(
        "batch_index,tick_base,generated_spikes,output_spikes_written,dropped_spikes".to_string(),
    );

    for b in 0..total_batches {
        let start_tick = b * ticks_per_batch;
        let end_tick = start_tick + ticks_per_batch;
        let batch_bitmask = &full_bitmask[start_tick..end_tick];

        let input = RuntimeBatchInput {
            input_bitmask: Some(batch_bitmask),
            incoming_spikes: None,
            incoming_spike_counts: &[0; 10],
        };

        let report = runtime.run_batch(input).expect("Batch failed");
        total_generated += report.batch_result.generated_spikes_count as u64;
        total_written += report.batch_result.output_spikes_written as u64;
        total_dropped += report.batch_result.dropped_spikes_count as u64;

        flat_output_spike_counts.extend_from_slice(&report.output_spike_counts);

        detailed_batches_csv.push(format!(
            "{},{},{},{},{}",
            b,
            report.tick_base,
            report.batch_result.generated_spikes_count,
            report.batch_result.output_spikes_written,
            report.batch_result.dropped_spikes_count
        ));
    }

    let wall_time_us = start_time.elapsed().as_micros() as u64;
    let _ = std::fs::remove_file(temp_axic_path);

    // 4. Compute metrics
    let nonzero_output_ticks = flat_output_spike_counts.iter().filter(|&&c| c > 0).count() as u64;
    let mut first_output_tick = -1i32;
    let mut last_output_tick = -1i32;
    let mut peak_output_per_tick = 0u64;
    let mut sum_output_nonzero = 0u64;

    for (t, &count) in flat_output_spike_counts.iter().enumerate() {
        if count > 0 {
            if first_output_tick == -1 {
                first_output_tick = t as i32;
            }
            last_output_tick = t as i32;
            if count as u64 > peak_output_per_tick {
                peak_output_per_tick = count as u64;
            }
            sum_output_nonzero += count as u64;
        }
    }

    let response_duration_ticks = if first_output_tick != -1 {
        (last_output_tick - first_output_tick + 1) as u64
    } else {
        0
    };

    let mean_output_per_nonzero_tick = if nonzero_output_ticks > 0 {
        sum_output_nonzero as f64 / nonzero_output_ticks as f64
    } else {
        0.0
    };

    let active_after_stimulus_ticks = flat_output_spike_counts[1..]
        .iter()
        .filter(|&&c| c > 0)
        .count() as u64;

    let dropped_ratio = if total_generated > 0 {
        total_dropped as f64 / total_generated as f64
    } else {
        0.0
    };

    let saturation = if total_generated > 0 {
        total_written as f64 / total_generated as f64
    } else {
        0.0
    };

    // Classify status
    let status = if total_written == 0 {
        "no-response"
    } else if dropped_ratio > 0.1 {
        "bottleneck"
    } else if nonzero_output_ticks as f64 / total_ticks as f64 > 0.75
        || (last_output_tick >= 135 && nonzero_output_ticks as f64 / total_ticks as f64 >= 0.5)
    {
        "runaway"
    } else if last_output_tick >= 135 {
        "sustained-response"
    } else {
        "transient-response"
    };

    writeln!(
        csv_file,
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        experiment,
        density,
        inhibitory_share,
        threshold,
        weight,
        stimulus_input_count,
        total_generated,
        total_written,
        total_dropped,
        dropped_ratio,
        saturation,
        nonzero_output_ticks,
        first_output_tick,
        last_output_tick,
        response_duration_ticks,
        peak_output_per_tick,
        mean_output_per_nonzero_tick,
        active_after_stimulus_ticks,
        wall_time_us,
        status
    )
    .unwrap();

    // Save characteristic examples if we haven't already
    let status_str = status.to_string();
    if !examples_written.contains(&status_str) && examples_written.len() < 4 {
        examples_written.insert(status_str.clone());
        let example_dir_name = format!(
            "{}_example_{}_t{}_w{}_in{}",
            status_str, experiment, threshold, weight, stimulus_input_count
        );
        let target_dir = examples_dir.join(example_dir_name);
        let _ = create_dir_all(&target_dir);

        let detailed_csv_path = target_dir.join("node_batches.csv");
        let mut ex_file = File::create(&detailed_csv_path).unwrap();
        for line in detailed_batches_csv {
            writeln!(ex_file, "{}", line).unwrap();
        }
    }
}
