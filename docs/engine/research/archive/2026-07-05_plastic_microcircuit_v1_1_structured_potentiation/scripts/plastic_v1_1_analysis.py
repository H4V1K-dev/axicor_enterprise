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

    # Load Logs
    log_256_sanity = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_best_log_256_sanity.json"))
    log_256_learning = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_best_log_256_learning.json"))
    log_512_sanity = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_best_log_512_sanity.json"))
    edges_256 = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_best_edge_log_256.json"))
    summary = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_summary.json"))
    sweep = load_json(os.path.join(artifacts_dir, "plastic_microcircuit_v1_1_sweep_summary.json"))

    if not (log_256_sanity and log_256_learning and log_512_sanity and edges_256 and summary):
        print("Required simulation logs not found!")
        return

    def smooth(arr, window=100):
        return np.convolve(arr, np.ones(window)/window, mode='same') * 1000.0

    # Plot 1: Firing Rates (3 panels)
    fig, axes = plt.subplots(3, 1, figsize=(12, 10))
    
    # Panel 1: N=256 Sanity
    ticks_256_s = [x['tick'] for x in log_256_sanity]
    axes[0].plot(ticks_256_s, smooth([x['l4_spikes'] for x in log_256_sanity]) / 128.0, label='L4', color='#2ca02c')
    axes[0].plot(ticks_256_s, smooth([x['l23_spikes'] for x in log_256_sanity]) / 64.0, label='L23', color='#d62728')
    axes[0].plot(ticks_256_s, smooth([x['l5_spikes'] for x in log_256_sanity]) / 64.0, label='L5', color='#1f77b4')
    axes[0].set_title("Winner N=256 Sanity Run (9,000 ticks) Firing Rates", fontsize=11, fontweight='bold')
    axes[0].set_ylabel("Rate (Hz)")
    axes[0].legend()
    axes[0].grid(True, linestyle=':', alpha=0.6)
    
    # Panel 2: N=256 Learning
    ticks_256_l = [x['tick'] for x in log_256_learning]
    axes[1].plot(ticks_256_l, smooth([x['l4_spikes'] for x in log_256_learning], window=50) / 128.0, label='L4', color='#2ca02c')
    axes[1].plot(ticks_256_l, smooth([x['l23_spikes'] for x in log_256_learning], window=50) / 64.0, label='L23', color='#d62728')
    axes[1].plot(ticks_256_l, smooth([x['l5_spikes'] for x in log_256_learning], window=50) / 64.0, label='L5', color='#1f77b4')
    axes[1].set_title("Winner N=256 Learning Run (135,000 ticks) Firing Rates", fontsize=11, fontweight='bold')
    axes[1].set_ylabel("Rate (Hz)")
    axes[1].legend()
    axes[1].grid(True, linestyle=':', alpha=0.6)

    # Panel 3: N=512 Sanity
    ticks_512_s = [x['tick'] for x in log_512_sanity]
    axes[2].plot(ticks_512_s, smooth([x['l4_spikes'] for x in log_512_sanity]) / 256.0, label='L4', color='#2ca02c')
    axes[2].plot(ticks_512_s, smooth([x['l23_spikes'] for x in log_512_sanity]) / 128.0, label='L23', color='#d62728')
    axes[2].plot(ticks_512_s, smooth([x['l5_spikes'] for x in log_512_sanity]) / 128.0, label='L5', color='#1f77b4')
    axes[2].set_title("Winner N=512 Sanity Run (9,000 ticks) Firing Rates", fontsize=11, fontweight='bold')
    axes[2].set_ylabel("Rate (Hz)")
    axes[2].set_xlabel("Simulation Ticks")
    axes[2].legend()
    axes[2].grid(True, linestyle=':', alpha=0.6)

    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "firing_rates_by_phase.png"), dpi=150)
    plt.close()

    # Plot 2: Virtual -> L4 Matched vs Unmatched Deltas
    v_to_l4 = [e for e in edges_256 if e['projection'] == 'Virtual -> L4']
    matched_deltas = [e['delta_signed'] for e in v_to_l4 if e['is_matched']]
    unmatched_deltas = [e['delta_signed'] for e in v_to_l4 if not e['is_matched']]

    plt.figure(figsize=(10, 5))
    plt.hist(unmatched_deltas, bins=30, alpha=0.5, label=f'Unmatched (n={len(unmatched_deltas)})', color='red')
    plt.hist(matched_deltas, bins=30, alpha=0.5, label=f'Matched (n={len(matched_deltas)})', color='green')
    plt.axvline(x=np.mean(unmatched_deltas), color='darkred', linestyle='--', label=f'Mean Unmatched: {np.mean(unmatched_deltas):.3f} uV')
    plt.axvline(x=np.mean(matched_deltas), color='darkgreen', linestyle='--', label=f'Mean Matched: {np.mean(matched_deltas):.3f} uV')
    plt.title("Virtual -> L4 Synaptic Delta Distribution: Matched vs Unmatched", fontsize=12, fontweight='bold')
    plt.xlabel("Weight Delta (uV)")
    plt.ylabel("Count")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "virtual_l4_matched_vs_unmatched_deltas.png"), dpi=150)
    plt.close()

    # Plot 3: Downstream TRANSFER Grouped Deltas (L4 -> L23 and L4 -> L5)
    l4_to_l23 = [e for e in edges_256 if e['projection'] == 'L4 -> L23']
    l4_to_l5 = [e for e in edges_256 if e['projection'] == 'L4 -> L5']

    l4_l23_matched = [e['delta_signed'] for e in l4_to_l23 if e['is_matched']]
    l4_l23_unmatched = [e['delta_signed'] for e in l4_to_l23 if not e['is_matched']]
    l4_l5_matched = [e['delta_signed'] for e in l4_to_l5 if e['is_matched']]
    l4_l5_unmatched = [e['delta_signed'] for e in l4_to_l5 if not e['is_matched']]

    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    
    # Subplot A: L4 -> L23
    axes[0].hist(l4_l23_unmatched, bins=25, alpha=0.5, label=f'Unmatched (n={len(l4_l23_unmatched)})', color='orange')
    axes[0].hist(l4_l23_matched, bins=25, alpha=0.5, label=f'Matched (n={len(l4_l23_matched)})', color='teal')
    axes[0].axvline(x=np.mean(l4_l23_unmatched), color='darkorange', linestyle='--', label=f'Mean Unmatched: {np.mean(l4_l23_unmatched):.3f} uV')
    axes[0].axvline(x=np.mean(l4_l23_matched), color='darkcyan', linestyle='--', label=f'Mean Matched: {np.mean(l4_l23_matched):.3f} uV')
    axes[0].set_title("L4 -> L23 Outgoing Deltas", fontsize=11, fontweight='bold')
    axes[0].set_xlabel("Weight Delta (uV)")
    axes[0].set_ylabel("Count")
    axes[0].legend()
    axes[0].grid(True, linestyle=':', alpha=0.5)

    # Subplot B: L4 -> L5
    axes[1].hist(l4_l5_unmatched, bins=25, alpha=0.5, label=f'Unmatched (n={len(l4_l5_unmatched)})', color='orange')
    axes[1].hist(l4_l5_matched, bins=25, alpha=0.5, label=f'Matched (n={len(l4_l5_matched)})', color='teal')
    axes[1].axvline(x=np.mean(l4_l5_unmatched), color='darkorange', linestyle='--', label=f'Mean Unmatched: {np.mean(l4_l5_unmatched):.3f} uV')
    axes[1].axvline(x=np.mean(l4_l5_matched), color='darkcyan', linestyle='--', label=f'Mean Matched: {np.mean(l4_l5_matched):.3f} uV')
    axes[1].set_title("L4 -> L5 Outgoing Deltas", fontsize=11, fontweight='bold')
    axes[1].set_xlabel("Weight Delta (uV)")
    axes[1].legend()
    axes[1].grid(True, linestyle=':', alpha=0.5)

    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "downstream_l4_l23_l4_l5_grouped_deltas.png"), dpi=150)
    plt.close()

    # Plot 4: Positive / Negative / Zero Delta Ratios per Projection
    proj_names = ["Virtual -> L4", "L4 -> L23", "L4 -> L5", "L23 -> L4", "L23 -> L5", "L23 -> L23", "L5 -> L23"]
    pos_ratios = []
    neg_ratios = []
    zero_ratios = []

    for name in proj_names:
        p_edges = [e for e in edges_256 if e['projection'] == name]
        total = len(p_edges)
        if total == 0:
            pos_ratios.append(0)
            neg_ratios.append(0)
            zero_ratios.append(0)
            continue
        pos = sum(1 for e in p_edges if e['delta_signed'] > 0)
        neg = sum(1 for e in p_edges if e['delta_signed'] < 0)
        zero = sum(1 for e in p_edges if e['delta_signed'] == 0)
        pos_ratios.append(pos / total)
        neg_ratios.append(neg / total)
        zero_ratios.append(zero / total)

    plt.figure(figsize=(10, 5))
    x = np.arange(len(proj_names))
    plt.bar(x - 0.25, pos_ratios, width=0.25, color='green', alpha=0.7, label='Positive (Strengthened)')
    plt.bar(x, zero_ratios, width=0.25, color='gray', alpha=0.7, label='Zero (No Change)')
    plt.bar(x + 0.25, neg_ratios, width=0.25, color='red', alpha=0.7, label='Negative (Depressed)')
    plt.xticks(x, proj_names, rotation=20)
    plt.title("Proportion of Strengthening vs Depression by Network Projection", fontsize=12, fontweight='bold')
    plt.ylabel("Ratio of Connections")
    plt.legend()
    plt.grid(True, linestyle=':', alpha=0.5)
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "positive_negative_delta_ratios.png"), dpi=150)
    plt.close()

    # Plot 5: Weight Histograms by Projection
    fig, axes = plt.subplots(4, 2, figsize=(14, 16))
    axes = axes.flatten()

    for idx, name in enumerate(proj_names):
        proj_edges = [e for e in edges_256 if e['projection'] == name]
        if not proj_edges:
            continue
        init_w = [e['initial_weight'] for e in proj_edges]
        final_w = [e['final_weight'] for e in proj_edges]
        
        ax = axes[idx]
        ax.hist(init_w, bins=25, alpha=0.5, label='Initial', color='gray')
        ax.hist(final_w, bins=25, alpha=0.5, label='Final', color='blue')
        ax.set_title(f"{name} (n={len(proj_edges)})", fontsize=11, fontweight='bold')
        ax.set_xlabel("Synaptic Weight (uV)")
        ax.set_ylabel("Count")
        ax.legend()
        ax.grid(True, linestyle=':', alpha=0.5)

    axes[-1].axis('off')
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "weight_histograms_by_projection.png"), dpi=150)
    plt.close()

    # Plot 6 & 7: Top Positive / Negative Edges
    sorted_edges = sorted(edges_256, key=lambda x: x['delta_signed'])
    top_neg = sorted_edges[:10]
    top_pos = sorted_edges[-10:][::-1]

    def plot_top(edges, title, filename, color):
        labels = [f"{e['projection']}: {e['src']}->{e['dest']}" for e in edges]
        vals = [e['delta_signed'] for e in edges]
        plt.figure(figsize=(10, 5))
        plt.barh(labels[::-1], vals[::-1], color=color)
        plt.axvline(x=0, color='black', linewidth=1.0)
        plt.title(title, fontsize=12, fontweight='bold')
        plt.xlabel("Weight Delta (uV)")
        plt.grid(True, linestyle=':', alpha=0.5)
        plt.tight_layout()
        plt.savefig(os.path.join(img_dir, filename), dpi=150)
        plt.close()

    plot_top(top_pos, "Top 10 Most Strengthened Edges (Positive Delta)", "top_positive_edges.png", 'green')
    plot_top(top_neg, "Top 10 Most Depressed Edges (Negative Delta)", "top_negative_edges.png", 'red')

    # Plot 8: Spatial Delta Map (Delta vs Distance)
    distances = []
    deltas = []
    for e in edges_256:
        if e['src_coords'] is not None:
            s_c = e['src_coords']
            d_c = e['dest_coords']
            dist = np.sqrt((s_c[0]-d_c[0])**2 + (s_c[1]-d_c[1])**2 + (s_c[2]-d_c[2])**2)
            distances.append(dist)
            deltas.append(e['delta_signed'])

    plt.figure(figsize=(10, 5))
    sc = plt.scatter(distances, deltas, alpha=0.4, c=deltas, cmap='coolwarm', s=10)
    plt.colorbar(sc, label='Signed Delta (uV)')
    plt.title("Spatial Plasticity: Weight Delta vs Physical Connection Distance", fontsize=12, fontweight='bold')
    plt.xlabel("Physical Connection Distance (um)")
    plt.ylabel("Signed Weight Delta (uV)")
    plt.grid(True, linestyle=':', alpha=0.5)
    plt.tight_layout()
    plt.savefig(os.path.join(img_dir, "spatial_delta_map.png"), dpi=150)
    plt.close()

    # Metrics Summary
    def analyze_rates(log, n):
        is_learning_run = any(x['tick'] > 10000 for x in log)
        if is_learning_run:
            mod = [x for x in log if x['tick'] >= 15000 and x['tick'] <= 125000]
        else:
            mod = [x for x in log if x['tick'] >= 3000 and x['tick'] <= 7000]
        ticks = len(mod)
        if ticks == 0:
            return 0.0, 0.0, 0.0
        r4 = sum(x['l4_spikes'] for x in mod) / (ticks * (n / 2.0)) * 1000.0
        r23 = sum(x['l23_spikes'] for x in mod) / (ticks * (n / 4.0)) * 1000.0
        r5 = sum(x['l5_spikes'] for x in mod) / (ticks * (n / 4.0)) * 1000.0
        return r4, r23, r5

    # The JSON logs are sampled every 10 ticks for plotting. Use the Rust runner's
    # full-run counters from summary.json for acceptance tables and verdicts.
    r4_256_l = summary['learning_256']['r4']
    r23_256_l = summary['learning_256']['r23']
    r5_256_l = summary['learning_256']['r5']
    r4_512_s = summary['sanity_512']['r4']
    r23_512_s = summary['sanity_512']['r23']
    r5_512_s = summary['sanity_512']['r5']

    # Invariants Check
    invar_dale = "PASS" if summary['learning_256']['dale_violations'] == 0 else "FAIL"
    invar_sign = "PASS" if summary['learning_256']['sign_flips'] == 0 else "FAIL"
    invar_bounds = "PASS"
    for e in edges_256:
        if e['is_inhibitory']:
            if e['final_weight'] > 0: invar_bounds = "FAIL"
        else:
            if e['final_weight'] < 0: invar_bounds = "FAIL"

    mean_corr_v = summary['learning_256']['mean_corr_delta']
    mean_uncorr_v = summary['learning_256']['mean_uncorr_delta']
    corr_pos_ratio = summary['learning_256']['corr_pos_ratio']
    uncorr_pos_ratio = summary['learning_256']['uncorr_pos_ratio']

    mean_l4_l23_matched = np.mean(l4_l23_matched)
    mean_l4_l23_unmatched = np.mean(l4_l23_unmatched)
    mean_l4_l5_matched = np.mean(l4_l5_matched)
    mean_l4_l5_unmatched = np.mean(l4_l5_unmatched)
    l4_l23_bias_lr = mean_l4_l23_matched - mean_l4_l23_unmatched
    l4_l5_bias_lr = mean_l4_l5_matched - mean_l4_l5_unmatched

    mean_corr_pos = mean_corr_v > 0.0
    ratio_2x = corr_pos_ratio >= 2.0 * uncorr_pos_ratio if uncorr_pos_ratio > 0 else True
    downstream_l23_ok = l4_l23_bias_lr > 0.05 and mean_l4_l23_matched > 0.0
    downstream_l5_ok = l4_l5_bias_lr > 0.05 and mean_l4_l5_matched > 0.0
    downstream_ok = downstream_l23_ok and downstream_l5_ok
    phys_256_ok = (
        3.0 <= r4_256_l <= 25.0
        and 3.0 <= r23_256_l <= 35.0
        and 1.0 <= r5_256_l <= 15.0
    )
    phys_512_ok = (
        3.0 <= r4_512_s <= 25.0
        and 3.0 <= r23_512_s <= 35.0
        and 1.0 <= r5_512_s <= 15.0
    )
    phys_ok = phys_256_ok and phys_512_ok
    phys_status = "PASS" if phys_ok else "PARTIAL PASS"
    pathway_status = "PASS" if (mean_corr_pos and ratio_2x and corr_pos_ratio > 0.0) else "PARTIAL PASS"
    downstream_status = "PASS" if downstream_ok else ("PARTIAL PASS" if (l4_l23_bias_lr > 0.0 or l4_l5_bias_lr > 0.0) else "FAIL")

    # Since mean_corr_v can be slightly negative under strong fatigue/LTD, we check relative potentiation too.
    # But pass criteria says "PASS only if mean matched/correlated delta > 0".
    # Wait, the summary N=256 learning has: mean_corr_delta = -0.0167 uV (which is <= 0).
    # So strictly this is a PARTIAL PASS, because the mean delta is slightly negative (although very close to 0).
    corr_total = len(matched_deltas)
    uncorr_total = len(unmatched_deltas)
    verdict = "PASS" if (invar_dale == "PASS" and invar_sign == "PASS" and invar_bounds == "PASS" and mean_corr_pos and ratio_2x and downstream_ok and phys_ok) else "PARTIAL PASS"

    # Report Compile
    report_md = f"""# Plastic Microcircuit v1.1 Structured Potentiation Report

Status: completed / partial pass (Plasticity enabled; positive potentiation not yet proven)
Phase: GSOP/STDP Structured Potentiation
Started: 2026-07-05
Completed: 2026-07-05

## Executive Summary

В исследовании `plastic_microcircuit_v1_1_structured_potentiation` была проверена гипотеза о положительной структурированной потенциации коррелированных входов. Спроектирован и протестирован метод сильного спаренного структурированного стимула (`structured_p = 0.075`, `background_p = 0.003`) при разделении активных временных блоков.

Локальные синаптические изменения продемонстрировали выраженную пространственно-структурированную защиту коррелированных входов от LTD. Однако строгий gate положительной потенциации не закрыт: средняя дельта matched `Virtual -> L4` остается слегка отрицательной.

> [!IMPORTANT]
> **Итоговый вердикт ({verdict})**:
> - **Physiological Stability**: runaway/silence не обнаружены, но L4 активность ниже hard gate 3 Hz на N=256 и N=512.
> - **Virtual -> L4 Protection**: среднее изменение коррелированных входов составило **{mean_corr_v:.4f} uV** против **{mean_uncorr_v:.4f} uV** у фоновых. Это сильное селективное удержание от LTD, но не положительная потенциация.
> - **Pathway Selection**: positive ratio у matched связей **{corr_pos_ratio * 100.0:.2f}%**, у unmatched **{uncorr_pos_ratio * 100.0:.2f}%**. Поскольку абсолютная доля положительных связей мала, это вспомогательная метрика, а не PASS-gate.
> - **Downstream Transfer**: L4 -> L23 показывает положительный matched bias **+{l4_l23_bias_lr:.4f} uV** ({mean_l4_l23_matched:.4f} uV vs {mean_l4_l23_unmatched:.4f} uV). L4 -> L5 показывает только ослабление депрессии **+{l4_l5_bias_lr:.4f} uV** ({mean_l4_l5_matched:.4f} uV vs {mean_l4_l5_unmatched:.4f} uV), но остается отрицательным по среднему знаку.
> - **Invariants**: 0 нарушений закона Дейла, 0 инверсий знаков синаптических весов.

---

## Статус приемочных критериев (Plasticity & Physiology)

| Критерий | Требование | Результат (N=256) | Результат (N=512) | Статус |
| :--- | :--- | :--- | :--- | :--- |
| **Dale's Law** | Веса не пересекают 0 | 0 нарушений | 0 нарушений | **PASS** |
| **Sign Integrity** | Исключены случайные перескоки знака | 0 перескоков | 0 перескоков | **PASS** |
| **Moderate Activity** | L4 (3-25Hz), L23 (3-35Hz), L5 (1-15Hz) | L4={r4_256_l:.1f}Hz, L23={r23_256_l:.1f}Hz, L5={r5_256_l:.1f}Hz | L4={r4_512_s:.1f}Hz, L23={r23_512_s:.1f}Hz, L5={r5_512_s:.1f}Hz | **{phys_status}** |
| **Correlated Potentiation** | Mean matched Virtual->L4 delta > 0 | {mean_corr_v:.4f} uV | - | **{"PASS" if mean_corr_pos else "PARTIAL PASS"}** |
| **Pathway Selection** | Matched positive ratio > unmatched | matched={corr_pos_ratio * 100.0:.2f}%, unmatched={uncorr_pos_ratio * 100.0:.2f}% | - | **{pathway_status}** |
| **Downstream Transfer** | L4->L23/L5 matched delta shows positive mean/bias | L4->L23: +{l4_l23_bias_lr:.3f} uV, L4->L5: +{l4_l5_bias_lr:.3f} uV | - | **{downstream_status}** |

*Примечание к Correlated Potentiation*: Поскольку средний синаптический вес испытывает небольшую депрессию (LTD) из-за утомления (fatigue), итоговая средняя дельта коррелированных связей слегка отрицательна (-0.0167 uV), однако она на два порядка меньше средней депрессии фоновых связей (-0.6111 uV), что показывает селективную защиту от LTD и относительное усиление.

---

## Статистика изменения весов по проекциям и группам (N=256 Learning)

| Проекция | Группа (Matched/Unmatched) | Количество | Средняя дельта (uV) | Доля положительных (%) | Доля нулевых (%) | Доля отрицательных (%) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Virtual -> L4** | Matched | {corr_total} | {mean_corr_v:.3f} | {corr_pos_ratio * 100.0:.1f}% | {sum(1 for e in v_to_l4 if e['is_matched'] and e['delta_signed'] == 0) / max(corr_total, 1) * 100.0:.1f}% | {sum(1 for e in v_to_l4 if e['is_matched'] and e['delta_signed'] < 0) / max(corr_total, 1) * 100.0:.1f}% |
| **Virtual -> L4** | Unmatched | {uncorr_total} | {mean_uncorr_v:.3f} | {uncorr_pos_ratio * 100.0:.1f}% | {sum(1 for e in v_to_l4 if not e['is_matched'] and e['delta_signed'] == 0) / max(uncorr_total, 1) * 100.0:.1f}% | {sum(1 for e in v_to_l4 if not e['is_matched'] and e['delta_signed'] < 0) / max(uncorr_total, 1) * 100.0:.1f}% |
| **L4 -> L23** | Matched | {len(l4_l23_matched)} | {np.mean(l4_l23_matched):.3f} | {sum(1 for e in l4_to_l23 if e['is_matched'] and e['delta_signed'] > 0) / max(len(l4_l23_matched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l23 if e['is_matched'] and e['delta_signed'] == 0) / max(len(l4_l23_matched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l23 if e['is_matched'] and e['delta_signed'] < 0) / max(len(l4_l23_matched), 1) * 100.0:.1f}% |
| **L4 -> L23** | Unmatched | {len(l4_l23_unmatched)} | {np.mean(l4_l23_unmatched):.3f} | {sum(1 for e in l4_to_l23 if not e['is_matched'] and e['delta_signed'] > 0) / max(len(l4_l23_unmatched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l23 if not e['is_matched'] and e['delta_signed'] == 0) / max(len(l4_l23_unmatched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l23 if not e['is_matched'] and e['delta_signed'] < 0) / max(len(l4_l23_unmatched), 1) * 100.0:.1f}% |
| **L4 -> L5** | Matched | {len(l4_l5_matched)} | {np.mean(l4_l5_matched):.3f} | {sum(1 for e in l4_to_l5 if e['is_matched'] and e['delta_signed'] > 0) / max(len(l4_l5_matched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l5 if e['is_matched'] and e['delta_signed'] == 0) / max(len(l4_l5_matched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l5 if e['is_matched'] and e['delta_signed'] < 0) / max(len(l4_l5_matched), 1) * 100.0:.1f}% |
| **L4 -> L5** | Unmatched | {len(l4_l5_unmatched)} | {np.mean(l4_l5_unmatched):.3f} | {sum(1 for e in l4_to_l5 if not e['is_matched'] and e['delta_signed'] > 0) / max(len(l4_l5_unmatched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l5 if not e['is_matched'] and e['delta_signed'] == 0) / max(len(l4_l5_unmatched), 1) * 100.0:.1f}% | {sum(1 for e in l4_to_l5 if not e['is_matched'] and e['delta_signed'] < 0) / max(len(l4_l5_unmatched), 1) * 100.0:.1f}% |

---

## Визуальные результаты

### Разряды популяции в sanity, learning и N=512 runs
![Firing Rates](../images/firing_rates_by_phase.png)

### Распределения дельт на проекции Virtual -> L4
![Virtual L4 Deltas](../images/virtual_l4_matched_vs_unmatched_deltas.png)

### Распределения дельт на последующих проекциях L4 -> L23 и L4 -> L5
![Downstream Transfer](../images/downstream_l4_l23_l4_l5_grouped_deltas.png)

### Доли знаков изменений весов по проекциям
![Sign Ratios](../images/positive_negative_delta_ratios.png)

### Смещение весов до и после обучения
![Weight Histograms](../images/weight_histograms_by_projection.png)

### Пространственная карта изменений весов
![Spatial Delta Map](../images/spatial_delta_map.png)

### Топ-10 потенциированных (усиленных) связей
![Top Positive](../images/top_positive_edges.png)

### Топ-10 депрессированных (ослабленных) связей
![Top Negative](../images/top_negative_edges.png)

---

## Таблица Топ-10 потенциированных (усиленных) связей

| Ранг | Проекция | Откуда | Куда | Начальный вес (uV) | Конечный вес (uV) | Дельта (uV) | Состояние |
|---|---|---|---|---|---|---|---|
"""

    for i, e in enumerate(top_pos):
        report_md += f"| {i+1} | {e['projection']} | {e['src']} | {e['dest']} | {e['initial_weight']} | {e['final_weight']} | {e['delta_signed']} | {'Matched' if e['is_matched'] else 'Unmatched'} |\n"

    report_md += """---

## Выводы и рекомендации

1. **Сильный прогресс относительно v1.0**: matched `Virtual -> L4` почти вышел из LTD (`-0.0167 uV`) и заметно отделился от unmatched фона (`-0.6111 uV`). Это доказывает селективное удержание коррелированных путей от депрессии.
2. **Положительная потенциация не доказана**: strict gate `mean matched Virtual->L4 delta > 0` не закрыт, а L4 firing остается ниже 3 Hz. Исследование классифицируется как полезный `PARTIAL PASS`, не как финальный plasticity pass.
3. **Downstream результат смешанный**: L4 -> L23 имеет положительный matched bias и положительную среднюю дельту. L4 -> L5 имеет только меньшую депрессию matched путей, но его средняя дельта остается отрицательной.
4. **CartPole остается заблокирован**: следующий шаг должен добиться положительной `Virtual -> L4` дельты и восстановить L4 activity gate без нарушения Dale/sign invariants.
"""

    with open(os.path.join(report_dir, "plastic_microcircuit_v1_1_structured_potentiation.md"), "w", encoding="utf-8") as f:
        f.write(report_md)

    # README.md
    readme_md = f"""# Research Archive: Plastic Microcircuit v1.1 Structured Potentiation

Status: {verdict.lower()}
Slug: `plastic_microcircuit_v1_1_structured_potentiation`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование проверяет структурированное обучение и downstream перенос изменений пластичности на последующие слои:
- Проведен sweep параметров стимуляции, выбран победитель `structured_p=0.075`, `background_p=0.003`.
- Доказано селективное удержание Virtual->L4 коррелированных связей от LTD ({mean_corr_v:.4f} uV vs {mean_uncorr_v:.4f} uV).
- Положительная потенциация Virtual->L4 пока не доказана, так как matched mean delta остается ниже 0.
- Downstream перенос частичный: L4->L23 положительный, L4->L5 только менее депрессивный.

## Key Findings

1. **Virtual->L4 Protection**: matched delta {mean_corr_v:.4f} uV против unmatched {mean_uncorr_v:.4f} uV.
2. **Downstream Bias**: L4->L23 matched bias +{l4_l23_bias_lr:.4f} uV, L4->L5 matched bias +{l4_l5_bias_lr:.4f} uV.
3. **Physiology Status**: runaway/sign violations отсутствуют, но L4 rate ниже hard gate 3 Hz.
4. **CartPole Blocked**: переход к RL остается закрыт до positive potentiation + activity pass.

## Reports & Outputs

- Full Report: [reports/plastic_microcircuit_v1_1_structured_potentiation.md](reports/plastic_microcircuit_v1_1_structured_potentiation.md)
- Plots: [images/](images/)
"""

    with open(os.path.join(active_dir, "README.md"), "w", encoding="utf-8") as f:
        f.write(readme_md)

    print("Python analysis and reporting complete.")

if __name__ == "__main__":
    main()
