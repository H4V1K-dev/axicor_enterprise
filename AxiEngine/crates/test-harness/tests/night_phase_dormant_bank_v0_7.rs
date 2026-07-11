#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]
#![allow(
    clippy::needless_range_loop,
    dead_code,
    unused_variables,
    clippy::manual_is_multiple_of
)]

use layout::VariantParameters;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use types::constants::AXON_SENTINEL;
use types::MasterSeed;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct Synapse {
    source_soma_id: u32,
    target_soma_id: u32,
    branch_id: u32,
    segment_offset: u8,
    distance_sq: f32,
    dendrite_idx: u32,
}

#[derive(Debug, Clone)]
struct FlatSynapse {
    source_soma_id: u32,
    flat_segment_idx: u32,
    target_soma_id: u32,
    dendrite_idx: u32,
    weight: i32,
    fatigue: u8,
    pre_hits: u16,
    coactivity_hits: u16,
    weight_trend: i8,
    short_trace: u16,
    long_trace: u16,
    age_or_grace: u8,
    pre_trace_timer: u8,
    initial_weight: i32,
}

#[derive(Debug, Clone)]
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

struct SomaState {
    id: u32,
    variant_id: u8,
    voltage: i32,
    thresh_offset: i32,
    refractory_timer: u8,
    burst_count: u32,
    spike_count: u32,
}

#[derive(Default, Clone, Debug)]
struct ReplayMetrics {
    firing_rates: HashMap<String, Vec<f64>>,
    active_fractions: HashMap<String, Vec<f64>>,
    vm_health_above_neg25: usize,
    mean_threshold_distances: HashMap<String, Vec<f64>>,
    silence_ticks: usize,
    runaway_ticks: usize,
    mean_fatigue: Vec<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DormantSynapse {
    source_soma_id: u32,
    target_soma_id: u32,
    flat_segment_idx: u32,
    weight: i32,
    long_trace: u16,
    short_trace: u16,
    dormant_age: u32,
    projection_class: String,
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

            let bias_vec = glam::Vec3::new(0.0, 0.0, vertical_bias);
            let noise_vec = deterministic_noise(seed, source_id as u32, step) * 0.4;

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

            if !target_type
                .growth
                .dendrite_whitelist
                .contains(&source_type.name)
            {
                continue;
            }

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
                    (1, 3) => 0,                                     // L4->L5
                    (0, 1) => 1,                                     // Virtual->L4
                    (1, 2) | (2, 1) | (2, 2) | (2, 3) | (3, 2) => 2, // Other expected
                    _ => 3,                                          // Unexpected
                };

                sorted_with_priority.push((syn, is_duplicate, proj_priority));
            }

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
            pre_hits: 0,
            coactivity_hits: 0,
            weight_trend: 0,
            short_trace: 0,
            long_trace: 0,
            age_or_grace: 0,
            pre_trace_timer: 0,
            initial_weight: w_mass,
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

fn rng_next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed
}

struct SimulationRunner<'a> {
    flat_axons: &'a [FlatAxon],
    flat_synapses: &'a mut Vec<FlatSynapse>,
    variants: &'a [VariantParameters],
    soma_variants: &'a [u8],
    somas: Vec<SomaState>,
    active_segments: Vec<Vec<bool>>,
    children: Vec<Vec<Vec<usize>>>,
    roots: Vec<Vec<usize>>,
    rng_seed: u64,
}

