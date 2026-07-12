#![cfg(all(feature = "full-chain-probe", feature = "mvp-cpu-replay"))]

use compute_api::{
    ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload, VramHandle,
};
use compute_cpu::{CpuBackend, CpuBackendConfig};
use layout::VariantParameters;
use std::fs;
use std::path::PathBuf;
use test_harness::{MvpAxonBuffer, MvpStateBuffer};
use types::SomaFlags;

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0xffffff) as f32 / 16777216.0
    }
    fn range(&mut self, min: usize, max: usize) -> usize {
        assert!(max >= min);
        let diff = max - min + 1;
        min + (self.next_u32() as usize % diff)
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
    if path.exists() {
        path
    } else {
        panic!(
            "Could not find modernized profile for {} at {}!",
            name,
            path.display()
        );
    }
}

fn load_variant(path: PathBuf) -> VariantParameters {
    let content = fs::read_to_string(&path)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cue {
    Left,
    Right,
}

struct TrialResult {
    choice: Option<Cue>,
    correct: bool,
    l4_spikes_a: usize,
    l4_spikes_b: usize,
}

fn execute_trial(
    cue: Cue,
    use_dopamine: bool,
    rng: &mut SimpleRng,
    tick_offset: &mut u64,
    backend: &mut CpuBackend,
    handle: VramHandle,
    n: usize,
    padded_n: usize,
    total_axons: usize,
    virt_count: usize,
    max_spikes: u32,
    mapped_somas: &[u32],
    incoming_padded: &mut [u32],
    out_spikes: &mut [u32],
    out_counts: &mut [u32],
    l4_group_a_pref: &[bool],
) -> TrialResult {
    let trial_ticks = 330;
    let batch_ticks = 5;
    let num_batches = trial_ticks / batch_ticks;

    let mut next_batch_dopamine: i16 = 0;
    let mut total_l4_spikes = vec![0; padded_n];

    let group_a_l4_range = 0..(n / 2);
    let get_trial_spikes = |l4_spikes: &[usize]| -> (usize, usize) {
        let mut a_spikes = 0;
        let mut b_spikes = 0;
        for i in group_a_l4_range.clone() {
            if l4_group_a_pref[i] {
                a_spikes += l4_spikes[i];
            } else {
                b_spikes += l4_spikes[i];
            }
        }
        (a_spikes, b_spikes)
    };

    for batch_idx in 0..num_batches {
        let mut batch_spikes = vec![0; padded_n];

        for t_local in 0..batch_ticks {
            let tick = *tick_offset;
            *tick_offset += 1;
            let mut incoming_count = 0;

            // Inputs (cadence: ticks 0, 2, ..., 18 of the trial)
            let trial_t = batch_idx * batch_ticks + t_local;
            if trial_t < 20 && trial_t % 2 == 0 {
                match cue {
                    Cue::Left => {
                        for src in n..(n + virt_count / 2) {
                            if rng.next_f32() < 0.1100 {
                                incoming_padded[incoming_count] = src as u32;
                                incoming_count += 1;
                            }
                        }
                    }
                    Cue::Right => {
                        for src in (n + virt_count / 2)..total_axons {
                            if rng.next_f32() < 0.1100 {
                                incoming_padded[incoming_count] = src as u32;
                                incoming_count += 1;
                            }
                        }
                    }
                }
            }

            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick,
                v_seg: 1,
                dopamine: next_batch_dopamine,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: mapped_somas,
                output_spikes: out_spikes,
                output_spike_counts: out_counts,
            };

            backend.run_day_batch(handle, cmd).unwrap();

            let count = out_counts[0] as usize;
            for &id in &out_spikes[0..count] {
                if id < padded_n as u32 {
                    batch_spikes[id as usize] += 1;
                    total_l4_spikes[id as usize] += 1;
                }
            }
        }

        // At the end of the batch, calculate closed-loop dopamine for the next batch
        if use_dopamine {
            let (a_sp, b_sp) = get_trial_spikes(&total_l4_spikes);
            if a_sp > b_sp {
                // Choice = Left
                next_batch_dopamine = if cue == Cue::Left { 50 } else { -50 };
            } else if b_sp > a_sp {
                // Choice = Right
                next_batch_dopamine = if cue == Cue::Right { 50 } else { -50 };
            } else {
                next_batch_dopamine = 0;
            }
        } else {
            next_batch_dopamine = 0;
        }
    }

    let (total_a, total_b) = get_trial_spikes(&total_l4_spikes);
    let choice = if total_a > total_b {
        Some(Cue::Left)
    } else if total_b > total_a {
        Some(Cue::Right)
    } else {
        None
    };

    // Cue Association: Cue Left -> Choice Left, Cue Right -> Choice Right
    let correct = match cue {
        Cue::Left => choice == Some(Cue::Left),
        Cue::Right => choice == Some(Cue::Right),
    };
    if total_a > 0 || total_b > 0 {
        println!(
            "    TRIAL: cue={:?} choice={:?} spikes_a={} spikes_b={} correct={}",
            cue, choice, total_a, total_b, correct
        );
    }
    TrialResult {
        choice,
        correct,
        l4_spikes_a: total_a,
        l4_spikes_b: total_b,
    }
}

