#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use compute_api::{ComputeBackend, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut, ShardUpload};
use compute_cpu::{CpuBackend, CpuBackendConfig};
use layout::{VariantParameters, VARIANT_LUT_LEN};
use test_harness::{MvpAxonBuffer, MvpStateBuffer};
use types::{PackedTarget, SomaFlags};

fn find_profile_path(name: &str) -> PathBuf {
    let paths = [
        format!("../Axicor_Neuron-Lib/modernized/{}.toml", name),
        format!("../../Axicor_Neuron-Lib/modernized/{}.toml", name),
        format!("Axicor_Neuron-Lib/modernized/{}.toml", name),
        format!(
            "/home/alex/AI_Home/workflow/Axicor_Neuron-Lib/modernized/{}.toml",
            name
        ),
    ];
    for p in &paths {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return pb;
        }
    }
    panic!("Could not find modernized profile for {}!", name);
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

fn calculate_isi_metrics(spike_ticks: &[u32]) -> (f64, f64, f64) {
    if spike_ticks.len() < 2 {
        return (0.0, 0.0, 0.0);
    }
    let mut isis = Vec::new();
    for i in 0..spike_ticks.len() - 1 {
        let diff = spike_ticks[i + 1] - spike_ticks[i];
        isis.push(diff as f64);
    }

    let sum: f64 = isis.iter().sum();
    let mean = sum / isis.len() as f64;

    let sq_diff_sum: f64 = isis.iter().map(|&x| (x - mean).powi(2)).sum();
    let var = sq_diff_sum / isis.len() as f64;
    let std_dev = var.sqrt();

    let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };

    let mut lv_sum = 0.0;
    for i in 0..isis.len() - 1 {
        let isi_curr = isis[i];
        let isi_next = isis[i + 1];
        let sum_curr_next = isi_curr + isi_next;
        if sum_curr_next > 0.0 {
            let term = (isi_curr - isi_next) / sum_curr_next;
            lv_sum += term.powi(2);
        }
    }
    let lv = if isis.len() > 1 {
        (3.0 / (isis.len() - 1) as f64) * lv_sum
    } else {
        0.0
    };

    (mean, cv, lv)
}

#[derive(Clone)]
struct ActiveStaCapture {
    spike_tick: u64,
    voltages: Vec<i32>,
}

