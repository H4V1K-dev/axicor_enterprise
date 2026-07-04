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
    root_dir = os.getcwd()
    artifacts_dir = os.path.join(root_dir, "artifacts")
    active_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    img_dir = os.path.join(active_dir, "images")
    report_dir = os.path.join(active_dir, "reports")

    os.makedirs(img_dir, exist_ok=True)
    os.makedirs(report_dir, exist_ok=True)

    # Load summary results
    summary_path = os.path.join(artifacts_dir, "static_microcircuit_scale_up_summary.json")
    summary = load_json(summary_path)
    if not summary:
        print(f"Summary file not found at {summary_path}!")
        return

    # 1. Runtime Scaling Plot
    sizes = [item['N'] for item in summary]
    avg_tick_times = [item['avg_tick_time_us'] for item in summary]
    init_times = [item['init_time_ms'] for item in summary]

    plt.figure(figsize=(10, 5))
    plt.subplot(1, 2, 1)
    plt.loglog(sizes, avg_tick_times, 'o-', color='purple', linewidth=2)
    plt.title("Avg Simulation Tick Duration", fontsize=11, fontweight='bold')
    plt.xlabel("Network Size (N)")
    plt.ylabel("Avg Tick Duration (us)")
    plt.grid(True, which="both", ls=":", alpha=0.5)

    plt.subplot(1, 2, 2)
    plt.loglog(sizes, init_times, 's-', color='teal', linewidth=2)
    plt.title("Initialization Time", fontsize=11, fontweight='bold')
    plt.xlabel("Network Size (N)")
    plt.ylabel("Init Time (ms)")
    plt.grid(True, which="both", ls=":", alpha=0.5)
    
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "perf_runtime_scaling.png"), dpi=150)
    plt.close()

    # Load and process logs for N = 128, 256, 512, 1024
    log_sizes = [128, 256, 512, 1024]
    
    gate_results = {}
    
    for n in log_sizes:
        log_path = os.path.join(artifacts_dir, f"static_microcircuit_scale_up_log_{n}.json")
        conn_path = os.path.join(artifacts_dir, f"static_microcircuit_scale_up_connectivity_{n}.json")
        
        log = load_json(log_path)
        conn = load_json(conn_path)
        
        if not log or not conn:
            continue
            
        ticks = [item['tick'] for item in log]
        
        # Plot 2: Firing Rates (smoothed over 100 ticks)
        l4_spikes = np.array([item['l4_spikes'] for item in log])
        l23_spikes = np.array([item['l23_spikes'] for item in log])
        l5_spikes = np.array([item['l5_spikes'] for item in log])
        
        def smooth(arr, window=100):
            return np.convolve(arr, np.ones(window)/window, mode='same') * 1000.0
            
        plt.figure(figsize=(12, 5))
        plt.plot(ticks, smooth(l4_spikes) / (n/2), label='L4', color='#2ca02c')
        plt.plot(ticks, smooth(l23_spikes) / (n/4), label='L23', color='#d62728')
        plt.plot(ticks, smooth(l5_spikes) / (n/4), label='L5', color='#1f77b4')
        
        if n <= 512:
            plt.axvline(x=1000, color='gray', linestyle='--', alpha=0.5)
            plt.axvline(x=3000, color='gray', linestyle='--', alpha=0.5)
            plt.axvline(x=5000, color='gray', linestyle='--', alpha=0.5)
            plt.axvline(x=7000, color='gray', linestyle='--', alpha=0.5)
        else:
            plt.axvline(x=250, color='gray', linestyle='--', alpha=0.5)
            plt.axvline(x=500, color='gray', linestyle='--', alpha=0.5)
            plt.axvline(x=750, color='gray', linestyle='--', alpha=0.5)
            
        plt.title(f"Smoothed Firing Rates by Layer (N = {n})", fontsize=12, fontweight='bold')
        plt.xlabel("Simulation Ticks")
        plt.ylabel("Firing Rate (Hz)")
        plt.legend()
        plt.grid(True, linestyle=':', alpha=0.5)
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, f"firing_rates_{n}.png"), dpi=150)
        plt.close()
        
        # Plot 3: Voltages and Threshold offsets
        l4_volts = np.array([item['l4_mean_voltage']/1000.0 for item in log])
        l4_th = np.array([item['l4_mean_threshold']/1000.0 for item in log])
        
        plt.figure(figsize=(12, 5))
        plt.subplot(2, 1, 1)
        plt.plot(ticks, l4_volts, color='#2ca02c', label='L4 Mean Vm')
        plt.ylabel("Vm (mV)")
        plt.title(f"L4 Mean Voltage and Threshold Offset (N = {n})", fontsize=11, fontweight='bold')
        plt.grid(True, linestyle=':', alpha=0.5)
        
        plt.subplot(2, 1, 2)
        plt.plot(ticks, l4_th, color='#2ca02c', linestyle='--', label='L4 Threshold Offset')
        plt.ylabel("Threshold Offset (mV)")
        plt.xlabel("Simulation Ticks")
        plt.grid(True, linestyle=':', alpha=0.5)
        
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, f"voltage_thresholds_{n}.png"), dpi=150)
        plt.close()
        
        # Plot 4: Fatigue Traces
        l4_fatigue = [item['l4_mean_fatigue'] for item in log]
        l23_fatigue = [item['l23_mean_fatigue'] for item in log]
        l5_fatigue = [item['l5_mean_fatigue'] for item in log]
        
        plt.figure(figsize=(12, 4.5))
        plt.plot(ticks, l4_fatigue, label='L4', color='#2ca02c')
        plt.plot(ticks, l23_fatigue, label='L23', color='#d62728')
        plt.plot(ticks, l5_fatigue, label='L5', color='#1f77b4')
        plt.title(f"Dendritic Fatigue (timer / capacity) (N = {n})", fontsize=12, fontweight='bold')
        plt.xlabel("Simulation Ticks")
        plt.ylabel("Fatigue Ratio")
        plt.legend()
        plt.grid(True, linestyle=':', alpha=0.5)
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, f"fatigue_{n}.png"), dpi=150)
        plt.close()

        # Plot 5: Fan-in Histogram
        fan_ins = [item['fan_in'] for item in conn['neurons']]
        plt.figure(figsize=(8, 4))
        plt.hist(fan_ins, bins=np.arange(min(fan_ins)-0.5, max(fan_ins)+1.5, 1.0), rwidth=0.8, color='skyblue', edgecolor='black')
        plt.title(f"Fan-In Distribution Histogram (N = {n})", fontsize=12, fontweight='bold')
        plt.xlabel("Number of Synapses per Neuron")
        plt.ylabel("Neuron Count")
        plt.grid(True, axis='y', linestyle=':', alpha=0.5)
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, f"fan_in_histogram_{n}.png"), dpi=150)
        plt.close()

        # --- EVALUATE SANITY GATES (N <= 512) ---
        if n <= 512:
            reg3_log = log[3000:5000] # moderate Poisson
            reg5_log = log[7000:9000] # recovery

            # Rates under moderate input
            r3_l4 = (sum(item['l4_spikes'] for item in reg3_log) / (2000.0 * (n/2))) * 1000.0
            r3_l23 = (sum(item['l23_spikes'] for item in reg3_log) / (2000.0 * (n/4))) * 1000.0
            r3_l5 = (sum(item['l5_spikes'] for item in reg3_log) / (2000.0 * (n/4))) * 1000.0

            # 1. Complete Silence Check
            gate_silence = r3_l4 > 0.1 and r3_l23 > 0.1 and r3_l5 > 0.1

            # 2. Runaway Check
            has_runaway = any(item['runaway_flag'] for item in reg3_log)
            gate_runaway = not has_runaway

            # 3. Vm Gating Check
            # check if L4 mean voltage is > -25 mV for more than 100 consecutive ticks
            l4_vm_series = np.array([item['l4_mean_voltage']/1000.0 for item in log])
            vm_above_limit = l4_vm_series > -25.0
            consec_vm_ticks = 0
            max_consec_vm = 0
            for val in vm_above_limit:
                if val:
                    consec_vm_ticks += 1
                    max_consec_vm = max(max_consec_vm, consec_vm_ticks)
                else:
                    consec_vm_ticks = 0
            gate_vm_health = max_consec_vm < 100

            # 4. Threshold Stability & Decay
            # threshold offset must decay during recovery (first half of recovery vs second half)
            rec_th_first = np.mean([item['l4_mean_threshold'] for item in log[7000:8000]])
            rec_th_second = np.mean([item['l4_mean_threshold'] for item in log[8000:9000]])
            gate_threshold_decay = rec_th_second < rec_th_first or rec_th_second < 1000.0 # close to zero

            gate_results[n] = {
                "silence": "PASS" if gate_silence else "FAIL",
                "runaway": "PASS" if gate_runaway else "FAIL",
                "vm_health": "PASS" if gate_vm_health else "FAIL",
                "thresh_decay": "PASS" if gate_threshold_decay else "FAIL",
                "r3_rates": f"L4={r3_l4:.1f}Hz, L23={r3_l23:.1f}Hz, L5={r3_l5:.1f}Hz",
                "max_consec_vm_above_limit": max_consec_vm
            }

    # Generate Report MD
    gate_table = ""
    for n, res in gate_results.items():
        gate_table += f"| **N = {n}** | {res['silence']} ({res['r3_rates']}) | {res['runaway']} | {res['vm_health']} (max consec={res['max_consec_vm_above_limit']}) | {res['thresh_decay']} |\n"

    # Connectivity and perf summary
    perf_table = ""
    for item in summary:
        perf_table += f"| {item['N']} | {item['init_time_ms']} ms | {item['avg_tick_time_us']:.1f} us | {item['edges_count']} |\n"

    report_md = f"""# Static Microcircuit Scale-Up Physiology Report v1

Status: completed (scale-up stability evaluated)
Phase: Network Scale-Up & Performance Load Gating
Started: 2026-07-04
Completed: 2026-07-04

## Executive Summary

В исследовании `static_microcircuit_scale_up_v1` проведена оценка стабильности и производительности статической микросети (L4/L2-3/L5) при увеличении числа нейронов от 128 до 1,000,000 нейронов на однопоточном CPU ядре AxiEngine.

> [!WARNING]
> **Итоговый вердикт (Performance Passed / Physiology Inconclusive)**: CPU load-test успешно выдерживает масштабирование до 1,000,000 нейронов с заполнением всех 128 дендритов (128 миллионов синапсов) в release-сборке. Физиология сетей N <= 512 пока не проходит hard gates: L5 почти молчит на N=128/256, а Vm health падает на N=128/256/512 из-за длительного подъема L4 mean Vm выше -25 mV в structured-фазе. Переход к plasticity преждевременен.

---

## Статус приемочных критериев (Physiology Gates)

| Масштаб | Silence Gate | Runaway Gate | Vm Health (No saturation > -25mV) | Threshold Decay (Recovery) |
| :--- | :--- | :--- | :--- | :--- |
{gate_table}

---

## Производительность и масштабирование (Performance Benchmarks)

| Размер сети (N) | Время инициализации | Среднее время тика | Число синапсов |
| :--- | :--- | :--- | :--- |
{perf_table}

### Оценка производительности
- **Release-only benchmark**: Числа производительности валидны для release-сборки; debug-сборка на больших масштабах не является репрезентативной.
- **Load Test 1,000,000 нейронов**: 1M прогон является perf/load-only сценарием на 10 тиков с искусственным заполнением 128 дендритных слотов. Он подтверждает отсутствие OOM/переполнений и дает оценку throughput, но не является физиологическим экспериментом.

---

## Визуальные результаты

### Зависимость производительности от размера сети
![Perf Scaling](../images/perf_runtime_scaling.png)

### Частота разряда (Firing Rate) популяций во времени

#### N = 128
![Rates 128](../images/firing_rates_128.png)

#### N = 256
![Rates 256](../images/firing_rates_256.png)

#### N = 512
![Rates 512](../images/firing_rates_512.png)

### Динамика мембранных потенциалов L4 и порогов гомеостаза

#### N = 256
![Vm 256](../images/voltage_thresholds_256.png)

#### N = 512
![Vm 512](../images/voltage_thresholds_512.png)

### Распределение Fan-in (Гистограмма плотности соединений)

#### N = 256
![Fan-in 256](../images/fan_in_histogram_256.png)

---

## Выводы и рекомендации

1. **Performance scale-up подтвержден**: CPU backend воспроизводимо проходит 128/256/512/1024 физиологические прогоны и 10k/100k/1M load-only прогоны.
2. **Физиология inconclusive**: Runaway не обнаружен, но silence gate падает на N=128/256 из-за почти молчащего L5, а Vm health падает на N=128/256/512.
3. **Recovery частичный**: После отключения входа firing rate падает почти до нуля, но L4 `threshold_offset` остается высоким к концу окна, поэтому recovery нужно измерять длиннее или мягче нагружать вход.
4. **Следующий шаг**: перед GSOP/STDP нужен `Static Microcircuit v1.1 Input Scale & E/I Ablation`: снизить входные веса/Poisson drive, добавить L23 ablation и количественно проверить Vm/threshold/fan-in/phase selectivity.
"""

    with open(os.path.join(report_dir, "static_microcircuit_scale_up_v1.md"), "w", encoding="utf-8") as f:
        f.write(report_md)

    # README.md
    readme_md = f"""# Research Archive: Static Microcircuit Scale-Up v1

Status: completed
Slug: `static_microcircuit_scale_up_v1`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование оценивает стабильность физиологии и производительность симулятора при масштабировании малой пространственной микросети от 128 до 1,000,000 нейронов.

## Key Findings

1. **Успешный release Load Test (1,000,000 нейронов)**: 10-тиковый perf/load-only сценарий со 128 миллионами синапсов запускается на CPU без OOM и переполнений.
2. **Физиология inconclusive**: N=128/256/512 не уходят в runaway, но Vm health падает, а L5 почти молчит на N=128/256.
3. **Переход к plasticity заблокирован**: нужен отдельный v1.1 прогон с input scaling, E/I ablation и жесткими Vm/threshold gates.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_scale_up_v1.md](reports/static_microcircuit_scale_up_v1.md)
- Plots: [images/](images/)
"""

    with open(os.path.join(active_dir, "README.md"), "w", encoding="utf-8") as f:
        f.write(readme_md)

    print("Python analysis and reporting complete.")

if __name__ == "__main__":
    main()
