//! DIAGNOSTIC PROBE RUNTIME HARNESS
//!
//! WARNING: This is a diagnostic tool to evaluate dynamic performance of the full chain.
//! It is NOT an architectural authority or reference design source.
//!
//! Execution:
//! cargo test -p test-harness --features full-chain-probe --test full_chain_probe -- --ignored --nocapture

#![cfg(feature = "full-chain-probe")]

use std::fs::{create_dir_all, remove_file, File};
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

fn make_probe_config_baseline() -> ShardConfig {
    let neuron_types = vec![
        make_neuron_type("Excitatory", false, 1000, 100),
        make_neuron_type("Inhibitory", true, 1000, 100),
    ];
    let layers = vec![
        LayerConfig {
            name: "L1".to_string(),
            height_pct: 0.5,
            density: 0.4,
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "Excitatory".to_string(),
                    share: 0.8,
                },
                NeuronTypeDistribution {
                    type_name: "Inhibitory".to_string(),
                    share: 0.2,
                },
            ],
        },
        LayerConfig {
            name: "L2".to_string(),
            height_pct: 0.5,
            density: 0.3,
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "Excitatory".to_string(),
                    share: 0.9,
                },
                NeuronTypeDistribution {
                    type_name: "Inhibitory".to_string(),
                    share: 0.1,
                },
            ],
        },
    ];

    ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: 15,
            d: 15,
            h: 15,
        },
        settings: ShardSettings {
            ghost_capacity: 512,
            prune_threshold: 0,
            max_sprouts: 4,
            night_interval_ticks: 100,
            save_checkpoints_interval_ticks: 1000,
        },
        layers,
        neuron_types,
        sockets: None,
        ports: None,
    }
}

fn make_probe_config_stimulated() -> ShardConfig {
    let neuron_types = vec![
        make_neuron_type("Excitatory", false, 10, 2000),
        make_neuron_type("Inhibitory", true, 40, 100),
    ];
    let layers = vec![
        LayerConfig {
            name: "L1".to_string(),
            height_pct: 0.5,
            density: 0.4,
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "Excitatory".to_string(),
                    share: 0.8,
                },
                NeuronTypeDistribution {
                    type_name: "Inhibitory".to_string(),
                    share: 0.2,
                },
            ],
        },
        LayerConfig {
            name: "L2".to_string(),
            height_pct: 0.5,
            density: 0.3,
            composition: vec![
                NeuronTypeDistribution {
                    type_name: "Excitatory".to_string(),
                    share: 0.9,
                },
                NeuronTypeDistribution {
                    type_name: "Inhibitory".to_string(),
                    share: 0.1,
                },
            ],
        },
    ];

    ShardConfig {
        meta: None,
        dimensions: ShardDimensions {
            w: 15,
            d: 15,
            h: 15,
        },
        settings: ShardSettings {
            ghost_capacity: 512,
            prune_threshold: 0,
            max_sprouts: 4,
            night_interval_ticks: 100,
            save_checkpoints_interval_ticks: 1000,
        },
        layers,
        neuron_types,
        sockets: None,
        ports: None,
    }
}

struct ScenarioResult {
    batches_csv_lines: Vec<String>,
    outputs_csv_lines: Vec<String>,
    output_spikes_csv_lines: Vec<String>,
    summary_records: Vec<(String, String)>,
}

