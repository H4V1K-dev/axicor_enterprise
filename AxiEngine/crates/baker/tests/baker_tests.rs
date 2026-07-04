use baker::{bake_local_shard, LocalShardBakeInput};
use config::{
    LayerConfig, NeuronType, NeuronTypeDistribution, ShardConfig, ShardDimensions, ShardSettings,
};
use layout::{
    align_to_padded_n, calculate_paths_file_size, calculate_paths_matrix_offset,
    calculate_state_blob_size, AxonsFileHeader, BurstHeads8, PathsFileHeader, StateFileHeader,
    VariantParameters, MAX_DENDRITES, MAX_SEGMENTS_PER_AXON, VARIANT_LUT_LEN,
};
use std::io::Write;
use types::{MasterSeed, PackedPosition, AXON_SENTINEL};

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

#[test]
fn test_baker_stage_a_success() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let res = bake_local_shard(&input);
    assert!(res.is_ok());
    let (_artifacts, report) = res.unwrap();
    assert!(report.total_somas > 0);
    assert_eq!(report.total_axons, report.total_somas);
}

#[test]
fn test_baker_stage_a_state_header_and_sizes() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&input).unwrap();
    let padded_n = align_to_padded_n(report.total_somas as usize);

    // .state shape checks
    let expected_state_size = calculate_state_blob_size(padded_n);
    assert_eq!(artifacts.state_blob.len(), expected_state_size);

    let state_header: &StateFileHeader = bytemuck::from_bytes(&artifacts.state_blob[0..16]);
    assert_eq!(state_header.padded_n, padded_n as u32);
    assert_eq!(state_header.total_axons, report.total_axons);

    // .axons shape checks
    let expected_axons_size =
        16 + (report.total_axons as usize) * std::mem::size_of::<BurstHeads8>();
    assert_eq!(artifacts.axons_blob.len(), expected_axons_size);

    let axons_header: &AxonsFileHeader = bytemuck::from_bytes(&artifacts.axons_blob[0..16]);
    assert_eq!(axons_header.total_axons, report.total_axons);

    // .paths shape checks
    let expected_paths_size = calculate_paths_file_size(report.total_axons as usize);
    assert_eq!(artifacts.paths_blob.len(), expected_paths_size);

    let paths_header: &PathsFileHeader = bytemuck::from_bytes(&artifacts.paths_blob[0..16]);
    assert_eq!(paths_header.total_axons, report.total_axons);
    assert_eq!(paths_header.max_segments, MAX_SEGMENTS_PER_AXON as u32);
}

#[test]
fn test_baker_stage_a_variant_table_fixed_lut() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, _) = bake_local_shard(&input).unwrap();
    assert_eq!(artifacts.variant_table.len(), VARIANT_LUT_LEN);

    // Index 0 and 1 should be initialized with config parameters
    assert_eq!(artifacts.variant_table[0].rest_potential, -70);
    assert_eq!(artifacts.variant_table[1].rest_potential, -70);

    // Unused indexes should remain zeroed
    assert_eq!(artifacts.variant_table[2].rest_potential, 0);
}

#[test]
fn test_baker_stage_a_state_weights_in_mass_domain() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&input).unwrap();
    let padded_n = align_to_padded_n(report.total_somas as usize);
    let offsets = layout::compute_state_offsets(padded_n);

    // Read the weights matrix
    let weights_slice = &artifacts.state_blob
        [offsets.off_weights..offsets.off_weights + MAX_DENDRITES * padded_n * 4];
    let weights: &[i32] = bytemuck::cast_slice(weights_slice);

    // Normal initial weights are 100 in config.
    // If a synapse is formed, its weight in state blob should be 100 << 16 (Mass Domain) or - (100 << 16)
    let expected_mass = 100 << 16;
    let mut found_synapse = false;
    for &w in weights {
        if w != 0 {
            assert!(w == expected_mass || w == -expected_mass);
            found_synapse = true;
        }
    }
    // We expect at least one synapse in a dense random shard placement
    assert!(found_synapse);
}

