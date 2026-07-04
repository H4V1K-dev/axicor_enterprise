#![allow(clippy::needless_range_loop)]

/*
README / DIAGNOSTIC NOTICE:
This file is a developer-only diagnostic probe for visually and numerically inspecting the behavior of the `physics` crate.
It is NOT a production component, NOT a benchmark reference, and NOT an architecture ground truth.
It executes pure simulation dynamics solely via the public APIs of `crates/physics` and `crates/types`.
*/

use physics::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use types::AXON_SENTINEL;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting AxiEngine Physics Diagnostic Probe...");

    let num_neurons = 32;
    let total_ticks = 100;

    // Physical constants and derived AOT parameters
    let v_seg = compute_v_seg(1.0, 1000, 20.0, 5).expect("valid v_seg derivation"); // 10 segments/tick
    let heartbeat_m = compile_stochastic_heartbeat_threshold(500); // Stochastic heartbeat period 500
    let v_th = -50;
    let rest_potential = -70;
    let v_reset = -75;
    let leak_shift = 4;
    let homeostasis_penalty = 10;
    let homeostasis_decay_amount = 1;
    let prop_len = 10;

    // Network state arrays
    let mut voltage = vec![rest_potential; num_neurons];
    let mut thresh_offset = vec![0i32; num_neurons];
    let mut heads = vec![[AXON_SENTINEL; 8]; num_neurons];

    // Synaptic weight matrix [presynaptic][postsynaptic] in Mass Domain (i32)
    let mut weights = vec![vec![0i32; num_neurons]; num_neurons];
    let inertia_curve = [128, 128, 128, 128, 128, 128, 128, 128];

    // Initialize toy ring topology with excitatory (+12,000,000) and inhibitory (-10,000,000) connections
    for i in 0..num_neurons {
        let next1 = (i + 1) % num_neurons;
        let next2 = (i + 2) % num_neurons;
        weights[i][next1] = 12_000_000; // Excitatory
        weights[i][next2] = -10_000_000; // Inhibitory
    }

    // Create output CSV files
    std::fs::create_dir_all("artifacts")?;
    let ticks_file = File::create("artifacts/physics_probe_ticks.csv")?;
    let mut ticks_writer = BufWriter::new(ticks_file);
    writeln!(
        ticks_writer,
        "tick,neuron_id,voltage,spike_glif,spike_heartbeat,final_spike,thresh_offset"
    )?;

    let weights_file = File::create("artifacts/physics_probe_weights.csv")?;
    let mut weights_writer = BufWriter::new(weights_file);
    writeln!(
        weights_writer,
        "# Synaptic Weight Matrix Snapshots (Mass Domain)"
    )?;

    println!(
        "Simulating {} neurons over {} ticks...",
        num_neurons, total_ticks
    );

    let mut total_glif_spikes = 0;
    let mut total_hb_spikes = 0;

    for tick in 0..total_ticks {
        // Record weight matrix snapshot at ticks 0, 50, 99
        if tick == 0 || tick == 50 || tick == 99 {
            writeln!(weights_writer, "Snapshot at Tick {}", tick)?;
            for i in 0..num_neurons {
                let row_str: Vec<String> = weights[i].iter().map(|w| w.to_string()).collect();
                writeln!(weights_writer, "N{:02}: {}", i, row_str.join(","))?;
            }
            writeln!(weights_writer)?;
        }

        let mut final_spikes = vec![false; num_neurons];

        // Phase 1: Integration & Spike Detection
        for i in 0..num_neurons {
            let mut i_in = 0i32;

            // External stimulation pulse on input neurons 0..4 every 10 ticks
            if i < 4 && tick % 10 == 5 {
                i_in += 500; // Charge pulse
            }

            // Synaptic input integration
            for j in 0..num_neurons {
                let w = weights[j][i];
                if w != 0 {
                    // Toy reading segment index 0
                    if active_tail_hit(&heads[j], 0, prop_len) {
                        i_in += weight_to_charge(w);
                    }
                }
            }

            // Membrane voltage integration
            voltage[i] = update_glif_voltage(
                voltage[i],
                i_in,
                rest_potential,
                thresh_offset[i],
                leak_shift,
                0,
                1,
                0,
            );

            // Spike detection predicates
            let is_glif = is_glif_spike(voltage[i], v_th, thresh_offset[i]);
            let is_hb = heartbeat_spike(tick, heartbeat_m, i as u32);
            let final_spike = is_glif || is_hb;
            final_spikes[i] = final_spike;

            if is_glif {
                total_glif_spikes += 1;
            }
            if is_hb {
                total_hb_spikes += 1;
            }

            // Write tick log record
            writeln!(
                ticks_writer,
                "{},{},{},{},{},{},{}",
                tick,
                i,
                voltage[i],
                is_glif as u8,
                is_hb as u8,
                final_spike as u8,
                thresh_offset[i]
            )?;
        }

        // Phase 2: Post-Spike Side Effects & Propagation Update
        for i in 0..num_neurons {
            let final_spike = final_spikes[i];
            let is_glif = is_glif_spike(voltage[i], v_th, thresh_offset[i]);

            if final_spike {
                // Shift burst heads and push newborn head at index 0
                let new_head = initial_axon_head(v_seg);
                for k in (1..8).rev() {
                    heads[i][k] = heads[i][k - 1];
                }
                heads[i][0] = new_head;
            }

            if is_glif {
                voltage[i] = v_reset;
                thresh_offset[i] = thresh_offset[i].wrapping_add(homeostasis_penalty);
            } else {
                thresh_offset[i] = homeostasis_decay(thresh_offset[i], homeostasis_decay_amount);
            }

            // Advance axonal heads
            for k in 0..8 {
                heads[i][k] = propagate_head(heads[i][k], v_seg);
            }
        }

        // Phase 3: Synaptic Plasticity (GSOP)
        for j in 0..num_neurons {
            if final_spikes[j] {
                for i in 0..num_neurons {
                    let w = weights[j][i];
                    if w != 0 {
                        weights[j][i] = apply_gsop_plasticity(
                            w,
                            &heads[j],
                            0,
                            prop_len,
                            0,
                            255,
                            100_000, // pot
                            50_000,  // dep
                            0,       // dopamine
                            0,       // d1
                            0,       // d2
                            1,       // burst_count
                            &inertia_curve,
                        );
                    }
                }
            }
        }
    }

    ticks_writer.flush()?;
    weights_writer.flush()?;

    println!("Diagnostic Probe Completed Successfully!");
    println!(
        "Total GLIF Spikes: {}, Total Heartbeat Spikes: {}",
        total_glif_spikes, total_hb_spikes
    );
    println!("Artifacts written to:");
    println!("  - artifacts/physics_probe_ticks.csv");
    println!("  - artifacts/physics_probe_weights.csv");

    Ok(())
}
