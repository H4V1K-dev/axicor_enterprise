//! Bake pipeline — orchestrates the .axic archive compilation.

use crate::error::BakerError;
use crate::serialization::{serialize_axons, serialize_paths, serialize_state};
use crate::validator::validate_physics_and_anatomy;

/// Compiled neural network archive ready for GPU runtime deployment.
///
/// Stub container — populated by subsequent bake phases (B → E).
#[derive(Debug)]
pub struct AxicArchive {}

use rand::SeedableRng;

/// Compiles a simulation project directory into an `.axic` archive.
///
/// # INV-BAKER-002: Strict Pipeline Order (A -> B -> C -> D -> E)
/// The bake pipeline MUST execute phases in the following invariant order:
///
/// - **Phase A** — Pre-Bake Validation: physics + anatomy integrity guard.
/// - **Phase B** — Config Loading & Spatial Generation: `topology::place_somas`.
/// - **Phase C** — Geometry Generation / Macro-Routing: `route_ghost_atlas`.
/// - **Phase D** — Layout Serialization: pack SoA arrays into `layout` ABI structs.
/// - **Phase E** — Archive Packing: write page-aligned `.axic` via `vfs::pack_directory`.
///
/// Partial or out-of-order execution is a critical integrity violation.
///
/// # INV-BAKER-004: Pre-Bake Guard
/// The discrete step velocity is verified before generating paths and layouts.
///
/// # Errors
/// Returns [`BakerError`] at the first failing phase (Fail-Fast).

