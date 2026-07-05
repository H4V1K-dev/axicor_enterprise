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

#[allow(clippy::too_many_arguments)]
fn simulate_phase4_fi_sweep(
    base_var: &VariantParameters,
    leak_shift: i32,
    rest_potential: i32,
    current_scale: f64,
    adaptive_gain: i32,
    adaptive_min_shift: i32,
    adaptive_mode: i32,
    amps: &[i32],
) -> serde_json::Value {
    let mut var = *base_var;
    var.leak_shift = leak_shift as u32;
    var.rest_potential = rest_potential;
    var.adaptive_leak_gain = adaptive_gain as u16;
    var.adaptive_leak_min_shift = adaptive_min_shift;
    var.adaptive_mode = adaptive_mode as u8;
    var.heartbeat_m = 0;

    let mut fi_data = Vec::new();

    for &amp in amps {
        let step_current = (amp as f64 * current_scale) as i32;
        let ticks = 3000;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        let (ticks_log, spikes, _) = full_neuron_replay_314900022_simulate_experimental(
            &var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );

        let stim_spikes = spikes
            .iter()
            .filter(|&&t| (1000..2000).contains(&t))
            .count();
        let stim_spike_ticks: Vec<usize> = spikes
            .iter()
            .cloned()
            .filter(|&t| (1000..2000).contains(&t))
            .collect();
        let first_spike_latency = stim_spike_ticks.first().map(|&t| t - 1000);

        let isis: Vec<usize> = stim_spike_ticks.windows(2).map(|w| w[1] - w[0]).collect();
        let first_isi = isis.first().cloned();
        let last_isi = isis.last().cloned();
        let isi_growth = if let (Some(f), Some(l)) = (first_isi, last_isi) {
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
        let min_v = voltages.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_v = voltages.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean_v = if !voltages.is_empty() {
            voltages.iter().sum::<f64>() / voltages.len() as f64
        } else {
            0.0
        };

        fi_data.push(serde_json::json!({
            "stimulus_pa": amp,
            "spike_count": stim_spikes,
            "first_spike_latency_ticks": first_spike_latency,
            "first_isi_ticks": first_isi,
            "last_isi_ticks": last_isi,
            "isi_growth_ratio": isi_growth,
            "min_v_mv": min_v,
            "max_v_mv": max_v,
            "mean_v_mv": mean_v,
        }));
    }

    serde_json::json!({
        "leak_shift": leak_shift,
        "rest_potential_uv": rest_potential,
        "current_scale": current_scale,
        "adaptive_leak_gain": adaptive_gain,
        "adaptive_leak_min_shift": adaptive_min_shift,
        "adaptive_mode": adaptive_mode,
        "fi_data": fi_data
    })
}

#[test]
fn run_full_neuron_replay_phase4_experiments() {
    println!("=== Starting Phase 4 Passive Excitability Calibration ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let var_visl4 = load_variant(path_visl4);

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let default_scale = 35.0;

    // 1. Static Sweep: leak_shift x rest_potential
    let leak_shifts = vec![1, 2, 3, 4, 5, 6, 7, 8, 10]; // 8 is baseline
    let rest_potentials = vec![-70000, -71000, -72000, -73000, -73443]; // uV

    let mut static_sweep_results = Vec::new();
    for &leak in &leak_shifts {
        for &rest in &rest_potentials {
            let res =
                simulate_phase4_fi_sweep(&var_visl4, leak, rest, default_scale, 0, 1, 0, &amps);
            static_sweep_results.push(res);
        }
    }

    let static_json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase4_static_sweep.json");
    let file = File::create(&static_json_path).unwrap();
    serde_json::to_writer_pretty(file, &static_sweep_results).unwrap();
    println!("Saved Phase 4 Static Sweep JSON to: {:?}", static_json_path);

    // 2. Control current_scale Sweep (varying scaling factor on baseline & key leak/rest candidates)
    let current_scales = vec![15.0, 20.0, 25.0, 30.0, 35.0, 40.0];
    let mut scale_sweep_results = Vec::new();
    for &scale in &current_scales {
        // baseline rest & leak
        let res_base = simulate_phase4_fi_sweep(
            &var_visl4,
            var_visl4.leak_shift as i32,
            var_visl4.rest_potential,
            scale,
            0,
            1,
            0,
            &amps,
        );
        scale_sweep_results.push(res_base);

        // leak_shift = 4, rest = -70000
        let res_leak4 = simulate_phase4_fi_sweep(&var_visl4, 4, -70000, scale, 0, 1, 0, &amps);
        scale_sweep_results.push(res_leak4);
    }

    let scale_json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase4_control_scale_sweep.json");
    let file = File::create(&scale_json_path).unwrap();
    serde_json::to_writer_pretty(file, &scale_sweep_results).unwrap();
    println!(
        "Saved Phase 4 Control Scale Sweep JSON to: {:?}",
        scale_json_path
    );

    // 3. Adaptive Leak Subphase Sweep
    let adaptive_gains = vec![0, 1, 2, 4, 8];
    let adaptive_min_shifts = vec![1, 2, 4];
    let adaptive_modes = vec![0, 1];

    let mut adaptive_sweep_results = Vec::new();
    for &leak in &[3, 4, 5, 6, 7, 8, 10] {
        for &rest in &[-70000, -73443] {
            for &gain in &adaptive_gains {
                for &min_shift in &adaptive_min_shifts {
                    for &mode in &adaptive_modes {
                        if mode == 0 && gain > 0 {
                            continue; // skip duplicate disabled modes
                        }
                        let res = simulate_phase4_fi_sweep(
                            &var_visl4,
                            leak,
                            rest,
                            default_scale,
                            gain,
                            min_shift,
                            mode,
                            &amps,
                        );
                        adaptive_sweep_results.push(res);
                    }
                }
            }
        }
    }

    let adaptive_json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase4_adaptive_sweep.json");
    let file = File::create(&adaptive_json_path).unwrap();
    serde_json::to_writer_pretty(file, &adaptive_sweep_results).unwrap();
    println!(
        "Saved Phase 4 Adaptive Sweep JSON to: {:?}",
        adaptive_json_path
    );

    // Save trace CSVs for 190 pA for baseline (leak=8, rest=-73443) and winner candidate (leak=4, rest=-70000)
    let mut baseline_var = var_visl4;
    baseline_var.heartbeat_m = 0;

    let mut candidate_var = var_visl4;
    candidate_var.leak_shift = 4;
    candidate_var.rest_potential = -70000;
    candidate_var.heartbeat_m = 0;

    let ticks = 3000;
    let step_current_190_base = (190.0 * default_scale) as i32;
    let mut i_ext_190 = vec![0; ticks];
    i_ext_190[1000..2000].fill(step_current_190_base);

    let (ticks_log_base_190, _, _) = full_neuron_replay_314900022_simulate_experimental(
        &baseline_var,
        &i_ext_190,
        ticks,
        ExperimentalRecoveryMode::HeartbeatGatedDischarge,
    );
    let (ticks_log_cand_190, _, _) = full_neuron_replay_314900022_simulate_experimental(
        &candidate_var,
        &i_ext_190,
        ticks,
        ExperimentalRecoveryMode::HeartbeatGatedDischarge,
    );

    let trace_base_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase4_trace_baseline_190.csv");
    let file = File::create(&trace_base_path).unwrap();
    let mut writer = BufWriter::new(file);
    writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
    for t in ticks_log_base_190 {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{}",
            t.tick,
            t.voltage_pre,
            t.voltage_candidate,
            t.voltage_post,
            t.threshold_offset,
            t.effective_threshold,
            t.i_ext,
            t.final_spike as u8
        )
        .unwrap();
    }

    let trace_cand_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase4_trace_candidate_190.csv");
    let file = File::create(&trace_cand_path).unwrap();
    let mut writer = BufWriter::new(file);
    writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
    for t in ticks_log_cand_190 {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{}",
            t.tick,
            t.voltage_pre,
            t.voltage_candidate,
            t.voltage_post,
            t.threshold_offset,
            t.effective_threshold,
            t.i_ext,
            t.final_spike as u8
        )
        .unwrap();
    }

    println!("Phase 4 Rust simulations complete.");
}

#[allow(clippy::too_many_arguments)]
fn simulate_phase5_fi_sweep(
    base_var: &VariantParameters,
    penalty: i32,
    decay: u16,
    amps: &[i32],
    current_scale: f64,
) -> serde_json::Value {
    let mut var = *base_var;
    var.leak_shift = 4;
    var.rest_potential = -70000;
    var.homeostasis_penalty = penalty;
    var.homeostasis_decay = decay;
    var.adaptive_leak_gain = 0;
    var.adaptive_leak_min_shift = 1;
    var.adaptive_mode = 0;
    var.heartbeat_m = 0;

    let mut fi_data = Vec::new();

    for &amp in amps {
        let step_current = (amp as f64 * current_scale) as i32;
        let ticks = 3000;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        let (ticks_log, spikes, _) = full_neuron_replay_314900022_simulate_experimental(
            &var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );

        let stim_spikes = spikes
            .iter()
            .filter(|&&t| (1000..2000).contains(&t))
            .count();
        let stim_spike_ticks: Vec<usize> = spikes
            .iter()
            .cloned()
            .filter(|&t| (1000..2000).contains(&t))
            .collect();
        let first_spike_latency = stim_spike_ticks.first().map(|&t| t - 1000);

        let isis: Vec<usize> = stim_spike_ticks.windows(2).map(|w| w[1] - w[0]).collect();
        let first_isi = isis.first().cloned();
        let last_isi = isis.last().cloned();
        let isi_growth = if let (Some(f), Some(l)) = (first_isi, last_isi) {
            l as f64 / f as f64
        } else {
            1.0
        };

        let adaptation_index = if isis.len() >= 2 {
            let mut sum = 0.0;
            for window in isis.windows(2) {
                let diff = window[1] as f64 - window[0] as f64;
                let add = window[1] as f64 + window[0] as f64;
                if add > 0.0 {
                    sum += diff / add;
                }
            }
            sum / (isis.len() - 1) as f64
        } else {
            0.0
        };

        let stim_ticks_log: Vec<&ExperimentalLoggedTick> = ticks_log
            .iter()
            .filter(|t| (1000..2000).contains(&t.tick))
            .collect();

        let voltages: Vec<f64> = stim_ticks_log
            .iter()
            .map(|t| t.voltage_pre as f64 / 1000.0)
            .collect();
        let min_v = voltages.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_v = voltages.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean_v = if !voltages.is_empty() {
            voltages.iter().sum::<f64>() / voltages.len() as f64
        } else {
            0.0
        };

        let th_offsets: Vec<f64> = stim_ticks_log
            .iter()
            .map(|t| t.threshold_offset as f64 / 1000.0)
            .collect();
        let max_th_offset = th_offsets.iter().fold(0.0f64, |a, &b| a.max(b));
        let mean_th_offset = if !th_offsets.is_empty() {
            th_offsets.iter().sum::<f64>() / th_offsets.len() as f64
        } else {
            0.0
        };

        fi_data.push(serde_json::json!({
            "stimulus_pa": amp,
            "spike_count": stim_spikes,
            "first_spike_latency_ticks": first_spike_latency,
            "first_isi_ticks": first_isi,
            "last_isi_ticks": last_isi,
            "isi_growth_ratio": isi_growth,
            "adaptation_index": adaptation_index,
            "min_v_mv": min_v,
            "max_v_mv": max_v,
            "mean_v_mv": mean_v,
            "threshold_offset_max_mv": max_th_offset,
            "threshold_offset_mean_mv": mean_th_offset,
        }));
    }

    serde_json::json!({
        "homeostasis_penalty": penalty,
        "homeostasis_decay": decay,
        "leak_shift": 4,
        "rest_potential_uv": -70000,
        "fi_data": fi_data
    })
}

#[test]
fn run_full_neuron_replay_phase5_experiments() {
    println!("=== Starting Phase 5 SFA & Homeostasis Calibration ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let var_visl4 = load_variant(path_visl4);

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let default_scale = 35.0;

    let penalties = vec![400, 800, 1200, 1600, 1940, 2400, 3200];
    let decays = vec![1, 2, 3, 4, 6, 8];

    let mut homeostasis_sweep_results = Vec::new();

    for &pen in &penalties {
        for &dec in &decays {
            let res = simulate_phase5_fi_sweep(&var_visl4, pen, dec, &amps, default_scale);
            homeostasis_sweep_results.push(res);
        }
    }

    let json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase5_homeostasis_sweep.json");
    let file = File::create(&json_path).unwrap();
    serde_json::to_writer_pretty(file, &homeostasis_sweep_results).unwrap();
    println!("Saved Phase 5 Homeostasis Sweep JSON to: {:?}", json_path);

    // Save trace CSVs for baseline (penalty=1940, decay=2) and winner candidate (penalty=1940, decay=4)
    let trace_amps = vec![90, 150, 190];

    let mut base_var = var_visl4;
    base_var.leak_shift = 4;
    base_var.rest_potential = -70000;
    base_var.homeostasis_penalty = 1940;
    base_var.homeostasis_decay = 2;
    base_var.adaptive_leak_min_shift = 1;
    base_var.adaptive_leak_gain = 0;
    base_var.adaptive_mode = 0;
    base_var.heartbeat_m = 0;

    let mut cand_var = var_visl4;
    cand_var.leak_shift = 4;
    cand_var.rest_potential = -70000;
    cand_var.homeostasis_penalty = 1940;
    cand_var.homeostasis_decay = 4;
    cand_var.adaptive_leak_min_shift = 1;
    cand_var.adaptive_leak_gain = 0;
    cand_var.adaptive_mode = 0;
    cand_var.heartbeat_m = 0;

    for &amp in &trace_amps {
        let ticks = 3000;
        let step_current = (amp as f64 * default_scale) as i32;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        // Baseline trace
        let (ticks_log_base, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &base_var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );
        let base_trace_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase5_trace_baseline_{}.csv",
            amp
        ));
        let file = File::create(&base_trace_path).unwrap();
        let mut writer = BufWriter::new(file);
        writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
        for t in ticks_log_base {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{}",
                t.tick,
                t.voltage_pre,
                t.voltage_candidate,
                t.voltage_post,
                t.threshold_offset,
                t.effective_threshold,
                t.i_ext,
                t.final_spike as u8
            )
            .unwrap();
        }

        // Candidate trace
        let (ticks_log_cand, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &cand_var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );
        let cand_trace_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase5_trace_candidate_{}.csv",
            amp
        ));
        let file = File::create(&cand_trace_path).unwrap();
        let mut writer = BufWriter::new(file);
        writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
        for t in ticks_log_cand {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{}",
                t.tick,
                t.voltage_pre,
                t.voltage_candidate,
                t.voltage_post,
                t.threshold_offset,
                t.effective_threshold,
                t.i_ext,
                t.final_spike as u8
            )
            .unwrap();
        }
    }

    println!("Phase 5 Rust simulations complete.");
}

#[allow(clippy::too_many_arguments)]
fn simulate_phase6_fi_sweep(
    base_var: &VariantParameters,
    ahp_amp: u16,
    refractory: u8,
    amps: &[i32],
    current_scale: f64,
) -> serde_json::Value {
    let mut var = *base_var;
    var.leak_shift = 4;
    var.rest_potential = -70000;
    var.homeostasis_penalty = 1940;
    var.homeostasis_decay = 4;
    var.ahp_amplitude = ahp_amp;
    var.refractory_period = refractory;
    var.adaptive_leak_gain = 0;
    var.adaptive_leak_min_shift = 1;
    var.adaptive_mode = 0;
    var.heartbeat_m = 0;

    let mut fi_data = Vec::new();

    for &amp in amps {
        let step_current = (amp as f64 * current_scale) as i32;
        let ticks = 3000;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        let (ticks_log, spikes, _) = full_neuron_replay_314900022_simulate_experimental(
            &var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );

        let stim_spikes = spikes
            .iter()
            .filter(|&&t| (1000..2000).contains(&t))
            .count();
        let stim_spike_ticks: Vec<usize> = spikes
            .iter()
            .cloned()
            .filter(|&t| (1000..2000).contains(&t))
            .collect();
        let first_spike_latency = stim_spike_ticks.first().map(|&t| t - 1000);

        let isis: Vec<usize> = stim_spike_ticks.windows(2).map(|w| w[1] - w[0]).collect();
        let first_isi = isis.first().cloned();
        let last_isi = isis.last().cloned();
        let isi_growth = if let (Some(f), Some(l)) = (first_isi, last_isi) {
            l as f64 / f as f64
        } else {
            1.0
        };

        let adaptation_index = if isis.len() >= 2 {
            let mut sum = 0.0;
            for window in isis.windows(2) {
                let diff = window[1] as f64 - window[0] as f64;
                let add = window[1] as f64 + window[0] as f64;
                if add > 0.0 {
                    sum += diff / add;
                }
            }
            sum / (isis.len() - 1) as f64
        } else {
            0.0
        };

        let stim_ticks_log: Vec<&ExperimentalLoggedTick> = ticks_log
            .iter()
            .filter(|t| (1000..2000).contains(&t.tick))
            .collect();

        let voltages: Vec<f64> = stim_ticks_log
            .iter()
            .map(|t| t.voltage_pre as f64 / 1000.0)
            .collect();
        let min_v = voltages.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_v = voltages.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean_v = if !voltages.is_empty() {
            voltages.iter().sum::<f64>() / voltages.len() as f64
        } else {
            0.0
        };

        let th_offsets: Vec<f64> = stim_ticks_log
            .iter()
            .map(|t| t.threshold_offset as f64 / 1000.0)
            .collect();
        let max_th_offset = th_offsets.iter().fold(0.0f64, |a, &b| a.max(b));
        let mean_th_offset = if !th_offsets.is_empty() {
            th_offsets.iter().sum::<f64>() / th_offsets.len() as f64
        } else {
            0.0
        };

        let mut post_spike_mins = Vec::new();
        let mut violations = 0;

        for &st in &stim_spike_ticks {
            let window_end = (st + 20).min(2000);
            let post_v: Vec<i32> = ticks_log
                .iter()
                .filter(|t| t.tick > st && t.tick <= window_end)
                .map(|t| t.voltage_pre)
                .collect();
            if let Some(&min_uv) = post_v.iter().min() {
                post_spike_mins.push(min_uv);
            }
        }

        let mean_post_spike_min_uv = if !post_spike_mins.is_empty() {
            post_spike_mins.iter().sum::<i32>() as f64 / post_spike_mins.len() as f64
        } else {
            -70000.0
        };
        let ahp_depth_observed_mv = (-70000.0 - mean_post_spike_min_uv) / 1000.0;

        let mut recovery_times = Vec::new();
        for &st in &stim_spike_ticks {
            let window_end = (st + 50).min(2000);
            let ticks_to_rest = ticks_log
                .iter()
                .filter(|t| t.tick > st && t.tick <= window_end)
                .position(|t| t.voltage_pre >= -70000);
            if let Some(pos) = ticks_to_rest {
                recovery_times.push(pos as f64);
            }
        }
        let mean_recovery_ticks = if !recovery_times.is_empty() {
            recovery_times.iter().sum::<f64>() / recovery_times.len() as f64
        } else {
            0.0
        };
        let recovery_slope_mv_per_ms = if mean_recovery_ticks > 0.0 {
            ahp_depth_observed_mv / mean_recovery_ticks
        } else {
            0.0
        };

        for &isi in &isis {
            if isi < refractory as usize {
                violations += 1;
            }
        }

        fi_data.push(serde_json::json!({
            "stimulus_pa": amp,
            "spike_count": stim_spikes,
            "first_spike_latency_ticks": first_spike_latency,
            "first_isi_ticks": first_isi,
            "last_isi_ticks": last_isi,
            "isi_growth_ratio": isi_growth,
            "adaptation_index": adaptation_index,
            "min_v_mv": min_v,
            "max_v_mv": max_v,
            "mean_v_mv": mean_v,
            "threshold_offset_max_mv": max_th_offset,
            "threshold_offset_mean_mv": mean_th_offset,
            "post_spike_min_v_mv": mean_post_spike_min_uv / 1000.0,
            "ahp_depth_observed_mv": ahp_depth_observed_mv,
            "recovery_ticks_to_rest": mean_recovery_ticks,
            "recovery_slope_mv_per_ms": recovery_slope_mv_per_ms,
            "refractory_flat_ticks": refractory,
            "violations": violations,
        }));
    }

    serde_json::json!({
        "ahp_amplitude": ahp_amp,
        "refractory_period": refractory,
        "leak_shift": 4,
        "rest_potential_uv": -70000,
        "homeostasis_penalty": 1940,
        "homeostasis_decay": 4,
        "fi_data": fi_data
    })
}

#[test]
fn run_full_neuron_replay_phase6_experiments() {
    println!("=== Starting Phase 6 AHP & Refractory Calibration ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let var_visl4 = load_variant(path_visl4);

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let default_scale = 35.0;

    let ahp_amplitudes: Vec<u16> = vec![3000, 4000, 5000, 6000, 7000, 8000];
    let refractory_periods: Vec<u8> = vec![8, 10, 12, 14, 16, 20];

    let mut sweep_results = Vec::new();

    for &ahp in &ahp_amplitudes {
        for &refractory in &refractory_periods {
            let res = simulate_phase6_fi_sweep(&var_visl4, ahp, refractory, &amps, default_scale);
            sweep_results.push(res);
        }
    }

    let json_path =
        artifacts_dir.join("full_neuron_replay_314900022_phase6_ahp_refractory_sweep.json");
    let file = File::create(&json_path).unwrap();
    serde_json::to_writer_pretty(file, &sweep_results).unwrap();
    println!("Saved Phase 6 AHP Sweep JSON to: {:?}", json_path);

    // Save trace CSVs for Phase 5 baseline (ahp=5000, refractory=14) and candidate (ahp=6000, refractory=14)
    let trace_amps = vec![90, 150, 190];

    let mut base_var = var_visl4;
    base_var.leak_shift = 4;
    base_var.rest_potential = -70000;
    base_var.homeostasis_penalty = 1940;
    base_var.homeostasis_decay = 4;
    base_var.ahp_amplitude = 5000;
    base_var.refractory_period = 14;
    base_var.adaptive_leak_min_shift = 1;
    base_var.adaptive_leak_gain = 0;
    base_var.adaptive_mode = 0;
    base_var.heartbeat_m = 0;

    let mut cand_var = var_visl4;
    cand_var.leak_shift = 4;
    cand_var.rest_potential = -70000;
    cand_var.homeostasis_penalty = 1940;
    cand_var.homeostasis_decay = 4;
    cand_var.ahp_amplitude = 6000;
    cand_var.refractory_period = 14;
    cand_var.adaptive_leak_min_shift = 1;
    cand_var.adaptive_leak_gain = 0;
    cand_var.adaptive_mode = 0;
    cand_var.heartbeat_m = 0;

    for &amp in &trace_amps {
        let ticks = 3000;
        let step_current = (amp as f64 * default_scale) as i32;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        // Baseline trace
        let (ticks_log_base, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &base_var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );
        let base_trace_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase6_trace_baseline_{}.csv",
            amp
        ));
        let file = File::create(&base_trace_path).unwrap();
        let mut writer = BufWriter::new(file);
        writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
        for t in ticks_log_base {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{}",
                t.tick,
                t.voltage_pre,
                t.voltage_candidate,
                t.voltage_post,
                t.threshold_offset,
                t.effective_threshold,
                t.i_ext,
                t.final_spike as u8
            )
            .unwrap();
        }

        // Candidate trace
        let (ticks_log_cand, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &cand_var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );
        let cand_trace_path = artifacts_dir.join(format!(
            "full_neuron_replay_314900022_phase6_trace_candidate_{}.csv",
            amp
        ));
        let file = File::create(&cand_trace_path).unwrap();
        let mut writer = BufWriter::new(file);
        writeln!(writer, "tick,voltage_pre,voltage_candidate,voltage_post,threshold_offset,effective_threshold,i_ext,final_spike").unwrap();
        for t in ticks_log_cand {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{}",
                t.tick,
                t.voltage_pre,
                t.voltage_candidate,
                t.voltage_post,
                t.threshold_offset,
                t.effective_threshold,
                t.i_ext,
                t.final_spike as u8
            )
            .unwrap();
        }
    }

    println!("Phase 6 Rust simulations complete.");
}

