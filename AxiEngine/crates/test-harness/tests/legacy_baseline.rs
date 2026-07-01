use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{bootstrap_local_shard_engine, LocalShardComputeInput};
use config::{LayerConfig, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings};
use runtime::{LocalRuntime, LocalRuntimeConfig, RuntimeBatchInput};
use serde::Deserialize;
use std::fs::{self, create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use types::MasterSeed;

#[derive(Deserialize)]
struct LegacyNeuronType {
    name: String,
    threshold: i32,
    rest_potential: i32,
    leak_shift: u32,
    homeostasis_penalty: i32,
    spontaneous_firing_period_ticks: u32,
    initial_synapse_weight: u16,
    gsop_potentiation: u16,
    gsop_depression: u16,
    homeostasis_decay: u8,
    refractory_period: u8,
    synapse_refractory_period: u8,
    signal_propagation_length: u32,
    is_inhibitory: bool,
    inertia_curve: Vec<u8>,
    ahp_amplitude: i32,
    adaptive_leak_min_shift: u8,
    adaptive_leak_gain: u8,
    adaptive_mode: u8,
    d1_affinity: u8,
    d2_affinity: u8,
    steering_fov_deg: f32,
    steering_radius_um: f32,
    growth_vertical_bias: f32,
    dendrite_radius_um: f32,
    type_affinity: f32,
    sprouting_weight_distance: f32,
    sprouting_weight_power: f32,
    sprouting_weight_explore: f32,
    sprouting_weight_type: f32,
    steering_weight_inertia: f32,
    steering_weight_sensor: f32,
    steering_weight_jitter: f32,
}

#[derive(Deserialize)]
struct LegacyNeuronFile {
    neuron_type: Vec<LegacyNeuronType>,
}

fn map_legacy_to_config(
    legacy: &LegacyNeuronType,
    disable_spontaneous: bool,
) -> config::NeuronType {
    config::NeuronType {
        name: legacy.name.clone(),
        membrane: config::MembraneParams {
            threshold: legacy.threshold,
            rest_potential: legacy.rest_potential,
            leak_shift: legacy.leak_shift,
            ahp_amplitude: legacy.ahp_amplitude as u16,
        },
        timing: config::TimingParams {
            refractory_period: legacy.refractory_period,
            synapse_refractory_period: legacy.synapse_refractory_period,
        },
        signal: config::SignalParams {
            signal_propagation_length: legacy.signal_propagation_length as u8,
        },
        homeostasis: config::HomeostasisParams {
            homeostasis_penalty: legacy.homeostasis_penalty,
            homeostasis_decay: legacy.homeostasis_decay as u16,
        },
        adaptive_leak: config::AdaptiveLeakParams {
            adaptive_leak_min_shift: legacy.adaptive_leak_min_shift as i32,
            adaptive_leak_gain: legacy.adaptive_leak_gain as u16,
            adaptive_mode: legacy.adaptive_mode,
        },
        dopamine: config::DopamineParams {
            d1_affinity: legacy.d1_affinity,
            d2_affinity: legacy.d2_affinity,
        },
        gsop: config::GsopParams {
            gsop_potentiation: legacy.gsop_potentiation,
            gsop_depression: legacy.gsop_depression,
            initial_synapse_weight: legacy.initial_synapse_weight,
            is_inhibitory: legacy.is_inhibitory,
            inertia_curve: legacy.inertia_curve.clone(),
        },
        growth: config::GrowthParams {
            steering_fov_deg: legacy.steering_fov_deg,
            steering_radius_um: legacy.steering_radius_um,
            steering_weight_inertia: legacy.steering_weight_inertia,
            steering_weight_sensor: legacy.steering_weight_sensor,
            steering_weight_jitter: legacy.steering_weight_jitter,
            dendrite_radius_um: legacy.dendrite_radius_um,
            growth_vertical_bias: legacy.growth_vertical_bias,
            type_affinity: legacy.type_affinity,
            dendrite_whitelist: vec![],
            sprouting_weight_distance: legacy.sprouting_weight_distance,
            sprouting_weight_power: legacy.sprouting_weight_power,
            sprouting_weight_explore: legacy.sprouting_weight_explore,
            sprouting_weight_type: legacy.sprouting_weight_type,
        },
        spontaneous: config::SpontaneousParams {
            spontaneous_firing_period_ticks: if disable_spontaneous {
                0
            } else {
                legacy.spontaneous_firing_period_ticks
            },
        },
    }
}

fn get_workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // AxiEngine
    path
}

