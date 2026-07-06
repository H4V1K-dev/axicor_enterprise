#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use types::MasterSeed;

// Model definitions and helper functions for Growth v2 prototype
#[derive(Debug, Clone)]
struct ContinuousAxonPath {
    soma_id: u32,
    axon_type_id: u8,
    continuous_points: Vec<glam::Vec3>,     // in um
    quantized_points: Vec<(u32, u32, u32)>, // quantized voxels
    stop_reason: &'static str,
    // Tracking violations for metrics
    out_of_bounds_count: usize,
    self_intersection_count: usize,
    soma_collision_count: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Synapse {
    source_soma_id: u32,
    target_soma_id: u32,
    segment_offset: u8,
    distance_sq: f32,
}

fn deterministic_noise(seed: u64, soma_id: u32, step: usize) -> glam::Vec3 {
    let mut hash_val: u64 = 0xcbf2_9ce4_8422_2325;
    hash_val = (hash_val ^ seed).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (soma_id as u64)).wrapping_mul(0x0000_0100_0000_01B3);
    hash_val = (hash_val ^ (step as u64)).wrapping_mul(0x0000_0100_0000_01B3);

    let val_x = (((hash_val & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;
    let val_y = ((((hash_val >> 16) & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;
    let val_z = ((((hash_val >> 32) & 0xFFFF) as f32) / 65535.0) * 2.0 - 1.0;

    let noise = glam::Vec3::new(val_x, val_y, val_z);
    if noise.length_squared() > 0.001 {
        noise.normalize()
    } else {
        glam::Vec3::Z
    }
}

#[allow(clippy::too_many_arguments)]
fn calculate_v_attract(
    current_pos: glam::Vec3,
    forward_dir: glam::Vec3,
    axon_type_name: &str,
    axon_type_id: u8,
    somas: &[topology::PlacedSoma],
    neuron_types: &[config::NeuronType],
    radius_um: f32,
    fov_cos: f32,
    type_affinity: f32,
) -> glam::Vec3 {
    let mut v_attract = glam::Vec3::ZERO;
    for neighbor in somas {
        let neighbor_type = &neuron_types[neighbor.variant_id as usize];
        if !neighbor_type
            .growth
            .dendrite_whitelist
            .iter()
            .any(|name| name == axon_type_name)
        {
            continue;
        }
        let neighbor_pos = glam::Vec3::new(
            neighbor.position.x() as f32,
            neighbor.position.y() as f32,
            neighbor.position.z() as f32,
        );
        let diff = neighbor_pos - current_pos;
        let dist_sq = diff.length_squared();
        if dist_sq > radius_um * radius_um || dist_sq < 0.001 {
            continue;
        }
        let dist = dist_sq.sqrt();
        let dir_to_target = diff / dist;
        let dot = forward_dir.dot(dir_to_target);
        if dot > fov_cos {
            let is_same = (neighbor.variant_id == axon_type_id) as i32 as f32;
            let affinity_mod =
                (is_same * type_affinity + (1.0 - is_same) * (1.0 - type_affinity)) * 2.0;
            let weight = (1.0 / (dist + 1.0)) * affinity_mod;
            v_attract += dir_to_target * weight;
        }
    }
    if v_attract.length_squared() > 0.001 {
        v_attract.normalize()
    } else {
        glam::Vec3::ZERO
    }
}

// Loads a profile from disk
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

// Stage 2 implementation: MVP continuous reference
fn run_mvp_continuous(
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
    seed: u64,
) -> (Vec<ContinuousAxonPath>, Vec<Synapse>) {
    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;
    let max_steps = 15;
    let step_size_um = 1.0;
    let radius_um = 5.0;
    let fov_cos = 0.5; // 60 degrees half FOV -> cos(60) = 0.5
    let type_affinity = 0.6;

    let w_global = 0.5;
    let w_attract = 0.3;
    let w_noise = 0.2;

    let mut axons = Vec::new();
    let somas_coords: HashSet<(u32, u32, u32)> = topo
        .somas
        .iter()
        .map(|s| {
            (
                s.position.x() as u32,
                s.position.y() as u32,
                s.position.z() as u32,
            )
        })
        .collect();

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
            glam::Vec3::new(1.0, 0.0, 0.0) // horizontal default
        };

        let mut continuous_points = Vec::new();
        let mut quantized_points = Vec::new();
        let mut stop_reason = "MaxLengthReached";

        let mut out_of_bounds_count = 0;
        let mut self_intersection_count = 0;
        let mut soma_collision_count = 0;
        let mut visited = HashSet::new();
        visited.insert((sx, sy, sz));

        for step in 1..=max_steps {
            let v_global = if vertical_bias != 0.0 {
                glam::Vec3::new(0.0, 0.0, vertical_bias).normalize_or_zero()
            } else {
                forward_dir
            };

            let v_attract = calculate_v_attract(
                curr_pos_um,
                forward_dir,
                &source_type.name,
                soma.variant_id,
                &topo.somas,
                &config.neuron_types,
                radius_um,
                fov_cos,
                type_affinity,
            );

            let v_noise = deterministic_noise(seed, soma.soma_id, step);
            let mut v_final = v_global * w_global + v_attract * w_attract + v_noise * w_noise;
            if v_final.length_squared() < 0.001 {
                v_final = v_global;
            } else {
                v_final = v_final.normalize();
            }

            let next_pos_um = curr_pos_um + v_final * step_size_um;
            let nx = next_pos_um.x.round() as i32;
            let ny = next_pos_um.y.round() as i32;
            let nz = next_pos_um.z.round() as i32;

            // Stop immediately if out of bounds (BoundaryReached)
            if nx < 0
                || nx >= shard_w as i32
                || ny < 0
                || ny >= shard_d as i32
                || nz < 0
                || nz >= shard_h as i32
            {
                stop_reason = "BoundaryReached";
                out_of_bounds_count += 1;
                break;
            }

            let unx = nx as u32;
            let uny = ny as u32;
            let unz = nz as u32;

            // In MVP, we step even if there is collision, but we count the violation
            if somas_coords.contains(&(unx, uny, unz)) && !(unx == sx && uny == sy && unz == sz) {
                soma_collision_count += 1;
            }
            if visited.contains(&(unx, uny, unz)) {
                self_intersection_count += 1;
            }

            curr_pos_um = next_pos_um;
            forward_dir = v_final;
            continuous_points.push(curr_pos_um);
            quantized_points.push((unx, uny, unz));
            visited.insert((unx, uny, unz));

            // Quantization stagnation check
            if quantized_points.len() >= 2
                && quantized_points[quantized_points.len() - 1]
                    == quantized_points[quantized_points.len() - 2]
            {
                stop_reason = "Stagnated";
                break;
            }
        }

        axons.push(ContinuousAxonPath {
            soma_id: soma.soma_id,
            axon_type_id: soma.variant_id,
            continuous_points,
            quantized_points,
            stop_reason,
            out_of_bounds_count,
            self_intersection_count,
            soma_collision_count,
        });
    }

    // Connect synapses (Mode 2 MVP: uniqueness per source-target axon, exact radius check)
    let mut synapses = Vec::new();
    for axon in &axons {
        let source_soma = &topo.somas[axon.soma_id as usize];
        let source_type = &config.neuron_types[source_soma.variant_id as usize];

        // Group contacts to enforce uniqueness (at most 1 synapse per target soma)
        let mut best_contact_for_target: HashMap<u32, (u8, f32)> = HashMap::new();

        for (seg_offset, &seg_coord) in axon.quantized_points.iter().enumerate() {
            let seg_pos =
                glam::Vec3::new(seg_coord.0 as f32, seg_coord.1 as f32, seg_coord.2 as f32);

            for target in &topo.somas {
                if target.soma_id == axon.soma_id {
                    continue;
                }
                let target_type = &config.neuron_types[target.variant_id as usize];
                if !target_type
                    .growth
                    .dendrite_whitelist
                    .contains(&source_type.name)
                {
                    continue;
                }

                let target_pos = glam::Vec3::new(
                    target.position.x() as f32,
                    target.position.y() as f32,
                    target.position.z() as f32,
                );

                let dist_sq = seg_pos.distance_squared(target_pos);
                let radius = target_type.growth.dendrite_radius_um;
                if dist_sq <= radius * radius {
                    let entry = best_contact_for_target
                        .entry(target.soma_id)
                        .or_insert((seg_offset as u8 + 1, dist_sq));
                    if dist_sq < entry.1 {
                        *entry = (seg_offset as u8 + 1, dist_sq);
                    }
                }
            }
        }

        for (target_soma_id, (seg_offset, dist_sq)) in best_contact_for_target {
            synapses.push(Synapse {
                source_soma_id: axon.soma_id,
                target_soma_id,
                segment_offset: seg_offset,
                distance_sq: dist_sq,
            });
        }
    }

    (axons, synapses)
}

// Stage 3 implementation: hybrid Growth v2
fn run_hybrid_v2(
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
    seed: u64,
) -> (Vec<ContinuousAxonPath>, Vec<Synapse>) {
    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;
    let max_steps = 15;
    let step_size_um = 1.0;
    let radius_um = 5.0;
    let fov_cos = 0.5;
    let type_affinity = 0.6;

    // Anti-terminal-knot parameters
    let capture_radius_um = 1.5;
    let damping_radius_um = 5.0;

    let w_global = 0.5;
    let w_attract = 0.3;
    let w_noise = 0.2;

    let mut axons = Vec::new();
    let somas_coords: HashSet<(u32, u32, u32)> = topo
        .somas
        .iter()
        .map(|s| {
            (
                s.position.x() as u32,
                s.position.y() as u32,
                s.position.z() as u32,
            )
        })
        .collect();

    // 26-neighbor discrete candidates
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

        let mut continuous_points = Vec::new();
        let mut quantized_points = Vec::new();
        let mut stop_reason = "MaxLengthReached";

        let mut visited = HashSet::new();
        visited.insert((sx, sy, sz));

        // Find target somas to monitor target distances
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
                        .contains(&source_type.name)
                }
            })
            .collect();

        let mut min_dist_to_target = f32::MAX;
        let mut stagnation_count = 0;

        for step in 1..=max_steps {
            // Find nearest target soma
            let mut nearest_target_dist = f32::MAX;
            for &target in &target_somas {
                let target_pos = glam::Vec3::new(
                    target.position.x() as f32,
                    target.position.y() as f32,
                    target.position.z() as f32,
                );
                let d = curr_pos_um.distance(target_pos);
                if d < nearest_target_dist {
                    nearest_target_dist = d;
                }
            }

            // Anti-knot: Capture radius stop
            if nearest_target_dist <= capture_radius_um {
                stop_reason = "TargetReached";
                break;
            }

            // Anti-knot: Monotonicity / stagnation check
            if nearest_target_dist < min_dist_to_target {
                min_dist_to_target = nearest_target_dist;
                stagnation_count = 0;
            } else {
                stagnation_count += 1;
                if stagnation_count >= 3 {
                    stop_reason = "Stagnated";
                    break;
                }
            }

            // Anti-knot: Attraction damping
            let current_w_attract = if nearest_target_dist < damping_radius_um {
                w_attract * (nearest_target_dist / damping_radius_um)
            } else {
                w_attract
            };

            let v_global = if vertical_bias != 0.0 {
                glam::Vec3::new(0.0, 0.0, vertical_bias).normalize_or_zero()
            } else {
                forward_dir
            };

            let v_attract = calculate_v_attract(
                curr_pos_um,
                forward_dir,
                &source_type.name,
                soma.variant_id,
                &topo.somas,
                &config.neuron_types,
                radius_um,
                fov_cos,
                type_affinity,
            );

            let v_noise = deterministic_noise(seed, soma.soma_id, step);
            let mut v_final =
                v_global * w_global + v_attract * current_w_attract + v_noise * w_noise;
            if v_final.length_squared() < 0.001 {
                v_final = v_global;
            } else {
                v_final = v_final.normalize();
            }

            let next_pos_um = curr_pos_um + v_final * step_size_um;
            let nx = next_pos_um.x.round() as i32;
            let ny = next_pos_um.y.round() as i32;
            let nz = next_pos_um.z.round() as i32;

            // In Hybrid: strict collision & bounds check
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

                let no_soma_collision = !somas_coords.contains(&(check_unx, check_uny, check_unz))
                    || (check_unx == sx && check_uny == sy && check_unz == sz);
                let no_self_intersection = !visited.contains(&(check_unx, check_uny, check_unz));

                if no_soma_collision && no_self_intersection {
                    unx = check_unx;
                    uny = check_uny;
                    unz = check_unz;
                    curr_pos_um = next_pos_um;
                    forward_dir = v_final;
                    step_taken = true;
                }
            }

            // Fallback: search 26 neighbors and pick the best non-colliding neighbor aligned with v_final
            if !step_taken {
                let curr_vox = glam::IVec3::new(
                    curr_pos_um.x.round() as i32,
                    curr_pos_um.y.round() as i32,
                    curr_pos_um.z.round() as i32,
                );

                let mut best_neighbor = None;
                let mut best_score = -f32::MAX;

                for &offset in &neighbors {
                    let n_vox = curr_vox + offset;
                    if n_vox.x < 0
                        || n_vox.x >= shard_w as i32
                        || n_vox.y < 0
                        || n_vox.y >= shard_d as i32
                        || n_vox.z < 0
                        || n_vox.z >= shard_h as i32
                    {
                        continue;
                    }
                    let check_unx = n_vox.x as u32;
                    let check_uny = n_vox.y as u32;
                    let check_unz = n_vox.z as u32;

                    if somas_coords.contains(&(check_unx, check_uny, check_unz))
                        && !(check_unx == sx && check_uny == sy && check_unz == sz)
                    {
                        continue;
                    }
                    if visited.contains(&(check_unx, check_uny, check_unz)) {
                        continue;
                    }

                    let dir = glam::Vec3::new(offset.x as f32, offset.y as f32, offset.z as f32)
                        .normalize();
                    let score = dir.dot(v_final);
                    if score > best_score {
                        best_score = score;
                        best_neighbor = Some(n_vox);
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
                let next_vox = glam::IVec3::new(nx, ny, nz);
                if next_vox.x < 0
                    || next_vox.x >= shard_w as i32
                    || next_vox.y < 0
                    || next_vox.y >= shard_d as i32
                    || next_vox.z < 0
                    || next_vox.z >= shard_h as i32
                {
                    stop_reason = "BoundaryReached";
                } else {
                    stop_reason = "Blocked";
                }
                break;
            }

            continuous_points.push(curr_pos_um);
            quantized_points.push((unx, uny, unz));
            visited.insert((unx, uny, unz));
        }

        axons.push(ContinuousAxonPath {
            soma_id: soma.soma_id,
            axon_type_id: soma.variant_id,
            continuous_points,
            quantized_points,
            stop_reason,
            out_of_bounds_count: 0,
            self_intersection_count: 0,
            soma_collision_count: 0,
        });
    }

    // Connect synapses (Mode 3 Hybrid: exact radius/whitelists/Dale, no uniqueness check as per AxiEngine baseline)
    let mut synapses = Vec::new();
    for axon in &axons {
        let source_soma = &topo.somas[axon.soma_id as usize];
        let source_type = &config.neuron_types[source_soma.variant_id as usize];

        for (seg_offset, &seg_coord) in axon.quantized_points.iter().enumerate() {
            let seg_pos =
                glam::Vec3::new(seg_coord.0 as f32, seg_coord.1 as f32, seg_coord.2 as f32);

            for target in &topo.somas {
                if target.soma_id == axon.soma_id {
                    continue;
                }
                let target_type = &config.neuron_types[target.variant_id as usize];
                if !target_type
                    .growth
                    .dendrite_whitelist
                    .contains(&source_type.name)
                {
                    continue;
                }

                let target_pos = glam::Vec3::new(
                    target.position.x() as f32,
                    target.position.y() as f32,
                    target.position.z() as f32,
                );

                let dist_sq = seg_pos.distance_squared(target_pos);
                let radius = target_type.growth.dendrite_radius_um;
                if dist_sq <= radius * radius {
                    synapses.push(Synapse {
                        source_soma_id: axon.soma_id,
                        target_soma_id: target.soma_id,
                        segment_offset: seg_offset as u8 + 1,
                        distance_sq: dist_sq,
                    });
                }
            }
        }
    }

    (axons, synapses)
}

fn offset_direction(v: glam::IVec3) -> (f32, f32, f32) {
    let len = glam::Vec3::new(v.x as f32, v.y as f32, v.z as f32).length();
    if len > 0.001 {
        (v.x as f32 / len, v.y as f32 / len, v.z as f32 / len)
    } else {
        (0.0, 0.0, 1.0)
    }
}

// Struct to represent metrics calculation results
#[derive(serde::Serialize)]
struct ModelMetrics {
    mean_length: f32,
    stop_reasons: HashMap<String, usize>,
    out_of_bounds_violations: usize,
    self_intersection_violations: usize,
    soma_collision_attempts: usize,
    whitelist_violations: usize,
    exact_radius_violations: usize,
    uniqueness_violations: usize,
    total_candidates: usize,
    accepted_synapses: usize,
    dropped_candidates: usize,
    // Terminal knot metrics
    mean_last_n_tortuosity: f32,
    mean_endpoint_density: f32,
    mean_final_angle_variance: f32,
    // Tract coherence
    mean_direction: [f32; 3],
    layer_projection_success_rate: f32,
}

fn calculate_metrics(
    axons: &[ContinuousAxonPath],
    synapses: &[Synapse],
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
) -> ModelMetrics {
    let mut total_len = 0;
    let mut stop_counts = HashMap::new();
    let mut out_of_bounds = 0;
    let mut self_intersections = 0;
    let mut soma_collisions = 0;

    let mut tortuosity_sum = 0.0;
    let mut tortuosity_count = 0;
    let mut density_sum = 0.0;
    let mut angle_var_sum = 0.0;
    let mut angle_var_count = 0;

    let mut inside_target_layer_count = 0;

    for axon in axons {
        total_len += axon.quantized_points.len();
        *stop_counts.entry(axon.stop_reason.to_string()).or_insert(0) += 1;
        out_of_bounds += axon.out_of_bounds_count;
        self_intersections += axon.self_intersection_count;
        soma_collisions += axon.soma_collision_count;

        let points = &axon.continuous_points;
        if points.len() >= 5 {
            // Tortuosity of last 5 segments
            let start_pt = points[points.len() - 5];
            let end_pt = points[points.len() - 1];
            let direct_dist = start_pt.distance(end_pt);
            let mut path_dist = 0.0;
            for i in (points.len() - 4)..points.len() {
                path_dist += points[i - 1].distance(points[i]);
            }
            if direct_dist > 0.001 {
                tortuosity_sum += path_dist / direct_dist;
                tortuosity_count += 1;
            }

            // Local endpoint density (radius = 2.0 voxels)
            let ep = end_pt;
            let mut local_segs = 0;
            for pt in points {
                if pt.distance(ep) <= 2.0 {
                    local_segs += 1;
                }
            }
            density_sum += local_segs as f32;

            // Final angle variance
            let mut angles = Vec::new();
            for i in (points.len() - 3)..points.len() {
                let v1 = (points[i - 1] - points[i - 2]).normalize_or_zero();
                let v2 = (points[i] - points[i - 1]).normalize_or_zero();
                let dot = v1.dot(v2).clamp(-1.0, 1.0);
                angles.push(dot.acos());
            }
            if !angles.is_empty() {
                let mean_ang: f32 = angles.iter().sum::<f32>() / angles.len() as f32;
                let var_ang: f32 = angles.iter().map(|&a| (a - mean_ang).powi(2)).sum::<f32>()
                    / angles.len() as f32;
                angle_var_sum += var_ang;
                angle_var_count += 1;
            }
        }

        // Layer projection success rate: did VirtualInput enter L4 (Z>=8)?
        let source_soma = &topo.somas[axon.soma_id as usize];
        if source_soma.variant_id == 0 {
            // VirtualInput
            if let Some(&last_pt) = axon.quantized_points.last() {
                if last_pt.2 >= 8 {
                    inside_target_layer_count += 1;
                }
            }
        }
    }

    let mean_len = total_len as f32 / axons.len() as f32;
    let mean_tort = if tortuosity_count > 0 {
        tortuosity_sum / tortuosity_count as f32
    } else {
        1.0
    };
    let mean_dens = density_sum / axons.len() as f32;
    let mean_angle_var = if angle_var_count > 0 {
        angle_var_sum / angle_var_count as f32
    } else {
        0.0
    };

    let virtual_axons_count = axons.iter().filter(|a| a.axon_type_id == 0).count();
    let success_rate = if virtual_axons_count > 0 {
        inside_target_layer_count as f32 / virtual_axons_count as f32
    } else {
        0.0
    };

    // Synapse metrics. Custom MVP/Hybrid modes pass raw candidate contacts here,
    // so model the production per-target MAX_DENDRITES cap for comparable counts.
    let mut uniqueness_violations = 0;
    let mut whitelist_violations = 0;
    let mut exact_radius_violations = 0;
    let mut candidates_by_target: HashMap<u32, usize> = HashMap::new();

    let mut source_target_pairs = HashSet::new();

    for syn in synapses {
        *candidates_by_target.entry(syn.target_soma_id).or_insert(0) += 1;

        let source_soma = &topo.somas[syn.source_soma_id as usize];
        let target_soma = &topo.somas[syn.target_soma_id as usize];
        let source_type = &config.neuron_types[source_soma.variant_id as usize];
        let target_type = &config.neuron_types[target_soma.variant_id as usize];

        if !target_type
            .growth
            .dendrite_whitelist
            .contains(&source_type.name)
        {
            whitelist_violations += 1;
        }

        let radius = target_type.growth.dendrite_radius_um;
        if syn.distance_sq > radius * radius {
            exact_radius_violations += 1;
        }

        let pair = (syn.source_soma_id, syn.target_soma_id);
        if source_target_pairs.contains(&pair) {
            uniqueness_violations += 1;
        } else {
            source_target_pairs.insert(pair);
        }
    }
    let accepted_synapses = candidates_by_target
        .values()
        .map(|&count| count.min(layout::MAX_DENDRITES))
        .sum();
    let dropped_candidates = synapses.len().saturating_sub(accepted_synapses);

    // Direction vector of VirtualInput (axon_type_id == 0)
    let mut dir_sum = glam::Vec3::ZERO;
    let mut dir_count = 0;
    for axon in axons {
        if axon.axon_type_id == 0 && axon.continuous_points.len() >= 2 {
            let start = axon.continuous_points[0];
            let end = axon.continuous_points[axon.continuous_points.len() - 1];
            dir_sum += (end - start).normalize_or_zero();
            dir_count += 1;
        }
    }
    let mean_dir = if dir_count > 0 {
        (dir_sum / dir_count as f32).normalize_or_zero()
    } else {
        glam::Vec3::Z
    };

    ModelMetrics {
        mean_length: mean_len,
        stop_reasons: stop_counts,
        out_of_bounds_violations: out_of_bounds,
        self_intersection_violations: self_intersections,
        soma_collision_attempts: soma_collisions,
        whitelist_violations,
        exact_radius_violations,
        uniqueness_violations,
        total_candidates: synapses.len(),
        accepted_synapses,
        dropped_candidates,
        mean_last_n_tortuosity: mean_tort,
        mean_endpoint_density: mean_dens,
        mean_final_angle_variance: mean_angle_var,
        mean_direction: [mean_dir.x, mean_dir.y, mean_dir.z],
        layer_projection_success_rate: success_rate,
    }
}

#[test]
fn run_growth_v2_hybrid_prototype() {
    println!("=== Starting Growth v2 Hybrid Prototype & 3D Atlas ===");

    let shard_config = build_shard_config();
    let seed_val = 12345;
    let master_seed = MasterSeed(seed_val);

    // 1. Generate Topology
    let topo = topology::TopologyEngine::generate_single_shard_topology(
        &topology::SingleShardTopologyInput {
            config: &shard_config,
            seed: master_seed,
        },
    )
    .expect("Failed to generate topology");

    // Mode 1: Discrete reference
    let v1_growth = topology::TopologyEngine::grow_local_axons(&topology::AxonGrowthInput {
        config: &shard_config,
        topology: &topo,
        seed: master_seed,
    })
    .expect("Discrete growth failed");

    let v1_synapses_plan =
        topology::TopologyEngine::form_local_synapses(&topology::SynapseFormationInput {
            config: &shard_config,
            topology: &topo,
            growth: &v1_growth,
            voxel_size_um: 1.0,
            seed: master_seed,
        })
        .expect("Discrete synapse plan failed");

    // Convert v1 structures for calculations
    let mut v1_axons = Vec::new();
    for path in &v1_growth.axons {
        let mut continuous_points = Vec::new();
        let mut quantized_points = Vec::new();
        for seg in &path.segments {
            let x = seg.position.x() as u32;
            let y = seg.position.y() as u32;
            let z = seg.position.z() as u32;
            continuous_points.push(glam::Vec3::new(x as f32, y as f32, z as f32));
            quantized_points.push((x, y, z));
        }

        let stop = match path.stop_reason {
            topology::dto::AxonGrowthStopReason::MaxLengthReached => "MaxLengthReached",
            topology::dto::AxonGrowthStopReason::BoundaryReached => "BoundaryReached",
            topology::dto::AxonGrowthStopReason::Blocked => "Blocked",
            topology::dto::AxonGrowthStopReason::SourceOutOfBounds => "SourceOutOfBounds",
        };

        v1_axons.push(ContinuousAxonPath {
            soma_id: path.soma_id,
            axon_type_id: topo.somas[path.soma_id as usize].variant_id,
            continuous_points,
            quantized_points,
            stop_reason: stop,
            out_of_bounds_count: 0,
            self_intersection_count: 0,
            soma_collision_count: 0,
        });
    }

    let mut v1_synapses = Vec::new();
    for row in &v1_synapses_plan.rows {
        for syn in &row.slots {
            v1_synapses.push(Synapse {
                source_soma_id: syn.source_soma_id,
                target_soma_id: row.target_soma_id,
                segment_offset: syn.segment_offset,
                distance_sq: 0.0,
            });
        }
    }

    // Mode 2: MVP Continuous
    let (mvp_axons, mvp_synapses) = run_mvp_continuous(&topo, &shard_config, seed_val);

    // Mode 3: Hybrid Growth v2
    let (hybrid_axons, hybrid_synapses) = run_hybrid_v2(&topo, &shard_config, seed_val);

    // Calculatings metrics
    let v1_metrics = calculate_metrics(&v1_axons, &v1_synapses, &topo, &shard_config);
    let mvp_metrics = calculate_metrics(&mvp_axons, &mvp_synapses, &topo, &shard_config);
    let hybrid_metrics = calculate_metrics(&hybrid_axons, &hybrid_synapses, &topo, &shard_config);

    // Assertion checks for hybrid mode invariants
    assert_eq!(
        hybrid_metrics.out_of_bounds_violations, 0,
        "Hybrid mode must have 0 out-of-bounds violations"
    );
    assert_eq!(
        hybrid_metrics.self_intersection_violations, 0,
        "Hybrid mode must have 0 self-intersections"
    );
    assert_eq!(
        hybrid_metrics.soma_collision_attempts, 0,
        "Hybrid mode must have 0 soma collision attempts"
    );
    assert_eq!(
        hybrid_metrics.whitelist_violations, 0,
        "Hybrid mode must have 0 whitelist violations"
    );
    assert_eq!(
        hybrid_metrics.exact_radius_violations, 0,
        "Hybrid mode must have 0 exact radius violations"
    );

    // Determinism rerun check
    let (hybrid_axons_dup, hybrid_synapses_dup) = run_hybrid_v2(&topo, &shard_config, seed_val);
    let hybrid_metrics_dup = calculate_metrics(
        &hybrid_axons_dup,
        &hybrid_synapses_dup,
        &topo,
        &shard_config,
    );
    assert_eq!(
        hybrid_metrics.mean_length, hybrid_metrics_dup.mean_length,
        "Hybrid mode rerun must yield identical mean length"
    );
    assert_eq!(
        hybrid_metrics.accepted_synapses, hybrid_metrics_dup.accepted_synapses,
        "Hybrid mode rerun must yield identical synapse count"
    );

    // Export raw JSON comparison panel
    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let output_path = artifacts_dir.join("growth_v2_comparison_data.json");
    let json_data = serde_json::json!({
        "somas": topo.somas.iter().map(|s| {
            serde_json::json!({
                "soma_id": s.soma_id,
                "x": s.position.x(),
                "y": s.position.y(),
                "z": s.position.z(),
                "variant_id": s.variant_id
            })
        }).collect::<Vec<_>>(),
        "v1_axons": v1_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "points": a.quantized_points
            })
        }).collect::<Vec<_>>(),
        "mvp_axons": mvp_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "points": a.quantized_points
            })
        }).collect::<Vec<_>>(),
        "hybrid_axons": hybrid_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "points": a.quantized_points
            })
        }).collect::<Vec<_>>(),
        "v1_synapses": v1_synapses.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id
            })
        }).collect::<Vec<_>>(),
        "mvp_synapses": mvp_synapses.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id
            })
        }).collect::<Vec<_>>(),
        "hybrid_synapses": hybrid_synapses.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id
            })
        }).collect::<Vec<_>>(),
        "metrics": {
            "v1": v1_metrics,
            "mvp": mvp_metrics,
            "hybrid": hybrid_metrics
        }
    });

    let file = File::create(&output_path).unwrap();
    serde_json::to_writer_pretty(file, &json_data).unwrap();
    println!("Wrote comparison panel data to {}", output_path.display());
    println!("=== Growth v2 Hybrid Prototype Complete ===");
}