fn run_scenario(
    scenario_name: &str,
    shard_config: ShardConfig,
    mapped_somas_limit: usize,
) -> ScenarioResult {
    println!("--- Running Scenario: {} ---", scenario_name);

    let baker_input = LocalShardBakeInput {
        shard_config: &shard_config,
        master_seed: MasterSeed(12345),
        voxel_size_um: 1.0,
    };
    let (artifacts, report) = bake_local_shard(&baker_input).expect("Baking failed");
    println!(
        "   Baking report: somas={}, axons={}",
        report.total_somas, report.total_axons
    );

    let axic_data = pack_local_shard_artifacts(&artifacts).expect("Packaging failed");
    let axic_size = axic_data.len();

    let temp_axic_path = std::env::temp_dir().join(format!(
        "probe_{}_{}.axic",
        scenario_name, report.total_somas
    ));
    {
        let mut f = File::create(&temp_axic_path).unwrap();
        f.write_all(&axic_data).unwrap();
    }

    let boot_input = LocalShardComputeInput {
        archive_path: temp_axic_path.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let (engine, _boot_bundle) =
        bootstrap_local_shard_engine(&boot_input).expect("Bootstrap failed");

    let mapped_somas =
        (0..std::cmp::min(mapped_somas_limit, report.total_somas as usize) as u32).collect();
    let runtime_config = LocalRuntimeConfig {
        sync_batch_ticks: 10,
        v_seg: 1,
        dopamine: 0,
        max_spikes_per_tick: 8,
        virtual_offset: 0,
        num_virtual_axons: 2,
        input_words_per_tick: 1,
        mapped_soma_ids: mapped_somas,
    };
    let mut runtime =
        LocalRuntime::new(engine, runtime_config.clone()).expect("Failed to create LocalRuntime");

    let total_batches = 300;
    let ticks_per_batch = runtime_config.sync_batch_ticks as usize;

    let mut batches_csv_lines = Vec::new();
    let mut outputs_csv_lines = Vec::new();
    let mut output_spikes_csv_lines = Vec::new();

    let mut wall_times = Vec::new();
    let mut backend_times = Vec::new();
    let mut total_input_active_bits = 0;
    let mut total_incoming_spikes = 0;

    let mut max_generated_per_batch = 0u32;
    let mut max_output_per_tick = 0u32;
    let mut nonzero_output_ticks = 0usize;
    let mut nonzero_generated_batches = 0usize;

    let mut pseudo_rand = 987654321u32;
    let mut next_pseudo_rand = || -> u32 {
        pseudo_rand = pseudo_rand.wrapping_mul(1664525).wrapping_add(1013904223);
        pseudo_rand
    };

    for b in 0..total_batches {
        let mut bitmask = vec![0u32; ticks_per_batch];
        if b < 100 {
            for (t, mask) in bitmask.iter_mut().enumerate() {
                if t % 2 == 0 {
                    *mask = 1; // activate axon 0
                }
            }
        } else if b >= 200 {
            for mask in &mut bitmask {
                *mask = next_pseudo_rand() % 4; // randomly pulse virtual axons
            }
        }

        let mut active_bits = 0;
        for &word in &bitmask {
            active_bits += word.count_ones() as usize;
        }
        total_input_active_bits += active_bits;

        let mut spike_counts = vec![0u32; ticks_per_batch];
        let max_spikes = runtime_config.max_spikes_per_tick as usize;
        let mut spikes_buffer = vec![0u32; ticks_per_batch * max_spikes];
        let mut has_spikes = false;

        if b % 2 != 0 {
            for (t, count) in spike_counts.iter_mut().enumerate() {
                if next_pseudo_rand() % 3 == 0 {
                    *count = 1;
                    let offset = t * max_spikes;
                    spikes_buffer[offset] = next_pseudo_rand() % 2; // target virtual axon 0 or 1
                    has_spikes = true;
                }
            }
        }
        let incoming_count: usize = spike_counts.iter().sum::<u32>() as usize;
        total_incoming_spikes += incoming_count;

        let input = RuntimeBatchInput {
            input_bitmask: Some(&bitmask),
            incoming_spikes: if has_spikes {
                Some(&spikes_buffer)
            } else {
                None
            },
            incoming_spike_counts: &spike_counts,
        };

        let start_time = Instant::now();
        let report_res = runtime.run_batch(input);
        let wall_elapsed = start_time.elapsed().as_micros() as u64;

        let report = report_res.expect("Batch execution failed");
        wall_times.push(wall_elapsed);
        backend_times.push(report.batch_result.execution_time_us);

        let gen_spikes_batch = report.batch_result.generated_spikes_count;
        if gen_spikes_batch > max_generated_per_batch {
            max_generated_per_batch = gen_spikes_batch;
        }
        if gen_spikes_batch > 0 {
            nonzero_generated_batches += 1;
        }

        let stats = runtime.stats();

        batches_csv_lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            scenario_name,
            b,
            report.tick_base,
            report.ticks_executed,
            gen_spikes_batch,
            report.batch_result.output_spikes_written,
            report.batch_result.dropped_spikes_count,
            wall_elapsed,
            report.batch_result.execution_time_us,
            stats.current_tick,
            stats.generated_spikes,
            stats.output_spikes_written,
            stats.dropped_spikes
        ));

        for t in 0..ticks_per_batch {
            let tick_index = report.tick_base + t as u64;
            let count = report.output_spike_counts[t];
            if count > max_output_per_tick {
                max_output_per_tick = count;
            }
            if count > 0 {
                nonzero_output_ticks += 1;
            }

            let input_active_bits = bitmask[t].count_ones();
            let input_word = bitmask[t];
            let incoming_count_tick = spike_counts[t];
            outputs_csv_lines.push(format!(
                "{},{},{},{},{},{},{}",
                scenario_name,
                b,
                tick_index,
                count,
                input_active_bits,
                input_word,
                incoming_count_tick
            ));

            for slot in 0..count as usize {
                let idx = t * max_spikes + slot;
                let soma_id = report.output_spikes[idx];
                output_spikes_csv_lines.push(format!(
                    "{},{},{},{},{}",
                    scenario_name, b, tick_index, slot, soma_id
                ));
            }
        }
    }

    let stats = runtime.stats();
    let final_state = runtime.state();
    let total_ticks = stats.current_tick;

    let activity_status = if stats.generated_spikes > 0 || stats.output_spikes_written > 0 {
        "Active"
    } else {
        "Silent"
    };

    if activity_status == "Silent" {
        println!(
            "WARNING: Scenario '{}' produced 0 generated or output spikes!",
            scenario_name
        );
    }

    let backend_exec_mean = backend_times.iter().sum::<u64>() as f64 / total_batches as f64;
    let wall_exec_mean = wall_times.iter().sum::<u64>() as f64 / total_batches as f64;

    let summary_records = vec![
        ("scenario".to_string(), scenario_name.to_string()),
        ("activity_status".to_string(), activity_status.to_string()),
        ("total_batches".to_string(), total_batches.to_string()),
        ("total_ticks".to_string(), total_ticks.to_string()),
        (
            "total_generated_spikes".to_string(),
            stats.generated_spikes.to_string(),
        ),
        (
            "total_output_spikes_written".to_string(),
            stats.output_spikes_written.to_string(),
        ),
        (
            "total_dropped_spikes".to_string(),
            stats.dropped_spikes.to_string(),
        ),
        (
            "max_generated_per_batch".to_string(),
            max_generated_per_batch.to_string(),
        ),
        (
            "max_output_per_tick".to_string(),
            max_output_per_tick.to_string(),
        ),
        (
            "nonzero_output_ticks".to_string(),
            nonzero_output_ticks.to_string(),
        ),
        (
            "nonzero_generated_batches".to_string(),
            nonzero_generated_batches.to_string(),
        ),
        (
            "backend_execution_time_us_mean".to_string(),
            format!("{:.2}", backend_exec_mean),
        ),
        (
            "wall_time_us_mean".to_string(),
            format!("{:.2}", wall_exec_mean),
        ),
        (
            "total_input_active_bits".to_string(),
            total_input_active_bits.to_string(),
        ),
        (
            "total_incoming_spikes".to_string(),
            total_incoming_spikes.to_string(),
        ),
        (
            "final_runtime_tick".to_string(),
            stats.current_tick.to_string(),
        ),
        (
            "final_runtime_state".to_string(),
            format!("{:?}", final_state),
        ),
        ("backend_kind".to_string(), "CPU".to_string()),
        ("axic_bytes".to_string(), axic_size.to_string()),
        (
            "state_bytes".to_string(),
            artifacts.state_blob.len().to_string(),
        ),
        (
            "axons_bytes".to_string(),
            artifacts.axons_blob.len().to_string(),
        ),
        (
            "paths_bytes".to_string(),
            artifacts.paths_blob.len().to_string(),
        ),
    ];

    let _ = remove_file(temp_axic_path);

    ScenarioResult {
        batches_csv_lines,
        outputs_csv_lines,
        output_spikes_csv_lines,
        summary_records,
    }
}

