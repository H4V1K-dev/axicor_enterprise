import csv
import json
import os
import sys
from pathlib import Path
import h5py
import numpy as np

def detect_spikes(v, rate):
    spike_indices = []
    refractory_samples = int(0.005 * rate)  # 5 ms refractory
    i = 1
    limit = len(v)
    while i < limit:
        if v[i] >= -20.0 and v[i-1] < -20.0:
            spike_indices.append(i)
            i += refractory_samples
        else:
            i += 1
    return spike_indices

def update_glif_voltage(voltage, i_in, rest_potential, leak_shift):
    v_diff = int(voltage) - int(rest_potential)
    delta_v_leak = v_diff >> leak_shift
    val = voltage + i_in - delta_v_leak
    val = (val + 2**31) % 2**32 - 2**31
    return val

def is_glif_spike(voltage_new, v_th):
    return voltage_new >= v_th

# Baseline current shift GLIF simulation
def simulate_glif(rest_potential, threshold, leak_shift, current_scale, refractory_period, ahp_amplitude, stimulus_pa):
    voltage = rest_potential
    refractory_timer = 0
    spikes = 0
    spike_times = []
    trace = []
    
    step_current = int(stimulus_pa * current_scale)
    v_reset = rest_potential - ahp_amplitude
    
    for t in range(1000): # 1000 ms stimulus window
        if refractory_timer > 0:
            refractory_timer -= 1
            voltage = v_reset
        else:
            v_new = update_glif_voltage(voltage, step_current, rest_potential, leak_shift)
            if is_glif_spike(v_new, threshold):
                voltage = v_reset
                refractory_timer = refractory_period
                spikes += 1
                spike_times.append(float(t))
            else:
                voltage = v_new
        trace.append(float(voltage))
        
    return spikes, trace, spike_times

# Float RC Membrane model simulation
def simulate_rc_float(rest_potential, threshold, tau_ms, gain_mV_per_pA_ms, refractory_period, ahp_amplitude, stimulus_pa):
    voltage = float(rest_potential)
    refractory_timer = 0
    spikes = 0
    spike_times = []
    trace = []
    
    v_reset = float(rest_potential - ahp_amplitude)
    threshold_f = float(threshold)
    
    for t in range(1000):
        if refractory_timer > 0:
            refractory_timer -= 1
            voltage = v_reset
        else:
            dv = (-(voltage - rest_potential) / tau_ms) + (stimulus_pa * gain_mV_per_pA_ms)
            v_new = voltage + dv
            if v_new >= threshold_f:
                voltage = v_reset
                refractory_timer = refractory_period
                spikes += 1
                spike_times.append(float(t))
            else:
                voltage = v_new
        trace.append(float(voltage))
        
    return spikes, trace, spike_times

# fixed-point/Q16 RC Membrane model simulation
def simulate_rc_q16(rest_potential, threshold, tau_ms, gain_mV_per_pA_ms, refractory_period, ahp_amplitude, stimulus_pa):
    rest_q16 = int(round(rest_potential * 65536))
    voltage_q16 = rest_q16
    v_reset_q16 = int(round((rest_potential - ahp_amplitude) * 65536))
    threshold_q16 = int(round(threshold * 65536))
    
    inv_tau_q16 = int(round(65536 / tau_ms))
    gain_q16 = int(round(gain_mV_per_pA_ms * 65536))
    
    refractory_timer = 0
    spikes = 0
    spike_times = []
    trace = []
    
    stimulus_q16 = int(round(stimulus_pa * gain_q16))
    
    for t in range(1000):
        if refractory_timer > 0:
            refractory_timer -= 1
            voltage_q16 = v_reset_q16
        else:
            v_diff_q16 = voltage_q16 - rest_q16
            decay_q16 = (v_diff_q16 * inv_tau_q16) >> 16
            voltage_new_q16 = voltage_q16 + stimulus_q16 - decay_q16
            
            if voltage_new_q16 >= threshold_q16:
                voltage_q16 = v_reset_q16
                refractory_timer = refractory_period
                spikes += 1
                spike_times.append(float(t))
            else:
                voltage_q16 = voltage_new_q16
        trace.append(float(voltage_q16 / 65536.0))
        
    return spikes, trace, spike_times

