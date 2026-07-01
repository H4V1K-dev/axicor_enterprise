import csv
import json
from pathlib import Path

import h5py
import numpy as np

import trace_match_314900022 as base


ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "artifacts"
DOCS = ROOT / "docs" / "engine" / "research"
NWB_PATH = ARTIFACTS / "cache" / "314900022.nwb"


def load_long_square_sweeps(nwb_path):
    sweeps = []
    with h5py.File(nwb_path, "r") as f:
        sweep_names = list(f["acquisition/timeseries"].keys())
        long_square_sweeps = []
        for name in sweep_names:
            grp = f[f"acquisition/timeseries/{name}"]
            stim_name = ""
            if "aibs_stimulus_name" in grp:
                val = grp["aibs_stimulus_name"][()]
                stim_name = val.decode("utf-8") if isinstance(val, bytes) else str(val)
            stim_desc = ""
            if "aibs_stimulus_description" in grp:
                val = grp["aibs_stimulus_description"][()]
                stim_desc = val.decode("utf-8") if isinstance(val, bytes) else str(val)
            if "Long Square" not in stim_name and "Long Square" not in stim_desc:
                continue

            amp = 0.0
            if "aibs_stimulus_amplitude_pa" in grp:
                amp = float(grp["aibs_stimulus_amplitude_pa"][()])
            long_square_sweeps.append((name, stim_name, amp))

        long_square_sweeps.sort(key=lambda x: x[2])

        for name, stim_name, amp in long_square_sweeps:
            grp = f[f"acquisition/timeseries/{name}"]
            v_data = grp["data"][:] * 1000.0
            rate = float(grp["starting_time"].attrs.get("rate", 200000.0))
            total_duration_s = len(v_data) / rate

            stim_path = f"stimulus/presentation/{name}"
            start_time_s = 1.02
            end_time_s = 2.02
            if stim_path in f:
                i_data = f[stim_path]["data"][:]
                baseline_i = i_data[int(min(len(i_data) - 1, 0.1 * rate))]
                step_i = i_data - baseline_i
                times_arr = np.arange(len(i_data)) / rate
                main_indices = np.where((np.abs(step_i) > 1e-11) & (times_arr > 0.5))[0]
                if len(main_indices) > 0:
                    start_time_s = float(main_indices[0] / rate)
                    end_time_s = float(main_indices[-1] / rate)

            sim_start_tick = int(start_time_s * 1000.0)
            sim_end_tick = int(end_time_s * 1000.0)

            spike_indices = base.detect_spikes(v_data, rate)
            spike_times_ms = [(idx / rate) * 1000.0 for idx in spike_indices]
            start_ms = start_time_s * 1000.0
            end_ms = end_time_s * 1000.0
            window_spikes = [t for t in spike_times_ms if start_ms <= t <= end_ms]
            bio_latency = None if not window_spikes else window_spikes[0] - start_ms
            bio_isi = [
                window_spikes[i] - window_spikes[i - 1]
                for i in range(1, len(window_spikes))
            ]

            downsample_factor = int(rate / 1000.0)
            downsampled_v = [float(v) for v in v_data[::downsample_factor]]
            bio_window_v = downsampled_v[sim_start_tick:sim_end_tick]

            sweeps.append(
                {
                    "sweep_name": name,
                    "sweep_id": int(name.split("_")[1]),
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
                }
            )
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