#[test]
#[ignore]
fn test_full_chain_probe() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let artifacts_dir = workspace_dir.join("artifacts");
    create_dir_all(&artifacts_dir).expect("Failed to create artifacts directory");

    // 1. Run baseline scenario
    let baseline_config = make_probe_config_baseline();
    let baseline_res = run_scenario("baseline", baseline_config, 10);

    // 2. Run stimulated scenario
    let stimulated_config = make_probe_config_stimulated();
    let stimulated_res = run_scenario("stimulated", stimulated_config, 200);

    // 3. Write combined batches.csv
    println!("Writing combined batches.csv...");
    let batches_csv_path = artifacts_dir.join("full_chain_runtime_batches.csv");
    let mut batches_file = File::create(&batches_csv_path).unwrap();
    writeln!(
        batches_file,
        "scenario,batch_idx,tick_base,ticks_executed,generated_spikes,output_spikes_written,dropped_spikes,wall_time_us,backend_execution_time_us,runtime_tick,cum_generated,cum_output_written,cum_dropped"
    )
    .unwrap();
    for rec in &baseline_res.batches_csv_lines {
        writeln!(batches_file, "{}", rec).unwrap();
    }
    for rec in &stimulated_res.batches_csv_lines {
        writeln!(batches_file, "{}", rec).unwrap();
    }

    // 4. Write combined outputs.csv
    println!("Writing combined outputs.csv...");
    let outputs_csv_path = artifacts_dir.join("full_chain_runtime_outputs.csv");
    let mut outputs_file = File::create(&outputs_csv_path).unwrap();
    writeln!(
        outputs_file,
        "scenario,batch_idx,tick_index,output_spike_count,input_active_bits,input_word,incoming_count"
    )
    .unwrap();
    for rec in &baseline_res.outputs_csv_lines {
        writeln!(outputs_file, "{}", rec).unwrap();
    }
    for rec in &stimulated_res.outputs_csv_lines {
        writeln!(outputs_file, "{}", rec).unwrap();
    }

    // 5. Write combined output_spikes.csv
    println!("Writing output spikes CSV...");
    let output_spikes_csv_path = artifacts_dir.join("full_chain_runtime_output_spikes.csv");
    let mut output_spikes_file = File::create(&output_spikes_csv_path).unwrap();
    writeln!(
        output_spikes_file,
        "scenario,batch_idx,tick_index,slot,soma_id"
    )
    .unwrap();
    for rec in &baseline_res.output_spikes_csv_lines {
        writeln!(output_spikes_file, "{}", rec).unwrap();
    }
    for rec in &stimulated_res.output_spikes_csv_lines {
        writeln!(output_spikes_file, "{}", rec).unwrap();
    }

    // 6. Write combined summary.csv
    println!("Writing summary CSV...");
    let summary_csv_path = artifacts_dir.join("full_chain_runtime_summary.csv");
    let mut summary_file = File::create(&summary_csv_path).unwrap();
    writeln!(summary_file, "key,value").unwrap();

    // Write baseline summary values
    for (k, v) in &baseline_res.summary_records {
        writeln!(summary_file, "baseline_{},{}", k, v).unwrap();
    }
    // Write stimulated summary values
    for (k, v) in &stimulated_res.summary_records {
        writeln!(summary_file, "stimulated_{},{}", k, v).unwrap();
    }

    println!("\n==bananes=============================================");
    println!("  AXIENGINE FULL CHAIN LOCAL RUNTIME DIAGNOSTIC SUMMARY");
    println!("=============================================LoL========");
    println!("  -- BASELINE SCENARIO --");
    for (k, v) in &baseline_res.summary_records {
        println!("  baseline_{:<30} : {}", k, v);
    }
    println!("\n  -- STIMULATED SCENARIO --");
    for (k, v) in &stimulated_res.summary_records {
        println!("  stimulated_{:<30} : {}", k, v);
    }
    println!("========================================================\n");
}