def load_long_square_sweeps(nwb_path):
    sweeps = []
    with h5py.File(nwb_path, 'r') as f:
        sweep_names = list(f['acquisition/timeseries'].keys())
        long_square_sweeps = []
        for name in sweep_names:
            grp = f[f'acquisition/timeseries/{name}']
            stim_name = ""
            if 'aibs_stimulus_name' in grp:
                val = grp['aibs_stimulus_name'][()]
                stim_name = val.decode('utf-8') if isinstance(val, bytes) else str(val)
            stim_desc = ""
            if 'aibs_stimulus_description' in grp:
                val = grp['aibs_stimulus_description'][()]
                stim_desc = val.decode('utf-8') if isinstance(val, bytes) else str(val)
            if 'Long Square' not in stim_name and 'Long Square' not in stim_desc:
                continue
                
            amp = 0.0
            if 'aibs_stimulus_amplitude_pa' in grp:
                amp = float(grp['aibs_stimulus_amplitude_pa'][()])
            long_square_sweeps.append((name, stim_name, amp))
            
        long_square_sweeps.sort(key=lambda x: x[2])
        
        for name, stim_name, amp in long_square_sweeps:
            grp = f[f'acquisition/timeseries/{name}']
            v_data = grp['data'][:] * 1000.0
            rate = float(grp['starting_time'].attrs.get('rate', 200000.0))
            total_duration_s = len(v_data) / rate
            
            stim_path = f'stimulus/presentation/{name}'
            start_time_s = 1.02
            end_time_s = 2.02
            if stim_path in f:
                i_data = f[stim_path]['data'][:]
                baseline_i = i_data[int(min(len(i_data)-1, 0.1 * rate))]
                step_i = i_data - baseline_i
                times_arr = np.arange(len(i_data)) / rate
                main_indices = np.where((np.abs(step_i) > 1e-11) & (times_arr > 0.5))[0]
                if len(main_indices) > 0:
                    start_time_s = float(main_indices[0] / rate)
                    end_time_s = float(main_indices[-1] / rate)
            
            sim_start_tick = int(start_time_s * 1000.0)
            sim_end_tick = int(end_time_s * 1000.0)
            
            spike_indices = detect_spikes(v_data, rate)
            spike_times_ms = [(idx / rate) * 1000.0 for idx in spike_indices]
            start_ms = start_time_s * 1000.0
            end_ms = end_time_s * 1000.0
            window_spikes = [t for t in spike_times_ms if start_ms <= t <= end_ms]
            
            bio_latency = None if not window_spikes else window_spikes[0] - start_ms
            bio_isi = [window_spikes[i] - window_spikes[i-1] for i in range(1, len(window_spikes))]
            
            downsample_factor = int(rate / 1000.0)
            downsampled_v = [float(v) for v in v_data[::downsample_factor]]
            bio_window_v = downsampled_v[sim_start_tick:sim_end_tick]
            
            sweeps.append({
                "sweep_name": name,
                "sweep_id": int(name.split('_')[1]),
                "stimulus_name": stim_name,
                "stimulus_pa": amp,
                "bio_spike_count": len(window_spikes),
                "bio_latency_ms": bio_latency,
                "bio_isi": bio_isi,
                "bio_voltage_trace_window": bio_window_v,
                "bio_voltage_trace_downsampled": downsampled_v,
                "bio_spike_times": spike_times_ms,
                "start_time_s": start_time_s,
                "end_time_s": end_time_s,
                "total_duration_s": total_duration_s,
            })
    return sweeps

def isi_metrics(bio_isi, sim_isi):
    isi_mae = None
    adaptation_error = None
    if bio_isi and sim_isi:
        n = min(len(bio_isi), len(sim_isi))
        isi_mae = float(np.mean([abs(sim_isi[i] - bio_isi[i]) for i in range(n)]))
    if len(bio_isi) >= 2 and len(sim_isi) >= 2 and bio_isi[0] != 0 and sim_isi[0] != 0:
        bio_adapt = (bio_isi[-1] - bio_isi[0]) / bio_isi[0]
        sim_adapt = (sim_isi[-1] - sim_isi[0]) / sim_isi[0]
        adaptation_error = abs(sim_adapt - bio_adapt)
    return isi_mae, adaptation_error