#[test]
#[allow(clippy::needless_range_loop)]
fn run_biology_metrics_verification() {
    println!("=== Biological Physics Verification Integration Test ===");

    // 1. Load profiles
    let path_visl4 = find_profile_path("L4_spiny_VISl4_4");
    let path_visp5 = find_profile_path("L5_spiny_VISp5_7");
    let path_visp23 = find_profile_path("L23_aspiny_VISp23_218");

    let var_visl4 = load_variant(path_visl4);
    let var_visp5 = load_variant(path_visp5);
    let var_visp23 = load_variant(path_visp23);

    println!("Loaded VISl4: {:?}", var_visl4);
    println!("Loaded VISp5: {:?}", var_visp5);
    println!("Loaded VISp23: {:?}", var_visp23);

    // 2. Setup variants table
    let mut variant_table = [bytemuck::Zeroable::zeroed(); VARIANT_LUT_LEN];
    variant_table[0] = var_visl4;
    variant_table[1] = var_visp5;
    variant_table[2] = var_visp23;

    // Simulation parameters
    let total_ticks = 1_000_000;
    let padded_n = 320;
    let total_axons = 320;

    // ----------------------------------------------------
    // TEST A: Heartbeat-only (pacemaker)
    // ----------------------------------------------------
    println!("Running Test A (Heartbeat-only) for 1,000,000 ticks...");

    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let axons_buf = MvpAxonBuffer::new(total_axons);

    // Initialize soma types, rest potentials, and mapping
    for i in 0..padded_n {
        let type_id = if i < 100 {
            0
        } else if i < 200 {
            1
        } else if i < 300 {
            2
        } else {
            0
        };
        let var = &variant_table[type_id];

        state_buf.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
        state_buf.write_soma_voltage(i, var.rest_potential);
        state_buf.write_soma_to_axon(i, i as u32);
    }

    // Set up CPU Backend
    let config = CpuBackendConfig {
        thread_count: Some(1),
    };
    let mut backend = CpuBackend::new(config).unwrap();
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

    // Setup outputs buffer
    let mut out_spikes = vec![0u32; padded_n];
    let mut out_counts = vec![0u32; 1];
    let mapped_somas: Vec<u32> = (0..300).collect();

    // Stats collections
    let mut spike_times_a = vec![Vec::new(); 300];
    let mut rolling_history_a = vec![VecDeque::new(); 3];
    let mut completed_sta_a = vec![Vec::new(); 3];
    let mut active_captures_a: Vec<Vec<ActiveStaCapture>> = vec![Vec::new(); 3];
    let rep_indices = [0, 100, 200];

    // Snapshot buffer
    let mut snap_state = vec![0u8; state_buf.as_bytes().len()];
    let mut snap_axons = vec![0u8; axons_buf.as_bytes().len()];

    for tick in 0..total_ticks {
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
            incoming_spikes: None,
            incoming_spike_counts: &[0],
            max_spikes_per_tick: padded_n as u32,
            num_outputs: 300,
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

        let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

        // Process spikes for this tick
        let count = out_counts[0] as usize;
        for s_idx in 0..count {
            let soma_id = out_spikes[s_idx] as usize;
            if soma_id < 300 {
                spike_times_a[soma_id].push(tick as u32);
            }
        }

        // Process STA tracking for representative neurons
        for (idx, &rep_id) in rep_indices.iter().enumerate() {
            let v = snap_state_buf.read_soma_voltage(rep_id);
            let flags = SomaFlags(snap_state_buf.read_soma_flags(rep_id));

            // Check if neuron spiked in this tick
            if flags.spiking() && rolling_history_a[idx].len() == 50 {
                let mut captured_v = rolling_history_a[idx].iter().copied().collect::<Vec<_>>();
                captured_v.push(v); // add current spike tick voltage (index 50)
                active_captures_a[idx].push(ActiveStaCapture {
                    spike_tick: tick as u64,
                    voltages: captured_v,
                });
            }

            // Push to rolling history
            rolling_history_a[idx].push_back(v);
            if rolling_history_a[idx].len() > 50 {
                rolling_history_a[idx].pop_front();
            }

            // Update active captures
            let mut still_active = Vec::new();
            for mut cap in active_captures_a[idx].clone() {
                if cap.spike_tick != tick as u64 {
                    // do not append on the tick we created it
                    cap.voltages.push(v);
                }
                if cap.voltages.len() == 151 {
                    completed_sta_a[idx].push(cap.voltages);
                } else {
                    still_active.push(cap);
                }
            }
            active_captures_a[idx] = still_active;
        }
    }

    backend.free_shard(handle).unwrap();

    // Calculate Test A metrics
    println!("\nTest A (Heartbeat-only) Results:");
    println!("Type | Firing Rate (Hz) | Mean ISI (ticks) | CV | LV | STA Spikes Count");
    for (idx, name) in ["VISl4", "VISp5", "VISp23"].iter().enumerate() {
        let start = idx * 100;
        let end = start + 100;
        let mut all_rates = Vec::new();
        let mut all_cvs = Vec::new();
        let mut all_lvs = Vec::new();

        for i in start..end {
            let spikes = &spike_times_a[i];
            let rate = spikes.len() as f64 / (total_ticks as f64 / 1000.0);
            all_rates.push(rate);

            let (_, cv, lv) = calculate_isi_metrics(spikes);
            all_cvs.push(cv);
            all_lvs.push(lv);
        }

        let avg_rate = all_rates.iter().sum::<f64>() / 100.0;
        let avg_cv = all_cvs.iter().sum::<f64>() / 100.0;
        let avg_lv = all_lvs.iter().sum::<f64>() / 100.0;

        let sta_count = completed_sta_a[idx].len();
        println!(
            "{} | {:.4} | {:.1} | {:.4} | {:.4} | {}",
            name,
            avg_rate,
            if avg_rate > 0.0 {
                1000.0 / avg_rate
            } else {
                0.0
            },
            avg_cv,
            avg_lv,
            sta_count
        );
    }

    // ----------------------------------------------------
    // TEST B: Synaptic-driven (Poisson noise bombardment)
    // ----------------------------------------------------
    println!("\nRunning Test B (Synaptic-driven) for 1,000,000 ticks...");

    // Set heartbeat_m = 0 to disable spontaneous pacemaker
    let mut variant_table_b = variant_table;
    for var in variant_table_b.iter_mut() {
        var.heartbeat_m = 0;
        var.spontaneous_firing_period_ticks = 0;
    }

    let mut state_buf_b = MvpStateBuffer::new(padded_n, total_axons);
    let axons_buf_b = MvpAxonBuffer::new(total_axons);

    // Configure somas, and connect slots 0..8 to EPSPs, 8..10 to IPSPs
    for i in 0..padded_n {
        let type_id = if i < 100 {
            0
        } else if i < 200 {
            1
        } else if i < 300 {
            2
        } else {
            0
        };
        let var = &variant_table_b[type_id];

        state_buf_b.write_soma_flags(i, SomaFlags::new(false, 0, type_id as u8).0);
        state_buf_b.write_soma_voltage(i, var.rest_potential);
        state_buf_b.write_soma_to_axon(i, i as u32);

        if i < 300 {
            // Slots 0..8 -> excitatory targets (input axons 300..308), weight = 1000 << 16
            for slot in 0..8 {
                let target = PackedTarget::pack((300 + slot) as u32, 0).0;
                state_buf_b.write_dendrite_target(slot, i, target);
                state_buf_b.write_dendrite_weight(slot, i, 1000 << 16);
            }
            // Slots 8..10 -> inhibitory targets (input axons 308..310), weight = -1000 << 16
            for slot in 8..10 {
                let target = PackedTarget::pack((300 + slot) as u32, 0).0;
                state_buf_b.write_dendrite_target(slot, i, target);
                state_buf_b.write_dendrite_weight(slot, i, -1000 << 16);
            }
        }
    }

    let handle_b = backend.alloc_shard(spec).unwrap();
    backend
        .upload_shard(
            handle_b,
            ShardUpload {
                state_blob: state_buf_b.as_bytes(),
                axons_blob: axons_buf_b.as_bytes(),
                variant_table: &variant_table_b,
            },
        )
        .unwrap();

    let mut rng = SimpleRng::new(42);
    let mut spike_times_b = vec![Vec::new(); 300];
    let mut rolling_history_b = vec![VecDeque::new(); 3];
    let mut completed_sta_b = vec![Vec::new(); 3];
    let mut active_captures_b: Vec<Vec<ActiveStaCapture>> = vec![Vec::new(); 3];

    // Synaptic Fatigue metrics tracking
    let mut fatigue_sums = [0.0f64; 3];
    let mut fatigue_counts = [0u64; 3];

    let mut incoming_padded = vec![0u32; padded_n];

    for tick in 0..total_ticks {
        // Generate Poisson spikes for the 10 input axons (300..310)
        // Rate = 50 Hz, meaning P = 0.05 per tick
        let mut incoming_count = 0;
        for axon_idx in 300..310 {
            if rng.next_f32() < 0.05 {
                incoming_padded[incoming_count] = axon_idx as u32;
                incoming_count += 1;
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
            incoming_spikes: Some(&incoming_padded),
            incoming_spike_counts: &[incoming_count as u32],
            max_spikes_per_tick: padded_n as u32,
            num_outputs: 300,
            mapped_soma_ids: &mapped_somas,
            output_spikes: &mut out_spikes,
            output_spike_counts: &mut out_counts,
        };

        backend.run_day_batch(handle_b, cmd).unwrap();

        // Read snapshot
        backend
            .debug_snapshot(
                handle_b,
                ShardSnapshotMut {
                    state_blob: &mut snap_state,
                    axons_blob: &mut snap_axons,
                },
            )
            .unwrap();

        let snap_state_buf = MvpStateBuffer::from_raw(padded_n, total_axons, snap_state.clone());

        // Process spikes for this tick
        let count = out_counts[0] as usize;
        for s_idx in 0..count {
            let soma_id = out_spikes[s_idx] as usize;
            if soma_id < 300 {
                spike_times_b[soma_id].push(tick as u32);

                // Collect fatigue metrics on local spike times
                let type_id = if soma_id < 100 {
                    0
                } else if soma_id < 200 {
                    1
                } else {
                    2
                };
                let var = &variant_table_b[type_id];

                let mut soma_fatigue_sum = 0.0;
                for slot in 0..10 {
                    let fatigue_val = snap_state_buf.read_dendrite_timer(slot, soma_id);
                    soma_fatigue_sum += fatigue_val as f64 / var.fatigue_capacity as f64;
                }
                fatigue_sums[type_id] += soma_fatigue_sum / 10.0;
                fatigue_counts[type_id] += 1;
            }
        }

        // Process STA tracking for representative neurons
        for (idx, &rep_id) in rep_indices.iter().enumerate() {
            let v = snap_state_buf.read_soma_voltage(rep_id);
            let flags = SomaFlags(snap_state_buf.read_soma_flags(rep_id));

            // Check if neuron spiked in this tick
            if flags.spiking() && rolling_history_b[idx].len() == 50 {
                let mut captured_v = rolling_history_b[idx].iter().copied().collect::<Vec<_>>();
                captured_v.push(v);
                active_captures_b[idx].push(ActiveStaCapture {
                    spike_tick: tick as u64,
                    voltages: captured_v,
                });
            }

            // Push to rolling history
            rolling_history_b[idx].push_back(v);
            if rolling_history_b[idx].len() > 50 {
                rolling_history_b[idx].pop_front();
            }

            // Update active captures
            let mut still_active = Vec::new();
            for mut cap in active_captures_b[idx].clone() {
                if cap.spike_tick != tick as u64 {
                    cap.voltages.push(v);
                }
                if cap.voltages.len() == 151 {
                    completed_sta_b[idx].push(cap.voltages);
                } else {
                    still_active.push(cap);
                }
            }
            active_captures_b[idx] = still_active;
        }
    }

    backend.free_shard(handle_b).unwrap();

    // Calculate Test B metrics
    println!("\nTest B (Synaptic-driven) Results:");
    println!("Type | Firing Rate (Hz) | Mean ISI (ticks) | CV | LV | Steady-State Fatigue Ratio | STA Spikes Count");
    for (idx, name) in ["VISl4", "VISp5", "VISp23"].iter().enumerate() {
        let start = idx * 100;
        let end = start + 100;
        let mut all_rates = Vec::new();
        let mut all_cvs = Vec::new();
        let mut all_lvs = Vec::new();

        for i in start..end {
            let spikes = &spike_times_b[i];
            let rate = spikes.len() as f64 / (total_ticks as f64 / 1000.0);
            all_rates.push(rate);

            let (_, cv, lv) = calculate_isi_metrics(spikes);
            all_cvs.push(cv);
            all_lvs.push(lv);
        }

        let avg_rate = all_rates.iter().sum::<f64>() / 100.0;
        let avg_cv = all_cvs.iter().sum::<f64>() / 100.0;
        let avg_lv = all_lvs.iter().sum::<f64>() / 100.0;

        let avg_fatigue = if fatigue_counts[idx] > 0 {
            fatigue_sums[idx] / fatigue_counts[idx] as f64
        } else {
            0.0
        };

        let sta_count = completed_sta_b[idx].len();
        println!(
            "{} | {:.4} | {:.1} | {:.4} | {:.4} | {:.4} | {}",
            name,
            avg_rate,
            if avg_rate > 0.0 {
                1000.0 / avg_rate
            } else {
                0.0
            },
            avg_cv,
            avg_lv,
            avg_fatigue,
            sta_count
        );
    }

    // Print STA Profiles to see pre/post spike values
    println!("\n=== Sample STA Voltage Traces (T-50 to T+100) ===");
    for (idx, name) in ["VISl4", "VISp5", "VISp23"].iter().enumerate() {
        println!("\nSTA Profile for {}:", name);
        if completed_sta_b[idx].is_empty() {
            println!("No completed STA windows recorded.");
            continue;
        }

        // Compute average profile across all spikes
        let mut avg_profile = vec![0.0f64; 151];
        let n_sta = completed_sta_b[idx].len();
        for profile in &completed_sta_b[idx] {
            for t in 0..151 {
                avg_profile[t] += profile[t] as f64;
            }
        }
        for t in 0..151 {
            avg_profile[t] /= n_sta as f64;
        }

        // Print key offsets
        println!("  T-50 (50 ticks before): {:.1}", avg_profile[0]);
        println!("  T-25 (25 ticks before): {:.1}", avg_profile[25]);
        println!("  T-1  (1 tick before):   {:.1}", avg_profile[49]);
        println!(
            "  T0   (Spike tick):      {:.1} (Expected reset potential)",
            avg_profile[50]
        );
        println!("  T+1  (1 tick after):    {:.1}", avg_profile[51]);
        println!("  T+25 (25 ticks after):  {:.1}", avg_profile[75]);
        println!("  T+50 (50 ticks after):  {:.1}", avg_profile[100]);
        println!("  T+100 (100 ticks after): {:.1}", avg_profile[150]);
    }
}
