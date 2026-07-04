#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]

use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use layout::VariantParameters;

fn get_artifacts_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // to crates
    path.pop(); // to AxiEngine
    path.pop(); // to workflow
    path.push("artifacts");
    path
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

struct LoggedTick {
    tick: usize,
    voltage_pre: i32,
    voltage_candidate: i32,
    voltage_post: i32,
    timer_before: i32,
    timer_after: i32,
    was_refractory: bool,
    threshold_offset: i32,
    effective_threshold: i32,
    i_syn: i32,
    i_ext: i32,
    is_glif_spike: bool,
    is_heartbeat_spike: bool,
    final_spike: bool,
    burst_count: u32,
}

fn simulate_full_neuron_replay(
    var: &VariantParameters,
    i_ext: &[i32],
    sim_ticks: usize,
    enable_heartbeat: bool,
) -> (Vec<LoggedTick>, Vec<usize>) {
    let mut voltage = var.rest_potential;
    let mut thresh_offset = 0i32;
    let mut refractory_timer = 0i32;
    let mut burst_count = 0u32;
    let mut spike_ticks = Vec::new();
    let mut logged_ticks = Vec::with_capacity(sim_ticks);

    let v_reset = var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);

    for t in 0..sim_ticks {
        let ext_current = if t < i_ext.len() { i_ext[t] } else { 0 };

        let voltage_pre = voltage;
        let timer_before = refractory_timer;
        let was_refractory = timer_before > 0;

        // 1. Homeostasis decay (runs first, updates thresh_offset for this tick)
        thresh_offset = physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

        let current_thresh_offset = thresh_offset;
        let eff_threshold = var.threshold.wrapping_add(current_thresh_offset);

        // 2. Refractory and voltage update
        let mut is_glif = false;
        let voltage_candidate;

        if refractory_timer > 0 {
            refractory_timer -= 1;
            voltage_candidate = voltage; // Unchanged during refractory
        } else {
            // Leak and integration (external current added directly)
            voltage_candidate = physics::update_glif_voltage(
                voltage,
                ext_current, // i_total = i_syn (0) + i_ext
                var.rest_potential,
                current_thresh_offset,
                var.leak_shift as i32,
                var.adaptive_leak_gain as i32,
                var.adaptive_leak_min_shift,
                var.adaptive_mode as i32,
            );

            is_glif =
                physics::is_glif_spike(voltage_candidate, var.threshold, current_thresh_offset);
            if !is_glif {
                voltage = voltage_candidate;
            }
        }

        // 3. Heartbeat check
        let is_heartbeat = if enable_heartbeat {
            physics::heartbeat_spike(t as u64, var.heartbeat_m, 0)
        } else {
            false
        };

        let final_spike = is_glif || is_heartbeat;

        if final_spike {
            voltage = v_reset;
            refractory_timer = var.refractory_period as i32;
            thresh_offset = thresh_offset.wrapping_add(var.homeostasis_penalty);
            burst_count = burst_count.saturating_add(1);
            spike_ticks.push(t);
        }

        let timer_after = refractory_timer;
        let voltage_post = voltage;

        logged_ticks.push(LoggedTick {
            tick: t,
            voltage_pre,
            voltage_candidate,
            voltage_post,
            timer_before,
            timer_after,
            was_refractory,
            threshold_offset: current_thresh_offset,
            effective_threshold: eff_threshold,
            i_syn: 0,
            i_ext: ext_current,
            is_glif_spike: is_glif,
            is_heartbeat_spike: is_heartbeat,
            final_spike,
            burst_count,
        });
    }

    (logged_ticks, spike_ticks)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExperimentalRecoveryMode {
    Baseline,
    BoundedInertia {
        max_inertia_uv: i32,
        inertia_shift: i32,
    },
    HeartbeatProductionControl,
    HeartbeatGated,
    HeartbeatGatedDischarge,
    BoundedInertiaPlusHeartbeatDischarge {
        max_inertia_uv: i32,
        inertia_shift: i32,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ExperimentalLoggedTick {
    pub tick: usize,
    pub voltage_pre: i32,
    pub voltage_candidate: i32,
    pub voltage_post: i32,
    pub timer_before: i32,
    pub timer_after: i32,
    pub was_refractory: bool,
    pub threshold_offset: i32,
    pub effective_threshold: i32,
    pub i_syn: i32,
    pub i_ext: i32,
    pub is_glif_spike: bool,
    pub is_heartbeat_spike: bool,
    pub final_spike: bool,
    pub burst_count: u32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatStats {
    pub raw_events: usize,
    pub raw_during_refractory: usize,
    pub accepted_events: usize,
    pub accepted_during_refractory: usize,
    pub suppressed_by_gating: usize,
}

pub fn full_neuron_replay_314900022_simulate_experimental(
    var: &VariantParameters,
    i_ext: &[i32],
    sim_ticks: usize,
    mode: ExperimentalRecoveryMode,
) -> (Vec<ExperimentalLoggedTick>, Vec<usize>, HeartbeatStats) {
    let mut voltage = var.rest_potential;
    let mut thresh_offset = 0i32;
    let mut refractory_timer = 0i32;
    let mut burst_count = 0u32;
    let mut spike_ticks = Vec::new();
    let mut logged_ticks = Vec::with_capacity(sim_ticks);

    let mut hb_stats = HeartbeatStats {
        raw_events: 0,
        raw_during_refractory: 0,
        accepted_events: 0,
        accepted_during_refractory: 0,
        suppressed_by_gating: 0,
    };

    let v_reset = var.rest_potential.wrapping_sub(var.ahp_amplitude as i32);

    for t in 0..sim_ticks {
        let ext_current = if t < i_ext.len() { i_ext[t] } else { 0 };

        let voltage_pre = voltage;
        let timer_before = refractory_timer;
        let was_refractory = timer_before > 0;

        // 1. Homeostasis decay (runs first)
        thresh_offset = physics::homeostasis_decay(thresh_offset, var.homeostasis_decay as i32);

        let current_thresh_offset = thresh_offset;
        let eff_threshold = var.threshold.wrapping_add(current_thresh_offset);

        // 2. Refractory and voltage update
        let mut is_glif = false;
        let voltage_candidate;

        if refractory_timer > 0 {
            refractory_timer -= 1;
            voltage_candidate = voltage; // Unchanged during refractory
        } else {
            voltage_candidate = physics::update_glif_voltage(
                voltage,
                ext_current,
                var.rest_potential,
                current_thresh_offset,
                var.leak_shift as i32,
                var.adaptive_leak_gain as i32,
                var.adaptive_leak_min_shift,
                var.adaptive_mode as i32,
            );

            is_glif =
                physics::is_glif_spike(voltage_candidate, var.threshold, current_thresh_offset);
            if !is_glif {
                voltage = voltage_candidate;
            }
        }

        // 3. Heartbeat check
        let enable_heartbeat = !matches!(
            mode,
            ExperimentalRecoveryMode::Baseline | ExperimentalRecoveryMode::BoundedInertia { .. }
        );

        let is_heartbeat = if enable_heartbeat {
            physics::heartbeat_spike(t as u64, var.heartbeat_m, 0)
        } else {
            false
        };

        // Apply heartbeat policies
        let mut is_heartbeat_final = is_heartbeat;
        let mut heartbeat_discharges = false;

        match mode {
            ExperimentalRecoveryMode::Baseline
            | ExperimentalRecoveryMode::BoundedInertia { .. } => {
                is_heartbeat_final = false;
            }
            ExperimentalRecoveryMode::HeartbeatProductionControl => {
                heartbeat_discharges = is_heartbeat_final;
            }
            ExperimentalRecoveryMode::HeartbeatGated => {
                if was_refractory {
                    is_heartbeat_final = false;
                }
                heartbeat_discharges = false;
            }
            ExperimentalRecoveryMode::HeartbeatGatedDischarge => {
                if was_refractory {
                    is_heartbeat_final = false;
                }
                heartbeat_discharges = is_heartbeat_final;
            }
            ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge { .. } => {
                if was_refractory {
                    is_heartbeat_final = false;
                }
                heartbeat_discharges = is_heartbeat_final;
            }
        }

        if is_heartbeat {
            hb_stats.raw_events += 1;
            if was_refractory {
                hb_stats.raw_during_refractory += 1;
            }
            if is_heartbeat_final {
                hb_stats.accepted_events += 1;
                if was_refractory {
                    hb_stats.accepted_during_refractory += 1;
                }
            } else {
                hb_stats.suppressed_by_gating += 1;
            }
        }

        let final_spike = is_glif || is_heartbeat_final;

        if final_spike {
            spike_ticks.push(t);
            burst_count = burst_count.saturating_add(1);

            let does_discharge = is_glif || heartbeat_discharges;

            if does_discharge {
                let mut inertia_uv = 0i32;
                match mode {
                    ExperimentalRecoveryMode::BoundedInertia {
                        max_inertia_uv,
                        inertia_shift,
                    }
                    | ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge {
                        max_inertia_uv,
                        inertia_shift,
                    } => {
                        let max_of_zero = std::cmp::max(0, current_thresh_offset);
                        inertia_uv = std::cmp::min(max_inertia_uv, max_of_zero >> inertia_shift);
                    }
                    _ => {}
                }

                voltage = v_reset.wrapping_sub(inertia_uv);
                refractory_timer = var.refractory_period as i32;
                thresh_offset = thresh_offset.wrapping_add(var.homeostasis_penalty);
            }
        }

        let timer_after = refractory_timer;
        let voltage_post = voltage;

        logged_ticks.push(ExperimentalLoggedTick {
            tick: t,
            voltage_pre,
            voltage_candidate,
            voltage_post,
            timer_before,
            timer_after,
            was_refractory,
            threshold_offset: current_thresh_offset,
            effective_threshold: eff_threshold,
            i_syn: 0,
            i_ext: ext_current,
            is_glif_spike: is_glif,
            is_heartbeat_spike: is_heartbeat_final,
            final_spike,
            burst_count,
        });
    }

    (logged_ticks, spike_ticks, hb_stats)
}

#[test]
fn run_full_neuron_replay_verification() {
    println!("=== Starting Phase 0 & 1 Replay Verification ===");

    // 1. EPHYS_PROBE_01 Parity Verification
    // Core parameters for Martionotti style neuron from audit
    let mut ephys_var = VariantParameters {
        threshold: -50000,
        rest_potential: -70000,
        leak_shift: 10,
        homeostasis_penalty: 1200,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 2,
        refractory_period: 14,
        fatigue_capacity: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 5000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 1,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: 0,
    };

    let sim_ticks = 10000;
    let i_ext_ephys = vec![350; sim_ticks];

    // Mode A: no_homeostasis
    ephys_var.homeostasis_penalty = 0;
    ephys_var.homeostasis_decay = 0;
    ephys_var.ahp_amplitude = 0;
    let (trace_a, spikes_a) =
        simulate_full_neuron_replay(&ephys_var, &i_ext_ephys, sim_ticks, false);
    println!("Mode A (no_homeostasis) spikes: {}", spikes_a.len());
    assert_eq!(spikes_a.len(), 137);

    // Mode B: homeostasis_only
    ephys_var.homeostasis_penalty = 1200;
    ephys_var.homeostasis_decay = 2;
    ephys_var.ahp_amplitude = 0;
    let (trace_b, spikes_b) =
        simulate_full_neuron_replay(&ephys_var, &i_ext_ephys, sim_ticks, false);
    println!("Mode B (homeostasis_only) spikes: {:?}", spikes_b);

    // Read Python Mode B spikes from CSV
    let artifacts_dir = get_artifacts_dir();
    let py_trace_path = artifacts_dir.join("ephys_probe_01_replay_trace.csv");
    if py_trace_path.exists() {
        let py_content = fs::read_to_string(&py_trace_path).unwrap();
        let mut py_spikes = Vec::new();
        let mut py_lines = py_content.lines();
        let _header = py_lines.next().unwrap();
        let mut t = 0;
        for line in py_lines {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            let py_b_v: f64 = parts[3].parse().unwrap();
            // In python, when a spike occurs, voltage is set to rest_potential - ahp_amplitude = -70.0 mV
            if py_b_v == -70.0 {
                // Since it is held at -70.0 during refractory, only count the transition
                if py_spikes.is_empty() || t > py_spikes.last().unwrap() + 14 {
                    py_spikes.push(t);
                }
            }
            t += 1;
        }
        println!("Python Mode B spikes: {:?}", py_spikes);
    }
    assert_eq!(spikes_b.len(), 61);

    // Mode C: ahp_only
    ephys_var.homeostasis_penalty = 0;
    ephys_var.homeostasis_decay = 0;
    ephys_var.ahp_amplitude = 5000;
    let (trace_c, spikes_c) =
        simulate_full_neuron_replay(&ephys_var, &i_ext_ephys, sim_ticks, false);
    println!("Mode C (ahp_only) spikes: {}", spikes_c.len());
    assert_eq!(spikes_c.len(), 115);

    // Mode D: ahp_plus_homeostasis
    ephys_var.homeostasis_penalty = 1200;
    ephys_var.homeostasis_decay = 2;
    ephys_var.ahp_amplitude = 5000;
    let (trace_d, spikes_d) =
        simulate_full_neuron_replay(&ephys_var, &i_ext_ephys, sim_ticks, false);
    println!("Mode D (ahp_plus_homeostasis) spikes: {}", spikes_d.len());
    assert_eq!(spikes_d.len(), 58);

    // Write Mode D trace
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();
    let csv_file =
        File::create(artifacts_dir.join("full_neuron_replay_314900022_trace.csv")).unwrap();
    let mut writer = BufWriter::new(csv_file);
    writeln!(
        writer,
        "tick,voltage_pre,voltage_candidate,voltage_post,timer_before,timer_after,was_refractory,threshold_offset,effective_threshold,i_syn,i_ext,is_glif_spike,is_heartbeat_spike,final_spike,burst_count"
    )
    .unwrap();

    for t in &trace_d {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            t.tick,
            t.voltage_pre,
            t.voltage_candidate,
            t.voltage_post,
            t.timer_before,
            t.timer_after,
            t.was_refractory,
            t.threshold_offset,
            t.effective_threshold,
            t.i_syn,
            t.i_ext,
            t.is_glif_spike as u8,
            t.is_heartbeat_spike as u8,
            t.final_spike as u8,
            t.burst_count
        )
        .unwrap();
    }
    println!("Output trace saved.");

    // Validate 100% trace parity line-by-line against Python baseline CSV
    let py_trace_path = artifacts_dir.join("ephys_probe_01_replay_trace.csv");
    if py_trace_path.exists() {
        println!("Loading Python trace baseline from: {:?}", py_trace_path);
        let py_content = fs::read_to_string(&py_trace_path).unwrap();
        let mut py_lines = py_content.lines();
        // Skip header
        let _header = py_lines.next().unwrap();

        let mut t_idx = 0;
        for py_line in py_lines {
            if py_line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = py_line.split(',').collect();
            let csv_tick: usize = parts[0].parse().unwrap();
            assert_eq!(csv_tick, t_idx);

            let py_a_v: f64 = parts[1].parse().unwrap();
            let py_a_th: f64 = parts[2].parse().unwrap();
            let py_b_v: f64 = parts[3].parse().unwrap();
            let py_b_th: f64 = parts[4].parse().unwrap();
            let py_c_v: f64 = parts[5].parse().unwrap();
            let py_c_th: f64 = parts[6].parse().unwrap();
            let py_d_v: f64 = parts[7].parse().unwrap();
            let py_d_th: f64 = parts[8].parse().unwrap();

            // Mode A
            let r_a_v = trace_a[t_idx].voltage_pre as f64 / 1000.0;
            let r_a_th = trace_a[t_idx].effective_threshold as f64 / 1000.0;
            assert!(
                (r_a_v - py_a_v).abs() < 1e-4,
                "Mode A V mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_a_v,
                py_a_v
            );
            assert!(
                (r_a_th - py_a_th).abs() < 1e-4,
                "Mode A Th mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_a_th,
                py_a_th
            );

            // Mode B
            let r_b_v = trace_b[t_idx].voltage_pre as f64 / 1000.0;
            let r_b_th = trace_b[t_idx].effective_threshold as f64 / 1000.0;
            assert!(
                (r_b_v - py_b_v).abs() < 1e-4,
                "Mode B V mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_b_v,
                py_b_v
            );
            assert!(
                (r_b_th - py_b_th).abs() < 1e-4,
                "Mode B Th mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_b_th,
                py_b_th
            );

            // Mode C
            let r_c_v = trace_c[t_idx].voltage_pre as f64 / 1000.0;
            let r_c_th = trace_c[t_idx].effective_threshold as f64 / 1000.0;
            assert!(
                (r_c_v - py_c_v).abs() < 1e-4,
                "Mode C V mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_c_v,
                py_c_v
            );
            assert!(
                (r_c_th - py_c_th).abs() < 1e-4,
                "Mode C Th mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_c_th,
                py_c_th
            );

            // Mode D
            let r_d_v = trace_d[t_idx].voltage_pre as f64 / 1000.0;
            let r_d_th = trace_d[t_idx].effective_threshold as f64 / 1000.0;
            assert!(
                (r_d_v - py_d_v).abs() < 1e-4,
                "Mode D V mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_d_v,
                py_d_v
            );
            assert!(
                (r_d_th - py_d_th).abs() < 1e-4,
                "Mode D Th mismatch at tick {}: rust={}, py={}",
                t_idx,
                r_d_th,
                py_d_th
            );

            t_idx += 1;
        }
        println!(
            "Trace-level 100% mathematical parity with Python baseline verified successfully."
        );
    } else {
        panic!(
            "Error: Python trace baseline file not found at {:?}.\n\
             Please run the baseline generation script first:\n\
             .venv/bin/python3 docs/engine/research/archive/_active/full_neuron_replay_314900022/scripts/ephys_probe_01_replay_audit.py",
            py_trace_path
        );
    }

    // 2. Specimen 314900022 f-I Sweep Baseline
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let var_visl4 = load_variant(path_visl4);
    println!(
        "Loaded VISl4 parameters: rest_potential={}, threshold={}, leak_shift={}",
        var_visl4.rest_potential, var_visl4.threshold, var_visl4.leak_shift
    );

    // Amplitudes to test (in pA)
    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let current_scale = 35.0; // Balanced winner scale (0.035 * 1000)

    let mut summary_file =
        File::create(artifacts_dir.join("full_neuron_replay_314900022_summary.json")).unwrap();
    let mut summary_data = String::new();
    summary_data.push_str("[\n");

    for (idx, &amp) in amps.iter().enumerate() {
        let step_current = (amp as f64 * current_scale) as i32;
        let ticks = 3000;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        let (ticks_log, spikes) = simulate_full_neuron_replay(&var_visl4, &i_ext, ticks, false);

        // Count spikes in the stimulus window [1000, 2000]
        let stim_spikes = spikes
            .iter()
            .filter(|&&t| (1000..2000).contains(&t))
            .count();

        // Calculate ISIs in stimulus window
        let stim_spike_times: Vec<usize> = spikes
            .iter()
            .cloned()
            .filter(|&t| (1000..2000).contains(&t))
            .collect();
        let isis: Vec<usize> = stim_spike_times.windows(2).map(|w| w[1] - w[0]).collect();
        let first_isi = isis.first().cloned();
        let last_isi = isis.last().cloned();
        let isi_growth_ratio = if let (Some(f), Some(l)) = (first_isi, last_isi) {
            l as f64 / f as f64
        } else {
            1.0
        };

        println!("Amp: {} pA | Spikes in stim window: {} | Spikes total: {} | First ISI: {:?} | Last ISI: {:?}", 
                 amp, stim_spikes, spikes.len(), first_isi, last_isi);

        let first_isi_str = match first_isi {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        };
        let last_isi_str = match last_isi {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        };

        let entry = format!(
            "  {{\n    \"stimulus_pa\": {},\n    \"spike_count\": {},\n    \"first_isi_ticks\": {},\n    \"last_isi_ticks\": {},\n    \"isi_growth_ratio\": {:.4}\n  }}",
            amp, stim_spikes, first_isi_str, last_isi_str, isi_growth_ratio
        );
        summary_data.push_str(&entry);
        if idx + 1 < amps.len() {
            summary_data.push_str(",\n");
        } else {
            summary_data.push('\n');
        }

        // Write trace for 190 pA to a separate csv for plotting
        if amp == 190 {
            let trace_file =
                File::create(artifacts_dir.join("full_neuron_replay_314900022_sweep_190.csv"))
                    .unwrap();
            let mut tr_writer = BufWriter::new(trace_file);
            writeln!(
                tr_writer,
                "tick,voltage_pre,voltage_candidate,voltage_post,timer_before,timer_after,was_refractory,threshold_offset,effective_threshold,i_syn,i_ext,is_glif_spike,is_heartbeat_spike,final_spike,burst_count"
            )
            .unwrap();

            for t in ticks_log {
                writeln!(
                    tr_writer,
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    t.tick,
                    t.voltage_pre,
                    t.voltage_candidate,
                    t.voltage_post,
                    t.timer_before,
                    t.timer_after,
                    t.was_refractory,
                    t.threshold_offset,
                    t.effective_threshold,
                    t.i_syn,
                    t.i_ext,
                    t.is_glif_spike as u8,
                    t.is_heartbeat_spike as u8,
                    t.final_spike as u8,
                    t.burst_count
                )
                .unwrap();
            }
        }
    }
    summary_data.push_str("]\n");
    summary_file.write_all(summary_data.as_bytes()).unwrap();
}

fn mode_to_string(mode: &ExperimentalRecoveryMode) -> String {
    match mode {
        ExperimentalRecoveryMode::Baseline => "baseline".to_string(),
        ExperimentalRecoveryMode::BoundedInertia {
            max_inertia_uv,
            inertia_shift,
        } => {
            format!("inertia_max{}_shift{}", max_inertia_uv, inertia_shift)
        }
        ExperimentalRecoveryMode::HeartbeatProductionControl => "heartbeat_production".to_string(),
        ExperimentalRecoveryMode::HeartbeatGated => "heartbeat_gated".to_string(),
        ExperimentalRecoveryMode::HeartbeatGatedDischarge => {
            "heartbeat_gated_discharge".to_string()
        }
        ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge {
            max_inertia_uv,
            inertia_shift,
        } => {
            format!("combined_max{}_shift{}", max_inertia_uv, inertia_shift)
        }
    }
}

#[test]
fn run_full_neuron_replay_phase3_experiments() {
    println!("=== Starting Phase 3 Experimental Recovery Modes ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // Clean up old phase 3 files to avoid stale results
    if let Ok(entries) = fs::read_dir(&artifacts_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("full_neuron_replay_314900022_phase3_") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let var_visl4 = load_variant(path_visl4);

    let heartbeat_m_spontaneous = physics::compile_stochastic_heartbeat_threshold(500);

    let mut modes = vec![
        ExperimentalRecoveryMode::Baseline,
        ExperimentalRecoveryMode::HeartbeatProductionControl,
        ExperimentalRecoveryMode::HeartbeatGated,
        ExperimentalRecoveryMode::HeartbeatGatedDischarge,
    ];

    let max_inertia_list = vec![1000, 2500, 5000];
    let inertia_shift_list = vec![3, 4, 5];
    for &max_i in &max_inertia_list {
        for &shift in &inertia_shift_list {
            modes.push(ExperimentalRecoveryMode::BoundedInertia {
                max_inertia_uv: max_i,
                inertia_shift: shift,
            });
        }
    }

    modes.push(
        ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge {
            max_inertia_uv: 2500,
            inertia_shift: 4,
        },
    );

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let current_scale = 35.0;

    for mode in &modes {
        let mode_str = mode_to_string(mode);
        let mut mode_var = var_visl4;

        match mode {
            ExperimentalRecoveryMode::Baseline
            | ExperimentalRecoveryMode::BoundedInertia { .. } => {
                mode_var.heartbeat_m = 0;
            }
            _ => {
                mode_var.heartbeat_m = heartbeat_m_spontaneous;
            }
        }

        let mut fi_data = Vec::new();

        for &amp in &amps {
            let step_current = (amp as f64 * current_scale) as i32;
            let ticks = 3000;
            let mut i_ext = vec![0; ticks];
            i_ext[1000..2000].fill(step_current);

            let (ticks_log, spikes, hb_stats) =
                full_neuron_replay_314900022_simulate_experimental(&mode_var, &i_ext, ticks, *mode);

            let stim_spikes = spikes
                .iter()
                .filter(|&&t| (1000..2000).contains(&t))
                .count();

            let stim_spike_times: Vec<usize> = spikes
                .iter()
                .cloned()
                .filter(|&t| (1000..2000).contains(&t))
                .collect();
            let isis: Vec<usize> = stim_spike_times.windows(2).map(|w| w[1] - w[0]).collect();
            let first_isi = isis.first().cloned();
            let last_isi = isis.last().cloned();
            let isi_growth_ratio = if let (Some(f), Some(l)) = (first_isi, last_isi) {
                l as f64 / f as f64
            } else {
                1.0
            };

            let stim_ticks_log: Vec<&ExperimentalLoggedTick> = ticks_log
                .iter()
                .filter(|t| (1000..2000).contains(&t.tick))
                .collect();

            let voltages: Vec<f64> = stim_ticks_log
                .iter()
                .map(|t| t.voltage_pre as f64 / 1000.0)
                .collect();
            let min_v = voltages.iter().fold(f64::INFINITY, |a: f64, &b| a.min(b));
            let max_v = voltages
                .iter()
                .fold(f64::NEG_INFINITY, |a: f64, &b| a.max(b));
            let mean_v = if !voltages.is_empty() {
                voltages.iter().sum::<f64>() / voltages.len() as f64
            } else {
                0.0
            };

            let th_offsets: Vec<f64> = stim_ticks_log
                .iter()
                .map(|t| t.threshold_offset as f64 / 1000.0)
                .collect();
            let max_th_offset = th_offsets.iter().fold(0.0f64, |a: f64, &b| a.max(b));
            let mean_th_offset = if !th_offsets.is_empty() {
                th_offsets.iter().sum::<f64>() / th_offsets.len() as f64
            } else {
                0.0
            };

            fi_data.push(serde_json::json!({
                "stimulus_pa": amp,
                "spike_count": stim_spikes,
                "first_isi_ticks": first_isi,
                "last_isi_ticks": last_isi,
                "isi_growth_ratio": isi_growth_ratio,
                "voltage_min_mv": min_v,
                "voltage_max_mv": max_v,
                "voltage_mean_mv": mean_v,
                "threshold_offset_max_mv": max_th_offset,
                "threshold_offset_mean_mv": mean_th_offset,
                "heartbeat_raw_events": hb_stats.raw_events,
                "heartbeat_raw_during_refractory": hb_stats.raw_during_refractory,
                "heartbeat_accepted_events": hb_stats.accepted_events,
                "heartbeat_accepted_during_refractory": hb_stats.accepted_during_refractory,
                "heartbeat_suppressed_by_gating": hb_stats.suppressed_by_gating
            }));
        }

        let json_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase3_fi_sweep_{}.json",
            mode_str
        ));
        let file = File::create(&json_path).unwrap();
        serde_json::to_writer_pretty(file, &fi_data).unwrap();
        println!("Saved Phase 3 f-I Sweep JSON to: {:?}", json_path);
    }

    let ephys_var = VariantParameters {
        threshold: -50000,
        rest_potential: -70000,
        leak_shift: 10,
        homeostasis_penalty: 1200,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 0,
        gsop_potentiation: 0,
        gsop_depression: 0,
        homeostasis_decay: 2,
        refractory_period: 14,
        fatigue_capacity: 0,
        signal_propagation_length: 0,
        is_inhibitory: 0,
        inertia_curve: [0; 8],
        ahp_amplitude: 5000,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 1,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 0,
        d2_affinity: 0,
        heartbeat_m: heartbeat_m_spontaneous,
    };

    let sim_ticks = 10000;
    let i_ext_ephys = vec![350; sim_ticks];

    let trace_modes = vec![
        ExperimentalRecoveryMode::Baseline,
        ExperimentalRecoveryMode::BoundedInertia {
            max_inertia_uv: 2500,
            inertia_shift: 4,
        },
        ExperimentalRecoveryMode::HeartbeatProductionControl,
        ExperimentalRecoveryMode::HeartbeatGated,
        ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge {
            max_inertia_uv: 2500,
            inertia_shift: 4,
        },
    ];

    for mode in &trace_modes {
        let mode_str = mode_to_string(mode);
        let mut mode_var = ephys_var;

        match mode {
            ExperimentalRecoveryMode::Baseline
            | ExperimentalRecoveryMode::BoundedInertia { .. } => {
                mode_var.heartbeat_m = 0;
            }
            _ => {
                mode_var.heartbeat_m = heartbeat_m_spontaneous;
            }
        }

        let (ticks_log, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &mode_var,
            &i_ext_ephys,
            sim_ticks,
            *mode,
        );

        let csv_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase3_trace_{}.csv",
            mode_str
        ));
        let file = File::create(&csv_path).unwrap();
        let mut tr_writer = BufWriter::new(file);
        writeln!(
            tr_writer,
            "tick,voltage_pre,voltage_candidate,voltage_post,timer_before,timer_after,was_refractory,threshold_offset,effective_threshold,i_syn,i_ext,is_glif_spike,is_heartbeat_spike,final_spike,burst_count"
        )
        .unwrap();

        for t in ticks_log {
            writeln!(
                tr_writer,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                t.tick,
                t.voltage_pre,
                t.voltage_candidate,
                t.voltage_post,
                t.timer_before,
                t.timer_after,
                t.was_refractory,
                t.threshold_offset,
                t.effective_threshold,
                t.i_syn,
                t.i_ext,
                t.is_glif_spike as u8,
                t.is_heartbeat_spike as u8,
                t.final_spike as u8,
                t.burst_count
            )
            .unwrap();
        }
        println!("Saved Phase 3 trace CSV to: {:?}", csv_path);
    }

    let stress_amps = vec![0, 15, 190];
    let hb_modes = vec![
        ExperimentalRecoveryMode::HeartbeatProductionControl,
        ExperimentalRecoveryMode::HeartbeatGated,
        ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        ExperimentalRecoveryMode::BoundedInertiaPlusHeartbeatDischarge {
            max_inertia_uv: 2500,
            inertia_shift: 4,
        },
    ];

    let mut stress_results = Vec::new();

    for mode in &hb_modes {
        let mode_str = mode_to_string(mode);
        let mut mode_var = var_visl4;
        mode_var.heartbeat_m = heartbeat_m_spontaneous;

        for &amp in &stress_amps {
            let step_current = (amp as f64 * current_scale) as i32;
            let ticks = 5000;
            let mut i_ext = vec![0; ticks];
            i_ext[1000..4000].fill(step_current);

            let (ticks_log, spikes, hb_stats) =
                full_neuron_replay_314900022_simulate_experimental(&mode_var, &i_ext, ticks, *mode);

            let stim_spikes = spikes
                .iter()
                .filter(|&&t| (1000..4000).contains(&t))
                .count();

            let stim_ticks_log: Vec<&ExperimentalLoggedTick> = ticks_log
                .iter()
                .filter(|t| (1000..4000).contains(&t.tick))
                .collect();
            let th_offsets: Vec<f64> = stim_ticks_log
                .iter()
                .map(|t| t.threshold_offset as f64 / 1000.0)
                .collect();
            let max_th_offset = th_offsets.iter().fold(0.0f64, |a: f64, &b| a.max(b));
            let mean_th_offset = if !th_offsets.is_empty() {
                th_offsets.iter().sum::<f64>() / th_offsets.len() as f64
            } else {
                0.0
            };
            let silence = stim_spikes == 0;
            let runaway = stim_spikes > 300 || max_th_offset > 120.0;

            stress_results.push(serde_json::json!({
                "mode": mode_str,
                "stimulus_pa": amp,
                "spike_count": stim_spikes,
                "heartbeat_raw_events": hb_stats.raw_events,
                "heartbeat_raw_during_refractory": hb_stats.raw_during_refractory,
                "heartbeat_accepted_events": hb_stats.accepted_events,
                "heartbeat_accepted_during_refractory": hb_stats.accepted_during_refractory,
                "heartbeat_suppressed_by_gating": hb_stats.suppressed_by_gating,
                "threshold_offset_max_mv": max_th_offset,
                "threshold_offset_mean_mv": mean_th_offset,
                "silence": silence,
                "runaway": runaway
            }));
        }
    }

    let stress_json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase3_heartbeat_stress.json");
    let file = File::create(&stress_json_path).unwrap();
    serde_json::to_writer_pretty(file, &stress_results).unwrap();
    println!(
        "Saved Phase 3 Heartbeat Stress Test JSON to: {:?}",
        stress_json_path
    );
}