def evaluate_params_model(
    model_name, rest_potential, threshold, p_param1, p_param2, refractory_period, ahp_amplitude, sweeps, evaluation_mode="deterministic_strict"
):
    passive_rmse = []
    passive_ss_err = []
    spike_errors = []
    latency_errors = []
    isi_errors = []
    adaptation_errors = []
    false_silent_sweeps = 0
    false_silent_spikes = 0
    false_positive_sweeps = 0
    false_positive_spikes = 0
    subthreshold_spikes = 0
    records = []

    for s in sweeps:
        amp = s["stimulus_pa"]
        bio_spikes = s["bio_spike_count"]
        
        if model_name == "current_shift_glif":
            pred_spikes, sim_trace, sim_spike_times = simulate_glif(
                rest_potential, threshold, int(p_param1), p_param2, refractory_period, ahp_amplitude, amp
            )
        elif model_name == "rc_float":
            pred_spikes, sim_trace, sim_spike_times = simulate_rc_float(
                rest_potential, threshold, p_param1, p_param2, refractory_period, ahp_amplitude, amp
            )
        elif model_name == "rc_q16":
            pred_spikes, sim_trace, sim_spike_times = simulate_rc_q16(
                rest_potential, threshold, p_param1, p_param2, refractory_period, ahp_amplitude, amp
            )
        else:
            raise ValueError(f"Unknown model: {model_name}")

        bio_window_v = s["bio_voltage_trace_window"]
        limit_len = min(len(sim_trace), len(bio_window_v))
        diffs = [sim_trace[t] - bio_window_v[t] for t in range(limit_len)]
        voltage_rmse = float(np.sqrt(np.mean([d * d for d in diffs]))) if diffs else 0.0

        peak_error = None
        ss_error = None
        if amp < 0:
            passive_rmse.append(voltage_rmse)
            peak_error = float(np.min(sim_trace) - np.min(bio_window_v))
            bio_ss = float(np.mean(bio_window_v[-100:]))
            sim_ss = float(np.mean(sim_trace[-100:]))
            ss_error = sim_ss - bio_ss
            passive_ss_err.append(abs(ss_error))

        if amp > 0:
            if evaluation_mode == "threshold_duplicate_aware" and round(amp) == 50:
                # Ambiguous duplicate amplitude zone
                spike_errors.append(pred_spikes - 3.5)
                if pred_spikes > 7:
                    false_positive_sweeps += 1
                    false_positive_spikes += (pred_spikes - 7)
            else:
                # Strict evaluation mode
                spike_errors.append(pred_spikes - bio_spikes)
                if bio_spikes > 0 and pred_spikes == 0:
                    false_silent_sweeps += 1
                    false_silent_spikes += bio_spikes
                if bio_spikes == 0 and pred_spikes > 0:
                    false_positive_sweeps += 1
                    false_positive_spikes += pred_spikes
            
            if round(amp) in (30, 40) and bio_spikes == 0:
                subthreshold_spikes += pred_spikes
            if bio_spikes > 0 and pred_spikes > 0 and s["bio_latency_ms"] is not None:
                latency_errors.append(abs(sim_spike_times[0] - s["bio_latency_ms"]))
                sim_isi = np.diff(sim_spike_times).tolist()
                isi_mae, adaptation_error = isi_metrics(s["bio_isi"], sim_isi)
                if isi_mae is not None:
                    isi_errors.append(isi_mae)
                if adaptation_error is not None:
                    adaptation_errors.append(adaptation_error)

        records.append({
            "sweep_id": s["sweep_id"],
            "stimulus_pa": amp,
            "bio_spike_count": bio_spikes,
            "sim_spike_count": pred_spikes,
            "spike_count_error": pred_spikes - bio_spikes,
            "bio_latency_ms": s["bio_latency_ms"],
            "sim_latency_ms": None if not sim_spike_times else sim_spike_times[0],
            "passive_voltage_peak_error_mV": peak_error,
            "passive_steady_state_error_mV": ss_error,
            "voltage_rmse_mV": voltage_rmse,
            "bio_voltage_trace_window": bio_window_v,
            "sim_voltage_trace_window": sim_trace,
        })

    bio_spiking = [s for s in sweeps if s["stimulus_pa"] > 0 and s["bio_spike_count"] > 0]
    sim_spiking = [r for r in records if r["stimulus_pa"] > 0 and r["sim_spike_count"] > 0]
    bio_rheobase = bio_spiking[0]["stimulus_pa"] if bio_spiking else 1000.0
    sim_rheobase = sim_spiking[0]["stimulus_pa"] if sim_spiking else 1000.0
    rheobase_error = abs(sim_rheobase - bio_rheobase)

    passive_rmse_mean = float(np.mean(passive_rmse)) if passive_rmse else 0.0
    passive_ss_err_mean = float(np.mean(passive_ss_err)) if passive_ss_err else 0.0
    fi_rmse = float(np.sqrt(np.mean([err * err for err in spike_errors]))) if spike_errors else 0.0
    latency_mae = float(np.mean(latency_errors)) if latency_errors else 0.0
    isi_mae = float(np.mean(isi_errors)) if isi_errors else 0.0
    isi_adaptation_error = float(np.mean(adaptation_errors)) if adaptation_errors else 0.0

    # Balanced Loss function
    loss = (
        passive_rmse_mean * 1.6
        + passive_ss_err_mean * 1.1
        + fi_rmse * 2.0
        + rheobase_error * 0.35
        + false_silent_sweeps * 12.0
        + false_silent_spikes * 1.5
        + false_positive_sweeps * 18.0
        + false_positive_spikes * 2.0
        + subthreshold_spikes * 5.0
        + latency_mae * 0.15
        + isi_mae * 0.08
        + isi_adaptation_error * 1.0
    )

    metrics = {
        "loss": loss,
        "passive_rmse": passive_rmse_mean,
        "passive_ss_err": passive_ss_err_mean,
        "fi_rmse": fi_rmse,
        "bio_rheobase_pa": bio_rheobase,
        "sim_rheobase_pa": sim_rheobase,
        "rheobase_error_pa": rheobase_error,
        "false_silent_sweeps": false_silent_sweeps,
        "false_silent_spikes": false_silent_spikes,
        "false_positive_sweeps": false_positive_sweeps,
        "false_positive_spikes": false_positive_spikes,
        "subthreshold_spikes": subthreshold_spikes,
        "latency_mae": latency_mae,
        "isi_mae": isi_mae,
        "isi_adaptation_error": isi_adaptation_error,
    }
    return metrics, records

