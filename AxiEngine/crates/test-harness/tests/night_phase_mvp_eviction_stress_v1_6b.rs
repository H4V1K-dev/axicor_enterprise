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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
enum OriginKind {
    Initial,
    Sprouted,
}

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
    origin_kind: OriginKind,
    initial_triple: (u32, u32, u32),
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
    driven_tick_count: usize,
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
    pre_trace_timer: u8,
    initial_weight: i32,
    origin_kind: OriginKind,
    initial_triple: (u32, u32, u32),
}

#[derive(Debug, Clone, serde::Serialize)]
struct CycleMetricsV16b {
    cycle: usize,
    active_count: usize,
    dormant_count: usize,
    dead_count: usize,
    pruned_to_dormant_count: usize,
    dormant_evicted_count: usize,
    sprouted_count: usize,
    reactivated_count: usize,

    // Dormant health metrics
    dormant_age_p50: u32,
    dormant_age_p90: u32,
    dormant_age_max: u32,
    dormant_long_trace_p50: u16,
    dormant_long_trace_p90: u16,
    dormant_long_trace_max: u16,
    dormant_bank_growth_rate: i32,
    max_dormant_per_target: usize,
    eviction_reason_counts: HashMap<String, usize>, // age_trace, target_cap, global_cap

    // Sprout metrics
    sprouted_target_count: usize,
    max_sprouts_on_single_target: usize,
    sprout_target_gini: f64,

    // Network health metrics
    spike_counts_per_layer: HashMap<String, u32>,
    silence_ticks: usize,
    runaway_ticks: usize,
    fan_in_gini: f64,
    top_5pct_fan_in_share: f64,
    projection_coverage: f64,
    under_recruited_count_before: usize,
    under_recruited_count_after: usize,