#[test]
fn test_baker_stage_a_deterministic_output() {
    let config = make_baker_test_setup();
    let input1 = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let input2 = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (art1, rep1) = bake_local_shard(&input1).unwrap();
    let (art2, rep2) = bake_local_shard(&input2).unwrap();

    assert_eq!(rep1.total_somas, rep2.total_somas);
    assert_eq!(rep1.total_synapses, rep2.total_synapses);
    assert_eq!(art1.state_blob, art2.state_blob);
    assert_eq!(art1.axons_blob, art2.axons_blob);
    assert_eq!(art1.paths_blob, art2.paths_blob);
}

#[test]
fn test_baker_stage_a_paths_origin_and_segments() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&input).unwrap();
    let total_axons = report.total_axons as usize;

    // lengths plane
    let lengths_slice = &artifacts.paths_blob[16..16 + total_axons * 2];
    let lengths: &[u16] = bytemuck::cast_slice(lengths_slice);

    // matrix plane
    let matrix_offset = calculate_paths_matrix_offset(total_axons);
    let matrix_slice = &artifacts.paths_blob
        [matrix_offset..matrix_offset + total_axons * MAX_SEGMENTS_PER_AXON * 4];
    let matrix_pos: &[PackedPosition] = bytemuck::cast_slice(matrix_slice);

    #[allow(clippy::needless_range_loop)]
    for axon_id in 0..total_axons {
        let len = lengths[axon_id] as usize;
        assert!(len >= 1); // at least origin soma
        assert!(len <= MAX_SEGMENTS_PER_AXON); // must not exceed maximum segments count

        let base_idx = axon_id * MAX_SEGMENTS_PER_AXON;

        // Slot 0 has a valid soma position
        assert!(matrix_pos[base_idx].0 != 0);

        // slots 1..len must have non-zero coordinates
        let mut nonzero_count = 0;
        for idx in 1..len {
            assert!(matrix_pos[base_idx + idx].0 != 0);
            nonzero_count += 1;
        }

        // Verify that lengths[axon_id] == 1 + count of active segments slots
        assert_eq!(len, 1 + nonzero_count);

        // slots len..256 must be PackedPosition(0)
        for idx in len..MAX_SEGMENTS_PER_AXON {
            assert_eq!(matrix_pos[base_idx + idx].0, 0);
        }
    }
}

#[test]
fn test_baker_stage_a_axons_all_sentinel() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&input).unwrap();

    let heads_slice = &artifacts.axons_blob[16..];
    let mut u32_buf = vec![0u32; heads_slice.len() / 4];
    bytemuck::cast_slice_mut(&mut u32_buf).copy_from_slice(heads_slice);

    assert_eq!(u32_buf.len(), (report.total_axons as usize) * 8);
    for &val in &u32_buf {
        assert_eq!(val, AXON_SENTINEL);
    }
}

#[test]
fn test_baker_stage_a_compute_upload_compatible() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };

    let (artifacts, report) = bake_local_shard(&input).unwrap();
    let padded_n = align_to_padded_n(report.total_somas as usize);

    // Create ShardAllocSpec and ShardUpload
    let spec = compute_api::ShardAllocSpec {
        padded_n: padded_n as u32,
        total_axons: report.total_axons,
        total_ghosts: 0,
        virtual_offset: 0,
    };

    let upload = compute_api::ShardUpload {
        state_blob: &artifacts.state_blob,
        axons_blob: &artifacts.axons_blob,
        variant_table: &artifacts.variant_table,
    };

    // Run compute_api::validate_upload to ensure size and alignment compatibility
    let res = compute_api::validate_upload(&spec, &upload);
    assert!(res.is_ok());
}

#[test]
fn test_baker_stage_b_pack_artifacts_to_axic() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let res = baker::pack_local_shard_artifacts(&artifacts);
    assert!(res.is_ok());
    let packed = res.unwrap();
    assert!(!packed.is_empty());
}