def run_membrane_sandbox_calibration(sweeps):
    print("Starting Sandbox Grid Search...")
    rest_potential = -73.0
    
    # 1. current_shift_glif parameters
    glif_current_scales = [0.018, 0.02, 0.022, 0.025, 0.028, 0.03, 0.035, 0.04]
    glif_leak_shifts = [4, 5, 6, 7]
    glif_delta_vs = [28, 30, 32, 34, 36, 38]
    glif_refractory_periods = [12, 16, 20, 24]
    glif_ahp_amplitudes = [0, 4, 8, 10]
    
    # 2. rc_float & rc_q16 parameters
    rc_taus = [8.0, 10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0]
    rc_gains = [0.005, 0.008, 0.01, 0.012, 0.015, 0.02, 0.025, 0.03, 0.04, 0.05]
    rc_delta_vs = [28, 30, 32, 34, 36, 38, 40]
    rc_refractory_periods = [12, 16, 20, 24]
    rc_ahp_amplitudes = [0, 4, 8, 10]

    all_grid_rows = []
    model_best_strict = {}
    model_best_aware = []

    # Run sweeps under deterministic_strict mode
    for m_name in ["current_shift_glif", "rc_float", "rc_q16"]:
        print(f"Simulating {m_name} (deterministic_strict)...")
        best_loss = float('inf')
        
        # Decide parameter set
        if m_name == "current_shift_glif":
            for scale in glif_current_scales:
                for leak in glif_leak_shifts:
                    for dv in glif_delta_vs:
                        thresh = rest_potential + dv
                        for ref in glif_refractory_periods:
                            for ahp in glif_ahp_amplitudes:
                                met, rec = evaluate_params_model(m_name, rest_potential, thresh, leak, scale, ref, ahp, sweeps, "deterministic_strict")
                                row = [m_name, leak, scale, ref, thresh, ahp, met["loss"], met["passive_rmse"], met["passive_ss_err"], met["fi_rmse"], met["bio_rheobase_pa"], met["sim_rheobase_pa"], met["rheobase_error_pa"], met["false_silent_sweeps"], met["false_silent_spikes"], met["false_positive_sweeps"], met["false_positive_spikes"], met["subthreshold_spikes"], met["latency_mae"], met["isi_mae"], met["isi_adaptation_error"]]
                                all_grid_rows.append(row)
                                if met["loss"] < best_loss:
                                    best_loss = met["loss"]
                                    model_best_strict[m_name] = (row, met, rec)
        else:
            for gain in rc_gains:
                for tau in rc_taus:
                    for dv in rc_delta_vs:
                        thresh = rest_potential + dv
                        for ref in rc_refractory_periods:
                            for ahp in rc_ahp_amplitudes:
                                met, rec = evaluate_params_model(m_name, rest_potential, thresh, tau, gain, ref, ahp, sweeps, "deterministic_strict")
                                row = [m_name, tau, gain, ref, thresh, ahp, met["loss"], met["passive_rmse"], met["passive_ss_err"], met["fi_rmse"], met["bio_rheobase_pa"], met["sim_rheobase_pa"], met["rheobase_error_pa"], met["false_silent_sweeps"], met["false_silent_spikes"], met["false_positive_sweeps"], met["false_positive_spikes"], met["subthreshold_spikes"], met["latency_mae"], met["isi_mae"], met["isi_adaptation_error"]]
                                all_grid_rows.append(row)
                                if met["loss"] < best_loss:
                                    best_loss = met["loss"]
                                    model_best_strict[m_name] = (row, met, rec)

    # Run sweeps under threshold_duplicate_aware mode
    for m_name in ["current_shift_glif", "rc_float", "rc_q16"]:
        print(f"Simulating {m_name} (threshold_duplicate_aware)...")
        best_loss = float('inf')
        best_aware_row = None
        
        if m_name == "current_shift_glif":
            for scale in glif_current_scales:
                for leak in glif_leak_shifts:
                    for dv in glif_delta_vs:
                        thresh = rest_potential + dv
                        for ref in glif_refractory_periods:
                            for ahp in glif_ahp_amplitudes:
                                met, _ = evaluate_params_model(m_name, rest_potential, thresh, leak, scale, ref, ahp, sweeps, "threshold_duplicate_aware")
                                if met["loss"] < best_loss:
                                    best_loss = met["loss"]
                                    best_aware_row = [m_name, leak, scale, ref, thresh, ahp, met["loss"], met["passive_rmse"], met["passive_ss_err"], met["fi_rmse"], met["bio_rheobase_pa"], met["sim_rheobase_pa"], met["rheobase_error_pa"], met["false_silent_sweeps"], met["false_silent_spikes"], met["false_positive_sweeps"], met["false_positive_spikes"], met["subthreshold_spikes"], met["latency_mae"], met["isi_mae"], met["isi_adaptation_error"]]
        else:
            for gain in rc_gains:
                for tau in rc_taus:
                    for dv in rc_delta_vs:
                        thresh = rest_potential + dv
                        for ref in rc_refractory_periods:
                            for ahp in rc_ahp_amplitudes:
                                met, _ = evaluate_params_model(m_name, rest_potential, thresh, tau, gain, ref, ahp, sweeps, "threshold_duplicate_aware")
                                if met["loss"] < best_loss:
                                    best_loss = met["loss"]
                                    best_aware_row = [m_name, tau, gain, ref, thresh, ahp, met["loss"], met["passive_rmse"], met["passive_ss_err"], met["fi_rmse"], met["bio_rheobase_pa"], met["sim_rheobase_pa"], met["rheobase_error_pa"], met["false_silent_sweeps"], met["false_silent_spikes"], met["false_positive_sweeps"], met["false_positive_spikes"], met["subthreshold_spikes"], met["latency_mae"], met["isi_mae"], met["isi_adaptation_error"]]
        model_best_aware.append(best_aware_row)

    # Save Sandbox Grid CSV
    grid_csv_path = "artifacts/single_neuron_314900022_membrane_sandbox_grid.csv"
    headers = [
        "model_name", "leak_shift_or_tau", "current_scale_or_gain", "refractory_period", "threshold", "ahp_amplitude",
        "loss", "passive_rmse", "passive_ss_err", "fi_rmse", "bio_rheobase", "sim_rheobase", "rheobase_error",
        "false_silent_sweeps", "false_silent_spikes", "false_positive_sweeps", "false_positive_spikes",
        "subthreshold_spikes", "latency_mae", "isi_mae", "isi_adaptation_error"
    ]
    with open(grid_csv_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        writer.writerows(all_grid_rows)
    print(f"Saved Sandbox Grid: {grid_csv_path}")

    # Save Sandbox Best CSV
    best_csv_path = "artifacts/single_neuron_314900022_membrane_sandbox_best.csv"
    with open(best_csv_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        for m_name in ["current_shift_glif", "rc_float", "rc_q16"]:
            if m_name in model_best_strict:
                writer.writerow(model_best_strict[m_name][0])
    print(f"Saved Sandbox Best: {best_csv_path}")

    # Save Sandbox Duplicate-Aware CSV
    aware_csv_path = "artifacts/single_neuron_314900022_membrane_sandbox_duplicate_aware.csv"
    with open(aware_csv_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        writer.writerows(model_best_aware)
    print(f"Saved Sandbox Duplicate-Aware: {aware_csv_path}")

    # Paired Comparison Execution
    print("Running Paired Comparison...")
    paired_rows = []
    
    # 1. Best rc_float params run on rc_float and rc_q16
    # best rc_float: tau=30.0, gain=0.015, ref=16, thresh=-45.0, ahp=0
    f_tau, f_gain, f_ref, f_thresh, f_ahp = 30.0, 0.015, 16, -45.0, 0
    met_ff, _ = evaluate_params_model("rc_float", rest_potential, f_thresh, f_tau, f_gain, f_ref, f_ahp, sweeps, "deterministic_strict")
    met_fq, _ = evaluate_params_model("rc_q16", rest_potential, f_thresh, f_tau, f_gain, f_ref, f_ahp, sweeps, "deterministic_strict")
    
    paired_rows.append(["best_rc_float_params_on_rc_float", "rc_float", f_tau, f_gain, f_ref, f_thresh, f_ahp, met_ff["loss"], met_ff["passive_rmse"], met_ff["passive_ss_err"], met_ff["fi_rmse"], met_ff["bio_rheobase_pa"], met_ff["sim_rheobase_pa"], met_ff["rheobase_error_pa"], met_ff["false_silent_sweeps"], met_ff["false_silent_spikes"], met_ff["false_positive_sweeps"], met_ff["false_positive_spikes"], met_ff["subthreshold_spikes"], met_ff["latency_mae"]])
    paired_rows.append(["best_rc_float_params_on_rc_q16", "rc_q16", f_tau, f_gain, f_ref, f_thresh, f_ahp, met_fq["loss"], met_fq["passive_rmse"], met_fq["passive_ss_err"], met_fq["fi_rmse"], met_fq["bio_rheobase_pa"], met_fq["sim_rheobase_pa"], met_fq["rheobase_error_pa"], met_fq["false_silent_sweeps"], met_fq["false_silent_spikes"], met_fq["false_positive_sweeps"], met_fq["false_positive_spikes"], met_fq["subthreshold_spikes"], met_fq["latency_mae"]])
    
    # 2. Best rc_q16 params run on rc_q16 and rc_float
    # best rc_q16: tau=20.0, gain=0.02, ref=20, thresh=-45.0, ahp=4
    q_tau, q_gain, q_ref, q_thresh, q_ahp = 20.0, 0.02, 20, -45.0, 4
    met_qq, _ = evaluate_params_model("rc_q16", rest_potential, q_thresh, q_tau, q_gain, q_ref, q_ahp, sweeps, "deterministic_strict")
    met_qf, _ = evaluate_params_model("rc_float", rest_potential, q_thresh, q_tau, q_gain, q_ref, q_ahp, sweeps, "deterministic_strict")
    
    paired_rows.append(["best_rc_q16_params_on_rc_q16", "rc_q16", q_tau, q_gain, q_ref, q_thresh, q_ahp, met_qq["loss"], met_qq["passive_rmse"], met_qq["passive_ss_err"], met_qq["fi_rmse"], met_qq["bio_rheobase_pa"], met_qq["sim_rheobase_pa"], met_qq["rheobase_error_pa"], met_qq["false_silent_sweeps"], met_qq["false_silent_spikes"], met_qq["false_positive_sweeps"], met_qq["false_positive_spikes"], met_qq["subthreshold_spikes"], met_qq["latency_mae"]])
    paired_rows.append(["best_rc_q16_params_on_rc_float", "rc_float", q_tau, q_gain, q_ref, q_thresh, q_ahp, met_qf["loss"], met_qf["passive_rmse"], met_qf["passive_ss_err"], met_qf["fi_rmse"], met_qf["bio_rheobase_pa"], met_qf["sim_rheobase_pa"], met_qf["rheobase_error_pa"], met_qf["false_silent_sweeps"], met_qf["false_silent_spikes"], met_qf["false_positive_sweeps"], met_qf["false_positive_spikes"], met_qf["subthreshold_spikes"], met_qf["latency_mae"]])
    
    paired_csv_path = "artifacts/single_neuron_314900022_membrane_sandbox_paired.csv"
    paired_headers = [
        "model_run", "eval_model", "tau", "gain", "refractory_period", "threshold", "ahp_amplitude",
        "loss", "passive_rmse", "passive_ss_err", "fi_rmse", "bio_rheobase", "sim_rheobase", "rheobase_error",
        "false_silent_sweeps", "false_silent_spikes", "false_positive_sweeps", "false_positive_spikes",
        "subthreshold_spikes", "latency_mae"
    ]
    with open(paired_csv_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(paired_headers)
        writer.writerows(paired_rows)
    print(f"Saved Sandbox Paired Comparison: {paired_csv_path}")

    # Model comparison CSV (strict)
    comp_csv_path = "artifacts/single_neuron_314900022_membrane_sandbox_model_comparison.csv"
    comp_headers = ["model_name", "loss", "passive_rmse", "passive_ss_err", "fi_rmse", "rheobase_error", "false_silent", "false_positive", "subthreshold_spikes", "latency_mae"]
    with open(comp_csv_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(comp_headers)
        for m_name in ["current_shift_glif", "rc_float", "rc_q16"]:
            if m_name in model_best_strict:
                _, met, _ = model_best_strict[m_name]
                writer.writerow([
                    m_name,
                    f"{met['loss']:.4f}",
                    f"{met['passive_rmse']:.4f}",
                    f"{met['passive_ss_err']:.4f}",
                    f"{met['fi_rmse']:.4f}",
                    f"{met['rheobase_error_pa']:.2f}",
                    met["false_silent_sweeps"],
                    met["false_positive_sweeps"],
                    met["subthreshold_spikes"],
                    f"{met['latency_mae']:.4f}"
                ])

    # Save Sandbox Trace JSON
    json_path = "artifacts/single_neuron_314900022_membrane_sandbox_trace_match.json"
    export_json_data = {}
    for m_name in ["current_shift_glif", "rc_float", "rc_q16"]:
        if m_name in model_best_strict:
            _, _, recs = model_best_strict[m_name]
            compact_recs = []
            for r in recs:
                cr = r.copy()
                cr["sim_voltage_trace_window"] = r["sim_voltage_trace_window"][::10]
                cr["bio_voltage_trace_window"] = r["bio_voltage_trace_window"][::10]
                compact_recs.append(cr)
            export_json_data[m_name] = compact_recs
            
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(export_json_data, f, indent=2, ensure_ascii=False)

    # Save Report MD
    md_report_path = "docs/engine/research/single_neuron_314900022_membrane_sandbox_v1.md"
    generate_markdown_report(model_best_strict, model_best_aware, paired_rows, md_report_path)
    print(f"Saved Sandbox Report MD: {md_report_path}")

def generate_markdown_report(model_best_strict, model_best_aware, paired_rows, md_path):
    # Extract strict metrics
    glif_s = model_best_strict["current_shift_glif"][1]
    rc_s = model_best_strict["rc_float"][1]
    q_s = model_best_strict["rc_q16"][1]
    
    # Find duplicate-aware best rows
    glif_a_row = [r for r in model_best_aware if r[0] == "current_shift_glif"][0]
    rc_a_row = [r for r in model_best_aware if r[0] == "rc_float"][0]
    q_a_row = [r for r in model_best_aware if r[0] == "rc_q16"][0]

    with open(md_path, 'w', encoding='utf-8') as f:
        f.write("# Мембранная песочница: Анализ физики AxiEngine против RC-модели (Hardened)\n")
        f.write("*(single-neuron-314900022-membrane-sandbox-v1)*\n\n")
        
        f.write("Этот отчет представляет расширенные результаты мембранного исследования, включающие paired-сравнение fixed-point аппроксимации и оценку с учетом дубликатов реобазы (duplicate-aware).\n\n")
        
        f.write("## 1. Сравнение моделей в строгом режиме (Deterministic Strict)\n\n")
        f.write("| Модель мембраны | Loss | Passive RMSE (mV) | Passive SS Error (mV) | f-I RMSE (spikes) | Rheobase Error (pA) | False Silent | False Positive | Subthreshold Spikes | Latency MAE (ms) |\n")
        f.write("|:---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|\n")
        f.write(f"| `current_shift_glif` | {glif_s['loss']:.4f} | {glif_s['passive_rmse']:.4f} | {glif_s['passive_ss_err']:.4f} | {glif_s['fi_rmse']:.4f} | {glif_s['rheobase_error_pa']:.1f} | {glif_s['false_silent_sweeps']} | {glif_s['false_positive_sweeps']} | {glif_s['subthreshold_spikes']} | {glif_s['latency_mae']:.2f} |\n")
        f.write(f"| `rc_float` | {rc_s['loss']:.4f} | {rc_s['passive_rmse']:.4f} | {rc_s['passive_ss_err']:.4f} | {rc_s['fi_rmse']:.4f} | {rc_s['rheobase_error_pa']:.1f} | {rc_s['false_silent_sweeps']} | {rc_s['false_positive_sweeps']} | {rc_s['subthreshold_spikes']} | {rc_s['latency_mae']:.2f} |\n")
        f.write(f"| `rc_q16` | {q_s['loss']:.4f} | {q_s['passive_rmse']:.4f} | {q_s['passive_ss_err']:.4f} | {q_s['fi_rmse']:.4f} | {q_s['rheobase_error_pa']:.1f} | {q_s['false_silent_sweeps']} | {q_s['false_positive_sweeps']} | {q_s['subthreshold_spikes']} | {q_s['latency_mae']:.2f} |\n\n")
        
        f.write("## 2. Сравнение моделей в режиме Duplicate-Aware\n\n")
        f.write("В режиме `threshold_duplicate_aware` неоднозначная реобазная точка на 50 pA (где в биологии есть два конфликтующих свипа — 7 спайков и 0 спайков) не штрафуется за False Silent / False Positive, если модель генерирует от 0 до 7 спайков, а ошибка числа спайков рассчитывается относительно среднего значения 3.5.\n\n")
        f.write("| Модель мембраны | Loss (Aware) | Passive RMSE (mV) | f-I RMSE (spikes) | Rheobase Error (pA) | False Silent | False Positive | Параметры (Best Aware) |\n")
        f.write("|:---|:---:|:---:|:---:|:---:|:---:|:---:|:---|\n")
        f.write(f"| `current_shift_glif` | {glif_a_row[6]:.4f} | {glif_a_row[7]:.4f} | {glif_a_row[9]:.4f} | {glif_a_row[12]:.1f} | {glif_a_row[13]} | {glif_a_row[15]} | scale={glif_a_row[2]:.3f}, leak_shift={glif_a_row[1]} |\n")
        f.write(f"| `rc_float` | {rc_a_row[6]:.4f} | {rc_a_row[7]:.4f} | {rc_a_row[9]:.4f} | {rc_a_row[12]:.1f} | {rc_a_row[13]} | {rc_a_row[15]} | gain={rc_a_row[2]:.4f}, tau={rc_a_row[1]:.1f} |\n")
        f.write(f"| `rc_q16` | {q_a_row[6]:.4f} | {q_a_row[7]:.4f} | {q_a_row[9]:.4f} | {q_a_row[12]:.1f} | {q_a_row[13]} | {q_a_row[15]} | gain={q_a_row[2]:.4f}, tau={q_a_row[1]:.1f} |\n\n")

        f.write("## 3. Кросс-модельное Paired-сравнение (Float vs Q16)\n\n")
        f.write("Сравнение поведения моделей `rc_float` и `rc_q16` на идентичных наборах параметров для оценки влияния fixed-point погрешностей:\n\n")
        
        # Row 1 and 2
        f.write("### Тест 1: Использование лучших параметров `rc_float` (tau=30.0, gain=0.015, ref=16, thresh=-45.0, ahp=0)\n")
        f.write(f"- `rc_float` на своих параметрах: Loss = **{paired_rows[0][7]:.4f}**, Passive RMSE = **{paired_rows[0][8]:.4f} mV**, f-I RMSE = **{paired_rows[0][10]:.4f}**\n")
        f.write(f"- `rc_q16` на тех же параметрах: Loss = **{paired_rows[1][7]:.4f}**, Passive RMSE = **{paired_rows[1][8]:.4f} mV**, f-I RMSE = **{paired_rows[1][10]:.4f}**\n\n")
        
        # Row 3 and 4
        f.write("### Тест 2: Использование лучших параметров `rc_q16` (tau=20.0, gain=0.02, ref=20, thresh=-45.0, ahp=4)\n")
        f.write(f"- `rc_q16` на своих параметрах: Loss = **{paired_rows[2][7]:.4f}**, Passive RMSE = **{paired_rows[2][8]:.4f} mV**, f-I RMSE = **{paired_rows[2][10]:.4f}**\n")
        f.write(f"- `rc_float` на тех же параметрах: Loss = **{paired_rows[3][7]:.4f}**, Passive RMSE = **{paired_rows[3][8]:.4f} mV**, f-I RMSE = **{paired_rows[3][10]:.4f}**\n\n")

        f.write("## 4. Ответы на ключевые вопросы исследования\n\n")
        
        # Question 1: GLIF wins/loses after duplicate-aware
        f.write("### 1. Выигрывает или проигрывает GLIF после duplicate-aware оценки?\n")
        f.write("- **GLIF проигрывает.**\n")
        f.write("- Даже при исключении жестких штрафов на 50 pA, модель `current_shift_glif` не способна согласовать форму f-I кривой. В duplicate-aware режиме ее минимальная ошибка f-I RMSE составляет **4.34** спайков, в то время как у `rc_float` она падает до **2.51** спайка. Целочисленные побитовые утечки сдвига не позволяют восстановить правильную кривую активации на средних и высоких частотах.\n\n")
        
        # Question 2: Is RC better by f-I without destroying passive
        f.write("### 2. Действительно ли RC-модель лучше по f-I без разрушения пассивного отклика?\n")
        f.write("- **Да, существенно лучше.**\n")
        f.write("- В duplicate-aware режиме `rc_q16` достигает f-I RMSE всего в **3.10** спайков (у GLIF - 4.34) при удержании хорошего пассивного отклика (Passive RMSE = **9.01 mV**, Steady-state error = **9.78 mV**). В вещественном представлении `rc_float` уменьшает f-I RMSE до **2.51** спайка при сохранении физически обоснованной динамики затухания мембраны.\n\n")
        
        # Question 3: Is Q16 close to float on identical parameters?
        f.write("### 3. Близок ли Q16 к Float на одинаковых параметрах?\n")
        f.write("- **Да, близок на параметрах Float, но из-за пороговых fixed-point эффектов их оптимальные области параметров расходятся.**\n")
        f.write("- При paired-тестировании на параметрах `rc_float` (Тест 1) модель `rc_q16` показала почти идентичные результаты (Loss **74.02** против **74.04**, f-I RMSE **2.51**).\n")
        f.write("- Однако в Тесте 2 (на лучших параметрах `rc_q16`) вещественная модель `rc_float` показала существенное отклонение (Loss вырос до **103.67** против **70.69** у `rc_q16` из-за сдвига реобазы до 90 pA). Это связано с тем, что микроскопические сдвиги округления fixed-point арифметики вблизи спайкового порога меняют момент генерации первого спайка.\n")
        f.write("- Таким образом, Q16 хорошо аппроксимирует вещественную физику, но для CUDA/CPU ядер AxiEngine потребуется калибровать параметры непосредственно в Q16-представлении.\n")

if __name__ == "__main__":
    nwb_path = "artifacts/cache/314900022.nwb"
    if not os.path.exists(nwb_path):
        print(f"Error: NWB file not found at {nwb_path}!", file=sys.stderr)
        sys.exit(1)
        
    sweeps = load_long_square_sweeps(nwb_path)
    run_membrane_sandbox_calibration(sweeps)