impl<'a> SimulationRunner<'a> {
    fn new(
        flat_axons: &'a [FlatAxon],
        flat_synapses: &'a mut Vec<FlatSynapse>,
        variants: &'a [VariantParameters],
        soma_variants: &'a [u8],
        seed_val: u64,
    ) -> Self {
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
                spike_count: 0,
            });
        }

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

        let mut active_segments = vec![Vec::new(); n];
        for axon in flat_axons {
            active_segments[axon.soma_id as usize] = vec![false; axon.total_segments];
        }

        Self {
            flat_axons,
            flat_synapses,
            variants,
            soma_variants,
            somas,
            active_segments,
            children,
            roots,
            rng_seed: seed_val,
        }
    }

    fn run_day(
        &mut self,
        max_ticks: usize,
        is_learning: bool,
        collect_counters: bool,
    ) -> ReplayMetrics {
        let n = self.soma_variants.len();
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

            for i in 0..n {
                if self.soma_variants[i] == 0 {
                    let block = tick % 100;
                    let group_a = i < 48;
                    let is_stim_spike = if (group_a && block == 0) || (!group_a && block == 50) {
                        true
                    } else {
                        rng_next(&mut self.rng_seed) % 200 == 0
                    };

                    if is_stim_spike {
                        spikes_this_tick[i] = true;
                        self.somas[i].voltage = self.variants[0]
                            .rest_potential
                            .wrapping_sub(self.variants[0].ahp_amplitude as i32);
                        self.somas[i].refractory_timer = self.variants[0].refractory_period;
                        self.somas[i].thresh_offset = self.somas[i]
                            .thresh_offset
                            .wrapping_add(self.variants[0].homeostasis_penalty);
                    }
                }
            }

            if is_learning && tick >= 10 {
                let block = (tick - 10) % 100;
                if block == 0 {
                    for i in 128..176 {
                        spikes_this_tick[i] = true;
                        self.somas[i].voltage = self.variants[1]
                            .rest_potential
                            .wrapping_sub(self.variants[1].ahp_amplitude as i32);
                        self.somas[i].refractory_timer = self.variants[1].refractory_period;
                        self.somas[i].thresh_offset = self.somas[i]
                            .thresh_offset
                            .wrapping_add(self.variants[1].homeostasis_penalty);
                    }
                }
            }

            let mut active_heads = vec![[AXON_SENTINEL; 8]; n];
            for source_idx in 0..n {
                let mut head_idx = 0;
                for (seg_idx, is_active) in self.active_segments[source_idx].iter().enumerate() {
                    if *is_active {
                        if head_idx >= active_heads[source_idx].len() {
                            break;
                        }
                        active_heads[source_idx][head_idx] = seg_idx as u32;
                        head_idx += 1;
                    }
                }
            }

            let mut i_in = vec![0i32; n];
            for syn in self.flat_synapses.iter_mut() {
                syn.fatigue = physics::recover_fatigue(syn.fatigue);

                let pre_axon = syn.source_soma_id as usize;
                let target_variant =
                    &self.variants[self.soma_variants[syn.target_soma_id as usize] as usize];
                let seg_idx = syn.flat_segment_idx as usize;
                let is_hit = self
                    .active_segments
                    .get(pre_axon)
                    .and_then(|segments| segments.get(seg_idx))
                    .copied()
                    .unwrap_or(false);

                if is_hit {
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

                    if collect_counters {
                        syn.pre_hits = syn.pre_hits.saturating_add(1);
                        syn.pre_trace_timer = 8;
                    }
                }
            }

            for i in 0..n {
                if self.soma_variants[i] == 0 {
                    if spikes_this_tick[i] {
                        active_somas_count[0].insert(i);
                        if collect_counters {
                            self.somas[i].spike_count = self.somas[i].spike_count.saturating_add(1);
                        }
                    }
                    continue;
                }

                let v_id = self.soma_variants[i] as usize;
                let variant = &self.variants[v_id];

                self.somas[i].thresh_offset = physics::homeostasis_decay(
                    self.somas[i].thresh_offset,
                    variant.homeostasis_decay as i32,
                );

                let noise_curr = (rng_next(&mut self.rng_seed) % 201) as i32 - 100;
                let total_i = i_in[i].wrapping_add(noise_curr);

                let mut is_glif = false;
                if self.somas[i].refractory_timer > 0 {
                    self.somas[i].refractory_timer -= 1;
                } else {
                    let v_new = physics::update_glif_voltage(
                        self.somas[i].voltage,
                        total_i,
                        variant.rest_potential,
                        self.somas[i].thresh_offset,
                        variant.leak_shift as i32,
                        variant.adaptive_leak_gain as i32,
                        variant.adaptive_leak_min_shift,
                        variant.adaptive_mode as i32,
                    );

                    is_glif = physics::is_glif_spike(
                        v_new,
                        variant.threshold,
                        self.somas[i].thresh_offset,
                    );
                    if !is_glif {
                        self.somas[i].voltage = v_new;
                    }
                }

                let is_heartbeat =
                    physics::heartbeat_spike(tick as u64, variant.heartbeat_m, i as u32);
                let final_spike = spikes_this_tick[i] || is_glif || is_heartbeat;

                if final_spike {
                    spikes_this_tick[i] = true;
                    self.somas[i].voltage = variant
                        .rest_potential
                        .wrapping_sub(variant.ahp_amplitude as i32);
                    self.somas[i].refractory_timer = variant.refractory_period;
                    self.somas[i].thresh_offset = self.somas[i]
                        .thresh_offset
                        .wrapping_add(variant.homeostasis_penalty);
                    self.somas[i].burst_count = self.somas[i].burst_count.saturating_add(1);

                    active_somas_count[v_id].insert(i);

                    if collect_counters {
                        self.somas[i].spike_count = self.somas[i].spike_count.saturating_add(1);
                    }
                }

                if self.somas[i].voltage > -25_000 {
                    vm_health_above_neg25 += 1;
                }
            }

            if collect_counters {
                for syn in self.flat_synapses.iter_mut() {
                    if spikes_this_tick[syn.target_soma_id as usize] && syn.pre_trace_timer > 0 {
                        syn.coactivity_hits = syn.coactivity_hits.saturating_add(1);
                    }
                }
            }

            if is_learning {
                for i in 0..n {
                    if spikes_this_tick[i] && self.soma_variants[i] > 0 {
                        let tgt_variant = &self.variants[self.soma_variants[i] as usize];
                        let burst_count = self.somas[i].burst_count;

                        for syn in self.flat_synapses.iter_mut() {
                            if syn.target_soma_id == i as u32 {
                                let pre_axon = syn.source_soma_id as usize;
                                let pre_variant =
                                    &self.variants[self.soma_variants[pre_axon] as usize];

                                let old_w = syn.weight;
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

                                if collect_counters {
                                    let old_abs = old_w.unsigned_abs();
                                    let new_abs = new_w.unsigned_abs();
                                    if new_abs > old_abs {
                                        syn.weight_trend = syn.weight_trend.saturating_add(1);
                                    } else if new_abs < old_abs {
                                        syn.weight_trend = syn.weight_trend.saturating_sub(1);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if collect_counters {
                for syn in self.flat_synapses.iter_mut() {
                    if syn.pre_trace_timer > 0 {
                        syn.pre_trace_timer -= 1;
                    }
                }
            }

            let mut next_active_segments = vec![Vec::new(); n];
            for axon in self.flat_axons {
                next_active_segments[axon.soma_id as usize] = vec![false; axon.total_segments];
            }

            for i in 0..n {
                if spikes_this_tick[i] {
                    for &root_idx in &self.roots[i] {
                        if let Some(root) = next_active_segments[i].get_mut(root_idx) {
                            *root = true;
                        }
                    }
                }

                for (seg_idx, is_active) in self.active_segments[i].iter().enumerate() {
                    if *is_active {
                        for &child_idx in &self.children[i][seg_idx] {
                            if let Some(child) = next_active_segments[i].get_mut(child_idx) {
                                *child = true;
                            }
                        }
                    }
                }
            }
            self.active_segments = next_active_segments;

            let mut spike_counts_layer = [0; 4];
            for i in 0..n {
                if spikes_this_tick[i] {
                    spike_counts_layer[self.soma_variants[i] as usize] += 1;
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
                    if self.soma_variants[i] as usize == v_idx {
                        let var = &self.variants[v_idx];
                        let th = (var.threshold + self.somas[i].thresh_offset) as f64;
                        let dist = th - (self.somas[i].voltage as f64);
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

            let sum_fatigue: u64 = self.flat_synapses.iter().map(|s| s.fatigue as u64).sum();
            mean_fatigue[tick] = if !self.flat_synapses.is_empty() {
                sum_fatigue as f64 / self.flat_synapses.len() as f64
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

    fn execute_night(&mut self, merge_traces: bool) {
        let n = self.soma_variants.len();

        for i in 0..n {
            let v_id = self.soma_variants[i];
            let var = &self.variants[v_id as usize];
            self.somas[i].voltage = var.rest_potential;
            self.somas[i].thresh_offset = 0;
            self.somas[i].refractory_timer = 0;
            self.somas[i].burst_count = 0;
            self.somas[i].spike_count = 0;
        }

        for axon in self.flat_axons {
            self.active_segments[axon.soma_id as usize] = vec![false; axon.total_segments];
        }

        for syn in self.flat_synapses.iter_mut() {
            syn.fatigue = 0;
        }

        if merge_traces {
            let k_short = 2; // K_SHORT
            let k_long = 5; // K_LONG
            let theta_capture = 5; // THETA_CAPTURE
            let capture_shift = 1; // CAPTURE_SHIFT

            for syn in self.flat_synapses.iter_mut() {
                syn.short_trace = syn.short_trace.saturating_sub(syn.short_trace >> k_short);
                syn.long_trace = syn.long_trace.saturating_sub(syn.long_trace >> k_long);

                syn.short_trace = syn.short_trace.saturating_add(syn.coactivity_hits);

                if syn.short_trace >= theta_capture {
                    syn.long_trace = syn
                        .long_trace
                        .saturating_add(syn.short_trace >> capture_shift);
                }

                syn.pre_hits = 0;
                syn.coactivity_hits = 0;
                syn.weight_trend = 0;
                syn.pre_trace_timer = 0;
            }
        }
    }
}

// ----------------- Metrics Calculator and Violations check -----------------

#[derive(Debug, Clone)]
struct MetricResults {
    total_synapses: usize,
    projections: HashMap<String, usize>,
    matched_mean_delta: f64,
    unmatched_mean_delta: f64,
    matched_bias: f64,
    dale_violations: usize,
    dense_violations: usize,
    duplicate_violations: usize,
    fan_in_p50: f64,
    fan_in_p90: f64,
    fan_in_p99: f64,
    fan_in_max: usize,
    saturated_target_count: usize,
    silence_ticks: usize,
    runaway_ticks: usize,
}

fn compute_metrics(
    flat_synapses: &[FlatSynapse],
    soma_variants: &[u8],
    replay_metrics: &ReplayMetrics,
) -> MetricResults {
    let total_synapses = flat_synapses.len();
    let n = soma_variants.len();

    let mut projections = HashMap::new();
    let expected_proj_pairs = vec![
        ("Virtual->L4", 0, 1),
        ("L4->L23", 1, 2),
        ("L4->L5", 1, 3),
        ("L23->L4", 2, 1),
        ("L23->L23", 2, 2),
        ("L23->L5", 2, 3),
        ("L5->L23", 3, 2),
    ];
    for &(name, _, _) in &expected_proj_pairs {
        projections.insert(name.to_string(), 0);
    }

    let mut dale_violations = 0;
    let mut matched_delta = 0i64;
    let mut matched_count = 0;
    let mut unmatched_delta = 0i64;
    let mut unmatched_count = 0;

    let mut fan_in_counts = vec![0usize; n];
    let mut by_target: Vec<Vec<FlatSynapse>> = vec![Vec::new(); n];

    for syn in flat_synapses {
        by_target[syn.target_soma_id as usize].push(syn.clone());
        fan_in_counts[syn.target_soma_id as usize] += 1;
    }

    let mut dense_violations = 0;
    let mut duplicate_violations = 0;

    for target_id in 0..n {
        let incoming = &by_target[target_id];
        let k = incoming.len();
        if k == 0 {
            continue;
        }

        let mut idxs: Vec<u32> = incoming.iter().map(|s| s.dendrite_idx).collect();
        idxs.sort();
        let expected: Vec<u32> = (0..k as u32).collect();
        if idxs != expected {
            dense_violations += 1;
        }

        let mut source_counts = HashMap::new();
        let mut exact_slots = HashSet::new();
        for syn in incoming {
            *source_counts.entry(syn.source_soma_id).or_insert(0) += 1;
            if !exact_slots.insert((syn.source_soma_id, syn.flat_segment_idx)) {
                duplicate_violations += 1;
            }
        }
        for (&src, &count) in &source_counts {
            if count > 2 {
                duplicate_violations += 1;
            }
        }
    }

    for (idx, syn) in flat_synapses.iter().enumerate() {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj_name = get_projection_type(sv, tv);
        *projections.entry(proj_name).or_insert(0) += 1;

        let delta = syn.weight.abs() as i64 - syn.initial_weight.abs() as i64;
        let is_matched =
            syn.source_soma_id < 48 && (syn.target_soma_id >= 128 && syn.target_soma_id < 176);
        let is_unmatched = syn.source_soma_id >= 48
            && syn.source_soma_id < 128
            && (syn.target_soma_id >= 128 && syn.target_soma_id < 176);

        if is_matched {
            matched_delta += delta;
            matched_count += 1;
        } else if is_unmatched {
            unmatched_delta += delta;
            unmatched_count += 1;
        }

        let src_inhibitory = sv == 2;
        if src_inhibitory && syn.weight > 0 {
            dale_violations += 1;
        }
        if !src_inhibitory && syn.weight < 0 {
            dale_violations += 1;
        }
    }

    let matched_mean_delta = if matched_count > 0 {
        matched_delta as f64 / matched_count as f64
    } else {
        0.0
    };
    let unmatched_mean_delta = if unmatched_count > 0 {
        unmatched_delta as f64 / unmatched_count as f64
    } else {
        0.0
    };

    let mut sorted_fan_in = fan_in_counts.clone();
    sorted_fan_in.sort();
    let f_len = sorted_fan_in.len();

    let fan_in_p50 = sorted_fan_in[f_len / 2] as f64;
    let fan_in_p90 = sorted_fan_in[(f_len * 9) / 10] as f64;
    let fan_in_p99 = sorted_fan_in[(f_len * 99) / 100] as f64;
    let fan_in_max = *sorted_fan_in.last().unwrap_or(&0);

    let saturated_target_count = fan_in_counts.iter().filter(|&&c| c >= 96).count();

    MetricResults {
        total_synapses,
        projections,
        matched_mean_delta,
        unmatched_mean_delta,
        matched_bias: matched_mean_delta - unmatched_mean_delta,
        dale_violations,
        dense_violations,
        duplicate_violations,
        fan_in_p50,
        fan_in_p90,
        fan_in_p99,
        fan_in_max,
        saturated_target_count,
        silence_ticks: replay_metrics.silence_ticks,
        runaway_ticks: replay_metrics.runaway_ticks,
    }
}

fn compute_full_cohort_metrics(
    initial_synapses: &[FlatSynapse],
    surviving_synapses: &[FlatSynapse],
    soma_variants: &[u8],
) -> f64 {
    let mut survivor_map = HashMap::new();
    for syn in surviving_synapses {
        survivor_map.insert(
            (syn.source_soma_id, syn.target_soma_id, syn.flat_segment_idx),
            syn.weight,
        );
    }

    let mut matched_delta = 0i64;
    let mut matched_count = 0;
    let mut unmatched_delta = 0i64;
    let mut unmatched_count = 0;

    for syn in initial_synapses {
        let post_w_abs = if let Some(&w) =
            survivor_map.get(&(syn.source_soma_id, syn.target_soma_id, syn.flat_segment_idx))
        {
            w.abs() as i64
        } else {
            0i64
        };
        let delta = post_w_abs - syn.initial_weight.abs() as i64;

        let is_matched =
            syn.source_soma_id < 48 && (syn.target_soma_id >= 128 && syn.target_soma_id < 176);
        let is_unmatched = syn.source_soma_id >= 48
            && syn.source_soma_id < 128
            && (syn.target_soma_id >= 128 && syn.target_soma_id < 176);

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
    m_mean - u_mean
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

#[derive(serde::Serialize)]
struct PolicyPlotDataV07 {
    name: String,
    active_day1: usize,
    active_day2: usize,
    active_day4: usize,
    dormant_day2: usize,
    dormant_day4: usize,
    deleted_day2: usize,
    deleted_day4: usize,
    matched_retention_day2_full: f64,
    matched_retention_day4_full: f64,
    matched_retention_day2_surv: f64,
    matched_retention_day4_surv: f64,
}

#[derive(serde::Serialize)]
struct DormantTraceData {
    long_trace: u16,
    age: u32,
}

#[derive(serde::Serialize)]
struct PlottingDataV07 {
    policies: Vec<PolicyPlotDataV07>,
    dormant_traces: Vec<DormantTraceData>,
}

#[test]
fn run_night_phase_dormant_bank_v0_7() {
    println!("=== Starting Night Phase Dormant/Cold Bank Stress Test v0.7 ===");

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

    let n_somas = topo.somas.len();
    let soma_variants: Vec<u8> = topo.somas.iter().map(|s| s.variant_id).collect();

    let nt_l4_real = load_variant(find_profile_path("L4_spiny_VISl4_4"));
    let nt_l23_real = load_variant(find_profile_path("L23_aspiny_VISp23_218"));
    let nt_l5_real = load_variant(find_profile_path("L5_spiny_VISp5_7"));
    let mut nt_virtual = nt_l4_real;
    nt_virtual.is_inhibitory = 0;
    let variants = vec![nt_virtual, nt_l4_real, nt_l23_real, nt_l5_real];

    let run_cfg = RunConfig {
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
    };

    let (axons, synapses) = run_multifield_simulation(&topo, &shard_config, seed_val, &run_cfg);
    let (mut flat_synapses, flat_axons) = build_flat_tuples(&axons, &synapses, &topo);

    let max_ticks = 10000;

    println!("Initial synapse count: {}", flat_synapses.len());

    // Day 1 Learning
    println!("Running Day 1 Learning...");
    {
        let mut runner = SimulationRunner::new(
            &flat_axons,
            &mut flat_synapses,
            &variants,
            &soma_variants,
            seed_val,
        );
        runner.run_day(max_ticks, true, true);
        runner.execute_night(true); // Night 1 trace merge
    }

    let initial_synapses_day1 = flat_synapses.clone();
    let mut initial_target_proj_counts = HashMap::new();
    for syn in &initial_synapses_day1 {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj = get_projection_type(sv, tv);
        *initial_target_proj_counts
            .entry((syn.target_soma_id, proj))
            .or_insert(0) += 1;
    }

    // Determine absolute floor budget limit (weight < 1498 * 65536)
    let budget_total = initial_synapses_day1
        .iter()
        .filter(|s| s.weight.abs() < 1498 << 16)
        .count();
    println!(
        "Budget total (based on absolute floor < 1498): {}",
        budget_total
    );

    // Baseline stats pre-night 1
    let dummy_metrics = ReplayMetrics::default();
    let pre_night_results = compute_metrics(&initial_synapses_day1, &soma_variants, &dummy_metrics);
    println!(
        "Pre-night 1 matched bias: {:.4}",
        pre_night_results.matched_bias
    );

    // Score-based Pruning score calculations (from trace-aware rule in v0.6)
    let mut prune_scores = Vec::new();
    let mut proj_pre_hits: HashMap<String, Vec<u16>> = HashMap::new();
    let mut proj_co_ratios: HashMap<String, Vec<u32>> = HashMap::new();
    for syn in &initial_synapses_day1 {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj = get_projection_type(sv, tv);
        proj_pre_hits
            .entry(proj.clone())
            .or_default()
            .push(syn.pre_hits);
        let ratio_pct = if syn.pre_hits > 0 {
            (syn.coactivity_hits as u32 * 1000) / (syn.pre_hits as u32)
        } else {
            0
        };
        proj_co_ratios.entry(proj).or_default().push(ratio_pct);
    }
    let mut proj_p25 = HashMap::new();
    let mut proj_p75 = HashMap::new();
    for (proj, hits) in proj_pre_hits {
        let hits_usize: Vec<usize> = hits.into_iter().map(|x| x as usize).collect();
        proj_p25.insert(proj, compute_percentile(&hits_usize, 0.25) as u16);
    }
    for (proj, ratios) in proj_co_ratios {
        let ratios_usize: Vec<usize> = ratios.into_iter().map(|x| x as usize).collect();
        proj_p75.insert(proj, compute_percentile(&ratios_usize, 0.75) as u32);
    }

    let mut target_proj_useful_count = HashMap::new();
    for syn in &initial_synapses_day1 {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj = get_projection_type(sv, tv);
        let coactivity_ratio_pct = if syn.pre_hits > 0 {
            (syn.coactivity_hits as u32 * 1000) / (syn.pre_hits as u32)
        } else {
            0
        };
        if coactivity_ratio_pct >= 400 || syn.long_trace >= 20 {
            *target_proj_useful_count
                .entry((syn.target_soma_id, proj))
                .or_insert(0) += 1;
        }
    }

    for (idx, syn) in initial_synapses_day1.iter().enumerate() {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj = get_projection_type(sv, tv);

        let low_weight_score = (1600 - (syn.weight.abs() >> 16)).max(0);
        let coactivity_ratio_pct = if syn.pre_hits > 0 {
            (syn.coactivity_hits as u32 * 1000) / (syn.pre_hits as u32)
        } else {
            0
        };
        let low_coactivity_score = 1000i32.saturating_sub(coactivity_ratio_pct as i32);
        let low_trace_score = (100 - syn.long_trace as i32).max(0) * 10;
        let negative_trend_score = if syn.weight_trend < 0 {
            -syn.weight_trend as i32 * 8
        } else {
            0
        };

        let mut protection_bonus = 0;
        if coactivity_ratio_pct >= 400 {
            protection_bonus += 500;
        }
        if syn.long_trace >= 20 {
            protection_bonus += 500;
        }
        let p25 = *proj_p25.get(&proj).unwrap_or(&0);
        let p75 = *proj_p75.get(&proj).unwrap_or(&0);
        if syn.pre_hits <= p25 && coactivity_ratio_pct >= p75 && syn.pre_hits > 0 {
            protection_bonus += 1500;
        }
        let count_useful = *target_proj_useful_count
            .get(&(syn.target_soma_id, proj.clone()))
            .unwrap_or(&0);
        if count_useful <= 2 && syn.long_trace > 0 {
            protection_bonus += 1000;
        }

        let prune_score = low_weight_score
            + 2 * low_coactivity_score
            + 2 * low_trace_score
            + negative_trend_score
            - protection_bonus;
        prune_scores.push((idx, prune_score));
    }

    let policies = vec![
        "hard_delete_absolute_floor",
        "hard_delete_trace_aware",
        "dormant_trace_aware",
        "dormant_trace_aware_with_return",
    ];

    let mut plotting_policies = Vec::new();
    let mut dormant_traces_for_plotting = Vec::new();

    for policy in &policies {
        println!("\n--- Testing Policy: {} ---", policy);

        let mut current_synapses = initial_synapses_day1.clone();
        let mut dormant_synapses: Vec<DormantSynapse> = Vec::new();

        // 1. Demote or delete under pruning policy
        match *policy {
            "hard_delete_absolute_floor" => {
                let mut survivors = Vec::new();
                for syn in current_synapses {
                    if syn.weight.abs() >= 1498 << 16 {
                        survivors.push(syn);
                    }
                }
                current_synapses = survivors;
            }
            "hard_delete_trace_aware" => {
                let mut sorted_scores = prune_scores.clone();
                sorted_scores.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
                let victims_indices: HashSet<usize> = sorted_scores
                    .iter()
                    .take(budget_total)
                    .map(|&(idx, _)| idx)
                    .collect();
                let mut survivors = Vec::new();
                for (idx, syn) in current_synapses.into_iter().enumerate() {
                    if !victims_indices.contains(&idx) {
                        survivors.push(syn);
                    }
                }
                current_synapses = survivors;
            }
            "dormant_trace_aware" | "dormant_trace_aware_with_return" => {
                let mut sorted_scores = prune_scores.clone();
                sorted_scores.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
                let victims_indices: HashSet<usize> = sorted_scores
                    .iter()
                    .take(budget_total)
                    .map(|&(idx, _)| idx)
                    .collect();

                let mut survivors = Vec::new();
                for (idx, syn) in current_synapses.into_iter().enumerate() {
                    let sv = soma_variants[syn.source_soma_id as usize];
                    let tv = soma_variants[syn.target_soma_id as usize];
                    let proj = get_projection_type(sv, tv);

                    if victims_indices.contains(&idx) {
                        dormant_synapses.push(DormantSynapse {
                            source_soma_id: syn.source_soma_id,
                            target_soma_id: syn.target_soma_id,
                            flat_segment_idx: syn.flat_segment_idx,
                            weight: syn.weight,
                            long_trace: syn.long_trace,
                            short_trace: syn.short_trace,
                            dormant_age: 0,
                            projection_class: proj,
                        });
                    } else {
                        survivors.push(syn);
                    }
                }
                current_synapses = survivors;
            }
            _ => {}
        }

        // Compact synapses after Demotion/Deletion
        let mut compacted = Vec::new();
        let mut by_target: Vec<Vec<FlatSynapse>> = vec![Vec::new(); n_somas];
        for syn in current_synapses {
            by_target[syn.target_soma_id as usize].push(syn);
        }
        for target_id in 0..n_somas {
            let mut incoming = by_target[target_id].clone();
            incoming.sort_by_key(|syn| std::cmp::Reverse(syn.weight.abs()));
            for (d_idx, syn) in incoming.iter_mut().enumerate() {
                syn.dendrite_idx = d_idx as u32;
            }
            compacted.extend(incoming);
        }
        current_synapses = compacted;

        let active_day1 = initial_synapses_day1.len();
        let active_day2 = current_synapses.len();
        let dormant_day2 = dormant_synapses.len();
        let deleted_day2 = active_day1 - active_day2 - dormant_day2;

        println!(
            "  Day 2 Active / Dormant / Deleted: {} / {} / {}",
            active_day2, dormant_day2, deleted_day2
        );

        // Day 2 Replay (without learning)
        println!("  Running Day 2 Replay (learning disabled)...");
        let day2_metrics: ReplayMetrics;
        let day2_results: MetricResults;
        {
            let mut runner = SimulationRunner::new(
                &flat_axons,
                &mut current_synapses,
                &variants,
                &soma_variants,
                seed_val,
            );
            day2_metrics = runner.run_day(max_ticks, false, true);
            day2_results = compute_metrics(&current_synapses, &soma_variants, &day2_metrics);
        }

        // Safety Gate Assertions Day 2
        assert_eq!(
            day2_results.dale_violations, 0,
            "Dale violations detected under {} on Day 2!",
            policy
        );
        assert_eq!(
            day2_results.dense_violations, 0,
            "Dense target violations detected under {} on Day 2!",
            policy
        );
        assert_eq!(
            day2_results.duplicate_violations, 0,
            "Duplicate violations detected under {} on Day 2!",
            policy
        );
        assert_eq!(
            day2_results.runaway_ticks, 0,
            "Runaway ticks detected under {} on Day 2!",
            policy
        );
        assert!(
            day2_results.silence_ticks < max_ticks,
            "Complete silence collapse under {} on Day 2!",
            policy
        );

        let matched_retention_day2_surv = if pre_night_results.matched_bias.abs() > 1e-5 {
            day2_results.matched_bias / pre_night_results.matched_bias
        } else {
            0.0
        };
        let full_cohort_bias_day2 =
            compute_full_cohort_metrics(&initial_synapses_day1, &current_synapses, &soma_variants);
        let matched_retention_day2_full = if pre_night_results.matched_bias.abs() > 1e-5 {
            full_cohort_bias_day2 / pre_night_results.matched_bias
        } else {
            0.0
        };

        // Day 3 Returned Context (learning enabled)
        println!("  Running Day 3 Returned Context (learning enabled)...");
        let day3_spikes: Vec<u32>;
        {
            let mut runner = SimulationRunner::new(
                &flat_axons,
                &mut current_synapses,
                &variants,
                &soma_variants,
                seed_val,
            );
            runner.run_day(max_ticks, true, true);
            day3_spikes = runner.somas.iter().map(|s| s.spike_count).collect();
        }

        // Night 2: trace merge, dormant decay, and reactivation pass
        println!("  Night 2 processing...");
        let mut runner = SimulationRunner::new(
            &flat_axons,
            &mut current_synapses,
            &variants,
            &soma_variants,
            seed_val,
        );
        runner.execute_night(true); // resets voltage, merges active traces

        // Decay traces of dormant synapses, increment age
        let k_short = 2;
        let k_long = 5;
        for ds in &mut dormant_synapses {
            ds.short_trace = ds.short_trace.saturating_sub(ds.short_trace >> k_short);
            ds.long_trace = ds.long_trace.saturating_sub(ds.long_trace >> k_long);
            ds.dormant_age += 1;
        }

        if *policy == "dormant_trace_aware_with_return" {
            // Save dormant traces before Night 2 reactivation
            for ds in &dormant_synapses {
                dormant_traces_for_plotting.push(DormantTraceData {
                    long_trace: ds.long_trace,
                    age: ds.dormant_age,
                });
            }

            // Reactivation pass
            let mut reactivated_indices = Vec::new();
            let mut target_active_counts = vec![0; n_somas];
            for syn in &*runner.flat_synapses {
                target_active_counts[syn.target_soma_id as usize] += 1;
            }
            let mut target_proj_counts = HashMap::new();
            for syn in &*runner.flat_synapses {
                let sv = soma_variants[syn.source_soma_id as usize];
                let tv = soma_variants[syn.target_soma_id as usize];
                let proj = get_projection_type(sv, tv);
                *target_proj_counts
                    .entry((syn.target_soma_id, proj))
                    .or_insert(0) += 1;
            }

            let mut trace_ok_count = 0;
            let mut slot_ok_count = 0;
            let mut diversity_ok_count = 0;
            let mut activity_ok_count = 0;

            for (d_idx, ds) in dormant_synapses.iter().enumerate() {
                let trace_ok = ds.long_trace >= 20; // THETA_REACTIVATE_TRACE
                let target_count = target_active_counts[ds.target_soma_id as usize];
                let slot_ok = target_count < 96; // target soft cap limit
                let initial_count = *initial_target_proj_counts
                    .get(&(ds.target_soma_id, ds.projection_class.clone()))
                    .unwrap_or(&0);
                let current_proj_count = *target_proj_counts
                    .get(&(ds.target_soma_id, ds.projection_class.clone()))
                    .unwrap_or(&0);
                let diversity_ok = current_proj_count < initial_count;

                let source_spikes = day3_spikes[ds.source_soma_id as usize];
                let target_spikes = day3_spikes[ds.target_soma_id as usize];
                let activity_ok = source_spikes > 0 && target_spikes > 0;

                if trace_ok {
                    trace_ok_count += 1;
                }
                if slot_ok {
                    slot_ok_count += 1;
                }
                if diversity_ok {
                    diversity_ok_count += 1;
                }
                if activity_ok {
                    activity_ok_count += 1;
                }

                if trace_ok && slot_ok && diversity_ok && activity_ok {
                    reactivated_indices.push(d_idx);
                    target_active_counts[ds.target_soma_id as usize] += 1;
                    *target_proj_counts
                        .entry((ds.target_soma_id, ds.projection_class.clone()))
                        .or_insert(0) += 1;
                }
            }

            println!(
                "  Reactivation blocker breakdown (out of {}):",
                dormant_synapses.len()
            );
            println!("    trace_ok: {}", trace_ok_count);
            println!("    slot_ok: {}", slot_ok_count);
            println!("    diversity_ok: {}", diversity_ok_count);
            println!("    activity_ok: {}", activity_ok_count);
            println!("    all_ok/reactivated: {}", reactivated_indices.len());

            let mut remaining_dormant = Vec::new();
            for (d_idx, ds) in dormant_synapses.iter().enumerate() {
                if reactivated_indices.contains(&d_idx) {
                    runner.flat_synapses.push(FlatSynapse {
                        source_soma_id: ds.source_soma_id,
                        flat_segment_idx: ds.flat_segment_idx,
                        target_soma_id: ds.target_soma_id,
                        dendrite_idx: 0,
                        weight: ds.weight,
                        fatigue: 0,
                        pre_hits: 0,
                        coactivity_hits: 0,
                        weight_trend: 0,
                        short_trace: ds.short_trace,
                        long_trace: ds.long_trace,
                        age_or_grace: 0,
                        pre_trace_timer: 0,
                        initial_weight: ds.weight,
                    });
                } else {
                    remaining_dormant.push(ds.clone());
                }
            }
            dormant_synapses = remaining_dormant;

            // Re-compact active synapses
            let mut compacted_synapses = Vec::new();
            let mut by_target: Vec<Vec<FlatSynapse>> = vec![Vec::new(); n_somas];
            for syn in runner.flat_synapses.iter() {
                by_target[syn.target_soma_id as usize].push(syn.clone());
            }
            for target_id in 0..n_somas {
                let mut incoming = by_target[target_id].clone();
                incoming.sort_by_key(|syn| std::cmp::Reverse(syn.weight.abs()));
                for (d_idx, syn) in incoming.iter_mut().enumerate() {
                    syn.dendrite_idx = d_idx as u32;
                }
                compacted_synapses.extend(incoming);
            }
            *runner.flat_synapses = compacted_synapses;
        }

        let active_day4 = runner.flat_synapses.len();
        let dormant_day4 = dormant_synapses.len();
        let deleted_day4 = active_day1 - active_day4 - dormant_day4;
        println!(
            "  Day 4 Active / Dormant / Deleted: {} / {} / {}",
            active_day4, dormant_day4, deleted_day4
        );

        // Day 4 Replay (without learning)
        println!("  Running Day 4 Replay (learning disabled)...");
        let day4_metrics = runner.run_day(max_ticks, false, true);
        let day4_results = compute_metrics(runner.flat_synapses, &soma_variants, &day4_metrics);

        // Safety Gate Assertions
        assert_eq!(
            day4_results.dale_violations, 0,
            "Dale violations detected under {} on Day 4!",
            policy
        );
        assert_eq!(
            day4_results.dense_violations, 0,
            "Dense target violations detected under {} on Day 4!",
            policy
        );
        assert_eq!(
            day4_results.duplicate_violations, 0,
            "Duplicate violations detected under {} on Day 4!",
            policy
        );
        assert_eq!(
            day4_results.runaway_ticks, 0,
            "Runaway ticks detected under {} on Day 4!",
            policy
        );
        assert!(
            day4_results.silence_ticks < max_ticks,
            "Complete silence collapse under {} on Day 4!",
            policy
        );

        let matched_retention_day4_surv = if pre_night_results.matched_bias.abs() > 1e-5 {
            day4_results.matched_bias / pre_night_results.matched_bias
        } else {
            0.0
        };
        let full_cohort_bias_day4 = compute_full_cohort_metrics(
            &initial_synapses_day1,
            runner.flat_synapses,
            &soma_variants,
        );
        let matched_retention_day4_full = if pre_night_results.matched_bias.abs() > 1e-5 {
            full_cohort_bias_day4 / pre_night_results.matched_bias
        } else {
            0.0
        };

        // Dormant metrics
        let dormant_matched = dormant_synapses
            .iter()
            .filter(|ds| {
                ds.source_soma_id < 48 && ds.target_soma_id >= 128 && ds.target_soma_id < 176
            })
            .count();
        let dormant_high_trace = dormant_synapses
            .iter()
            .filter(|ds| ds.long_trace >= 20)
            .count();

        println!(
            "  Matched full-cohort retention Day2 / Day4: {:.4} / {:.4}",
            matched_retention_day2_full, matched_retention_day4_full
        );
        println!(
            "  Survivor retention Day2 / Day4: {:.4} / {:.4}",
            matched_retention_day2_surv, matched_retention_day4_surv
        );
        println!(
            "  Silence ticks Day2 / Day4: {} / {}",
            day2_results.silence_ticks, day4_results.silence_ticks
        );
        println!("  Dormant Protected Matched count: {}", dormant_matched);
        println!(
            "  Dormant Protected High Long Trace count: {}",
            dormant_high_trace
        );

        plotting_policies.push(PolicyPlotDataV07 {
            name: policy.to_string(),
            active_day1,
            active_day2,
            active_day4,
            dormant_day2,
            dormant_day4,
            deleted_day2,
            deleted_day4,
            matched_retention_day2_full,
            matched_retention_day4_full,
            matched_retention_day2_surv,
            matched_retention_day4_surv,
        });
    }

    // Save plot data
    let plot_data = PlottingDataV07 {
        policies: plotting_policies,
        dormant_traces: dormant_traces_for_plotting,
    };
    let json_content = serde_json::to_string(&plot_data).unwrap();
    let dest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/engine/research/archive/2026-07-06_night_phase_dormant_bank_v0_7/artifacts/plot_data.json");
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&dest_path, json_content).unwrap();
    println!("Saved v0.7 plotting data to {:?}", dest_path);
}
