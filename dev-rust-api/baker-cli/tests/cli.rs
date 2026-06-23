use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// Import the Cli parser from baker-cli
#[path = "../src/main.rs"]
#[allow(dead_code)]
mod cli_main;

use cli_main::{Cli, Commands};

#[test]
fn test_cli_parse_bake() {
    let args = Cli::try_parse_from(&["baker-cli", "bake", "some_config_dir", "-o", "some_output.axic"]).unwrap();
    match args.command {
        Commands::Bake { config_dir, output } => {
            assert_eq!(config_dir, PathBuf::from("some_config_dir"));
            assert_eq!(output, PathBuf::from("some_output.axic"));
        }
        _ => panic!("Expected Bake command"),
    }
}

#[test]
fn test_cli_parse_distill() {
    let args = Cli::try_parse_from(&[
        "baker-cli",
        "distill",
        "some_archive.axic",
        "-o",
        "some_out_dir",
        "-k",
        "16",
    ])
    .unwrap();
    match args.command {
        Commands::Distill {
            archive,
            out_dir,
            target_slots,
        } => {
            assert_eq!(archive, PathBuf::from("some_archive.axic"));
            assert_eq!(out_dir, PathBuf::from("some_out_dir"));
            assert_eq!(target_slots, 16);
        }
        _ => panic!("Expected Distill command"),
    }
}

#[test]
fn test_cli_parse_invalid_args() {
    let res = Cli::try_parse_from(&["baker-cli", "invalid_cmd"]);
    assert!(res.is_err());
}