struct ExperimentResult {
    baseline_acc: f64,
    eval_acc: f64,
    matched_avg: f64,
    unmatched_avg: f64,
}

fn run_lp4_experiment(
    seed: u64,
    condition: &str, // "normal", "da_off", "plasticity_off"
) -> ExperimentResult {
    let n = 256;
    let padded_n = 256;
    let total_axons = 384;
    let virt_count = 128;

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    let mut var_visp5 = load_variant(path_visp5);
    let mut var_visp23 = load_variant(path_visp23);

    // Apply baseline homeostasis overrides matching full_neuron_replay.rs calibration
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;

    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;

    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;

    // Apply winner overrides to L4
    var_visl4.fatigue_capacity = 18;
    var_visl4.gsop_potentiation = 240;
    var_visl4.gsop_depression = 68;

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    let mut rng = SimpleRng::new(seed);

    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let axons_buf = MvpAxonBuffer::new(total_axons);

    // Initialize Somas
    for i in 0..padded_n {
        let type_id = if i < n / 2 {
            0 // L4
        } else if i < 3 * n / 4 {
            2 // L23
        } else if i < n {
            1 // L5
        } else {
            0
        };
        let var = &variant_table[type_id];
        state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
        state_buf.write_soma_voltage(i, var.rest_potential);
        state_buf.write_soma_to_axon(i, i as u32);
    }

    // Define coordinates
    let mut coordinates = Vec::new();
    for i in 0..padded_n {
        let (x, y, z) = if i < n / 2 {
            ((i % 16) as f32 * 12.0, (i / 16) as f32 * 12.0, 10.0f32)
        } else if i < 3 * n / 4 {
            (
                ((i - n / 2) % 8) as f32 * 18.0,
                ((i - n / 2) / 8) as f32 * 18.0,
                20.0f32,
            )
        } else if i < n {
            (
                ((i - 3 * n / 4) % 8) as f32 * 18.0,
                ((i - 3 * n / 4) / 8) as f32 * 18.0,
                30.0f32,
            )
        } else {
            (0.0f32, 0.0f32, 0.0f32)
        };
        coordinates.push((x, y, z));
    }

    // Determine Group Preferences
    let mut l4_group_a_pref = vec![false; n / 2];
    for i in 0..(n / 2) {
        let mut rng_local = SimpleRng::new(500 + i as u64);
        l4_group_a_pref[i] = rng_local.next_u32() % 2 == 0;
    }

    // Topology Edges
    let mut edges = Vec::new();

    // Virtual -> L4
    let virt_w = 3500;
    for dest in 0..(n / 2) {
        let pref_a = l4_group_a_pref[dest];
        let mut matched_candidates = Vec::new();
        let mut unmatched_candidates = Vec::new();

        for src in n..total_axons {
            let virt_idx = src - n;
            let is_virt_a = virt_idx < virt_count / 2;
            let d = if pref_a == is_virt_a {
                rng.range(50, 150) as f32
            } else {
                rng.range(200, 400) as f32
            };

            if pref_a == is_virt_a {
                matched_candidates.push((src, d));
            } else {
                unmatched_candidates.push((src, d));
            }
        }

        matched_candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        unmatched_candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // 7 matched synapses
        for k in 0..7 {
            edges.push((matched_candidates[k].0, dest, virt_w));
        }
        // 5 unmatched synapses
        for k in 0..5 {
            edges.push((unmatched_candidates[k].0, dest, virt_w));
        }
    }

    // Exc L4 -> L23
    let exc_w_l4_l23 = 3000;
    for dest in (n / 2)..(3 * n / 4) {
        let fan_in_target = rng.range(8, 24);
        let mut candidates = Vec::new();
        for src in 0..(n / 2) {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..fan_in_target {
            edges.push((candidates[k].0, dest, exc_w_l4_l23));
        }
    }

    // Exc L4 -> L5
    let exc_w_l4_l5 = 5000;
    for dest in (3 * n / 4)..n {
        let mut candidates = Vec::new();
        for src in 0..(n / 2) {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..16 {
            edges.push((candidates[k].0, dest, exc_w_l4_l5));
        }
    }

    // Inh L23 -> L4
    let inh_w_l23_l4 = -900;
    for dest in 0..(n / 2) {
        let fan_in_target = rng.range(8, 24);
        let mut candidates = Vec::new();
        for src in (n / 2)..(3 * n / 4) {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..fan_in_target {
            edges.push((candidates[k].0, dest, inh_w_l23_l4));
        }
    }

    // Inh L23 -> L5
    let inh_w_l23_l5 = -1250;
    for dest in (3 * n / 4)..n {
        let fan_in_target = rng.range(6, 18);
        let mut candidates = Vec::new();
        for src in (n / 2)..(3 * n / 4) {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..fan_in_target {
            edges.push((candidates[k].0, dest, inh_w_l23_l5));
        }
    }

    // Inh L23 -> L23
    for dest in (n / 2)..(3 * n / 4) {
        let fan_in_target = rng.range(4, 12);
        let mut candidates = Vec::new();
        for src in (n / 2)..(3 * n / 4) {
            if src == dest {
                continue;
            }
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..fan_in_target.min(candidates.len()) {
            edges.push((candidates[k].0, dest, -2000));
        }
    }

    // Exc L5 -> L23
    for dest in (n / 2)..(3 * n / 4) {
        let fan_in_target = rng.range(8, 24);
        let mut candidates = Vec::new();
        for src in (3 * n / 4)..n {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            candidates.push((src, d));
        }
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for k in 0..fan_in_target {
            edges.push((candidates[k].0, dest, 3000));
        }
    }

    // Write edges
    let mut dest_fan_in = vec![0; padded_n];
    for &(src, dest, w) in &edges {
        let slot = dest_fan_in[dest];
        assert!(slot < 128, "Soma {} exceeded 128 synapses", dest);
        let target = types::PackedTarget::pack(src as u32, 0).0;
        state_buf.write_dendrite_target(slot, dest, target);
        state_buf.write_dendrite_weight(slot, dest, w << 16);
        dest_fan_in[dest] += 1;
    }

    let backend_config = CpuBackendConfig {
        thread_count: Some(1),
    };
    let mut backend = CpuBackend::new(backend_config).unwrap();
    let spec = ShardAllocSpec {
        padded_n: padded_n as u32,
        total_axons: total_axons as u32,
        total_ghosts: 0,
        virtual_offset: 0,
    };
    let handle = backend.alloc_shard(spec).unwrap();
    backend
        .upload_shard(
            handle,
            ShardUpload {
                state_blob: state_buf.as_bytes(),
                axons_blob: axons_buf.as_bytes(),
                variant_table: &variant_table,
            },
        )
        .unwrap();

    let max_spikes = total_axons as u32;
    let mut out_spikes = vec![0u32; max_spikes as usize];
    let mut out_counts = vec![0u32; 1];
    let mapped_somas: Vec<u32> = (0..padded_n as u32).collect();

    let mut incoming_padded = vec![0u32; max_spikes as usize];
    let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
    let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

    let mut trial_rng = SimpleRng::new(seed + 1000);
    let mut tick_offset = 0u64;

    // 1. Untrained Baseline Evaluation
    physics::set_plasticity_enabled(false);
    let mut correct_baseline = 0;
    for trial_idx in 0..100 {
        let cue = if trial_idx % 2 == 0 {
            Cue::Left
        } else {
            Cue::Right
        };
        let res = execute_trial(
            cue,
            false,
            &mut trial_rng,
            &mut tick_offset,
            &mut backend,
            handle,
            n,
            padded_n,
            total_axons,
            virt_count,
            max_spikes,
            &mapped_somas,
            &mut incoming_padded,
            &mut out_spikes,
            &mut out_counts,
            &l4_group_a_pref,
        );
        if res.correct {
            correct_baseline += 1;
        }
    }
    let baseline_acc = correct_baseline as f64 / 100.0;

    // 2. Training Phase
    let enable_plasticity = condition != "plasticity_off";
    let enable_da = condition == "normal";
    physics::set_plasticity_enabled(enable_plasticity);

    for trial_idx in 0..500 {
        let cue = if trial_idx % 2 == 0 {
            Cue::Left
        } else {
            Cue::Right
        };
        let _res = execute_trial(
            cue,
            enable_da,
            &mut trial_rng,
            &mut tick_offset,
            &mut backend,
            handle,
            n,
            padded_n,
            total_axons,
            virt_count,
            max_spikes,
            &mapped_somas,
            &mut incoming_padded,
            &mut out_spikes,
            &mut out_counts,
            &l4_group_a_pref,
        );
    }

    // Read snapshot after training to check weights
    backend
        .debug_snapshot(
            handle,
            ShardSnapshotMut {
                state_blob: &mut snap_state,
                axons_blob: &mut snap_axons,
            },
        )
        .unwrap();
    let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());
    let mut sum_w_matched = 0i64;
    let mut sum_w_unmatched = 0i64;
    let mut count_matched = 0;
    let mut count_unmatched = 0;
    for dest in 0..(n / 2) {
        let pref_a = l4_group_a_pref[dest];
        for slot in 0..128 {
            let target = snap_state_buf.read_dendrite_target(slot, dest);
            if types::PackedTarget(target).is_inactive() {
                continue;
            }
            if let Some((src_id, _)) = types::PackedTarget(target).unpack() {
                if src_id >= n as u32 && src_id < total_axons as u32 {
                    let w = snap_state_buf.read_dendrite_weight(slot, dest) as i64 >> 16;
                    let virt_idx = src_id as usize - n;
                    let is_virt_a = virt_idx < virt_count / 2;
                    if pref_a == is_virt_a {
                        sum_w_matched += w;
                        count_matched += 1;
                    } else {
                        sum_w_unmatched += w;
                        count_unmatched += 1;
                    }
                }
            }
        }
    }
    println!(
        "  [{}] Post-train: matched avg = {:.2} ({}/{}), unmatched avg = {:.2} ({}/{})",
        condition,
        sum_w_matched as f64 / count_matched.max(1) as f64,
        sum_w_matched,
        count_matched,
        sum_w_unmatched as f64 / count_unmatched.max(1) as f64,
        sum_w_unmatched,
        count_unmatched
    );

    // 3. Trained Evaluation Phase
    physics::set_plasticity_enabled(false);
    let mut correct_eval = 0;
    for trial_idx in 0..100 {
        let cue = if trial_idx % 2 == 0 {
            Cue::Left
        } else {
            Cue::Right
        };
        let res = execute_trial(
            cue,
            false,
            &mut trial_rng,
            &mut tick_offset,
            &mut backend,
            handle,
            n,
            padded_n,
            total_axons,
            virt_count,
            max_spikes,
            &mapped_somas,
            &mut incoming_padded,
            &mut out_spikes,
            &mut out_counts,
            &l4_group_a_pref,
        );
        if res.correct {
            correct_eval += 1;
        }
    }
    let eval_acc = correct_eval as f64 / 100.0;

    ExperimentResult {
        baseline_acc,
        eval_acc,
        matched_avg: sum_w_matched as f64 / count_matched.max(1) as f64,
        unmatched_avg: sum_w_unmatched as f64 / count_unmatched.max(1) as f64,
    }
}

#[test]
#[ignore]
fn test_external_task_learning_lp4() {
    let seeds = [42, 100, 2026];

    let mut normal_results = Vec::new();
    let mut da_off_results = Vec::new();
    let mut plasticity_off_results = Vec::new();

    for &seed in &seeds {
        println!("============================================================");
        println!("Running LP-4 Task Learning Experiment for Seed: {}", seed);

        // Condition A: Normal
        let res_norm = run_lp4_experiment(seed, "normal");
        normal_results.push((res_norm.baseline_acc, res_norm.eval_acc));
        println!(
            "  Condition Normal: Baseline = {:.2}%, Evaluation = {:.2}%",
            res_norm.baseline_acc * 100.0,
            res_norm.eval_acc * 100.0
        );

        // Condition B: DA-off
        let res_da = run_lp4_experiment(seed, "da_off");
        da_off_results.push((res_da.baseline_acc, res_da.eval_acc));
        println!(
            "  Condition DA-off: Baseline = {:.2}%, Evaluation = {:.2}%",
            res_da.baseline_acc * 100.0,
            res_da.eval_acc * 100.0
        );

        // Condition C: Plasticity-off
        let res_plast = run_lp4_experiment(seed, "plasticity_off");
        plasticity_off_results.push((res_plast.baseline_acc, res_plast.eval_acc));
        println!(
            "  Condition Plasticity-off: Baseline = {:.2}%, Evaluation = {:.2}%",
            res_plast.baseline_acc * 100.0,
            res_plast.eval_acc * 100.0
        );
    }

    let avg_normal_eval = normal_results.iter().map(|r| r.1).sum::<f64>() / seeds.len() as f64;
    let avg_da_off_eval = da_off_results.iter().map(|r| r.1).sum::<f64>() / seeds.len() as f64;
    let avg_plast_off_eval =
        plasticity_off_results.iter().map(|r| r.1).sum::<f64>() / seeds.len() as f64;

    println!("------------------------------------------------------------");
    println!("LP-4 Task Learning Experiment Complete.");
    println!(
        "Average Normal Trained Evaluation Accuracy: {:.2}%",
        avg_normal_eval * 100.0
    );
    println!(
        "Average DA-off Trained Evaluation Accuracy: {:.2}%",
        avg_da_off_eval * 100.0
    );
    println!(
        "Average Plasticity-off Trained Evaluation Accuracy: {:.2}%",
        avg_plast_off_eval * 100.0
    );

    // Assert C4 Success Criteria
    assert!(
        avg_normal_eval >= 0.70,
        "Seed-average Normal Trained Evaluation Accuracy ({:.2}%) is below the success threshold (>= 70%)",
        avg_normal_eval * 100.0
    );

    let da_diff = avg_normal_eval - avg_da_off_eval;
    assert!(
        da_diff >= 0.15,
        "Difference between Normal and DA-off ({:.2}%) is below target (>= 15%)",
        da_diff * 100.0
    );

    let plast_diff = avg_normal_eval - avg_plast_off_eval;
    assert!(
        plast_diff >= 0.15,
        "Difference between Normal and Plasticity-off ({:.2}%) is below target (>= 15%)",
        plast_diff * 100.0
    );

    println!("All C4 Task Learning success criteria met successfully!");
}

#[test]
fn test_network_weight_differentiation_probe() {
    let seed = 42;

    // 1. Plasticity-off control: weights must remain exactly equal to initial weight (3500.0)
    let res_plast = run_lp4_experiment(seed, "plasticity_off");
    assert_eq!(
        res_plast.matched_avg, 3500.0,
        "Plasticity-off matched synapses must remain flat"
    );
    assert_eq!(
        res_plast.unmatched_avg, 3500.0,
        "Plasticity-off unmatched synapses must remain flat"
    );

    // 2. Normal condition: unmatched must depress (unmatched < 3500.0) and matched-unmatched gap must be >= 100 mass units
    let res_norm = run_lp4_experiment(seed, "normal");

    let matched_delta_mass = (res_norm.matched_avg - 3500.0) * 65536.0;
    let unmatched_delta_mass = (res_norm.unmatched_avg - 3500.0) * 65536.0;
    let gap_mass = matched_delta_mass - unmatched_delta_mass;

    println!(
        "  [L053 Probe] matched delta mass = {:.2}, unmatched delta mass = {:.2}, gap = {:.2}",
        matched_delta_mass, unmatched_delta_mass, gap_mass
    );

    // Sanity assertion: some postsynaptic updates must have occurred (weights changed)
    assert!(
        res_norm.matched_avg != 3500.0 || res_norm.unmatched_avg != 3500.0,
        "Plastic updates must occur under Normal condition"
    );

    // Unmatched must depress (unmatched_delta_mass < 0)
    assert!(
        unmatched_delta_mass < 0.0,
        "Unmatched synapses must depress under competitive LTD (unmatched delta = {:.2} < 0)",
        unmatched_delta_mass
    );

    // Matched must be greater than unmatched, and the gap must satisfy the prereg threshold of >= 100 mass units
    assert!(
        gap_mass >= 100.0,
        "Matched-unmatched differentiation gap ({:.2}) must be >= 100 mass units",
        gap_mass
    );
}