#[test]
fn test_baker_stage_b_axic_roundtrip_with_vfs() {
    use baker::{
        AXONS_ARCHIVE_PATH, PATHS_ARCHIVE_PATH, STATE_ARCHIVE_PATH, VARIANT_TABLE_ARCHIVE_PATH,
    };
    use std::fs::remove_file;
    use std::io::Write;

    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = baker::pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("roundtrip_{}.axic", rand));

    {
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(&packed).unwrap();
    }

    let archive = vfs::AxicArchive::open(&temp).unwrap();

    assert_eq!(
        archive.require_file(STATE_ARCHIVE_PATH).unwrap(),
        &artifacts.state_blob[..]
    );
    assert_eq!(
        archive.require_file(AXONS_ARCHIVE_PATH).unwrap(),
        &artifacts.axons_blob[..]
    );
    assert_eq!(
        archive.require_file(PATHS_ARCHIVE_PATH).unwrap(),
        &artifacts.paths_blob[..]
    );
    assert_eq!(
        archive.require_file(VARIANT_TABLE_ARCHIVE_PATH).unwrap(),
        bytemuck::cast_slice(&artifacts.variant_table)
    );

    remove_file(temp).unwrap();
}

#[test]
fn test_baker_stage_b_variant_table_bytes() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = baker::pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("vt_test_{}.axic", rand));

    {
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(&packed).unwrap();
    }

    let archive = vfs::AxicArchive::open(&temp).unwrap();
    let vt_bytes = archive
        .require_file(baker::VARIANT_TABLE_ARCHIVE_PATH)
        .unwrap();
    let vt: &[VariantParameters; VARIANT_LUT_LEN] = bytemuck::from_bytes(vt_bytes);

    assert_eq!(
        vt[0].rest_potential,
        artifacts.variant_table[0].rest_potential
    );
    assert_eq!(
        vt[1].rest_potential,
        artifacts.variant_table[1].rest_potential
    );
    assert_eq!(vt[2].rest_potential, 0);

    std::fs::remove_file(temp).unwrap();
}

#[test]
fn test_baker_stage_b_deterministic_axic_output() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed1 = baker::pack_local_shard_artifacts(&artifacts).unwrap();
    let packed2 = baker::pack_local_shard_artifacts(&artifacts).unwrap();
    assert_eq!(packed1, packed2);
}

#[test]
fn test_baker_stage_b_bake_local_shard_axic_success() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (packed, report) = baker::bake_local_shard_axic(&input).unwrap();
    assert!(report.total_somas > 0);
    assert!(report.total_axons > 0);

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("bake_axic_{}.axic", rand));

    {
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(&packed).unwrap();
    }

    let archive = vfs::AxicArchive::open(&temp).unwrap();
    assert!(archive.contains(baker::STATE_ARCHIVE_PATH));
    assert!(archive.contains(baker::AXONS_ARCHIVE_PATH));
    assert!(archive.contains(baker::PATHS_ARCHIVE_PATH));
    assert!(archive.contains(baker::VARIANT_TABLE_ARCHIVE_PATH));

    std::fs::remove_file(temp).unwrap();
}

#[test]
fn test_baker_stage_b_archive_paths_are_stable() {
    let config = make_baker_test_setup();
    let input = LocalShardBakeInput {
        shard_config: &config,
        master_seed: MasterSeed(42),
        voxel_size_um: 1.0,
    };
    let (artifacts, _) = bake_local_shard(&input).unwrap();
    let packed = baker::pack_local_shard_artifacts(&artifacts).unwrap();

    let mut temp = std::env::temp_dir();
    let rand = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp.push(format!("paths_test_{}.axic", rand));

    {
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(&packed).unwrap();
    }

    let archive = vfs::AxicArchive::open(&temp).unwrap();
    let paths: Vec<&str> = archive.list_files().collect();
    assert_eq!(
        paths,
        vec![
            baker::AXONS_ARCHIVE_PATH,
            baker::PATHS_ARCHIVE_PATH,
            baker::STATE_ARCHIVE_PATH,
            baker::VARIANT_TABLE_ARCHIVE_PATH,
        ]
    );

    std::fs::remove_file(temp).unwrap();
}