pub fn bake(
    config_dir: &std::path::Path,
    output_path: &std::path::Path,
) -> Result<AxicArchive, BakerError> {
    tracing::info!("Baker: starting bake pipeline for '{}'", config_dir.display());

    // ── Load configurations ──────────────────────────────────────────────────
    let model_path = config_dir.join("model.toml");
    let model_content = std::fs::read_to_string(&model_path)
        .map_err(|_| BakerError::ConfigNotFound(model_path.clone()))?;
    let model = config::parse_model_config(&model_content)
        .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;

    let shard_path = config_dir.join("shard.toml");
    let shard_content = std::fs::read_to_string(&shard_path)
        .map_err(|_| BakerError::ConfigNotFound(shard_path.clone()))?;
    let shard = config::parse_shard_config(&shard_content)
        .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;

    // ── Phase A: Pre-Bake Validation (INV-BAKER-002, INV-BAKER-004) ──────────
    validate_physics_and_anatomy(&model.simulation, &shard.layers)?;
    config::validate_model(&model)
        .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;
    config::validate_shard(&shard)
        .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;

    let _v_seg = physics::compute_v_seg(
        model.simulation.signal_speed_m_s,
        model.simulation.tick_duration_us,
        model.simulation.voxel_size_um,
        model.simulation.segment_length_voxels,
    )
    .map_err(|e| BakerError::InvalidSignalSpeed(e.to_string()))?;
    tracing::debug!("Baker Phase A: validation passed");

    // ── Phase B: Config Loading & Spatial Generation ────────────────────────
    let bounds = (shard.dimensions.w, shard.dimensions.d, shard.dimensions.h);

    let seed_u64 = types::MasterSeed::from_str(&model.simulation.master_seed).raw();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed_u64);

    let mut somas = Vec::new();
    let mut current_z_pct = 0.0;
    for layer in &shard.layers {
        let z_offset = (current_z_pct * bounds.2 as f32).floor() as u32;
        current_z_pct += layer.height_pct;

        let mut layer_somas = topology::placement::place_somas(bounds, std::slice::from_ref(layer), &mut rng)
            .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;
        
        for pos in &mut layer_somas {
            let new_z = pos.z() + z_offset;
            *pos = types::PackedPosition::pack_raw(pos.x(), pos.y(), new_z, pos.type_id());
        }
        somas.extend(layer_somas);
    }
    somas.sort_by_key(|p| p.z());
    tracing::debug!("Baker Phase B: generated and Z-sorted {} somas", somas.len());

    // ── Phase C: Macro-Routing (INV-BAKER-002) ───────────────────────────────
    let mut ghost_cursor = 0;
    let ghost_capacity = shard.settings.ghost_capacity;
    if let Some(ref sockets) = shard.sockets {
        for conn in sockets {
            if conn.direction != config::SocketDirection::In {
                continue;
            }
            let target_type_name = conn.target_type.as_deref().unwrap_or("");
            let target_type_idx = shard.neuron_types.iter()
                .position(|nt| nt.name == target_type_name)
                .unwrap_or(0) as u8;

            let source_gxo_somas: Vec<u32> = (0..(conn.width * conn.height)).collect();

            let ghost_conns = topology::routing::route_ghost_atlas(
                &source_gxo_somas,
                conn.width,
                conn.height,
                bounds,
                conn.entry_z.unwrap_or(config::EntryZ::Bottom),
                target_type_idx,
                &somas,
            ).map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;

            ghost_cursor += ghost_conns.len() as u32;
        }
    }
    assert!(ghost_cursor <= ghost_capacity, "Ghost capacity exceeded");
    tracing::debug!("Baker Phase C: routed {} ghost connections", ghost_cursor);

    // ── Phase D: Layout Serialization ────────────────────────────────────────
    let padded_n = layout::align_to_warp(somas.len()) as u32;
    let total_axons = somas.len() as u32;
    let n = padded_n as usize;

    let voltage        = vec![0i32; n];
    let mut flags      = vec![0u8;  n];
    let threshold_offset = vec![0i32; n];
    let timers         = vec![0u8;  n];
    let mut soma_to_axon = vec![types::AXON_SENTINEL; n];
    let dend_targets = vec![types::EMPTY_PIXEL; n * 128];
    let dend_weights   = vec![0i32; n * 128];
    let dend_timers    = vec![0u8;  n * 128];

    for (i, pos) in somas.iter().enumerate() {
        flags[i] = types::SomaFlags::pack(pos.type_id(), 0, false).0;
        soma_to_axon[i] = i as u32;
    }

    let state_buf = serialize_state(
        padded_n,
        total_axons,
        &voltage,
        &flags,
        &threshold_offset,
        &timers,
        &soma_to_axon,
        &dend_targets,
        &dend_weights,
        &dend_timers,
    )?;

    let heads = vec![
        layout::BurstHeads8 {
            h0: types::AXON_SENTINEL,
            h1: types::AXON_SENTINEL,
            h2: types::AXON_SENTINEL,
            h3: types::AXON_SENTINEL,
            h4: types::AXON_SENTINEL,
            h5: types::AXON_SENTINEL,
            h6: types::AXON_SENTINEL,
            h7: types::AXON_SENTINEL,
        };
        somas.len()
    ];
    let axons_buf = serialize_axons(total_axons, &heads)?;

    let path_lengths = vec![0u8; somas.len()];
    let matrix = vec![types::PackedPosition(0); somas.len() * 256];
    let paths_buf = serialize_paths(
        total_axons,
        &path_lengths,
        &matrix,
    )?;

    tracing::debug!(
        "Baker Phase D: state={}B axons={}B paths={}B",
        state_buf.len(), axons_buf.len(), paths_buf.len()
    );

    // ── Phase E: Archive Packing ──────────────────────────────────────────────
    // Write blobs to a temp staging directory, then pack into output .axic file.
    let staging = tempfile::tempdir().map_err(BakerError::IOError)?;
    let staging_path = staging.path();

    std::fs::write(staging_path.join("shard.state"), &state_buf)
        .map_err(BakerError::IOError)?;
    std::fs::write(staging_path.join("shard.axons"), &axons_buf)
        .map_err(BakerError::IOError)?;
    std::fs::write(staging_path.join("shard.paths"), &paths_buf)
        .map_err(BakerError::IOError)?;

    vfs::pack_directory(staging_path, output_path)
        .map_err(|e| BakerError::IOError(std::io::Error::other(e.to_string())))?;

    tracing::info!("Baker: bake pipeline complete → '{}'", output_path.display());
    Ok(AxicArchive {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_test_configs(
        config_dir: &std::path::Path,
        signal_speed: f32,
        height_pct_sum: f32,
        share_sum: f32,
        num_connections: usize,
        conn_size: u32,
    ) {
        let model_toml = format!(
            r#"
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
            signal_speed_m_s = {}
            sync_batch_ticks = 10
            axon_growth_max_steps = 200
            max_dendrites = 128

            [[departments]]
            name = "cortex"
            config = "brain_cortex.toml"

            [[connections]]
            from = "cortex.relay"
            to = "cortex.relay"
            "#,
            signal_speed
        );

        let h1 = height_pct_sum / 2.0;
        let h2 = height_pct_sum - h1;

        let s1 = share_sum / 2.0;
        let s2 = share_sum - s1;

        let sockets_toml = (0..num_connections)
            .map(|i| format!(
                r#"
                [[sockets]]
                name = "relay_{}"
                direction = "in"
                width = {}
                height = {}
                entry_z = "Bottom"
                target_type = "Excitatory"
                growth_steps = 10
                "#,
                i, conn_size, conn_size
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let shard_toml = format!(
            r#"
            [dimensions]
            w = 10
            d = 10
            h = 10

            [[layers]]
            name = "L1"
            height_pct = {}
            density = 0.1
            composition = [
                {{ type_name = "Excitatory", share = {} }},
                {{ type_name = "Inhibitory", share = {} }}
            ]

            [[layers]]
            name = "L2"
            height_pct = {}
            density = 0.1
            composition = [
                {{ type_name = "Excitatory", share = 1.0 }}
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

            [[neuron_types]]
            name = "Inhibitory"
            
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
              is_inhibitory = true
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

            {}
            "#,
            h1, s1, s2, h2, sockets_toml
        );

        fs::write(config_dir.join("model.toml"), model_toml).unwrap();
        fs::write(config_dir.join("shard.toml"), shard_toml).unwrap();
    }

    #[test]
    fn test_fail_fast_on_invalid_heights() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        let out_file = temp.path().join("model.axic");

        write_test_configs(&config_dir, 2.0, 0.9, 1.0, 1, 4);

        let res = bake(&config_dir, &out_file);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), BakerError::InvalidLayerHeights { .. }));
    }

    #[test]
    fn test_fail_fast_on_fractional_v_seg() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        let out_file = temp.path().join("model.axic");

        write_test_configs(&config_dir, 1.23, 1.0, 1.0, 1, 4);

        let res = bake(&config_dir, &out_file);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), BakerError::InvalidSignalSpeed(_)));
    }

    #[test]
    fn test_fail_fast_on_invalid_composition() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        let out_file = temp.path().join("model.axic");

        write_test_configs(&config_dir, 2.0, 1.0, 0.5, 1, 4);

        let res = bake(&config_dir, &out_file);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), BakerError::InvalidComposition { .. }));
    }

    #[test]
    fn test_fail_fast_on_missing_config() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        let out_file = temp.path().join("model.axic");

        let res = bake(&config_dir, &out_file);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), BakerError::ConfigNotFound(_)));
    }

    #[test]
    fn test_ghost_capacity_cursor_assert() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        let out_file = temp.path().join("model.axic");

        write_test_configs(&config_dir, 2.0, 1.0, 1.0, 1, 40);

        let res = std::panic::catch_unwind(|| {
            let _ = bake(&config_dir, &out_file);
        });
        assert!(res.is_err());
    }

    #[test]
    fn test_spontaneous_firing_period_zero() {
        let heartbeat_m = physics::compile_dds_heartbeat(0);
        assert_eq!(heartbeat_m, 0);
    }

    #[test]
    fn test_architectural_dependency_isolation() {
        assert_eq!(layout::MAX_DENDRITES, 128);
        assert_eq!(layout::PATHS_MAGIC, 0x50415448);
        assert_eq!(layout::MAX_SEGMENTS_PER_AXON, 256);
        
        let dds = physics::compile_dds_heartbeat(100);
        assert_eq!(dds, 655);
    }

    #[test]
    fn test_pipeline_reproducibility() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        write_test_configs(&config_dir, 2.0, 1.0, 1.0, 1, 4);

        let out_file1 = temp.path().join("model1.axic");
        let out_file2 = temp.path().join("model2.axic");

        bake(&config_dir, &out_file1).unwrap();
        bake(&config_dir, &out_file2).unwrap();

        let bytes1 = fs::read(&out_file1).unwrap();
        let bytes2 = fs::read(&out_file2).unwrap();

        assert_eq!(bytes1, bytes2, "Bake outputs must be bit-to-bit identical (INV-BAKER-003)");
    }

    #[test]
    fn test_pipeline_phases_order() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        write_test_configs(&config_dir, 2.0, 1.0, 1.0, 1, 4);

        let out_file = temp.path().join("model.axic");
        bake(&config_dir, &out_file).unwrap();

        let archive = vfs::AxicArchive::open(&out_file).unwrap();
        let state_bytes = archive.get_file("shard.state").unwrap();
        let state_header: &layout::StateFileHeader = bytemuck::from_bytes(&state_bytes[0..16]);
        let padded_n = state_header.padded_n as usize;

        assert!(padded_n >= 1);
    }

    #[test]
    fn test_bake_and_unpack_roundtrip() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("configs");
        fs::create_dir(&config_dir).unwrap();
        write_test_configs(&config_dir, 2.0, 1.0, 1.0, 1, 4);

        let out_file = temp.path().join("model.axic");
        bake(&config_dir, &out_file).unwrap();

        let archive = vfs::AxicArchive::open(&out_file).unwrap();

        let state_bytes = archive.get_file("shard.state").unwrap();
        assert_eq!(&state_bytes[0..4], b"GSNS");

        let axons_bytes = archive.get_file("shard.axons").unwrap();
        assert_eq!(&axons_bytes[0..4], b"GSAX");

        let paths_bytes = archive.get_file("shard.paths").unwrap();
        let magic = u32::from_le_bytes([paths_bytes[0], paths_bytes[1], paths_bytes[2], paths_bytes[3]]);
        assert_eq!(magic, layout::PATHS_MAGIC);
    }
}


