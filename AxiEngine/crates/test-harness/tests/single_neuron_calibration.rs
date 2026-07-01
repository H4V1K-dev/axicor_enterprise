#![cfg(feature = "single-neuron-calibration")]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use serde::Deserialize;
use serde_json::Value;

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

fn simulate_glif_fi(
    rest_potential: i32,
    threshold: i32,
    leak_shift: i32,
    current_scale: f64,
    refractory_period: i32,
    stimulus_pa: f64,
) -> usize {
    let total_ticks = 3000;
    let mut voltage = rest_potential;
    let mut thresh_offset = 0i32;
    let mut refractory_timer = 0i32;
    let mut spikes = 0;
    
    let step_current = (stimulus_pa * current_scale) as i32;
    
    for t in 0..total_ticks {
        let i_in = if t >= 1000 && t < 2000 {
            step_current
        } else {
            0
        };
        
        if refractory_timer > 0 {
            refractory_timer -= 1;
            voltage = rest_potential;
            thresh_offset = homeostasis_decay(thresh_offset, 1);
        } else {
            let v_new = update_glif_voltage(
                voltage,
                i_in,
                rest_potential,
                thresh_offset,
                leak_shift,
                0, // adaptive leak gain
                1, // adaptive leak min shift
                0, // adaptive mode
            );
            
            if is_glif_spike(v_new, threshold, thresh_offset) {
                voltage = rest_potential; // reset
                refractory_timer = refractory_period;
                thresh_offset += 0; // no homeostasis penalty for simple LIF calibration
                if t >= 1000 && t < 2000 {
                    spikes += 1;
                }
            } else {
                voltage = v_new;
                thresh_offset = homeostasis_decay(thresh_offset, 1);
            }
        }
    }
    spikes
}

