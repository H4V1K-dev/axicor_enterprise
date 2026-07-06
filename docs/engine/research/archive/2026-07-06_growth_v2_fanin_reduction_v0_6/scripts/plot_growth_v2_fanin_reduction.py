#!/usr/bin/env python3
import json
import os
import sys

os.environ['MPLCONFIGDIR'] = '/tmp/matplotlib'

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    archive_dir = os.path.dirname(script_dir)
    images_dir = os.path.join(archive_dir, 'images')
    os.makedirs(images_dir, exist_ok=True)

    plot_data_path = os.path.join(archive_dir, 'artifacts', 'growth_v2_fanin_reduction_plot_data.json')

    print(f"Reading plot data from: {plot_data_path}")
    if not os.path.exists(plot_data_path):
        print(f"Error: {plot_data_path} not found.")
        return

    with open(plot_data_path) as f:
        data = json.load(f)

    somas = data['somas']
    sweep = data['sweep']
    replays = data['replays']
    gsop_results = data['gsop_results']
    baseline_name = data['baseline_name']
    winner_1_name = data['winner_1_name']
    winner_2_name = data['winner_2_name']

    layer_names = ["Virtual", "L4", "L23", "L5"]
    layer_colors = {
        0: '#3a86c8', # Virtual - Slate Blue
        1: '#ff006e', # L4 - Hot Pink
        2: '#8338ec', # L23 - Violet
        3: '#fb5607', # L5 - Orange
    }

    # =========================================================================
    # 1. Projection Heatmap Comparison (Baseline vs Winner 1 vs Winner 2)
    # =========================================================================
    fig, axes = plt.subplots(1, 3, figsize=(18, 5))
    fig.suptitle("Synapse Count Projection Heatmaps Comparison", fontsize=16, fontweight='bold')

    names_to_plot = [baseline_name, winner_1_name, winner_2_name]
    proj_map = {
        "Virtual->L4": (0, 1),
        "L4->L23": (1, 2),
        "L4->L5": (1, 3),
        "L23->L4": (2, 1),
        "L23->L23": (2, 2),
        "L23->L5": (2, 3),
        "L5->L23": (3, 2)
    }

    for idx, name in enumerate(names_to_plot):
        cfg_sweep = next(c for c in sweep if c['name'] == name)
        matrix = np.zeros((4, 4))
        
        # Populate counts
        matrix[0, 1] = cfg_sweep['count_v_l4']
        matrix[1, 2] = cfg_sweep['count_l4_l23']
        matrix[1, 3] = cfg_sweep['count_l4_l5']
        matrix[2, 1] = cfg_sweep['count_l23_l4']
        matrix[2, 2] = cfg_sweep['count_l23_l23']
        matrix[2, 3] = cfg_sweep['count_l23_l5']
        matrix[3, 2] = cfg_sweep['count_l5_l23']

        im = axes[idx].imshow(matrix, cmap='viridis', interpolation='nearest', vmin=0, vmax=15000)
        axes[idx].set_title(name.replace('_', ' '), fontsize=11, fontweight='bold')
        axes[idx].set_xticks(range(4))
        axes[idx].set_yticks(range(4))
        axes[idx].set_xticklabels(layer_names)
        axes[idx].set_yticklabels(layer_names)
        axes[idx].set_xlabel("Target Layer")
        axes[idx].set_ylabel("Source Layer")
        for r in range(4):
            for c in range(4):
                if matrix[r, c] > 0:
                    axes[idx].text(c, r, f"{int(matrix[r, c])}", ha='center', va='center', color='white', fontweight='bold')
        fig.colorbar(im, ax=axes[idx], shrink=0.7)

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "projection_heatmap_comparison.png"), dpi=150)
    plt.close()
    print("Wrote projection_heatmap_comparison.png")

    # =========================================================================
    # 2. Fan-In Histograms Comparison
    # =========================================================================
    fig, axes = plt.subplots(1, 3, figsize=(18, 5), sharey=True)
    fig.suptitle("Post-Synaptic Fan-In (In-Degree) Distribution Comparison", fontsize=15, fontweight='bold')

    for idx, name in enumerate(names_to_plot):
        replay_data = replays[name]
        # Collect target fan-ins
        fan_in_counts = [0] * 384
        for s in replay_data['gsop_synapses']:
            fan_in_counts[s['target']] += 1
            
        axes[idx].hist(fan_in_counts, bins=range(0, 135, 5), color='#8338ec', alpha=0.8, edgecolor='black')
        axes[idx].set_title(name.replace('_', ' '), fontsize=11, fontweight='bold')
        axes[idx].set_xlabel("Synapses per Soma")
        if idx == 0:
            axes[idx].set_ylabel("Soma Count")
        axes[idx].axvline(128, color='red', linestyle='--', alpha=0.5, label='Hard Limit 128')
        axes[idx].grid(True, alpha=0.3)
        axes[idx].legend(loc='upper left', fontsize=8)

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "fanin_histogram_comparison.png"), dpi=150)
    plt.close()
    print("Wrote fanin_histogram_comparison.png")

    # =========================================================================
    # 3. Saturated Target Count Bar Chart (All 24 Configs)
    # =========================================================================
    fig = plt.figure(figsize=(12, 6))
    indices = range(len(sweep))
    names = [s['name'] for s in sweep]
    sat_counts = [s['saturated_somas'] for s in sweep]
    
    colors = ['#ff006e' if n == baseline_name else ('#4CAF50' if n in [winner_1_name, winner_2_name] else '#3a86c8') for n in names]
    
    plt.bar(indices, sat_counts, color=colors, alpha=0.8, edgecolor='black')
    plt.xticks(indices, [f"C{i}" for i in indices], rotation=0)
    plt.title("Saturated Target Somas per Configuration (fan-in = 128)", fontsize=14, fontweight='bold')
    plt.xlabel("Configuration Index")
    plt.ylabel("Saturated Somas Count")
    plt.grid(True, alpha=0.3, axis='y')
    
    # Legend
    from matplotlib.patches import Patch
    legend_elements = [
        Patch(facecolor='#ff006e', edgecolor='black', label='Baseline v0.5'),
        Patch(facecolor='#4CAF50', edgecolor='black', label='Winner Candidates v0.6'),
        Patch(facecolor='#3a86c8', edgecolor='black', label='Swept Candidates')
    ]
    plt.legend(handles=legend_elements)
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "saturated_target_count.png"), dpi=150)
    plt.close()
    print("Wrote saturated_target_count.png")

    # =========================================================================
    # 4. Accepted Synapses / Projection Count Comparison
    # =========================================================================
    fig, ax1 = plt.subplots(figsize=(14, 6))
    ax2 = ax1.twinx()
    
    indices = np.arange(len(sweep))
    total_synapses = [s['total_synapses'] for s in sweep]
    l4_l5_synapses = [s['count_l4_l5'] for s in sweep]
    
    b1 = ax1.bar(indices - 0.2, total_synapses, 0.4, label='Total Synapses', color='#3a86c8', alpha=0.8, edgecolor='black')
    b2 = ax2.bar(indices + 0.2, l4_l5_synapses, 0.4, label='L4->L5 Synapses', color='#fb5607', alpha=0.8, edgecolor='black')
    
    ax1.set_xlabel("Configuration Index")
    ax1.set_ylabel("Total Synapses", color='#3a86c8')
    ax2.set_ylabel("L4->L5 Synapses", color='#fb5607')
    ax1.set_xticks(indices)
    ax1.set_xticklabels([f"C{i}" for i in indices])
    
    plt.title("Accepted Synapses and Critical L4->L5 Projections Across Sweep", fontsize=14, fontweight='bold')
    
    # Combined legend
    lines = [b1, b2]
    labels = [l.get_label() for l in lines]
    ax1.legend(lines, labels, loc='upper right')
    
    ax1.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "synapses_projection_comparison.png"), dpi=150)
    plt.close()
    print("Wrote synapses_projection_comparison.png")

    # =========================================================================
    # 5. Separate-Stream Compile Audit Chart
    # =========================================================================
    fig, axes = plt.subplots(2, 1, figsize=(14, 10))
    fig.suptitle("Production Axon Stream Compilation Audit", fontsize=15, fontweight='bold')
    
    indices = np.arange(len(sweep))
    compiled_streams = [s['stream_audit']['compiled_stream_count'] for s in sweep]
    active_streams = [s['stream_audit']['streams_with_synapses'] for s in sweep]
    dropped_streams = [s['stream_audit']['streams_dropped'] for s in sweep]
    total_segments = [s['stream_audit']['total_compiled_stream_segments'] for s in sweep]
    
    # Top plot: Stream counts
    axes[0].bar(indices - 0.2, compiled_streams, 0.4, label='Total Compiled Streams', color='#8338ec', alpha=0.8, edgecolor='black')
    axes[0].bar(indices + 0.2, active_streams, 0.4, label='Active Streams (with synapses)', color='#4CAF50', alpha=0.8, edgecolor='black')
    axes[0].set_ylabel("Stream Count")
    axes[0].set_xticks(indices)
    axes[0].set_xticklabels([f"C{i}" for i in indices])
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title("Compiled vs Active (Non-zero Synapses) Streams")
    
    # Bottom plot: Segment Count
    axes[1].bar(indices, total_segments, 0.6, label='Active Stream Segments', color='#20b2aa', alpha=0.8, edgecolor='black')
    axes[1].set_ylabel("Total Segments")
    axes[1].set_xlabel("Configuration Index")
    axes[1].set_xticks(indices)
    axes[1].set_xticklabels([f"C{i}" for i in indices])
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    axes[1].set_title("Total Axon Stream Segments (Memory Footprint after Dropping)")
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "stream_compile_audit.png"), dpi=150)
    plt.close()
    print("Wrote stream_compile_audit.png")

    # =========================================================================
    # 6. Layer Firing Rate Curves
    # =========================================================================
    fig, axes = plt.subplots(3, 1, figsize=(12, 12), sharex=True)
    fig.suptitle("Layer-Wise Mean Firing Rates Comparison (GSOP Plasticity Active)", fontsize=16, fontweight='bold')
    
    for idx, name in enumerate(names_to_plot):
        replay_data = replays[name]
        for l_idx, (layer_v, c) in enumerate(layer_colors.items()):
            lname = layer_names[l_idx]
            axes[idx].plot(replay_data['gsop_firing'][lname], color=c, label=lname, alpha=0.8)
            
        axes[idx].set_title(f"{name.replace('_', ' ')} (Plastic Replay)", fontsize=11, fontweight='bold')
        axes[idx].set_ylabel("Firing Rate (Hz/Soma)")
        axes[idx].legend(loc='upper right')
        axes[idx].grid(True, alpha=0.3)
        
    axes[2].set_xlabel("Time (Ticks)")
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "layer_firing_rates.png"), dpi=150)
    plt.close()
    print("Wrote layer_firing_rates.png")

    # =========================================================================
    # 7. Active Fraction Curves
    # =========================================================================
    fig, axes = plt.subplots(3, 1, figsize=(12, 12), sharex=True)
    fig.suptitle("Fraction of Recruited Active Somas Comparison (GSOP Plasticity Active)", fontsize=16, fontweight='bold')
    
    for idx, name in enumerate(names_to_plot):
        replay_data = replays[name]
        for l_idx, (layer_v, c) in enumerate(layer_colors.items()):
            lname = layer_names[l_idx]
            axes[idx].plot(replay_data['gsop_active'][lname], color=c, label=lname, alpha=0.8)
            
        axes[idx].set_title(f"{name.replace('_', ' ')} (Plastic Replay)", fontsize=11, fontweight='bold')
        axes[idx].set_ylabel("Active Soma Fraction")
        axes[idx].legend(loc='lower right')
        axes[idx].grid(True, alpha=0.3)
        
    axes[2].set_xlabel("Time (Ticks)")
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "active_fractions.png"), dpi=150)
    plt.close()
    print("Wrote active_fractions.png")

    # =========================================================================
    # 8. Matched vs. Unmatched Bar Chart
    # =========================================================================
    fig = plt.figure(figsize=(10, 6))
    
    labels = [n.replace('_', ' ') for n in names_to_plot]
    matched_means = [gsop_results[n]['matched_mean'] / (1 << 16) for n in names_to_plot]
    unmatched_means = [gsop_results[n]['unmatched_mean'] / (1 << 16) for n in names_to_plot]
    
    x = np.arange(len(labels))
    width = 0.35
    
    plt.bar(x - width/2, matched_means, width, label='Matched (Co-active)', color='#4CAF50', alpha=0.8, edgecolor='black')
    plt.bar(x + width/2, unmatched_means, width, label='Unmatched (Control)', color='#F44336', alpha=0.8, edgecolor='black')
    
    plt.title("GSOP Synaptic Weight Delta Matched vs. Unmatched Separation", fontsize=14, fontweight='bold')
    plt.xticks(x, labels, rotation=10)
    plt.ylabel(r"Mean Synaptic Weight Change ($\Delta W$ in Mass Domain)")
    plt.legend()
    plt.grid(True, alpha=0.3, axis='y')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "matched_vs_unmatched.png"), dpi=150)
    plt.close()
    print("Wrote matched_vs_unmatched.png")

    # =========================================================================
    # 9. Weight Delta Histogram
    # =========================================================================
    fig, axes = plt.subplots(1, 3, figsize=(18, 5))
    fig.suptitle(r"Synaptic Weight Changes Histogram ($\Delta W$)", fontsize=15, fontweight='bold')

    for idx, name in enumerate(names_to_plot):
        replay_data = replays[name]
        # Find corresponding configurations in sweep to determine initial weights
        cfg_sweep = next(c for c in sweep if c['name'] == name)
        
        # We can reconstruct deltas from the final synapses in gsop_synapses
        # The initial weights were 1500 (excitatory) or -1500 (inhibitory) in mass domain
        deltas = []
        for s in replay_data['gsop_synapses']:
            is_inh = s['type'] in ["L23->L4", "L23->L23", "L23->L5"]
            init_w = -1500 * (1 << 16) if is_inh else 1500 * (1 << 16)
            delta = (s['weight'] - init_w) / (1 << 16)
            deltas.append(delta)
            
        axes[idx].hist(deltas, bins=30, color='#8338ec', alpha=0.8, edgecolor='black')
        axes[idx].set_title(name.replace('_', ' '), fontsize=11, fontweight='bold')
        axes[idx].set_xlabel("Weight Change Value")
        if idx == 0:
            axes[idx].set_ylabel("Synapse Count")
        axes[idx].grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "weight_delta_histogram.png"), dpi=150)
    plt.close()
    print("Wrote weight_delta_histogram.png")

    # =========================================================================
    # 10. Fan-In vs Matched-Bias Pareto Plot
    # =========================================================================
    fig = plt.figure(figsize=(10, 8))
    
    # Sweep candidates are marked in blue
    # Winners are marked in green
    # Baseline marked in red
    
    for idx, c in enumerate(sweep):
        name = c['name']
        x_val = c['fan_in_p90']
        # Check if we ran replay for this configuration to get matched bias
        if name in gsop_results:
            y_val = gsop_results[name]['matched_mean'] / (1 << 16)
        else:
            y_val = 0.0 # Not simulated
            
        if name == baseline_name:
            plt.scatter(x_val, y_val, color='#ff006e', s=120, edgecolors='black', zorder=5, label='Baseline v0.5' if idx == 0 else "")
            plt.text(x_val + 1, y_val, "Baseline", fontsize=9, fontweight='bold')
        elif name in [winner_1_name, winner_2_name]:
            plt.scatter(x_val, y_val, color='#4CAF50', s=120, edgecolors='black', zorder=5, label='Winner Candidates v0.6' if 'Winner' not in plt.gca().get_legend_handles_labels()[1] else "")
            plt.text(x_val + 1, y_val, name.replace('_', ' '), fontsize=8, fontweight='bold')
        else:
            if y_val > 0.0:
                plt.scatter(x_val, y_val, color='#3a86c8', s=80, edgecolors='black', zorder=3)
                plt.text(x_val + 1, y_val, f"C{idx}", fontsize=8)
            else:
                # Not simulated, plot on x-axis
                plt.scatter(x_val, y_val, color='#cccccc', s=40, edgecolors='gray', zorder=2, alpha=0.5)
                plt.text(x_val + 0.5, y_val + 1, f"C{idx}", fontsize=7, alpha=0.5)
                
    plt.title("Pareto Frontier: Fan-in Pressure (p90) vs. Functional Matched Bias", fontsize=14, fontweight='bold')
    plt.xlabel("Fan-in p90 Pressure (Lower is Better)")
    plt.ylabel("Functional Matched Bias delta-W Mean (Higher is Better)")
    plt.grid(True, alpha=0.3)
    plt.legend(loc='upper right')
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "pareto_fanin_vs_matched_bias.png"), dpi=150)
    plt.close()
    print("Wrote pareto_fanin_vs_matched_bias.png")

    print("=== All 10 figures generated successfully ===")

if __name__ == "__main__":
    main()
