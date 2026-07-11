#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]

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
#[allow(dead_code)]
struct MultifieldAxonPath {
    soma_id: u32,
    axon_type_id: u8,
    branches: Vec<Vec<MultifieldSegment>>,
    stop_reason: &'static str,
    state_transitions: Vec<String>,
}

#[derive(Debug, Clone)]
struct Synapse {
    source_soma_id: u32,
    target_soma_id: u32,
    branch_id: u32,
    segment_offset: u8,
    distance_sq: f32,
}

#[derive(Debug, Clone)]
struct SweepConfig {
    name: String,
    one_per_source_target: bool,
    softmax_cap_per_pair: Option<(usize, f32)>, // (max_per_pair, beta)
    max_branches: usize,
    max_branch_len: usize,
    w_fascicle: f32,
    r_fascicle: f32,
    r_repulsion: f32,
    override_dendrite_radius: Option<f32>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct SweepRunResult {
    config_name: String,
    one_per_source_target: bool,
    softmax_cap_per_pair: Option<(usize, f32)>,
    max_branches: usize,
    max_branch_len: usize,
    w_fascicle: f32,
    r_fascicle: f32,
    r_repulsion: f32,
    override_dendrite_radius: Option<f32>,

    // Geometry/Correctness
    out_of_bounds_violations: usize,
    self_intersection_violations: usize,
    soma_collision_attempts: usize,
    whitelist_violations: usize,
    exact_radius_violations: usize,

    // Projections
    expected_projections_count: HashMap<String, usize>,
    unexpected_projection_count: usize,
    virtual_to_l4_success_rate: f32,

    // Density/Pruning
    raw_candidate_count: usize,
    accepted_synapse_count: usize,
    dropped_candidate_count: usize,
    uniqueness_pruned_count: usize,
    saturated_target_somas_count: usize,
    fan_in_mean: f32,
    fan_in_p50: f32,
    fan_in_p90: f32,
    fan_in_p99: f32,
    source_out_degree_mean: f32,
    source_out_degree_p90: f32,
    duplicate_source_target_pairs_count: usize,

    // Morphology
    mean_path_length: f32,
    mean_branch_count: f32,
    mean_terminal_knot_index: f32,
    mean_arbor_spread_radius: f32,

    // Compile-readiness
    total_branch_segments: usize,
    estimated_flat_segment_count: usize,
    estimated_axon_stream_count: usize,
    can_map_to_flat_tuple: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrowthState {
    Pathfinding,
    TractFollowing,
    TargetZoneCapture,
    TerminalArborization,
    Terminated,
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

fn prune_candidates_sweep(
    mut target_candidates: Vec<Vec<Synapse>>,
    one_per_source_target: bool,
    softmax_cap_per_pair: Option<(usize, f32)>,
    cap_limit: usize,
    seed: u64,
) -> (Vec<Synapse>, usize, usize, usize, usize) {
    let mut accepted = Vec::new();
    let mut total_candidates = 0;
    let mut accepted_count_total = 0;
    let mut dropped_by_cap = 0;
    let mut dropped_by_uniqueness = 0;

    #[allow(clippy::needless_range_loop)]
    for target_idx in 0..target_candidates.len() {
        let candidates = &mut target_candidates[target_idx];
        total_candidates += candidates.len();

        candidates.sort_by(|a, b| {
            a.distance_sq
                .partial_cmp(&b.distance_sq)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.segment_offset.cmp(&b.segment_offset))
                .then(a.source_soma_id.cmp(&b.source_soma_id))
        });

        let mut selected_candidates = Vec::new();

        if one_per_source_target {
            let mut source_seen = HashSet::new();
            for cand in candidates.iter() {
                if source_seen.contains(&cand.source_soma_id) {
                    dropped_by_uniqueness += 1;
                    continue;
                }
                source_seen.insert(cand.source_soma_id);
                selected_candidates.push(cand.clone());
            }
        } else if let Some((max_per_pair, beta)) = softmax_cap_per_pair {
            let mut groups: HashMap<u32, Vec<Synapse>> = HashMap::new();
            for cand in candidates.iter() {
                groups
                    .entry(cand.source_soma_id)
                    .or_default()
                    .push(cand.clone());
            }

            for (source_id, mut group_cands) in groups {
                if group_cands.len() <= max_per_pair {
                    selected_candidates.extend(group_cands);
                } else {
                    let mut chosen = Vec::new();
                    let mut rng_seed = deterministic_rng(seed, source_id, target_idx);

                    while chosen.len() < max_per_pair && !group_cands.is_empty() {
                        let mut min_d_sq = f32::MAX;
                        for c in &group_cands {
                            if c.distance_sq < min_d_sq {
                                min_d_sq = c.distance_sq;
                            }
                        }

                        let mut weights = Vec::new();
                        let mut sum_w = 0.0;
                        for c in &group_cands {
                            let score = -(c.distance_sq - min_d_sq) * beta;
                            let w = score.exp();
                            weights.push(w);
                            sum_w += w;
                        }

                        if sum_w <= 0.0 {
                            let idx = (rng_seed as usize) % group_cands.len();
                            chosen.push(group_cands.remove(idx));
                        } else {
                            rng_seed = deterministic_rng(rng_seed, 0, 0);
                            let u = (rng_seed % 10000) as f32 / 10000.0;
                            let mut accumulated = 0.0;
                            let mut selected_idx = 0;
                            for (idx, &w) in weights.iter().enumerate() {
                                accumulated += w / sum_w;
                                if accumulated >= u {
                                    selected_idx = idx;
                                    break;
                                }
                            }
                            if selected_idx >= group_cands.len() {
                                selected_idx = group_cands.len() - 1;
                            }
                            chosen.push(group_cands.remove(selected_idx));
                        }
                    }
                    dropped_by_uniqueness += group_cands.len();
                    selected_candidates.extend(chosen);
                }
            }

            selected_candidates.sort_by(|a, b| {
                a.distance_sq
                    .partial_cmp(&b.distance_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.segment_offset.cmp(&b.segment_offset))
                    .then(a.source_soma_id.cmp(&b.source_soma_id))
            });
        } else {
            selected_candidates = candidates.clone();
        }

        let mut accepted_count = 0;
        for cand in selected_candidates {
            if accepted_count >= cap_limit {
                dropped_by_cap += 1;
                continue;
            }
            accepted.push(cand);
            accepted_count += 1;
            accepted_count_total += 1;
        }
    }

    (
        accepted,
        total_candidates,
        accepted_count_total,
        dropped_by_cap,
        dropped_by_uniqueness,
    )
}

fn run_multifield_parameterized(
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
    seed: u64,
    sweep_config: &SweepConfig,
) -> (Vec<MultifieldAxonPath>, Vec<Synapse>, usize, usize, usize) {
    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;
    let max_steps = 15;
    let step_size_um = 1.0;

    let target_zone_radius_um = 5.0;
    let capture_radius_um = 1.8;
    let damping_radius_um = 5.0;

    let soma_core_radius = 0.5;
    let repulsion_radius = sweep_config.r_repulsion;

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
        let mut transitions = vec!["Pathfinding".to_string()];
        let mut stop_reason = "MaxLengthReached";
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
            let mut nearest_soma: Option<&topology::PlacedSoma> = None;
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
                            if d <= sweep_config.r_fascicle && d > 0.01 {
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

            let old_state = state;
            if nearest_target_dist <= capture_radius_um {
                state = GrowthState::TerminalArborization;
            } else if nearest_target_dist <= target_zone_radius_um {
                state = GrowthState::TargetZoneCapture;
            } else if found_compatible_segment {
                state = GrowthState::TractFollowing;
            } else {
                state = GrowthState::Pathfinding;
            }

            if state != old_state {
                transitions.push(format!("{:?}", state));
            }

            if state == GrowthState::TerminalArborization {
                break;
            }

            let (w_persist, w_layer, w_fascicle_wt, w_local, w_repulse, w_noise) = match state {
                GrowthState::Pathfinding => (0.3, 0.5, 0.0, 0.0, 0.8, 0.1),
                GrowthState::TractFollowing => (0.3, 0.2, sweep_config.w_fascicle, 0.0, 0.8, 0.1),
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

                    let n_pos =
                        glam::Vec3::new(check_unx as f32, check_uny as f32, check_unz as f32);
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
                        if n_pos.distance(soma_pos) <= soma_core_radius {
                            collided = true;
                            break;
                        }
                    }

                    if collided || visited.contains(&(check_unx, check_uny, check_unz)) {
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

        if state == GrowthState::TerminalArborization && sweep_config.max_branches > 0 {
            let branch_origin = curr_pos_um;
            let rng_val = deterministic_rng(seed, soma.soma_id, 100);
            let num_branches = (rng_val % sweep_config.max_branches as u64) + 1;

            for b in 1..=num_branches {
                let mut terminal_branch = Vec::new();
                let mut b_pos_um = branch_origin;
                let mut b_dir = forward_dir;
                let branch_len = (deterministic_rng(seed, soma.soma_id, 200 + b as usize)
                    % sweep_config.max_branch_len as u64)
                    + 1;

                for b_step in 1..=branch_len {
                    let w_persist = 0.1;
                    let w_local = 0.4;
                    let w_repulse = 0.9;
                    let w_noise = 0.5;

                    let mut nearest_target_dist = f32::MAX;
                    let mut nearest_soma: Option<&topology::PlacedSoma> = None;
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

                    let v_noise =
                        deterministic_noise(seed, soma.soma_id, 300 + b as usize + b_step as usize);
                    let mut v_final = b_dir * w_persist
                        + v_local * w_local
                        + v_repulse * w_repulse
                        + v_noise * w_noise;
                    if v_final.length_squared() < 0.001 {
                        v_final = b_dir;
                    } else {
                        v_final = v_final.normalize();
                    }

                    let next_pos_um = b_pos_um + v_final * step_size_um;
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

                        let no_self_intersection =
                            !visited.contains(&(check_unx, check_uny, check_unz));

                        if !collided && no_self_intersection {
                            unx = check_unx;
                            uny = check_uny;
                            unz = check_unz;
                            b_pos_um = next_pos_um;
                            b_dir = v_final;
                            step_taken = true;
                        }
                    }

                    if !step_taken {
                        let curr_vox = glam::IVec3::new(
                            b_pos_um.x.round() as i32,
                            b_pos_um.y.round() as i32,
                            b_pos_um.z.round() as i32,
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

                            let n_pos = glam::Vec3::new(
                                check_unx as f32,
                                check_uny as f32,
                                check_unz as f32,
                            );
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
                                if n_pos.distance(soma_pos) <= soma_core_radius {
                                    collided = true;
                                    break;
                                }
                            }

                            if collided || visited.contains(&(check_unx, check_uny, check_unz)) {
                                continue;
                            }

                            let dir =
                                glam::Vec3::new(offset.x as f32, offset.y as f32, offset.z as f32)
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
                            b_pos_um = glam::Vec3::new(unx as f32, uny as f32, unz as f32);
                            let offset = chosen_vox - curr_vox;
                            b_dir = glam::Vec3::new(
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

                    terminal_branch.push(MultifieldSegment {
                        x: b_pos_um.x,
                        y: b_pos_um.y,
                        z: b_pos_um.z,
                        segment_offset: b_step as u8,
                        branch_id: b as u32,
                    });
                    visited.insert((unx, uny, unz));
                }
                branches.push(terminal_branch);
            }
            stop_reason = "TargetReached";
            transitions.push("Terminated".to_string());
        }

        completed_axons.push(MultifieldAxonPath {
            soma_id: soma.soma_id,
            axon_type_id: soma.variant_id,
            branches,
            stop_reason,
            state_transitions: transitions,
        });
    }

    let mut target_candidates: Vec<Vec<Synapse>> = vec![Vec::new(); topo.somas.len()];

    for axon in &completed_axons {
        let source_soma = &topo.somas[axon.soma_id as usize];
        let source_type = &config.neuron_types[source_soma.variant_id as usize];

        for branch in &axon.branches {
            for seg in branch {
                let seg_pos = glam::Vec3::new(seg.x, seg.y, seg.z);

                for target in &topo.somas {
                    if target.soma_id == axon.soma_id {
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

                    let dist_sq = seg_pos.distance_squared(target_pos);
                    let radius = if let Some(r) = sweep_config.override_dendrite_radius {
                        r
                    } else {
                        target_type.growth.dendrite_radius_um
                    };

                    if dist_sq <= radius * radius {
                        target_candidates[target.soma_id as usize].push(Synapse {
                            source_soma_id: axon.soma_id,
                            target_soma_id: target.soma_id,
                            branch_id: seg.branch_id,
                            segment_offset: seg.segment_offset,
                            distance_sq: dist_sq,
                        });
                    }
                }
            }
        }
    }

    let (synapses, total_cand, _accepted_cnt, dropped_cap, dropped_uniq) = prune_candidates_sweep(
        target_candidates,
        sweep_config.one_per_source_target,
        sweep_config.softmax_cap_per_pair,
        128,
        seed,
    );

    (
        completed_axons,
        synapses,
        total_cand,
        dropped_cap,
        dropped_uniq,
    )
}

fn percentile(v: &mut [usize], pct: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort();
    let idx = ((v.len() - 1) as f32 * pct).round() as usize;
    v[idx] as f32
}

#[allow(clippy::too_many_arguments)]
fn analyze_sweep_run(
    config_name: String,
    sweep_config: &SweepConfig,
    axons: &[MultifieldAxonPath],
    synapses: &[Synapse],
    topo: &topology::SingleShardTopology,
    config: &config::ShardConfig,
    total_candidates: usize,
    dropped_candidates: usize,
    uniqueness_pruned_count: usize,
) -> SweepRunResult {
    let shard_w = config.dimensions.w;
    let shard_d = config.dimensions.d;
    let shard_h = config.dimensions.h;
    let soma_core_radius = 0.5f32;

    let mut out_of_bounds = 0;
    let mut self_intersections = 0;
    let mut soma_collisions = 0;

    let mut total_len = 0;
    let mut branch_count_sum = 0.0;
    let mut spread_radius_sum = 0.0;
    let mut knot_index_sum = 0.0;

    let mut total_branch_segments = 0;
    let mut estimated_axon_stream_count = 0;

    let mut virtual_axons_count = 0;
    let mut virtual_inside_target_layer = 0;

    for axon in axons {
        let mut total_points = 0;
        let mut points = Vec::new();
        branch_count_sum += axon.branches.len() as f32;
        estimated_axon_stream_count += axon.branches.len();

        for b in &axon.branches {
            total_points += b.len();
            total_branch_segments += b.len();
            for seg in b {
                points.push(glam::Vec3::new(seg.x, seg.y, seg.z));
            }
        }
        total_len += total_points;

        let mut visited_voxels = HashSet::new();
        for &pt in &points {
            let x = pt.x.round() as i32;
            let y = pt.y.round() as i32;
            let z = pt.z.round() as i32;

            if x < 0
                || x >= shard_w as i32
                || y < 0
                || y >= shard_d as i32
                || z < 0
                || z >= shard_h as i32
            {
                out_of_bounds += 1;
            }

            let vox = (
                x.clamp(0, shard_w as i32 - 1) as u32,
                y.clamp(0, shard_d as i32 - 1) as u32,
                z.clamp(0, shard_h as i32 - 1) as u32,
            );
            if visited_voxels.contains(&vox) {
                self_intersections += 1;
            } else {
                visited_voxels.insert(vox);
            }

            for s in &topo.somas {
                if s.soma_id == axon.soma_id {
                    continue;
                }
                let soma_pos = glam::Vec3::new(
                    s.position.x() as f32,
                    s.position.y() as f32,
                    s.position.z() as f32,
                );
                if pt.distance(soma_pos) <= soma_core_radius {
                    soma_collisions += 1;
                }
            }
        }

        let last_stem_pt = if !axon.branches[0].is_empty() {
            let last_idx = axon.branches[0].len() - 1;
            let last_seg = &axon.branches[0][last_idx];
            glam::Vec3::new(last_seg.x, last_seg.y, last_seg.z)
        } else {
            glam::Vec3::ZERO
        };

        let ep = last_stem_pt;
        let mut local_segs = 0;
        let mut max_spread = 0.0f32;
        for b in &axon.branches {
            for seg in b {
                let pt = glam::Vec3::new(seg.x, seg.y, seg.z);
                let d = pt.distance(ep);
                if d <= 2.0 {
                    local_segs += 1;
                }
                if seg.branch_id > 0 && d > max_spread {
                    max_spread = d;
                }
            }
        }
        spread_radius_sum += max_spread;
        let knot_idx = local_segs as f32 / (axon.branches.len() as f32 + 1.0);
        knot_index_sum += knot_idx;

        if axon.axon_type_id == 0 {
            virtual_axons_count += 1;
            let mut reached = false;
            for b in &axon.branches {
                for seg in b {
                    if seg.z.round() as u32 >= 8 {
                        reached = true;
                        break;
                    }
                }
            }
            if reached {
                virtual_inside_target_layer += 1;
            }
        }
    }

    let mean_path_length = total_len as f32 / axons.len() as f32;
    let mean_branch_count = branch_count_sum / axons.len() as f32;
    let mean_terminal_knot_index = knot_index_sum / axons.len() as f32;
    let mean_arbor_spread_radius = spread_radius_sum / axons.len() as f32;

    let virtual_to_l4_success_rate = if virtual_axons_count > 0 {
        virtual_inside_target_layer as f32 / virtual_axons_count as f32
    } else {
        0.0
    };

    // Synapse validations and counts
    let mut whitelist_violations = 0;
    let mut exact_radius_violations = 0;
    let mut unexpected_projection_count = 0;
    let mut expected_projections_count = HashMap::new();

    let mut fan_in_counts = vec![0; topo.somas.len()];
    let mut source_out_degrees = vec![0; topo.somas.len()];
    let mut source_target_pairs = HashSet::new();
    let mut duplicate_source_target_pairs_count = 0;

    for syn in synapses {
        let source_soma = &topo.somas[syn.source_soma_id as usize];
        let target_soma = &topo.somas[syn.target_soma_id as usize];
        let source_type = &config.neuron_types[source_soma.variant_id as usize];
        let target_type = &config.neuron_types[target_soma.variant_id as usize];

        fan_in_counts[syn.target_soma_id as usize] += 1;
        source_out_degrees[syn.source_soma_id as usize] += 1;

        if !target_type
            .growth
            .dendrite_whitelist
            .contains(&source_type.name)
        {
            whitelist_violations += 1;
            unexpected_projection_count += 1;
        } else {
            let key = format!("{} -> {}", source_type.name, target_type.name);
            *expected_projections_count.entry(key).or_insert(0) += 1;
        }

        let radius = if let Some(r) = sweep_config.override_dendrite_radius {
            r
        } else {
            target_type.growth.dendrite_radius_um
        };
        if syn.distance_sq > radius * radius {
            exact_radius_violations += 1;
        }

        let pair = (syn.source_soma_id, syn.target_soma_id);
        if source_target_pairs.contains(&pair) {
            duplicate_source_target_pairs_count += 1;
        } else {
            source_target_pairs.insert(pair);
        }
    }

    let mut saturated_target_somas_count = 0;
    for &count in &fan_in_counts {
        if count >= 128 {
            saturated_target_somas_count += 1;
        }
    }

    let fan_in_mean = synapses.len() as f32 / topo.somas.len() as f32;
    let mut temp_fan_in = fan_in_counts.clone();
    let fan_in_p50 = percentile(&mut temp_fan_in, 0.50);
    let fan_in_p90 = percentile(&mut temp_fan_in, 0.90);
    let fan_in_p99 = percentile(&mut temp_fan_in, 0.99);

    let source_out_degree_mean = synapses.len() as f32 / topo.somas.len() as f32;
    let mut temp_out = source_out_degrees.clone();
    let source_out_degree_p90 = percentile(&mut temp_out, 0.90);

    let mut can_map_to_flat_tuple = true;
    let mut dendrite_counters = vec![0; topo.somas.len()];

    for syn in synapses {
        let axon = &axons[syn.source_soma_id as usize];
        let branch_idx = syn.branch_id as usize;

        if branch_idx >= axon.branches.len() {
            can_map_to_flat_tuple = false;
            break;
        }

        let seg_offset = syn.segment_offset as usize;
        if seg_offset == 0 || seg_offset > axon.branches[branch_idx].len() {
            can_map_to_flat_tuple = false;
            break;
        }

        let mut flat_segment_idx = 0;
        for b_i in 0..branch_idx {
            flat_segment_idx += axon.branches[b_i].len();
        }
        flat_segment_idx += seg_offset - 1;

        let total_axon_segments: usize = axon.branches.iter().map(|b| b.len()).sum();
        if flat_segment_idx >= total_axon_segments {
            can_map_to_flat_tuple = false;
            break;
        }

        let dendrite_idx = dendrite_counters[syn.target_soma_id as usize];
        dendrite_counters[syn.target_soma_id as usize] += 1;

        let _flat_tuple = (
            syn.source_soma_id,
            flat_segment_idx,
            syn.target_soma_id,
            dendrite_idx,
        );
    }

    SweepRunResult {
        config_name,
        one_per_source_target: sweep_config.one_per_source_target,
        softmax_cap_per_pair: sweep_config.softmax_cap_per_pair,
        max_branches: sweep_config.max_branches,
        max_branch_len: sweep_config.max_branch_len,
        w_fascicle: sweep_config.w_fascicle,
        r_fascicle: sweep_config.r_fascicle,
        r_repulsion: sweep_config.r_repulsion,
        override_dendrite_radius: sweep_config.override_dendrite_radius,
        out_of_bounds_violations: out_of_bounds,
        self_intersection_violations: self_intersections,
        soma_collision_attempts: soma_collisions,
        whitelist_violations,
        exact_radius_violations,
        expected_projections_count,
        unexpected_projection_count,
        virtual_to_l4_success_rate,
        raw_candidate_count: total_candidates,
        accepted_synapse_count: synapses.len(),
        dropped_candidate_count: dropped_candidates,
        uniqueness_pruned_count,
        saturated_target_somas_count,
        fan_in_mean,
        fan_in_p50,
        fan_in_p90,
        fan_in_p99,
        source_out_degree_mean,
        source_out_degree_p90,
        duplicate_source_target_pairs_count,
        mean_path_length,
        mean_branch_count,
        mean_terminal_knot_index,
        mean_arbor_spread_radius,
        total_branch_segments,
        estimated_flat_segment_count: total_branch_segments,
        estimated_axon_stream_count,
        can_map_to_flat_tuple,
    }
}

#[test]
fn run_growth_v2_pruning_sweep() {
    println!("=== Starting Growth v2 Parameter Sweep & Pruning Policy v0.3 ===");

    let shard_config = build_shard_config();
    let seed_val = 12345;
    let master_seed = MasterSeed(seed_val);

    let topo = topology::TopologyEngine::generate_single_shard_topology(
        &topology::SingleShardTopologyInput {
            config: &shard_config,
            seed: master_seed,
        },
    )
    .expect("Failed to generate topology");

    let sweep_configs = vec![
        SweepConfig {
            name: "1. Baseline (Multifield v0.2)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "2. No Uniqueness Pruning (Raw contacts)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "3. Softmax Cap 1 (beta=2.0)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: Some((1, 2.0)),
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "4. Softmax Cap 2 (beta=2.0)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: Some((2, 2.0)),
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "5. Softmax Cap 2 (beta=0.5)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: Some((2, 0.5)),
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "6. Softmax Cap 3 (beta=5.0)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: Some((3, 5.0)),
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "7. Low Branching (max 1)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 1,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "8. High Branching (max 5)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 5,
            max_branch_len: 4,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "9. Low Branch Length (max 1)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 1,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "10. High Fasciculation (w=0.9)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.9,
            r_fascicle: 3.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "11. No Fasciculation (w=0.0)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.0,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "12. High Repulsion (R=1.8)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.8,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "13. Low Repulsion (R=0.6)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 0.6,
            override_dendrite_radius: None,
        },
        SweepConfig {
            name: "14. Tight Dendrite Radius (1.0 um)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: Some(1.0),
        },
        SweepConfig {
            name: "15. Large Dendrite Radius (2.5 um)".to_string(),
            one_per_source_target: true,
            softmax_cap_per_pair: None,
            max_branches: 3,
            max_branch_len: 3,
            w_fascicle: 0.4,
            r_fascicle: 2.5,
            r_repulsion: 1.2,
            override_dendrite_radius: Some(2.5),
        },
        SweepConfig {
            name: "16. Compile Candidate (Softmax 2, beta=2.0, max branch 2, length 2, tight repulsion 1.0, dendrite radius 1.5)".to_string(),
            one_per_source_target: false,
            softmax_cap_per_pair: Some((2, 2.0)),
            max_branches: 2,
            max_branch_len: 2,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.0,
            override_dendrite_radius: Some(1.5),
        },
    ];

    let mut results = Vec::new();
    let mut winner_result: Option<SweepRunResult> = None;

    let mut baseline_axons_out = Vec::new();
    let mut baseline_synapses_out = Vec::new();
    let mut winner_axons_out = Vec::new();
    let mut winner_synapses_out = Vec::new();

    for cfg in &sweep_configs {
        println!("Running sweep config: {}", cfg.name);
        let (axons, synapses, total_cand, dropped_cap, dropped_uniq) =
            run_multifield_parameterized(&topo, &shard_config, seed_val, cfg);

        let run_res = analyze_sweep_run(
            cfg.name.clone(),
            cfg,
            &axons,
            &synapses,
            &topo,
            &shard_config,
            total_cand,
            dropped_cap,
            dropped_uniq,
        );

        assert_eq!(
            run_res.out_of_bounds_violations, 0,
            "Config {} has out-of-bounds violations",
            cfg.name
        );
        assert_eq!(
            run_res.self_intersection_violations, 0,
            "Config {} has self-intersections",
            cfg.name
        );
        assert_eq!(
            run_res.soma_collision_attempts, 0,
            "Config {} has soma core collisions",
            cfg.name
        );
        assert_eq!(
            run_res.whitelist_violations, 0,
            "Config {} has whitelist violations",
            cfg.name
        );
        assert_eq!(
            run_res.exact_radius_violations, 0,
            "Config {} has exact radius violations",
            cfg.name
        );
        assert!(
            run_res.can_map_to_flat_tuple,
            "Config {} cannot be mapped to flat segment indexing",
            cfg.name
        );

        if cfg.name.contains("Compile Candidate") {
            winner_result = Some(run_res.clone());
        }

        if cfg.name.starts_with("1. Baseline") {
            baseline_axons_out = axons.clone();
            baseline_synapses_out = synapses.clone();
        }
        if cfg.name.starts_with("16. Compile") {
            winner_axons_out = axons.clone();
            winner_synapses_out = synapses.clone();
        }

        results.push(run_res);
    }

    if let Some(winner) = winner_result {
        println!("Verifying determinism of the Compile Candidate configuration...");
        let winner_cfg = sweep_configs.last().unwrap();
        let (axons_dup, synapses_dup, total_cand_dup, dropped_cap_dup, dropped_uniq_dup) =
            run_multifield_parameterized(&topo, &shard_config, seed_val, winner_cfg);

        let winner_res_dup = analyze_sweep_run(
            "Compile Candidate Rerun Check".to_string(),
            winner_cfg,
            &axons_dup,
            &synapses_dup,
            &topo,
            &shard_config,
            total_cand_dup,
            dropped_cap_dup,
            dropped_uniq_dup,
        );

        assert_eq!(
            winner.accepted_synapse_count, winner_res_dup.accepted_synapse_count,
            "Compile candidate rerun does not match on synapse count!"
        );
        assert_eq!(
            winner.mean_path_length, winner_res_dup.mean_path_length,
            "Compile candidate rerun does not match on mean path length!"
        );
    }

    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("docs");
    artifacts_dir.push("engine");
    artifacts_dir.push("research");
    artifacts_dir.push("archive");
    artifacts_dir.push("2026-07-06_growth_v2_pruning_sweep_v0_3");
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let output_path = artifacts_dir.join("growth_v2_sweep_results.json");
    let file = File::create(&output_path).unwrap();
    serde_json::to_writer_pretty(file, &results).unwrap();

    let plot_data_path = artifacts_dir.join("growth_v2_sweep_plot_data.json");
    let plot_data_file = File::create(&plot_data_path).unwrap();
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
        "baseline_axons": baseline_axons_out.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "branches": a.branches.iter().map(|b| {
                    b.iter().map(|seg| [seg.x, seg.y, seg.z]).collect::<Vec<_>>()
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "winner_axons": winner_axons_out.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "branches": a.branches.iter().map(|b| {
                    b.iter().map(|seg| [seg.x, seg.y, seg.z]).collect::<Vec<_>>()
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "baseline_synapses": baseline_synapses_out.iter().map(|s| {
            let axon = &baseline_axons_out[s.source_soma_id as usize];
            let seg = &axon.branches[s.branch_id as usize][s.segment_offset as usize - 1];
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "x": seg.x,
                "y": seg.y,
                "z": seg.z,
                "source_variant": topo.somas[s.source_soma_id as usize].variant_id,
                "target_variant": topo.somas[s.target_soma_id as usize].variant_id
            })
        }).collect::<Vec<_>>(),
        "winner_synapses": winner_synapses_out.iter().map(|s| {
            let axon = &winner_axons_out[s.source_soma_id as usize];
            let seg = &axon.branches[s.branch_id as usize][s.segment_offset as usize - 1];
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "x": seg.x,
                "y": seg.y,
                "z": seg.z,
                "source_variant": topo.somas[s.source_soma_id as usize].variant_id,
                "target_variant": topo.somas[s.target_soma_id as usize].variant_id
            })
        }).collect::<Vec<_>>(),
    });
    serde_json::to_writer_pretty(plot_data_file, &plot_json).unwrap();
    println!("Wrote detailed plot data to {}", plot_data_path.display());
    println!("=== Growth v2 Parameter Sweep Complete ===");
}
