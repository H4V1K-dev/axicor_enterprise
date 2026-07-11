#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]
#![allow(clippy::needless_range_loop, dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use types::MasterSeed;

#[derive(Debug, Clone, serde::Serialize)]
struct MultifieldSegment {
    x: f32,
    y: f32,
    z: f32,
    segment_offset: u8,
    branch_id: u32,
}

#[derive(Debug, Clone)]
struct MultifieldAxonPath {
    soma_id: u32,
    axon_type_id: u8,
    branches: Vec<Vec<MultifieldSegment>>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Synapse {
    source_soma_id: u32,
    target_soma_id: u32,
    branch_id: u32,
    segment_offset: u8,
    distance_sq: f32,
    dendrite_idx: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FlatSynapse {
    source_soma_id: u32,
    flat_segment_idx: u32,
    target_soma_id: u32,
    dendrite_idx: u32,
    weight: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FlatAxon {
    soma_id: u32,
    total_segments: usize,
    parents: Vec<Option<usize>>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct ParityEvent {
    tick: usize,
    source_soma_id: u32,
    flat_segment_idx: u32,
    target_soma_id: u32,
    dendrite_idx: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrowthState {
    Pathfinding,
    TractFollowing,
    TargetZoneCapture,
    TerminalArborization,
    Terminated,
}

#[derive(Debug, Clone)]
struct RunConfig {
    name: String,
    max_branches: usize,
    max_branch_len: usize,
    w_fascicle: f32,
    r_fascicle: f32,
    r_repulsion: f32,
    override_dendrite_radius: Option<f32>,
}

fn deterministic_rng(seed: u64, soma_id: u32, step: usize) -> u64 {
    let mut hash_val: u64 = 0xcbf2_9ce4_8422_2325;
    hash_val = (hash_val ^ seed).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (soma_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (step as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val
}

fn deterministic_noise(seed: u64, soma_id: u32, step: usize) -> glam::Vec3 {
    let rng_val = deterministic_rng(seed, soma_id, step);
    let rx = ((rng_val & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
    let ry = (((rng_val >> 8) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
    let rz = (((rng_val >> 16) & 0xFF) as f32 / 255.0) * 2.0 - 1.0;
    glam::Vec3::new(rx, ry, rz).normalize_or_zero()
}

fn offset_direction(v: glam::IVec3) -> (f32, f32, f32) {
    let len = (v.x * v.x + v.y * v.y + v.z * v.z) as f32;
    if len > 0.001 {
        let l = len.sqrt();
        (v.x as f32 / l, v.y as f32 / l, v.z as f32 / l)
    } else {
        (0.0, 0.0, 0.0)
    }
}

fn find_profile_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // to crates
    path.pop(); // to AxiEngine
    path.pop(); // to workflow
    path.push("Axicor_Neuron-Lib");
    path.push("modernized");
    path.push(format!("{}.toml", name));
    path
}

fn load_neuron_type(name: &str) -> config::NeuronType {
    let path = find_profile_path(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Could not read {}: {:?}", path.display(), e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse TOML from {}: {:?}", path.display(), e))
}

fn build_shard_config() -> config::ShardConfig {
    let nt_l4_real = load_neuron_type("L4_spiny_VISl4_4");
    let nt_l23_real = load_neuron_type("L23_aspiny_VISp23_218");
    let nt_l5_real = load_neuron_type("L5_spiny_VISp5_7");

    let mut nt_virtual = nt_l4_real.clone();
    nt_virtual.name = "VirtualInput".to_string();
    nt_virtual.growth.dendrite_whitelist = vec!["NoDendriteSource".to_string()];
    nt_virtual.growth.growth_vertical_bias = 2.0;
    nt_virtual.growth.dendrite_radius_um = 10.0;

    let mut nt_no_dendrite_source = nt_l4_real.clone();
    nt_no_dendrite_source.name = "NoDendriteSource".to_string();

    let mut nt_l4 = nt_l4_real.clone();
    nt_l4.name = "L4_spiny".to_string();
    nt_l4.growth.dendrite_whitelist = vec!["VirtualInput".to_string(), "L23_aspiny".to_string()];
    nt_l4.growth.growth_vertical_bias = 1.0;
    nt_l4.growth.dendrite_radius_um = 12.0;

    let mut nt_l23 = nt_l23_real.clone();
    nt_l23.name = "L23_aspiny".to_string();
    nt_l23.growth.dendrite_whitelist = vec![
        "L4_spiny".to_string(),
        "L5_spiny".to_string(),
        "L23_aspiny".to_string(),
    ];
    nt_l23.growth.growth_vertical_bias = 0.0;
    nt_l23.growth.dendrite_radius_um = 10.0;

    let mut nt_l5 = nt_l5_real.clone();
    nt_l5.name = "L5_spiny".to_string();
    nt_l5.growth.dendrite_whitelist = vec!["L4_spiny".to_string(), "L23_aspiny".to_string()];
    nt_l5.growth.growth_vertical_bias = -1.5;
    nt_l5.growth.dendrite_radius_um = 10.0;

    let layers = vec![
        config::LayerConfig {
            name: "Virtual".to_string(),
            height_pct: 0.25,
            density: 0.0625,
            composition: vec![config::NeuronTypeDistribution {
                type_name: "VirtualInput".to_string(),
                share: 1.0,
            }],
        },
        config::LayerConfig {
            name: "L4".to_string(),
            height_pct: 0.25,
            density: 0.0625,
            composition: vec![config::NeuronTypeDistribution {
                type_name: "L4_spiny".to_string(),
                share: 1.0,
            }],
        },
        config::LayerConfig {
            name: "L23".to_string(),
            height_pct: 0.25,
            density: 0.03125,
            composition: vec![config::NeuronTypeDistribution {
                type_name: "L23_aspiny".to_string(),
                share: 1.0,
            }],
        },
        config::LayerConfig {
            name: "L5".to_string(),
            height_pct: 0.25,
            density: 0.03125,
            composition: vec![config::NeuronTypeDistribution {
                type_name: "L5_spiny".to_string(),
                share: 1.0,
            }],
        },
    ];

    config::ShardConfig {
        meta: None,
        dimensions: config::ShardDimensions {
            w: 16,
            d: 16,
            h: 32,
        },
        settings: config::ShardSettings {
            ghost_capacity: 1024,
            prune_threshold: 0,
            max_sprouts: 8,
            night_interval_ticks: 100,
            save_checkpoints_interval_ticks: 1000,
        },
        layers,
        neuron_types: vec![nt_virtual, nt_l4, nt_l23, nt_l5, nt_no_dendrite_source],
        sockets: None,
        ports: None,
    }
}

fn run_multifield_simulation(
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
    seed: u64,
    run_cfg: &RunConfig,
) -> (Vec<MultifieldAxonPath>, Vec<Synapse>) {
    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;
    let max_steps = 15;
    let step_size_um = 1.0;

    let target_zone_radius_um = 5.0;
    let capture_radius_um = 1.8;
    let damping_radius_um = 5.0;

    let soma_core_radius = 0.5;
    let repulsion_radius = run_cfg.r_repulsion;

    let mut completed_axons: Vec<MultifieldAxonPath> = Vec::new();
    let mut neighbors = Vec::new();
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                neighbors.push(glam::IVec3::new(dx, dy, dz));
            }
        }
    }

    for soma in &topo.somas {
        let variant_idx = soma.variant_id as usize;
        let source_type = &config.neuron_types[variant_idx];
        let vertical_bias = source_type.growth.growth_vertical_bias;

        let sx = soma.position.x() as u32;
        let sy = soma.position.y() as u32;
        let sz = soma.position.z() as u32;

        let mut curr_pos_um = glam::Vec3::new(sx as f32, sy as f32, sz as f32);
        let mut forward_dir = if vertical_bias != 0.0 {
            glam::Vec3::new(0.0, 0.0, vertical_bias.signum())
        } else {
            glam::Vec3::new(1.0, 0.0, 0.0)
        };

        let mut state = GrowthState::Pathfinding;
        let mut visited = HashSet::new();
        visited.insert((sx, sy, sz));

        let target_somas: Vec<&topology::PlacedSoma> = topo
            .somas
            .iter()
            .filter(|&target| {
                if target.soma_id == soma.soma_id {
                    false
                } else {
                    let target_type = &config.neuron_types[target.variant_id as usize];
                    target_type
                        .growth
                        .dendrite_whitelist
                        .iter()
                        .any(|name| name == &source_type.name)
                }
            })
            .collect();

        let mut main_branch = Vec::new();

        for step in 1..=max_steps {
            if state == GrowthState::Terminated || state == GrowthState::TerminalArborization {
                break;
            }

            let target_z = if source_type.name == "VirtualInput" {
                11.5
            } else if source_type.name == "L4_spiny" {
                23.5
            } else {
                19.5
            };

            let mut nearest_target_dist = f32::MAX;
            let mut nearest_soma = None;
            for &target in &target_somas {
                let target_pos = glam::Vec3::new(
                    target.position.x() as f32,
                    target.position.y() as f32,
                    target.position.z() as f32,
                );
                let d = curr_pos_um.distance(target_pos);
                if d < nearest_target_dist {
                    nearest_target_dist = d;
                    nearest_soma = Some(target);
                }
            }

            let mut v_fascicle = glam::Vec3::ZERO;
            let mut found_compatible_segment = false;
            for other_axon in &completed_axons {
                if other_axon.axon_type_id == soma.variant_id {
                    for b in &other_axon.branches {
                        for seg in b {
                            let seg_pos = glam::Vec3::new(seg.x, seg.y, seg.z);
                            let d = curr_pos_um.distance(seg_pos);
                            if d <= run_cfg.r_fascicle && d > 0.01 {
                                let dir = if seg.segment_offset > 1 {
                                    let prev_seg = &b[seg.segment_offset as usize - 2];
                                    glam::Vec3::new(
                                        seg.x - prev_seg.x,
                                        seg.y - prev_seg.y,
                                        seg.z - prev_seg.z,
                                    )
                                    .normalize_or_zero()
                                } else {
                                    glam::Vec3::Z
                                };
                                v_fascicle +=
                                    (seg_pos - curr_pos_um).normalize_or_zero() * 0.4 + dir * 0.6;
                                found_compatible_segment = true;
                            }
                        }
                    }
                }
            }
            if v_fascicle.length_squared() > 0.01 {
                v_fascicle = v_fascicle.normalize();
            }

            if nearest_target_dist <= capture_radius_um {
                state = GrowthState::TerminalArborization;
            } else if nearest_target_dist <= target_zone_radius_um {
                state = GrowthState::TargetZoneCapture;
            } else if found_compatible_segment {
                state = GrowthState::TractFollowing;
            } else {
                state = GrowthState::Pathfinding;
            }

            if state == GrowthState::TerminalArborization {
                break;
            }

            let (w_persist, w_layer, w_fascicle_wt, w_local, w_repulse, w_noise) = match state {
                GrowthState::Pathfinding => (0.3, 0.5, 0.0, 0.0, 0.8, 0.1),
                GrowthState::TractFollowing => (0.3, 0.2, run_cfg.w_fascicle, 0.0, 0.8, 0.1),
                GrowthState::TargetZoneCapture => {
                    let damped_local = if nearest_target_dist < damping_radius_um {
                        0.5 * (nearest_target_dist / damping_radius_um)
                    } else {
                        0.5
                    };
                    (0.2, 0.0, 0.0, damped_local, 0.8, 0.1)
                }
                _ => (0.3, 0.5, 0.0, 0.0, 0.8, 0.1),
            };

            let v_persist = forward_dir;
            let v_layer = glam::Vec3::new(0.0, 0.0, target_z - curr_pos_um.z).normalize_or_zero();
            let v_local = if let Some(target) = nearest_soma {
                let target_pos = glam::Vec3::new(
                    target.position.x() as f32,
                    target.position.y() as f32,
                    target.position.z() as f32,
                );
                (target_pos - curr_pos_um).normalize_or_zero()
            } else {
                glam::Vec3::ZERO
            };

            let mut v_repulse = glam::Vec3::ZERO;
            for n_soma in &topo.somas {
                let soma_pos = glam::Vec3::new(
                    n_soma.position.x() as f32,
                    n_soma.position.y() as f32,
                    n_soma.position.z() as f32,
                );
                let d = curr_pos_um.distance(soma_pos);
                if d <= repulsion_radius && d > 0.01 {
                    let repel_mag = (repulsion_radius - d) / (repulsion_radius - soma_core_radius);
                    v_repulse += (curr_pos_um - soma_pos).normalize_or_zero() * repel_mag;
                }
            }
            if v_repulse.length_squared() > 0.01 {
                v_repulse = v_repulse.normalize();
            }

            let v_noise = deterministic_noise(seed, soma.soma_id, step);

            let mut v_final = v_persist * w_persist
                + v_layer * w_layer
                + v_fascicle * w_fascicle_wt
                + v_local * w_local
                + v_repulse * w_repulse
                + v_noise * w_noise;

            if v_final.length_squared() < 0.001 {
                v_final = v_persist;
            } else {
                v_final = v_final.normalize();
            }

            let next_pos_um = curr_pos_um + v_final * step_size_um;
            let nx = next_pos_um.x.round() as i32;
            let ny = next_pos_um.y.round() as i32;
            let nz = next_pos_um.z.round() as i32;

            let mut step_taken = false;
            let mut unx = 0;
            let mut uny = 0;
            let mut unz = 0;

            if nx >= 0
                && nx < shard_w as i32
                && ny >= 0
                && ny < shard_d as i32
                && nz >= 0
                && nz < shard_h as i32
            {
                let check_unx = nx as u32;
                let check_uny = ny as u32;
                let check_unz = nz as u32;

                let mut collided = false;
                for n_soma in &topo.somas {
                    if n_soma.soma_id == soma.soma_id {
                        continue;
                    }
                    let soma_pos = glam::Vec3::new(
                        n_soma.position.x() as f32,
                        n_soma.position.y() as f32,
                        n_soma.position.z() as f32,
                    );
                    if next_pos_um.distance(soma_pos) <= soma_core_radius {
                        collided = true;
                        break;
                    }
                }

                let no_self_intersection = !visited.contains(&(check_unx, check_uny, check_unz));

                if !collided && no_self_intersection {
                    unx = check_unx;
                    uny = check_uny;
                    unz = check_unz;
                    curr_pos_um = next_pos_um;
                    forward_dir = v_final;
                    step_taken = true;
                }
            }

            if !step_taken {
                let curr_vox = glam::IVec3::new(
                    curr_pos_um.x.round() as i32,
                    curr_pos_um.y.round() as i32,
                    curr_pos_um.z.round() as i32,
                );

                let mut best_neighbor = None;
                let mut best_score = -f32::MAX;

                for &offset in &neighbors {
                    let test_vox = curr_vox + offset;
                    if test_vox.x >= 0
                        && test_vox.x < shard_w as i32
                        && test_vox.y >= 0
                        && test_vox.y < shard_d as i32
                        && test_vox.z >= 0
                        && test_vox.z < shard_h as i32
                    {
                        let test_unx = test_vox.x as u32;
                        let test_uny = test_vox.y as u32;
                        let test_unz = test_vox.z as u32;

                        if visited.contains(&(test_unx, test_uny, test_unz)) {
                            continue;
                        }

                        let mut collided = false;
                        let test_pos_um =
                            glam::Vec3::new(test_unx as f32, test_uny as f32, test_unz as f32);
                        for n_soma in &topo.somas {
                            if n_soma.soma_id == soma.soma_id {
                                continue;
                            }
                            let soma_pos = glam::Vec3::new(
                                n_soma.position.x() as f32,
                                n_soma.position.y() as f32,
                                n_soma.position.z() as f32,
                            );
                            if test_pos_um.distance(soma_pos) <= soma_core_radius {
                                collided = true;
                                break;
                            }
                        }

                        if !collided {
                            let score = forward_dir.dot(
                                glam::Vec3::new(offset.x as f32, offset.y as f32, offset.z as f32)
                                    .normalize_or_zero(),
                            );
                            if score > best_score {
                                best_score = score;
                                best_neighbor = Some(test_vox);
                            }
                        }
                    }
                }

                if let Some(chosen_vox) = best_neighbor {
                    unx = chosen_vox.x as u32;
                    uny = chosen_vox.y as u32;
                    unz = chosen_vox.z as u32;
                    curr_pos_um = glam::Vec3::new(unx as f32, uny as f32, unz as f32);
                    let offset = chosen_vox - curr_vox;
                    forward_dir = glam::Vec3::new(
                        offset_direction(offset).0,
                        offset_direction(offset).1,
                        offset_direction(offset).2,
                    );
                    step_taken = true;
                }
            }

            if !step_taken {
                break;
            }

            main_branch.push(MultifieldSegment {
                x: curr_pos_um.x,
                y: curr_pos_um.y,
                z: curr_pos_um.z,
                segment_offset: step as u8,
                branch_id: 0,
            });
            visited.insert((unx, uny, unz));
        }

        let mut branches = vec![main_branch];

        if state == GrowthState::TerminalArborization && run_cfg.max_branches > 0 {
            let branch_origin = curr_pos_um;
            let rng_val = deterministic_rng(seed, soma.soma_id, 100);
            let num_branches = (rng_val % run_cfg.max_branches as u64) + 1;

            for b in 1..=num_branches {
                let mut terminal_branch = Vec::new();
                let mut b_pos_um = branch_origin;
                let mut b_dir = forward_dir;
                let branch_len = (deterministic_rng(seed, soma.soma_id, 200 + b as usize)
                    % run_cfg.max_branch_len as u64)
                    + 1;

                for b_step in 1..=branch_len {
                    let w_persist = 0.1;
                    let w_local = 0.4;
                    let w_repulse = 0.9;
                    let w_noise = 0.5;

                    let mut nearest_target_dist = f32::MAX;
                    let mut nearest_soma = None;
                    for &target in &target_somas {
                        let target_pos = glam::Vec3::new(
                            target.position.x() as f32,
                            target.position.y() as f32,
                            target.position.z() as f32,
                        );
                        let d = b_pos_um.distance(target_pos);
                        if d < nearest_target_dist {
                            nearest_target_dist = d;
                            nearest_soma = Some(target);
                        }
                    }

                    let v_persist = b_dir;
                    let v_local = if let Some(target) = nearest_soma {
                        let target_pos = glam::Vec3::new(
                            target.position.x() as f32,
                            target.position.y() as f32,
                            target.position.z() as f32,
                        );
                        (target_pos - b_pos_um).normalize_or_zero()
                    } else {
                        glam::Vec3::ZERO
                    };

                    let mut v_repulse = glam::Vec3::ZERO;
                    for n_soma in &topo.somas {
                        let soma_pos = glam::Vec3::new(
                            n_soma.position.x() as f32,
                            n_soma.position.y() as f32,
                            n_soma.position.z() as f32,
                        );
                        let d = b_pos_um.distance(soma_pos);
                        if d <= repulsion_radius && d > 0.01 {
                            let repel_mag =
                                (repulsion_radius - d) / (repulsion_radius - soma_core_radius);
                            v_repulse += (b_pos_um - soma_pos).normalize_or_zero() * repel_mag;
                        }
                    }
                    if v_repulse.length_squared() > 0.01 {
                        v_repulse = v_repulse.normalize();
                    }

                    let v_noise = deterministic_noise(
                        seed,
                        soma.soma_id,
                        300 + b as usize + b_step as usize * 10,
                    );

                    let mut v_final = v_persist * w_persist
                        + v_local * w_local
                        + v_repulse * w_repulse
                        + v_noise * w_noise;

                    if v_final.length_squared() < 0.001 {
                        v_final = v_persist;
                    } else {
                        v_final = v_final.normalize();
                    }

                    let next_pos_um = b_pos_um + v_final * step_size_um;
                    let nx = next_pos_um.x.round() as i32;
                    let ny = next_pos_um.y.round() as i32;
                    let nz = next_pos_um.z.round() as i32;

                    let mut b_step_taken = false;
                    let mut bunx = 0;
                    let mut buny = 0;
                    let mut bunz = 0;

                    if nx >= 0
                        && nx < shard_w as i32
                        && ny >= 0
                        && ny < shard_d as i32
                        && nz >= 0
                        && nz < shard_h as i32
                    {
                        let check_unx = nx as u32;
                        let check_uny = ny as u32;
                        let check_unz = nz as u32;

                        let mut collided = false;
                        for n_soma in &topo.somas {
                            if n_soma.soma_id == soma.soma_id {
                                continue;
                            }
                            let soma_pos = glam::Vec3::new(
                                n_soma.position.x() as f32,
                                n_soma.position.y() as f32,
                                n_soma.position.z() as f32,
                            );
                            if next_pos_um.distance(soma_pos) <= soma_core_radius {
                                collided = true;
                                break;
                            }
                        }

                        let no_self_intersection =
                            !visited.contains(&(check_unx, check_uny, check_unz));

                        if !collided && no_self_intersection {
                            bunx = check_unx;
                            buny = check_uny;
                            bunz = check_unz;
                            b_pos_um = next_pos_um;
                            b_dir = v_final;
                            b_step_taken = true;
                        }
                    }

                    if !b_step_taken {
                        let curr_vox = glam::IVec3::new(
                            b_pos_um.x.round() as i32,
                            b_pos_um.y.round() as i32,
                            b_pos_um.z.round() as i32,
                        );

                        let mut best_neighbor = None;
                        let mut best_score = -f32::MAX;

                        for &offset in &neighbors {
                            let test_vox = curr_vox + offset;
                            if test_vox.x >= 0
                                && test_vox.x < shard_w as i32
                                && test_vox.y >= 0
                                && test_vox.y < shard_d as i32
                                && test_vox.z >= 0
                                && test_vox.z < shard_h as i32
                            {
                                let test_unx = test_vox.x as u32;
                                let test_uny = test_vox.y as u32;
                                let test_unz = test_vox.z as u32;

                                if visited.contains(&(test_unx, test_uny, test_unz)) {
                                    continue;
                                }

                                let mut collided = false;
                                let test_pos_um = glam::Vec3::new(
                                    test_unx as f32,
                                    test_uny as f32,
                                    test_unz as f32,
                                );
                                for n_soma in &topo.somas {
                                    if n_soma.soma_id == soma.soma_id {
                                        continue;
                                    }
                                    let soma_pos = glam::Vec3::new(
                                        n_soma.position.x() as f32,
                                        n_soma.position.y() as f32,
                                        n_soma.position.z() as f32,
                                    );
                                    if test_pos_um.distance(soma_pos) <= soma_core_radius {
                                        collided = true;
                                        break;
                                    }
                                }

                                if !collided {
                                    let score = b_dir.dot(
                                        glam::Vec3::new(
                                            offset.x as f32,
                                            offset.y as f32,
                                            offset.z as f32,
                                        )
                                        .normalize_or_zero(),
                                    );
                                    if score > best_score {
                                        best_score = score;
                                        best_neighbor = Some(test_vox);
                                    }
                                }
                            }
                        }

                        if let Some(chosen_vox) = best_neighbor {
                            bunx = chosen_vox.x as u32;
                            buny = chosen_vox.y as u32;
                            bunz = chosen_vox.z as u32;
                            b_pos_um = glam::Vec3::new(bunx as f32, buny as f32, bunz as f32);
                            let offset = chosen_vox - curr_vox;
                            b_dir = glam::Vec3::new(
                                offset_direction(offset).0,
                                offset_direction(offset).1,
                                offset_direction(offset).2,
                            );
                            b_step_taken = true;
                        }
                    }

                    if !b_step_taken {
                        break;
                    }

                    terminal_branch.push(MultifieldSegment {
                        x: b_pos_um.x,
                        y: b_pos_um.y,
                        z: b_pos_um.z,
                        segment_offset: b_step as u8,
                        branch_id: b as u32,
                    });
                    visited.insert((bunx, buny, bunz));
                }

                if !terminal_branch.is_empty() {
                    branches.push(terminal_branch);
                }
            }
        }

        completed_axons.push(MultifieldAxonPath {
            soma_id: soma.soma_id,
            axon_type_id: soma.variant_id,
            branches,
        });
    }

    // Touch candidate formation
    let mut target_candidates = vec![Vec::new(); topo.somas.len()];

    for axon in &completed_axons {
        let source_id = axon.soma_id;
        let source_type = &config.neuron_types[axon.axon_type_id as usize];

        for target in &topo.somas {
            if target.soma_id == source_id {
                continue;
            }
            let target_type = &config.neuron_types[target.variant_id as usize];
            if !target_type
                .growth
                .dendrite_whitelist
                .iter()
                .any(|name| name == &source_type.name)
            {
                continue;
            }

            let target_pos = glam::Vec3::new(
                target.position.x() as f32,
                target.position.y() as f32,
                target.position.z() as f32,
            );

            for b in &axon.branches {
                for seg in b {
                    let seg_pos = glam::Vec3::new(seg.x, seg.y, seg.z);
                    let dist_sq = seg_pos.distance_squared(target_pos);
                    let radius = if let Some(r) = run_cfg.override_dendrite_radius {
                        r
                    } else {
                        target_type.growth.dendrite_radius_um
                    };

                    if dist_sq <= radius * radius {
                        target_candidates[target.soma_id as usize].push(Synapse {
                            source_soma_id: source_id,
                            target_soma_id: target.soma_id,
                            branch_id: seg.branch_id,
                            segment_offset: seg.segment_offset,
                            distance_sq: dist_sq,
                            dendrite_idx: 0,
                        });
                    }
                }
            }
        }
    }

    // Softmax cap per pair pruning
    let mut accepted_synapses = Vec::new();
    let cap_limit = 128; // MAX_DENDRITES

    for target_idx in 0..target_candidates.len() {
        let candidates = &mut target_candidates[target_idx];
        if candidates.is_empty() {
            continue;
        }

        candidates.sort_by(|a, b| {
            a.distance_sq
                .partial_cmp(&b.distance_sq)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut groups: HashMap<u32, Vec<Synapse>> = HashMap::new();
        for cand in candidates.iter() {
            groups
                .entry(cand.source_soma_id)
                .or_default()
                .push(cand.clone());
        }

        let mut selected = Vec::new();
        for (source_id, group_cands) in groups {
            // Cap at 2 per pair
            let max_per_pair = 2;
            if group_cands.len() <= max_per_pair {
                selected.extend(group_cands);
            } else {
                let mut chosen = Vec::new();
                let mut cands_temp = group_cands.clone();
                let mut rng_seed = deterministic_rng(seed, source_id, target_idx);

                while chosen.len() < max_per_pair && !cands_temp.is_empty() {
                    let mut min_d_sq = f32::MAX;
                    for c in &cands_temp {
                        if c.distance_sq < min_d_sq {
                            min_d_sq = c.distance_sq;
                        }
                    }

                    let beta = 2.0;
                    let mut weights = Vec::new();
                    let mut sum_w = 0.0;
                    for c in &cands_temp {
                        let diff = c.distance_sq - min_d_sq;
                        let w = (-beta * diff.max(0.0)).exp();
                        weights.push(w);
                        sum_w += w;
                    }

                    if sum_w <= 0.0 {
                        chosen.push(cands_temp.remove(0));
                        continue;
                    }

                    let rng_val = (rng_seed & 0xFFFFFFFF) as f32 / 4294967295.0;
                    rng_seed = deterministic_rng(rng_seed, source_id, chosen.len() + 10);
                    let target_val = rng_val * sum_w;
                    let mut acc_w = 0.0;
                    let mut idx_to_remove = 0;
                    for (i, &w) in weights.iter().enumerate() {
                        acc_w += w;
                        if acc_w >= target_val {
                            idx_to_remove = i;
                            break;
                        }
                    }
                    chosen.push(cands_temp.remove(idx_to_remove));
                }
                selected.extend(chosen);
            }
        }

        // Apply target MAX_DENDRITES capacity
        selected.sort_by(|a, b| {
            a.distance_sq
                .partial_cmp(&b.distance_sq)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut dendrite_idx = 0;
        for mut syn in selected {
            if dendrite_idx < cap_limit {
                syn.dendrite_idx = dendrite_idx as u32;
                accepted_synapses.push(syn);
                dendrite_idx += 1;
            }
        }
    }

    (completed_axons, accepted_synapses)
}

fn build_flat_tuples(
    axons: &[MultifieldAxonPath],
    synapses: &[Synapse],
    _topo: &topology::SingleShardTopology,
) -> (Vec<FlatSynapse>, Vec<FlatAxon>) {
    let mut flat_synapses = Vec::new();
    let mut flat_axons = Vec::new();

    for axon in axons {
        let mut total_segments = 0;
        let mut parents = Vec::new();

        let main_len = axon.branches[0].len();
        total_segments += main_len;

        for i in 0..main_len {
            if i == 0 {
                parents.push(None);
            } else {
                parents.push(Some(i - 1));
            }
        }

        for b_idx in 1..axon.branches.len() {
            let branch_len = axon.branches[b_idx].len();
            let b_start_idx = total_segments;
            total_segments += branch_len;

            for i in 0..branch_len {
                if i == 0 {
                    if main_len > 0 {
                        parents.push(Some(main_len - 1));
                    } else {
                        parents.push(None);
                    }
                } else {
                    parents.push(Some(b_start_idx + i - 1));
                }
            }
        }

        flat_axons.push(FlatAxon {
            soma_id: axon.soma_id,
            total_segments,
            parents,
        });
    }

    for syn in synapses {
        let axon = &axons[syn.source_soma_id as usize];
        let flat_segment_idx = flat_segment_idx_for(axon, syn.branch_id, syn.segment_offset);

        // Weight is distance-softmax-like or constant 1.0
        let weight = 1.0;

        flat_synapses.push(FlatSynapse {
            source_soma_id: syn.source_soma_id,
            flat_segment_idx,
            target_soma_id: syn.target_soma_id,
            dendrite_idx: syn.dendrite_idx,
            weight,
        });
    }

    (flat_synapses, flat_axons)
}

fn flat_segment_idx_for(axon: &MultifieldAxonPath, branch_id: u32, segment_offset: u8) -> u32 {
    let branch_idx = branch_id as usize;
    let seg_offset = segment_offset as usize;
    debug_assert!(branch_idx < axon.branches.len());
    debug_assert!(seg_offset > 0 && seg_offset <= axon.branches[branch_idx].len());

    let mut flat_segment_idx = 0;
    for b_i in 0..branch_idx {
        flat_segment_idx += axon.branches[b_i].len();
    }
    (flat_segment_idx + seg_offset - 1) as u32
}

#[derive(Debug, Clone, Copy)]
struct AP {
    source_soma_id: u32,
    branch_id: u32,
    segment_offset: usize,
}

fn simulate_aot_oracle(
    axons: &[MultifieldAxonPath],
    synapses: &[Synapse],
    spikes: &HashMap<u32, Vec<usize>>,
    stimulated_somas: &[u32],
    max_ticks: usize,
) -> Vec<ParityEvent> {
    let mut active_aps: Vec<AP> = Vec::new();
    let mut events = Vec::new();

    for tick in 0..max_ticks {
        let mut next_aps = Vec::new();

        // 1. Trigger APs for somas spiking at this tick
        for &soma_id in stimulated_somas {
            if let Some(spike_times) = spikes.get(&soma_id) {
                if spike_times.contains(&tick) {
                    let axon = &axons[soma_id as usize];
                    if axon.branches[0].is_empty() {
                        for b_idx in 1..axon.branches.len() {
                            next_aps.push(AP {
                                source_soma_id: soma_id,
                                branch_id: b_idx as u32,
                                segment_offset: 1,
                            });
                        }
                    } else {
                        next_aps.push(AP {
                            source_soma_id: soma_id,
                            branch_id: 0,
                            segment_offset: 1,
                        });
                    }
                }
            }
        }

        // 2. Process active APs at the current tick
        for ap in active_aps {
            let axon = &axons[ap.source_soma_id as usize];
            let branch_len = axon.branches[ap.branch_id as usize].len();

            // Trigger any synapses formed on this segment
            for syn in synapses {
                if syn.source_soma_id == ap.source_soma_id
                    && syn.branch_id == ap.branch_id
                    && syn.segment_offset as usize == ap.segment_offset
                {
                    events.push(ParityEvent {
                        tick,
                        source_soma_id: syn.source_soma_id,
                        flat_segment_idx: flat_segment_idx_for(
                            axon,
                            syn.branch_id,
                            syn.segment_offset,
                        ),
                        target_soma_id: syn.target_soma_id,
                        dendrite_idx: syn.dendrite_idx,
                    });
                }
            }

            // 3. Move AP along axon branches
            if ap.branch_id == 0 {
                if ap.segment_offset < branch_len {
                    next_aps.push(AP {
                        source_soma_id: ap.source_soma_id,
                        branch_id: 0,
                        segment_offset: ap.segment_offset + 1,
                    });
                } else {
                    // Split into terminal branches
                    for b_idx in 1..axon.branches.len() {
                        next_aps.push(AP {
                            source_soma_id: ap.source_soma_id,
                            branch_id: b_idx as u32,
                            segment_offset: 1,
                        });
                    }
                }
            } else {
                if ap.segment_offset < branch_len {
                    next_aps.push(AP {
                        source_soma_id: ap.source_soma_id,
                        branch_id: ap.branch_id,
                        segment_offset: ap.segment_offset + 1,
                    });
                }
            }
        }

        active_aps = next_aps;
    }

    events
}

fn simulate_flat_runtime(
    flat_axons: &[FlatAxon],
    flat_synapses: &[FlatSynapse],
    spikes: &HashMap<u32, Vec<usize>>,
    _stimulated_somas: &[u32],
    max_ticks: usize,
) -> Vec<ParityEvent> {
    let mut active_segments = HashMap::new();
    for axon in flat_axons {
        active_segments.insert(axon.soma_id, vec![false; axon.total_segments]);
    }

    let mut events = Vec::new();

    for tick in 0..max_ticks {
        let mut next_active = HashMap::new();
        for axon in flat_axons {
            next_active.insert(axon.soma_id, vec![false; axon.total_segments]);
        }

        // 1. Soma spikes at this tick activate root segments (parent == None) at tick + 1
        for axon in flat_axons {
            if let Some(spike_times) = spikes.get(&axon.soma_id) {
                if spike_times.contains(&tick) {
                    let act = next_active.get_mut(&axon.soma_id).unwrap();
                    for (seg_idx, parent) in axon.parents.iter().enumerate() {
                        if parent.is_none() {
                            act[seg_idx] = true;
                        }
                    }
                }
            }
        }

        // 2. Propagate activity from previously active segments using parent pointer arrays
        for axon in flat_axons {
            let prev_active = &active_segments[&axon.soma_id];
            let curr_next_active = next_active.get_mut(&axon.soma_id).unwrap();

            for i in 0..axon.total_segments {
                if prev_active[i] {
                    for j in 0..axon.total_segments {
                        if axon.parents[j] == Some(i) {
                            curr_next_active[j] = true;
                        }
                    }
                }
            }
        }

        // 3. Trigger events for active segments at this tick
        for axon in flat_axons {
            let active = &active_segments[&axon.soma_id];
            for syn in flat_synapses {
                if syn.source_soma_id == axon.soma_id {
                    let idx = syn.flat_segment_idx as usize;
                    if idx < active.len() && active[idx] {
                        events.push(ParityEvent {
                            tick,
                            source_soma_id: syn.source_soma_id,
                            flat_segment_idx: syn.flat_segment_idx,
                            target_soma_id: syn.target_soma_id,
                            dendrite_idx: syn.dendrite_idx,
                        });
                    }
                }
            }
        }

        active_segments = next_active;
    }

    events
}

#[test]
fn run_growth_v2_flat_parity() {
    println!("=== Starting Growth v2 AOT-to-Flat Parity v0.4 ===");

    let seed_val = 12345;
    let master_seed = MasterSeed(seed_val);
    let shard_config = build_shard_config();

    let topo = topology::TopologyEngine::generate_single_shard_topology(
        &topology::SingleShardTopologyInput {
            config: &shard_config,
            seed: master_seed,
        },
    )
    .expect("Failed to generate topology");

    // 1. Build AOT Topologies
    let clean_cfg = RunConfig {
        name: "Clean Case".to_string(),
        max_branches: 2,
        max_branch_len: 2,
        w_fascicle: 0.5,
        r_fascicle: 2.5,
        r_repulsion: 1.0,
        override_dendrite_radius: Some(1.5),
    };

    let dense_cfg = RunConfig {
        name: "Dense Stress Case".to_string(),
        max_branches: 3,
        max_branch_len: 3,
        w_fascicle: 0.4,
        r_fascicle: 2.5,
        r_repulsion: 1.2,
        override_dendrite_radius: None,
    };

    println!("Running continuous growth for Clean Case...");
    let (clean_axons, clean_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &clean_cfg);

    println!("Running continuous growth for Dense Case...");
    let (dense_axons, dense_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &dense_cfg);

    // Assert that the Dense case has some L4 -> L5 synapses for a true stress test!
    let dense_l4_to_l5_count = dense_synapses
        .iter()
        .filter(|s| {
            let src_var = topo.somas[s.source_soma_id as usize].variant_id;
            let tgt_var = topo.somas[s.target_soma_id as usize].variant_id;
            // 1 is L4_spiny, 3 is L5_spiny
            src_var == 1 && tgt_var == 3
        })
        .count();

    println!(
        "Dense Case has {} synapses from L4_spiny to L5_spiny",
        dense_l4_to_l5_count
    );
    assert!(
        dense_l4_to_l5_count > 0,
        "Dense Case must contain L4 -> L5 synapses!"
    );

    // 2. Compile to Flat Contract
    let (clean_flat_synapses, clean_flat_axons) =
        build_flat_tuples(&clean_axons, &clean_synapses, &topo);
    let (dense_flat_synapses, dense_flat_axons) =
        build_flat_tuples(&dense_axons, &dense_synapses, &topo);

    // 3. Select stimulated somas (12.5% of somas, deterministic)
    let mut stimulated_somas = Vec::new();
    for (idx, soma) in topo.somas.iter().enumerate() {
        if idx % 8 == 0 {
            stimulated_somas.push(soma.soma_id);
        }
    }
    println!(
        "Selected {} stimulated somas out of {}",
        stimulated_somas.len(),
        topo.somas.len()
    );

    // Setup spikes for three patterns
    let max_ticks = 120;

    // Pattern 1: Single Tick Burst (all spike at tick 0)
    let mut pattern_1_spikes = HashMap::new();
    for &soma_id in &stimulated_somas {
        pattern_1_spikes.insert(soma_id, vec![0]);
    }

    // Pattern 2: Staggered Wave (spikes staggered modulo 16)
    let mut pattern_2_spikes = HashMap::new();
    for (idx, &soma_id) in stimulated_somas.iter().enumerate() {
        let tick = idx % 16;
        pattern_2_spikes.insert(soma_id, vec![tick]);
    }

    // Pattern 3: Repeated Sparse Pulses (spike every 20 ticks)
    let mut pattern_3_spikes = HashMap::new();
    for &soma_id in &stimulated_somas {
        let mut times = Vec::new();
        let mut next_t = soma_id as usize % 15;
        while next_t < max_ticks - 25 {
            times.push(next_t);
            next_t += 20;
        }
        pattern_3_spikes.insert(soma_id, times);
    }

    // Run simulations and assert parity for all cases
    let mut plot_spikes = Vec::new();
    for (&soma_id, times) in &pattern_3_spikes {
        for &t in times {
            plot_spikes.push(serde_json::json!({
                "soma_id": soma_id,
                "tick": t
            }));
        }
    }

    // Parity variables
    let mut clean_aot_evs = Vec::new();
    let mut clean_flat_evs = Vec::new();
    let mut dense_aot_evs = Vec::new();
    let mut dense_flat_evs = Vec::new();

    let mut clean_pattern_1_counts = Vec::new();
    let mut clean_pattern_2_counts = Vec::new();
    let mut clean_pattern_3_counts = Vec::new();
    let mut dense_pattern_1_counts = Vec::new();
    let mut dense_pattern_2_counts = Vec::new();
    let mut dense_pattern_3_counts = Vec::new();

    for (p_idx, spikes) in [pattern_1_spikes, pattern_2_spikes, pattern_3_spikes]
        .into_iter()
        .enumerate()
    {
        println!("Running pattern {} parity checks...", p_idx + 1);

        // Clean AOT vs Flat
        let c_aot = simulate_aot_oracle(
            &clean_axons,
            &clean_synapses,
            &spikes,
            &stimulated_somas,
            max_ticks,
        );
        let c_flat = simulate_flat_runtime(
            &clean_flat_axons,
            &clean_flat_synapses,
            &spikes,
            &stimulated_somas,
            max_ticks,
        );

        let mut c_aot_sorted = c_aot.clone();
        c_aot_sorted.sort();
        let mut c_flat_sorted = c_flat.clone();
        c_flat_sorted.sort();

        println!(
            "Clean Case Pattern {}: AOT events = {}, Flat events = {}",
            p_idx + 1,
            c_aot_sorted.len(),
            c_flat_sorted.len()
        );

        if c_aot_sorted != c_flat_sorted {
            println!("Clean Case mismatch details:");
            println!(
                "AOT events sample (first 10): {:?}",
                &c_aot_sorted[..c_aot_sorted.len().min(10)]
            );
            println!(
                "Flat events sample (first 10): {:?}",
                &c_flat_sorted[..c_flat_sorted.len().min(10)]
            );
        }

        assert_eq!(
            c_aot_sorted.len(),
            c_flat_sorted.len(),
            "Clean Pattern {} event counts do not match! AOT={}, Flat={}",
            p_idx + 1,
            c_aot_sorted.len(),
            c_flat_sorted.len()
        );
        assert_eq!(
            c_aot_sorted,
            c_flat_sorted,
            "Clean Pattern {} event details mismatch!",
            p_idx + 1
        );

        // Dense AOT vs Flat
        let d_aot = simulate_aot_oracle(
            &dense_axons,
            &dense_synapses,
            &spikes,
            &stimulated_somas,
            max_ticks,
        );
        let d_flat = simulate_flat_runtime(
            &dense_flat_axons,
            &dense_flat_synapses,
            &spikes,
            &stimulated_somas,
            max_ticks,
        );

        let mut d_aot_sorted = d_aot.clone();
        d_aot_sorted.sort();
        let mut d_flat_sorted = d_flat.clone();
        d_flat_sorted.sort();

        println!(
            "Dense Case Pattern {}: AOT events = {}, Flat events = {}",
            p_idx + 1,
            d_aot_sorted.len(),
            d_flat_sorted.len()
        );

        if d_aot_sorted != d_flat_sorted {
            println!("Dense Case mismatch details:");
            let mut aot_only = Vec::new();
            for ev in &d_aot_sorted {
                if !d_flat_sorted.contains(ev) {
                    aot_only.push(ev.clone());
                }
            }
            println!(
                "Events in AOT only (first 20): {:?}",
                &aot_only[..aot_only.len().min(20)]
            );

            let mut flat_only = Vec::new();
            for ev in &d_flat_sorted {
                if !d_aot_sorted.contains(ev) {
                    flat_only.push(ev.clone());
                }
            }
            println!(
                "Events in Flat only (first 20): {:?}",
                &flat_only[..flat_only.len().min(20)]
            );
        }

        assert_eq!(
            d_aot_sorted.len(),
            d_flat_sorted.len(),
            "Dense Pattern {} event counts do not match! AOT={}, Flat={}",
            p_idx + 1,
            d_aot_sorted.len(),
            d_flat_sorted.len()
        );
        assert_eq!(
            d_aot_sorted,
            d_flat_sorted,
            "Dense Pattern {} event details mismatch!",
            p_idx + 1
        );

        let mut clean_counts = vec![0; max_ticks];
        for e in &c_aot {
            clean_counts[e.tick] += 1;
        }
        let mut dense_counts = vec![0; max_ticks];
        for e in &d_aot {
            dense_counts[e.tick] += 1;
        }

        if p_idx == 0 {
            clean_pattern_1_counts = clean_counts;
            dense_pattern_1_counts = dense_counts;
        } else if p_idx == 1 {
            clean_pattern_2_counts = clean_counts;
            dense_pattern_2_counts = dense_counts;
        } else if p_idx == 2 {
            clean_pattern_3_counts = clean_counts;
            dense_pattern_3_counts = dense_counts;
            clean_aot_evs = c_aot;
            clean_flat_evs = c_flat;
            dense_aot_evs = d_aot;
            dense_flat_evs = d_flat;
        }
    }

    println!("All compile parity tests completed successfully!");

    // Serialize plot data
    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("docs");
    artifacts_dir.push("engine");
    artifacts_dir.push("research");
    artifacts_dir.push("archive");
    artifacts_dir.push("2026-07-06_growth_v2_aot_flat_parity_v0_4");
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let output_path = artifacts_dir.join("growth_v2_aot_flat_parity_plot_data.json");
    let file = File::create(&output_path).unwrap();

    let plot_json = serde_json::json!({
        "somas": topo.somas.iter().map(|s| {
            serde_json::json!({
                "soma_id": s.soma_id,
                "x": s.position.x(),
                "y": s.position.y(),
                "z": s.position.z(),
                "variant_id": s.variant_id
            })
        }).collect::<Vec<_>>(),
        "stimulated_somas": stimulated_somas,
        "spikes": plot_spikes,
        "clean_axons": clean_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "branches": a.branches.iter().map(|b| {
                    b.iter().map(|seg| [seg.x, seg.y, seg.z]).collect::<Vec<_>>()
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "dense_axons": dense_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "branches": a.branches.iter().map(|b| {
                    b.iter().map(|seg| [seg.x, seg.y, seg.z]).collect::<Vec<_>>()
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "clean_synapses": clean_synapses.iter().map(|s| {
            let axon = &clean_axons[s.source_soma_id as usize];
            let seg = &axon.branches[s.branch_id as usize][s.segment_offset as usize - 1];
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "x": seg.x,
                "y": seg.y,
                "z": seg.z,
                "source_variant": topo.somas[s.source_soma_id as usize].variant_id,
                "target_variant": topo.somas[s.target_soma_id as usize].variant_id,
                "branch_id": s.branch_id,
                "segment_offset": s.segment_offset,
                "flat_segment_idx": flat_segment_idx_for(axon, s.branch_id, s.segment_offset)
            })
        }).collect::<Vec<_>>(),
        "dense_synapses": dense_synapses.iter().map(|s| {
            let axon = &dense_axons[s.source_soma_id as usize];
            let seg = &axon.branches[s.branch_id as usize][s.segment_offset as usize - 1];
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "x": seg.x,
                "y": seg.y,
                "z": seg.z,
                "source_variant": topo.somas[s.source_soma_id as usize].variant_id,
                "target_variant": topo.somas[s.target_soma_id as usize].variant_id,
                "branch_id": s.branch_id,
                "segment_offset": s.segment_offset,
                "flat_segment_idx": flat_segment_idx_for(axon, s.branch_id, s.segment_offset)
            })
        }).collect::<Vec<_>>(),
        "clean_aot_events": clean_aot_evs.iter().map(|e| {
            serde_json::json!({
                "tick": e.tick,
                "source": e.source_soma_id,
                "target": e.target_soma_id,
                "flat_segment_idx": e.flat_segment_idx
            })
        }).collect::<Vec<_>>(),
        "clean_flat_events": clean_flat_evs.iter().map(|e| {
            serde_json::json!({
                "tick": e.tick,
                "source": e.source_soma_id,
                "target": e.target_soma_id,
                "flat_segment_idx": e.flat_segment_idx
            })
        }).collect::<Vec<_>>(),
        "dense_aot_events": dense_aot_evs.iter().map(|e| {
            serde_json::json!({
                "tick": e.tick,
                "source": e.source_soma_id,
                "target": e.target_soma_id,
                "flat_segment_idx": e.flat_segment_idx
            })
        }).collect::<Vec<_>>(),
        "dense_flat_events": dense_flat_evs.iter().map(|e| {
            serde_json::json!({
                "tick": e.tick,
                "source": e.source_soma_id,
                "target": e.target_soma_id,
                "flat_segment_idx": e.flat_segment_idx
            })
        }).collect::<Vec<_>>(),
        "clean_pattern_1_counts": clean_pattern_1_counts,
        "clean_pattern_2_counts": clean_pattern_2_counts,
        "clean_pattern_3_counts": clean_pattern_3_counts,
        "dense_pattern_1_counts": dense_pattern_1_counts,
        "dense_pattern_2_counts": dense_pattern_2_counts,
        "dense_pattern_3_counts": dense_pattern_3_counts,
        "clean_metrics": {
            "soma_count": topo.somas.len(),
            "axon_count": clean_axons.len(),
            "branch_count": clean_axons.iter().map(|a| a.branches.len()).sum::<usize>(),
            "total_branch_segments": clean_axons.iter().map(|a| a.branches.iter().map(|b| b.len()).sum::<usize>()).sum::<usize>(),
            "accepted_synapses": clean_synapses.len(),
            "max_flat_segment_idx": clean_flat_axons.iter().map(|a| a.total_segments).max().unwrap_or(0),
        },
        "dense_metrics": {
            "soma_count": topo.somas.len(),
            "axon_count": dense_axons.len(),
            "branch_count": dense_axons.iter().map(|a| a.branches.len()).sum::<usize>(),
            "total_branch_segments": dense_axons.iter().map(|a| a.branches.iter().map(|b| b.len()).sum::<usize>()).sum::<usize>(),
            "accepted_synapses": dense_synapses.len(),
            "max_flat_segment_idx": dense_flat_axons.iter().map(|a| a.total_segments).max().unwrap_or(0),
        }
    });

    serde_json::to_writer_pretty(file, &plot_json).unwrap();
    println!("Wrote detailed plot data to {}", output_path.display());
    println!("=== Growth v2 AOT-to-Flat Parity Verification Complete ===");
}
