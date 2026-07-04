//! Integration test suite for Stage A local archive bootloader.

use std::fs::{remove_file, File};
use std::io::Write;
use std::path::PathBuf;

use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
use boot::{LocalShardBootInput, LocalShardComputeInput};
use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use types::MasterSeed;
use vfs::{pack_entries, ArchiveEntry};

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
        density: 0.2, // sparse enough to deterministic place
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
    temp.push(format!("boot_test_{}.axic", rand));
    temp
}

fn generate_valid_axic_bytes() -> Vec<u8> {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    pack_local_shard_artifacts(&artifacts).unwrap()
}

fn get_valid_artifacts() -> baker::LocalShardArtifacts {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    artifacts
}

#[test]
fn test_boot_stage_a_load_baker_axic_success() {
    let bytes = generate_valid_axic_bytes();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&bytes).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let bundle = boot::load_local_shard_archive(&input);
    assert!(
        bundle.is_ok(),
        "Expected load to succeed, got: {:?}",
        bundle.err()
    );
    let bundle = bundle.unwrap();
    assert!(!bundle.state_blob.is_empty());
    assert!(!bundle.axons_blob.is_empty());
    assert!(!bundle.paths_blob.is_empty());

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_missing_required_file() {
    let artifacts = get_valid_artifacts();
    // Exclude variant_table.bin
    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::load_local_shard_archive(&input);
    assert!(matches!(
        res,
        Err(boot::BootError::MissingRequiredFile { .. })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_reject_bad_state_header() {
    let artifacts = get_valid_artifacts();
    // Corrupt state magic
    let mut bad_state = artifacts.state_blob.clone();
    bad_state[0] = 0;

    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &bad_state,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
        ArchiveEntry {
            path: boot::VARIANT_TABLE_ARCHIVE_PATH,
            bytes: bytemuck::cast_slice(&artifacts.variant_table),
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::load_local_shard_archive(&input);
    assert!(matches!(
        res,
        Err(boot::BootError::InvalidArtifact {
            path: boot::STATE_ARCHIVE_PATH,
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_reject_bad_axons_header() {
    let artifacts = get_valid_artifacts();
    // Corrupt axons magic
    let mut bad_axons = artifacts.axons_blob.clone();
    bad_axons[0] = 0;

    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &bad_axons,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
        ArchiveEntry {
            path: boot::VARIANT_TABLE_ARCHIVE_PATH,
            bytes: bytemuck::cast_slice(&artifacts.variant_table),
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::load_local_shard_archive(&input);
    assert!(matches!(
        res,
        Err(boot::BootError::InvalidArtifact {
            path: boot::AXONS_ARCHIVE_PATH,
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_reject_bad_paths_header() {
    let artifacts = get_valid_artifacts();
    // Corrupt paths magic
    let mut bad_paths = artifacts.paths_blob.clone();
    bad_paths[0] = 0;

    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &bad_paths,
        },
        ArchiveEntry {
            path: boot::VARIANT_TABLE_ARCHIVE_PATH,
            bytes: bytemuck::cast_slice(&artifacts.variant_table),
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::load_local_shard_archive(&input);
    assert!(matches!(
        res,
        Err(boot::BootError::InvalidArtifact {
            path: boot::PATHS_ARCHIVE_PATH,
            ..
        })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_reject_variant_table_size() {
    let artifacts = get_valid_artifacts();
    // Shorten variant table bytes
    let vt_bytes = bytemuck::cast_slice::<layout::VariantParameters, u8>(&artifacts.variant_table);
    let short_vt = &vt_bytes[0..vt_bytes.len() - 1];

    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
        ArchiveEntry {
            path: boot::VARIANT_TABLE_ARCHIVE_PATH,
            bytes: short_vt,
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::load_local_shard_archive(&input);
    assert!(matches!(
        res,
        Err(boot::BootError::VariantTableSizeMismatch { .. })
    ));

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_alloc_spec_from_headers() {
    let bytes = generate_valid_axic_bytes();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&bytes).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 10,
        total_ghosts: 5,
    };
    let bundle = boot::load_local_shard_archive(&input).unwrap();
    assert_eq!(bundle.spec.virtual_offset, 10);
    assert_eq!(bundle.spec.total_ghosts, 5);
    assert!(bundle.spec.padded_n > 0);
    assert!(bundle.spec.total_axons > 0);

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_upload_view_matches_owned_buffers() {
    let bytes = generate_valid_axic_bytes();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&bytes).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let bundle = boot::load_local_shard_archive(&input).unwrap();
    let upload = bundle.upload();
    assert_eq!(upload.state_blob, &bundle.state_blob[..]);
    assert_eq!(upload.axons_blob, &bundle.axons_blob[..]);
    assert_eq!(
        bytemuck::cast_slice::<layout::VariantParameters, u8>(upload.variant_table),
        bytemuck::cast_slice::<layout::VariantParameters, u8>(&bundle.variant_table)
    );

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_compute_api_validation() {
    let bytes = generate_valid_axic_bytes();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&bytes).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let mut bundle = boot::load_local_shard_archive(&input).unwrap();
    // Validate it works initially
    assert!(compute_api::validation::validate_upload(&bundle.spec, &bundle.upload()).is_ok());

    // Corrupt spec to trigger validation failure
    bundle.spec.padded_n = 99999;
    assert!(compute_api::validation::validate_upload(&bundle.spec, &bundle.upload()).is_err());

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_bootstrap_mock_or_cpu_engine() {
    let bytes = generate_valid_axic_bytes();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&bytes).unwrap();
    }

    let input = LocalShardComputeInput {
        archive_path: path.clone(),
        backend_preference: compute::BackendPreference::Cpu,
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let res = boot::bootstrap_local_shard_engine(&input);
    assert!(
        res.is_ok(),
        "Failed to bootstrap CPU engine: {:?}",
        res.err()
    );
    let (engine, bundle) = res.unwrap();
    assert!(!bundle.state_blob.is_empty());
    drop(engine);

    let _ = remove_file(path);
}

#[test]
fn test_boot_stage_a_ignore_extra_files() {
    let artifacts = get_valid_artifacts();
    let entries = [
        ArchiveEntry {
            path: boot::STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: boot::AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: boot::PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
        ArchiveEntry {
            path: boot::VARIANT_TABLE_ARCHIVE_PATH,
            bytes: bytemuck::cast_slice(&artifacts.variant_table),
        },
        ArchiveEntry {
            path: "manifest.toml",
            bytes: b"ignored_manifest_content = true",
        },
        ArchiveEntry {
            path: "extra.bin",
            bytes: &[1, 2, 3, 4],
        },
    ];
    let packed = pack_entries(&entries).unwrap();
    let path = get_temp_axic_path();
    {
        let mut f = File::create(&path).unwrap();
        f.write_all(&packed).unwrap();
    }

    let input = LocalShardBootInput {
        archive_path: path.clone(),
        virtual_offset: 0,
        total_ghosts: 0,
    };
    let bundle = boot::load_local_shard_archive(&input);
    assert!(
        bundle.is_ok(),
        "Expected bootloader to ignore extra files and succeed"
    );
    let bundle = bundle.unwrap();
    assert!(!bundle.state_blob.is_empty());

    let _ = remove_file(path);
}
