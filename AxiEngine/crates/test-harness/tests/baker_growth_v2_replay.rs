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
    weight: i32, // Mass Domain
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

        let is_inhibitory = topo.somas[syn.source_soma_id as usize].variant_id == 2; // 2: L23_aspiny is inhibitory
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

    // Virtual Group A: 0..48
    // L4 matched: 128..176

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

        // Co-activate matched L4 somas during learning phase (matched post co-spikes 10 ticks after pre)
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

        // 2. Synaptic current integration across synapses
        let mut i_in = vec![0i32; n];
        for syn in flat_synapses.iter_mut() {
            syn.fatigue = physics::recover_fatigue(syn.fatigue);

            let pre_axon = syn.source_soma_id as usize;
            let pre_variant = &variants[soma_variants[pre_axon] as usize];
            let seg_idx = syn.flat_segment_idx as usize;
            if active_segments
                .get(pre_axon)
                .and_then(|segments| segments.get(seg_idx))
                .copied()
                .unwrap_or(false)
            {
                // Spike hit!
                let att_w = physics::apply_synaptic_fatigue(
                    syn.weight,
                    syn.fatigue,
                    pre_variant.fatigue_capacity,
                );
                let charge = physics::weight_to_charge(att_w);
                i_in[syn.target_soma_id as usize] =
                    i_in[syn.target_soma_id as usize].wrapping_add(charge);

                syn.fatigue =
                    physics::fatigue_after_spike(syn.fatigue, pre_variant.fatigue_capacity);
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

            let noise_curr = (rng_next(&mut rng_seed) % 201) as i32 - 100; // [-100, 100]
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

        // 4. GSOP Plasticity Updates (when post-synaptic target soma spikes)
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

#[test]
fn run_growth_v2_functional_replay() {
    println!("=== Starting Growth v2 Functional Replay v0.5 ===");

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

    // Load Variant parameters
    let nt_l4_real = load_variant(find_profile_path("L4_spiny_VISl4_4"));
    let nt_l23_real = load_variant(find_profile_path("L23_aspiny_VISp23_218"));
    let nt_l5_real = load_variant(find_profile_path("L5_spiny_VISp5_7"));

    let mut nt_virtual = nt_l4_real;
    nt_virtual.is_inhibitory = 0; // Exc driver

    let variants = vec![nt_virtual, nt_l4_real, nt_l23_real, nt_l5_real];
    let soma_variants: Vec<u8> = topo.somas.iter().map(|s| s.variant_id).collect();

    // 1. Build Candidate Topologies
    let sparse_cfg = RunConfig {
        name: "Sparse Clean".to_string(),
        max_branches: 2,
        max_branch_len: 2,
        w_fascicle: 0.5,
        r_fascicle: 2.5,
        r_repulsion: 1.0,
        override_dendrite_radius: Some(1.5),
        max_per_pair: 2,
        beta: 2.0,
    };

    let dense_cfg = RunConfig {
        name: "Dense Stress".to_string(),
        max_branches: 3,
        max_branch_len: 3,
        w_fascicle: 0.4,
        r_fascicle: 2.5,
        r_repulsion: 1.2,
        override_dendrite_radius: None,
        max_per_pair: 2,
        beta: 2.0,
    };

    let balanced_cfg = RunConfig {
        name: "Balanced Functional".to_string(),
        max_branches: 2,
        max_branch_len: 3,
        w_fascicle: 0.5,
        r_fascicle: 2.5,
        r_repulsion: 1.1,
        override_dendrite_radius: Some(9.0),
        max_per_pair: 2,
        beta: 2.0,
    };

    println!("Generating Sparse Clean Candidate...");
    let (s_axons, s_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &sparse_cfg);
    let (s_flat_syn, s_flat_ax) = build_flat_tuples(&s_axons, &s_synapses, &topo);

    println!("Generating Dense Stress Candidate...");
    let (d_axons, d_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &dense_cfg);
    let (d_flat_syn, d_flat_ax) = build_flat_tuples(&d_axons, &d_synapses, &topo);

    println!("Generating Balanced Functional Candidate...");
    let (b_axons, b_synapses) =
        run_multifield_simulation(&topo, &shard_config, seed_val, &balanced_cfg);
    let (b_flat_syn, b_flat_ax) = build_flat_tuples(&b_axons, &b_synapses, &topo);

    // Verify projection expected presence
    let check_projections = |syns: &[FlatSynapse], name: &str| {
        let mut count_l4_l5 = 0;
        let mut count_v_l4 = 0;
        let mut count_l4_l23 = 0;
        let mut count_l23_l4 = 0;
        let mut count_l23_l23 = 0;
        let mut count_l23_l5 = 0;
        let mut count_l5_l23 = 0;
        let mut unexpected = 0;

        for s in syns {
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
        println!("{}: Virtual->L4={}, L4->L23={}, L4->L5={}, L23->L4={}, L23->L23={}, L23->L5={}, L5->L23={}, Unexpected={}", 
                 name, count_v_l4, count_l4_l23, count_l4_l5, count_l23_l4, count_l23_l23, count_l23_l5, count_l5_l23, unexpected);

        assert_eq!(
            unexpected, 0,
            "{} has unexpected whitelisted projections!",
            name
        );
        if name == "Balanced Functional" {
            assert!(
                count_l4_l5 > 0,
                "Balanced candidate must have L4->L5 synapses (>0)!"
            );
        }
        (
            count_v_l4,
            count_l4_l23,
            count_l4_l5,
            count_l23_l4,
            count_l23_l23,
            count_l23_l5,
            count_l5_l23,
        )
    };

    println!("Analyzing Candidate Topologies Projections...");
    check_projections(&s_flat_syn, "Sparse Clean");
    check_projections(&d_flat_syn, "Dense Stress");
    let b_proj = check_projections(&b_flat_syn, "Balanced Functional");

    // 2. Replay simulations
    let max_ticks = 10000;

    println!("Running Replays for Sparse Clean...");
    let s_static_met = run_replay_simulation(
        &s_flat_ax,
        &mut s_flat_syn.clone(),
        &variants,
        &soma_variants,
        false,
        max_ticks,
        seed_val,
    );
    let s_gsop_met = run_replay_simulation(
        &s_flat_ax,
        &mut s_flat_syn.clone(),
        &variants,
        &soma_variants,
        true,
        max_ticks,
        seed_val,
    );

    println!("Running Replays for Dense Stress...");
    let d_static_met = run_replay_simulation(
        &d_flat_ax,
        &mut d_flat_syn.clone(),
        &variants,
        &soma_variants,
        false,
        max_ticks,
        seed_val,
    );
    let d_gsop_met = run_replay_simulation(
        &d_flat_ax,
        &mut d_flat_syn.clone(),
        &variants,
        &soma_variants,
        true,
        max_ticks,
        seed_val,
    );

    println!("Running Replays for Balanced Functional...");
    let mut b_flat_syn_gsop = b_flat_syn.clone();
    let mut b_flat_syn_static = b_flat_syn.clone();
    let b_static_met = run_replay_simulation(
        &b_flat_ax,
        &mut b_flat_syn_static,
        &variants,
        &soma_variants,
        false,
        max_ticks,
        seed_val,
    );
    let b_gsop_met = run_replay_simulation(
        &b_flat_ax,
        &mut b_flat_syn_gsop,
        &variants,
        &soma_variants,
        true,
        max_ticks,
        seed_val,
    );

    // Verify Dale's law and weight shifts on GSOP
    let verify_gsop_changes = |initial: &[FlatSynapse], final_syns: &[FlatSynapse], name: &str| {
        let mut total_delta = 0i64;
        let mut matched_delta = 0i64;
        let mut matched_count = 0;
        let mut unmatched_delta = 0i64;
        let mut unmatched_count = 0;
        let mut sign_violations = 0;

        for (idx, (f, i)) in final_syns.iter().zip(initial.iter()).enumerate() {
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

            // Matched synapse check: Virtual Group A (source < 48) to matched L4 target (128..176)
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
                 name, total_delta, sign_violations, m_mean, u_mean);

        assert_eq!(
            sign_violations, 0,
            "{} has Dale's law sign violations!",
            name
        );
        if name != "Sparse Clean Static Check" {
            assert!(
                total_delta > 0,
                "{} must show nonzero weight plasticity!",
                name
            );
        }

        // Matched co-activation should lead to higher potentiation or less depression than unmatched
        if name == "Balanced Functional" {
            assert!(m_mean > u_mean, "Balanced candidate must demonstrate positive matched bias co-activation separation (matched_mean={:.4} > unmatched_mean={:.4})!", m_mean, u_mean);
        }

        (total_delta, m_mean, u_mean)
    };

    println!("Verifying GSOP plasticity outcomes...");
    verify_gsop_changes(&s_flat_syn, &s_flat_syn, "Sparse Clean Static Check"); // sanity
    let b_gsop_res = verify_gsop_changes(&b_flat_syn, &b_flat_syn_gsop, "Balanced Functional");

    // Replay Hard Gates Verification
    let verify_gates = |met: &ReplayMetrics, name: &str| {
        println!("{}: Replay verification gates check:", name);
        println!(
            "  Silence Ticks = {}, Runaway Ticks = {}",
            met.silence_ticks, met.runaway_ticks
        );
        println!(
            "  Vm Health above -25mV occurrences = {}",
            met.vm_health_above_neg25
        );

        assert!(
            met.silence_ticks < max_ticks - 100,
            "{} collapsed into total silence!",
            name
        );
        assert!(
            met.runaway_ticks < max_ticks / 2,
            "{} collapsed into pathological runaway!",
            name
        );
        let limit = max_ticks * topo.somas.len() * 30 / 100;
        assert!(
            met.vm_health_above_neg25 <= limit,
            "{} exceeded Vm health threshold bounds!",
            name
        );
    };

    verify_gates(&s_static_met, "Sparse Clean");
    verify_gates(&d_static_met, "Dense Stress");
    verify_gates(&b_static_met, "Balanced Functional");

    println!("All Functional Replay Replay gates passed successfully!");

    // 3. Serialize plot data
    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("docs");
    artifacts_dir.push("engine");
    artifacts_dir.push("research");
    artifacts_dir.push("archive");
    artifacts_dir.push("2026-07-06_growth_v2_functional_replay_v0_5");
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let output_path = artifacts_dir.join("growth_v2_functional_replay_plot_data.json");
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
        "balanced_axons": b_axons.iter().map(|a| {
            serde_json::json!({
                "soma_id": a.soma_id,
                "branches": a.branches.iter().map(|b| {
                    b.iter().map(|seg| [seg.x, seg.y, seg.z]).collect::<Vec<_>>()
                }).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "sparse_synapses": s_flat_syn.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "weight": s.weight,
                "type": get_projection_type(soma_variants[s.source_soma_id as usize], soma_variants[s.target_soma_id as usize])
            })
        }).collect::<Vec<_>>(),
        "dense_synapses": d_flat_syn.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "weight": s.weight,
                "type": get_projection_type(soma_variants[s.source_soma_id as usize], soma_variants[s.target_soma_id as usize])
            })
        }).collect::<Vec<_>>(),
        "balanced_synapses": b_flat_syn_static.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "weight": s.weight,
                "fatigue": s.fatigue,
                "type": get_projection_type(soma_variants[s.source_soma_id as usize], soma_variants[s.target_soma_id as usize])
            })
        }).collect::<Vec<_>>(),
        "balanced_static_firing": b_static_met.firing_rates,
        "balanced_static_active": b_static_met.active_fractions,
        "balanced_static_vm": b_static_met.mean_threshold_distances,
        "balanced_static_fatigue": b_static_met.mean_fatigue,
        "balanced_gsop_firing": b_gsop_met.firing_rates,
        "balanced_gsop_active": b_gsop_met.active_fractions,
        "balanced_gsop_vm": b_gsop_met.mean_threshold_distances,
        "balanced_gsop_fatigue": b_gsop_met.mean_fatigue,
        "balanced_gsop_synapses": b_flat_syn_gsop.iter().map(|s| {
            serde_json::json!({
                "source": s.source_soma_id,
                "target": s.target_soma_id,
                "weight": s.weight,
                "fatigue": s.fatigue,
                "type": get_projection_type(soma_variants[s.source_soma_id as usize], soma_variants[s.target_soma_id as usize])
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "max_ticks": max_ticks,
            "sparse_static_silence_ticks": s_static_met.silence_ticks,
            "sparse_static_runaway_ticks": s_static_met.runaway_ticks,
            "sparse_static_vm_above_neg25": s_static_met.vm_health_above_neg25,
            "dense_static_silence_ticks": d_static_met.silence_ticks,
            "dense_static_runaway_ticks": d_static_met.runaway_ticks,
            "dense_static_vm_above_neg25": d_static_met.vm_health_above_neg25,
            "balanced_static_silence_ticks": b_static_met.silence_ticks,
            "balanced_static_runaway_ticks": b_static_met.runaway_ticks,
            "balanced_static_vm_above_neg25": b_static_met.vm_health_above_neg25,
            "balanced_gsop_total_abs_delta": b_gsop_res.0,
        },
        "balanced_gsop_matched_mean": b_gsop_res.1,
        "balanced_gsop_unmatched_mean": b_gsop_res.2,
    });

    serde_json::to_writer_pretty(file, &plot_json).unwrap();
    println!("Wrote detailed plot data to {}", output_path.display());
    println!("=== Growth v2 Functional Replay Verification Complete ===");
}