fn make_legacy_shard_config(
    exc: config::NeuronType,
    inh: config::NeuronType,
    density: f64,
    inhibitory_share: f64,
) -> ShardConfig {
    let neuron_types = vec![exc, inh];
    let composition = vec![
        NeuronTypeDistribution {
            type_name: "L23_spiny_VISp23_1".to_string(),
            share: (1.0 - inhibitory_share) as f32,
        },
        NeuronTypeDistribution {
            type_name: "L23_aspiny_VISp23_1".to_string(),
            share: inhibitory_share as f32,
        },
    ];

    let layers = vec![LayerConfig {
        name: "L23".to_string(),
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
fn test_legacy_baseline() {
    let workspace = get_workspace_root();
    let exc_toml_path = Path::new(
        "W:\\Workspace\\axicor-master\\Axicor_Neuron-Lib\\Cortex\\L23\\spiny\\VISp23\\1.toml",
    );
    let inh_toml_path = Path::new(
        "W:\\Workspace\\axicor-master\\Axicor_Neuron-Lib\\Cortex\\L23\\aspiny\\VISp23\\1.toml",
    );

    let exc_toml_str = fs::read_to_string(exc_toml_path).expect("Failed to read excitatory TOML");
    let inh_toml_str = fs::read_to_string(inh_toml_path).expect("Failed to read inhibitory TOML");

    let exc_file: LegacyNeuronFile =
        toml::from_str(&exc_toml_str).expect("Failed to deserialize excitatory TOML");
    let inh_file: LegacyNeuronFile =
        toml::from_str(&inh_toml_str).expect("Failed to deserialize inhibitory TOML");

    let exc_legacy = &exc_file.neuron_type[0];
    let inh_legacy = &inh_file.neuron_type[0];

    // Config 1: Spontaneous Enabled
    let exc_type_spon = map_legacy_to_config(exc_legacy, false);
    let inh_type_spon = map_legacy_to_config(inh_legacy, false);

    // Config 2: Spontaneous Disabled
    let exc_type_no_spon = map_legacy_to_config(exc_legacy, true);
    let inh_type_no_spon = map_legacy_to_config(inh_legacy, true);

    let density = 0.2;
    let inhibitory_share = 0.2;

    // Bake both configurations
    let shard_config_spon =
        make_legacy_shard_config(exc_type_spon, inh_type_spon, density, inhibitory_share);
    let baker_input_spon = LocalShardBakeInput {
        shard_config: &shard_config_spon,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts_spon, report_spon) = bake_local_shard(&baker_input_spon).expect("Baking failed");
    let axic_data_spon = pack_local_shard_artifacts(&artifacts_spon).expect("Packaging failed");

    let shard_config_no_spon = make_legacy_shard_config(
        exc_type_no_spon,
        inh_type_no_spon,
        density,
        inhibitory_share,
    );
    let baker_input_no_spon = LocalShardBakeInput {
        shard_config: &shard_config_no_spon,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts_no_spon, report_no_spon) =
        bake_local_shard(&baker_input_no_spon).expect("Baking failed");
    let axic_data_no_spon =
        pack_local_shard_artifacts(&artifacts_no_spon).expect("Packaging failed");

    let artifacts_dir = workspace.join("artifacts");
    create_dir_all(&artifacts_dir).unwrap();

    let decomp_csv_path = artifacts_dir.join("legacy_baseline_causality_decomposition.csv");
    let mut decomp_file = File::create(&decomp_csv_path).unwrap();

    writeln!(
        decomp_file,
        "scenario,total_somas,excitatory_somas,inhibitory_somas,spontaneous_period_exc,spontaneous_period_inh,dds_heartbeat_m_exc,dds_heartbeat_m_inh,estimated_spontaneous_seeds_per_tick,total_generated_spikes,total_output_spikes_written,dropped_ratio,nonzero_output_ticks,first_output_tick,last_output_tick,mean_output_per_nonzero_tick,max_output_per_tick,activity_status"
    ).unwrap();

    let total_ticks = 1000;

    // Scenario 1: legacy_spontaneous + no_stimulus
    {
        let full_bitmask = vec![0u32; total_ticks];
        run_scenario_decomp(
            "legacy_spontaneous + no_stimulus",
            &axic_data_spon,
            report_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            false,
        );
    }

    // Scenario 2: spontaneous_disabled + no_stimulus
    {
        let full_bitmask = vec![0u32; total_ticks];
        run_scenario_decomp(
            "spontaneous_disabled + no_stimulus",
            &axic_data_no_spon,
            report_no_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            true,
        );
    }

    // Scenario 3: spontaneous_disabled + single_pulse_1
    {
        let mut full_bitmask = vec![0u32; total_ticks];
        full_bitmask[0] = 0b1;
        run_scenario_decomp(
            "spontaneous_disabled + single_pulse_1",
            &axic_data_no_spon,
            report_no_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            true,
        );
    }

    // Scenario 4: spontaneous_disabled + single_pulse_2
    {
        let mut full_bitmask = vec![0u32; total_ticks];
        full_bitmask[0] = 0b11;
        run_scenario_decomp(
            "spontaneous_disabled + single_pulse_2",
            &axic_data_no_spon,
            report_no_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            true,
        );
    }

    // Scenario 5: spontaneous_disabled + single_pulse_3
    {
        let mut full_bitmask = vec![0u32; total_ticks];
        full_bitmask[0] = 0b111;
        run_scenario_decomp(
            "spontaneous_disabled + single_pulse_3",
            &axic_data_no_spon,
            report_no_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            true,
        );
    }

    // Scenario 6: spontaneous_disabled + periodic_pulse_1
    {
        let mut full_bitmask = vec![0u32; total_ticks];
        for (t, mask) in full_bitmask.iter_mut().enumerate() {
            if t % 100 == 0 {
                *mask = 0b1;
            }
        }
        run_scenario_decomp(
            "spontaneous_disabled + periodic_pulse_1",
            &axic_data_no_spon,
            report_no_spon.total_somas,
            &full_bitmask,
            &mut decomp_file,
            exc_legacy,
            inh_legacy,
            true,
        );
    }

    println!("Legacy baseline decomposition test successfully completed.");
}

#[allow(clippy::too_many_arguments)]
fn run_scenario_decomp(
    scenario_name: &str,
    axic_data: &[u8],
    total_somas: u32,
    full_bitmask: &[u32],
    decomp_file: &mut File,
    exc_legacy: &LegacyNeuronType,
    inh_legacy: &LegacyNeuronType,
    spontaneous_disabled: bool,
) {
    let temp_axic_path = std::env::temp_dir().join(format!(
        "legacy_decomp_{}.axic",
        scenario_name.replace(" + ", "_").replace(" ", "_")
    ));
    {
        let mut f = File::create(&temp_axic_path).unwrap();
        f.write_all(axic_data).unwrap();
    }

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
        sync_batch_ticks: 100,
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

    let total_batches = 10;
    let ticks_per_batch = 100;

    let mut total_generated = 0u64;
    let mut total_written = 0u64;
    let mut total_dropped = 0u64;
    let mut flat_output_spike_counts = Vec::new();

    for b in 0..total_batches {
        let start_tick = b * ticks_per_batch;
        let end_tick = start_tick + ticks_per_batch;
        let batch_bitmask = &full_bitmask[start_tick..end_tick];

        let input = RuntimeBatchInput {
            input_bitmask: Some(batch_bitmask),
            incoming_spikes: None,
            incoming_spike_counts: &vec![0; ticks_per_batch],
        };

        let report = runtime.run_batch(input).expect("Batch failed");

        total_generated += report.batch_result.generated_spikes_count as u64;
        total_written += report.batch_result.output_spikes_written as u64;
        total_dropped += report.batch_result.dropped_spikes_count as u64;

        flat_output_spike_counts.extend_from_slice(&report.output_spike_counts);
    }

    let _ = std::fs::remove_file(temp_axic_path);

    // Compute metrics
    let nonzero_output_ticks = flat_output_spike_counts.iter().filter(|&&c| c > 0).count() as u64;
    let mut first_output_tick = -1i32;
    let mut last_output_tick = -1i32;
    let mut peak_output_per_tick = 0u64;
    let mut sum_output_spikes = 0u64;

    for (t, &count) in flat_output_spike_counts.iter().enumerate() {
        if count > 0 {
            if first_output_tick == -1 {
                first_output_tick = t as i32;
            }
            last_output_tick = t as i32;
            if count as u64 > peak_output_per_tick {
                peak_output_per_tick = count as u64;
            }
            sum_output_spikes += count as u64;
        }
    }

    let mean_output_per_nonzero_tick = if nonzero_output_ticks > 0 {
        sum_output_spikes as f64 / nonzero_output_ticks as f64
    } else {
        0.0
    };

    let dropped_ratio = if total_generated > 0 {
        total_dropped as f64 / total_generated as f64
    } else {
        0.0
    };

    let activity_status = if total_generated == 0 {
        "no-response"
    } else if last_output_tick < 900 {
        "transient-response"
    } else {
        if total_dropped > 0 {
            "runaway"
        } else {
            "sustained-activity"
        }
    };

    let excitatory_somas = (total_somas as f64 * 0.8).round() as u32;
    let inhibitory_somas = (total_somas as f64 * 0.2).round() as u32;

    let (spon_period_exc, spon_period_inh) = if spontaneous_disabled {
        (0, 0)
    } else {
        (
            exc_legacy.spontaneous_firing_period_ticks,
            inh_legacy.spontaneous_firing_period_ticks,
        )
    };

    let dds_heartbeat_m_exc = if spontaneous_disabled {
        0
    } else {
        physics::compile_dds_heartbeat(exc_legacy.spontaneous_firing_period_ticks as u64)
    };

    let dds_heartbeat_m_inh = if spontaneous_disabled {
        0
    } else {
        physics::compile_dds_heartbeat(inh_legacy.spontaneous_firing_period_ticks as u64)
    };

    let estimated_spontaneous_seeds_per_tick = if spontaneous_disabled {
        0.0
    } else {
        (excitatory_somas as f64 / exc_legacy.spontaneous_firing_period_ticks as f64)
            + (inhibitory_somas as f64 / inh_legacy.spontaneous_firing_period_ticks as f64)
    };

    writeln!(
        decomp_file,
        "{},{},{},{},{},{},{},{},{:.6},{},{},{:.6},{},{},{},{:.6},{},{}",
        scenario_name,
        total_somas,
        excitatory_somas,
        inhibitory_somas,
        spon_period_exc,
        spon_period_inh,
        dds_heartbeat_m_exc,
        dds_heartbeat_m_inh,
        estimated_spontaneous_seeds_per_tick,
        total_generated,
        total_written,
        dropped_ratio,
        nonzero_output_ticks,
        first_output_tick,
        last_output_tick,
        mean_output_per_nonzero_tick,
        peak_output_per_tick,
        activity_status
    )
    .unwrap();
}