fn write_mock_configs(config_dir: &std::path::Path) {
    let model_toml = r#"
        [world]
        width_um = 100.0
        depth_um = 100.0
        height_um = 100.0

        [simulation]
        tick_duration_us = 1000
        total_ticks = 0
        master_seed = "test-seed"
        voxel_size_um = 10.0
        segment_length_voxels = 2
        signal_speed_m_s = 2.0
        sync_batch_ticks = 10
        axon_growth_max_steps = 200
        max_dendrites = 128

        [[departments]]
        name = "cortex"
        config = "brain_cortex.toml"

        [[connections]]
        from = "cortex.relay"
        to = "cortex.relay"
    "#;

    let shard_toml = r#"
        [dimensions]
        w = 10
        d = 10
        h = 10

        [[layers]]
        name = "L1"
        height_pct = 1.0
        density = 0.1
        composition = [
            { type_name = "Excitatory", share = 1.0 }
        ]

        [[neuron_types]]
        name = "Excitatory"
        
          [neuron_types.membrane]
          threshold = 20000
          rest_potential = -70000
          leak_shift = 4
          ahp_amplitude = 0
          
          [neuron_types.timings]
          refractory_period = 5
          synapse_refractory_period = 10
          
          [neuron_types.signal]
          signal_propagation_length = 8
          
          [neuron_types.homeostasis]
          homeostasis_penalty = 1500
          homeostasis_decay = 990
          
          [neuron_types.adaptive_leak]
          adaptive_leak_min_shift = -5
          adaptive_leak_gain = 2
          adaptive_mode = 1
          
          [neuron_types.dopamine]
          d1_affinity = 80
          d2_affinity = 20
          
          [neuron_types.gsop]
          gsop_potentiation = 15
          gsop_depression = 5
          is_inhibitory = false
          inertia_curve = [10, 20, 30, 40, 50, 60, 70, 80]

          [neuron_types.growth]
          steering_fov_deg = 60.0
          steering_radius_um = 100.0
          steering_weight_inertia = 0.6
          steering_weight_sensor = 0.3
          steering_weight_jitter = 0.1
          dendrite_radius_um = 150.0
          growth_vertical_bias = 0.7
          type_affinity = 0.5
          dendrite_whitelist = []
          sprouting_weight_distance = 0.4
          sprouting_weight_power = 0.4
          sprouting_weight_explore = 0.1
          sprouting_weight_type = 0.1
          
          [neuron_types.spontaneous]
          spontaneous_firing_period_ticks = 10000

        [settings]
        ghost_capacity = 1000
        prune_threshold = 10
        max_sprouts = 4
        night_interval_ticks = 1000
        save_checkpoints_interval_ticks = 10000

        [[sockets]]
        name = "relay"
        direction = "in"
        width = 4
        height = 4
        entry_z = "Bottom"
        target_type = "Excitatory"
        growth_steps = 10
    "#;

    fs::write(config_dir.join("model.toml"), model_toml).unwrap();
    fs::write(config_dir.join("shard.toml"), shard_toml).unwrap();
}
#[test]
fn test_cli_bake_and_distill_integration() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir().unwrap();
    let config_dir = temp.path().join("configs");
    fs::create_dir(&config_dir).unwrap();
    write_mock_configs(&config_dir);
    let archive_file = temp.path().join("model.axic");

    // Test Bake pipeline
    let bake_args = Cli::try_parse_from(&[
        "baker-cli",
        "bake",
        config_dir.to_str().unwrap(),
        "-o",
        archive_file.to_str().unwrap(),
    ])
    .unwrap();

    // Directly invoke the bake routing logic
    let res = match bake_args.command {
        Commands::Bake { config_dir, output } => baker::bake(&config_dir, &output),
        _ => unreachable!(),
    };
    assert!(res.is_ok());
    assert!(archive_file.exists());

    // Test Distill pipeline
    let out_dir = temp.path().join("edge_out");
    fs::create_dir(&out_dir).unwrap();

    let distill_args = Cli::try_parse_from(&[
        "baker-cli",
        "distill",
        archive_file.to_str().unwrap(),
        "-o",
        out_dir.to_str().unwrap(),
        "-k",
        "16",
    ])
    .unwrap();

    // Directly invoke the distill routing logic
    let res: Result<(), Box<dyn std::error::Error>> = match distill_args.command {
        Commands::Distill {
            archive,
            out_dir,
            target_slots,
        } => {
            let ax_archive = vfs::AxicArchive::open(&archive)?;
            let config = edge_model::EdgeConfig {
                target_dendrite_slots: target_slots,
            };
            let model = edge_model::distill::convert_archive(&ax_archive, &config)?;
            edge_model::export::export_c_headers(&model, &out_dir)?;
            Ok(())
        }
        _ => unreachable!(),
    };
    assert!(res.is_ok());

    // Verify expected C headers and binary blobs are created
    assert!(out_dir.join("axicor_hot_state.bin").exists());
    assert!(out_dir.join("axicor_static_topology.bin").exists());
    assert!(out_dir.join("axicor_hot_state.h").exists());
    assert!(out_dir.join("axicor_static_topology.h").exists());

    Ok(())
}

#[test]
fn test_cli_edge_slots_bounds() {
    let temp = tempdir().unwrap();
    let config_dir = temp.path().join("configs");
    fs::create_dir(&config_dir).unwrap();
    write_mock_configs(&config_dir);
    let archive_file = temp.path().join("model.axic");

    // Generate valid archive first
    baker::bake(&config_dir, &archive_file).unwrap();

    let out_dir = temp.path().join("edge_out");
    fs::create_dir(&out_dir).unwrap();

    // Test slots target limit > 128
    {
        let ax_archive = vfs::AxicArchive::open(&archive_file).unwrap();
        let config = edge_model::EdgeConfig {
            target_dendrite_slots: 129,
        };
        let res = edge_model::distill::convert_archive(&ax_archive, &config);
        assert!(res.is_err());
    }

    // Test slots target limit == 0
    {
        let ax_archive = vfs::AxicArchive::open(&archive_file).unwrap();
        let config = edge_model::EdgeConfig {
            target_dendrite_slots: 0,
        };
        let res = edge_model::distill::convert_archive(&ax_archive, &config);
        assert!(res.is_err());
    }
}

#[test]
fn test_cli_corrupted_archive() {
    let temp = tempdir().unwrap();
    let corrupted_file = temp.path().join("corrupted.axic");
    fs::write(&corrupted_file, b"not a valid archive").unwrap();

    let res = vfs::AxicArchive::open(&corrupted_file);
    assert!(res.is_err());
}
