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

// ----------------- Persistent Simulation Runner -----------------

struct SimulationRunner<'a> {
    flat_axons: &'a [FlatAxon],
    flat_synapses: &'a mut [FlatSynapse],
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
        flat_synapses: &'a mut [FlatSynapse],
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

    fn run_day(&mut self, max_ticks: usize, is_learning: bool) -> ReplayMetrics {
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

            // 1. External Drive / Heartbeat for Virtual
            for i in 0..n {
                if self.soma_variants[i] == 0 {
                    let block = tick % 100;
                    let group_a = i < 48;
                    let is_stim_spike = if (group_a && block == 0) || (!group_a && block == 50) {
                        true
                    } else {
                        rng_next(&mut self.rng_seed) % 200 == 0 // 0.5%
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

            // Co-activate matched L4 somas during learning phase
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

            // 2. Synaptic current integration
            let mut i_in = vec![0i32; n];
            for syn in self.flat_synapses.iter_mut() {
                syn.fatigue = physics::recover_fatigue(syn.fatigue);

                let pre_axon = syn.source_soma_id as usize;
                let target_variant =
                    &self.variants[self.soma_variants[syn.target_soma_id as usize] as usize];
                let seg_idx = syn.flat_segment_idx as usize;
                if self
                    .active_segments
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
                if self.soma_variants[i] == 0 {
                    if spikes_this_tick[i] {
                        active_somas_count[0].insert(i);
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
                }

                if self.somas[i].voltage > -25_000 {
                    vm_health_above_neg25 += 1;
                }
            }

            // 4. GSOP Plasticity Updates
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

            // 6. Track Tick Metrics
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

    fn execute_night(&mut self, decay_weights: bool) {
        let n = self.soma_variants.len();

        // 1. Passive Recovery & Reset
        for i in 0..n {
            let v_id = self.soma_variants[i];
            let var = &self.variants[v_id as usize];
            self.somas[i].voltage = var.rest_potential;
            self.somas[i].thresh_offset = 0;
            self.somas[i].refractory_timer = 0;
            self.somas[i].burst_count = 0;
        }

        // Reset active action potentials
        for axon in self.flat_axons {
            self.active_segments[axon.soma_id as usize] = vec![false; axon.total_segments];
        }

        // 2. Synaptic fatigue decay/reset
        for syn in self.flat_synapses.iter_mut() {
            syn.fatigue = 0;
        }

        // 3. Synaptic weight decay (if enabled)
        if decay_weights {
            for syn in self.flat_synapses.iter_mut() {
                // 0.1% sign-preserving weight decay
                let decay_amount = (syn.weight.abs() as f64 * 0.001).round() as i32;
                let sign = syn.weight.signum();
                let mut new_mag = syn.weight.abs() - decay_amount;
                if new_mag < 0 {
                    new_mag = 0;
                }
                syn.weight = new_mag * sign;
            }
        }
    }
}

// ----------------- Metrics Calculator Helper -----------------

#[derive(Debug, Clone)]
struct MetricResults {
    total_synapses: usize,
    projections: HashMap<String, usize>,
    matched_mean_delta: f64,
    unmatched_mean_delta: f64,
    matched_bias: f64,
    dale_violations: usize,
    silence_ticks: usize,
    runaway_ticks: usize,
}

fn compute_metrics(
    flat_synapses: &[FlatSynapse],
    initial_weights: &[i32],
    soma_variants: &[u8],
    replay_metrics: &ReplayMetrics,
) -> MetricResults {
    let total_synapses = flat_synapses.len();

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

    for (idx, syn) in flat_synapses.iter().enumerate() {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];

        let name = get_projection_type(sv, tv);
        if projections.contains_key(&name) {
            *projections.get_mut(&name).unwrap() += 1;
        }

        let is_inhibitory = sv == 2;
        if is_inhibitory {
            if syn.weight > 0 {
                dale_violations += 1;
            }
        } else {
            if syn.weight < 0 {
                dale_violations += 1;
            }
        }

        let initial_w = initial_weights[idx];
        let delta = (syn.weight.abs() - initial_w.abs()) as i64;

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
    let m_bias = m_mean - u_mean;

    MetricResults {
        total_synapses,
        projections,
        matched_mean_delta: m_mean,
        unmatched_mean_delta: u_mean,
        matched_bias: m_bias,
        dale_violations,
        silence_ticks: replay_metrics.silence_ticks,
        runaway_ticks: replay_metrics.runaway_ticks,
    }
}

// ----------------- The Main Test Function -----------------

#[test]
fn run_night_phase_passive_recovery() {
    println!("=== Starting Night Phase Passive Recovery v0.2 ===");

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

    let nt_l4_real = load_variant(find_profile_path("L4_spiny_VISl4_4"));
    let nt_l23_real = load_variant(find_profile_path("L23_aspiny_VISp23_218"));
    let nt_l5_real = load_variant(find_profile_path("L5_spiny_VISp5_7"));
    let mut nt_virtual = nt_l4_real;
    nt_virtual.is_inhibitory = 0;
    let variants = vec![nt_virtual, nt_l4_real, nt_l23_real, nt_l5_real];

    // Winner 2 config (C17)
    let winner_cfg = RunConfig {
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

    let policies = vec![
        "no_night_control",
        "passive_recovery_only",
        "passive_recovery_plus_light_weight_decay",
    ];

    let max_ticks = 10000;
    let mut policy_metrics = HashMap::new();

    for policy in &policies {
        println!("\n--- Testing Night Phase Policy: {} ---", policy);

        let (axons, synapses) =
            run_multifield_simulation(&topo, &shard_config, seed_val, &winner_cfg);
        let (mut flat_synapses, flat_axons) = build_flat_tuples(&axons, &synapses, &topo);
        let initial_weights: Vec<i32> = flat_synapses.iter().map(|s| s.weight).collect();

        let mut runner = SimulationRunner::new(
            &flat_axons,
            &mut flat_synapses,
            &variants,
            &soma_variants,
            seed_val,
        );

        // Day 1 Learning
        println!("  Running Day 1 Learning...");
        let day1_metrics = runner.run_day(max_ticks, true);
        let pre_night_results = compute_metrics(
            &runner.flat_synapses,
            &initial_weights,
            &soma_variants,
            &day1_metrics,
        );

        // Execute Night Phase
        println!("  Executing Night Phase policy...");
        match *policy {
            "no_night_control" => {
                // Do nothing
            }
            "passive_recovery_only" => {
                runner.execute_night(false);
            }
            "passive_recovery_plus_light_weight_decay" => {
                runner.execute_night(true);
            }
            _ => panic!("Unknown night phase policy: {}", policy),
        }

        // Day 2 Replay
        println!("  Running Day 2 Replay...");
        let day2_metrics = runner.run_day(max_ticks, false);
        let post_night_results = compute_metrics(
            &runner.flat_synapses,
            &initial_weights,
            &soma_variants,
            &day2_metrics,
        );

        let retention_ratio = if pre_night_results.matched_bias.abs() > 1e-5 {
            post_night_results.matched_bias / pre_night_results.matched_bias
        } else {
            0.0
        };

        println!(
            "  Policy results: pre_bias={:.4}, post_bias={:.4}, retention={:.4}",
            pre_night_results.matched_bias, post_night_results.matched_bias, retention_ratio
        );

        // Assert Hard Gates (Acceptance Gates)
        // 1. Topology unchanged for passive recovery policies
        assert_eq!(
            pre_night_results.total_synapses, post_night_results.total_synapses,
            "{} changed synapse count from {} to {}!",
            policy, pre_night_results.total_synapses, post_night_results.total_synapses
        );
        assert_eq!(
            pre_night_results.projections, post_night_results.projections,
            "{} changed projection counts!",
            policy
        );

        // 2. Expected projections preserved
        for (proj, &count) in &post_night_results.projections {
            assert!(
                count > 0,
                "Projection {} lost completely (count = 0) in {}!",
                proj,
                policy
            );
        }

        // 3. Dale/sign violations = 0
        assert_eq!(
            post_night_results.dale_violations, 0,
            "{} has {} Dale/sign violations!",
            policy, post_night_results.dale_violations
        );

        // 4. Runaway ticks = 0
        assert_eq!(
            post_night_results.runaway_ticks, 0,
            "{} triggered runaway dynamics (runaway_ticks = {})!",
            policy, post_night_results.runaway_ticks
        );

        // 5. No complete silence collapse
        assert!(
            post_night_results.silence_ticks < max_ticks,
            "{} triggered complete silence collapse!",
            policy
        );

        // 6. Matched bias remains positive
        assert!(
            post_night_results.matched_bias > 0.0,
            "{} has non-positive matched bias ({:.4}) after night!",
            policy,
            post_night_results.matched_bias
        );

        // 7. Retention ratio check for passive_recovery_only
        if *policy == "passive_recovery_only" {
            assert!(
                retention_ratio >= 0.50,
                "passive_recovery_only retention ({:.4}) below 0.50!",
                retention_ratio
            );
        }

        policy_metrics.insert(
            policy.to_string(),
            (pre_night_results, post_night_results, retention_ratio),
        );
    }

    println!("\n=== Night Phase Passive Recovery Summary ===");
    for policy in &policies {
        let (pre, post, ret) = &policy_metrics[*policy];
        println!("  - Policy: {}", policy);
        println!("    Total Synapses: {}", pre.total_synapses);
        println!("    Pre-night matched bias:  {:.4}", pre.matched_bias);
        println!("    Post-night matched bias: {:.4}", post.matched_bias);
        println!("    Retention Ratio:         {:.4}", ret);
        println!("    Dale Violations:         {}", post.dale_violations);
        println!(
            "    Silence / Runaway Ticks: {} / {}",
            post.silence_ticks, post.runaway_ticks
        );
    }
}