    // Safety and topology metrics
    dale_violations: usize,
    dense_violations: usize,
    duplicate_violations: usize,
    invalid_geometry_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PlottingDataV16b {
    cycles: Vec<CycleMetricsV16b>,
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
    path.push("Axicor_Neuron-Lib");
    if !path.exists() {
        path.pop();
        path.push("Axicor_NeUniform-Lib");
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
                    let mut arbor_pos = glam::Vec3::new(end_seg.x, end_seg.y, end_seg.z);
                    for step in 0..run_cfg.max_branch_len {
                        let bias_vec = glam::Vec3::new(0.0, 0.0, vertical_bias * 0.5);
                        let noise_vec =
                            deterministic_noise(seed + b_idx as u64, source_id as u32, step) * 0.8;
                        let steer = (bias_vec + noise_vec).normalize_or_zero();
                        arbor_pos += steer;
                        arbor.push(MultifieldSegment {
                            x: arbor_pos.x,
                            y: arbor_pos.y,
                            z: arbor_pos.z,
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

    let mut accepted_synapses = Vec::new();
    let mut all_compatible = Vec::new();

    for source_idx in 0..n {
        let axon = &completed_axons[source_idx];
        let mut connections_made = HashMap::new();

        for target_idx in 0..n {
            if source_idx == target_idx {
                continue;
            }
            let target_soma = &topo.somas[target_idx];
            let target_type = &shard_config.neuron_types[target_soma.variant_id as usize];
            let source_type = &shard_config.neuron_types[axon.axon_type_id as usize];

            if !target_type
                .growth
                .dendrite_whitelist
                .contains(&source_type.name)
            {
                continue;
            }

            let r_dendrite = run_cfg.override_dendrite_radius.unwrap_or(12.0);
            let r_dendrite_sq = r_dendrite * r_dendrite;

            for b_idx in 0..axon.branches.len() {
                let branch = &axon.branches[b_idx];
                for seg in branch {
                    let seg_pos = glam::Vec3::new(seg.x, seg.y, seg.z);
                    let target_pos = glam::Vec3::new(
                        target_soma.position.x() as f32,
                        target_soma.position.y() as f32,
                        target_soma.position.z() as f32,
                    );
                    let d_sq = seg_pos.distance_squared(target_pos);
                    if d_sq <= r_dendrite_sq {
                        let compat_syn = Synapse {
                            source_soma_id: source_idx as u32,
                            target_soma_id: target_idx as u32,
                            branch_id: seg.branch_id,
                            segment_offset: seg.segment_offset,
                            distance_sq: d_sq,
                            dendrite_idx: 0,
                        };
                        all_compatible.push(compat_syn);

                        let cur_count = *connections_made.entry(target_idx).or_insert(0);
                        if cur_count < run_cfg.max_per_pair {
                            accepted_synapses.push(Synapse {
                                source_soma_id: source_idx as u32,
                                target_soma_id: target_idx as u32,
                                branch_id: seg.branch_id,
                                segment_offset: seg.segment_offset,
                                distance_sq: d_sq,
                                dendrite_idx: cur_count as u32,
                            });
                            connections_made.insert(target_idx, cur_count + 1);
                        }
                    }
                }
            }
        }
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
            origin_kind: OriginKind::Initial,
            initial_triple: (syn.source_soma_id, flat_segment_idx, syn.target_soma_id),
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
            origin_kind: OriginKind::Initial,
            initial_triple: (syn.source_soma_id, flat_segment_idx, syn.target_soma_id),
        });
    }

    (flat_synapses, flat_axons)
}

fn get_projection_type(source_variant: u8, target_variant: u8) -> String {
    match (source_variant, target_variant) {
        (0, 1) => "Virtual->L4".to_string(),
        (1, 1) => "L4->L4".to_string(),
        (1, 2) => "L4->L23".to_string(),
        (1, 3) => "L4->L5".to_string(),
        (2, 1) => "L23->L4".to_string(),
        (2, 2) => "L23->L23".to_string(),
        (2, 3) => "L23->L5".to_string(),
        (3, 1) => "L5->L4".to_string(),
        (3, 2) => "L5->L23".to_string(),
        (3, 3) => "L5->L5".to_string(),
        _ => "Other".to_string(),
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
        has_context_b: bool,
        day_spike_counts: Option<&mut Vec<u32>>,
    ) -> ReplayMetrics {
        let n_somas = self.somas.len();
        let mut by_target: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
        for (idx, syn) in self.flat_synapses.iter().enumerate() {
            by_target[syn.target_soma_id as usize].push(idx);
        }

        let mut spiking_somas = Vec::new();
        let mut metrics = ReplayMetrics::default();
        let mut driven_ticks = 0;

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

            // Stimulus Schedule: Context A always active, Context B active on has_context_b
            if tick % 50 == 0 {
                driven_ticks += 1;
                let drive_soma_a = (tick / 50) % n_somas;
                self.somas[drive_soma_a].voltage += 25000;

                if has_context_b && (tick / 50) % 2 == 1 {
                    let drive_soma_b = ((tick / 50) * 7 + 13) % n_somas;
                    self.somas[drive_soma_b].voltage += 25000;
                }
            }

            if record_trace {
                for &src_id in &spiking_somas {
                    for syn in self.flat_synapses.iter_mut() {
                        if syn.source_soma_id == src_id {
                            syn.pre_trace_timer = 4;
                        }
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
                        }

                        syn.short_trace = syn.short_trace.saturating_add(100);
                        if syn.short_trace > 150 {
                            syn.long_trace = syn.long_trace.saturating_add(10);
                        }

                        if enable_learning {
                            let is_coactive = spiking_somas.contains(&(target_id as u32));
                            let is_inhibitory =
                                self.somas[syn.source_soma_id as usize].variant_id == 2;
                            if is_inhibitory {
                                if is_coactive {
                                    syn.weight -= 100 << 16;
                                } else {
                                    syn.weight += 10 << 16;
                                }
                                syn.weight = syn.weight.min(0);
                            } else {
                                if is_coactive {
                                    syn.weight += 100 << 16;
                                } else {
                                    syn.weight -= 10 << 16;
                                }
                                syn.weight = syn.weight.max(0);
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

        if let Some(counts) = day_spike_counts {
            for soma in &self.somas {
                counts[soma.id as usize] = soma.spike_count;
            }
        }

        metrics.driven_tick_count = driven_ticks;
        metrics
    }
}

fn compute_safety_metrics(
    flat_synapses: &[FlatSynapse],
    soma_variants: &[u8],
    replay_metrics: &ReplayMetrics,
) -> (usize, usize, usize, usize, usize, usize) {
    let n = soma_variants.len();
    let mut by_target: Vec<Vec<&FlatSynapse>> = vec![Vec::new(); n];
    for syn in flat_synapses {
        by_target[syn.target_soma_id as usize].push(syn);
    }

    let mut dense_violations = 0;
    let shard_cfg = build_shard_config();

    let mut duplicate_violations = 0;
    let mut invalid_geometry = 0;

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

            // Check geometry whitelist
            let tgt_type = &shard_cfg.neuron_types[soma_variants[target_id] as usize];
            let src_type =
                &shard_cfg.neuron_types[soma_variants[syn.source_soma_id as usize] as usize];
            if !tgt_type.growth.dendrite_whitelist.contains(&src_type.name) {
                invalid_geometry += 1;
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

    (
        dale_violations,
        dense_violations,
        duplicate_violations,
        invalid_geometry,
        replay_metrics.runaway_ticks,
        replay_metrics.silence_ticks,
    )
}

#[test]
fn run_night_phase_mvp_eviction_stress_v1_6b() {
    println!("=== Starting Night Phase MVP Eviction Stress Probe v1.6b ===");

    let seed_val = 54321;
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
        name: "MVP_Lifecycle_Stress".to_string(),
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
    let (mut active_synapses, flat_axons) = build_flat_tuples(&axons, &synapses, &topo);

    let max_ticks = 2000;
    let total_cycles = 12;

    let proj_classes = [
        "Virtual->L4",
        "L4->L4",
        "L4->L23",
        "L4->L5",
        "L23->L4",
        "L23->L23",
        "L23->L5",
        "L5->L4",
        "L5->L23",
        "L5->L5",
    ];

    let mut dormant_synapses: Vec<DormantSynapse> = Vec::new();
    let mut dead_count = 0;
    let mut cycle_metrics_list = Vec::new();

    // Configuration Limits under EVICTION STRESS
    let max_dormant_age = 2; // Age eviction fires easily
    let max_dormant_total = 200; // Global cap eviction fires easily
    let max_dormant_per_target = 3; // Target cap eviction fires easily
    let min_target_active_count = 5;
    let min_projection_active_count = 2;
    let layer_targets = [100, 50, 20, 30]; // Target spike rates

    let mut prev_dormant_count = 0;

    for cycle in 1..=total_cycles {
        // Stimulus schedule: Cycles 1-2 (Mixed), Cycles 3-8 (Sparse / Depression Stress), Cycles 9-12 (Mixed Returns)
        let has_context_b = match cycle {
            1 | 2 | 9..=12 => true,
            3..=8 => false,
            _ => unreachable!(),
        };

        println!(
            "  Cycle {} / {} (Context B Active: {})",
            cycle, total_cycles, has_context_b
        );

        // --- 1. DAY PHASE ---
        let mut day_spike_counts = vec![0u32; n_somas];
        let day_metrics = {
            let mut runner = SimulationRunner::new(
                &flat_axons,
                &mut active_synapses,
                &variants,
                &soma_variants,
                seed_val + (cycle as u64) * 100,
            );
            runner.run_day(
                max_ticks,
                true,
                true,
                has_context_b,
                Some(&mut day_spike_counts),
            )
        };

        // --- 2. PASSIVE RECOVERY ---
        for syn in active_synapses.iter_mut() {
            syn.fatigue = 0;
        }

        // --- 3. TRACE DECAY ---
        for syn in active_synapses.iter_mut() {
            syn.short_trace = syn.short_trace.saturating_sub(syn.short_trace >> 1); // fast
            syn.long_trace = syn.long_trace.saturating_sub(syn.long_trace >> 4); // slow
            if syn.age_or_grace > 0 {
                syn.age_or_grace -= 1;
            }
        }
        for ds in dormant_synapses.iter_mut() {
            ds.short_trace = ds.short_trace.saturating_sub(ds.short_trace >> 1); // fast
            ds.long_trace = ds.long_trace.saturating_sub(ds.long_trace >> 4); // slow
            ds.dormant_age += 1;
        }

        // --- 4. ACTIVE -> DORMANT PRUNING ---
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

        let mut remaining_active = Vec::new();
        let mut pruned_to_dormant_count = 0;

        // EVICTION STRESS: prune if weight is below 1490 << 16 (captures almost all depressed synapses)
        let prune_threshold_stress = 1490i32 << 16;

        for syn in active_synapses {
            let sv = soma_variants[syn.source_soma_id as usize];
            let tv = soma_variants[syn.target_soma_id as usize];
            let proj = get_projection_type(sv, tv);

            let abs_w = syn.weight.abs();
            let is_weak = abs_w < prune_threshold_stress;
            let low_coact = syn.coactivity_hits < 2;
            let grace_done = syn.age_or_grace == 0;

            let target_active = active_count_per_target[syn.target_soma_id as usize];
            let proj_active = *active_proj_counts
                .get(&(syn.target_soma_id, proj.clone()))
                .unwrap_or(&0);

            let has_headroom = target_active > min_target_active_count
                && proj_active > min_projection_active_count;

            if is_weak && low_coact && grace_done && has_headroom {
                pruned_to_dormant_count += 1;
                active_count_per_target[syn.target_soma_id as usize] -= 1;
                *active_proj_counts
                    .get_mut(&(syn.target_soma_id, proj.clone()))
                    .unwrap() -= 1;

                dormant_synapses.push(DormantSynapse {
                    source_soma_id: syn.source_soma_id,
                    target_soma_id: syn.target_soma_id,
                    flat_segment_idx: syn.flat_segment_idx,
                    weight: syn.weight,
                    long_trace: syn.long_trace,
                    short_trace: syn.short_trace,
                    dormant_age: 0,
                    projection_class: proj,
                    pre_trace_timer: syn.pre_trace_timer,
                    initial_weight: syn.initial_weight,
                    origin_kind: syn.origin_kind,
                    initial_triple: syn.initial_triple,
                });
            } else {
                remaining_active.push(syn);
            }
        }
        active_synapses = remaining_active;

        // Reset coactivity hits and pre hits for the next day
        for syn in active_synapses.iter_mut() {
            syn.coactivity_hits = 0;
            syn.pre_hits = 0;
        }

        // Re-compact active synapse indices (dendrite_idx)
        let mut by_target_active: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
        for (idx, syn) in active_synapses.iter().enumerate() {
            by_target_active[syn.target_soma_id as usize].push(idx);
        }
        for syn_indices in by_target_active {
            for (d_idx, syn_idx) in syn_indices.into_iter().enumerate() {
                active_synapses[syn_idx].dendrite_idx = d_idx as u32;
            }
        }

        // --- 5. DORMANT BANK BOUNDED EVICTION ---
        let mut fresh_dormant = Vec::new();
        let mut dormant_evicted_count = 0;
        let mut age_trace_evicted = 0;
        let mut target_cap_evicted = 0;
        let mut global_cap_evicted = 0;

        // Age/Trace prune filter
        for ds in dormant_synapses {
            if ds.dormant_age > max_dormant_age && ds.long_trace == 0 {
                dead_count += 1;
                dormant_evicted_count += 1;
                age_trace_evicted += 1;
            } else {
                fresh_dormant.push(ds);
            }
        }
        dormant_synapses = fresh_dormant;

        // Target cap bounds eviction
        let mut by_target_dormant: HashMap<u32, Vec<DormantSynapse>> = HashMap::new();
        for ds in dormant_synapses {
            by_target_dormant
                .entry(ds.target_soma_id)
                .or_default()
                .push(ds);
        }

        let mut final_dormant = Vec::new();
        for (target_id, mut target_list) in by_target_dormant {
            if target_list.len() > max_dormant_per_target {
                target_list.sort_by(|a, b| {
                    let cmp_trace = a.long_trace.cmp(&b.long_trace);
                    if cmp_trace == std::cmp::Ordering::Equal {
                        b.dormant_age.cmp(&a.dormant_age) // oldest first
                    } else {
                        cmp_trace
                    }
                });
                let evict_count = target_list.len() - max_dormant_per_target;
                target_cap_evicted += evict_count;
                dormant_evicted_count += evict_count;
                dead_count += evict_count;

                final_dormant.extend(target_list.into_iter().skip(evict_count));
            } else {
                final_dormant.extend(target_list);
            }
        }
        dormant_synapses = final_dormant;

        // Global cap bounds eviction
        if dormant_synapses.len() > max_dormant_total {
            dormant_synapses.sort_by(|a, b| {
                let cmp_trace = a.long_trace.cmp(&b.long_trace);
                if cmp_trace == std::cmp::Ordering::Equal {
                    b.dormant_age.cmp(&a.dormant_age) // oldest first
                } else {
                    cmp_trace
                }
            });
            let evict_count = dormant_synapses.len() - max_dormant_total;
            global_cap_evicted += evict_count;
            dormant_evicted_count += evict_count;
            dead_count += evict_count;

            dormant_synapses = dormant_synapses.into_iter().skip(evict_count).collect();
        }

        let mut eviction_reason_counts = HashMap::new();
        eviction_reason_counts.insert("age_trace".to_string(), age_trace_evicted);
        eviction_reason_counts.insert("target_cap".to_string(), target_cap_evicted);
        eviction_reason_counts.insert("global_cap".to_string(), global_cap_evicted);

        // --- 6. UNDER-RECRUITED TARGET SPROUTING ---
        let mut under_recruited_targets = Vec::new();
        for t_idx in 0..n_somas {
            let sv = soma_variants[t_idx];
            let target_rate = layer_targets[sv as usize];
            let pressure = target_rate - day_spike_counts[t_idx] as i32;
            if pressure > 0 {
                under_recruited_targets.push((t_idx, pressure));
            }
        }
        under_recruited_targets.sort_by_key(|&(_, p)| -p);
        let under_recruited_count_before = under_recruited_targets.len();

        let mut active_pairs: HashSet<(u32, u32, u32)> = active_synapses
            .iter()
            .map(|s| (s.source_soma_id, s.flat_segment_idx, s.target_soma_id))
            .collect();
        let dormant_pairs: HashSet<(u32, u32, u32)> = dormant_synapses
            .iter()
            .map(|s| (s.source_soma_id, s.flat_segment_idx, s.target_soma_id))
            .collect();

        // Update active count per target
        active_count_per_target = vec![0; n_somas];
        for syn in &active_synapses {
            active_count_per_target[syn.target_soma_id as usize] += 1;
        }

        let mut pair_counts = HashMap::new();
        for syn in &active_synapses {
            *pair_counts
                .entry((syn.source_soma_id, syn.target_soma_id))
                .or_insert(0) += 1;
        }

        // Active projection counts
        active_proj_counts.clear();
        for syn in &active_synapses {
            let sv = soma_variants[syn.source_soma_id as usize];
            let tv = soma_variants[syn.target_soma_id as usize];
            let proj = get_projection_type(sv, tv);
            *active_proj_counts
                .entry((syn.target_soma_id, proj))
                .or_insert(0) += 1;
        }

        let all_compatible_flat = convert_to_flat_synapses(&all_compatible_synapses, &axons, &topo);
        let mut candidate_pool = Vec::new();
        for i in 0..all_compatible_flat.len() {
            candidate_pool.push(SproutCandidate {
                syn: all_compatible_flat[i].clone(),
                orig_syn: all_compatible_synapses[i].clone(),
            });
        }

        let mut sprouted_count = 0;
        let mut sprouted_targets = HashSet::new();
        let mut sprouts_per_target_in_cycle = vec![0usize; n_somas];

        for &(target_id, _pressure) in &under_recruited_targets {
            let mut target_candidates = Vec::new();
            for cand in &candidate_pool {
                if cand.syn.target_soma_id == target_id as u32 {
                    let key = (
                        cand.syn.source_soma_id,
                        cand.syn.flat_segment_idx,
                        cand.syn.target_soma_id,
                    );

                    if active_pairs.contains(&key) || dormant_pairs.contains(&key) {
                        continue;
                    }

                    if active_count_per_target[target_id] >= 96 {
                        continue;
                    }

                    let current_pair_count = *pair_counts
                        .get(&(cand.syn.source_soma_id, target_id as u32))
                        .unwrap_or(&0);
                    if current_pair_count >= 2 {
                        continue;
                    }

                    target_candidates.push(cand.clone());
                }
            }

            if target_candidates.is_empty() {
                continue;
            }

            let mut sprouted_on_target = 0;
            let max_sprouts_target = 8;
            let mut rng_seed = deterministic_rng(seed_val, target_id as u32, cycle * 100);
            let mut step_count = 0;

            // Projection classes tracking on this target
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

            let mut eligible_candidates = target_candidates;

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
                    let diversity_bonus = if proj_count <= mean_proj { 3.0 } else { 1.0 };

                    let w = (-beta * cand.orig_syn.distance_sq).exp() * diversity_bonus;
                    weights.push(w);
                    sum_w += w;
                }

                if sum_w <= 0.0 {
                    break;
                }

                let rng_val = (rng_seed & 0xFFFFFFFF) as f32 / 4294967295.0;
                step_count += 1;
                rng_seed = deterministic_rng(rng_seed, target_id as u32, step_count + 100);
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
                    let base_w = if is_inhibitory { -1500i32 } else { 1500i32 };
                    let w_init = base_w << 16;

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
                        origin_kind: OriginKind::Sprouted,
                        initial_triple: (
                            cand.syn.source_soma_id,
                            cand.syn.flat_segment_idx,
                            target_id as u32,
                        ),
                    };

                    active_synapses.push(sprouted_syn);
                    active_pairs.insert((
                        cand.syn.source_soma_id,
                        cand.syn.flat_segment_idx,
                        target_id as u32,
                    ));

                    active_count_per_target[target_id] += 1;
                    *active_proj_counts
                        .entry((target_id as u32, proj.clone()))
                        .or_insert(0) += 1;
                    *pair_counts
                        .get_mut(&(cand.syn.source_soma_id, target_id as u32))
                        .unwrap() += 1;
                    sprouted_on_target += 1;
                    sprouted_count += 1;
                    sprouted_targets.insert(target_id);
                    sprouts_per_target_in_cycle[target_id] += 1;

                    let src = cand.syn.source_soma_id;
                    let new_pc = *pair_counts.get(&(src, target_id as u32)).unwrap_or(&0);
                    if new_pc >= 2 {
                        eligible_candidates.retain(|c| c.syn.source_soma_id != src);
                    }
                }
            }
        }

        // Recompute remaining under-recruited target count after sprouting
        let mut under_recruited_count_after = 0;
        for &(target_id, _p) in &under_recruited_targets {
            if !sprouted_targets.contains(&target_id) {
                under_recruited_count_after += 1;
            }
        }

        // Re-compact active synapses
        let mut by_target_active: Vec<Vec<usize>> = vec![Vec::new(); n_somas];
        for (idx, syn) in active_synapses.iter().enumerate() {
            by_target_active[syn.target_soma_id as usize].push(idx);
        }
        for syn_indices in by_target_active {
            for (d_idx, syn_idx) in syn_indices.into_iter().enumerate() {
                active_synapses[syn_idx].dendrite_idx = d_idx as u32;
            }
        }

        // --- 7. SAFETY GATES ---
        let (dale_v, dense_v, duplicate_v, geom_v, runaway_v, silence_v) =
            compute_safety_metrics(&active_synapses, &soma_variants, &day_metrics);

        assert_eq!(dale_v, 0, "Dale violations detected in cycle {}!", cycle);
        assert_eq!(dense_v, 0, "Dense violations detected in cycle {}!", cycle);
        assert_eq!(
            duplicate_v, 0,
            "Duplicate violations detected in cycle {}!",
            cycle
        );
        assert_eq!(
            geom_v, 0,
            "Geometry whitelist violations detected in cycle {}!",
            cycle
        );
        assert_eq!(
            runaway_v, 0,
            "Runaway tick violations detected in cycle {}!",
            cycle
        );
        assert!(
            !active_synapses.is_empty(),
            "Active synapses collapsed to 0 in cycle {}!",
            cycle
        );

        let current_dormant_count = dormant_synapses.len();
        let dormant_bank_growth_rate = current_dormant_count as i32 - prev_dormant_count as i32;
        prev_dormant_count = current_dormant_count;

        // Hard checks on stress caps
        assert!(
            current_dormant_count <= max_dormant_total,
            "Dormant bank leaked beyond max_dormant_total ({}) in cycle {}!",
            max_dormant_total,
            cycle
        );

        // Compute dormant bank statistics for logging
        let mut ages: Vec<u32> = dormant_synapses.iter().map(|d| d.dormant_age).collect();
        ages.sort();
        let dormant_age_p50 = if ages.is_empty() {
            0
        } else {
            ages[ages.len() / 2]
        };
        let dormant_age_p90 = if ages.is_empty() {
            0
        } else {
            ages[(ages.len() as f64 * 0.9) as usize % ages.len()]
        };
        let dormant_age_max = ages.iter().max().cloned().unwrap_or(0);

        let mut long_traces: Vec<u16> = dormant_synapses.iter().map(|d| d.long_trace).collect();
        long_traces.sort();
        let dormant_long_trace_p50 = if long_traces.is_empty() {
            0
        } else {
            long_traces[long_traces.len() / 2]
        };
        let dormant_long_trace_p90 = if long_traces.is_empty() {
            0
        } else {
            long_traces[(long_traces.len() as f64 * 0.9) as usize % long_traces.len()]
        };
        let dormant_long_trace_max = long_traces.iter().max().cloned().unwrap_or(0);

        let mut dormant_counts_per_target = HashMap::new();
        for ds in &dormant_synapses {
            *dormant_counts_per_target
                .entry(ds.target_soma_id)
                .or_insert(0) += 1;
        }
        let max_dormant_per_target = dormant_counts_per_target
            .values()
            .max()
            .cloned()
            .unwrap_or(0);

        // Spike count per layer
        let mut spike_counts_per_layer = HashMap::new();
        let mut layer_spikes = [0u32; 4];
        let mut layer_somas = [0u32; 4];
        for soma in &topo.somas {
            layer_spikes[soma.variant_id as usize] += day_spike_counts[soma.soma_id as usize];
            layer_somas[soma.variant_id as usize] += 1;
        }
        spike_counts_per_layer.insert("Virtual".to_string(), layer_spikes[0]);
        spike_counts_per_layer.insert("L4".to_string(), layer_spikes[1]);
        spike_counts_per_layer.insert("L23".to_string(), layer_spikes[2]);
        spike_counts_per_layer.insert("L5".to_string(), layer_spikes[3]);

        // Gini & Monopoly
        let mut fan_in_counts = vec![0; n_somas];
        for syn in &active_synapses {
            fan_in_counts[syn.target_soma_id as usize] += 1;
        }
        let fan_in_gini = compute_gini(&fan_in_counts);

        let mut sprout_target_counts: HashMap<u32, usize> = HashMap::new();
        for syn in &active_synapses {
            if syn.origin_kind == OriginKind::Sprouted {
                *sprout_target_counts.entry(syn.target_soma_id).or_insert(0) += 1;
            }
        }
        let top_5pct_fan_in_share = if sprout_target_counts.is_empty() {
            0.0
        } else {
            let mut counts: Vec<usize> = sprout_target_counts.values().cloned().collect();
            counts.sort_by(|a, b| b.cmp(a));
            let top_5pct_len = ((n_somas as f64) * 0.05).ceil() as usize;
            let top_5pct_sum: usize = counts.iter().take(top_5pct_len).sum();
            let total_sprouted: usize = counts.iter().sum();
            top_5pct_sum as f64 / total_sprouted as f64
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
        let projection_coverage = active_proj_types.len() as f64 / proj_classes.len() as f64;

        // Custom sprout metrics for stress validation
        let sprouted_target_count = sprouted_targets.len();
        let max_sprouts_on_single_target = sprouts_per_target_in_cycle
            .iter()
            .max()
            .cloned()
            .unwrap_or(0);
        let sprout_target_gini = compute_gini(&sprouts_per_target_in_cycle);

        cycle_metrics_list.push(CycleMetricsV16b {
            cycle,
            active_count: active_synapses.len(),
            dormant_count: current_dormant_count,
            dead_count,
            pruned_to_dormant_count,
            dormant_evicted_count,
            sprouted_count,
            reactivated_count: 0,

            dormant_age_p50,
            dormant_age_p90,
            dormant_age_max,
            dormant_long_trace_p50,
            dormant_long_trace_p90,
            dormant_long_trace_max,
            dormant_bank_growth_rate,
            max_dormant_per_target,
            eviction_reason_counts,

            sprouted_target_count,
            max_sprouts_on_single_target,
            sprout_target_gini,

            spike_counts_per_layer,
            silence_ticks: day_metrics.silence_ticks,
            runaway_ticks: day_metrics.runaway_ticks,
            fan_in_gini,
            top_5pct_fan_in_share,
            projection_coverage,
            under_recruited_count_before,
            under_recruited_count_after,

            dale_violations: dale_v,
            dense_violations: dense_v,
            duplicate_violations: duplicate_v,
            invalid_geometry_count: geom_v,
        });
    }

    // Write metrics to v1.6b archive artifacts folder
    let archive_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/engine/research/archive/2026-07-06_night_phase_mvp_eviction_stress_v1_6b/artifacts");
    std::fs::create_dir_all(&archive_dir).expect("Failed to create archive artifacts dir");
    let json_path = archive_dir.join("plot_data.json");
    let data = PlottingDataV16b {
        cycles: cycle_metrics_list,
    };
    let json_str = serde_json::to_string_pretty(&data).unwrap();
    std::fs::write(&json_path, json_str).expect("Failed to write plot_data.json");
    println!("Saved v1.6b plotting data to {:?}", json_path);
}
