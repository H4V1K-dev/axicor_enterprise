#![cfg(feature = "single-neuron-calibration")]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use serde::Deserialize;

// Import GLIF physics from crates/physics
use physics::{update_glif_voltage, is_glif_spike, homeostasis_decay};

#[derive(Deserialize, Debug, Clone)]
struct AggregatedFiPoint {
    stimulus_pa: f64,
    sweep_count: usize,
    spike_count_mean: f64,
    firing_rate_mean: f64,
}

#[derive(Deserialize, Debug, Clone)]
struct CalibrationNeuron {
    specimen_id: u64,
    role_label: String,
    source_status: String,
    rest_vrest_mv: f64,
    first_spike_threshold_mv: f64,
    rheobase_pa: f64,
    aggregated_fi_points: Vec<AggregatedFiPoint>,
}

fn find_json_path() -> PathBuf {
    let paths = [
        "w:/Workspace/artifacts/biological_calibration_pack_v1.json",
        "artifacts/biological_calibration_pack_v1.json",
        "../artifacts/biological_calibration_pack_v1.json",
        "../../artifacts/biological_calibration_pack_v1.json",
    ];
    for p in &paths {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return pb;
        }
    }
    panic!("Could not find biological_calibration_pack_v1.json!");
}

// V2 GLIF simulation returning (spike_count, adaptation_index)
fn simulate_glif_fi_v2(
    rest_potential: i32,
    threshold: i32,
    leak_shift: i32,
    current_scale: f64,
    refractory_period: i32,
    stimulus_pa: f64,
    // V2 Adaptive Parameters
    homeostasis_penalty: i32,
    homeostasis_decay_amount: i32,
    adaptive_mode: i32,
    adaptive_leak_gain: i32,
    ahp_amplitude: i32,
) -> (usize, f64) {
    let total_ticks = 3000;
    let mut voltage = rest_potential;
    let mut thresh_offset = 0i32;
    let mut refractory_timer = 0i32;
    let mut spikes = 0;
    let mut spike_ticks = Vec::new();
    
    let step_current = (stimulus_pa * current_scale) as i32;
    let v_reset = rest_potential - ahp_amplitude;
    
    for t in 0..total_ticks {
        let i_in = if t >= 1000 && t < 2000 {
            step_current
        } else {
            0
        };
        
        if refractory_timer > 0 {
            refractory_timer -= 1;
            voltage = v_reset;
            thresh_offset = homeostasis_decay(thresh_offset, homeostasis_decay_amount);
        } else {
            let v_new = update_glif_voltage(
                voltage,
                i_in,
                rest_potential,
                thresh_offset,
                leak_shift,
                adaptive_leak_gain,
                1, // adaptive leak min shift
                adaptive_mode,
            );
            
            if is_glif_spike(v_new, threshold, thresh_offset) {
                voltage = v_reset; // reset to v_reset
                refractory_timer = refractory_period;
                thresh_offset = thresh_offset.wrapping_add(homeostasis_penalty);
                if t >= 1000 && t < 2000 {
                    spikes += 1;
                    spike_ticks.push(t);
                }
            } else {
                voltage = v_new;
                thresh_offset = homeostasis_decay(thresh_offset, homeostasis_decay_amount);
            }
        }
    }
    
    // Adaptation index proxy: (last_isi - first_isi) / first_isi
    let mut adaptation_val = 0.0;
    if spike_ticks.len() >= 3 {
        let first_isi = (spike_ticks[1] - spike_ticks[0]) as f64;
        let last_isi = (spike_ticks[spike_ticks.len() - 1] - spike_ticks[spike_ticks.len() - 2]) as f64;
        if first_isi > 0.0 {
            adaptation_val = (last_isi - first_isi) / first_isi;
        }
    }
    
    (spikes, adaptation_val)
}