def evaluate_params(
    rest_potential,
    threshold,
    leak_shift,
    current_scale,
    refractory_period,
    ahp_amplitude,
    sweeps,
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
        pred_spikes, sim_trace, sim_spike_times = base.simulate_stimulus_only(
            rest_potential,
            threshold,
            leak_shift,
            current_scale,
            refractory_period,
            ahp_amplitude,
            amp,
        )

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
                isi_mae, adaptation_error = isi_metrics(s["bio_isi"], np.diff(sim_spike_times).tolist())
                if isi_mae is not None:
                    isi_errors.append(isi_mae)
                if adaptation_error is not None:
                    adaptation_errors.append(adaptation_error)

        records.append(
            {
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
            }
        )

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


def param_grid(rest_potential):
    current_scales = [0.018, 0.02, 0.022, 0.025, 0.028, 0.03, 0.035, 0.04]
    leak_shifts = [4, 5, 6, 7]
    delta_vs = [28, 30, 32, 34, 36, 38]
    refractory_periods = [12, 16, 20, 24]
    ahp_amplitudes = [0, 4, 8, 10]
    for current_scale in current_scales:
        for leak_shift in leak_shifts:
            for delta_v in delta_vs:
                threshold = rest_potential + delta_v
                for refractory_period in refractory_periods:
                    for ahp_amplitude in ahp_amplitudes:
                        yield leak_shift, current_scale, refractory_period, threshold, ahp_amplitude


def run_balanced_grid(rest_potential, sweeps):
    rows = []
    best = None
    best_metrics = None
    best_records = None

    for params in param_grid(rest_potential):
        metrics, records = evaluate_params(rest_potential, params[3], params[0], params[1], params[2], params[4], sweeps)
        row = list(params) + [metrics[k] for k in METRIC_FIELDS]
        rows.append(row)
        if best_metrics is None or metrics["loss"] < best_metrics["loss"]:
            best = params
            best_metrics = metrics
            best_records = records

    return rows, best, best_metrics, best_records


METRIC_FIELDS = [
    "loss",
    "passive_rmse",
    "passive_ss_err",
    "fi_rmse",
    "bio_rheobase_pa",
    "sim_rheobase_pa",
    "rheobase_error_pa",
    "false_silent_sweeps",
    "false_silent_spikes",
    "false_positive_sweeps",
    "false_positive_spikes",
    "subthreshold_spikes",
    "latency_mae",
    "isi_mae",
    "isi_adaptation_error",
]

PARAM_FIELDS = ["leak_shift", "current_scale", "refractory_period", "threshold", "ahp_amplitude"]
GRID_FIELDS = PARAM_FIELDS + METRIC_FIELDS


def row_to_dict(row):
    return {field: row[idx] for idx, field in enumerate(GRID_FIELDS)}


def best_candidate(rows, predicate):
    candidates = [row_to_dict(row) for row in rows if predicate(row_to_dict(row))]
    if not candidates:
        return None
    return min(candidates, key=lambda item: float(item["loss"]))


def write_grid(rows):
    path = ARTIFACTS / "single_neuron_314900022_balanced_grid.csv"
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["leak_shift", "current_scale", "refractory_period", "threshold", "ahp_amplitude"] + METRIC_FIELDS)
        writer.writerows(rows)
    return path


def write_candidate_comparison(rows):
    candidate_rows = [
        ("overall_best", best_candidate(rows, lambda _item: True)),
        ("exact_rheobase_best", best_candidate(rows, lambda item: float(item["sim_rheobase_pa"]) == 50.0)),
        ("no_false_silent_best", best_candidate(rows, lambda item: int(item["false_silent_sweeps"]) == 0)),
        ("good_passive_rheobase_le_70_best", best_candidate(rows, lambda item: float(item["passive_rmse"]) <= 7.0 and float(item["sim_rheobase_pa"]) <= 70.0)),
    ]
    path = ARTIFACTS / "single_neuron_314900022_balanced_candidate_comparison.csv"
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["candidate"] + GRID_FIELDS)
        for name, item in candidate_rows:
            if item is None:
                writer.writerow([name] + ["n/a"] * len(GRID_FIELDS))
            else:
                writer.writerow([name] + [item[field] for field in GRID_FIELDS])
    return path, candidate_rows


def write_best(params, metrics):
    path = ARTIFACTS / "single_neuron_314900022_balanced_best.csv"
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["leak_shift", "current_scale", "refractory_period", "threshold", "ahp_amplitude"] + METRIC_FIELDS)
        writer.writerow(list(params) + [metrics[k] for k in METRIC_FIELDS])
    return path