#[test]
#[ignore] // Can be run explicitly via cargo test --test single_neuron_calibration --features single-neuron-calibration -- --ignored
fn run_single_neuron_calibration() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting single neuron calibration probe...");
    let pack_path = find_json_path();
    let file = File::open(pack_path)?;
    let neurons: Vec<CalibrationNeuron> = serde_json::from_reader(file)?;
    
    let mut grid_records = Vec::new();
    let mut best_records = Vec::new();
    
    // Scanned grid candidates
    let leak_shifts = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let current_scales = vec![0.002, 0.005, 0.01, 0.015, 0.02, 0.03, 0.05, 0.08, 0.1, 0.2, 0.5];
    let refractory_periods = vec![1, 2, 3, 4, 6, 8, 10, 12, 16, 20];
    
    for neuron in &neurons {
        if neuron.source_status != "method_ready" {
            println!("Skipping {} (status: {})", neuron.specimen_id, neuron.source_status);
            continue;
        }
        
        println!("Processing cell {} (`{}`)...", neuron.specimen_id, neuron.role_label);
        
        let rest_v = neuron.rest_vrest_mv.round() as i32;
        let threshold_v_biol = neuron.first_spike_threshold_mv.round() as i32;
        let delta_v_biol = threshold_v_biol - rest_v;
        
        // Scan thresholds around the biological threshold
        let delta_vs = vec![delta_v_biol - 4, delta_v_biol - 2, delta_v_biol, delta_v_biol + 2, delta_v_biol + 4];
        
        let mut best_score = f64::MAX;
        let mut best_params = (0, 0.0, 0, 0);
        let mut best_rmse = f64::MAX;
        let mut best_rheobase_err = f64::MAX;
        let mut best_pred_fi = Vec::new();
        
        // Positive stimulus points for RMSE calculation
        let test_points: Vec<&AggregatedFiPoint> = neuron.aggregated_fi_points
            .iter()
            .filter(|pt| pt.stimulus_pa > 0.0)
            .collect();
            
        if test_points.is_empty() {
            println!("  Warning: no positive stimulus points found for cell {}", neuron.specimen_id);
            continue;
        }
        
        for &leak_shift in &leak_shifts {
            for &current_scale in &current_scales {
                for &ref_period in &refractory_periods {
                    for &delta_v in &delta_vs {
                        let threshold_v = rest_v + delta_v;
                        
                        let mut sum_sq_err = 0.0;
                        let mut predicted_spikes = Vec::new();
                        let mut pred_rheobase = None;
                        
                        for pt in &test_points {
                            let pred = simulate_glif_fi(
                                rest_v,
                                threshold_v,
                                leak_shift,
                                current_scale,
                                ref_period,
                                pt.stimulus_pa,
                            );
                            predicted_spikes.push(pred);
                            
                            if pred > 0 && pred_rheobase.is_none() {
                                pred_rheobase = Some(pt.stimulus_pa);
                            }
                            
                            let err = (pred as f64) - pt.spike_count_mean;
                            sum_sq_err += err * err;
                        }
                        
                        let rmse = (sum_sq_err / test_points.len() as f64).sqrt();
                        
                        let pred_rheobase_val = pred_rheobase.unwrap_or(1000.0);
                        let rheobase_err = (pred_rheobase_val - neuron.rheobase_pa).abs();
                        
                        // Combined loss function (balance RMSE and Rheobase error)
                        let score = rmse + (rheobase_err / 10.0);
                        
                        // Record in grid records (limit size or store all)
                        grid_records.push(format!(
                            "{},{},{},{},{},{},{:.4},{:.2},{:.2},{:.2}",
                            neuron.specimen_id,
                            neuron.role_label,
                            leak_shift,
                            current_scale,
                            ref_period,
                            threshold_v,
                            rmse,
                            pred_rheobase_val,
                            rheobase_err,
                            score
                        ));
                        
                        if score < best_score {
                            best_score = score;
                            best_rmse = rmse;
                            best_rheobase_err = rheobase_err;
                            best_params = (leak_shift, current_scale, ref_period, threshold_v);
                            best_pred_fi = predicted_spikes;
                        }
                    }
                }
            }
        }
        
        let (b_leak, b_scale, b_ref, b_thresh) = best_params;
        println!("  Best params: leak={}, scale={}, ref={}, thresh={} (RMSE={:.4}, RheoErr={:.2})",
            b_leak, b_scale, b_ref, b_thresh, best_rmse, best_rheobase_err);
            
        // Calculate best predicted rheobase
        let mut best_pred_rheobase = 1000.0;
        for i in 0..test_points.len() {
            if best_pred_fi[i] > 0 {
                best_pred_rheobase = test_points[i].stimulus_pa;
                break;
            }
        }
        
        // Save best record
        best_records.push(format!(
            "{},{},{},{:.4},{},{},{:.4},{:.2},{:.2}",
            neuron.specimen_id,
            neuron.role_label,
            b_leak,
            b_scale,
            b_ref,
            b_thresh,
            best_rmse,
            best_pred_rheobase,
            best_rheobase_err
        ));
    }
    
    // Create artifacts directory
    std::fs::create_dir_all("w:/Workspace/artifacts")?;
    std::fs::create_dir_all("artifacts")?;
    
    // Write Grid CSV
    let grid_file = File::create("w:/Workspace/artifacts/single_neuron_calibration_grid.csv")?;
    let mut grid_writer = BufWriter::new(grid_file);
    writeln!(grid_writer, "specimen_id,role_label,leak_shift,current_scale,refractory_period,threshold,rmse,pred_rheobase,rheobase_error,score")?;
    for row in grid_records {
        writeln!(grid_writer, "{}", row)?;
    }
    grid_writer.flush()?;
    
    // Write Best CSV
    let best_file = File::create("w:/Workspace/artifacts/single_neuron_calibration_best.csv")?;
    let mut best_writer = BufWriter::new(best_file);
    writeln!(best_writer, "specimen_id,role_label,leak_shift,current_scale,refractory_period,threshold,rmse,pred_rheobase,rheobase_error")?;
    for row in best_records {
        writeln!(best_writer, "{}", row)?;
    }
    best_writer.flush()?;
    
    println!("Calibration probe run complete! Files generated successfully.");
    Ok(())
}