#[test]
#[ignore]
fn run_single_neuron_calibration() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting single neuron V2 adaptive calibration probe...");
    let pack_path = find_json_path();
    let file = File::open(pack_path)?;
    let neurons: Vec<CalibrationNeuron> = serde_json::from_reader(file)?;
    
    let mut grid_records = Vec::new();
    let mut best_records = Vec::new();
    
    // Define best parameters from V1 for each cell
    // (specimen_id -> (leak_shift, current_scale, refractory_period, threshold))
    // Note: threshold is rest_potential + delta_v (so delta_v is mapped)
    // 313861608: PV -> leak=2, scale=0.02, ref=2, thresh = -49 (delta_v = 26)
    // 490376252: Cux2 -> leak=6, scale=0.01, ref=20, thresh = -35 (delta_v = 41)
    // 314900022: Scnn1a -> leak=6, scale=0.02, ref=20, thresh = -37 (delta_v = 36)
    // 324493977: SST -> leak=4, scale=0.02, ref=12, thresh = -28 (delta_v = 44)
    
    for neuron in &neurons {
        if neuron.source_status != "method_ready" {
            continue;
        }
        
        // Find best baseline V1 constants
        let (leak_shift, current_scale, refractory_period, threshold_v) = match neuron.specimen_id {
            313861608 => (2, 0.02, 2, -49),
            490376252 => (6, 0.01, 20, -35),
            314900022 => (6, 0.02, 20, -37),
            324493977 => (4, 0.02, 12, -28),
            _ => continue,
        };
        
        let rest_v = neuron.rest_vrest_mv.round() as i32;
        
        // Target adaptation index proxies
        let target_adaptation = match neuron.specimen_id {
            313861608 => 0.0, // PV has regular fast firing
            324493977 => 0.5, // SST has high adaptation
            490376252 => 0.4, // Cux2 (excitatory) has adaptation
            314900022 => 0.4, // Scnn1a (excitatory) has adaptation
            _ => 0.0,
        };
        
        // Positive stimulus points
        let test_points: Vec<&AggregatedFiPoint> = neuron.aggregated_fi_points
            .iter()
            .filter(|pt| pt.stimulus_pa > 0.0)
            .collect();
            
        let max_pt = test_points.iter().max_by(|a, b| a.stimulus_pa.partial_cmp(&b.stimulus_pa).unwrap()).unwrap();
        let biol_max_spikes = max_pt.spike_count_mean;
        
        // Define V2 parameters grid scan
        let penalties = vec![0, 2, 5, 10, 20];
        let decays = vec![1, 2, 5];
        let adaptive_modes = vec![0, 1];
        let leak_gains = vec![0, 32, 64, 128];
        let ahp_amplitudes = vec![0, 2, 5, 10];
        
        let mut best_score = f64::MAX;
        let mut best_v2_params = (0, 1, 0, 0, 0, "lif_baseline");
        let mut best_rmse = f64::MAX;
        let mut best_rheobase_err = f64::MAX;
        let mut best_sat_err = f64::MAX;
        let mut best_adapt_err = f64::MAX;
        let mut best_pred_rheobase = 1000.0;
        let mut best_sim_adapt = 0.0;
        
        for &penalty in &penalties {
            for &decay in &decays {
                for &ad_mode in &adaptive_modes {
                    for &ad_gain in &leak_gains {
                        for &ahp in &ahp_amplitudes {
                            // Filter logic for modes
                            let mode_name = if penalty == 0 && ad_mode == 0 && ahp == 0 {
                                "lif_baseline"
                            } else if penalty > 0 && ad_mode == 0 && ahp == 0 {
                                "glif_homeostasis"
                            } else if penalty > 0 && ad_mode == 1 && ahp == 0 {
                                "glif_adaptive_leak"
                            } else if penalty == 0 && ad_mode == 0 && ahp > 0 {
                                "glif_ahp_reset"
                            } else if penalty > 0 && ad_mode == 1 && ahp > 0 {
                                "combined_glif"
                            } else {
                                continue; // Skip redundant combinations
                            };
                            
                            let mut sum_sq_err = 0.0;
                            let mut pred_rheobase = None;
                            let mut sim_max_adaptation = 0.0;
                            let mut sim_max_spikes = 0;
                            
                            for pt in &test_points {
                                let (pred, adapt) = simulate_glif_fi_v2(
                                    rest_v,
                                    threshold_v,
                                    leak_shift,
                                    current_scale,
                                    refractory_period,
                                    pt.stimulus_pa,
                                    penalty,
                                    decay,
                                    ad_mode,
                                    ad_gain,
                                    ahp,
                                );
                                
                                if pred > 0 && pred_rheobase.is_none() {
                                    pred_rheobase = Some(pt.stimulus_pa);
                                }
                                
                                if pt.stimulus_pa == max_pt.stimulus_pa {
                                    sim_max_spikes = pred;
                                    sim_max_adaptation = adapt;
                                }
                                
                                let err = (pred as f64) - pt.spike_count_mean;
                                sum_sq_err += err * err;
                            }
                            
                            let rmse = (sum_sq_err / test_points.len() as f64).sqrt();
                            let pred_rheobase_val = pred_rheobase.unwrap_or(1000.0);
                            let rheobase_err = (pred_rheobase_val - neuron.rheobase_pa).abs();
                            let sat_err = ((sim_max_spikes as f64) - biol_max_spikes).abs();
                            let adapt_err = (sim_max_adaptation - target_adaptation).abs();
                            
                            // Combined Score
                            let score = rmse + (rheobase_err / 10.0) + (sat_err / 5.0) + (adapt_err * 15.0);
                            
                            // Record to grid CSV
                            grid_records.push(format!(
                                "{},{},{},{},{},{},{},{},{},{:.4},{:.2},{:.2},{:.4},{:.4},{:.4}",
                                neuron.specimen_id,
                                neuron.role_label,
                                mode_name,
                                penalty,
                                decay,
                                ad_mode,
                                ad_gain,
                                ahp,
                                threshold_v,
                                rmse,
                                pred_rheobase_val,
                                rheobase_err,
                                sat_err,
                                sim_max_adaptation,
                                score
                            ));
                            
                            if score < best_score {
                                best_score = score;
                                best_rmse = rmse;
                                best_rheobase_err = rheobase_err;
                                best_sat_err = sat_err;
                                best_adapt_err = adapt_err;
                                best_v2_params = (penalty, decay, ad_mode, ad_gain, ahp, mode_name);
                                best_pred_rheobase = pred_rheobase_val;
                                best_sim_adapt = sim_max_adaptation;
                            }
                        }
                    }
                }
            }
        }
        
        let (bp, bd, bam, bag, bahp, b_mode) = best_v2_params;
        best_records.push(format!(
            "{},{},{},{},{},{},{},{},{:.4},{:.2},{:.4},{:.4}",
            neuron.specimen_id,
            neuron.role_label,
            b_mode,
            bp,
            bd,
            bam,
            bag,
            bahp,
            best_rmse,
            best_pred_rheobase,
            best_sat_err,
            best_sim_adapt
        ));
        
        println!("  Cell {}: Best Mode: {} (penalty={}, decay={}, ad_mode={}, gain={}, ahp={}) -> RMSE={:.4}, SatErr={:.2}, Adapt={:.4}",
            neuron.specimen_id, b_mode, bp, bd, bam, bag, bahp, best_rmse, best_sat_err, best_sim_adapt);
    }
    
    // Write Grid CSV
    let grid_file = File::create("w:/Workspace/artifacts/single_neuron_calibration_v2_grid.csv")?;
    let mut grid_writer = BufWriter::new(grid_file);
    writeln!(grid_writer, "specimen_id,role_label,mode_name,homeostasis_penalty,homeostasis_decay,adaptive_mode,adaptive_leak_gain,ahp_amplitude,threshold,rmse,pred_rheobase,rheobase_error,saturation_error,sim_adaptation,score")?;
    for row in grid_records {
        writeln!(grid_writer, "{}", row)?;
    }
    grid_writer.flush()?;
    
    // Write Best CSV
    let best_file = File::create("w:/Workspace/artifacts/single_neuron_calibration_v2_best.csv")?;
    let mut best_writer = BufWriter::new(best_file);
    writeln!(best_writer, "specimen_id,role_label,mode_name,homeostasis_penalty,homeostasis_decay,adaptive_mode,adaptive_leak_gain,ahp_amplitude,rmse,pred_rheobase,saturation_error,sim_adaptation")?;
    for row in best_records {
        writeln!(best_writer, "{}", row)?;
    }
    best_writer.flush()?;
    
    println!("V2 Calibration complete. Outputs saved successfully.");
    Ok(())
}