def write_sweeps(records):
    path = ARTIFACTS / "single_neuron_314900022_balanced_trace_match_sweeps.csv"
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(
            [
                "sweep_id",
                "stimulus_pa",
                "bio_spike_count",
                "sim_spike_count",
                "spike_count_error",
                "bio_latency_ms",
                "sim_latency_ms",
                "passive_voltage_peak_error_mV",
                "passive_steady_state_error_mV",
                "voltage_rmse_mV",
            ]
        )
        for r in records:
            writer.writerow(
                [
                    r["sweep_id"],
                    r["stimulus_pa"],
                    r["bio_spike_count"],
                    r["sim_spike_count"],
                    r["spike_count_error"],
                    "n/a" if r["bio_latency_ms"] is None else f"{r['bio_latency_ms']:.2f}",
                    "n/a" if r["sim_latency_ms"] is None else f"{r['sim_latency_ms']:.2f}",
                    "n/a" if r["passive_voltage_peak_error_mV"] is None else f"{r['passive_voltage_peak_error_mV']:.2f}",
                    "n/a" if r["passive_steady_state_error_mV"] is None else f"{r['passive_steady_state_error_mV']:.2f}",
                    f"{r['voltage_rmse_mV']:.2f}",
                ]
            )
    return path


def write_summary(params, metrics):
    path = ARTIFACTS / "single_neuron_314900022_balanced_summary.csv"
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["specimen_id", "leak_shift", "current_scale", "refractory_period", "threshold", "ahp_amplitude"] + METRIC_FIELDS)
        writer.writerow([314900022] + list(params) + [metrics[k] for k in METRIC_FIELDS])
    return path


def write_json(records):
    path = ARTIFACTS / "single_neuron_314900022_balanced_trace_match.json"
    export = []
    for r in records:
        item = {k: v for k, v in r.items() if k not in ("bio_voltage_trace_window", "sim_voltage_trace_window")}
        item["bio_voltage_trace_window"] = r["bio_voltage_trace_window"][::10]
        item["sim_voltage_trace_window"] = r["sim_voltage_trace_window"][::10]
        export.append(item)
    with path.open("w", encoding="utf-8") as f:
        json.dump(export, f, indent=2, ensure_ascii=False)
    return path


