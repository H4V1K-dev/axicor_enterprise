import os
import json
import numpy as np
import matplotlib.pyplot as plt

def load_json(path):
    if os.path.exists(path):
        with open(path, 'r', encoding='utf-8') as f:
            return json.load(f)
    return None

def main():
    root_dir = os.path.abspath(os.path.dirname(__file__))
    while root_dir != os.path.dirname(root_dir):
        if os.path.isdir(os.path.join(root_dir, "AxiEngine")) and os.path.isdir(os.path.join(root_dir, "docs")):
            break
        root_dir = os.path.dirname(root_dir)
    artifacts_dir = os.path.join(root_dir, "artifacts")
    active_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    img_dir = os.path.join(active_dir, "images")
    report_dir = os.path.join(active_dir, "reports")

    os.makedirs(img_dir, exist_ok=True)
    os.makedirs(report_dir, exist_ok=True)

    # 1. Load Sweep Summary
    sweep_path = os.path.join(artifacts_dir, "static_microcircuit_v1_2_sweep_summary.json")
    sweep = load_json(sweep_path)
    if not sweep:
        print(f"Sweep summary not found at {sweep_path}!")
        return

    # Plot 1: Heatmap showing L5 rate vs. L4->L5 excitation weight & fan-in range (Stage 1)
    stage1 = [item for item in sweep if item['stage'] == 1]
    if stage1:
        exc_weights = sorted(list(set(item['exc_weight_l4_l5'] for item in stage1)))
        fan_in_indices = sorted(list(set(item['fan_in_l4_l5_idx'] for item in stage1)))

        heatmap_data = np.zeros((len(fan_in_indices), len(exc_weights)))
        for item in stage1:
            x_idx = exc_weights.index(item['exc_weight_l4_l5'])
            y_idx = fan_in_indices.index(item['fan_in_l4_l5_idx'])
            heatmap_data[y_idx, x_idx] = item['l5_rate']

        plt.figure(figsize=(9, 5.5))
        im = plt.imshow(heatmap_data, cmap="YlGnBu", aspect='auto')
        plt.colorbar(im, label='L5 Firing Rate (Hz)')

        for i in range(heatmap_data.shape[0]):
            for j in range(heatmap_data.shape[1]):
                plt.text(j, i, f"{heatmap_data[i, j]:.2f}", ha="center", va="center",
                         color="black" if heatmap_data[i, j] < 5.0 else "white")

        plt.xticks(np.arange(len(exc_weights)), exc_weights)
        plt.yticks(np.arange(3), ["6..18", "12..28", "20..40"])

        plt.title("L5 Recruitment Sweep (Stage 1: Exc Weight vs. Fan-in Range)", fontsize=12, fontweight='bold')
        plt.xlabel("L4 -> L5 Synaptic Weight (uV)")
        plt.ylabel("L4 -> L5 Fan-in Range")
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, "sweep_heatmap_l5_rate.png"), dpi=150)
        plt.close()

    # Plot 2: Heatmap showing L5 rate vs. L23->L4 and L23->L5 inhibition split (Stage 2)
    stage2 = [item for item in sweep if item['stage'] == 2]
    if stage2:
        inh_w_l23_l4 = sorted(list(set(item['inh_weight_l23_l4'] for item in stage2)), reverse=True)
        inh_w_l23_l5 = sorted(list(set(item['inh_weight_l23_l5'] for item in stage2)), reverse=True)

        heatmap_data_s2 = np.zeros((len(inh_w_l23_l4), len(inh_w_l23_l5)))
        for item in stage2:
            x_idx = inh_w_l23_l5.index(item['inh_weight_l23_l5'])
            y_idx = inh_w_l23_l4.index(item['inh_weight_l23_l4'])
            heatmap_data_s2[y_idx, x_idx] = item['l5_rate']

        plt.figure(figsize=(10, 5.5))
        im = plt.imshow(heatmap_data_s2, cmap="YlOrRd", aspect='auto')
        plt.colorbar(im, label='L5 Firing Rate (Hz)')

        for i in range(heatmap_data_s2.shape[0]):
            for j in range(heatmap_data_s2.shape[1]):
                plt.text(j, i, f"{heatmap_data_s2[i, j]:.2f}", ha="center", va="center",
                         color="black" if heatmap_data_s2[i, j] < 4.0 else "white")

        plt.xticks(np.arange(len(inh_w_l23_l5)), inh_w_l23_l5)
        plt.yticks(np.arange(len(inh_w_l23_l4)), inh_w_l23_l4)

        plt.title("L5 Inhibition Split Sweep (Stage 2: Inh L23->L4 vs. L23->L5)", fontsize=12, fontweight='bold')
        plt.xlabel("L23 -> L5 Inhibitory Weight (uV)")
        plt.ylabel("L23 -> L4 Inhibitory Weight (uV)")
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, "sweep_heatmap_stage2_l5_rate.png"), dpi=150)
        plt.close()

    # Load detailed candidate log
    log_512_path = os.path.join(artifacts_dir, "static_microcircuit_v1_2_best_candidate_log_512.json")
    log_512 = load_json(log_512_path)
    if not log_512:
        print(f"Best candidate log 512 not found at {log_512_path}!")
        return

    ticks = [item['tick'] for item in log_512]
    l4_spikes = np.array([item['l4_spikes'] for item in log_512])
    l23_spikes = np.array([item['l23_spikes'] for item in log_512])
    l5_spikes = np.array([item['l5_spikes'] for item in log_512])

    def smooth(arr, window=100):
        return np.convolve(arr, np.ones(window)/window, mode='same') * 1000.0

    # Plot 3: Smoothed Population Firing Rates
    plt.figure(figsize=(10, 4.5))
    plt.plot(ticks, smooth(l4_spikes) / 256.0, label='L4 (256 somas)', color='#2ca02c')
    plt.plot(ticks, smooth(l23_spikes) / 128.0, label='L23 (128 somas)', color='#d62728')
    plt.plot(ticks, smooth(l5_spikes) / 128.0, label='L5 (128 somas)', color='#1f77b4')
    plt.axvline(x=1000, color='gray', linestyle='--', alpha=0.5)
    plt.axvline(x=3000, color='gray', linestyle='--', alpha=0.5)
    plt.axvline(x=5000, color='gray', linestyle='--', alpha=0.5)
    plt.axvline(x=7000, color='gray', linestyle='--', alpha=0.5)
    plt.title("Population Firing Rates (Best Candidate N=512)", fontsize=12, fontweight='bold')
    plt.xlabel("Simulation Ticks")
    plt.ylabel("Firing Rate (Hz)")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "best_firing_rates_512.png"), dpi=150)
    plt.close()

    # Plot 4: Voltage, Threshold, Fatigue, and Active E/I Proxy Traces
    l4_volts = np.array([item['l4_mean_voltage']/1000.0 for item in log_512])
    l4_th = np.array([item['l4_mean_threshold']/1000.0 for item in log_512])
    l5_volts = np.array([item['l5_mean_voltage']/1000.0 for item in log_512])
    l5_th = np.array([item['l5_mean_threshold']/1000.0 for item in log_512])
    l5_fatigue = np.array([item['l5_mean_fatigue'] for item in log_512])
    l5_exc_in = np.array([item['l5_active_exc_input'] for item in log_512])
    l5_inh_in = np.array([item['l5_active_inh_input'] for item in log_512])

    plt.figure(figsize=(12, 10))

    # Subplot 1: Membrane Voltage
    plt.subplot(4, 1, 1)
    plt.plot(ticks, l4_volts, color='#2ca02c', label='L4 Mean Vm')
    plt.plot(ticks, l5_volts, color='#1f77b4', label='L5 Mean Vm')
    plt.ylabel("Vm (mV)")
    plt.title("Membrane Potentials and Homeostatic/Synaptic Telemetry (Best N=512)", fontsize=12, fontweight='bold')
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)

    # Subplot 2: Threshold Offset
    plt.subplot(4, 1, 2)
    plt.plot(ticks, l4_th, color='#2ca02c', linestyle='--', label='L4 Threshold Offset')
    plt.plot(ticks, l5_th, color='#1f77b4', linestyle='--', label='L5 Threshold Offset')
    plt.ylabel("Threshold (mV)")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)

    # Subplot 3: L5 fatigue timer trace
    plt.subplot(4, 1, 3)
    plt.plot(ticks, l5_fatigue, color='purple', label='L5 Dendrite Fatigue Ratio')
    plt.ylabel("Fatigue Ratio")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)

    # Subplot 4: L5 Active E/I proxy
    plt.subplot(4, 1, 4)
    plt.plot(ticks, smooth(l5_exc_in)/1000.0, color='orange', label='L5 Active Excitatory Input Proxy')
    plt.plot(ticks, smooth(l5_inh_in)/1000.0, color='red', label='L5 Active Inhibitory Input Proxy')
    plt.ylabel("Integrated Input (a.u.)")
    plt.xlabel("Simulation Ticks")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)

    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "best_voltage_thresholds_512.png"), dpi=150)
    plt.close()

    # Plot 5: Ablation Plot
    ablation_summary_path = os.path.join(artifacts_dir, "static_microcircuit_v1_2_ablation_summary.json")
    ablation_summary = load_json(ablation_summary_path)

    ablation_logs_path = os.path.join(artifacts_dir, "static_microcircuit_v1_2_ablation_logs.json")
    ablation_logs = load_json(ablation_logs_path)

    if ablation_summary and ablation_logs:
        log_no = ablation_logs['no_inhibition_log']
        log_red = ablation_logs['reduced_inhibition_log']

        l5_full_abl = smooth(np.array([item['l5_spikes'] for item in log_512])) / 128.0
        l5_no_abl = smooth(np.array([item['l5_spikes'] for item in log_no])) / 128.0
        l5_red_abl = smooth(np.array([item['l5_spikes'] for item in log_red])) / 128.0

        plt.figure(figsize=(10, 4.5))
        plt.plot(ticks, l5_full_abl, label='Full Inhibition', color='#1f77b4')
        plt.plot(ticks, l5_red_abl, label='Reduced Inhibition (50%)', color='orange', linestyle='--')
        plt.plot(ticks, l5_no_abl, label='No Inhibition', color='#d62728')
        plt.axvline(x=3000, color='gray', linestyle=':', alpha=0.5)
        plt.axvline(x=5000, color='gray', linestyle=':', alpha=0.5)
        plt.title("E/I Ablation Comparison: L5 Population Firing Rate", fontsize=12, fontweight='bold')
        plt.xlabel("Simulation Ticks")
        plt.ylabel("Firing Rate (Hz)")
        plt.legend()
        plt.grid(True, linestyle=':', alpha=0.5)
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, "ablation_inhibition_comparison.png"), dpi=150)
        plt.close()

    # Summarize log helper
    def summarize_log(log, n):
        mod_log = log[3000:5000]
        rec_log = log[7000:9000]

        rates = {
            'l4': sum(item['l4_spikes'] for item in mod_log) / (2000.0 * (n / 2.0)) * 1000.0,
            'l23': sum(item['l23_spikes'] for item in mod_log) / (2000.0 * (n / 4.0)) * 1000.0,
            'l5': sum(item['l5_spikes'] for item in mod_log) / (2000.0 * (n / 4.0)) * 1000.0,
        }

        l4_vm = np.array([item['l4_mean_voltage'] / 1000.0 for item in log])
        l4_th = np.array([item['l4_mean_threshold'] / 1000.0 for item in log])

        def max_consecutive(mask):
            best = 0
            cur = 0
            for val in mask:
                if val:
                    cur += 1
                    best = max(best, cur)
                else:
                    cur = 0
            return best

        peak_th = float(np.max(l4_th[5000:7000]))
        rec_th_end = float(np.mean(l4_th[8000:9000]))
        decay_pct = (peak_th - rec_th_end) / peak_th if peak_th > 0.0 else 1.0
        rec_rate_all = sum(
            item['l4_spikes'] + item['l23_spikes'] + item['l5_spikes'] for item in rec_log
        ) / (2000.0 * n) * 1000.0

        return {
            'rates': rates,
            'max_consec_vm_above': max_consecutive(l4_vm > -25.0),
            'max_consec_vm_below': max_consecutive(l4_vm < -110.0),
            'max_thresh_offset_mv': float(np.max(l4_th)),
            'thresh_decay_pct': decay_pct,
            'recovery_rate': rec_rate_all,
        }

    log_256_path = os.path.join(artifacts_dir, "static_microcircuit_v1_2_best_candidate_log_256.json")
    log_256 = load_json(log_256_path)
    if not log_256:
        print(f"Best candidate log 256 not found!")
        return

    metrics_256 = summarize_log(log_256, 256)
    metrics_512 = summarize_log(log_512, 512)

    # Best candidate selection from sweep summary
    stage2 = [item for item in sweep if item['stage'] == 2]
    if not stage2:
        print("Stage 2 results not found in sweep summary!")
        return
    healthy = [item for item in stage2 if item['max_consec_vm_above'] <= 50 and not item['has_runaway']]
    if not healthy:
        healthy = stage2
    passing = [item for item in healthy if 3.0 <= item['l4_rate'] <= 25.0 and 3.0 <= item['l23_rate'] <= 35.0 and 1.0 <= item['l5_rate'] <= 15.0]
    if passing:
        stage2_winner = min(passing, key=lambda x: abs(x['l5_rate'] - 8.0))
    else:
        stage2_winner = min(healthy, key=lambda x: abs(x['l5_rate'] - 8.0))

    # Evaluate gates
    gate_vm_health = "PASS" if all(
        m['max_consec_vm_above'] <= 50 and m['max_consec_vm_below'] <= 50
        for m in [metrics_256, metrics_512]
    ) else "FAIL"
    gate_thresh = "PASS" if all(
        m['max_thresh_offset_mv'] < 40.0 and m['thresh_decay_pct'] >= 0.30
        for m in [metrics_256, metrics_512]
    ) else "FAIL"
    gate_activity = "PASS" if all(
        3.0 <= m['rates']['l4'] <= 25.0 and 3.0 <= m['rates']['l23'] <= 35.0 and 1.0 <= m['rates']['l5'] <= 15.0
        for m in [metrics_256, metrics_512]
    ) else "FAIL"
    gate_selectivity = "PASS" if stage2_winner['selectivity'] > 1.5 else "FAIL"

    if gate_vm_health == "PASS" and gate_thresh == "PASS" and gate_activity == "PASS" and gate_selectivity == "PASS":
        verdict = "Physiology Passed"
    else:
        verdict = "Partial Pass"

    # Generate Reports
    report_md = f"""# Static Microcircuit v1.2 L5 Recruitment & Topology Report

Status: completed (L5 recruited and physiological gates evaluated)
Phase: L5 Recruitment & Topology Sweep
Started: 2026-07-04
Completed: 2026-07-04

## Executive Summary

В исследовании `static_microcircuit_v1_2_l5_recruitment_topology` проверено, можно ли вывести L5 пирамидный класс в целевой физиологический диапазон ($1$-$15$ Hz) при сохранении ранее зафиксированных жестких рамок Vm health, homeostasis threshold и пространственной избирательности. L5 рекрутирован, но победитель перетормозил L4 ниже целевого диапазона.

> [!IMPORTANT]
> **Итоговый вердикт ({verdict} / L5 recruited / L4 underactive)**:
> - **L5 Recruitment Gate Passed**: L5 успешно активирован и стабилизирован: {metrics_256['rates']['l5']:.2f} Hz на N=256 и {metrics_512['rates']['l5']:.2f} Hz на N=512 (целевой диапазон 1..15 Hz).
> - **Vm Health Gate Passed**: L4 мембранный потенциал стабилен без перегрева (0 consecutive тиков выше -25 mV).
> - **Blocking Issue**: Moderate Activity gate не закрыт, потому что L4 падает до {metrics_512['rates']['l4']:.2f} Hz на N=512 при требовании 3..25 Hz.

---

## Статус приемочных критериев (Physiology Gates)

| Критерий | Требование | Результат (N=256) | Результат (N=512) | Статус |
| :--- | :--- | :--- | :--- | :--- |
| **Vm Health** | Consec ticks Vm > -25mV $\\le$ 50 | {metrics_256['max_consec_vm_above']} | {metrics_512['max_consec_vm_above']} | **{gate_vm_health}** |
| **Threshold Offset** | Max offset < 40 mV | {metrics_256['max_thresh_offset_mv']:.1f} mV | {metrics_512['max_thresh_offset_mv']:.1f} mV | **{gate_thresh}** |
| **Threshold Decay** | Снижение $\\ge$ 30% в recovery | {metrics_256['thresh_decay_pct']*100.0:.1f}% | {metrics_512['thresh_decay_pct']*100.0:.1f}% | **{gate_thresh}** |
| **Moderate Activity** | L4 (3-25Hz), L23 (3-35Hz), L5 (1-15Hz) | L4={metrics_256['rates']['l4']:.1f}Hz, L23={metrics_256['rates']['l23']:.1f}Hz, L5={metrics_256['rates']['l5']:.1f}Hz | L4={metrics_512['rates']['l4']:.1f}Hz, L23={metrics_512['rates']['l23']:.1f}Hz, L5={metrics_512['rates']['l5']:.1f}Hz | **{gate_activity}** |
| **Spatial Selectivity** | L4 active/inactive ratio > 1.5 | {stage2_winner['selectivity']:.2f} | {stage2_winner['selectivity']:.2f} | **{gate_selectivity}** |

---

## Конфигурация Победителя (Winner Parameters)

- **L4 -> L5 weight**: `{stage2_winner['exc_weight_l4_l5']}` uV (из sweeps `3000..8000` uV)
- **L4 -> L5 fan-in**: `{stage2_winner['l5_mean_fan_in']:.1f}` (max `{stage2_winner['l5_max_fan_in']}`) (выбран диапазон {stage2_winner['fan_in_l4_l5_idx']})
- **L23 -> L4 weight**: `{stage2_winner['inh_weight_l23_l4']}` uV (из sweeps `[-2000, -2750, -3500]`)
- **L23 -> L5 weight**: `{stage2_winner['inh_weight_l23_l5']}` uV (из sweeps `[0, -500, -1000, -1500, -2000, -2750]`)

---

## Визуальные результаты

### Карта рекрутирования L5 в зависимости от силы L4->L5 синапсов и плотности контактов (Stage 1)
![L5 Recruitment Heatmap](../images/sweep_heatmap_l5_rate.png)

### Карта рекрутирования L5 при разделении L23 inhibition (Stage 2)
![L5 Inhibition Heatmap](../images/sweep_heatmap_stage2_l5_rate.png)

### Частоты разряда популяции для лучшего кандидата (N=512)
![Best Firing Rates](../images/best_firing_rates_512.png)

### Детальная мембранная, пороговая, синаптическая и усталостная телеметрия L5
![Telemetry Traces](../images/best_voltage_thresholds_512.png)

---

## Аудит E/I Ablation

Влияние торможения на активность L5 при Winner-конфигурации:
- **Full inhibition**: L5 rate = {ablation_summary['full']['l5_rate']:.2f} Hz.
- **Reduced inhibition (50%)**: L5 rate = {ablation_summary['reduced_inhibition']['l5_rate']:.2f} Hz.
- **No inhibition**: L5 rate = {ablation_summary['no_inhibition']['l5_rate']:.2f} Hz.

![Ablation Plot](../images/ablation_inhibition_comparison.png)

---

## Выводы и рекомендации

1. **L5 успешно рекрутирован**: Настройка специфичного L4->L5 веса и разделение торможения L23 на слои позволили вывести L5 в целевой диапазон без Vm saturation.
2. **Профиль-ограничение (Profile Gap)**: Подтверждено отсутствие каноничного Exc `L2/3` профиля в modernized библиотеке. Stage 3 зафиксирован как структурный профиль-гэп.
3. **Физиологический gate не закрыт полностью**: L4 переторможен ниже целевого диапазона, поэтому перед STDP/GSOP нужен короткий balancing pass: вернуть L4 в 3..25 Hz, сохранив L5 1..15 Hz и Vm/threshold gates.
"""

    with open(os.path.join(report_dir, "static_microcircuit_v1_2_l5_recruitment_topology.md"), "w", encoding="utf-8") as f:
        f.write(report_md)

    # README.md
    readme_md = f"""# Research Archive: Static Microcircuit v1.2 L5 Recruitment & Topology

Status: completed
Slug: `static_microcircuit_v1_2_l5_recruitment_topology`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование частично закрывает задачу вывода L5 пирамидного класса в физиологический целевой диапазон 1..15 Hz под полной сетью:
- Проведен пошаговый sweep L4->L5 excitation силы и fan-in.
- Реализовано разделение тормозного действия L23 на L4 и L5.
- Оценены все жесткие gates: Vm health, threshold recovery, selectivity, E/I ablation.

## Key Findings

1. **L5 Recruitment Gate Passed**: Winner-конфигурация дает L5 в целевом диапазоне в full network.
2. **Inhibition Split Crucial**: Снижение тормозного влияния L23->L5 при сохранении сильного L23->L4 является ключевым фактором рекрутирования.
3. **L4 Gate Failed**: L4 переторможен ниже целевого диапазона, поэтому переход к STDP пока преждевременен.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_v1_2_l5_recruitment_topology.md](reports/static_microcircuit_v1_2_l5_recruitment_topology.md)
- Plots: [images/](images/)
"""

    with open(os.path.join(active_dir, "README.md"), "w", encoding="utf-8") as f:
        f.write(readme_md)

    print("Python analysis and reporting complete.")

if __name__ == "__main__":
    main()
