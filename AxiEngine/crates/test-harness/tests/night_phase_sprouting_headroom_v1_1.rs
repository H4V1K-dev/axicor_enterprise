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
    dormant_context_hits: u16,
    pre_trace_timer: u8,
    initial_weight: i32,
}

#[derive(Default, Clone)]
struct IndexedEvidence {
    source_segment_hit_set: HashSet<(u32, u32)>,
    target_spike_set: HashSet<u32>,
    source_segment_buckets: HashSet<(u32, u32, usize)>,
    target_spike_buckets: HashSet<(u32, usize)>,
    day_inserts: usize,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
struct BlockerBreakdown {
    pair_cap_blocked: usize,
    exact_duplicate_blocked: usize,
    target_fan_in_blocked: usize,
    projection_diversity_blocked: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct EvaluationPlotDataV11 {
    topology: String,
    policy: String,
    active_count: usize,
    dormant_count: usize,
    dead_count: usize,
    sprouted_count: usize,
    eligible_candidate_count: usize,
    blocker_breakdown: BlockerBreakdown,
    sprouted_by_proj: HashMap<String, usize>,
    fan_in_distribution: Vec<usize>,
    gini_coefficient: f64,
    projection_coverage: f64,
    under_recruited_activity_before: f64,
    under_recruited_activity_after: f64,
    monopoly_top_5pct_share: f64,
    dale_violations: usize,
    dense_violations: usize,
    duplicate_violations: usize,
    runaway_ticks: usize,
    silence_ticks: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PlottingDataV11 {
    evaluations: Vec<EvaluationPlotDataV11>,
}

#[derive(Debug, Clone)]
struct SproutCandidate {
    syn: FlatSynapse,
    orig_syn: Synapse,
}

fn compute_gini(values: &[usize]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mut sum_diff = 0.0;
    let mut sum_val = 0.0;
    for i in 0..n {
        sum_val += values[i] as f64;
        for j in 0..n {
            sum_diff += ((values[i] as i64 - values[j] as i64).abs()) as f64;
        }
    }
    if sum_val == 0.0 {
        return 0.0;
    }
    sum_diff / (2.0 * n as f64 * sum_val)
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
) -> (Vec<MultifieldAxonPath>, Vec<Synapse>, Vec<Synapse>) {
    let mut completed_axons = Vec::new();
    let n = topo.somas.len();

    let neuron_types: Vec<&config::NeuronType> = topo
        .somas
        .iter()
        .map(|s| &shard_config.neuron_types[s.variant_id as usize])
        .collect();

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

    let mut all_compatible = Vec::new();
    for list in &target_candidates {
        all_compatible.extend(list.clone());
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
                a.distance_sq
                    .partial_cmp(&b.distance_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        if selected.len() > cap_limit {
            selected.truncate(cap_limit);
        }

        for (d_idx, syn) in selected.iter_mut().enumerate() {
            syn.dendrite_idx = d_idx as u32;
        }

        accepted_synapses.extend(selected);
    }

    (completed_axons, accepted_synapses, all_compatible)
}

fn convert_to_flat_synapses(
    synapses: &[Synapse],
    axons: &[MultifieldAxonPath],
    topo: &topology::SingleShardTopology,
) -> Vec<FlatSynapse> {
    let mut flat = Vec::new();
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
        flat.push(FlatSynapse {
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
    flat
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

fn get_projection_type(source_variant: u8, target_variant: u8) -> String {
    match (source_variant, target_variant) {
        (0, 1) => "L4->L23".to_string(),
        (0, 3) => "L4->L5".to_string(),
        (1, 0) => "L23->L4".to_string(),
        (1, 1) => "L23->L23".to_string(),
        (1, 3) => "L23->L5".to_string(),
        (3, 1) => "L5->L23".to_string(),
        _ => "Virtual->L4".to_string(),
    }
}

struct SimulationRunner<'a> {
    axons: &'a [FlatAxon],
    flat_synapses: &'a mut Vec<FlatSynapse>,
    somas: Vec<SomaState>,
    variant_params: &'a [VariantParameters],
    v_rest: i32,
    v_reset: i32,
    v_thresh_base: i32,
}

impl<'a> SimulationRunner<'a> {
    fn new(
        axons: &'a [FlatAxon],
        flat_synapses: &'a mut Vec<FlatSynapse>,
        variants: &'a [VariantParameters],
        soma_variants: &[u8],
        seed: u64,
    ) -> Self {
        let v_rest = -70000;
        let v_reset = -70000;
        let v_thresh_base = -55000;

        let mut somas = Vec::with_capacity(soma_variants.len());
        for (i, &var_id) in soma_variants.iter().enumerate() {
            let noise_offset = (deterministic_rng(seed, i as u32, 42) % 4000) as i32 - 2000;
            somas.push(SomaState {
                id: i as u32,
                variant_id: var_id,
                voltage: v_rest + noise_offset,
                thresh_offset: 0,
                refractory_timer: 0,
                burst_count: 0,
                spike_count: 0,
            });
        }

        Self {
            axons,
            flat_synapses,
            somas,
            variant_params: variants,
            v_rest,
            v_reset,
            v_thresh_base,
        }
    }

    fn run_day(
        &mut self,
        ticks: usize,
        enable_learning: bool,
        record_trace: bool,
        mut indexed_evidence: Option<&mut IndexedEvidence>,
        day3_spike_counts: Option<&mut Vec<u32>>,
    ) -> ReplayMetrics {
        let n_somas = self.somas.len();
        let mut by_target: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
        for (idx, syn) in self.flat_synapses.iter().enumerate() {
            by_target[syn.target_soma_id as usize].push(idx);
        }

        let mut spiking_somas = Vec::new();
        let mut metrics = ReplayMetrics::default();

        let k_short = 2;
        let k_long = 7;

        for tick in 0..ticks {
            spiking_somas.clear();

            for soma in &mut self.somas {
                if soma.refractory_timer > 0 {
                    soma.refractory_timer -= 1;
                    soma.voltage = self.v_reset;
                    continue;
                }

                let dynamic_thresh = self.v_thresh_base + soma.thresh_offset;
                if soma.voltage >= dynamic_thresh {
                    spiking_somas.push(soma.id);
                    soma.spike_count += 1;
                    soma.refractory_timer = 2;
                    soma.voltage = self.v_reset;
                    soma.thresh_offset += 5000;
                } else {
                    let leak = (soma.voltage - self.v_rest) >> 4;
                    soma.voltage -= leak;
                    soma.thresh_offset = (soma.thresh_offset * 99) / 100;
                }
            }

            if tick % 50 == 0 {
                let drive_soma = (tick / 50) % n_somas;
                self.somas[drive_soma].voltage += 25000;
            }

            if record_trace {
                for &src_id in &spiking_somas {
                    for syn in self.flat_synapses.iter_mut() {
                        if syn.source_soma_id == src_id {
                            syn.pre_trace_timer = 4;
                        }
                    }

                    if let Some(ref mut ev) = indexed_evidence {
                        ev.target_spike_set.insert(src_id);
                        ev.target_spike_buckets.insert((src_id, tick / 8));
                    }
                }
            }

            for target_id in 0..n_somas {
                let incoming = &by_target[target_id];
                if incoming.is_empty() {
                    continue;
                }

                let mut i_in: i32 = 0;
                for &syn_idx in incoming {
                    let syn = &mut self.flat_synapses[syn_idx];
                    if syn.pre_trace_timer > 0 {
                        i_in += syn.weight >> 16;
                        if record_trace {
                            syn.pre_hits = syn.pre_hits.saturating_add(1);
                            let target_spiked = spiking_somas.contains(&(target_id as u32));
                            if target_spiked {
                                syn.coactivity_hits = syn.coactivity_hits.saturating_add(1);
                            }

                            if let Some(ref mut ev) = indexed_evidence {
                                ev.source_segment_hit_set
                                    .insert((syn.source_soma_id, syn.flat_segment_idx));
                                ev.source_segment_buckets.insert((
                                    syn.source_soma_id,
                                    syn.flat_segment_idx,
                                    tick / 8,
                                ));
                                ev.day_inserts += 1;
                            }
                        }

                        syn.short_trace = syn.short_trace.saturating_add(100);
                        if syn.short_trace > 1000 {
                            syn.long_trace = syn.long_trace.saturating_add(10);
                        }

                        if enable_learning {
                            let is_coactive = spiking_somas.contains(&(target_id as u32));
                            if is_coactive {
                                syn.weight += 100 << 16;
                            } else {
                                syn.weight -= 10 << 16;
                            }
                        }
                    }

                    if record_trace {
                        syn.short_trace =
                            syn.short_trace.saturating_sub(syn.short_trace >> k_short);
                        syn.long_trace = syn.long_trace.saturating_sub(syn.long_trace >> k_long);
                        if syn.pre_trace_timer > 0 {
                            syn.pre_trace_timer -= 1;
                        }
                    }
                }

                let soma = &mut self.somas[target_id];
                if soma.refractory_timer == 0 {
                    soma.voltage += i_in;
                }
            }

            if spiking_somas.is_empty() {
                metrics.silence_ticks += 1;
            }
            if spiking_somas.len() > n_somas / 2 {
                metrics.runaway_ticks += 1;
            }
        }

        if let Some(counts) = day3_spike_counts {
            for soma in &self.somas {
                counts[soma.id as usize] = soma.spike_count;
            }
        }

        metrics
    }

    fn execute_night(&mut self, trace_merge: bool) {
        for syn in self.flat_synapses.iter_mut() {
            if trace_merge {
                if syn.long_trace >= 20 {
                    syn.weight += 500 << 16;
                } else if syn.long_trace == 0 {
                    syn.weight -= 500 << 16;
                }
            }
            syn.short_trace = 0;
            syn.long_trace = 0;
            syn.pre_hits = 0;
            syn.coactivity_hits = 0;
        }
    }
}

struct SafetyGatesResult {
    dale_violations: usize,
    dense_violations: usize,
    duplicate_violations: usize,
    runaway_ticks: usize,
    silence_ticks: usize,
    matched_bias: f64,
}

fn compute_metrics(
    flat_synapses: &[FlatSynapse],
    soma_variants: &[u8],
    replay_metrics: &ReplayMetrics,
) -> SafetyGatesResult {
    let n = soma_variants.len();
    let mut by_target: Vec<Vec<&FlatSynapse>> = vec![Vec::new(); n];
    for syn in flat_synapses {
        by_target[syn.target_soma_id as usize].push(syn);
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
        for (&_src, &count) in &source_counts {
            if count > 2 {
                duplicate_violations += 1;
            }
        }
    }

    let mut dale_violations = 0;
    for syn in flat_synapses {
        let sv = soma_variants[syn.source_soma_id as usize];
        let is_inhibitory = sv == 2;
        if is_inhibitory && syn.weight > 0 {
            dale_violations += 1;
        }
        if !is_inhibitory && syn.weight < 0 {
            dale_violations += 1;
        }
    }

    let mut total_matched_weight: f64 = 0.0;
    for syn in flat_synapses {
        let sv = soma_variants[syn.source_soma_id as usize];
        let tv = soma_variants[syn.target_soma_id as usize];
        let proj = get_projection_type(sv, tv);
        if proj == "L4->L23" || proj == "L23->L5" {
            total_matched_weight += (syn.weight >> 16) as f64;
        }
    }

    SafetyGatesResult {
        dale_violations,
        dense_violations,
        duplicate_violations,
        runaway_ticks: replay_metrics.runaway_ticks,
        silence_ticks: replay_metrics.silence_ticks,
        matched_bias: total_matched_weight,
    }
}

#[test]
fn run_night_phase_sprouting_headroom_v1_1() {
    println!("=== Starting Night Phase Sprouting Headroom v1.1 ===");

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

    let (axons, synapses, _) = run_multifield_simulation(&topo, &shard_config, seed_val, &run_cfg);
    let mut run_cfg_sprout = run_cfg.clone();
    run_cfg_sprout.override_dendrite_radius = Some(15.0);
    let (_, _, all_compatible_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &run_cfg_sprout);
    let (mut flat_synapses, flat_axons) = build_flat_tuples(&axons, &synapses, &topo);

    let max_ticks = 10000;

    println!("Initial synapse count: {}", flat_synapses.len());

    println!("Running Day 1 Learning...");
    {
        let mut runner = SimulationRunner::new(
            &flat_axons,
            &mut flat_synapses,
            &variants,
            &soma_variants,
            seed_val,
        );
        runner.run_day(max_ticks, true, true, None, None);
        runner.execute_night(true);
    }

    let initial_synapses_day1 = flat_synapses.clone();

    // Topologies & Policies
    let topologies = [
        "saturated_C17_control",
        "headroom_C17_pair1",
        "post_prune_headroom",
    ];

    let policies = [
        "no_sprouting_baseline",
        "deterministic_under_recruited_projection_diversity",
        "stochastic_geometry_projection_diversity",
    ];

    let proj_classes = [
        "Virtual->L4",
        "L4->L23",
        "L4->L5",
        "L23->L4",
        "L23->L23",
        "L23->L5",
        "L5->L23",
    ];

    let mut evaluations = Vec::new();

    for topo_name in &topologies {
        let base_synapses = if *topo_name == "headroom_C17_pair1" {
            let mut pair_counts_init = HashMap::new();
            initial_synapses_day1
                .iter()
                .filter(|syn| {
                    let count = pair_counts_init
                        .entry((syn.source_soma_id, syn.target_soma_id))
                        .or_insert(0);
                    if *count < 1 {
                        *count += 1;
                        true
                    } else {
                        false
                    }
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            initial_synapses_day1.clone()
        };

        for policy in &policies {
            println!(
                "--- Testing Topology: {} | Policy: {} ---",
                topo_name, policy
            );
            let mut active_synapses;
            let mut dormant_synapses = Vec::new();
            let mut day3_spike_counts = vec![0u32; n_somas];
            let mut indexed_evidence = IndexedEvidence::default();

            if *topo_name == "post_prune_headroom" {
                // Full pruning/reactivation path
                let mut current_synapses = base_synapses.clone();
                let day2_metrics = {
                    let mut runner = SimulationRunner::new(
                        &flat_axons,
                        &mut current_synapses,
                        &variants,
                        &soma_variants,
                        seed_val + 1,
                    );
                    runner.run_day(max_ticks, false, false, None, None)
                };

                active_synapses = Vec::new();
                for syn in current_synapses {
                    let sv = soma_variants[syn.source_soma_id as usize];
                    let tv = soma_variants[syn.target_soma_id as usize];
                    let proj = get_projection_type(sv, tv);

                    if syn.weight.abs() < 1498 << 16 {
                        dormant_synapses.push(DormantSynapse {
                            source_soma_id: syn.source_soma_id,
                            target_soma_id: syn.target_soma_id,
                            flat_segment_idx: syn.flat_segment_idx,
                            weight: syn.weight,
                            long_trace: syn.long_trace,
                            short_trace: syn.short_trace,
                            dormant_age: 0,
                            projection_class: proj,
                            dormant_context_hits: 0,
                            pre_trace_timer: syn.pre_trace_timer,
                            initial_weight: syn.initial_weight,
                        });
                    } else {
                        active_synapses.push(syn);
                    }
                }

                // Re-compact active synapses
                let mut by_target: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
                for (idx, syn) in active_synapses.iter().enumerate() {
                    by_target[syn.target_soma_id as usize].push(idx);
                }
                for syn_indices in by_target {
                    for (d_idx, syn_idx) in syn_indices.into_iter().enumerate() {
                        active_synapses[syn_idx].dendrite_idx = d_idx as u32;
                    }
                }

                // Day 3 Returned Context
                {
                    let mut runner = SimulationRunner::new(
                        &flat_axons,
                        &mut active_synapses,
                        &variants,
                        &soma_variants,
                        seed_val + 2,
                    );
                    runner.run_day(
                        max_ticks,
                        true,
                        true,
                        Some(&mut indexed_evidence),
                        Some(&mut day3_spike_counts),
                    );
                    runner.execute_night(false);
                }

                // Night 2 processing: Dormant trace decay & Reactivation pass
                let k_long = 7;
                for ds in dormant_synapses.iter_mut() {
                    ds.long_trace = ds.long_trace.saturating_sub(ds.long_trace >> k_long);
                    ds.dormant_age += 1;
                }

                let mut reactivated_indices = Vec::new();
                let mut target_active_counts = vec![0; n_somas];
                for syn in &active_synapses {
                    target_active_counts[syn.target_soma_id as usize] += 1;
                }
                let mut target_proj_counts = HashMap::new();
                for syn in &active_synapses {
                    let sv = soma_variants[syn.source_soma_id as usize];
                    let tv = soma_variants[syn.target_soma_id as usize];
                    let proj = get_projection_type(sv, tv);
                    *target_proj_counts
                        .entry((syn.target_soma_id, proj))
                        .or_insert(0) += 1;
                }

                let proj_classes = [
                    "Virtual->L4",
                    "L4->L23",
                    "L4->L5",
                    "L23->L4",
                    "L23->L23",
                    "L23->L5",
                    "L5->L23",
                ];

                for (d_idx, ds) in dormant_synapses.iter().enumerate() {
                    let trace_ok = ds.long_trace >= 20;
                    let mut context_ok = false;
                    for b in 0..(max_ticks / 8) {
                        if indexed_evidence.source_segment_buckets.contains(&(
                            ds.source_soma_id,
                            ds.flat_segment_idx,
                            b,
                        )) && (indexed_evidence
                            .target_spike_buckets
                            .contains(&(ds.target_soma_id, b))
                            || indexed_evidence
                                .target_spike_buckets
                                .contains(&(ds.target_soma_id, b + 1)))
                        {
                            context_ok = true;
                            break;
                        }
                    }
                    let context_ok = context_ok && (ds.short_trace > 0 || ds.long_trace > 0);
                    let pass_evidence = trace_ok || context_ok;

                    let target_count = target_active_counts[ds.target_soma_id as usize];
                    let slot_ok = target_count < 96;

                    let mut proj_counts_on_target = Vec::new();
                    for pc in &proj_classes {
                        let c = *target_proj_counts
                            .get(&(ds.target_soma_id, pc.to_string()))
                            .unwrap_or(&0);
                        if c > 0 {
                            proj_counts_on_target.push(c);
                        }
                    }
                    let mean_proj = total_proj_count_calc(&proj_counts_on_target);
                    let current_proj_count = *target_proj_counts
                        .get(&(ds.target_soma_id, ds.projection_class.clone()))
                        .unwrap_or(&0);
                    let diversity_ok = current_proj_count <= mean_proj;

                    if pass_evidence && slot_ok && diversity_ok {
                        reactivated_indices.push(d_idx);
                        target_active_counts[ds.target_soma_id as usize] += 1;
                        *target_proj_counts
                            .entry((ds.target_soma_id, ds.projection_class.clone()))
                            .or_insert(0) += 1;
                    }
                }

                for &r_idx in &reactivated_indices {
                    let ds = &dormant_synapses[r_idx];
                    active_synapses.push(FlatSynapse {
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
                        initial_weight: ds.initial_weight,
                    });
                }

                let remaining_dormant: Vec<_> = dormant_synapses
                    .into_iter()
                    .enumerate()
                    .filter(|(idx, _)| !reactivated_indices.contains(idx))
                    .map(|(_, ds)| ds)
                    .collect();
                dormant_synapses = remaining_dormant;

                // Re-compact active synapses
                let mut by_target: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
                for (idx, syn) in active_synapses.iter().enumerate() {
                    by_target[syn.target_soma_id as usize].push(idx);
                }
                for syn_indices in by_target {
                    for (d_idx, syn_idx) in syn_indices.into_iter().enumerate() {
                        active_synapses[syn_idx].dendrite_idx = d_idx as u32;
                    }
                }
            } else {
                // Pure headroom isolation (saturated_C17_control and headroom_C17_pair1):
                // Skip pruning/Dormant/reactivation. Use active_synapses = base_synapses.
                active_synapses = base_synapses.clone();
                {
                    let mut runner = SimulationRunner::new(
                        &flat_axons,
                        &mut active_synapses,
                        &variants,
                        &soma_variants,
                        seed_val + 2,
                    );
                    runner.run_day(max_ticks, false, false, None, Some(&mut day3_spike_counts));
                }
            }

            // 3. Sprouting Pass
            let layer_targets = [100, 50, 20, 30];
            let mut under_recruited_targets = Vec::new();
            for t_idx in 0..n_somas {
                let sv = soma_variants[t_idx];
                let target_rate = layer_targets[sv as usize];
                let pressure = (day3_spike_counts[t_idx] as i32) - target_rate;
                if pressure < 0 {
                    under_recruited_targets.push((t_idx, pressure));
                }
            }
            under_recruited_targets.sort_by_key(|&(_, p)| p);

            let active_pairs: HashSet<(u32, u32, u32)> = active_synapses
                .iter()
                .map(|s| (s.source_soma_id, s.flat_segment_idx, s.target_soma_id))
                .collect();
            let dormant_pairs: HashSet<(u32, u32, u32)> = dormant_synapses
                .iter()
                .map(|s| (s.source_soma_id, s.flat_segment_idx, s.target_soma_id))
                .collect();

            let mut active_count_per_target = vec![0; n_somas];
            for syn in &active_synapses {
                active_count_per_target[syn.target_soma_id as usize] += 1;
            }
            let mut active_proj_counts = HashMap::new();
            for syn in &active_synapses {
                let sv = soma_variants[syn.source_soma_id as usize];
                let tv = soma_variants[syn.target_soma_id as usize];
                let proj = get_projection_type(sv, tv);
                *active_proj_counts
                    .entry((syn.target_soma_id, proj))
                    .or_insert(0) += 1;
            }

            let mut pair_counts = HashMap::new();
            for syn in &active_synapses {
                *pair_counts
                    .entry((syn.source_soma_id, syn.target_soma_id))
                    .or_insert(0) += 1;
            }

            let all_compatible_flat =
                convert_to_flat_synapses(&all_compatible_synapses, &axons, &topo);
            let mut candidate_pool = Vec::new();
            for i in 0..all_compatible_flat.len() {
                candidate_pool.push(SproutCandidate {
                    syn: all_compatible_flat[i].clone(),
                    orig_syn: all_compatible_synapses[i].clone(),
                });
            }

            let mut sprouted_synapses = Vec::new();
            let mut blocker = BlockerBreakdown::default();
            let mut total_eligible_candidates = 0;

            if *policy != "no_sprouting_baseline" {
                for &(target_id, _pressure) in &under_recruited_targets {
                    let mut target_candidates = Vec::new();
                    for cand in &candidate_pool {
                        if cand.syn.target_soma_id == target_id as u32 {
                            let key = (
                                cand.syn.source_soma_id,
                                cand.syn.flat_segment_idx,
                                cand.syn.target_soma_id,
                            );

                            // Blocker check 1: Exact duplicate
                            if active_pairs.contains(&key) || dormant_pairs.contains(&key) {
                                blocker.exact_duplicate_blocked += 1;
                                continue;
                            }

                            // Blocker check 2: Fan-in limit (96)
                            if active_count_per_target[target_id] >= 96 {
                                blocker.target_fan_in_blocked += 1;
                                continue;
                            }

                            // Blocker check 3: Strict Source-Target Pair Cap (2)
                            let current_pair_count = *pair_counts
                                .get(&(cand.syn.source_soma_id, target_id as u32))
                                .unwrap_or(&0);
                            if current_pair_count >= 2 {
                                blocker.pair_cap_blocked += 1;
                                continue;
                            }

                            // Blocker check 4: Projection diversity check (if hard policy)
                            let sv = soma_variants[cand.syn.source_soma_id as usize];
                            let tv = soma_variants[cand.syn.target_soma_id as usize];
                            let proj = get_projection_type(sv, tv);

                            if *policy == "deterministic_under_recruited_projection_diversity" {
                                let mut proj_counts_on_target = Vec::new();
                                for pc in &proj_classes {
                                    let c = *active_proj_counts
                                        .get(&(target_id as u32, pc.to_string()))
                                        .unwrap_or(&0);
                                    if c > 0 {
                                        proj_counts_on_target.push(c);
                                    }
                                }
                                let mean_proj = total_proj_count_calc(&proj_counts_on_target);
                                let current_proj_count = *active_proj_counts
                                    .get(&(target_id as u32, proj.clone()))
                                    .unwrap_or(&0);
                                if current_proj_count > mean_proj {
                                    blocker.projection_diversity_blocked += 1;
                                    continue;
                                }
                            }

                            total_eligible_candidates += 1;
                            target_candidates.push(cand.clone());
                        }
                    }

                    if target_candidates.is_empty() {
                        continue;
                    }

                    let mut sprouted_on_target = 0;
                    let max_sprouts_target = 8;

                    if *policy == "stochastic_geometry_projection_diversity" {
                        let mut rng_seed = deterministic_rng(seed_val, target_id as u32, 999);
                        let mut step_count = 0;

                        let mut proj_counts_map = HashMap::new();
                        let mut total_proj_count = 0usize;
                        let mut n_present = 0usize;
                        for pc in &proj_classes {
                            let c = *active_proj_counts
                                .get(&(target_id as u32, pc.to_string()))
                                .unwrap_or(&0);
                            if c > 0 {
                                proj_counts_map.insert(pc.to_string(), c);
                                total_proj_count += c;
                                n_present += 1;
                            }
                        }
                        let mean_proj = total_proj_count.checked_div(n_present).unwrap_or(1);

                        let mut eligible_candidates: Vec<_> = target_candidates;

                        while sprouted_on_target < max_sprouts_target
                            && active_count_per_target[target_id] < 96
                            && !eligible_candidates.is_empty()
                        {
                            let beta = 2.0;
                            let mut weights = Vec::new();
                            let mut sum_w = 0.0;
                            for cand in &eligible_candidates {
                                let sv_c = soma_variants[cand.syn.source_soma_id as usize];
                                let tv_c = soma_variants[cand.syn.target_soma_id as usize];
                                let proj_c = get_projection_type(sv_c, tv_c);
                                let proj_count = *proj_counts_map.get(&proj_c).unwrap_or(&0);
                                let diversity_bonus =
                                    if proj_count <= mean_proj { 3.0 } else { 1.0 };
                                let w = (-beta * cand.orig_syn.distance_sq).exp() * diversity_bonus;
                                weights.push(w);
                                sum_w += w;
                            }

                            if sum_w <= 0.0 {
                                break;
                            }

                            let rng_val = (rng_seed & 0xFFFFFFFF) as f32 / 4294967295.0;
                            step_count += 1;
                            rng_seed =
                                deterministic_rng(rng_seed, target_id as u32, step_count + 100);
                            let target_val = rng_val * sum_w;

                            let mut acc_w = 0.0;
                            let mut chosen_idx = 0;
                            for (idx, &w) in weights.iter().enumerate() {
                                acc_w += w;
                                if target_val <= acc_w {
                                    chosen_idx = idx;
                                    break;
                                }
                            }

                            let cand = eligible_candidates.remove(chosen_idx);
                            let current_pair_count = *pair_counts
                                .entry((cand.syn.source_soma_id, target_id as u32))
                                .or_insert(0);

                            if current_pair_count < 2 {
                                let sv = soma_variants[cand.syn.source_soma_id as usize];
                                let tv = soma_variants[cand.syn.target_soma_id as usize];
                                let proj = get_projection_type(sv, tv);

                                let is_inhibitory = sv == 2;
                                let w_init = if is_inhibitory {
                                    -1500i32 << 16
                                } else {
                                    1500i32 << 16
                                };

                                let sprouted_syn = FlatSynapse {
                                    source_soma_id: cand.syn.source_soma_id,
                                    flat_segment_idx: cand.syn.flat_segment_idx,
                                    target_soma_id: target_id as u32,
                                    dendrite_idx: 0,
                                    weight: w_init,
                                    fatigue: 0,
                                    pre_hits: 0,
                                    coactivity_hits: 0,
                                    weight_trend: 0,
                                    short_trace: 50,
                                    long_trace: 0,
                                    age_or_grace: 3,
                                    pre_trace_timer: 0,
                                    initial_weight: w_init,
                                };

                                sprouted_synapses.push(sprouted_syn.clone());
                                active_synapses.push(sprouted_syn);

                                active_count_per_target[target_id] += 1;
                                *active_proj_counts
                                    .entry((target_id as u32, proj))
                                    .or_insert(0) += 1;
                                *pair_counts
                                    .get_mut(&(cand.syn.source_soma_id, target_id as u32))
                                    .unwrap() += 1;
                                sprouted_on_target += 1;

                                let src = cand.syn.source_soma_id;
                                let new_pc =
                                    *pair_counts.get(&(src, target_id as u32)).unwrap_or(&0);
                                if new_pc >= 2 {
                                    eligible_candidates.retain(|c| c.syn.source_soma_id != src);
                                }
                            }
                        }
                    } else {
                        // Deterministic spatial sorting
                        target_candidates.sort_by(|a, b| {
                            a.orig_syn
                                .distance_sq
                                .partial_cmp(&b.orig_syn.distance_sq)
                                .unwrap()
                        });

                        for cand in target_candidates {
                            if sprouted_on_target >= max_sprouts_target
                                || active_count_per_target[target_id] >= 96
                            {
                                break;
                            }

                            let sv = soma_variants[cand.syn.source_soma_id as usize];
                            let tv = soma_variants[cand.syn.target_soma_id as usize];
                            let proj = get_projection_type(sv, tv);

                            let current_pair_count = *pair_counts
                                .entry((cand.syn.source_soma_id, target_id as u32))
                                .or_insert(0);
                            if current_pair_count >= 2 {
                                continue;
                            }

                            let is_inhibitory = sv == 2;
                            let w_init = if is_inhibitory {
                                -1500i32 << 16
                            } else {
                                1500i32 << 16
                            };

                            let sprouted_syn = FlatSynapse {
                                source_soma_id: cand.syn.source_soma_id,
                                flat_segment_idx: cand.syn.flat_segment_idx,
                                target_soma_id: target_id as u32,
                                dendrite_idx: 0,
                                weight: w_init,
                                fatigue: 0,
                                pre_hits: 0,
                                coactivity_hits: 0,
                                weight_trend: 0,
                                short_trace: 50,
                                long_trace: 0,
                                age_or_grace: 3,
                                pre_trace_timer: 0,
                                initial_weight: w_init,
                            };

                            sprouted_synapses.push(sprouted_syn.clone());
                            active_synapses.push(sprouted_syn);

                            active_count_per_target[target_id] += 1;
                            *active_proj_counts
                                .entry((target_id as u32, proj))
                                .or_insert(0) += 1;
                            *pair_counts
                                .get_mut(&(cand.syn.source_soma_id, target_id as u32))
                                .unwrap() += 1;
                            sprouted_on_target += 1;
                        }
                    }
                }
            }

            // Re-compact active synapses
            let mut by_target: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
            for (idx, syn) in active_synapses.iter().enumerate() {
                by_target[syn.target_soma_id as usize].push(idx);
            }
            for syn_indices in by_target {
                for (d_idx, syn_idx) in syn_indices.into_iter().enumerate() {
                    active_synapses[syn_idx].dendrite_idx = d_idx as u32;
                }
            }

            // Day 4 Replay
            let mut day4_spike_counts = vec![0u32; n_somas];
            let day4_metrics = {
                let mut runner = SimulationRunner::new(
                    &flat_axons,
                    &mut active_synapses,
                    &variants,
                    &soma_variants,
                    seed_val + 3,
                );
                runner.run_day(max_ticks, false, false, None, Some(&mut day4_spike_counts))
            };

            let day4_results = compute_metrics(&active_synapses, &soma_variants, &day4_metrics);
            println!(
                "  Day 4 Active / Dormant / Sprouted: {} / {} / {}",
                active_synapses.len(),
                dormant_synapses.len(),
                sprouted_synapses.len()
            );

            // Safety Gate Assertions
            assert_eq!(
                day4_results.dale_violations, 0,
                "Dale violations under {} / {}!",
                topo_name, policy
            );
            assert_eq!(
                day4_results.dense_violations, 0,
                "Dense target violations under {} / {}!",
                topo_name, policy
            );
            assert_eq!(
                day4_results.duplicate_violations, 0,
                "Duplicate violations under {} / {}!",
                topo_name, policy
            );
            assert_eq!(
                day4_results.runaway_ticks, 0,
                "Runaway ticks under {} / {}!",
                topo_name, policy
            );
            assert!(
                day4_results.silence_ticks < max_ticks,
                "Silence collapse under {} / {}!",
                topo_name,
                policy
            );

            // Metrics collection
            let mut sprouted_by_proj = HashMap::new();
            for pc in &proj_classes {
                sprouted_by_proj.insert(pc.to_string(), 0);
            }
            for syn in &sprouted_synapses {
                let sv = soma_variants[syn.source_soma_id as usize];
                let tv = soma_variants[syn.target_soma_id as usize];
                let proj = get_projection_type(sv, tv);
                *sprouted_by_proj.entry(proj).or_insert(0) += 1;
            }

            let mut fan_in_counts = vec![0; n_somas];
            for syn in &active_synapses {
                fan_in_counts[syn.target_soma_id as usize] += 1;
            }
            let gini = compute_gini(&fan_in_counts);

            let mut under_recruited_somas = Vec::new();
            for &(idx, _) in &under_recruited_targets {
                under_recruited_somas.push(idx);
            }

            let (ur_before, ur_after) = if under_recruited_somas.is_empty() {
                (0.0, 0.0)
            } else {
                let sum_before: u64 = under_recruited_somas
                    .iter()
                    .map(|&idx| day3_spike_counts[idx] as u64)
                    .sum();
                let sum_after: u64 = under_recruited_somas
                    .iter()
                    .map(|&idx| day4_spike_counts[idx] as u64)
                    .sum();
                (
                    sum_before as f64 / under_recruited_somas.len() as f64,
                    sum_after as f64 / under_recruited_somas.len() as f64,
                )
            };

            let mut sprout_target_counts: HashMap<u32, usize> = HashMap::new();
            for syn in &sprouted_synapses {
                *sprout_target_counts.entry(syn.target_soma_id).or_insert(0) += 1;
            }

            let monopoly_share = if sprouted_synapses.is_empty() {
                0.0
            } else {
                let mut counts: Vec<usize> = sprout_target_counts.values().cloned().collect();
                counts.sort_by(|a, b| b.cmp(a));
                let top_5pct_len = ((under_recruited_targets.len() as f64) * 0.05).ceil() as usize;
                let top_5pct_sum: usize = counts.iter().take(top_5pct_len).sum();
                top_5pct_sum as f64 / sprouted_synapses.len() as f64
            };

            let active_proj_types: HashSet<String> = active_synapses
                .iter()
                .map(|s| {
                    get_projection_type(
                        soma_variants[s.source_soma_id as usize],
                        soma_variants[s.target_soma_id as usize],
                    )
                })
                .collect();
            let proj_cov = active_proj_types.len() as f64 / proj_classes.len() as f64;

            evaluations.push(EvaluationPlotDataV11 {
                topology: topo_name.to_string(),
                policy: policy.to_string(),
                active_count: active_synapses.len(),
                dormant_count: dormant_synapses.len(),
                dead_count: 0,
                sprouted_count: sprouted_synapses.len(),
                eligible_candidate_count: total_eligible_candidates,
                blocker_breakdown: blocker,
                sprouted_by_proj,
                fan_in_distribution: fan_in_counts,
                gini_coefficient: gini,
                projection_coverage: proj_cov,
                under_recruited_activity_before: ur_before,
                under_recruited_activity_after: ur_after,
                monopoly_top_5pct_share: monopoly_share,
                dale_violations: day4_results.dale_violations,
                dense_violations: day4_results.dense_violations,
                duplicate_violations: day4_results.duplicate_violations,
                runaway_ticks: day4_results.runaway_ticks,
                silence_ticks: day4_results.silence_ticks,
            });
        }
    }

    let archive_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/engine/research/archive/2026-07-06_night_phase_sprouting_headroom_v1_1/artifacts");
    std::fs::create_dir_all(&archive_dir).expect("Failed to create archive artifacts dir");
    let json_path = archive_dir.join("plot_data.json");
    let data = PlottingDataV11 { evaluations };
    let json_str = serde_json::to_string_pretty(&data).unwrap();
    std::fs::write(&json_path, json_str).expect("Failed to write plot_data.json");
    println!("Saved v1.1 plotting data to {:?}", json_path);
}

fn total_proj_count_calc(proj_counts: &[usize]) -> usize {
    proj_counts
        .iter()
        .sum::<usize>()
        .checked_div(proj_counts.len())
        .unwrap_or(1)
}
