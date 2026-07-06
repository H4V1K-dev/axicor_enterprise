#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]
#![allow(
    clippy::needless_range_loop,
    dead_code,
    unused_variables,
    clippy::manual_is_multiple_of
)]

use layout::VariantParameters;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use types::constants::AXON_SENTINEL;
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
    weight: i32,
    fatigue: u8,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FlatAxon {
    soma_id: u32,
    total_segments: usize,
    parents: Vec<Option<usize>>,
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
    max_per_pair: usize,
    beta: f32,
    soft_cap: usize,
    projection_aware: bool,
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
    let v = glam::Vec3::new(rx, ry, rz);
    let l = v.length();
    if l > 0.0 {
        (v.x / l, v.y / l, v.z / l).into()
    } else {
        glam::Vec3::ZERO
    }
}

fn find_profile_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // to crates
    path.pop(); // to AxiEngine
    path.pop(); // to workflow
    path.push("Axicor_NeUniform-Lib");
    if !path.exists() {
        // Fallback for standard path structure
        path.pop();
        path.push("Axicor_Neuron-Lib");
    }
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

fn load_variant(path: PathBuf) -> VariantParameters {
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Could not read {}: {:?}", path.display(), e));
    let nt: config::NeuronType = toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse TOML from {}: {:?}", path.display(), e));

    let heartbeat_m = physics::compile_stochastic_heartbeat_threshold(
        nt.spontaneous.spontaneous_firing_period_ticks as u64,
    );

    let mut inertia_curve = [0u8; 8];
    for (i, &v) in nt.gsop.inertia_curve.iter().enumerate().take(8) {
        inertia_curve[i] = v;
    }

    VariantParameters {
        threshold: nt.membrane.threshold,
        rest_potential: nt.membrane.rest_potential,
        leak_shift: nt.membrane.leak_shift,
        homeostasis_penalty: nt.homeostasis.homeostasis_penalty,
        spontaneous_firing_period_ticks: nt.spontaneous.spontaneous_firing_period_ticks,
        initial_synapse_weight: nt.gsop.initial_synapse_weight,
        gsop_potentiation: nt.gsop.gsop_potentiation,
        gsop_depression: nt.gsop.gsop_depression,
        homeostasis_decay: nt.homeostasis.homeostasis_decay,
        refractory_period: nt.timing.refractory_period,
        fatigue_capacity: nt.timing.fatigue_capacity,
        signal_propagation_length: nt.signal.signal_propagation_length,
        is_inhibitory: if nt.gsop.is_inhibitory { 1 } else { 0 },
        inertia_curve,
        ahp_amplitude: nt.membrane.ahp_amplitude,
        _pad1: [0; 6],
        adaptive_leak_min_shift: nt.adaptive_leak.adaptive_leak_min_shift,
        adaptive_leak_gain: nt.adaptive_leak.adaptive_leak_gain,
        adaptive_mode: nt.adaptive_leak.adaptive_mode,
        _leak_pad: [0; 3],
        d1_affinity: nt.dopamine.d1_affinity,
        d2_affinity: nt.dopamine.d2_affinity,
        heartbeat_m,
    }
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
    shard_config: &config::ShardConfig,
    seed: u64,
    run_cfg: &RunConfig,
) -> (Vec<MultifieldAxonPath>, Vec<Synapse>) {
    let mut completed_axons = Vec::new();
    let n = topo.somas.len();

    let neuron_types: Vec<&config::NeuronType> = topo
        .somas
        .iter()
        .map(|s| &shard_config.neuron_types[s.variant_id as usize])
        .collect();

    let soma_variants: Vec<u8> = topo.somas.iter().map(|s| s.variant_id).collect();

    for source_id in 0..n {
        let soma = &topo.somas[source_id];
        let neuron_type = neuron_types[source_id];

        let mut branches: Vec<Vec<MultifieldSegment>> = Vec::new();

        // Phase 1: Main stem growth
        let mut main_stem = Vec::new();
        let mut pos = glam::Vec3::new(
            soma.position.x() as f32,
            soma.position.y() as f32,
            soma.position.z() as f32,
        );
        let vertical_bias = neuron_type.growth.growth_vertical_bias;

        let max_steps = 15;
        let mut state = GrowthState::Pathfinding;

        for step in 0..max_steps {
            if state == GrowthState::Terminated {
                break;
            }

            // Check boundary limits
            if pos.x < 0.0
                || pos.x > 16.0
                || pos.y < 0.0
                || pos.y > 16.0
                || pos.z < 0.0
                || pos.z > 32.0
            {
                state = GrowthState::Terminated;
                break;
            }

            // Multi-vector steering
            let bias_vec = glam::Vec3::new(0.0, 0.0, vertical_bias);
            let noise_vec = deterministic_noise(seed, source_id as u32, step) * 0.4;

            // Deflection/Repulsion from nearby somas
            let mut repulsion_vec = glam::Vec3::ZERO;
            for other in &topo.somas {
                if other.soma_id != source_id as u32 {
                    let o_pos = glam::Vec3::new(
                        other.position.x() as f32,
                        other.position.y() as f32,
                        other.position.z() as f32,
                    );
                    let dist = pos.distance(o_pos);
                    if dist < run_cfg.r_repulsion {
                        let dir = pos - o_pos;
                        let force = (run_cfg.r_repulsion - dist) / run_cfg.r_repulsion;
                        repulsion_vec += dir.normalize_or_zero() * force * 1.5;
                    }
                }
            }

            // Fasciculation check
            let mut fascicle_vec = glam::Vec3::ZERO;
            for prev_axon in &completed_axons {
                let prev_path: &MultifieldAxonPath = prev_axon;
                if prev_path.axon_type_id == soma.variant_id {
                    for b in &prev_path.branches {
                        for seg in b {
                            let seg_p = glam::Vec3::new(seg.x, seg.y, seg.z);
                            let d = pos.distance(seg_p);
                            if d < run_cfg.r_fascicle {
                                fascicle_vec +=
                                    (seg_p - pos).normalize_or_zero() * run_cfg.w_fascicle;
                            }
                        }
                    }
                }
            }

            let steer = (bias_vec + noise_vec + repulsion_vec + fascicle_vec).normalize_or_zero();
            pos += steer;

            main_stem.push(MultifieldSegment {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                segment_offset: (step + 1) as u8,
                branch_id: 0,
            });

            // Check transition to target zone
            let target_z = if vertical_bias > 0.0 {
                pos.z >= 16.0
            } else if vertical_bias < 0.0 {
                pos.z <= 16.0
            } else {
                step >= 6
            };

            if target_z {
                state = GrowthState::TerminalArborization;
                break;
            }
        }

        branches.push(main_stem);

        // Terminal Arborization (Branching)
        if state == GrowthState::TerminalArborization {
            let main_end = branches[0].last().cloned();
            if let Some(end_seg) = main_end {
                let num_arbors = run_cfg.max_branches;
                for b_idx in 1..=num_arbors {
                    let mut arbor = Vec::new();
                    let mut b_pos = glam::Vec3::new(end_seg.x, end_seg.y, end_seg.z);

                    for step in 0..run_cfg.max_branch_len {
                        let lateral_bias = glam::Vec3::new(
                            ((source_id + b_idx) % 3) as f32 - 1.0,
                            ((source_id * b_idx) % 3) as f32 - 1.0,
                            (step % 2) as f32 * 0.5 - 0.25,
                        )
                        .normalize_or_zero();

                        let noise =
                            deterministic_noise(seed, (source_id + b_idx * 100) as u32, step) * 0.5;
                        let steer = (lateral_bias + noise).normalize_or_zero();
                        b_pos += steer;

                        arbor.push(MultifieldSegment {
                            x: b_pos.x,
                            y: b_pos.y,
                            z: b_pos.z,
                            segment_offset: (step + 1) as u8,
                            branch_id: b_idx as u32,
                        });
                    }
                    branches.push(arbor);
                }
            }
        }

        completed_axons.push(MultifieldAxonPath {
            soma_id: source_id as u32,
            axon_type_id: soma.variant_id,
            branches,
        });
    }

    // Phase 2: Touch Detection and Capping
    let mut target_candidates: Vec<Vec<Synapse>> = vec![Vec::new(); n];

    for target in &topo.somas {
        let target_type = neuron_types[target.soma_id as usize];
        let target_pos = glam::Vec3::new(
            target.position.x() as f32,
            target.position.y() as f32,
            target.position.z() as f32,
        );

        for axon in &completed_axons {
            let source_id = axon.soma_id;
            let source_type = neuron_types[source_id as usize];

            // Whitelist Check
            if !target_type
                .growth
                .dendrite_whitelist
                .contains(&source_type.name)
            {
                continue;
            }

            // Radius check across segments
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

    let mut accepted_synapses = Vec::new();
    let cap_limit = run_cfg.soft_cap;

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
            let max_per_pair = run_cfg.max_per_pair;
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

                    let beta = run_cfg.beta;
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

        if run_cfg.projection_aware {
            // Priority capping logic
            // 1. Identify duplicates first: sort by source and distance
            selected.sort_by(|a, b| {
                a.source_soma_id.cmp(&b.source_soma_id).then(
                    a.distance_sq
                        .partial_cmp(&b.distance_sq)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
            });

            let mut sorted_with_priority = Vec::new();
            let mut last_source = None;
            let mut count_from_source = 0;
            for syn in selected {
                let is_duplicate = if last_source == Some(syn.source_soma_id) {
                    count_from_source += 1;
                    count_from_source > 1
                } else {
                    last_source = Some(syn.source_soma_id);
                    count_from_source = 1;
                    false
                };

                let sv = soma_variants[syn.source_soma_id as usize];
                let tv = soma_variants[syn.target_soma_id as usize];
                let proj_priority = match (sv, tv) {
                    (1, 3) => 0,                                     // L4->L5 (highest priority)
                    (0, 1) => 1,                                     // Virtual->L4
                    (1, 2) | (2, 1) | (2, 2) | (2, 3) | (3, 2) => 2, // Other expected
                    _ => 3,                                          // Unexpected/Other
                };

                sorted_with_priority.push((syn, is_duplicate, proj_priority));
            }

            // Sort: non-duplicates first, then higher priority first, then closer distance first
            sorted_with_priority.sort_by(|a, b| {
                a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(
                    a.0.distance_sq
                        .partial_cmp(&b.0.distance_sq)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
            });

            let mut dendrite_idx = 0;
            for (mut syn, _, _) in sorted_with_priority {
                if dendrite_idx < cap_limit {
                    syn.dendrite_idx = dendrite_idx as u32;
                    accepted_synapses.push(syn);
                    dendrite_idx += 1;
                }
            }
        } else {
            // Sort by distance_sq
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
    }

    (completed_axons, accepted_synapses)
}

fn build_flat_tuples(
    axons: &[MultifieldAxonPath],
    synapses: &[Synapse],
    topo: &topology::SingleShardTopology,
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
        let axon_idx = syn.source_soma_id as usize;
        let axon = &axons[axon_idx];

        let main_len = axon.branches[0].len();
        let flat_segment_idx = if syn.branch_id == 0 {
            syn.segment_offset as u32 - 1
        } else {
            let mut offset = main_len;
            for b in 1..syn.branch_id as usize {
                offset += axon.branches[b].len();
            }
            (offset + syn.segment_offset as usize - 1) as u32
        };

        let is_inhibitory = topo.somas[syn.source_soma_id as usize].variant_id == 2;
        let raw_w = if is_inhibitory { -1500i32 } else { 1500i32 };
        let w_mass = raw_w << 16;

        flat_synapses.push(FlatSynapse {
            source_soma_id: syn.source_soma_id,
            flat_segment_idx,
            target_soma_id: syn.target_soma_id,
            dendrite_idx: syn.dendrite_idx,
            weight: w_mass,
            fatigue: 0,
        });
    }

    (flat_synapses, flat_axons)
}

fn get_projection_type(src_var: u8, tgt_var: u8) -> String {
    match (src_var, tgt_var) {
        (0, 1) => "Virtual->L4".to_string(),
        (1, 2) => "L4->L23".to_string(),
        (1, 3) => "L4->L5".to_string(),
        (2, 1) => "L23->L4".to_string(),
        (2, 2) => "L23->L23".to_string(),
        (2, 3) => "L23->L5".to_string(),
        (3, 2) => "L5->L23".to_string(),
        _ => "Other".to_string(),
    }
}

// ----------------- Somatic GLIF and Signal Propagation Replay -----------------

struct SomaState {
    id: u32,
    variant_id: u8,
    voltage: i32,
    thresh_offset: i32,
    refractory_timer: u8,
    burst_count: u32,
}

#[derive(Default, Clone, Debug, serde::Serialize)]
struct ReplayMetrics {
    firing_rates: HashMap<String, Vec<f64>>,
    active_fractions: HashMap<String, Vec<f64>>,
    vm_health_above_neg25: usize,
    mean_threshold_distances: HashMap<String, Vec<f64>>,
    silence_ticks: usize,
    runaway_ticks: usize,
    mean_fatigue: Vec<f64>,
}

fn rng_next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed
}

fn run_replay_simulation(
    flat_axons: &[FlatAxon],
    flat_synapses: &mut [FlatSynapse],
    variants: &[VariantParameters],
    soma_variants: &[u8],
    is_learning: bool,
    max_ticks: usize,
    seed_val: u64,
) -> ReplayMetrics {
    let n = soma_variants.len();
    let mut somas = Vec::new();
    for i in 0..n {
        let v_id = soma_variants[i];
        let var = &variants[v_id as usize];
        somas.push(SomaState {
            id: i as u32,
            variant_id: v_id,
            voltage: var.rest_potential,
            thresh_offset: 0,
            refractory_timer: 0,
            burst_count: 0,
        });
    }

    let mut rng_seed = seed_val;

    let mut children = vec![Vec::<Vec<usize>>::new(); n];
    let mut roots = vec![Vec::<usize>::new(); n];
    for axon in flat_axons {
        let axon_idx = axon.soma_id as usize;
        children[axon_idx] = vec![Vec::new(); axon.total_segments];
        for (seg_idx, parent) in axon.parents.iter().enumerate() {
            if let Some(parent_idx) = parent {
                children[axon_idx][*parent_idx].push(seg_idx);
            } else {
                roots[axon_idx].push(seg_idx);
            }
        }
    }
    let empty_active_segments = || {
        let mut buffers = vec![Vec::new(); n];
        for axon in flat_axons {
            buffers[axon.soma_id as usize] = vec![false; axon.total_segments];
        }
        buffers
    };
    let mut active_segments = empty_active_segments();

    // Metrics tracking
    let mut firing_rates = HashMap::new();
    let mut active_fractions = HashMap::new();
    let mut mean_threshold_distances = HashMap::new();

    let layer_names = vec![
        "Virtual".to_string(),
        "L4".to_string(),
        "L23".to_string(),
        "L5".to_string(),
    ];
    for name in &layer_names {
        firing_rates.insert(name.clone(), vec![0.0; max_ticks]);
        active_fractions.insert(name.clone(), vec![0.0; max_ticks]);
        mean_threshold_distances.insert(name.clone(), vec![0.0; max_ticks]);
    }

    let mut active_somas_count = vec![HashSet::new(); 4];
    let mut vm_health_above_neg25 = 0;
    let mut silence_ticks = 0;
    let mut runaway_ticks = 0;
    let mut mean_fatigue = vec![0.0; max_ticks];

    for tick in 0..max_ticks {
        let mut spikes_this_tick = vec![false; n];

        // 1. External Drive / Heartbeat for Virtual
        for i in 0..n {
            if soma_variants[i] == 0 {
                let block = tick % 100;
                let group_a = i < 48;
                let is_stim_spike = if (group_a && block == 0) || (!group_a && block == 50) {
                    true
                } else {
                    rng_next(&mut rng_seed) % 200 == 0 // 0.5%
                };

                if is_stim_spike {
                    spikes_this_tick[i] = true;
                    somas[i].voltage = variants[0]
                        .rest_potential
                        .wrapping_sub(variants[0].ahp_amplitude as i32);
                    somas[i].refractory_timer = variants[0].refractory_period;
                    somas[i].thresh_offset = somas[i]
                        .thresh_offset
                        .wrapping_add(variants[0].homeostasis_penalty);
                }
            }
        }

        // Co-activate matched L4 somas during learning phase
        if is_learning && tick >= 10 {
            let block = (tick - 10) % 100;
            if block == 0 {
                for i in 128..176 {
                    spikes_this_tick[i] = true;
                    somas[i].voltage = variants[1]
                        .rest_potential
                        .wrapping_sub(variants[1].ahp_amplitude as i32);
                    somas[i].refractory_timer = variants[1].refractory_period;
                    somas[i].thresh_offset = somas[i]
                        .thresh_offset
                        .wrapping_add(variants[1].homeostasis_penalty);
                }
            }
        }

        let mut active_heads = vec![[AXON_SENTINEL; 8]; n];
        for source_idx in 0..n {
            let mut head_idx = 0;
            for (seg_idx, is_active) in active_segments[source_idx].iter().enumerate() {
                if *is_active {
                    if head_idx >= active_heads[source_idx].len() {
                        break;
                    }
                    active_heads[source_idx][head_idx] = seg_idx as u32;
                    head_idx += 1;
                }
            }
        }

        // 2. Synaptic current integration
        let mut i_in = vec![0i32; n];
        for syn in flat_synapses.iter_mut() {
            syn.fatigue = physics::recover_fatigue(syn.fatigue);

            let pre_axon = syn.source_soma_id as usize;
            let target_variant = &variants[soma_variants[syn.target_soma_id as usize] as usize];
            let seg_idx = syn.flat_segment_idx as usize;
            if active_segments
                .get(pre_axon)
                .and_then(|segments| segments.get(seg_idx))
                .copied()
                .unwrap_or(false)
            {
                let att_w = physics::apply_synaptic_fatigue(
                    syn.weight,
                    syn.fatigue,
                    target_variant.fatigue_capacity,
                );
                let charge = physics::weight_to_charge(att_w);
                i_in[syn.target_soma_id as usize] =
                    i_in[syn.target_soma_id as usize].wrapping_add(charge);

                syn.fatigue =
                    physics::fatigue_after_spike(syn.fatigue, target_variant.fatigue_capacity);
            }
        }

        // 3. Somatic GLIF updates
        for i in 0..n {
            if soma_variants[i] == 0 {
                if spikes_this_tick[i] {
                    active_somas_count[0].insert(i);
                }
                continue;
            }

            let v_id = soma_variants[i] as usize;
            let variant = &variants[v_id];

            somas[i].thresh_offset = physics::homeostasis_decay(
                somas[i].thresh_offset,
                variant.homeostasis_decay as i32,
            );

            let noise_curr = (rng_next(&mut rng_seed) % 201) as i32 - 100;
            let total_i = i_in[i].wrapping_add(noise_curr);

            let mut is_glif = false;
            if somas[i].refractory_timer > 0 {
                somas[i].refractory_timer -= 1;
            } else {
                let v_new = physics::update_glif_voltage(
                    somas[i].voltage,
                    total_i,
                    variant.rest_potential,
                    somas[i].thresh_offset,
                    variant.leak_shift as i32,
                    variant.adaptive_leak_gain as i32,
                    variant.adaptive_leak_min_shift,
                    variant.adaptive_mode as i32,
                );

                is_glif = physics::is_glif_spike(v_new, variant.threshold, somas[i].thresh_offset);
                if !is_glif {
                    somas[i].voltage = v_new;
                }
            }

            let is_heartbeat = physics::heartbeat_spike(tick as u64, variant.heartbeat_m, i as u32);
            let final_spike = spikes_this_tick[i] || is_glif || is_heartbeat;

            if final_spike {
                spikes_this_tick[i] = true;
                somas[i].voltage = variant
                    .rest_potential
                    .wrapping_sub(variant.ahp_amplitude as i32);
                somas[i].refractory_timer = variant.refractory_period;
                somas[i].thresh_offset = somas[i]
                    .thresh_offset
                    .wrapping_add(variant.homeostasis_penalty);
                somas[i].burst_count = somas[i].burst_count.saturating_add(1);

                active_somas_count[v_id].insert(i);
            }

            if somas[i].voltage > -25_000 {
                vm_health_above_neg25 += 1;
            }
        }

        // 4. GSOP Plasticity Updates
        if is_learning {
            for i in 0..n {
                if spikes_this_tick[i] && soma_variants[i] > 0 {
                    let tgt_variant = &variants[soma_variants[i] as usize];
                    let burst_count = somas[i].burst_count;

                    for syn in flat_synapses.iter_mut() {
                        if syn.target_soma_id == i as u32 {
                            let pre_axon = syn.source_soma_id as usize;
                            let pre_variant = &variants[soma_variants[pre_axon] as usize];

                            let new_w = physics::apply_gsop_plasticity(
                                syn.weight,
                                &active_heads[pre_axon],
                                syn.flat_segment_idx,
                                pre_variant.signal_propagation_length as u32,
                                syn.fatigue,
                                tgt_variant.fatigue_capacity,
                                tgt_variant.gsop_potentiation as i32,
                                tgt_variant.gsop_depression as i32,
                                40,
                                tgt_variant.d1_affinity as i32,
                                tgt_variant.d2_affinity as i32,
                                burst_count.max(1),
                                &tgt_variant.inertia_curve.map(|x| x as i32),
                            );
                            syn.weight = new_w;
                        }
                    }
                }
            }
        }

        // 5. Axonal Propagation
        let mut next_active_segments = empty_active_segments();
        for i in 0..n {
            if spikes_this_tick[i] {
                for &root_idx in &roots[i] {
                    if let Some(root) = next_active_segments[i].get_mut(root_idx) {
                        *root = true;
                    }
                }
            }

            for (seg_idx, is_active) in active_segments[i].iter().enumerate() {
                if *is_active {
                    for &child_idx in &children[i][seg_idx] {
                        if let Some(child) = next_active_segments[i].get_mut(child_idx) {
                            *child = true;
                        }
                    }
                }
            }
        }
        active_segments = next_active_segments;

        // 6. Track Tick Metrics
        let mut spike_counts_layer = [0; 4];
        for i in 0..n {
            if spikes_this_tick[i] {
                spike_counts_layer[soma_variants[i] as usize] += 1;
            }
        }

        let layer_sizes = [128.0, 128.0, 64.0, 64.0];
        let layer_indices = ["Virtual", "L4", "L23", "L5"];

        for v_idx in 0..4 {
            let name = &layer_indices[v_idx];
            let rate = (spike_counts_layer[v_idx] as f64) / layer_sizes[v_idx];
            firing_rates.get_mut(*name).unwrap()[tick] = rate;

            let active_frac = (active_somas_count[v_idx].len() as f64) / layer_sizes[v_idx];
            active_fractions.get_mut(*name).unwrap()[tick] = active_frac;

            let mut sum_dist = 0.0;
            let mut count = 0;
            for i in 0..n {
                if soma_variants[i] as usize == v_idx {
                    let var = &variants[v_idx];
                    let th = (var.threshold + somas[i].thresh_offset) as f64;
                    let dist = th - (somas[i].voltage as f64);
                    sum_dist += dist;
                    count += 1;
                }
            }
            mean_threshold_distances.get_mut(*name).unwrap()[tick] = if count > 0 {
                sum_dist / count as f64
            } else {
                0.0
            };
        }

        let total_spikes: usize = spike_counts_layer.iter().sum();
        if total_spikes == 0 {
            silence_ticks += 1;
        }
        if total_spikes > (n * 30 / 100) {
            runaway_ticks += 1;
        }

        let sum_fatigue: u64 = flat_synapses.iter().map(|s| s.fatigue as u64).sum();
        mean_fatigue[tick] = if !flat_synapses.is_empty() {
            sum_fatigue as f64 / flat_synapses.len() as f64
        } else {
            0.0
        };
    }

    ReplayMetrics {
        firing_rates,
        active_fractions,
        vm_health_above_neg25,
        mean_threshold_distances,
        silence_ticks,
        runaway_ticks,
        mean_fatigue,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct StreamAuditReport {
    biological_axon_count: usize,
    compiled_stream_count: usize,
    streams_with_synapses: usize,
    streams_dropped: usize,
    total_compiled_stream_segments: usize,
    mean_stream_length: f64,
    p90_stream_length: f64,
    max_simultaneous_root_streams_per_soma: usize,
    estimated_runtime_head_count: usize,
}

fn audit_separate_streams(
    axons: &[MultifieldAxonPath],
    synapses: &[Synapse],
    topo: &topology::SingleShardTopology,
) -> StreamAuditReport {
    let n = topo.somas.len();
    let mut compiled_stream_count = 0;
    let mut streams_with_synapses = 0;
    let mut stream_lengths = Vec::new();
    let mut streams_per_soma = vec![0usize; n];

    for axon in axons {
        let axon_id = axon.soma_id as usize;
        let num_branches = axon.branches.len();
        if num_branches == 0 {
            continue;
        }

        let main_len = axon.branches[0].len();
        if num_branches == 1 {
            compiled_stream_count += 1;
            let has_syn = synapses
                .iter()
                .any(|s| s.source_soma_id == axon.soma_id && s.branch_id == 0);
            if has_syn {
                streams_with_synapses += 1;
                stream_lengths.push(main_len);
                streams_per_soma[axon_id] += 1;
            }
        } else {
            let num_streams = num_branches - 1;
            compiled_stream_count += num_streams;
            for s in 0..num_streams {
                let branch_id_target = (s + 1) as u32;
                let has_syn = synapses.iter().any(|syn| {
                    syn.source_soma_id == axon.soma_id
                        && (syn.branch_id == 0 || syn.branch_id == branch_id_target)
                });
                if has_syn {
                    streams_with_synapses += 1;
                    let len = main_len + axon.branches[s + 1].len();
                    stream_lengths.push(len);
                    streams_per_soma[axon_id] += 1;
                }
            }
        }
    }

    let streams_dropped = compiled_stream_count - streams_with_synapses;
    let total_compiled_stream_segments: usize = stream_lengths.iter().sum();
    let mean_stream_length = if !stream_lengths.is_empty() {
        total_compiled_stream_segments as f64 / stream_lengths.len() as f64
    } else {
        0.0
    };

    let mut sorted_lengths = stream_lengths.clone();
    sorted_lengths.sort();
    let p90_stream_length = if !sorted_lengths.is_empty() {
        let p90_idx = (sorted_lengths.len() as f64 * 0.90) as usize;
        sorted_lengths[p90_idx.min(sorted_lengths.len() - 1)] as f64
    } else {
        0.0
    };

    let max_simultaneous_root_streams_per_soma = *streams_per_soma.iter().max().unwrap_or(&0);

    StreamAuditReport {
        biological_axon_count: n,
        compiled_stream_count,
        streams_with_synapses,
        streams_dropped,
        total_compiled_stream_segments,
        mean_stream_length,
        p90_stream_length,
        max_simultaneous_root_streams_per_soma,
        estimated_runtime_head_count: streams_with_synapses,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct SweepResult {
    name: String,
    dendrite_radius_um: f32,
    max_per_pair: usize,
    beta: f32,
    soft_cap: usize,
    projection_aware: bool,
    total_synapses: usize,
    count_v_l4: usize,
    count_l4_l23: usize,
    count_l4_l5: usize,
    count_l23_l4: usize,
    count_l23_l23: usize,
    count_l23_l5: usize,
    count_l5_l23: usize,
    unexpected: usize,
    fan_in_mean: f64,
    fan_in_p50: f64,
    fan_in_p90: f64,
    fan_in_p99: f64,
    fan_in_max: usize,
    saturated_somas: usize,
    out_degree_mean: f64,
    out_degree_p50: f64,
    out_degree_p90: f64,
    out_degree_p99: f64,
    out_degree_max: usize,
    structural_pass: bool,
    stream_audit: StreamAuditReport,
}

fn compute_percentile(vals: &[usize], pct: f64) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    let mut sorted = vals.to_vec();
    sorted.sort();
    let idx = (sorted.len() as f64 * pct) as usize;
    sorted[idx.min(sorted.len() - 1)] as f64
}

#[test]
fn run_growth_v2_fanin_reduction() {
    println!("=== Starting Growth v2 Fan-in Pressure Reduction v0.6 ===");

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

    let n = topo.somas.len();
    let soma_variants: Vec<u8> = topo.somas.iter().map(|s| s.variant_id).collect();

    // Load Variant parameters
    let nt_l4_real = load_variant(find_profile_path("L4_spiny_VISl4_4"));
    let nt_l23_real = load_variant(find_profile_path("L23_aspiny_VISp23_218"));
    let nt_l5_real = load_variant(find_profile_path("L5_spiny_VISp5_7"));
    let mut nt_virtual = nt_l4_real;
    nt_virtual.is_inhibitory = 0;
    let variants = vec![nt_virtual, nt_l4_real, nt_l23_real, nt_l5_real];

    // Define the Sweep Configurations
    let sweep_configs = vec![
        // Baseline (Balanced v0.5)
        RunConfig {
            name: "Baseline_Balanced_v0.5".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        // Sweeps
        // Varying radius (128 cap, max_per_pair=2)
        RunConfig {
            name: "Radius_8_Cap_128_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_7_Cap_128_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_6_Cap_128_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(6.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_5_Cap_128_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(5.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_4_Cap_128_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(4.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        // Varying max_per_pair to 1 (duplicates removed, 128 cap)
        RunConfig {
            name: "Radius_9_Cap_128_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_8_Cap_128_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_7_Cap_128_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_6_Cap_128_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(6.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_5_Cap_128_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(5.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 128,
            projection_aware: false,
        },
        // Varying soft cap to 96
        RunConfig {
            name: "Radius_9_Cap_96_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_8_Cap_96_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_7_Cap_96_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
        // Varying soft cap to 112
        RunConfig {
            name: "Radius_9_Cap_112_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 112,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_8_Cap_112_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 112,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_7_Cap_112_Pair_2".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 112,
            projection_aware: false,
        },
        // Projection-aware sweeps with cap 96
        RunConfig {
            name: "Radius_9_Cap_96_Pair_2_ProjAware".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: true,
        },
        RunConfig {
            name: "Radius_8_Cap_96_Pair_2_ProjAware".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: true,
        },
        RunConfig {
            name: "Radius_7_Cap_96_Pair_2_ProjAware".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: true,
        },
        RunConfig {
            name: "Radius_6_Cap_96_Pair_2_ProjAware".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(6.0),
            max_per_pair: 2,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: true,
        },
        // Pair 1 with cap 96
        RunConfig {
            name: "Radius_9_Cap_96_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(9.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_8_Cap_96_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(8.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
        RunConfig {
            name: "Radius_7_Cap_96_Pair_1".to_string(),
            max_branches: 2,
            max_branch_len: 3,
            w_fascicle: 0.5,
            r_fascicle: 2.5,
            r_repulsion: 1.1,
            override_dendrite_radius: Some(7.0),
            max_per_pair: 1,
            beta: 2.0,
            soft_cap: 96,
            projection_aware: false,
        },
    ];

    let mut sweep_results = Vec::new();

    for (idx, cfg) in sweep_configs.iter().enumerate() {
        println!("Sweeping candidate {}: {}...", idx, cfg.name);
        let (axons, synapses) = run_multifield_simulation(&topo, &shard_config, seed_val, cfg);
        let (flat_syn, flat_ax) = build_flat_tuples(&axons, &synapses, &topo);

        // Calculate expected projections counts
        let mut count_l4_l5 = 0;
        let mut count_v_l4 = 0;
        let mut count_l4_l23 = 0;
        let mut count_l23_l4 = 0;
        let mut count_l23_l23 = 0;
        let mut count_l23_l5 = 0;
        let mut count_l5_l23 = 0;
        let mut unexpected = 0;

        for s in &flat_syn {
            let sv = soma_variants[s.source_soma_id as usize];
            let tv = soma_variants[s.target_soma_id as usize];
            match (sv, tv) {
                (0, 1) => count_v_l4 += 1,
                (1, 2) => count_l4_l23 += 1,
                (1, 3) => count_l4_l5 += 1,
                (2, 1) => count_l23_l4 += 1,
                (2, 2) => count_l23_l23 += 1,
                (2, 3) => count_l23_l5 += 1,
                (3, 2) => count_l5_l23 += 1,
                _ => unexpected += 1,
            }
        }

        let expected_present = count_v_l4 > 0
            && count_l4_l23 > 0
            && count_l4_l5 > 0
            && count_l23_l4 > 0
            && count_l23_l23 > 0
            && count_l23_l5 > 0
            && count_l5_l23 > 0;
        let structural_pass = expected_present && (unexpected == 0);

        // Fan-in stats
        let mut fan_in_counts = vec![0usize; n];
        for s in &flat_syn {
            fan_in_counts[s.target_soma_id as usize] += 1;
        }

        let mut saturated_somas = 0;
        for &fi in &fan_in_counts {
            if fi == 128 {
                saturated_somas += 1;
            }
        }

        let fi_mean = fan_in_counts.iter().sum::<usize>() as f64 / n as f64;
        let fi_p50 = compute_percentile(&fan_in_counts, 0.50);
        let fi_p90 = compute_percentile(&fan_in_counts, 0.90);
        let fi_p99 = compute_percentile(&fan_in_counts, 0.99);
        let fi_max = *fan_in_counts.iter().max().unwrap_or(&0);

        // Out-degree stats
        let mut out_degree_counts = vec![0usize; n];
        for s in &flat_syn {
            out_degree_counts[s.source_soma_id as usize] += 1;
        }
        let od_mean = out_degree_counts.iter().sum::<usize>() as f64 / n as f64;
        let od_p50 = compute_percentile(&out_degree_counts, 0.50);
        let od_p90 = compute_percentile(&out_degree_counts, 0.90);
        let od_p99 = compute_percentile(&out_degree_counts, 0.99);
        let od_max = *out_degree_counts.iter().max().unwrap_or(&0);

        // Run separate-stream compile audit
        let stream_audit = audit_separate_streams(&axons, &synapses, &topo);

        sweep_results.push(SweepResult {
            name: cfg.name.clone(),
            dendrite_radius_um: cfg.override_dendrite_radius.unwrap_or(10.0),
            max_per_pair: cfg.max_per_pair,
            beta: cfg.beta,
            soft_cap: cfg.soft_cap,
            projection_aware: cfg.projection_aware,
            total_synapses: flat_syn.len(),
            count_v_l4,
            count_l4_l23,
            count_l4_l5,
            count_l23_l4,
            count_l23_l23,
            count_l23_l5,
            count_l5_l23,
            unexpected,
            fan_in_mean: fi_mean,
            fan_in_p50: fi_p50,
            fan_in_p90: fi_p90,
            fan_in_p99: fi_p99,
            fan_in_max: fi_max,
            saturated_somas,
            out_degree_mean: od_mean,
            out_degree_p50: od_p50,
            out_degree_p90: od_p90,
            out_degree_p99: od_p99,
            out_degree_max: od_max,
            structural_pass,
            stream_audit,
        });
    }

    // Select winner candidates programmatically from structural_pass == true
    let mut structural_winners = Vec::new();
    for (idx, res) in sweep_results.iter().enumerate() {
        if idx == 0 {
            continue; // Skip baseline
        }
        if res.structural_pass && res.count_l4_l5 > 0 {
            structural_winners.push((idx, res));
        }
    }

    // Sort winners:
    // 1st priority: p90 fan-in (ascending)
    // 2nd priority: saturated target somas (ascending)
    // 3rd priority: total accepted synapses (descending)
    structural_winners.sort_by(|a, b| {
        a.1.fan_in_p90
            .partial_cmp(&b.1.fan_in_p90)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.saturated_somas.cmp(&b.1.saturated_somas))
            .then(b.1.total_synapses.cmp(&a.1.total_synapses))
    });

    println!("\n=== Parameter Sweep Structural Winners ===");
    for (rank, &(idx, res)) in structural_winners.iter().enumerate() {
        println!("  Rank {}: Winner Candidate Index = {}, Name = {}, Saturated Somas = {}, Fan-in p90 = {}, L4->L5 = {}",
                 rank + 1, idx, res.name, res.saturated_somas, res.fan_in_p90, res.count_l4_l5);
    }

    // Determine the baseline and top 2 winners to run functional replays on
    let baseline_idx = 0;
    let baseline_cfg = &sweep_configs[baseline_idx];
    let baseline_res = &sweep_results[baseline_idx];

    let mut winner_1_idx = None;
    let mut winner_2_idx = None;

    if !structural_winners.is_empty() {
        winner_1_idx = Some(structural_winners[0].0);
    }
    if structural_winners.len() > 1 {
        winner_2_idx = Some(structural_winners[1].0);
    }

    // Fallbacks if no structural winners found
    let w1_idx = winner_1_idx.unwrap_or_else(|| {
        println!("WARNING: No structural winners found! Falling back to Config 17 (ProjAware 96).");
        17
    });
    let w2_idx = winner_2_idx.unwrap_or_else(|| {
        println!(
            "WARNING: Only 1 structural winner found! Falling back to Config 7 (Radius 8, Pair 1)."
        );
        7
    });

    let w1_cfg = &sweep_configs[w1_idx];
    let w2_cfg = &sweep_configs[w2_idx];

    println!("\n=== Replay Config Selection ===");
    println!("  Baseline: {}", baseline_cfg.name);
    println!("  Winner 1: {}", w1_cfg.name);
    println!("  Winner 2: {}", w2_cfg.name);

    // Run simulations for selected replays
    let max_ticks = 10000;
    let mut replays_data = HashMap::new();
    let mut gsop_results_data = HashMap::new();

    let mut run_replays_for_cfg = |cfg: &RunConfig, idx: usize| {
        println!("\nGenerating flat topology for Replay on {}...", cfg.name);
        let (axons, synapses) = run_multifield_simulation(&topo, &shard_config, seed_val, cfg);
        let (mut flat_syn_gsop, flat_ax) = build_flat_tuples(&axons, &synapses, &topo);
        let mut flat_syn_static = flat_syn_gsop.clone();

        println!("Running static simulation for {}...", cfg.name);
        let static_met = run_replay_simulation(
            &flat_ax,
            &mut flat_syn_static,
            &variants,
            &soma_variants,
            false,
            max_ticks,
            seed_val,
        );

        println!("Running GSOP simulation for {}...", cfg.name);
        let gsop_met = run_replay_simulation(
            &flat_ax,
            &mut flat_syn_gsop,
            &variants,
            &soma_variants,
            true,
            max_ticks,
            seed_val,
        );

        // Verify GSOP Changes
        let mut total_delta = 0i64;
        let mut matched_delta = 0i64;
        let mut matched_count = 0;
        let mut unmatched_delta = 0i64;
        let mut unmatched_count = 0;
        let mut sign_violations = 0;

        for (f, i) in flat_syn_gsop.iter().zip(flat_syn_static.iter()) {
            let is_inhibitory = soma_variants[f.source_soma_id as usize] == 2;
            if is_inhibitory {
                if f.weight > 0 {
                    sign_violations += 1;
                }
            } else {
                if f.weight < 0 {
                    sign_violations += 1;
                }
            }

            let delta = (f.weight.abs() - i.weight.abs()) as i64;
            total_delta += delta.abs();

            let is_matched =
                f.source_soma_id < 48 && (f.target_soma_id >= 128 && f.target_soma_id < 176);
            let is_unmatched = f.source_soma_id >= 48
                && f.source_soma_id < 128
                && (f.target_soma_id >= 128 && f.target_soma_id < 176);

            if is_matched {
                matched_delta += delta;
                matched_count += 1;
            } else if is_unmatched {
                unmatched_delta += delta;
                unmatched_count += 1;
            }
        }

        let m_mean = if matched_count > 0 {
            matched_delta as f64 / matched_count as f64
        } else {
            0.0
        };
        let u_mean = if unmatched_count > 0 {
            unmatched_delta as f64 / unmatched_count as f64
        } else {
            0.0
        };

        println!("{}: GSOP Weight verification: total_delta_abs={}, sign_violations={}, matched_mean={:.4}, unmatched_mean={:.4}",
                 cfg.name, total_delta, sign_violations, m_mean, u_mean);

        assert_eq!(
            sign_violations, 0,
            "{} has Dale's law sign violations!",
            cfg.name
        );
        assert!(
            total_delta > 0,
            "{} must show nonzero weight plasticity!",
            cfg.name
        );
        assert!(
            m_mean > u_mean,
            "{} matched mean ({:.4}) must exceed unmatched mean ({:.4})!",
            cfg.name,
            m_mean,
            u_mean
        );

        // Verification gates checks
        assert!(
            static_met.silence_ticks < max_ticks - 100,
            "{} collapsed into total silence!",
            cfg.name
        );
        assert!(
            static_met.runaway_ticks < max_ticks / 2,
            "{} collapsed into pathological runaway!",
            cfg.name
        );

        let final_synapses_json = flat_syn_gsop
            .iter()
            .map(|s| {
                serde_json::json!({
                    "source": s.source_soma_id,
                    "target": s.target_soma_id,
                    "weight": s.weight,
                    "fatigue": s.fatigue,
                    "type": get_projection_type(soma_variants[s.source_soma_id as usize], soma_variants[s.target_soma_id as usize])
                })
            })
            .collect::<Vec<_>>();

        replays_data.insert(
            cfg.name.clone(),
            serde_json::json!({
                "static_firing": static_met.firing_rates,
                "static_active": static_met.active_fractions,
                "static_vm": static_met.mean_threshold_distances,
                "static_fatigue": static_met.mean_fatigue,
                "static_silence_ticks": static_met.silence_ticks,
                "static_runaway_ticks": static_met.runaway_ticks,
                "static_vm_above_neg25": static_met.vm_health_above_neg25,
                "gsop_firing": gsop_met.firing_rates,
                "gsop_active": gsop_met.active_fractions,
                "gsop_vm": gsop_met.mean_threshold_distances,
                "gsop_fatigue": gsop_met.mean_fatigue,
                "gsop_synapses": final_synapses_json,
            }),
        );

        gsop_results_data.insert(
            cfg.name.clone(),
            serde_json::json!({
                "total_delta_abs": total_delta,
                "matched_mean": m_mean,
                "unmatched_mean": u_mean,
            }),
        );
    };

    run_replays_for_cfg(baseline_cfg, baseline_idx);
    run_replays_for_cfg(w1_cfg, w1_idx);
    run_replays_for_cfg(w2_cfg, w2_idx);

    // Serialize plot data and save artifacts
    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.push("docs");
    artifacts_dir.push("engine");
    artifacts_dir.push("research");
    artifacts_dir.push("archive");
    artifacts_dir.push("2026-07-06_growth_v2_fanin_reduction_v0_6");
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let output_path = artifacts_dir.join("growth_v2_fanin_reduction_plot_data.json");
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
        "sweep": sweep_results,
        "replays": replays_data,
        "gsop_results": gsop_results_data,
        "baseline_name": baseline_cfg.name,
        "winner_1_name": w1_cfg.name,
        "winner_2_name": w2_cfg.name,
    });

    serde_json::to_writer_pretty(file, &plot_json).unwrap();
    println!("Wrote detailed plot data to {}", output_path.display());
    println!("=== Growth v2 Fan-in Pressure Reduction v0.6 Test Complete ===");
}