#[allow(clippy::too_many_arguments)]
fn simulate_cross_profile_fi_sweep(
    base_var: &VariantParameters,
    profile_name: &str,
    leak_shift: u32,
    rest_potential: i32,
    homeostasis_penalty: i32,
    homeostasis_decay: u16,
    ahp_amp: u16,
    refractory: u8,
    amps: &[i32],
    current_scale: f64,
) -> serde_json::Value {
    let mut var = *base_var;
    var.leak_shift = leak_shift;
    var.rest_potential = rest_potential;
    var.homeostasis_penalty = homeostasis_penalty;
    var.homeostasis_decay = homeostasis_decay;
    var.ahp_amplitude = ahp_amp;
    var.refractory_period = refractory;
    var.adaptive_leak_min_shift = 1;
    var.adaptive_leak_gain = 0;
    var.adaptive_mode = 0;
    var.heartbeat_m = 0;

    let mut fi_data = Vec::new();
    for &amp in amps {
        let ticks = 3000;
        let step_current = (amp as f64 * current_scale) as i32;
        let mut i_ext = vec![0; ticks];
        i_ext[1000..2000].fill(step_current);

        let (ticks_log, _, _) = full_neuron_replay_314900022_simulate_experimental(
            &var,
            &i_ext,
            ticks,
            ExperimentalRecoveryMode::HeartbeatGatedDischarge,
        );

        let stim_log: Vec<_> = ticks_log
            .iter()
            .filter(|t| t.tick >= 1000 && t.tick < 2000)
            .collect();
        let stim_spikes = stim_log.iter().filter(|t| t.final_spike).count();
        let stim_spike_ticks: Vec<usize> = stim_log
            .iter()
            .filter(|t| t.final_spike)
            .map(|t| t.tick)
            .collect();

        let first_spike_latency = stim_spike_ticks.first().map(|&t| t - 1000);
        let mut isis = Vec::new();
        for i in 0..stim_spike_ticks.len().saturating_sub(1) {
            isis.push(stim_spike_ticks[i + 1] - stim_spike_ticks[i]);
        }

        let first_isi = isis.first().copied();
        let last_isi = isis.last().copied();
        let isi_growth = match (first_isi, last_isi) {
            (Some(f), Some(l)) if f > 0 => l as f64 / f as f64,
            _ => 1.0,
        };

        fi_data.push(serde_json::json!({
            "stimulus_pa": amp,
            "spike_count": stim_spikes,
            "first_spike_latency_ticks": first_spike_latency,
            "first_isi_ticks": first_isi,
            "last_isi_ticks": last_isi,
            "isi_growth_ratio": isi_growth,
        }));
    }

    serde_json::json!({
        "profile_name": profile_name,
        "leak_shift": leak_shift,
        "rest_potential_uv": rest_potential,
        "threshold_uv": var.threshold,
        "homeostasis_penalty": homeostasis_penalty,
        "homeostasis_decay": homeostasis_decay,
        "ahp_amplitude": ahp_amp,
        "refractory_period": refractory,
        "fi_data": fi_data
    })
}

#[test]
fn run_cross_profile_glif_hierarchy_experiments() {
    println!("=== Starting Cross-Profile GLIF Calibration Hierarchy v1 Experiments ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    let profile_names = vec![
        "L4_spiny_VISl4_4",
        "L5_spiny_VISp5_7",
        "L23_aspiny_VISp23_218",
    ];
    let mut profiles = Vec::new();

    for name in &profile_names {
        let path = find_profile_path(name);
        let var = load_variant(path);
        profiles.push((name.to_string(), var));
    }

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let default_scale = 35.0;

    // Phase A: Inventory
    let mut inventory = Vec::new();
    for (name, var) in &profiles {
        inventory.push(serde_json::json!({
            "profile_name": name,
            "threshold_uv": var.threshold,
            "rest_potential_uv": var.rest_potential,
            "leak_shift": var.leak_shift,
            "ahp_amplitude": var.ahp_amplitude,
            "refractory_period": var.refractory_period,
            "homeostasis_penalty": var.homeostasis_penalty,
            "homeostasis_decay": var.homeostasis_decay,
            "has_allen_bio_target": name.contains("L4_spiny"),
        }));
    }

    let inv_path = artifacts_dir.join("cross_profile_glif_inventory.json");
    let file = File::create(&inv_path).unwrap();
    serde_json::to_writer_pretty(file, &inventory).unwrap();
    println!("Saved inventory to: {:?}", inv_path);

    // Phase B: Baseline Replay
    let mut baseline_results = Vec::new();
    for (name, var) in &profiles {
        let res = simulate_cross_profile_fi_sweep(
            var,
            name,
            var.leak_shift,
            var.rest_potential,
            var.homeostasis_penalty,
            var.homeostasis_decay,
            var.ahp_amplitude,
            var.refractory_period,
            &amps,
            default_scale,
        );
        baseline_results.push(res);
    }

    let base_path = artifacts_dir.join("cross_profile_glif_baseline_replay.json");
    let file = File::create(&base_path).unwrap();
    serde_json::to_writer_pretty(file, &baseline_results).unwrap();
    println!("Saved baseline replay to: {:?}", base_path);

    // Phase C1: Passive Membrane Sweep per profile
    let leak_shifts = vec![2u32, 4, 6, 8, 10];
    let rest_potentials = vec![-75000i32, -73000, -70000, -68000];
    let mut passive_results = Vec::new();

    for (name, var) in &profiles {
        for &leak in &leak_shifts {
            for &rest in &rest_potentials {
                let res = simulate_cross_profile_fi_sweep(
                    var,
                    name,
                    leak,
                    rest,
                    var.homeostasis_penalty,
                    var.homeostasis_decay,
                    var.ahp_amplitude,
                    var.refractory_period,
                    &amps,
                    default_scale,
                );
                passive_results.push(res);
            }
        }
    }

    let pass_path = artifacts_dir.join("cross_profile_glif_passive_sweep.json");
    let file = File::create(&pass_path).unwrap();
    serde_json::to_writer_pretty(file, &passive_results).unwrap();
    println!("Saved passive sweep to: {:?}", pass_path);

    // Freeze Phase C1 chosen passive candidates per profile for Phase C2 & C3
    let frozen_passives: Vec<(&str, u32, i32)> = vec![
        ("L4_spiny_VISl4_4", 4u32, -70000i32),
        ("L5_spiny_VISp5_7", 4u32, -73000i32),
        ("L23_aspiny_VISp23_218", 2u32, -68000i32),
    ];

    // Phase C2: Homeostasis Sweep per profile (FROZEN ON PASSED CANDIDATE PER PROFILE)
    let penalties = vec![500i32, 1000, 1500, 1940, 2500];
    let decays = vec![2u16, 4, 6, 9];
    let mut homeostasis_results = Vec::new();

    for (name, var) in &profiles {
        let &(_, frozen_leak, frozen_rest) =
            frozen_passives.iter().find(|(n, _, _)| n == name).unwrap();
        for &pen in &penalties {
            for &dec in &decays {
                let res = simulate_cross_profile_fi_sweep(
                    var,
                    name,
                    frozen_leak,
                    frozen_rest,
                    pen,
                    dec,
                    var.ahp_amplitude,
                    var.refractory_period,
                    &amps,
                    default_scale,
                );
                homeostasis_results.push(res);
            }
        }
    }

    let hom_path = artifacts_dir.join("cross_profile_glif_homeostasis_sweep.json");
    let file = File::create(&hom_path).unwrap();
    serde_json::to_writer_pretty(file, &homeostasis_results).unwrap();
    println!("Saved homeostasis sweep to: {:?}", hom_path);

    // Phase C3: AHP / Refractory Sweep per profile (FROZEN ON PASSED PASSIVE + HOMEOSTASIS CANDIDATE)
    let ahp_amps = vec![3000u16, 5000, 7000];
    let refractories = vec![10u8, 14, 18];
    let mut ahp_results = Vec::new();

    for (name, var) in &profiles {
        let &(_, frozen_leak, frozen_rest) =
            frozen_passives.iter().find(|(n, _, _)| n == name).unwrap();
        for &ahp in &ahp_amps {
            for &refr in &refractories {
                let res = simulate_cross_profile_fi_sweep(
                    var,
                    name,
                    frozen_leak,
                    frozen_rest,
                    1940,
                    4,
                    ahp,
                    refr,
                    &amps,
                    default_scale,
                );
                ahp_results.push(res);
            }
        }
    }

    let ahp_path = artifacts_dir.join("cross_profile_glif_ahp_refractory_sweep.json");
    let file = File::create(&ahp_path).unwrap();
    serde_json::to_writer_pretty(file, &ahp_results).unwrap();
    println!("Saved AHP / refractory sweep to: {:?}", ahp_path);

    println!("Cross-Profile GLIF Hierarchy v1 Rust simulations complete.");
}