def write_report(params, metrics, records, candidate_rows):
    leak_shift, current_scale, refractory_period, threshold, ahp_amplitude = params
    path = DOCS / "single_neuron_314900022_balanced_v1.md"
    with path.open("w", encoding="utf-8") as f:
        f.write("# Balanced-калибровка одиночного нейрона 314900022\n\n")
        f.write("Цель прогона - проверить, можно ли текущей GLIF-математикой AxiEngine одновременно удержать пассивный отклик, реобазу, f-I кривую и отсутствие ложного молчания на активных sweep.\n\n")
        f.write("## Лучший найденный набор\n\n")
        f.write("| Параметр | Значение |\n|:---|:---|\n")
        f.write(f"| leak_shift | {leak_shift} |\n")
        f.write(f"| current_scale | {current_scale} |\n")
        f.write(f"| refractory_period | {refractory_period} ms |\n")
        f.write(f"| threshold | {threshold} mV |\n")
        f.write(f"| ahp_amplitude | {ahp_amplitude} mV |\n")
        f.write(f"| loss | {metrics['loss']:.4f} |\n\n")

        f.write("## Scoreboard\n\n")
        f.write("| Метрика | Значение |\n|:---|:---|\n")
        f.write(f"| Passive RMSE | {metrics['passive_rmse']:.4f} mV |\n")
        f.write(f"| Passive steady-state error | {metrics['passive_ss_err']:.4f} mV |\n")
        f.write(f"| f-I RMSE | {metrics['fi_rmse']:.4f} spikes |\n")
        f.write(f"| Bio rheobase | {metrics['bio_rheobase_pa']:.1f} pA |\n")
        f.write(f"| Sim rheobase | {metrics['sim_rheobase_pa']:.1f} pA |\n")
        f.write(f"| Rheobase error | {metrics['rheobase_error_pa']:.1f} pA |\n")
        f.write(f"| False silent sweeps | {metrics['false_silent_sweeps']} |\n")
        f.write(f"| False silent missing spikes | {metrics['false_silent_spikes']} |\n")
        f.write(f"| False positive sweeps | {metrics['false_positive_sweeps']} |\n")
        f.write(f"| False positive spikes | {metrics['false_positive_spikes']} |\n")
        f.write(f"| Subthreshold false spikes | {metrics['subthreshold_spikes']} |\n")
        f.write(f"| Latency MAE | {metrics['latency_mae']:.4f} ms |\n")
        f.write(f"| ISI MAE | {metrics['isi_mae']:.4f} ms |\n")
        f.write(f"| ISI adaptation error | {metrics['isi_adaptation_error']:.4f} |\n\n")

        f.write("## Candidate comparison\n\n")
        f.write("| Кандидат | loss | passive RMSE | f-I RMSE | sim rheobase | false silent | false positive |\n")
        f.write("|:---|---:|---:|---:|---:|---:|---:|\n")
        for name, item in candidate_rows:
            if item is None:
                f.write(f"| {name} | n/a | n/a | n/a | n/a | n/a | n/a |\n")
            else:
                f.write(
                    f"| {name} | {float(item['loss']):.4f} | {float(item['passive_rmse']):.4f} | "
                    f"{float(item['fi_rmse']):.4f} | {float(item['sim_rheobase_pa']):.1f} | "
                    f"{int(item['false_silent_sweeps'])} | {int(item['false_positive_sweeps'])} |\n"
                )
        f.write("\n")

        f.write("## Sweep table\n\n")
        f.write("| Sweep | pA | Bio spikes | Sim spikes | Error | Bio latency | Sim latency | Passive peak err | Passive SS err | Voltage RMSE |\n")
        f.write("|:---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n")
        for r in records:
            bio_lat = "n/a" if r["bio_latency_ms"] is None else f"{r['bio_latency_ms']:.2f}"
            sim_lat = "n/a" if r["sim_latency_ms"] is None else f"{r['sim_latency_ms']:.2f}"
            peak = "n/a" if r["passive_voltage_peak_error_mV"] is None else f"{r['passive_voltage_peak_error_mV']:.2f}"
            ss = "n/a" if r["passive_steady_state_error_mV"] is None else f"{r['passive_steady_state_error_mV']:.2f}"
            f.write(f"| {r['sweep_id']} | {r['stimulus_pa']:.1f} | {r['bio_spike_count']} | {r['sim_spike_count']} | {r['spike_count_error']} | {bio_lat} | {sim_lat} | {peak} | {ss} | {r['voltage_rmse_mV']:.2f} |\n")

        f.write("\n## Вывод\n\n")
        if metrics["rheobase_error_pa"] > 0 and metrics["false_silent_sweeps"] > 0:
            f.write("Balanced-прогон все еще показывает конфликт: параметры, которые держат пассивный отклик, не дают корректно стартовать спайкам на части биологически активных sweep. Это указывает не только на подбор конфига, но и на возможное ограничение текущей формулы мембраны/масштабирования тока.\n")
        else:
            f.write("Balanced-прогон удержал реобазу без ложного молчания. Следующий шаг - проверить переносимость этих правил на остальные seed-нейроны.\n")
    return path


def main():
    if not NWB_PATH.exists():
        raise FileNotFoundError(f"Cached NWB not found: {NWB_PATH}")

    rest_potential = -73
    sweeps = load_long_square_sweeps(NWB_PATH)
    print(f"Loaded {len(sweeps)} Long Square sweeps")
    rows, best, metrics, records = run_balanced_grid(rest_potential, sweeps)
    print(f"Best params: {best}")
    print(f"Best metrics: {metrics}")

    print(f"Saved {write_grid(rows)}")
    candidate_path, candidate_rows = write_candidate_comparison(rows)
    print(f"Saved {candidate_path}")
    print(f"Saved {write_best(best, metrics)}")
    print(f"Saved {write_sweeps(records)}")
    print(f"Saved {write_summary(best, metrics)}")
    print(f"Saved {write_json(records)}")
    print(f"Saved {write_report(best, metrics, records, candidate_rows)}")


if __name__ == "__main__":
    main()