#[test]
fn run_class_specific_glif_calibration_experiments() {
    println!("=== Starting Class-Specific GLIF Calibration v1 Experiments ===");
    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    let profile_specs = vec![
        ("L4_spiny_VISl4_4", "L4_spiny"),
        ("L5_spiny_VISp5_7", "L5_spiny"),
        ("L23_aspiny_VISp23_218", "L23_aspiny"),
    ];

    let mut profiles = Vec::new();
    for (name, class) in &profile_specs {
        let path = find_profile_path(name);
        let var = load_variant(path);
        profiles.push((name.to_string(), class.to_string(), var));
    }

    let amps = vec![-100, -50, 0, 30, 40, 50, 70, 90, 110, 130, 150, 190, 200];
    let default_scale = 35.0;

    // Phase A: Profile Cohort Expansion & Inventory
    let mut inventory = Vec::new();
    for (name, class, var) in &profiles {
        let has_bio = name.contains("L4_spiny");
        inventory.push(serde_json::json!({
            "profile_name": name,
            "inferred_class": class,
            "threshold_uv": var.threshold,
            "rest_potential_uv": var.rest_potential,
            "leak_shift": var.leak_shift,
            "ahp_amplitude": var.ahp_amplitude,
            "refractory_period": var.refractory_period,
            "homeostasis_penalty": var.homeostasis_penalty,
            "homeostasis_decay": var.homeostasis_decay,
            "has_exact_bio_target": has_bio,
            "class_status": if has_bio { "exact-target" } else { "single-profile qualitative only" },
        }));
    }

    let inv_path = artifacts_dir.join("class_specific_glif_inventory.json");
    let file = File::create(&inv_path).unwrap();
    serde_json::to_writer_pretty(file, &inventory).unwrap();
    println!("Saved class-specific inventory to: {:?}", inv_path);

    // Phase B: Baseline Replay
    let mut baseline_results = Vec::new();
    for (name, _class, var) in &profiles {
        let res = simulate_cross_profile_fi_sweep(
            var,
            name,
            var.leak_shift,
            var.rest_potential,
            var.homeostasis_penalty,
            var.homeostasis_decay,
            var.ahp_amplitude,
            var.refractory_period,
            &amps,
            default_scale,
        );
        baseline_results.push(res);
    }

    let base_path = artifacts_dir.join("class_specific_glif_baseline_replay.json");
    let file = File::create(&base_path).unwrap();
    serde_json::to_writer_pretty(file, &baseline_results).unwrap();
    println!("Saved class-specific baseline replay to: {:?}", base_path);

    // Phase C: Class-Specific Passive Sweep
    let leak_shifts = vec![1u32, 2, 3, 4, 5, 6, 7, 8, 10];
    let rest_potentials = vec![-76000i32, -74000, -73000, -72000, -70000, -68000, -66000];
    let mut passive_results = Vec::new();

    for (name, _class, var) in &profiles {
        for &leak in &leak_shifts {
            for &rest in &rest_potentials {
                let res = simulate_cross_profile_fi_sweep(
                    var,
                    name,
                    leak,
                    rest,
                    var.homeostasis_penalty,
                    var.homeostasis_decay,
                    var.ahp_amplitude,
                    var.refractory_period,
                    &amps,
                    default_scale,
                );
                passive_results.push(res);
            }
        }
    }

    let pass_path = artifacts_dir.join("class_specific_glif_passive_sweep.json");
    let file = File::create(&pass_path).unwrap();
    serde_json::to_writer_pretty(file, &passive_results).unwrap();
    println!("Saved class-specific passive sweep to: {:?}", pass_path);

    // Phase D: Class-Specific Homeostasis Sweep
    let penalties = vec![500i32, 1000, 1500, 1940, 2500, 3200];
    let decays = vec![1u16, 2, 4, 6, 9];
    let mut homeostasis_results = Vec::new();

    for (name, _class, var) in &profiles {
        for &leak in &leak_shifts {
            for &rest in &rest_potentials {
                for &pen in &penalties {
                    for &dec in &decays {
                        let res = simulate_cross_profile_fi_sweep(
                            var,
                            name,
                            leak,
                            rest,
                            pen,
                            dec,
                            var.ahp_amplitude,
                            var.refractory_period,
                            &amps,
                            default_scale,
                        );
                        homeostasis_results.push(res);
                    }
                }
            }
        }
    }

    let hom_path = artifacts_dir.join("class_specific_glif_homeostasis_sweep.json");
    let file = File::create(&hom_path).unwrap();
    serde_json::to_writer_pretty(file, &homeostasis_results).unwrap();
    println!("Saved class-specific homeostasis sweep to: {:?}", hom_path);

    // Phase E: AHP / Refractory Sanity
    let ahp_amps = vec![3000u16, 5000, 7000];
    let refractories = vec![10u8, 14, 18];
    let mut ahp_results = Vec::new();

    for (name, _class, var) in &profiles {
        for &leak in &leak_shifts {
            for &rest in &rest_potentials {
                for &ahp in &ahp_amps {
                    for &refr in &refractories {
                        let res = simulate_cross_profile_fi_sweep(
                            var,
                            name,
                            leak,
                            rest,
                            1940,
                            4,
                            ahp,
                            refr,
                            &amps,
                            default_scale,
                        );
                        ahp_results.push(res);
                    }
                }
            }
        }
    }

    let ahp_path = artifacts_dir.join("class_specific_glif_ahp_refractory_sanity.json");
    let file = File::create(&ahp_path).unwrap();
    serde_json::to_writer_pretty(file, &ahp_results).unwrap();
    println!(
        "Saved class-specific AHP / refractory sanity to: {:?}",
        ahp_path
    );

    println!("Class-Specific GLIF Calibration v1 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec
)]
fn run_static_microcircuit_physiology_experiments() {
    println!("=== Starting Static Microcircuit Physiology v1 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // 1. Load profiles and setup variants
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    let mut var_visp5 = load_variant(path_visp5);
    let mut var_visp23 = load_variant(path_visp23);

    // Override with calibrated class-specific parameters
    // L4
    var_visl4.leak_shift = 4;
    var_visl4.rest_potential = -70000;
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;
    var_visl4.ahp_amplitude = 5000;
    var_visl4.refractory_period = 14;
    var_visl4.heartbeat_m = 0;
    var_visl4.gsop_potentiation = 0;
    var_visl4.gsop_depression = 0;

    // L5
    var_visp5.leak_shift = 4;
    var_visp5.rest_potential = -76000;
    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;
    var_visp5.ahp_amplitude = 5000;
    var_visp5.refractory_period = 14;
    var_visp5.heartbeat_m = 0;
    var_visp5.gsop_potentiation = 0;
    var_visp5.gsop_depression = 0;

    // L23
    var_visp23.leak_shift = 2;
    var_visp23.rest_potential = -66000;
    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;
    var_visp23.ahp_amplitude = 5000;
    var_visp23.refractory_period = 14;
    var_visp23.heartbeat_m = 0;
    var_visp23.gsop_potentiation = 0;
    var_visp23.gsop_depression = 0;

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // 2. Setup Network layout
    let padded_n = 64;
    let total_axons = 96;

    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let axons_buf = MvpAxonBuffer::new(total_axons);

    // Soma flags, rest potentials, and mapping
    for i in 0..padded_n {
        let type_id = if i < 32 {
            0 // L4
        } else if i < 48 {
            2 // L23
        } else {
            1 // L5
        };
        let var = &variant_table[type_id];

        state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
        state_buf.write_soma_voltage(i, var.rest_potential);
        state_buf.write_soma_to_axon(i, i as u32);
    }

    // Assign spatial coordinates
    let mut coordinates = Vec::new();
    for i in 0..64 {
        let (x, y, z) = if i < 32 {
            ((i % 6) as f32 * 12.0, (i / 6) as f32 * 12.0, 10.0f32)
        } else if i < 48 {
            (
                ((i - 32) % 4) as f32 * 18.0,
                ((i - 32) / 4) as f32 * 18.0,
                20.0f32,
            )
        } else {
            (
                ((i - 48) % 4) as f32 * 18.0,
                ((i - 48) / 4) as f32 * 18.0,
                0.0f32,
            )
        };
        coordinates.push((x, y, z));
    }

    // Determine sparse distance-based connections
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
            (self.next_u32() as f32) / (u32::MAX as f32)
        }
    }

    let mut rng = SimpleRng::new(42);
    let mut edges = Vec::new();
    let mut next_slot = vec![0usize; 64];

    // L4 (0..32) -> L23 (32..48) [excitatory]
    for src in 0..32 {
        for dest in 32..48 {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            if d < 28.0 && rng.next_f32() < 0.45 {
                edges.push((src, dest, 6500i32));
            }
        }
    }

    // L4 (0..32) -> L5 (48..64) [excitatory]
    for src in 0..32 {
        for dest in 48..64 {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            if d < 28.0 && rng.next_f32() < 0.45 {
                edges.push((src, dest, 6500i32));
            }
        }
    }

    // L23 (32..48) -> L4 (0..32) [inhibitory]
    for src in 32..48 {
        for dest in 0..32 {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            if d < 32.0 && rng.next_f32() < 0.55 {
                edges.push((src, dest, -4000i32));
            }
        }
    }

    // L23 (32..48) -> L5 (48..64) [inhibitory]
    for src in 32..48 {
        for dest in 48..64 {
            let (x1, y1, z1) = coordinates[src];
            let (x2, y2, z2) = coordinates[dest];
            let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
            if d < 32.0 && rng.next_f32() < 0.55 {
                edges.push((src, dest, -4000i32));
            }
        }
    }

    // Virtual inputs: Connect each L4 neuron to 10 virtual axons for dense Poisson bombardment
    for i in 0..32 {
        for k in 0..10 {
            let axon_idx = 64 + (i + k) % 32;
            edges.push((axon_idx, i, 6000i32));
        }
    }

    // Write to dendrite slots
    for &(src, dest, weight) in &edges {
        let slot = next_slot[dest];
        assert!(slot < 128, "Dendrite slots exceeded 128 limit!");
        let target = PackedTarget::pack(src as u32, 0).0;
        state_buf.write_dendrite_target(slot, dest, target);
        state_buf.write_dendrite_weight(slot, dest, weight << 16);
        next_slot[dest] += 1;
    }

    // Save topology/connectivity specs
    let mut neurons_json = Vec::new();
    for i in 0..64 {
        let (x, y, z) = coordinates[i];
        let class = if i < 32 {
            "L4_spiny"
        } else if i < 48 {
            "L23_aspiny"
        } else {
            "L5_spiny"
        };
        neurons_json.push(serde_json::json!({
            "id": i,
            "class": class,
            "x": x,
            "y": y,
            "z": z
        }));
    }

    let mut edges_json = Vec::new();
    for &(src, dest, weight) in &edges {
        edges_json.push(serde_json::json!({
            "src": src,
            "dest": dest,
            "weight": weight
        }));
    }

    let conn_json = serde_json::json!({
        "neurons": neurons_json,
        "edges": edges_json
    });
    let conn_path = artifacts_dir.join("static_microcircuit_connectivity.json");
    let file = File::create(&conn_path).unwrap();
    serde_json::to_writer_pretty(file, &conn_json).unwrap();
    println!("Saved microcircuit connectivity to {:?}", conn_path);

    // 3. Simulation run using CPU backend
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

    let mut out_spikes = vec![0u32; padded_n];
    let mut out_counts = vec![0u32; 1];
    let mapped_somas: Vec<u32> = (0..64).collect();

    let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
    let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

    let mut incoming_padded = vec![0u32; 64];
    let mut recent_spikes = VecDeque::new();
    let mut sim_log = Vec::new();

    for tick in 0..4000 {
        let mut incoming_count = 0;
        let p_val = if tick < 1000 {
            0.0
        } else if tick < 2000 {
            0.015 // weak Poisson drive
        } else if tick < 3000 {
            0.060 // moderate Poisson drive
        } else {
            0.090 // structured drive
        };

        if tick < 3000 {
            for axon_idx in 64..96 {
                if rng.next_f32() < p_val {
                    incoming_padded[incoming_count] = axon_idx as u32;
                    incoming_count += 1;
                }
            }
        } else {
            // Structured alternating: Alternate inputs for Group A (64..80) and Group B (80..96)
            let group_a = ((tick - 3000) / 250) % 2 == 0;
            for axon_idx in 64..96 {
                let is_a = axon_idx < 80;
                if is_a == group_a {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }
        }

        out_counts[0] = 0;
        out_spikes.fill(0);

        let cmd = DayBatchCmd {
            sync_batch_ticks: 1,
            tick_base: tick as u64,
            v_seg: 1,
            dopamine: 0,
            input_bitmask: None,
            num_virtual_axons: 0,
            virtual_offset: 0,
            input_words_per_tick: 0,
            incoming_spikes: if incoming_count > 0 {
                Some(&incoming_padded)
            } else {
                None
            },
            incoming_spike_counts: &[incoming_count as u32],
            max_spikes_per_tick: padded_n as u32,
            num_outputs: 64,
            mapped_soma_ids: &mapped_somas,
            output_spikes: &mut out_spikes,
            output_spike_counts: &mut out_counts,
        };

        backend.run_day_batch(handle, cmd).unwrap();

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

        // Process spikes for this tick
        let count = out_counts[0] as usize;
        let mut tick_spikes = vec![false; 64];
        let mut l4_spikes = 0;
        let mut l23_spikes = 0;
        let mut l5_spikes = 0;

        for s_idx in 0..count {
            let soma_id = out_spikes[s_idx] as usize;
            if soma_id < 64 {
                tick_spikes[soma_id] = true;
                if soma_id < 32 {
                    l4_spikes += 1;
                } else if soma_id < 48 {
                    l23_spikes += 1;
                } else {
                    l5_spikes += 1;
                }
            }
        }

        let mut l4_voltages = Vec::new();
        let mut l23_voltages = Vec::new();
        let mut l5_voltages = Vec::new();

        let mut l4_thresh = Vec::new();
        let mut l23_thresh = Vec::new();
        let mut l5_thresh = Vec::new();

        let mut l4_fatigues = Vec::new();
        let mut l23_fatigues = Vec::new();
        let mut l5_fatigues = Vec::new();

        let mut active_exc_weights = 0i64;
        let mut active_inh_weights = 0i64;

        for i in 0..64 {
            let v = snap_state_buf.read_soma_voltage(i);
            let th = snap_state_buf.read_threshold_offset(i);

            let slots_count = next_slot[i];
            let mut fatigue_sum = 0.0;
            let cap = 15.0f64; // fatigue capacity

            for slot in 0..slots_count {
                let f_timer = snap_state_buf.read_dendrite_timer(slot, i);
                fatigue_sum += f_timer as f64 / cap;

                // Track active synaptic inputs to compute active weights sum (E/I balance)
                let target = PackedTarget(snap_state_buf.read_dendrite_target(slot, i));
                if target.is_active() {
                    if let Some((raw_axon, _seg)) = target.unpack() {
                        let is_active = if raw_axon < 64 {
                            tick_spikes[raw_axon as usize]
                        } else {
                            incoming_padded[0..incoming_count].contains(&raw_axon)
                        };

                        if is_active {
                            let w = snap_state_buf.read_dendrite_weight(slot, i) >> 16;
                            if w > 0 {
                                active_exc_weights += w as i64;
                            } else {
                                active_inh_weights += w.abs() as i64;
                            }
                        }
                    }
                }
            }

            let avg_fatigue = if slots_count > 0 {
                fatigue_sum / slots_count as f64
            } else {
                0.0
            };

            if i < 32 {
                l4_voltages.push(v);
                l4_thresh.push(th);
                l4_fatigues.push(avg_fatigue);
            } else if i < 48 {
                l23_voltages.push(v);
                l23_thresh.push(th);
                l23_fatigues.push(avg_fatigue);
            } else {
                l5_voltages.push(v);
                l5_thresh.push(th);
                l5_fatigues.push(avg_fatigue);
            }
        }

        recent_spikes.push_back((l4_spikes, l23_spikes, l5_spikes));
        if recent_spikes.len() > 100 {
            recent_spikes.pop_front();
        }

        let (total_l4, total_l23, total_l5) = recent_spikes
            .iter()
            .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
        let total_all = total_l4 + total_l23 + total_l5;

        let silence_flag = tick >= 1100 && total_all == 0;
        let l4_runaway = total_l4 as f64 / 32.0 > 25.0;
        let l23_runaway = total_l23 as f64 / 16.0 > 25.0;
        let l5_runaway = total_l5 as f64 / 16.0 > 25.0;
        let runaway_flag = l4_runaway || l23_runaway || l5_runaway;

        sim_log.push(serde_json::json!({
            "tick": tick,
            "regime": if tick < 1000 { 1 } else if tick < 2000 { 2 } else if tick < 3000 { 3 } else { 4 },
            "l4_spikes": l4_spikes,
            "l23_spikes": l23_spikes,
            "l5_spikes": l5_spikes,
            "l4_mean_voltage": l4_voltages.iter().sum::<i32>() as f64 / 32.0,
            "l23_mean_voltage": l23_voltages.iter().sum::<i32>() as f64 / 16.0,
            "l5_mean_voltage": l5_voltages.iter().sum::<i32>() as f64 / 16.0,
            "l4_mean_threshold": l4_thresh.iter().sum::<i32>() as f64 / 32.0,
            "l23_mean_threshold": l23_thresh.iter().sum::<i32>() as f64 / 16.0,
            "l5_mean_threshold": l5_thresh.iter().sum::<i32>() as f64 / 16.0,
            "l4_mean_fatigue": l4_fatigues.iter().sum::<f64>() / 32.0,
            "l23_mean_fatigue": l23_fatigues.iter().sum::<f64>() / 16.0,
            "l5_mean_fatigue": l5_fatigues.iter().sum::<f64>() / 16.0,
            "active_exc_weights": active_exc_weights,
            "active_inh_weights": active_inh_weights,
            "silence_flag": silence_flag,
            "runaway_flag": runaway_flag,
            "spiked_neuron_ids": out_spikes[0..count].to_vec()
        }));
    }

    backend.free_shard(handle).unwrap();

    let log_path = artifacts_dir.join("static_microcircuit_simulation_log.json");
    let file = File::create(&log_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log).unwrap();
    println!("Saved microcircuit simulation log to {:?}", log_path);

    println!("Static Microcircuit Physiology v1 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec,
    clippy::manual_range_contains
)]
fn run_static_microcircuit_scale_up_experiments() {
    println!("=== Starting Static Microcircuit Scale-Up v1 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use std::time::Instant;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // 1. Setup deterministic PRNG
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
            (self.next_u32() as f32) / (u32::MAX as f32)
        }
        fn range(&mut self, min: usize, max: usize) -> usize {
            assert!(max >= min);
            let diff = max - min + 1;
            min + (self.next_u32() as usize % diff)
        }
    }

    // Calibrated variant parameters
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    let mut var_visp5 = load_variant(path_visp5);
    let mut var_visp23 = load_variant(path_visp23);

    // Set standard passive and SFA parameters
    var_visl4.leak_shift = 4;
    var_visl4.rest_potential = -70000;
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;
    var_visl4.ahp_amplitude = 5000;
    var_visl4.refractory_period = 14;
    var_visl4.heartbeat_m = 0;
    var_visl4.gsop_potentiation = 0;
    var_visl4.gsop_depression = 0;

    var_visp5.leak_shift = 4;
    var_visp5.rest_potential = -76000;
    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;
    var_visp5.ahp_amplitude = 5000;
    var_visp5.refractory_period = 14;
    var_visp5.heartbeat_m = 0;
    var_visp5.gsop_potentiation = 0;
    var_visp5.gsop_depression = 0;

    var_visp23.leak_shift = 2;
    var_visp23.rest_potential = -66000;
    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;
    var_visp23.ahp_amplitude = 5000;
    var_visp23.refractory_period = 14;
    var_visp23.heartbeat_m = 0;
    var_visp23.gsop_potentiation = 0;
    var_visp23.gsop_depression = 0;

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // Simulation scaling list
    let scale_sizes: Vec<usize> = vec![128, 256, 512, 1024, 10000, 100000, 1000000];
    let mut summary_results = Vec::new();

    for &n in &scale_sizes {
        println!("--- Running Scale-Up Experiment for N = {} ---", n);
        let mut rng = SimpleRng::new(100 + n as u64);

        let padded_n = n.div_ceil(64) * 64;
        let total_axons = padded_n + padded_n / 2;

        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axons_buf = MvpAxonBuffer::new(total_axons);

        // Soma layout
        for i in 0..padded_n {
            let type_id = if i < n / 2 {
                0 // L4 spiny (excitatory, 50%)
            } else if i < 3 * n / 4 {
                2 // L23 aspiny (inhibitory, 25%)
            } else if i < n {
                1 // L5 spiny (excitatory, 25%)
            } else {
                0 // Padding somas
            };
            let var = &variant_table[type_id];
            state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
            state_buf.write_soma_voltage(i, var.rest_potential);
            state_buf.write_soma_to_axon(i, i as u32);
        }

        // Coordinates
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
                    0.0f32,
                )
            } else {
                (0.0f32, 0.0f32, 0.0f32) // Padding coordinates
            };
            coordinates.push((x, y, z));
        }

        // Synapse connection list
        let mut edges = Vec::new();
        let mut next_slot = vec![0usize; padded_n];

        if n <= 1024 {
            // Precise target fan-in distance-based connectivity
            // 1. L4 (0..N/2) -> L23 (N/2..3N/4): target 8-24 excitatory synapses per target
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
                    edges.push((candidates[k].0, dest, 3000i32));
                }
            }

            // 2. L4 (0..N/2) -> L5 (3N/4..N): target 6-18 excitatory synapses per target
            for dest in (3 * n / 4)..n {
                let fan_in_target = rng.range(6, 18);
                let mut candidates = Vec::new();
                for src in 0..(n / 2) {
                    let (x1, y1, z1) = coordinates[src];
                    let (x2, y2, z2) = coordinates[dest];
                    let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                    candidates.push((src, d));
                }
                candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                for k in 0..fan_in_target {
                    edges.push((candidates[k].0, dest, 3000i32));
                }
            }

            // 3. L23 (N/2..3N/4) -> L4 (0..N/2): target 8-24 inhibitory synapses per target
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
                    edges.push((candidates[k].0, dest, -3500i32));
                }
            }

            // 4. L23 (N/2..3N/4) -> L5 (3N/4..N): target 6-18 inhibitory synapses per target
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
                    edges.push((candidates[k].0, dest, -3500i32));
                }
            }

            // 5. Virtual inputs -> L4: target 8-24 synapses per target
            for dest in 0..(n / 2) {
                let fan_in_target = rng.range(8, 24);
                for _ in 0..fan_in_target {
                    let src = rng.range(n, n + n / 2 - 1);
                    edges.push((src, dest, 4500i32));
                }
            }
        } else {
            // For N >= 10,000, we populate ALL 128 dendrite slots for memory/bandwidth stress tests
            for dest in 0..padded_n {
                for _ in 0..128 {
                    let src = rng.range(0, total_axons - 1);
                    let weight = if src < n / 2 {
                        3000i32 // L4 excitatory
                    } else if src < 3 * n / 4 {
                        -3500i32 // L23 inhibitory
                    } else if src < n {
                        3000i32 // L5 excitatory
                    } else {
                        4500i32 // Virtual input excitatory
                    };
                    edges.push((src, dest, weight));
                }
            }
        }

        // Write connections to state buffer
        for &(src, dest, weight) in &edges {
            let slot = next_slot[dest];
            assert!(slot < 128, "MAX_DENDRITES limit of 128 exceeded!");
            let target = PackedTarget::pack(src as u32, 0).0;
            state_buf.write_dendrite_target(slot, dest, target);
            state_buf.write_dendrite_weight(slot, dest, weight << 16);
            next_slot[dest] += 1;
        }

        // Save connectivity specs to json (N <= 1024 only to avoid multi-GB files)
        if n <= 1024 {
            let mut neurons_json = Vec::new();
            for i in 0..padded_n {
                let (x, y, z) = coordinates[i];
                let class = if i < n / 2 {
                    "L4_spiny"
                } else if i < 3 * n / 4 {
                    "L23_aspiny"
                } else {
                    "L5_spiny"
                };
                neurons_json.push(serde_json::json!({
                    "id": i,
                    "class": class,
                    "x": x,
                    "y": y,
                    "z": z,
                    "fan_in": next_slot[i]
                }));
            }
            let mut edges_json = Vec::new();
            for &(src, dest, weight) in &edges {
                edges_json.push(serde_json::json!({
                    "src": src,
                    "dest": dest,
                    "weight": weight
                }));
            }
            let conn_json = serde_json::json!({
                "neurons": neurons_json,
                "edges": edges_json
            });
            let conn_path = artifacts_dir.join(format!(
                "static_microcircuit_scale_up_connectivity_{}.json",
                n
            ));
            let file = File::create(&conn_path).unwrap();
            serde_json::to_writer_pretty(file, &conn_json).unwrap();
        }

        // Initialize CPU simulation backend
        let init_start = Instant::now();
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
        let init_dur_ms = init_start.elapsed().as_millis();

        // 3. Run simulation ticks
        // Determine number of ticks based on N
        let num_ticks = if n <= 512 {
            9000
        } else if n == 1024 {
            1000
        } else {
            10 // 10 ticks load test for very large sizes (10k, 100k, 1M)
        };

        let max_spikes = total_axons as u32;
        let mut out_spikes = vec![0u32; max_spikes as usize];
        let mut out_counts = vec![0u32; 1];
        let mapped_somas: Vec<u32> = (0..padded_n as u32).collect();

        let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
        let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

        let mut incoming_padded = vec![0u32; max_spikes as usize];
        let mut recent_spikes = VecDeque::new();
        let mut sim_log = Vec::new();
        let mut total_sim_time_us = 0u64;

        for tick in 0..num_ticks {
            let mut incoming_count = 0;

            // Get regime specific Poisson rate
            let p_val = if n <= 512 {
                if tick < 1000 {
                    0.0
                } else if tick < 3000 {
                    0.012 // weak Poisson
                } else if tick < 5000 {
                    0.045 // moderate Poisson
                } else if tick < 7000 {
                    0.075 // structured
                } else {
                    0.0 // recovery
                }
            } else if n == 1024 {
                if tick < 250 {
                    0.0
                } else if tick < 500 {
                    0.012
                } else if tick < 750 {
                    0.045
                } else {
                    0.0
                }
            } else {
                // Large perf only runs
                if tick < 5 {
                    0.012
                } else {
                    0.045
                }
            };

            // Generate virtual input spikes
            let virt_count = total_axons - n;
            if n <= 512 && tick >= 5000 && tick < 7000 {
                // Structured alternating spatial groups
                let group_a = ((tick - 5000) / 500) % 2 == 0;
                for axon_idx in n..total_axons {
                    let is_a = axon_idx < n + virt_count / 2;
                    if is_a == group_a {
                        if rng.next_f32() < p_val {
                            incoming_padded[incoming_count] = axon_idx as u32;
                            incoming_count += 1;
                        }
                    }
                }
            } else {
                for axon_idx in n..total_axons {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }

            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick as u64,
                v_seg: 1,
                dopamine: 0,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(&incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: &mapped_somas,
                output_spikes: &mut out_spikes,
                output_spike_counts: &mut out_counts,
            };

            let tick_start = Instant::now();
            backend.run_day_batch(handle, cmd).unwrap();
            let tick_dur_us = tick_start.elapsed().as_micros() as u64;
            total_sim_time_us += tick_dur_us;

            // Only pull debug state snapshots for N <= 1024 to save memory and CPU time
            if n <= 1024 {
                backend
                    .debug_snapshot(
                        handle,
                        ShardSnapshotMut {
                            state_blob: &mut snap_state,
                            axons_blob: &mut snap_axons,
                        },
                    )
                    .unwrap();

                let snap_state_buf =
                    MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

                let count = out_counts[0] as usize;
                let mut tick_spikes = vec![false; padded_n];
                let mut l4_spikes = 0;
                let mut l23_spikes = 0;
                let mut l5_spikes = 0;

                for s_idx in 0..count {
                    let soma_id = out_spikes[s_idx] as usize;
                    if soma_id < padded_n {
                        tick_spikes[soma_id] = true;
                        if soma_id < n / 2 {
                            l4_spikes += 1;
                        } else if soma_id < 3 * n / 4 {
                            l23_spikes += 1;
                        } else {
                            l5_spikes += 1;
                        }
                    }
                }

                let mut l4_voltages = Vec::new();
                let mut l23_voltages = Vec::new();
                let mut l5_voltages = Vec::new();

                let mut l4_thresh = Vec::new();
                let mut l23_thresh = Vec::new();
                let mut l5_thresh = Vec::new();

                let mut l4_fatigues = Vec::new();
                let mut l23_fatigues = Vec::new();
                let mut l5_fatigues = Vec::new();

                let mut active_exc_weights = 0i64;
                let mut active_inh_weights = 0i64;

                for i in 0..padded_n {
                    let v = snap_state_buf.read_soma_voltage(i);
                    let th = snap_state_buf.read_threshold_offset(i);

                    let slots_count = next_slot[i];
                    let mut fatigue_sum = 0.0;
                    let cap = 15.0f64;

                    for slot in 0..slots_count {
                        let f_timer = snap_state_buf.read_dendrite_timer(slot, i);
                        fatigue_sum += f_timer as f64 / cap;

                        let target = PackedTarget(snap_state_buf.read_dendrite_target(slot, i));
                        if target.is_active() {
                            if let Some((raw_axon, _seg)) = target.unpack() {
                                let is_active = if raw_axon < n as u32 {
                                    tick_spikes[raw_axon as usize]
                                } else {
                                    incoming_padded[0..incoming_count].contains(&raw_axon)
                                };

                                if is_active {
                                    let w = snap_state_buf.read_dendrite_weight(slot, i) >> 16;
                                    if w > 0 {
                                        active_exc_weights += w as i64;
                                    } else {
                                        active_inh_weights += w.abs() as i64;
                                    }
                                }
                            }
                        }
                    }

                    let avg_fatigue = if slots_count > 0 {
                        fatigue_sum / slots_count as f64
                    } else {
                        0.0
                    };

                    if i < n / 2 {
                        l4_voltages.push(v);
                        l4_thresh.push(th);
                        l4_fatigues.push(avg_fatigue);
                    } else if i < 3 * n / 4 {
                        l23_voltages.push(v);
                        l23_thresh.push(th);
                        l23_fatigues.push(avg_fatigue);
                    } else {
                        l5_voltages.push(v);
                        l5_thresh.push(th);
                        l5_fatigues.push(avg_fatigue);
                    }
                }

                recent_spikes.push_back((l4_spikes, l23_spikes, l5_spikes));
                if recent_spikes.len() > 100 {
                    recent_spikes.pop_front();
                }

                let (total_l4, total_l23, total_l5) = recent_spikes
                    .iter()
                    .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
                let total_all = total_l4 + total_l23 + total_l5;

                let silence_flag = tick >= 1100 && total_all == 0;
                let l4_runaway = total_l4 as f64 / (n / 2) as f64 > 25.0;
                let l23_runaway = total_l23 as f64 / (n / 4) as f64 > 25.0;
                let l5_runaway = total_l5 as f64 / (n / 4) as f64 > 25.0;
                let runaway_flag = l4_runaway || l23_runaway || l5_runaway;

                sim_log.push(serde_json::json!({
                    "tick": tick,
                    "l4_spikes": l4_spikes,
                    "l23_spikes": l23_spikes,
                    "l5_spikes": l5_spikes,
                    "l4_mean_voltage": l4_voltages.iter().sum::<i32>() as f64 / (n / 2) as f64,
                    "l23_mean_voltage": l23_voltages.iter().sum::<i32>() as f64 / (n / 4) as f64,
                    "l5_mean_voltage": l5_voltages.iter().sum::<i32>() as f64 / (n / 4) as f64,
                    "l4_mean_threshold": l4_thresh.iter().sum::<i32>() as f64 / (n / 2) as f64,
                    "l23_mean_threshold": l23_thresh.iter().sum::<i32>() as f64 / (n / 4) as f64,
                    "l5_mean_threshold": l5_thresh.iter().sum::<i32>() as f64 / (n / 4) as f64,
                    "l4_mean_fatigue": l4_fatigues.iter().sum::<f64>() / (n / 2) as f64,
                    "l23_mean_fatigue": l23_fatigues.iter().sum::<f64>() / (n / 4) as f64,
                    "l5_mean_fatigue": l5_fatigues.iter().sum::<f64>() / (n / 4) as f64,
                    "active_exc_weights": active_exc_weights,
                    "active_inh_weights": active_inh_weights,
                    "silence_flag": silence_flag,
                    "runaway_flag": runaway_flag,
                    "runtime_us": tick_dur_us,
                    "spiked_neuron_ids": out_spikes[0..count].to_vec()
                }));
            }
        }

        backend.free_shard(handle).unwrap();

        // Calculate average tick duration and print summary info
        let avg_tick_us = total_sim_time_us as f64 / num_ticks as f64;
        println!(
            "N = {}: Init duration = {} ms, total simulation time = {} us, avg tick time = {:.2} us",
            n, init_dur_ms, total_sim_time_us, avg_tick_us
        );

        summary_results.push(serde_json::json!({
            "N": n,
            "init_time_ms": init_dur_ms,
            "total_sim_time_us": total_sim_time_us,
            "avg_tick_time_us": avg_tick_us,
            "num_ticks": num_ticks,
            "edges_count": edges.len()
        }));

        // Save log to file for analysis (N <= 1024 only)
        if n <= 1024 {
            let log_path =
                artifacts_dir.join(format!("static_microcircuit_scale_up_log_{}.json", n));
            let file = File::create(&log_path).unwrap();
            serde_json::to_writer_pretty(file, &sim_log).unwrap();
        }
    }

    // Save final summary result to file
    let summary_path = artifacts_dir.join("static_microcircuit_scale_up_summary.json");
    let file = File::create(&summary_path).unwrap();
    serde_json::to_writer_pretty(file, &summary_results).unwrap();
    println!("Saved final scale-up summary to {:?}", summary_path);

    println!("Static Microcircuit Scale-Up v1 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec,
    clippy::manual_range_contains,
    clippy::type_complexity
)]
fn run_static_microcircuit_v1_4_experiments() {
    println!("=== Starting Static Microcircuit v1.4 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use std::fs::File;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

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

    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // 1. Define calibrated neuron profiles (variants)
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    var_visl4.leak_shift = 4;
    var_visl4.rest_potential = -70000;
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;
    var_visl4.ahp_amplitude = 5000;
    var_visl4.refractory_period = 14;
    var_visl4.heartbeat_m = 0;
    var_visl4.gsop_potentiation = 0;
    var_visl4.gsop_depression = 0;

    let mut var_visp5 = load_variant(path_visp5);
    var_visp5.leak_shift = 4;
    var_visp5.rest_potential = -76000;
    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;
    var_visp5.ahp_amplitude = 5000;
    var_visp5.refractory_period = 14;
    var_visp5.heartbeat_m = 0;
    var_visp5.gsop_potentiation = 0;
    var_visp5.gsop_depression = 0;

    let mut var_visp23 = load_variant(path_visp23);
    var_visp23.leak_shift = 2;
    var_visp23.rest_potential = -66000;
    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;
    var_visp23.ahp_amplitude = 5000;
    var_visp23.refractory_period = 14;
    var_visp23.heartbeat_m = 0;
    var_visp23.gsop_potentiation = 0;
    var_visp23.gsop_depression = 0;

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // 2. Define sweep parameters
    let noise_profiles = vec![
        (0.006, 0.020, 0.035), // Low
        (0.009, 0.030, 0.050), // Mid
        (0.012, 0.045, 0.075), // High
    ];

    #[derive(serde::Serialize, Clone)]
    struct SweepResult {
        stage: usize,
        virtual_weight: i32,
        noise_profile: usize,
        exc_weight_l4_l23: i32,
        exc_weight_l4_l5: i32,
        fan_in_l4_l5_idx: usize,
        inh_weight_l23_l4: i32,
        inh_weight_l23_l5: i32,
        l4_rate: f64,
        l23_rate: f64,
        l5_rate: f64,
        max_consec_vm_above: usize,
        max_consec_vm_below: usize,
        max_thresh_offset_mv: f64,
        thresh_decay_pct: f64,
        selectivity: f64,
        has_runaway: bool,
        passed_all_gates: bool,
        l5_mean_fan_in: f64,
        l5_max_fan_in: usize,
    }

    let mut sweep_results = Vec::new();

    let run_config = |n: usize,
                      virt_w: i32,
                      profile_idx: usize,
                      exc_w_l4_l23: i32,
                      exc_w_l4_l5: i32,
                      fan_in_l4_l5_range_idx: usize,
                      inh_w_l23_l4: i32,
                      inh_w_l23_l5: i32,
                      stage: usize,
                      variant_table: &[VariantParameters; layout::VARIANT_LUT_LEN]|
     -> (SweepResult, Vec<serde_json::Value>) {
        let padded_n = n.div_ceil(64) * 64;
        let total_axons = padded_n + padded_n / 2;
        let virt_count = total_axons - n;
        let l5_count = n / 4;

        let mut rng = SimpleRng::new(
            42 + (virt_w as u64) * 7
                + (exc_w_l4_l23 as u64) * 13
                + (exc_w_l4_l5 as u64) * 17
                + (inh_w_l23_l4.unsigned_abs() as u64) * 19
                + (inh_w_l23_l5.unsigned_abs() as u64) * 23
                + (fan_in_l4_l5_range_idx as u64) * 31,
        );

        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axons_buf = MvpAxonBuffer::new(total_axons);

        // Soma layout
        for i in 0..padded_n {
            let type_id = if i < n / 2 {
                0 // L4 spiny
            } else if i < 3 * n / 4 {
                2 // L23 aspiny
            } else if i < n {
                1 // L5 spiny
            } else {
                0 // padding
            };
            let var = &variant_table[type_id];
            state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
            state_buf.write_soma_voltage(i, var.rest_potential);
            state_buf.write_soma_to_axon(i, i as u32);
        }

        // Coordinates
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
                    0.0f32,
                )
            } else {
                (0.0f32, 0.0f32, 0.0f32)
            };
            coordinates.push((x, y, z));
        }

        let mut edges = Vec::new();
        let mut next_slot = vec![0usize; padded_n];

        // L4 -> L23
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

        // L4 -> L5
        let fan_in_l4_l5 = match fan_in_l4_l5_range_idx {
            0 => rng.range(6, 18),
            1 => rng.range(12, 28),
            2 => rng.range(20, 40),
            _ => rng.range(6, 18),
        };
        for dest in (3 * n / 4)..n {
            let mut candidates = Vec::new();
            for src in 0..(n / 2) {
                let (x1, y1, z1) = coordinates[src];
                let (x2, y2, z2) = coordinates[dest];
                let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                candidates.push((src, d));
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let actual_fan_in = fan_in_l4_l5.min(candidates.len());
            for k in 0..actual_fan_in {
                edges.push((candidates[k].0, dest, exc_w_l4_l5));
            }
        }

        // L23 -> L4 (inhibitory)
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

        // L23 -> L5 (inhibitory)
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

        // L23 -> L23 (inhibitory feedback)
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
            let mut candidates = Vec::new();
            for src in (n / 2)..(3 * n / 4) {
                if src != dest {
                    let (x1, y1, z1) = coordinates[src];
                    let (x2, y2, z2) = coordinates[dest];
                    let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                    candidates.push((src, d));
                }
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let act_target = fan_in_target.min(candidates.len());
            for k in 0..act_target {
                edges.push((candidates[k].0, dest, inh_w_l23_l4));
            }
        }

        // L5 -> L23
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
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

        // Virtual inputs -> L4
        for dest in 0..(n / 2) {
            let fan_in_target = rng.range(8, 24);
            for _ in 0..fan_in_target {
                let src = rng.range(n, n + virt_count - 1);
                edges.push((src, dest, virt_w));
            }
        }

        // Write connections to state buffer
        for &(src, dest, weight) in &edges {
            let slot = next_slot[dest];
            if slot < 128 {
                let target = PackedTarget::pack(src as u32, 0).0;
                state_buf.write_dendrite_target(slot, dest, target);
                state_buf.write_dendrite_weight(slot, dest, weight << 16);
                next_slot[dest] += 1;
            }
        }

        let mut l4_group_a_pref = vec![false; n / 2];
        for i in 0..(n / 2) {
            let mut count_a = 0;
            let mut count_b = 0;
            for &(src, dest, _) in &edges {
                if dest == i && src >= n {
                    let virt_idx = src - n;
                    if virt_idx < virt_count / 2 {
                        count_a += 1;
                    } else {
                        count_b += 1;
                    }
                }
            }
            l4_group_a_pref[i] = count_a >= count_b;
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
                    variant_table,
                },
            )
            .unwrap();

        let num_ticks = 9000;
        let max_spikes = total_axons as u32;
        let mut out_spikes = vec![0u32; max_spikes as usize];
        let mut out_counts = vec![0u32; 1];
        let mapped_somas: Vec<u32> = (0..padded_n as u32).collect();

        let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
        let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

        let mut incoming_padded = vec![0u32; max_spikes as usize];
        let mut recent_spikes = VecDeque::new();
        let mut sim_log = Vec::new();

        let (p_weak, p_mod, p_struct) = noise_profiles[profile_idx];
        let mut spikes_active_group = 0;
        let mut spikes_inactive_group = 0;

        let mut total_incoming_spikes = 0;
        let mut max_l4_voltage: f64 = -100000.0;

        for tick in 0..num_ticks {
            let mut incoming_count = 0;

            let p_val = if tick < 1000 {
                0.0
            } else if tick < 3000 {
                p_weak
            } else if tick < 5000 {
                p_mod
            } else if tick < 7000 {
                p_struct
            } else {
                0.0
            };

            if tick >= 5000 && tick < 7000 {
                let group_a = ((tick - 5000) / 500) % 2 == 0;
                for axon_idx in n..total_axons {
                    let is_a = axon_idx < n + virt_count / 2;
                    if is_a == group_a {
                        if rng.next_f32() < p_val {
                            incoming_padded[incoming_count] = axon_idx as u32;
                            incoming_count += 1;
                        }
                    }
                }
            } else {
                for axon_idx in n..total_axons {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }

            total_incoming_spikes += incoming_count;
            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick as u64,
                v_seg: 1,
                dopamine: 0,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(&incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: &mapped_somas,
                output_spikes: &mut out_spikes,
                output_spike_counts: &mut out_counts,
            };

            backend.run_day_batch(handle, cmd).unwrap();

            backend
                .debug_snapshot(
                    handle,
                    ShardSnapshotMut {
                        state_blob: &mut snap_state,
                        axons_blob: &mut snap_axons,
                    },
                )
                .unwrap();

            let snap_state_buf =
                MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

            let count = out_counts[0] as usize;
            let mut tick_spikes = vec![false; padded_n];
            for &id in &out_spikes[0..count] {
                if id < padded_n as u32 {
                    tick_spikes[id as usize] = true;
                }
            }

            let mut l4_sp = 0;
            let mut l23_sp = 0;
            let mut l5_sp = 0;

            let mut l4_v_sum = 0.0;
            let mut l4_th_sum = 0.0;
            let mut l4_fatigue_sum = 0.0;
            let mut l23_fatigue_sum = 0.0;
            let mut l5_v_sum = 0.0;
            let mut l5_th_sum = 0.0;
            let mut l5_fatigue_sum = 0.0;

            let mut min_l4_v = 0.0f64;
            let mut max_l4_v = -100000.0f64;
            let mut min_l5_v = 0.0f64;
            let mut max_l5_v = -100000.0f64;

            for i in 0..padded_n {
                let v = snap_state_buf.read_soma_voltage(i) as f64;
                let th = snap_state_buf.read_threshold_offset(i) as f64;

                let mut fatigue_timer_sum = 0;
                for slot in 0..128 {
                    let timer = snap_state_buf.read_dendrite_timer(slot, i);
                    fatigue_timer_sum += timer as usize;
                }
                let fatigue = fatigue_timer_sum as f64 / (128.0 * 255.0);

                if i < n / 2 {
                    l4_v_sum += v;
                    l4_th_sum += th;
                    l4_fatigue_sum += fatigue;
                    max_l4_voltage = max_l4_voltage.max(v);
                    if min_l4_v == 0.0 || v < min_l4_v {
                        min_l4_v = v;
                    }
                    if v > max_l4_v {
                        max_l4_v = v;
                    }
                    if tick_spikes[i] {
                        l4_sp += 1;
                        if tick >= 5000 && tick < 7000 {
                            let group_a_active = ((tick - 5000) / 500) % 2 == 0;
                            let is_a_responder = l4_group_a_pref[i];
                            if is_a_responder == group_a_active {
                                spikes_active_group += 1;
                            } else {
                                spikes_inactive_group += 1;
                            }
                        }
                    }
                } else if i < 3 * n / 4 {
                    l23_fatigue_sum += fatigue;
                    if tick_spikes[i] {
                        l23_sp += 1;
                    }
                } else if i < n {
                    l5_v_sum += v;
                    l5_th_sum += th;
                    l5_fatigue_sum += fatigue;
                    if min_l5_v == 0.0 || v < min_l5_v {
                        min_l5_v = v;
                    }
                    if v > max_l5_v {
                        max_l5_v = v;
                    }
                    if tick_spikes[i] {
                        l5_sp += 1;
                    }
                }
            }

            let mut virt_spiked = vec![false; total_axons];
            for &axon_id in &incoming_padded[0..incoming_count] {
                if (axon_id as usize) < total_axons {
                    virt_spiked[axon_id as usize] = true;
                }
            }

            let mut l5_exc_input = 0.0;
            let mut l5_inh_input = 0.0;
            let l5_count = n / 4;

            for dest in (3 * n / 4)..n {
                for &(src, d_node, w) in &edges {
                    if d_node == dest {
                        let spiked = if src < n {
                            tick_spikes[src]
                        } else {
                            virt_spiked[src]
                        };
                        if spiked {
                            if w > 0 {
                                l5_exc_input += w as f64;
                            } else {
                                l5_inh_input += w.unsigned_abs() as f64;
                            }
                        }
                    }
                }
            }

            recent_spikes.push_back((l4_sp, l23_sp, l5_sp));
            if recent_spikes.len() > 100 {
                recent_spikes.pop_front();
            }

            let sum_recent: (usize, usize, usize) = recent_spikes
                .iter()
                .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
            let rate_recent_l4 =
                (sum_recent.0 as f64 / (recent_spikes.len() as f64 * (n / 2) as f64)) * 1000.0;
            let rate_recent_l23 =
                (sum_recent.1 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;
            let rate_recent_l5 =
                (sum_recent.2 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;

            let silence_flag = rate_recent_l4 < 0.01;
            let runaway_flag =
                rate_recent_l4 > 120.0 || rate_recent_l23 > 120.0 || rate_recent_l5 > 120.0;

            sim_log.push(serde_json::json!({
                "tick": tick,
                "l4_spikes": l4_sp,
                "l23_spikes": l23_sp,
                "l5_spikes": l5_sp,
                "l4_mean_voltage": l4_v_sum / (n/2) as f64,
                "l4_min_voltage": min_l4_v,
                "l4_max_voltage": max_l4_v,
                "l5_mean_voltage": l5_v_sum / l5_count as f64,
                "l5_min_voltage": min_l5_v,
                "l5_max_voltage": max_l5_v,
                "l4_mean_threshold": l4_th_sum / (n/2) as f64,
                "l5_mean_threshold": l5_th_sum / l5_count as f64,
                "l4_mean_fatigue": l4_fatigue_sum / (n/2) as f64,
                "l23_mean_fatigue": l23_fatigue_sum / (n/4) as f64,
                "l5_mean_fatigue": l5_fatigue_sum / l5_count as f64,
                "l5_active_exc_input": l5_exc_input / l5_count as f64,
                "l5_active_inh_input": l5_inh_input / l5_count as f64,
                "silence_flag": silence_flag,
                "runaway_flag": runaway_flag,
            }));
        }

        backend.free_shard(handle).unwrap();

        let mod_log = &sim_log[3000..5000];
        let total_mod_ticks = 2000.0;
        let r3_l4 = (mod_log
            .iter()
            .map(|item| item["l4_spikes"].as_f64().unwrap())
            .sum::<f64>()
            / (total_mod_ticks * (n / 2) as f64))
            * 1000.0;
        let r3_l23 = (mod_log
            .iter()
            .map(|item| item["l23_spikes"].as_f64().unwrap())
            .sum::<f64>()
            / (total_mod_ticks * (n / 4) as f64))
            * 1000.0;
        let r3_l5 = (mod_log
            .iter()
            .map(|item| item["l5_spikes"].as_f64().unwrap())
            .sum::<f64>()
            / (total_mod_ticks * (n / 4) as f64))
            * 1000.0;

        let mut max_consec_above = 0;
        let mut consec_above = 0;
        let mut max_consec_below = 0;
        let mut consec_below = 0;

        for item in &sim_log {
            let vm = item["l4_mean_voltage"].as_f64().unwrap() / 1000.0;
            if vm > -25.0 {
                consec_above += 1;
                max_consec_above = max_consec_above.max(consec_above);
            } else {
                consec_above = 0;
            }

            if vm < -110.0 {
                consec_below += 1;
                max_consec_below = max_consec_below.max(consec_below);
            } else {
                consec_below = 0;
            }
        }

        let gate_vm_health = max_consec_above <= 50 && max_consec_below <= 50;

        let l4_th_series: Vec<f64> = sim_log
            .iter()
            .map(|item| item["l4_mean_threshold"].as_f64().unwrap() / 1000.0)
            .collect();
        let max_th_mv = l4_th_series.iter().fold(0.0f64, |a, &b| a.max(b));
        let gate_th_max = max_th_mv < 40.0;

        let peak_th = l4_th_series[5000..7000]
            .iter()
            .fold(0.0f64, |a, &b| a.max(b));
        let rec_th_end = l4_th_series[8000..9000].iter().sum::<f64>() / 1000.0;
        let decay_pct = if peak_th > 0.0 {
            (peak_th - rec_th_end) / peak_th
        } else {
            1.0
        };
        let gate_th_decay = decay_pct >= 0.30;

        let rec_log = &sim_log[7000..9000];
        let rec_rate_all = (rec_log
            .iter()
            .map(|item| {
                item["l4_spikes"].as_f64().unwrap()
                    + item["l23_spikes"].as_f64().unwrap()
                    + item["l5_spikes"].as_f64().unwrap()
            })
            .sum::<f64>()
            / (2000.0 * n as f64))
            * 1000.0;
        let gate_rec_silent = rec_rate_all < 0.5;

        let gate_l4_act = r3_l4 >= 3.0 && r3_l4 <= 25.0;
        let gate_l23_act = r3_l23 >= 3.0 && r3_l23 <= 35.0;
        let gate_l5_act = r3_l5 >= 1.0 && r3_l5 <= 15.0;

        let mut max_consec_runaway = 0;
        let mut consec_runaway = 0;
        for item in &sim_log {
            if item["runaway_flag"].as_bool().unwrap() {
                consec_runaway += 1;
                max_consec_runaway = max_consec_runaway.max(consec_runaway);
            } else {
                consec_runaway = 0;
            }
        }
        let gate_no_runaway = max_consec_runaway <= 200;
        let gate_no_silence = r3_l4 > 0.05 && r3_l23 > 0.05 && r3_l5 > 0.05;

        let sel_ratio = spikes_active_group as f64 / spikes_inactive_group.max(1) as f64;
        let gate_selectivity = sel_ratio > 1.5;

        let passed_all = gate_vm_health
            && gate_th_max
            && gate_th_decay
            && gate_rec_silent
            && gate_l4_act
            && gate_l23_act
            && gate_l5_act
            && gate_no_runaway
            && gate_no_silence
            && gate_selectivity;

        let mut l5_fan_in = vec![0; l5_count];
        for &(_, dest, _) in &edges {
            if dest >= 3 * n / 4 && dest < n {
                l5_fan_in[dest - 3 * n / 4] += 1;
            }
        }
        let l5_mean_fan = l5_fan_in.iter().sum::<usize>() as f64 / l5_count as f64;
        let l5_max_fan = *l5_fan_in.iter().max().unwrap_or(&0);

        let res = SweepResult {
            stage,
            virtual_weight: virt_w,
            noise_profile: profile_idx,
            exc_weight_l4_l23: exc_w_l4_l23,
            exc_weight_l4_l5: exc_w_l4_l5,
            fan_in_l4_l5_idx: fan_in_l4_l5_range_idx,
            inh_weight_l23_l4: inh_w_l23_l4,
            inh_weight_l23_l5: inh_w_l23_l5,
            l4_rate: r3_l4,
            l23_rate: r3_l23,
            l5_rate: r3_l5,
            max_consec_vm_above: max_consec_above,
            max_consec_vm_below: max_consec_below,
            max_thresh_offset_mv: max_th_mv,
            thresh_decay_pct: decay_pct,
            selectivity: sel_ratio,
            has_runaway: max_consec_runaway > 200,
            passed_all_gates: passed_all,
            l5_mean_fan_in: l5_mean_fan,
            l5_max_fan_in: l5_max_fan,
        };

        println!(
            "  Debug Config: virt_w = {}, profile = {}, total_incoming = {}, max_l4_v = {:.1} mV",
            virt_w,
            profile_idx,
            total_incoming_spikes,
            max_l4_voltage / 1000.0
        );
        (res, sim_log)
    };

    // Stage 0: Audit & Baseline metrics
    println!("--- STAGE 0: Baseline Metrics (N=256 and N=512) ---");
    let (res_baseline_256, _) =
        run_config(256, 1500, 0, 3000, 3000, 0, -2750, -2750, 0, &variant_table);
    let (res_baseline_512, _) =
        run_config(512, 1500, 0, 3000, 3000, 0, -2750, -2750, 0, &variant_table);
    println!(
        "  Baseline N=256: L4={:.1}Hz, L23={:.1}Hz, L5={:.1}Hz, consec_above={}",
        res_baseline_256.l4_rate,
        res_baseline_256.l23_rate,
        res_baseline_256.l5_rate,
        res_baseline_256.max_consec_vm_above
    );
    println!(
        "  Baseline N=512: L4={:.1}Hz, L23={:.1}Hz, L5={:.1}Hz, consec_above={}",
        res_baseline_512.l4_rate,
        res_baseline_512.l23_rate,
        res_baseline_512.l5_rate,
        res_baseline_512.max_consec_vm_above
    );
    sweep_results.push(res_baseline_256);
    sweep_results.push(res_baseline_512);

    // Primary Sweep Configuration
    let inh_w_l23_l4_options = vec![-1500, -1400, -1300, -1200, -1100, -1000];
    let inh_w_l23_l5_options = vec![-1250, -1000, -750];

    // Sweep executor helper
    let run_sweep = |virt_w: i32,
                     sweep_results: &mut Vec<SweepResult>,
                     stage: usize|
     -> Option<SweepResult> {
        println!(
            "--- RUNNING SWEEP STAGE {} with virt_w = {} ---",
            stage, virt_w
        );
        let mut stage_results = Vec::new();
        for &inh_l23_l4 in &inh_w_l23_l4_options {
            for &inh_l23_l5 in &inh_w_l23_l5_options {
                // Run on N=256
                let (res_256, _) = run_config(
                    256,
                    virt_w,
                    0,
                    3000,
                    5000,
                    1,
                    inh_l23_l4,
                    inh_l23_l5,
                    stage,
                    &variant_table,
                );
                // Run on N=512
                let (res_512, _) = run_config(
                    512,
                    virt_w,
                    0,
                    3000,
                    5000,
                    1,
                    inh_l23_l4,
                    inh_l23_l5,
                    stage,
                    &variant_table,
                );
                println!(
                    "  Inh L23->L4 = {}, Inh L23->L5 = {}: N=256 passed = {}, L4 = {:.2} Hz, L5 = {:.2} Hz | N=512 passed = {}, L4 = {:.2} Hz, L5 = {:.2} Hz",
                    inh_l23_l4, inh_l23_l5, res_256.passed_all_gates, res_256.l4_rate, res_256.l5_rate, res_512.passed_all_gates, res_512.l4_rate, res_512.l5_rate
                );
                stage_results.push((res_256.clone(), res_512.clone()));
                sweep_results.push(res_256);
                sweep_results.push(res_512);
            }
        }

        // Winner Selection Policy:
        // 1. Prefer candidates with passed_all_gates == true on BOTH N=256 and N=512.
        // 2. Among passing candidates, prefer L4 not barely at threshold: target L4 >= 3.5 Hz on N=512.
        // 3. Minimize deviation from L5 target center around 8 Hz on N=512.
        let mut passing_indices = Vec::new();
        for (idx, (r256, r512)) in stage_results.iter().enumerate() {
            if r256.passed_all_gates && r512.passed_all_gates {
                passing_indices.push(idx);
            }
        }

        if !passing_indices.is_empty() {
            let pref_l4_indices: Vec<usize> = passing_indices
                .iter()
                .cloned()
                .filter(|&idx| stage_results[idx].1.l4_rate >= 3.5)
                .collect();
            let subset = if !pref_l4_indices.is_empty() {
                &pref_l4_indices
            } else {
                &passing_indices
            };

            let mut best_idx = subset[0];
            let mut best_dist = f64::MAX;
            for &idx in subset {
                let r512 = &stage_results[idx].1;
                let dist = (r512.l5_rate - 8.0).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = idx;
                }
            }
            Some(stage_results[best_idx].0.clone())
        } else {
            None
        }
    };

    let primary_winner = run_sweep(1500, &mut sweep_results, 2);
    let (winner, final_virt_w) = if let Some(winner_primary) = primary_winner {
        println!(
            "Primary sweep succeeded! Selected winner: Inh L23->L4 = {}, Inh L23->L5 = {}",
            winner_primary.inh_weight_l23_l4, winner_primary.inh_weight_l23_l5
        );
        (winner_primary, 1500)
    } else {
        println!("Primary sweep failed to find a full-pass candidate on both N sizes. Running Fallback Stage 3 (+5% input, virt_w = 1575)...");
        let fallback_5_winner = run_sweep(1575, &mut sweep_results, 3);
        if let Some(winner_fb5) = fallback_5_winner {
            println!(
                "Fallback +5% sweep succeeded! Selected winner: Inh L23->L4 = {}, Inh L23->L5 = {}",
                winner_fb5.inh_weight_l23_l4, winner_fb5.inh_weight_l23_l5
            );
            (winner_fb5, 1575)
        } else {
            println!("Fallback +5% sweep failed. Running Fallback Stage 3 (+10% input, virt_w = 1650)...");
            let fallback_10_winner = run_sweep(1650, &mut sweep_results, 3);
            if let Some(winner_fb10) = fallback_10_winner {
                println!("Fallback +10% sweep succeeded! Selected winner: Inh L23->L4 = {}, Inh L23->L5 = {}", winner_fb10.inh_weight_l23_l4, winner_fb10.inh_weight_l23_l5);
                (winner_fb10, 1650)
            } else {
                println!("All sweeps failed to find a candidate passing all gates on both N sizes. Selecting best partial candidate...");
                let stage2_results: Vec<SweepResult> = sweep_results
                    .iter()
                    .filter(|r| r.stage == 2)
                    .cloned()
                    .collect();

                let mut best_pair_idx = 0;
                let mut best_pair_dist = f64::MAX;
                for i in 0..18 {
                    let r256 = &stage2_results[2 * i];
                    let r512 = &stage2_results[2 * i + 1];
                    let dist = (r512.l4_rate - 3.5).abs()
                        + (r512.l5_rate - 8.0).abs()
                        + (r256.l4_rate - 3.5).abs();
                    if dist < best_pair_dist {
                        best_pair_dist = dist;
                        best_pair_idx = i;
                    }
                }
                (stage2_results[2 * best_pair_idx].clone(), 1500)
            }
        }
    };

    let best_inh_w_l23_l4 = winner.inh_weight_l23_l4;
    let best_inh_w_l23_l5 = winner.inh_weight_l23_l5;

    // Stage 4: Confirmation detailed best candidate runs
    println!("=== Sweeps complete. Running detailed best candidate runs ===");
    let (_best_res_256, best_log_256) = run_config(
        256,
        final_virt_w,
        0,
        3000,
        5000,
        1,
        best_inh_w_l23_l4,
        best_inh_w_l23_l5,
        4,
        &variant_table,
    );
    let (best_res_512, best_log_512) = run_config(
        512,
        final_virt_w,
        0,
        3000,
        5000,
        1,
        best_inh_w_l23_l4,
        best_inh_w_l23_l5,
        4,
        &variant_table,
    );

    let log_path_256 = artifacts_dir.join("static_microcircuit_v1_4_best_candidate_log_256.json");
    let file = File::create(&log_path_256).unwrap();
    serde_json::to_writer_pretty(file, &best_log_256).unwrap();

    let log_path_512 = artifacts_dir.join("static_microcircuit_v1_4_best_candidate_log_512.json");
    let file = File::create(&log_path_512).unwrap();
    serde_json::to_writer_pretty(file, &best_log_512).unwrap();

    println!("--- Running Ablation cases on N=512 ---");
    let (res_no_inh, log_no_inh) =
        run_config(512, final_virt_w, 0, 3000, 5000, 1, 0, 0, 5, &variant_table);
    let (res_red_inh, log_red_inh) = run_config(
        512,
        final_virt_w,
        0,
        3000,
        5000,
        1,
        best_inh_w_l23_l4 / 2,
        best_inh_w_l23_l5 / 2,
        5,
        &variant_table,
    );

    let ablation_summary = serde_json::json!({
        "full": {
            "l4_rate": best_res_512.l4_rate,
            "l23_rate": best_res_512.l23_rate,
            "l5_rate": best_res_512.l5_rate,
            "max_consec_vm_above": best_res_512.max_consec_vm_above,
            "has_runaway": best_res_512.has_runaway,
        },
        "no_inhibition": {
            "l4_rate": res_no_inh.l4_rate,
            "l23_rate": res_no_inh.l23_rate,
            "l5_rate": res_no_inh.l5_rate,
            "max_consec_vm_above": res_no_inh.max_consec_vm_above,
            "has_runaway": res_no_inh.has_runaway,
        },
        "reduced_inhibition": {
            "l4_rate": res_red_inh.l4_rate,
            "l23_rate": res_red_inh.l23_rate,
            "l5_rate": res_red_inh.l5_rate,
            "max_consec_vm_above": res_red_inh.max_consec_vm_above,
            "has_runaway": res_red_inh.has_runaway,
        }
    });

    let ablation_path = artifacts_dir.join("static_microcircuit_v1_4_ablation_summary.json");
    let file = File::create(&ablation_path).unwrap();
    serde_json::to_writer_pretty(file, &ablation_summary).unwrap();

    let ablation_logs = serde_json::json!({
        "no_inhibition_log": log_no_inh,
        "reduced_inhibition_log": log_red_inh
    });
    let ablation_logs_path = artifacts_dir.join("static_microcircuit_v1_4_ablation_logs.json");
    let file = File::create(&ablation_logs_path).unwrap();
    serde_json::to_writer_pretty(file, &ablation_logs).unwrap();

    let sweep_summary_path = artifacts_dir.join("static_microcircuit_v1_4_sweep_summary.json");
    let file = File::create(&sweep_summary_path).unwrap();
    serde_json::to_writer_pretty(file, &sweep_results).unwrap();

    println!("Static Microcircuit v1.4 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec,
    clippy::manual_range_contains,
    clippy::type_complexity
)]
fn run_plastic_microcircuit_v1_0_experiments() {
    println!("=== Starting Plastic Microcircuit v1.0 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use std::fs::File;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

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

    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // 1. Define calibrated neuron profiles (variants) with GSOP/STDP enabled
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    var_visl4.leak_shift = 4;
    var_visl4.rest_potential = -70000;
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;
    var_visl4.ahp_amplitude = 5000;
    var_visl4.refractory_period = 14;
    var_visl4.heartbeat_m = 0;
    // gsop_potentiation, gsop_depression, and fatigue_capacity are loaded from TOML default

    let mut var_visp5 = load_variant(path_visp5);
    var_visp5.leak_shift = 4;
    var_visp5.rest_potential = -76000;
    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;
    var_visp5.ahp_amplitude = 5000;
    var_visp5.refractory_period = 14;
    var_visp5.heartbeat_m = 0;

    let mut var_visp23 = load_variant(path_visp23);
    var_visp23.leak_shift = 2;
    var_visp23.rest_potential = -66000;
    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;
    var_visp23.ahp_amplitude = 5000;
    var_visp23.refractory_period = 14;
    var_visp23.heartbeat_m = 0;

    println!("GSOP Audit:");
    println!(
        "  L4: gsop_pot = {}, gsop_dep = {}, fatigue_cap = {}",
        var_visl4.gsop_potentiation, var_visl4.gsop_depression, var_visl4.fatigue_capacity
    );
    println!(
        "  L5: gsop_pot = {}, gsop_dep = {}, fatigue_cap = {}",
        var_visp5.gsop_potentiation, var_visp5.gsop_depression, var_visp5.fatigue_capacity
    );
    println!(
        "  L23: gsop_pot = {}, gsop_dep = {}, fatigue_cap = {}",
        var_visp23.gsop_potentiation, var_visp23.gsop_depression, var_visp23.fatigue_capacity
    );

    assert!(var_visl4.gsop_potentiation > 0);
    assert!(var_visl4.gsop_depression > 0);
    assert!(var_visl4.fatigue_capacity > 0);

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    let noise_profiles = vec![
        (0.006, 0.020, 0.035), // Low
        (0.009, 0.030, 0.050), // Mid
        (0.012, 0.045, 0.075), // High
    ];

    let run_simulation = |n: usize,
                          num_ticks: usize,
                          is_learning: bool,
                          variant_table: &[VariantParameters; layout::VARIANT_LUT_LEN]|
     -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
        let virt_w = 1500;
        let exc_w_l4_l23 = 3000;
        let exc_w_l4_l5 = 5000;
        let fan_in_l4_l5_range_idx = 1;
        let inh_w_l23_l4 = -1200;
        let inh_w_l23_l5 = -1250;

        let padded_n = n.div_ceil(64) * 64;
        let total_axons = padded_n + padded_n / 2;
        let virt_count = total_axons - n;
        let l5_count = n / 4;

        let mut rng = SimpleRng::new(42 + (n as u64) * 11 + (num_ticks as u64) * 13);

        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axons_buf = MvpAxonBuffer::new(total_axons);

        // Soma layout
        for i in 0..padded_n {
            let type_id = if i < n / 2 {
                0 // L4 spiny
            } else if i < 3 * n / 4 {
                2 // L23 aspiny
            } else if i < n {
                1 // L5 spiny
            } else {
                0 // padding
            };
            let var = &variant_table[type_id];
            state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
            state_buf.write_soma_voltage(i, var.rest_potential);
            state_buf.write_soma_to_axon(i, i as u32);
        }

        // Coordinates
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
                    0.0f32,
                )
            } else {
                (0.0f32, 0.0f32, 0.0f32)
            };
            coordinates.push((x, y, z));
        }

        let mut edges = Vec::new();
        let mut next_slot = vec![0usize; padded_n];

        // L4 -> L23
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

        // L4 -> L5
        let fan_in_l4_l5 = match fan_in_l4_l5_range_idx {
            0 => rng.range(6, 18),
            1 => rng.range(12, 28),
            2 => rng.range(20, 40),
            _ => rng.range(6, 18),
        };
        for dest in (3 * n / 4)..n {
            let mut candidates = Vec::new();
            for src in 0..(n / 2) {
                let (x1, y1, z1) = coordinates[src];
                let (x2, y2, z2) = coordinates[dest];
                let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                candidates.push((src, d));
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let actual_fan_in = fan_in_l4_l5.min(candidates.len());
            for k in 0..actual_fan_in {
                edges.push((candidates[k].0, dest, exc_w_l4_l5));
            }
        }

        // L23 -> L4 (inhibitory)
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

        // L23 -> L5 (inhibitory)
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

        // L23 -> L23 (inhibitory feedback)
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
            let mut candidates = Vec::new();
            for src in (n / 2)..(3 * n / 4) {
                if src != dest {
                    let (x1, y1, z1) = coordinates[src];
                    let (x2, y2, z2) = coordinates[dest];
                    let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                    candidates.push((src, d));
                }
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let act_target = fan_in_target.min(candidates.len());
            for k in 0..act_target {
                edges.push((candidates[k].0, dest, inh_w_l23_l4));
            }
        }

        // L5 -> L23
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
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

        // Virtual inputs -> L4
        for dest in 0..(n / 2) {
            let fan_in_target = rng.range(8, 24);
            for _ in 0..fan_in_target {
                let src = rng.range(n, n + virt_count - 1);
                edges.push((src, dest, virt_w));
            }
        }

        // Write connections to state buffer
        for &(src, dest, weight) in &edges {
            let slot = next_slot[dest];
            if slot < 128 {
                let target = PackedTarget::pack(src as u32, 0).0;
                state_buf.write_dendrite_target(slot, dest, target);
                state_buf.write_dendrite_weight(slot, dest, weight << 16);
                next_slot[dest] += 1;
            }
        }

        let mut l4_group_a_pref = vec![false; n / 2];
        for i in 0..(n / 2) {
            let mut count_a = 0;
            let mut count_b = 0;
            for &(src, dest, _) in &edges {
                if dest == i && src >= n {
                    let virt_idx = src - n;
                    if virt_idx < virt_count / 2 {
                        count_a += 1;
                    } else {
                        count_b += 1;
                    }
                }
            }
            l4_group_a_pref[i] = count_a >= count_b;
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
                    variant_table,
                },
            )
            .unwrap();

        let max_spikes = total_axons as u32;
        let mut out_spikes = vec![0u32; max_spikes as usize];
        let mut out_counts = vec![0u32; 1];
        let mapped_somas: Vec<u32> = (0..padded_n as u32).collect();

        let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
        let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

        let mut incoming_padded = vec![0u32; max_spikes as usize];
        let mut recent_spikes = VecDeque::new();
        let mut sim_log = Vec::new();

        let (p_weak, p_mod, p_struct) = noise_profiles[0]; // Low noise profile

        for tick in 0..num_ticks {
            let mut incoming_count = 0;

            // Define timing parameters depending on run length (sanity = 9,000, learning = 50,000)
            let p_val = if !is_learning {
                // 9,000 tick schedule
                if tick < 1000 {
                    0.0
                } else if tick < 3000 {
                    p_weak
                } else if tick < 5000 {
                    p_mod
                } else if tick < 7000 {
                    p_struct
                } else {
                    0.0
                }
            } else {
                // 50,000 tick schedule
                if tick < 5000 {
                    0.0
                } else if tick < 15000 {
                    p_weak
                } else if tick < 25000 {
                    p_mod
                } else if tick < 45000 {
                    p_struct
                } else {
                    0.0
                }
            };

            // Structured alternating input blocks
            let is_structured = if !is_learning {
                tick >= 5000 && tick < 7000
            } else {
                tick >= 25000 && tick < 45000
            };

            if is_structured {
                let block_start = if !is_learning { 5000 } else { 25000 };
                let group_a = ((tick - block_start) / 500) % 2 == 0;
                for axon_idx in n..total_axons {
                    let is_a = axon_idx < n + virt_count / 2;
                    if is_a == group_a {
                        if rng.next_f32() < p_val {
                            incoming_padded[incoming_count] = axon_idx as u32;
                            incoming_count += 1;
                        }
                    }
                }
            } else {
                for axon_idx in n..total_axons {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }

            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick as u64,
                v_seg: 1,
                dopamine: 0,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(&incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: &mapped_somas,
                output_spikes: &mut out_spikes,
                output_spike_counts: &mut out_counts,
            };

            backend.run_day_batch(handle, cmd).unwrap();

            backend
                .debug_snapshot(
                    handle,
                    ShardSnapshotMut {
                        state_blob: &mut snap_state,
                        axons_blob: &mut snap_axons,
                    },
                )
                .unwrap();

            let snap_state_buf_tick =
                MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

            let count = out_counts[0] as usize;
            let mut tick_spikes = vec![false; padded_n];
            for &id in &out_spikes[0..count] {
                if id < padded_n as u32 {
                    tick_spikes[id as usize] = true;
                }
            }

            let mut l4_sp = 0;
            let mut l23_sp = 0;
            let mut l5_sp = 0;

            let mut l4_v_sum = 0.0;
            let mut l4_th_sum = 0.0;
            let mut l4_fatigue_sum = 0.0;
            let mut l5_v_sum = 0.0;
            let mut l5_th_sum = 0.0;
            let mut l5_fatigue_sum = 0.0;

            for i in 0..padded_n {
                let v = snap_state_buf_tick.read_soma_voltage(i) as f64;
                let th = snap_state_buf_tick.read_threshold_offset(i) as f64;

                let mut fatigue_timer_sum = 0;
                for slot in 0..128 {
                    let timer = snap_state_buf_tick.read_dendrite_timer(slot, i);
                    fatigue_timer_sum += timer as usize;
                }
                let fatigue = fatigue_timer_sum as f64 / (128.0 * 255.0);

                if i < n / 2 {
                    l4_v_sum += v;
                    l4_th_sum += th;
                    l4_fatigue_sum += fatigue;
                    if tick_spikes[i] {
                        l4_sp += 1;
                    }
                } else if i < 3 * n / 4 {
                    if tick_spikes[i] {
                        l23_sp += 1;
                    }
                } else if i < n {
                    l5_v_sum += v;
                    l5_th_sum += th;
                    l5_fatigue_sum += fatigue;
                    if tick_spikes[i] {
                        l5_sp += 1;
                    }
                }
            }

            recent_spikes.push_back((l4_sp, l23_sp, l5_sp));
            if recent_spikes.len() > 100 {
                recent_spikes.pop_front();
            }

            let sum_recent: (usize, usize, usize) = recent_spikes
                .iter()
                .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
            let rate_recent_l4 =
                (sum_recent.0 as f64 / (recent_spikes.len() as f64 * (n / 2) as f64)) * 1000.0;
            let rate_recent_l23 =
                (sum_recent.1 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;
            let rate_recent_l5 =
                (sum_recent.2 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;

            let silence_flag = rate_recent_l4 < 0.01;
            let runaway_flag =
                rate_recent_l4 > 120.0 || rate_recent_l23 > 120.0 || rate_recent_l5 > 120.0;

            // Log every 10 ticks for the learning run to keep files compact, every tick for sanity runs
            let should_log = !is_learning || (tick % 10 == 0);
            if should_log {
                sim_log.push(serde_json::json!({
                    "tick": tick,
                    "l4_spikes": l4_sp,
                    "l23_spikes": l23_sp,
                    "l5_spikes": l5_sp,
                    "l4_mean_voltage": l4_v_sum / (n/2) as f64,
                    "l5_mean_voltage": l5_v_sum / l5_count as f64,
                    "l4_mean_threshold": l4_th_sum / (n/2) as f64,
                    "l5_mean_threshold": l5_th_sum / l5_count as f64,
                    "l4_mean_fatigue": l4_fatigue_sum / (n/2) as f64,
                    "l5_mean_fatigue": l5_fatigue_sum / l5_count as f64,
                    "silence_flag": silence_flag,
                    "runaway_flag": runaway_flag,
                }));
            }
        }

        // Trace and extract weight changes
        let mut edge_log = Vec::new();
        let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());
        for &(src, dest, initial_w) in &edges {
            let mut final_w = initial_w;
            for slot in 0..128 {
                let target = snap_state_buf.read_dendrite_target(slot, dest);
                if let Some((src_id, _)) = types::PackedTarget(target).unpack() {
                    if src_id == src as u32 {
                        final_w = snap_state_buf.read_dendrite_weight(slot, dest) >> 16;
                        break;
                    }
                }
            }
            let delta = final_w - initial_w;

            let src_layer = if src < n / 2 {
                "L4"
            } else if src < 3 * n / 4 {
                "L23"
            } else if src < n {
                "L5"
            } else {
                "Virtual"
            };

            let dest_layer = if dest < n / 2 {
                "L4"
            } else if dest < 3 * n / 4 {
                "L23"
            } else {
                "L5"
            };

            let is_inhibitory = src >= n / 2 && src < 3 * n / 4;

            let src_coords = if src < n {
                let c = coordinates[src];
                Some((c.0, c.1, c.2))
            } else {
                None
            };
            let dest_coords = coordinates[dest];

            let is_correlated = if src >= n && dest < n / 2 {
                let virt_idx = src - n;
                let group_a_axon = virt_idx < virt_count / 2;
                let l4_prefers_a = l4_group_a_pref[dest];
                group_a_axon == l4_prefers_a
            } else {
                false
            };

            edge_log.push(serde_json::json!({
                "src": src,
                "dest": dest,
                "src_layer": src_layer,
                "dest_layer": dest_layer,
                "initial_weight": initial_w,
                "final_weight": final_w,
                "delta": delta,
                "is_inhibitory": is_inhibitory,
                "src_coords": src_coords,
                "dest_coords": (dest_coords.0, dest_coords.1, dest_coords.2),
                "is_correlated": is_correlated,
            }));
        }

        backend.free_shard(handle).unwrap();
        (sim_log, edge_log)
    };

    // Phase 1: N=256 Sanity Run (9,000 ticks)
    println!("--- Phase 1: Running N=256 Sanity Run (9,000 ticks) ---");
    let (sim_log_256_sanity, edge_log_256_sanity) =
        run_simulation(256, 9000, false, &variant_table);

    let file_path = artifacts_dir.join("plastic_microcircuit_v1_0_log_256_sanity.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_256_sanity).unwrap();

    // Check invariants on Phase 1
    let check_invariants = |edges: &[serde_json::Value]| -> (usize, usize, f64) {
        let mut dale_violations = 0;
        let mut sign_flips = 0;
        let mut total_deltas = 0;
        let mut sum_abs_delta = 0.0;

        for edge in edges {
            let init_w = edge["initial_weight"].as_i64().unwrap();
            let final_w = edge["final_weight"].as_i64().unwrap();
            let is_inh = edge["is_inhibitory"].as_bool().unwrap();
            let delta = (final_w - init_w).abs();

            total_deltas += 1;
            sum_abs_delta += delta as f64;

            if is_inh {
                if final_w > 0 {
                    dale_violations += 1;
                }
                if init_w < 0 && final_w > 0 {
                    sign_flips += 1;
                }
            } else {
                if final_w < 0 {
                    dale_violations += 1;
                }
                if init_w > 0 && final_w < 0 {
                    sign_flips += 1;
                }
            }
        }
        (
            dale_violations,
            sign_flips,
            sum_abs_delta / total_deltas as f64,
        )
    };

    let (dale_val, sign_val, mean_delta) = check_invariants(&edge_log_256_sanity);
    println!("Phase 1 Invariants:");
    println!("  Dale Violations: {}", dale_val);
    println!("  Sign Flips: {}", sign_val);
    println!("  Mean Abs Delta: {:.4} uV", mean_delta);

    // Phase 2: N=256 Structured Learning Run (50,000 ticks)
    println!("--- Phase 2: Running N=256 Structured Learning Run (50,000 ticks) ---");
    let (sim_log_256_learning, edge_log_256_learning) =
        run_simulation(256, 50000, true, &variant_table);

    let file_path = artifacts_dir.join("plastic_microcircuit_v1_0_log_256_learning.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_256_learning).unwrap();

    let file_path_edges = artifacts_dir.join("plastic_microcircuit_v1_0_edge_log_256.json");
    let file_edges = File::create(&file_path_edges).unwrap();
    serde_json::to_writer_pretty(file_edges, &edge_log_256_learning).unwrap();

    let (dale_val_lr, sign_val_lr, mean_delta_lr) = check_invariants(&edge_log_256_learning);
    println!("Phase 2 Invariants:");
    println!("  Dale Violations: {}", dale_val_lr);
    println!("  Sign Flips: {}", sign_val_lr);
    println!("  Mean Abs Delta: {:.4} uV", mean_delta_lr);

    // Phase 3: N=512 Sanity Run (9,000 ticks)
    println!("--- Phase 3: Running N=512 Sanity Run (9,000 ticks) ---");
    let (sim_log_512_sanity, edge_log_512_sanity) =
        run_simulation(512, 9000, false, &variant_table);

    let file_path = artifacts_dir.join("plastic_microcircuit_v1_0_log_512_sanity.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_512_sanity).unwrap();

    let (dale_val_512, sign_val_512, mean_delta_512) = check_invariants(&edge_log_512_sanity);
    println!("Phase 3 Invariants:");
    println!("  Dale Violations: {}", dale_val_512);
    println!("  Sign Flips: {}", sign_val_512);
    println!("  Mean Abs Delta: {:.4} uV", mean_delta_512);

    // Write research summary to summary JSON
    let summary = serde_json::json!({
        "sanity_256": {
            "mean_abs_delta": mean_delta,
            "dale_violations": dale_val,
            "sign_flips": sign_val,
        },
        "learning_256": {
            "mean_abs_delta": mean_delta_lr,
            "dale_violations": dale_val_lr,
            "sign_flips": sign_val_lr,
        },
        "sanity_512": {
            "mean_abs_delta": mean_delta_512,
            "dale_violations": dale_val_512,
            "sign_flips": sign_val_512,
        }
    });

    let summary_path = artifacts_dir.join("plastic_microcircuit_v1_0_summary.json");
    let file = File::create(&summary_path).unwrap();
    serde_json::to_writer_pretty(file, &summary).unwrap();

    println!("Plastic Microcircuit v1.0 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec,
    clippy::manual_range_contains,
    clippy::type_complexity,
    clippy::manual_is_multiple_of
)]
fn run_plastic_microcircuit_v1_1_experiments() {
    println!("=== Starting Plastic Microcircuit v1.1 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use std::fs::File;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

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

    let artifacts_dir = get_artifacts_dir();
    fs::create_dir_all(&artifacts_dir).unwrap();

    // 1. Define calibrated neuron profiles (variants) with GSOP/STDP enabled
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let mut var_visl4 = load_variant(path_visl4);
    var_visl4.leak_shift = 4;
    var_visl4.rest_potential = -70000;
    var_visl4.homeostasis_penalty = 1940;
    var_visl4.homeostasis_decay = 4;
    var_visl4.ahp_amplitude = 5000;
    var_visl4.refractory_period = 14;
    var_visl4.heartbeat_m = 0;

    let mut var_visp5 = load_variant(path_visp5);
    var_visp5.leak_shift = 4;
    var_visp5.rest_potential = -76000;
    var_visp5.homeostasis_penalty = 1940;
    var_visp5.homeostasis_decay = 9;
    var_visp5.ahp_amplitude = 5000;
    var_visp5.refractory_period = 14;
    var_visp5.heartbeat_m = 0;

    let mut var_visp23 = load_variant(path_visp23);
    var_visp23.leak_shift = 2;
    var_visp23.rest_potential = -66000;
    var_visp23.homeostasis_penalty = 500;
    var_visp23.homeostasis_decay = 4;
    var_visp23.ahp_amplitude = 5000;
    var_visp23.refractory_period = 14;
    var_visp23.heartbeat_m = 0;

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // Simulation wrapper
    let run_simulation = |n: usize,
                          learning_ticks: usize,
                          structured_p: f32,
                          background_p: f32,
                          block_size: usize|
     -> (
        Vec<serde_json::Value>, // sim_log
        Vec<serde_json::Value>, // edge_log
        f64,                    // r4
        f64,                    // r23
        f64,                    // r5
        bool,                   // silence_any
        bool,                   // runaway_any
    ) {
        let virt_w = 1500;
        let exc_w_l4_l23 = 3000;
        let exc_w_l4_l5 = 5000;
        let fan_in_l4_l5_range_idx = 1;
        let inh_w_l23_l4 = -1200;
        let inh_w_l23_l5 = -1250;

        let padded_n = n.div_ceil(64) * 64;
        let total_axons = padded_n + padded_n / 2;
        let virt_count = total_axons - n;
        let l5_count = n / 4;

        let mut rng = SimpleRng::new(100 + (n as u64) * 31 + (learning_ticks as u64) * 7);

        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axons_buf = MvpAxonBuffer::new(total_axons);

        // Soma layout
        for i in 0..padded_n {
            let type_id = if i < n / 2 {
                0
            } else if i < 3 * n / 4 {
                2
            } else if i < n {
                1
            } else {
                0
            };
            let var = &variant_table[type_id];
            state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
            state_buf.write_soma_voltage(i, var.rest_potential);
            state_buf.write_soma_to_axon(i, i as u32);
        }

        // Coordinates
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
                    0.0f32,
                )
            } else {
                (0.0f32, 0.0f32, 0.0f32)
            };
            coordinates.push((x, y, z));
        }

        let mut edges = Vec::new();
        let mut next_slot = vec![0usize; padded_n];

        // L4 -> L23
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

        // L4 -> L5
        let fan_in_l4_l5 = match fan_in_l4_l5_range_idx {
            0 => rng.range(6, 18),
            1 => rng.range(12, 28),
            2 => rng.range(20, 40),
            _ => rng.range(6, 18),
        };
        for dest in (3 * n / 4)..n {
            let mut candidates = Vec::new();
            for src in 0..(n / 2) {
                let (x1, y1, z1) = coordinates[src];
                let (x2, y2, z2) = coordinates[dest];
                let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                candidates.push((src, d));
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let actual_fan_in = fan_in_l4_l5.min(candidates.len());
            for k in 0..actual_fan_in {
                edges.push((candidates[k].0, dest, exc_w_l4_l5));
            }
        }

        // L23 -> L4 (inhibitory)
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

        // L23 -> L5 (inhibitory)
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

        // L23 -> L23 (inhibitory feedback)
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
            let mut candidates = Vec::new();
            for src in (n / 2)..(3 * n / 4) {
                if src != dest {
                    let (x1, y1, z1) = coordinates[src];
                    let (x2, y2, z2) = coordinates[dest];
                    let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                    candidates.push((src, d));
                }
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let act_target = fan_in_target.min(candidates.len());
            for k in 0..act_target {
                edges.push((candidates[k].0, dest, inh_w_l23_l4));
            }
        }

        // L5 -> L23
        for dest in (n / 2)..(3 * n / 4) {
            let fan_in_target = rng.range(4, 12);
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

        // Virtual inputs -> L4
        for dest in 0..(n / 2) {
            let fan_in_target = rng.range(8, 24);
            for _ in 0..fan_in_target {
                let src = rng.range(n, n + virt_count - 1);
                edges.push((src, dest, virt_w));
            }
        }

        // Write connections to state buffer
        for &(src, dest, weight) in &edges {
            let slot = next_slot[dest];
            if slot < 128 {
                let target = PackedTarget::pack(src as u32, 0).0;
                state_buf.write_dendrite_target(slot, dest, target);
                state_buf.write_dendrite_weight(slot, dest, weight << 16);
                next_slot[dest] += 1;
            }
        }

        // Semantic preference calculation
        let mut l4_group_a_pref = vec![false; n / 2];
        for i in 0..(n / 2) {
            let mut count_a = 0;
            let mut count_b = 0;
            for &(src, dest, _) in &edges {
                if dest == i && src >= n {
                    let virt_idx = src - n;
                    if virt_idx < virt_count / 2 {
                        count_a += 1;
                    } else {
                        count_b += 1;
                    }
                }
            }
            l4_group_a_pref[i] = count_a >= count_b;
        }

        let mut l23_group_a_pref = vec![false; n / 4];
        for i in 0..(n / 4) {
            let dest_node = n / 2 + i;
            let mut count_a = 0;
            let mut count_b = 0;
            for &(src, dest, _) in &edges {
                if dest == dest_node && src < n / 2 {
                    if l4_group_a_pref[src] {
                        count_a += 1;
                    } else {
                        count_b += 1;
                    }
                }
            }
            l23_group_a_pref[i] = count_a >= count_b;
        }

        let mut l5_group_a_pref = vec![false; n / 4];
        for i in 0..(n / 4) {
            let dest_node = 3 * n / 4 + i;
            let mut count_a = 0;
            let mut count_b = 0;
            for &(src, dest, _) in &edges {
                if dest == dest_node && src < n / 2 {
                    if l4_group_a_pref[src] {
                        count_a += 1;
                    } else {
                        count_b += 1;
                    }
                }
            }
            l5_group_a_pref[i] = count_a >= count_b;
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

        let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
        let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

        let mut incoming_padded = vec![0u32; max_spikes as usize];
        let mut recent_spikes = VecDeque::new();
        let mut sim_log = Vec::new();

        // Stimulus schedule variables
        let total_ticks = 5000 + 10000 + 10000 + learning_ticks + 15000;
        let mut total_l4_spikes = 0;
        let mut total_l23_spikes = 0;
        let mut total_l5_spikes = 0;
        let mut silence_flag_any = false;
        let mut runaway_flag_any = false;

        let p_weak = 0.006;
        let p_mod = 0.020;

        for tick in 0..total_ticks {
            let mut incoming_count = 0;

            let p_val = if tick < 5000 {
                0.0
            } else if tick < 15000 {
                p_weak
            } else if tick < 25000 {
                p_mod
            } else if tick < 25000 + learning_ticks {
                structured_p
            } else {
                0.0
            };

            let is_structured = tick >= 25000 && tick < 25000 + learning_ticks;

            if is_structured {
                let block_start = 25000;
                let group_a = ((tick - block_start) / block_size) % 2 == 0;
                for axon_idx in n..total_axons {
                    let is_a = axon_idx < n + virt_count / 2;
                    let prob = if is_a == group_a { p_val } else { background_p };
                    if rng.next_f32() < prob {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            } else {
                for axon_idx in n..total_axons {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }

            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick as u64,
                v_seg: 1,
                dopamine: 0,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(&incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: &mapped_somas,
                output_spikes: &mut out_spikes,
                output_spike_counts: &mut out_counts,
            };

            backend.run_day_batch(handle, cmd).unwrap();

            backend
                .debug_snapshot(
                    handle,
                    ShardSnapshotMut {
                        state_blob: &mut snap_state,
                        axons_blob: &mut snap_axons,
                    },
                )
                .unwrap();

            let snap_state_buf_tick =
                MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

            let count = out_counts[0] as usize;
            let mut tick_spikes = vec![false; padded_n];
            for &id in &out_spikes[0..count] {
                if id < padded_n as u32 {
                    tick_spikes[id as usize] = true;
                }
            }

            let mut l4_sp = 0;
            let mut l23_sp = 0;
            let mut l5_sp = 0;

            let mut l4_v_sum = 0.0;
            let mut l4_th_sum = 0.0;
            let mut l4_fatigue_sum = 0.0;
            let mut l5_v_sum = 0.0;
            let mut l5_th_sum = 0.0;
            let mut l5_fatigue_sum = 0.0;

            for i in 0..padded_n {
                let v = snap_state_buf_tick.read_soma_voltage(i) as f64;
                let th = snap_state_buf_tick.read_threshold_offset(i) as f64;

                let mut fatigue_timer_sum = 0;
                for slot in 0..128 {
                    let timer = snap_state_buf_tick.read_dendrite_timer(slot, i);
                    fatigue_timer_sum += timer as usize;
                }
                let fatigue = fatigue_timer_sum as f64 / (128.0 * 255.0);

                if i < n / 2 {
                    l4_v_sum += v;
                    l4_th_sum += th;
                    l4_fatigue_sum += fatigue;
                    if tick_spikes[i] {
                        l4_sp += 1;
                    }
                } else if i < 3 * n / 4 {
                    if tick_spikes[i] {
                        l23_sp += 1;
                    }
                } else if i < n {
                    l5_v_sum += v;
                    l5_th_sum += th;
                    l5_fatigue_sum += fatigue;
                    if tick_spikes[i] {
                        l5_sp += 1;
                    }
                }
            }

            total_l4_spikes += l4_sp;
            total_l23_spikes += l23_sp;
            total_l5_spikes += l5_sp;

            recent_spikes.push_back((l4_sp, l23_sp, l5_sp));
            if recent_spikes.len() > 100 {
                recent_spikes.pop_front();
            }

            let sum_recent: (usize, usize, usize) = recent_spikes
                .iter()
                .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
            let rate_recent_l4 =
                (sum_recent.0 as f64 / (recent_spikes.len() as f64 * (n / 2) as f64)) * 1000.0;
            let rate_recent_l23 =
                (sum_recent.1 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;
            let rate_recent_l5 =
                (sum_recent.2 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;

            let silence_flag = rate_recent_l4 < 0.01;
            let runaway_flag =
                rate_recent_l4 > 120.0 || rate_recent_l23 > 120.0 || rate_recent_l5 > 120.0;

            if silence_flag {
                silence_flag_any = true;
            }
            if runaway_flag {
                runaway_flag_any = true;
            }

            // Save log every 10 ticks for structured learning run
            if tick % 10 == 0 {
                sim_log.push(serde_json::json!({
                    "tick": tick,
                    "l4_spikes": l4_sp,
                    "l23_spikes": l23_sp,
                    "l5_spikes": l5_sp,
                    "l4_mean_voltage": l4_v_sum / (n/2) as f64,
                    "l5_mean_voltage": l5_v_sum / l5_count as f64,
                    "l4_mean_threshold": l4_th_sum / (n/2) as f64,
                    "l5_mean_threshold": l5_th_sum / l5_count as f64,
                    "l4_mean_fatigue": l4_fatigue_sum / (n/2) as f64,
                    "l5_mean_fatigue": l5_fatigue_sum / l5_count as f64,
                    "silence_flag": silence_flag,
                    "runaway_flag": runaway_flag,
                }));
            }
        }

        // Trace and extract weight changes
        let mut edge_log = Vec::new();
        let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());
        for &(src, dest, initial_w) in &edges {
            let mut final_w = initial_w;
            for slot in 0..128 {
                let target = snap_state_buf.read_dendrite_target(slot, dest);
                if let Some((src_id, _)) = types::PackedTarget(target).unpack() {
                    if src_id == src as u32 {
                        final_w = snap_state_buf.read_dendrite_weight(slot, dest) >> 16;
                        break;
                    }
                }
            }
            let delta = final_w - initial_w;

            let projection = if src >= n && dest < n / 2 {
                "Virtual -> L4"
            } else if src < n / 2 && dest >= n / 2 && dest < 3 * n / 4 {
                "L4 -> L23"
            } else if src < n / 2 && dest >= 3 * n / 4 && dest < n {
                "L4 -> L5"
            } else if src >= n / 2 && src < 3 * n / 4 && dest < n / 2 {
                "L23 -> L4"
            } else if src >= n / 2 && src < 3 * n / 4 && dest >= 3 * n / 4 && dest < n {
                "L23 -> L5"
            } else if src >= n / 2 && src < 3 * n / 4 && dest >= n / 2 && dest < 3 * n / 4 {
                "L23 -> L23"
            } else if src >= 3 * n / 4 && src < n && dest >= n / 2 && dest < 3 * n / 4 {
                "L5 -> L23"
            } else {
                "Unknown"
            };

            let virtual_group = if src >= n {
                let virt_idx = src - n;
                if virt_idx < virt_count / 2 {
                    "A"
                } else {
                    "B"
                }
            } else {
                "None"
            };

            let l4_preferred_group = if dest < n / 2 {
                if l4_group_a_pref[dest] {
                    "A"
                } else {
                    "B"
                }
            } else {
                "None"
            };

            let is_matched = if src >= n && dest < n / 2 {
                virtual_group == l4_preferred_group
            } else if src < n / 2 && dest >= n / 2 && dest < 3 * n / 4 {
                let src_pref = if l4_group_a_pref[src] { "A" } else { "B" };
                let target_pref = if l23_group_a_pref[dest - n / 2] {
                    "A"
                } else {
                    "B"
                };
                src_pref == target_pref
            } else if src < n / 2 && dest >= 3 * n / 4 && dest < n {
                let src_pref = if l4_group_a_pref[src] { "A" } else { "B" };
                let target_pref = if l5_group_a_pref[dest - 3 * n / 4] {
                    "A"
                } else {
                    "B"
                };
                src_pref == target_pref
            } else {
                false
            };

            let src_coords = if src < n {
                let c = coordinates[src];
                Some((c.0, c.1, c.2))
            } else {
                None
            };
            let dest_coords = coordinates[dest];

            edge_log.push(serde_json::json!({
                "src": src,
                "dest": dest,
                "projection": projection,
                "src_group": virtual_group,
                "dest_group": l4_preferred_group,
                "is_matched": is_matched,
                "initial_weight": initial_w,
                "final_weight": final_w,
                "delta_signed": delta,
                "delta_abs": delta.abs(),
                "is_inhibitory": src >= n / 2 && src < 3 * n / 4,
                "src_coords": src_coords,
                "dest_coords": (dest_coords.0, dest_coords.1, dest_coords.2),
            }));
        }

        let total_time_sec = total_ticks as f64 / 1000.0;
        let r4 = (total_l4_spikes as f64 / (n / 2) as f64) / total_time_sec;
        let r23 = (total_l23_spikes as f64 / (n / 4) as f64) / total_time_sec;
        let r5 = (total_l5_spikes as f64 / (n / 4) as f64) / total_time_sec;

        backend.free_shard(handle).unwrap();
        (
            sim_log,
            edge_log,
            r4,
            r23,
            r5,
            silence_flag_any,
            runaway_flag_any,
        )
    };

    // Parameters sweep definitions
    let structured_p_opts = vec![0.050, 0.075];
    let background_p_opts = vec![0.000, 0.003];
    let block_size_opts = vec![250, 500];
    let learning_ticks = 50000;

    let mut sweep_results = Vec::new();
    let mut best_score = -999999.0;
    let mut best_params = (0.050, 0.000, 500);

    println!("--- Phase 1: Parameter Sweep on N=256 (8 combinations) ---");

    for &s_p in &structured_p_opts {
        for &b_p in &background_p_opts {
            for &blk_sz in &block_size_opts {
                println!(
                    "Running sweep: structured_p={:.3}, background_p={:.3}, block_size={}",
                    s_p, b_p, blk_sz
                );
                let (_sim_log, edge_log, r4, r23, r5, silence, runaway) =
                    run_simulation(256, learning_ticks, s_p, b_p, blk_sz);

                // Compute sweep metrics
                let mut corr_deltas = Vec::new();
                let mut uncorr_deltas = Vec::new();
                let mut corr_pos_count = 0;
                let mut corr_total = 0;
                let mut uncorr_pos_count = 0;
                let mut uncorr_total = 0;

                let mut l4_l23_corr_deltas = Vec::new();
                let mut l4_l23_uncorr_deltas = Vec::new();

                for e in &edge_log {
                    let proj = e["projection"].as_str().unwrap();
                    let is_matched = e["is_matched"].as_bool().unwrap();
                    let delta = e["delta_signed"].as_i64().unwrap();

                    if proj == "Virtual -> L4" {
                        if is_matched {
                            corr_deltas.push(delta);
                            corr_total += 1;
                            if delta > 0 {
                                corr_pos_count += 1;
                            }
                        } else {
                            uncorr_deltas.push(delta);
                            uncorr_total += 1;
                            if delta > 0 {
                                uncorr_pos_count += 1;
                            }
                        }
                    } else if proj == "L4 -> L23" {
                        if is_matched {
                            l4_l23_corr_deltas.push(delta);
                        } else {
                            l4_l23_uncorr_deltas.push(delta);
                        }
                    }
                }

                let mean_corr = if corr_deltas.is_empty() {
                    0.0
                } else {
                    corr_deltas.iter().sum::<i64>() as f64 / corr_deltas.len() as f64
                };
                let mean_uncorr = if uncorr_deltas.is_empty() {
                    0.0
                } else {
                    uncorr_deltas.iter().sum::<i64>() as f64 / uncorr_deltas.len() as f64
                };
                let corr_pos_ratio = if corr_total == 0 {
                    0.0
                } else {
                    corr_pos_count as f64 / corr_total as f64
                };
                let uncorr_pos_ratio = if uncorr_total == 0 {
                    0.0
                } else {
                    uncorr_pos_count as f64 / uncorr_total as f64
                };

                let l4_l23_corr_mean = if l4_l23_corr_deltas.is_empty() {
                    0.0
                } else {
                    l4_l23_corr_deltas.iter().sum::<i64>() as f64 / l4_l23_corr_deltas.len() as f64
                };
                let l4_l23_uncorr_mean = if l4_l23_uncorr_deltas.is_empty() {
                    0.0
                } else {
                    l4_l23_uncorr_deltas.iter().sum::<i64>() as f64
                        / l4_l23_uncorr_deltas.len() as f64
                };
                let l4_l23_bias = l4_l23_corr_mean - l4_l23_uncorr_mean;

                // Check physiology gates
                let phys_ok = r4 >= 3.0
                    && r4 <= 25.0
                    && r23 >= 3.0
                    && r23 <= 35.0
                    && r5 >= 1.0
                    && r5 <= 15.0
                    && !silence
                    && !runaway;
                let mean_corr_pos = mean_corr > 0.0;
                let ratio_2x = corr_pos_ratio >= 2.0 * uncorr_pos_ratio;
                let passed_all = phys_ok && mean_corr_pos && ratio_2x && (l4_l23_bias > 0.0);

                println!("  L4: {:.2} Hz, L23: {:.2} Hz, L5: {:.2} Hz | Mean Corr Delta: {:.3} uV (pos ratio: {:.3}), Mean Uncorr: {:.3} uV (pos ratio: {:.3})", 
                         r4, r23, r5, mean_corr, corr_pos_ratio, mean_uncorr, uncorr_pos_ratio);
                println!("  Downstream L4->L23 Matched Mean: {:.3} uV, Unmatched: {:.3} uV, Bias: {:.3} uV", l4_l23_corr_mean, l4_l23_uncorr_mean, l4_l23_bias);
                println!("  Passed All Gates: {}", passed_all);

                sweep_results.push(serde_json::json!({
                    "structured_p": s_p,
                    "background_p": b_p,
                    "block_size": blk_sz,
                    "l4_rate": r4,
                    "l23_rate": r23,
                    "l5_rate": r5,
                    "mean_corr_delta": mean_corr,
                    "mean_uncorr_delta": mean_uncorr,
                    "corr_pos_ratio": corr_pos_ratio,
                    "uncorr_pos_ratio": uncorr_pos_ratio,
                    "l4_l23_bias": l4_l23_bias,
                    "passed_all_gates": passed_all,
                }));

                // Score candidate (we maximize mean_corr if phys_ok, otherwise negative score)
                let score = if phys_ok {
                    let bonus = if mean_corr_pos { 1000.0 } else { 0.0 };
                    let ratio_bonus = if ratio_2x { 500.0 } else { 0.0 };
                    mean_corr + bonus + ratio_bonus + l4_l23_bias * 2.0
                } else {
                    -10000.0 + r4
                };

                if score > best_score {
                    best_score = score;
                    best_params = (s_p, b_p, blk_sz);
                }
            }
        }
    }

    let sweep_summary_path = artifacts_dir.join("plastic_microcircuit_v1_1_sweep_summary.json");
    let file = File::create(&sweep_summary_path).unwrap();
    serde_json::to_writer_pretty(file, &sweep_results).unwrap();

    let (s_p_best, b_p_best, blk_sz_best) = best_params;
    println!(
        "Winner Parameters: structured_p={:.3}, background_p={:.3}, block_size={}",
        s_p_best, b_p_best, blk_sz_best
    );

    // Phase 2: N=256 Learning Run with best candidate (extended to 100,000 learning ticks)
    println!("--- Phase 2: Running Winner N=256 Learning Run (100,000 ticks) ---");
    let learning_ticks_winner = 100000;
    let (sim_log_winner, edge_log_winner, r4_w, r23_w, r5_w, _, _) =
        run_simulation(256, learning_ticks_winner, s_p_best, b_p_best, blk_sz_best);

    let winner_sim_path =
        artifacts_dir.join("plastic_microcircuit_v1_1_best_log_256_learning.json");
    let file = File::create(&winner_sim_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_winner).unwrap();

    let winner_edge_path = artifacts_dir.join("plastic_microcircuit_v1_1_best_edge_log_256.json");
    let file = File::create(&winner_edge_path).unwrap();
    serde_json::to_writer_pretty(file, &edge_log_winner).unwrap();

    // Check invariants and delta counts
    let mut dale_violations = 0;
    let mut sign_flips = 0;
    let mut corr_pos_count = 0;
    let mut corr_total = 0;
    let mut uncorr_pos_count = 0;
    let mut uncorr_total = 0;
    let mut sum_abs_delta = 0.0;
    let mut corr_deltas = Vec::new();
    let mut uncorr_deltas = Vec::new();

    for edge in &edge_log_winner {
        let init_w = edge["initial_weight"].as_i64().unwrap();
        let final_w = edge["final_weight"].as_i64().unwrap();
        let is_inh = edge["is_inhibitory"].as_bool().unwrap();
        let delta = edge["delta_signed"].as_i64().unwrap();
        let proj = edge["projection"].as_str().unwrap();
        let is_matched = edge["is_matched"].as_bool().unwrap();

        sum_abs_delta += delta.abs() as f64;

        if is_inh {
            if final_w > 0 {
                dale_violations += 1;
            }
            if init_w < 0 && final_w > 0 {
                sign_flips += 1;
            }
        } else {
            if final_w < 0 {
                dale_violations += 1;
            }
            if init_w > 0 && final_w < 0 {
                sign_flips += 1;
            }
        }

        if proj == "Virtual -> L4" {
            if is_matched {
                corr_deltas.push(delta);
                corr_total += 1;
                if delta > 0 {
                    corr_pos_count += 1;
                }
            } else {
                uncorr_deltas.push(delta);
                uncorr_total += 1;
                if delta > 0 {
                    uncorr_pos_count += 1;
                }
            }
        }
    }

    let mean_corr = if corr_deltas.is_empty() {
        0.0
    } else {
        corr_deltas.iter().sum::<i64>() as f64 / corr_deltas.len() as f64
    };
    let mean_uncorr = if uncorr_deltas.is_empty() {
        0.0
    } else {
        uncorr_deltas.iter().sum::<i64>() as f64 / uncorr_deltas.len() as f64
    };
    let corr_pos_ratio = if corr_total == 0 {
        0.0
    } else {
        corr_pos_count as f64 / corr_total as f64
    };
    let uncorr_pos_ratio = if uncorr_total == 0 {
        0.0
    } else {
        uncorr_pos_count as f64 / uncorr_total as f64
    };
    let mean_abs = sum_abs_delta / edge_log_winner.len() as f64;

    println!("Winner N=256 Learning Statistics:");
    println!(
        "  Dale Violations: {}, Sign Flips: {}",
        dale_violations, sign_flips
    );
    println!("  Mean Abs Weight Delta: {:.4} uV", mean_abs);
    println!(
        "  Mean Matched/Correlated Virtual->L4 Delta: {:.4} uV (pos ratio: {:.3})",
        mean_corr, corr_pos_ratio
    );
    println!(
        "  Mean Unmatched/Uncorrelated Virtual->L4 Delta: {:.4} uV (pos ratio: {:.3})",
        mean_uncorr, uncorr_pos_ratio
    );

    // Also run a short N=256 sanity run of 9,000 ticks with best candidate parameters
    println!("--- Phase 2.1: Running Winner N=256 Sanity Run (9,000 ticks) ---");
    let (sim_log_w_sanity, _, _, _, _, _, _) =
        run_simulation(256, 9000, s_p_best, b_p_best, blk_sz_best);
    let winner_sanity_path =
        artifacts_dir.join("plastic_microcircuit_v1_1_best_log_256_sanity.json");
    let file = File::create(&winner_sanity_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_w_sanity).unwrap();

    // Phase 3: N=512 Sanity Run (9,000 ticks)
    println!("--- Phase 3: Running Winner N=512 Sanity Run (9,000 ticks) ---");
    let (sim_log_512_sanity, edge_log_512_sanity, r4_512, r23_512, r5_512, _, _) =
        run_simulation(512, 9000, s_p_best, b_p_best, blk_sz_best);

    let file_path = artifacts_dir.join("plastic_microcircuit_v1_1_best_log_512_sanity.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_512_sanity).unwrap();

    let mut dale_violations_512 = 0;
    let mut sign_flips_512 = 0;
    for edge in &edge_log_512_sanity {
        let final_w = edge["final_weight"].as_i64().unwrap();
        let init_w = edge["initial_weight"].as_i64().unwrap();
        let is_inh = edge["is_inhibitory"].as_bool().unwrap();
        if is_inh {
            if final_w > 0 {
                dale_violations_512 += 1;
            }
            if init_w < 0 && final_w > 0 {
                sign_flips_512 += 1;
            }
        } else {
            if final_w < 0 {
                dale_violations_512 += 1;
            }
            if init_w > 0 && final_w < 0 {
                sign_flips_512 += 1;
            }
        }
    }

    // Write research summary to summary JSON
    let summary = serde_json::json!({
        "winner_params": {
            "structured_p": s_p_best,
            "background_p": b_p_best,
            "block_size": blk_sz_best,
        },
        "learning_256": {
            "r4": r4_w,
            "r23": r23_w,
            "r5": r5_w,
            "mean_abs_delta": mean_abs,
            "dale_violations": dale_violations,
            "sign_flips": sign_flips,
            "mean_corr_delta": mean_corr,
            "mean_uncorr_delta": mean_uncorr,
            "corr_pos_ratio": corr_pos_ratio,
            "uncorr_pos_ratio": uncorr_pos_ratio,
        },
        "sanity_512": {
            "r4": r4_512,
            "r23": r23_512,
            "r5": r5_512,
            "dale_violations": dale_violations_512,
            "sign_flips": sign_flips_512,
        }
    });

    let summary_path = artifacts_dir.join("plastic_microcircuit_v1_1_summary.json");
    let file = File::create(&summary_path).unwrap();
    serde_json::to_writer_pretty(file, &summary).unwrap();

    println!("Plastic Microcircuit v1.1 Rust simulations complete.");
}

#[test]
#[allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::useless_vec,
    clippy::manual_range_contains,
    clippy::type_complexity,
    clippy::manual_is_multiple_of,
    unused_imports,
    unused_variables,
    unused_assignments,
    clippy::unnecessary_cast
)]
fn run_plastic_microcircuit_v1_2_experiments() {
    println!("=== Starting Plastic Microcircuit v1.2 Experiments ===");
    use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
    use compute_cpu::{CpuBackend, CpuBackendConfig};
    use std::collections::VecDeque;
    use std::fs::File;
    use test_harness::{MvpAxonBuffer, MvpStateBuffer};
    use types::{PackedTarget, SomaFlags};

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

    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("artifacts");
    fs::create_dir_all(&artifacts_dir).unwrap();

    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let var_visl4_base = load_variant(path_visl4);
    let var_visp5 = load_variant(path_visp5);
    let var_visp23 = load_variant(path_visp23);

    let mut variant_table = [bytemuck::Zeroable::zeroed(); layout::VARIANT_LUT_LEN];
    variant_table[0] = var_visl4_base;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // Simulation wrapper
    let run_simulation = |n: usize,
                          learning_ticks: usize,
                          structured_p: f32,
                          background_p: f32,
                          block_size: usize,
                          fatigue_cap: u8,
                          gsop_pot: u16,
                          gsop_dep: u16,
                          virt_w: i32,
                          inh_l23_l4: i32|
     -> (
        Vec<serde_json::Value>, // sim_log
        Vec<serde_json::Value>, // edge_log
        f64,                    // r4
        f64,                    // r23
        f64,                    // r5
        bool,                   // silence_any
        bool,                   // runaway_any
    ) {
        let exc_w_l4_l23 = 3000;
        let exc_w_l4_l5 = 5000;
        let fan_in_l4_l5_range_idx = 1;
        let inh_w_l23_l5 = -1250;

        let padded_n = n.div_ceil(64) * 64;
        let total_axons = padded_n + padded_n / 2;
        let virt_count = total_axons - n;

        let mut rng = SimpleRng::new(100 + (n as u64) * 31 + (learning_ticks as u64) * 7);

        let mut variant_table_local = variant_table;
        variant_table_local[0].fatigue_capacity = fatigue_cap;
        variant_table_local[0].gsop_potentiation = gsop_pot;
        variant_table_local[0].gsop_depression = gsop_dep;

        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axons_buf = MvpAxonBuffer::new(total_axons);

        // Soma layout
        for i in 0..padded_n {
            let type_id = if i < n / 2 {
                0
            } else if i < 3 * n / 4 {
                2
            } else if i < n {
                1
            } else {
                0
            };
            let var = &variant_table_local[type_id];
            state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
            state_buf.write_soma_voltage(i, var.rest_potential);
            state_buf.write_soma_to_axon(i, i as u32);
        }

        // Coordinates
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

        // Group assignments
        let mut l4_group_a_pref = vec![false; n / 2];
        for i in 0..(n / 2) {
            let mut rng_local = SimpleRng::new(500 + i as u64);
            l4_group_a_pref[i] = rng_local.next_u32() % 2 == 0;
        }

        let mut l23_group_a_pref = vec![false; n / 4];
        for i in 0..(n / 4) {
            let mut rng_local = SimpleRng::new(600 + i as u64);
            l23_group_a_pref[i] = rng_local.next_u32() % 2 == 0;
        }

        let mut l5_group_a_pref = vec![false; n / 4];
        for i in 0..(n / 4) {
            let mut rng_local = SimpleRng::new(700 + i as u64);
            l5_group_a_pref[i] = rng_local.next_u32() % 2 == 0;
        }

        // Topology Edges
        let mut edges = Vec::new();

        // Virtual -> L4 (excitatory)
        for dest in 0..(n / 2) {
            let pref_a = l4_group_a_pref[dest];
            let mut candidates = Vec::new();
            for src in n..total_axons {
                let virt_idx = src - n;
                let is_virt_a = virt_idx < virt_count / 2;
                let d = if pref_a == is_virt_a {
                    rng.range(50, 150) as f32
                } else {
                    rng.range(200, 400) as f32
                };
                candidates.push((src, d));
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            for k in 0..8 {
                edges.push((candidates[k].0, dest, virt_w));
            }
        }

        // L4 -> L23 (excitatory)
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

        // L4 -> L5 (excitatory)
        let fan_in_l4_l5_opts = [32, 16];
        let fan_in_l4_l5 = fan_in_l4_l5_opts[fan_in_l4_l5_range_idx];
        for dest in (3 * n / 4)..n {
            let mut candidates = Vec::new();
            for src in 0..(n / 2) {
                let (x1, y1, z1) = coordinates[src];
                let (x2, y2, z2) = coordinates[dest];
                let d = ((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)).sqrt();
                candidates.push((src, d));
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let actual_fan_in = fan_in_l4_l5.min(candidates.len());
            for k in 0..actual_fan_in {
                edges.push((candidates[k].0, dest, exc_w_l4_l5));
            }
        }

        // L23 -> L4 (inhibitory)
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
                edges.push((candidates[k].0, dest, inh_l23_l4));
            }
        }

        // L23 -> L5 (inhibitory)
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

        // L23 -> L23 (inhibitory)
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

        // L5 -> L23 (excitatory)
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

        // Write edges to dendrites
        let mut dest_fan_in = vec![0; padded_n];
        for &(src, dest, w) in &edges {
            let slot = dest_fan_in[dest];
            assert!(slot < 128, "Soma {} exceeded 128 synapses", dest);
            let target = types::PackedTarget::pack(src as u32, 0).0;
            state_buf.write_dendrite_target(slot, dest, target);
            state_buf.write_dendrite_weight(slot, dest, (w as i32) << 16);
            dest_fan_in[dest] += 1;
        }

        // Run CPU backend
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
                    variant_table: &variant_table_local,
                },
            )
            .unwrap();

        let max_spikes = total_axons as u32;
        let mut out_spikes = vec![0u32; max_spikes as usize];
        let mut out_counts = vec![0u32; 1];
        let mapped_somas: Vec<u32> = (0..padded_n as u32).collect();

        let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
        let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

        let mut incoming_padded = vec![0u32; max_spikes as usize];
        let mut recent_spikes = VecDeque::new();
        let mut sim_log = Vec::new();

        let is_learning = learning_ticks > 10000;
        let total_ticks = learning_ticks + 25000 + 10000;
        let block_start = 25000;

        let mut total_l4_spikes = 0;
        let mut total_l23_spikes = 0;
        let mut total_l5_spikes = 0;

        let mut active_l4_spikes = 0;
        let mut active_l23_spikes = 0;
        let mut active_l5_spikes = 0;

        let mut silence_flag_any = false;
        let mut runaway_flag_any = false;

        let p_weak = 0.006;
        let p_mod = 0.020;

        for tick in 0..total_ticks {
            let mut incoming_count = 0;

            let p_val = if !is_learning {
                if tick < 1000 {
                    0.0
                } else if tick < 3000 {
                    p_weak
                } else if tick < 5000 {
                    p_mod
                } else if tick < 7000 {
                    structured_p
                } else {
                    0.0
                }
            } else {
                if tick < 5000 {
                    0.0
                } else if tick < 15000 {
                    p_weak
                } else if tick < 25000 {
                    p_mod
                } else if tick < block_start + learning_ticks {
                    structured_p
                } else {
                    0.0
                }
            };

            let is_structured = if !is_learning {
                tick >= 5000 && tick < 7000
            } else {
                tick >= block_start && tick < block_start + learning_ticks
            };

            if is_structured {
                let current_block_start = if !is_learning { 5000 } else { block_start };
                let group_a = ((tick - current_block_start) / block_size) % 2 == 0;
                for axon_idx in n..total_axons {
                    let is_a = axon_idx < n + virt_count / 2;
                    let prob = if is_a == group_a { p_val } else { background_p };
                    if rng.next_f32() < prob {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            } else {
                for axon_idx in n..total_axons {
                    if rng.next_f32() < p_val {
                        incoming_padded[incoming_count] = axon_idx as u32;
                        incoming_count += 1;
                    }
                }
            }

            out_counts[0] = 0;
            out_spikes.fill(0);

            let cmd = DayBatchCmd {
                sync_batch_ticks: 1,
                tick_base: tick as u64,
                v_seg: 1,
                dopamine: 0,
                input_bitmask: None,
                num_virtual_axons: 0,
                virtual_offset: 0,
                input_words_per_tick: 0,
                incoming_spikes: if incoming_count > 0 {
                    Some(&incoming_padded)
                } else {
                    None
                },
                incoming_spike_counts: &[incoming_count as u32],
                max_spikes_per_tick: max_spikes,
                num_outputs: padded_n as u32,
                mapped_soma_ids: &mapped_somas,
                output_spikes: &mut out_spikes,
                output_spike_counts: &mut out_counts,
            };

            backend.run_day_batch(handle, cmd).unwrap();

            // Read snapshot
            backend
                .debug_snapshot(
                    handle,
                    ShardSnapshotMut {
                        state_blob: &mut snap_state,
                        axons_blob: &mut snap_axons,
                    },
                )
                .unwrap();

            let snap_state_buf_tick =
                MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

            let mut l4_v_sum = 0.0;
            let mut l4_th_sum = 0.0;
            let mut l4_fatigue_sum = 0.0;

            let mut l5_v_sum = 0.0;
            let mut l5_th_sum = 0.0;
            let mut l5_fatigue_sum = 0.0;
            let mut l5_count = 0;

            for i in 0..padded_n {
                let v = snap_state_buf_tick.read_soma_voltage(i) as f64;
                let th = snap_state_buf_tick.read_threshold_offset(i) as f64;

                let mut fatigue_timer_sum = 0;
                for slot in 0..128 {
                    let timer = snap_state_buf_tick.read_dendrite_timer(slot, i);
                    fatigue_timer_sum += timer as usize;
                }
                let fatigue = fatigue_timer_sum as f64 / (128.0 * 255.0);

                if i < n / 2 {
                    l4_v_sum += v;
                    l4_th_sum += th;
                    l4_fatigue_sum += fatigue;
                } else if i >= 3 * n / 4 && i < n {
                    l5_v_sum += v;
                    l5_th_sum += th;
                    l5_fatigue_sum += fatigue;
                    l5_count += 1;
                }
            }

            let count = out_counts[0] as usize;
            let mut tick_spikes = vec![false; padded_n];
            for &id in &out_spikes[0..count] {
                if id < padded_n as u32 {
                    tick_spikes[id as usize] = true;
                }
            }

            let mut l4_sp = 0;
            let mut l23_sp = 0;
            let mut l5_sp = 0;
            for i in 0..n {
                if tick_spikes[i] {
                    if i < n / 2 {
                        l4_sp += 1;
                    } else if i < 3 * n / 4 {
                        l23_sp += 1;
                    } else {
                        l5_sp += 1;
                    }
                }
            }

            total_l4_spikes += l4_sp;
            total_l23_spikes += l23_sp;
            total_l5_spikes += l5_sp;

            let is_active = if !is_learning {
                tick >= 3000 && tick <= 7000
            } else {
                tick >= 15000 && tick <= 125000
            };

            if is_active {
                active_l4_spikes += l4_sp;
                active_l23_spikes += l23_sp;
                active_l5_spikes += l5_sp;
            }

            recent_spikes.push_back((l4_sp, l23_sp, l5_sp));
            if recent_spikes.len() > 100 {
                recent_spikes.pop_front();
            }

            let sum_recent: (usize, usize, usize) = recent_spikes
                .iter()
                .fold((0, 0, 0), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
            let rate_recent_l4 =
                (sum_recent.0 as f64 / (recent_spikes.len() as f64 * (n / 2) as f64)) * 1000.0;
            let rate_recent_l23 =
                (sum_recent.1 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;
            let rate_recent_l5 =
                (sum_recent.2 as f64 / (recent_spikes.len() as f64 * (n / 4) as f64)) * 1000.0;

            let silence_flag = rate_recent_l4 < 0.01;
            let runaway_flag =
                rate_recent_l4 > 120.0 || rate_recent_l23 > 120.0 || rate_recent_l5 > 120.0;

            if silence_flag {
                silence_flag_any = true;
            }
            if runaway_flag {
                runaway_flag_any = true;
            }

            if tick % 10 == 0 {
                sim_log.push(serde_json::json!({
                    "tick": tick,
                    "l4_spikes": l4_sp,
                    "l23_spikes": l23_sp,
                    "l5_spikes": l5_sp,
                    "l4_mean_voltage": l4_v_sum / (n/2) as f64,
                    "l5_mean_voltage": l5_v_sum / l5_count as f64,
                    "l4_mean_threshold": l4_th_sum / (n/2) as f64,
                    "l5_mean_threshold": l5_th_sum / l5_count as f64,
                    "l4_mean_fatigue": l4_fatigue_sum / (n/2) as f64,
                    "l5_mean_fatigue": l5_fatigue_sum / l5_count as f64,
                    "silence_flag": silence_flag,
                    "runaway_flag": runaway_flag,
                }));
            }
        }

        // Trace and extract weight changes
        let mut edge_log = Vec::new();
        let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());
        for &(src, dest, initial_w) in &edges {
            let initial_mass = (initial_w as i64) << 16;
            let mut final_mass = initial_mass;
            for slot in 0..128 {
                let target = snap_state_buf.read_dendrite_target(slot, dest);
                if let Some((src_id, _)) = types::PackedTarget(target).unpack() {
                    if src_id == src as u32 {
                        final_mass = snap_state_buf.read_dendrite_weight(slot, dest) as i64;
                        break;
                    }
                }
            }
            let delta_mass = final_mass - initial_mass;
            let initial_charge = initial_mass >> 16;
            let final_charge = final_mass >> 16;
            let delta_charge_visible = final_charge - initial_charge;
            let delta_charge_exact = delta_mass as f64 / 65536.0;

            let projection = if src >= n && dest < n / 2 {
                "Virtual -> L4"
            } else if src < n / 2 && dest >= n / 2 && dest < 3 * n / 4 {
                "L4 -> L23"
            } else if src < n / 2 && dest >= 3 * n / 4 && dest < n {
                "L4 -> L5"
            } else if src >= n / 2 && src < 3 * n / 4 && dest < n / 2 {
                "L23 -> L4"
            } else if src >= n / 2 && src < 3 * n / 4 && dest >= 3 * n / 4 && dest < n {
                "L23 -> L5"
            } else if src >= n / 2 && src < 3 * n / 4 && dest >= n / 2 && dest < 3 * n / 4 {
                "L23 -> L23"
            } else if src >= 3 * n / 4 && src < n && dest >= n / 2 && dest < 3 * n / 4 {
                "L5 -> L23"
            } else {
                "Unknown"
            };

            let virtual_group = if src >= n {
                let virt_idx = src - n;
                if virt_idx < virt_count / 2 {
                    "A"
                } else {
                    "B"
                }
            } else {
                "None"
            };

            let l4_preferred_group = if dest < n / 2 {
                if l4_group_a_pref[dest] {
                    "A"
                } else {
                    "B"
                }
            } else {
                "None"
            };

            let is_matched = if src >= n && dest < n / 2 {
                virtual_group == l4_preferred_group
            } else if src < n / 2 && dest >= n / 2 && dest < 3 * n / 4 {
                let src_pref = if l4_group_a_pref[src] { "A" } else { "B" };
                let target_pref = if l23_group_a_pref[dest - n / 2] {
                    "A"
                } else {
                    "B"
                };
                src_pref == target_pref
            } else if src < n / 2 && dest >= 3 * n / 4 && dest < n {
                let src_pref = if l4_group_a_pref[src] { "A" } else { "B" };
                let target_pref = if l5_group_a_pref[dest - 3 * n / 4] {
                    "A"
                } else {
                    "B"
                };
                src_pref == target_pref
            } else {
                false
            };

            let src_coords = if src < n {
                let c = coordinates[src];
                Some((c.0, c.1, c.2))
            } else {
                None
            };
            let dest_coords = coordinates[dest];

            edge_log.push(serde_json::json!({
                "src": src,
                "dest": dest,
                "projection": projection,
                "src_group": virtual_group,
                "dest_group": l4_preferred_group,
                "is_matched": is_matched,
                "initial_weight": initial_charge,
                "final_weight": final_charge,
                "delta_signed": delta_charge_visible,
                "initial_mass": initial_mass,
                "final_mass": final_mass,
                "delta_mass": delta_mass,
                "initial_charge": initial_charge,
                "final_charge": final_charge,
                "delta_charge_visible": delta_charge_visible,
                "delta_charge_exact": delta_charge_exact,
                "is_inhibitory": src >= n / 2 && src < 3 * n / 4,
                "src_coords": src_coords,
                "dest_coords": (dest_coords.0, dest_coords.1, dest_coords.2),
            }));
        }

        let active_duration = if !is_learning { 4000.0 } else { 110000.0 } / 1000.0;

        let r4 = (active_l4_spikes as f64 / (n / 2) as f64) / active_duration;
        let r23 = (active_l23_spikes as f64 / (n / 4) as f64) / active_duration;
        let r5 = (active_l5_spikes as f64 / (n / 4) as f64) / active_duration;

        backend.free_shard(handle).unwrap();

        (
            sim_log,
            edge_log,
            r4,
            r23,
            r5,
            silence_flag_any,
            runaway_flag_any,
        )
    };

    // Predefined 16 sweep candidates
    let sweep_candidates = vec![
        (15, 138, 81, 1500, -1200), // 0: Baseline
        (18, 138, 81, 1500, -1200), // 1: Fatigue +25%
        (22, 138, 81, 1500, -1200), // 2: Fatigue +50%
        (15, 172, 81, 1500, -1200), // 3: LTP +25%
        (15, 207, 81, 1500, -1200), // 4: LTP +50%
        (15, 138, 68, 1500, -1200), // 5: LTD -15%
        (15, 138, 56, 1500, -1200), // 6: LTD -30%
        (15, 138, 81, 1750, -1200), // 7: VirtW 1750
        (15, 138, 81, 2000, -1200), // 8: VirtW 2000
        (15, 138, 81, 1500, -1000), // 9: Inh -1000
        (15, 138, 81, 1500, -900),  // 10: Inh -900
        (18, 172, 68, 1750, -1000), // 11: Combined 1
        (22, 207, 56, 2000, -900),  // 12: Combined 2
        (18, 207, 68, 2000, -1000), // 13: Combined 3
        (22, 172, 56, 1750, -900),  // 14: Combined 4
        (18, 172, 56, 2000, -900),  // 15: Combined 5
    ];

    let learning_ticks = 50000;
    let structured_p = 0.075;
    let background_p = 0.003;
    let block_size = 250;

    let mut sweep_results = Vec::new();
    let mut best_score = -999999.0;
    let mut best_params = (15, 138, 81, 1500, -1200);

    println!("--- Phase 1: Parameter Sweep on N=256 (16 combinations) ---");

    for (idx, &(fatigue_cap, gsop_pot, gsop_dep, virt_w, inh)) in
        sweep_candidates.iter().enumerate()
    {
        println!(
            "Running sweep combo {}: fatigue_cap={}, gsop_pot={}, gsop_dep={}, virt_w={}, inh={}",
            idx, fatigue_cap, gsop_pot, gsop_dep, virt_w, inh
        );
        let (sim_log, edge_log, r4, r23, r5, silence, runaway) = run_simulation(
            256,
            learning_ticks,
            structured_p,
            background_p,
            block_size,
            fatigue_cap,
            gsop_pot,
            gsop_dep,
            virt_w,
            inh,
        );

        let mut corr_deltas_mass = Vec::new();
        let mut uncorr_deltas_mass = Vec::new();
        let mut corr_deltas_exact = Vec::new();
        let mut uncorr_deltas_exact = Vec::new();
        let mut corr_deltas_visible = Vec::new();
        let mut uncorr_deltas_visible = Vec::new();
        let mut corr_pos_count = 0;
        let mut corr_total = 0;
        let mut uncorr_pos_count = 0;
        let mut uncorr_total = 0;

        let mut l4_l23_matched_deltas = Vec::new();
        let mut l4_l23_unmatched_deltas = Vec::new();
        let mut l4_l5_matched_deltas = Vec::new();
        let mut l4_l5_unmatched_deltas = Vec::new();

        for edge in &edge_log {
            let is_matched = edge["is_matched"].as_bool().unwrap();
            let proj = edge["projection"].as_str().unwrap();
            let d_mass = edge["delta_mass"].as_i64().unwrap();
            let d_exact = edge["delta_charge_exact"].as_f64().unwrap();
            let d_visible = edge["delta_charge_visible"].as_i64().unwrap();

            if proj == "Virtual -> L4" {
                if is_matched {
                    corr_deltas_mass.push(d_mass);
                    corr_deltas_exact.push(d_exact);
                    corr_deltas_visible.push(d_visible);
                    corr_total += 1;
                    if d_mass > 0 {
                        corr_pos_count += 1;
                    }
                } else {
                    uncorr_deltas_mass.push(d_mass);
                    uncorr_deltas_exact.push(d_exact);
                    uncorr_deltas_visible.push(d_visible);
                    uncorr_total += 1;
                    if d_mass > 0 {
                        uncorr_pos_count += 1;
                    }
                }
            } else if proj == "L4 -> L23" {
                if is_matched {
                    l4_l23_matched_deltas.push(d_mass);
                } else {
                    l4_l23_unmatched_deltas.push(d_mass);
                }
            } else if proj == "L4 -> L5" {
                if is_matched {
                    l4_l5_matched_deltas.push(d_mass);
                } else {
                    l4_l5_unmatched_deltas.push(d_mass);
                }
            }
        }

        let mean_corr_mass = if corr_deltas_mass.is_empty() {
            0.0
        } else {
            corr_deltas_mass.iter().sum::<i64>() as f64 / corr_deltas_mass.len() as f64
        };
        let mean_uncorr_mass = if uncorr_deltas_mass.is_empty() {
            0.0
        } else {
            uncorr_deltas_mass.iter().sum::<i64>() as f64 / uncorr_deltas_mass.len() as f64
        };

        let mean_corr_exact = if corr_deltas_exact.is_empty() {
            0.0
        } else {
            corr_deltas_exact.iter().sum::<f64>() / corr_deltas_exact.len() as f64
        };
        let mean_uncorr_exact = if uncorr_deltas_exact.is_empty() {
            0.0
        } else {
            uncorr_deltas_exact.iter().sum::<f64>() / uncorr_deltas_exact.len() as f64
        };

        let mean_corr_visible = if corr_deltas_visible.is_empty() {
            0.0
        } else {
            corr_deltas_visible.iter().sum::<i64>() as f64 / corr_deltas_visible.len() as f64
        };
        let mean_uncorr_visible = if uncorr_deltas_visible.is_empty() {
            0.0
        } else {
            uncorr_deltas_visible.iter().sum::<i64>() as f64 / uncorr_deltas_visible.len() as f64
        };

        let corr_pos_ratio = if corr_total == 0 {
            0.0
        } else {
            corr_pos_count as f64 / corr_total as f64
        };
        let uncorr_pos_ratio = if uncorr_total == 0 {
            0.0
        } else {
            uncorr_pos_count as f64 / uncorr_total as f64
        };

        let l4_l23_matched_mean = if l4_l23_matched_deltas.is_empty() {
            0.0
        } else {
            l4_l23_matched_deltas.iter().sum::<i64>() as f64 / l4_l23_matched_deltas.len() as f64
        };
        let l4_l23_unmatched_mean = if l4_l23_unmatched_deltas.is_empty() {
            0.0
        } else {
            l4_l23_unmatched_deltas.iter().sum::<i64>() as f64
                / l4_l23_unmatched_deltas.len() as f64
        };
        let l4_l23_bias_mass = l4_l23_matched_mean - l4_l23_unmatched_mean;

        let l4_l5_matched_mean = if l4_l5_matched_deltas.is_empty() {
            0.0
        } else {
            l4_l5_matched_deltas.iter().sum::<i64>() as f64 / l4_l5_matched_deltas.len() as f64
        };
        let l4_l5_unmatched_mean = if l4_l5_unmatched_deltas.is_empty() {
            0.0
        } else {
            l4_l5_unmatched_deltas.iter().sum::<i64>() as f64 / l4_l5_unmatched_deltas.len() as f64
        };
        let l4_l5_bias_mass = l4_l5_matched_mean - l4_l5_unmatched_mean;

        // Gates check
        let phys_ok =
            r4 >= 3.0 && r4 <= 25.0 && r23 >= 3.0 && r23 <= 35.0 && r5 >= 1.0 && r5 <= 15.0;
        let mean_corr_pos = mean_corr_mass > 0.0;
        let has_virtual_control = corr_total > 0 && uncorr_total > 0;
        let ratio_ok = has_virtual_control && corr_pos_ratio > uncorr_pos_ratio;
        let passed_all_gates = phys_ok && mean_corr_pos && ratio_ok && !silence && !runaway;

        println!(
            "  L4: {:.2} Hz, L23: {:.2} Hz, L5: {:.2} Hz | Mean Corr Delta Mass: {:.1} (exact: {:.4}), Mean Uncorr Mass: {:.1} (exact: {:.4})",
            r4, r23, r5, mean_corr_mass, mean_corr_exact, mean_uncorr_mass, mean_uncorr_exact
        );
        println!(
            "  L4->L23 Matched Bias Mass: {:.1}, L4->L5 Matched Bias Mass: {:.1} | Passed All Gates: {}",
            l4_l23_bias_mass, l4_l5_bias_mass, passed_all_gates
        );

        sweep_results.push(serde_json::json!({
            "idx": idx,
            "fatigue_cap": fatigue_cap,
            "gsop_pot": gsop_pot,
            "gsop_dep": gsop_dep,
            "virt_w": virt_w,
            "inh": inh,
            "r4": r4,
            "r23": r23,
            "r5": r5,
            "silence": silence,
            "runaway": runaway,
            "mean_corr_mass": mean_corr_mass,
            "mean_uncorr_mass": mean_uncorr_mass,
            "mean_corr_exact": mean_corr_exact,
            "mean_uncorr_exact": mean_uncorr_exact,
            "mean_corr_visible": mean_corr_visible,
            "mean_uncorr_visible": mean_uncorr_visible,
            "corr_pos_ratio": corr_pos_ratio,
            "uncorr_pos_ratio": uncorr_pos_ratio,
            "corr_total": corr_total,
            "uncorr_total": uncorr_total,
            "has_virtual_control": has_virtual_control,
            "l4_l23_bias_mass": l4_l23_bias_mass,
            "l4_l5_bias_mass": l4_l5_bias_mass,
            "passed_all_gates": passed_all_gates,
        }));

        // Selection score: penalize failed physiology heavily, then maximize mean_corr_mass
        let mut score = mean_corr_mass;
        if !phys_ok {
            score -= 100000.0;
        }
        if silence {
            score -= 50000.0;
        }
        if runaway {
            score -= 50000.0;
        }
        // Strongly prefer positive potentiation
        if mean_corr_pos {
            score += 10000.0;
        }

        if score > best_score {
            best_score = score;
            best_params = (fatigue_cap, gsop_pot, gsop_dep, virt_w, inh);
        }
    }

    let sweep_summary_path = artifacts_dir.join("plastic_microcircuit_v1_2_sweep_summary.json");
    let file = File::create(&sweep_summary_path).unwrap();
    serde_json::to_writer_pretty(file, &sweep_results).unwrap();

    let (f_best, p_best, d_best, v_w_best, inh_best) = best_params;
    println!(
        "Winner Parameters: fatigue_cap={}, gsop_pot={}, gsop_dep={}, virt_w={}, inh={}",
        f_best, p_best, d_best, v_w_best, inh_best
    );

    // Phase 2: Winner N=256 Learning Run (100,000 ticks)
    println!("--- Phase 2: Running Winner N=256 Learning Run (100,000 ticks) ---");
    let learning_ticks_winner = 100000;
    let (sim_log_winner, edge_log_winner, r4_w, r23_w, r5_w, _, _) = run_simulation(
        256,
        learning_ticks_winner,
        structured_p,
        background_p,
        block_size,
        f_best,
        p_best,
        d_best,
        v_w_best,
        inh_best,
    );

    let winner_sim_path =
        artifacts_dir.join("plastic_microcircuit_v1_2_best_log_256_learning.json");
    let file = File::create(&winner_sim_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_winner).unwrap();

    let winner_edge_path = artifacts_dir.join("plastic_microcircuit_v1_2_best_edge_log_256.json");
    let file = File::create(&winner_edge_path).unwrap();
    serde_json::to_writer_pretty(file, &edge_log_winner).unwrap();

    let mut dale_violations = 0;
    let mut sign_flips = 0;
    let mut corr_pos_count = 0;
    let mut corr_total = 0;
    let mut uncorr_pos_count = 0;
    let mut uncorr_total = 0;
    let mut sum_abs_delta_mass = 0.0;
    let mut corr_deltas_mass = Vec::new();
    let mut uncorr_deltas_mass = Vec::new();
    let mut corr_deltas_exact = Vec::new();
    let mut uncorr_deltas_exact = Vec::new();
    let mut corr_deltas_visible = Vec::new();
    let mut uncorr_deltas_visible = Vec::new();

    for edge in &edge_log_winner {
        let init_mass = edge["initial_mass"].as_i64().unwrap();
        let final_mass = edge["final_mass"].as_i64().unwrap();
        let is_inh = edge["is_inhibitory"].as_bool().unwrap();
        let d_mass = edge["delta_mass"].as_i64().unwrap();
        let d_exact = edge["delta_charge_exact"].as_f64().unwrap();
        let d_visible = edge["delta_charge_visible"].as_i64().unwrap();
        let proj = edge["projection"].as_str().unwrap();
        let is_matched = edge["is_matched"].as_bool().unwrap();

        sum_abs_delta_mass += d_mass.abs() as f64;

        if is_inh {
            if final_mass > 0 {
                dale_violations += 1;
            }
            if init_mass < 0 && final_mass > 0 {
                sign_flips += 1;
            }
        } else {
            if final_mass < 0 {
                dale_violations += 1;
            }
            if init_mass > 0 && final_mass < 0 {
                sign_flips += 1;
            }
        }

        if proj == "Virtual -> L4" {
            if is_matched {
                corr_deltas_mass.push(d_mass);
                corr_deltas_exact.push(d_exact);
                corr_deltas_visible.push(d_visible);
                corr_total += 1;
                if d_mass > 0 {
                    corr_pos_count += 1;
                }
            } else {
                uncorr_deltas_mass.push(d_mass);
                uncorr_deltas_exact.push(d_exact);
                uncorr_deltas_visible.push(d_visible);
                uncorr_total += 1;
                if d_mass > 0 {
                    uncorr_pos_count += 1;
                }
            }
        }
    }

    let mean_corr_mass = if corr_deltas_mass.is_empty() {
        0.0
    } else {
        corr_deltas_mass.iter().sum::<i64>() as f64 / corr_deltas_mass.len() as f64
    };
    let mean_uncorr_mass = if uncorr_deltas_mass.is_empty() {
        0.0
    } else {
        uncorr_deltas_mass.iter().sum::<i64>() as f64 / uncorr_deltas_mass.len() as f64
    };

    let mean_corr_exact = if corr_deltas_exact.is_empty() {
        0.0
    } else {
        corr_deltas_exact.iter().sum::<f64>() / corr_deltas_exact.len() as f64
    };
    let mean_uncorr_exact = if uncorr_deltas_exact.is_empty() {
        0.0
    } else {
        uncorr_deltas_exact.iter().sum::<f64>() / uncorr_deltas_exact.len() as f64
    };

    let mean_corr_visible = if corr_deltas_visible.is_empty() {
        0.0
    } else {
        corr_deltas_visible.iter().sum::<i64>() as f64 / corr_deltas_visible.len() as f64
    };
    let mean_uncorr_visible = if uncorr_deltas_visible.is_empty() {
        0.0
    } else {
        uncorr_deltas_visible.iter().sum::<i64>() as f64 / uncorr_deltas_visible.len() as f64
    };

    let corr_pos_ratio = if corr_total == 0 {
        0.0
    } else {
        corr_pos_count as f64 / corr_total as f64
    };
    let uncorr_pos_ratio = if uncorr_total == 0 {
        0.0
    } else {
        uncorr_pos_count as f64 / uncorr_total as f64
    };
    let mean_abs_mass = sum_abs_delta_mass / edge_log_winner.len() as f64;

    println!("Winner N=256 Learning Statistics:");
    println!(
        "  Dale Violations: {}, Sign Flips: {}",
        dale_violations, sign_flips
    );
    println!("  Mean Abs Weight Delta (Mass): {:.2}", mean_abs_mass);
    println!(
        "  Mean Matched/Correlated Virtual->L4 Delta Mass: {:.2} (exact: {:.4}, visible: {:.2}, pos ratio: {:.3})",
        mean_corr_mass, mean_corr_exact, mean_corr_visible, corr_pos_ratio
    );
    println!(
        "  Mean Unmatched/Uncorrelated Virtual->L4 Delta Mass: {:.2} (exact: {:.4}, visible: {:.2}, pos ratio: {:.3})",
        mean_uncorr_mass, mean_uncorr_exact, mean_uncorr_visible, uncorr_pos_ratio
    );

    // Run short N=256 sanity run of 9,000 ticks with best candidate parameters
    println!("--- Phase 2.1: Running Winner N=256 Sanity Run (9,000 ticks) ---");
    let (sim_log_w_sanity, _, _, _, _, _, _) = run_simulation(
        256,
        9000,
        structured_p,
        background_p,
        block_size,
        f_best,
        p_best,
        d_best,
        v_w_best,
        inh_best,
    );
    let winner_sanity_path =
        artifacts_dir.join("plastic_microcircuit_v1_2_best_log_256_sanity.json");
    let file = File::create(&winner_sanity_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_w_sanity).unwrap();

    // Phase 3: N=512 Sanity Run (9,000 ticks)
    println!("--- Phase 3: Running Winner N=512 Sanity Run (9,000 ticks) ---");
    let (sim_log_512_sanity, edge_log_512_sanity, r4_512, r23_512, r5_512, _, _) = run_simulation(
        512,
        9000,
        structured_p,
        background_p,
        block_size,
        f_best,
        p_best,
        d_best,
        v_w_best,
        inh_best,
    );
    let file_path = artifacts_dir.join("plastic_microcircuit_v1_2_best_log_512_sanity.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer_pretty(file, &sim_log_512_sanity).unwrap();

    let mut dale_violations_512 = 0;
    let mut sign_flips_512 = 0;
    for edge in &edge_log_512_sanity {
        let final_mass = edge["final_mass"].as_i64().unwrap();
        let init_mass = edge["initial_mass"].as_i64().unwrap();
        let is_inh = edge["is_inhibitory"].as_bool().unwrap();
        if is_inh {
            if final_mass > 0 {
                dale_violations_512 += 1;
            }
            if init_mass < 0 && final_mass > 0 {
                sign_flips_512 += 1;
            }
        } else {
            if final_mass < 0 {
                dale_violations_512 += 1;
            }
            if init_mass > 0 && final_mass < 0 {
                sign_flips_512 += 1;
            }
        }
    }

    // Write research summary to summary JSON
    let summary = serde_json::json!({
        "winner_params": {
            "fatigue_cap": f_best,
            "gsop_pot": p_best,
            "gsop_dep": d_best,
            "virt_w": v_w_best,
            "inh": inh_best,
        },
        "learning_256": {
            "r4": r4_w,
            "r23": r23_w,
            "r5": r5_w,
            "mean_abs_delta_mass": mean_abs_mass,
            "dale_violations": dale_violations,
            "sign_flips": sign_flips,
            "mean_corr_delta_mass": mean_corr_mass,
            "mean_uncorr_delta_mass": mean_uncorr_mass,
            "mean_corr_delta_exact": mean_corr_exact,
            "mean_uncorr_delta_exact": mean_uncorr_exact,
            "mean_corr_delta_visible": mean_corr_visible,
            "mean_uncorr_delta_visible": mean_uncorr_visible,
            "corr_pos_ratio": corr_pos_ratio,
            "uncorr_pos_ratio": uncorr_pos_ratio,
            "corr_total": corr_total,
            "uncorr_total": uncorr_total,
            "has_virtual_control": corr_total > 0 && uncorr_total > 0,
        },
        "sanity_512": {
            "r4": r4_512,
            "r23": r23_512,
            "r5": r5_512,
            "dale_violations": dale_violations_512,
            "sign_flips": sign_flips_512,
        }
    });

    let summary_path = artifacts_dir.join("plastic_microcircuit_v1_2_summary.json");
    let file = File::create(&summary_path).unwrap();
    serde_json::to_writer_pretty(file, &summary).unwrap();

    println!("Plastic Microcircuit v1.2 Rust simulations complete.");
}
