#!/usr/bin/env python3
import json
import os
import sys

os.environ['MPLCONFIGDIR'] = '/tmp/matplotlib'

import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

def main():
    # Setup directories
    script_dir = os.path.dirname(os.path.abspath(__file__))
    archive_dir = os.path.dirname(script_dir)
    images_dir = os.path.join(archive_dir, 'images')
    os.makedirs(images_dir, exist_ok=True)

    plot_data_path = os.path.join(archive_dir, 'artifacts', 'growth_v2_functional_replay_plot_data.json')

    print(f"Reading plot data from: {plot_data_path}")
    if not os.path.exists(plot_data_path):
        print(f"Error: {plot_data_path} not found.")
        return

    with open(plot_data_path) as f:
        data = json.load(f)

    somas = data['somas']
    balanced_axons = data['balanced_axons']
    sparse_syn = data['sparse_synapses']
    dense_syn = data['dense_synapses']
    balanced_syn = data['balanced_synapses']
    balanced_gsop_syn = data['balanced_gsop_synapses']
    
    # Dynamics (Balanced candidate)
    b_static_firing = data['balanced_static_firing']
    b_static_active = data['balanced_static_active']
    b_static_vm = data['balanced_static_vm']
    b_gsop_firing = data['balanced_gsop_firing']
    b_gsop_active = data['balanced_gsop_active']
    b_gsop_vm = data['balanced_gsop_vm']
    
    matched_mean = data['balanced_gsop_matched_mean']
    unmatched_mean = data['balanced_gsop_unmatched_mean']

    # Color palette
    layer_colors = {
        0: '#3a86c8', # Virtual - Slate Blue
        1: '#ff006e', # L4 - Hot Pink
        2: '#8338ec', # L23 - Violet
        3: '#fb5607', # L5 - Orange
    }
    layer_names = ["Virtual", "L4", "L23", "L5"]

    # 1. 3D Topology Balanced Candidate
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection='3d')
    ax.set_facecolor('#ffffff')
    soma_x = [s['x'] for s in somas]
    soma_y = [s['y'] for s in somas]
    soma_z = [s['z'] for s in somas]
    soma_colors = [layer_colors[s['variant_id']] for s in somas]
    ax.scatter(soma_x, soma_y, soma_z, c=soma_colors, s=40, depthshade=True, label='Somas')
    
    # Plot first 15 axons for visual clarity
    for idx, axon in enumerate(balanced_axons[:15]):
        soma = next(s for s in somas if s['soma_id'] == axon['soma_id'])
        c = layer_colors[soma['variant_id']]
        for branch in axon['branches']:
            bx = [b[0] for b in branch]
            by = [b[1] for b in branch]
            bz = [b[2] for b in branch]
            ax.plot(bx, by, bz, color=c, alpha=0.6, linewidth=1.5)
            
    ax.set_title("3D Topology Overview (Balanced Functional Candidate)", fontsize=14, fontweight='bold')
    ax.set_xlabel("X (um)")
    ax.set_ylabel("Y (um)")
    ax.set_zlabel("Z (um)")
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "3d_topology_balanced.png"), dpi=150)
    plt.close()
    print("Wrote 3d_topology_balanced.png")

    # 2. Projection Heatmap (Sparse, Dense, Balanced)
    fig, axes = plt.subplots(1, 3, figsize=(18, 5))
    fig.suptitle("Synapse Count Projection Heatmaps", fontsize=16, fontweight='bold')
    
    projection_pairs = [
        "Virtual->L4", "L4->L23", "L4->L5", "L23->L4", "L23->L23", "L23->L5", "L5->L23"
    ]
    
    for idx, (syns, name) in enumerate([
        (sparse_syn, "Sparse Clean"),
        (balanced_syn, "Balanced Functional"),
        (dense_syn, "Dense Stress")
    ]):
        counts = {p: 0 for p in projection_pairs}
        for s in syns:
            ptype = s['type']
            if ptype in counts:
                counts[ptype] += 1
                
        matrix = np.zeros((4, 4))
        # rows = source, cols = target
        proj_map = {
            "Virtual->L4": (0, 1),
            "L4->L23": (1, 2),
            "L4->L5": (1, 3),
            "L23->L4": (2, 1),
            "L23->L23": (2, 2),
            "L23->L5": (2, 3),
            "L5->L23": (3, 2)
        }
        for p, count in counts.items():
            r, c = proj_map[p]
            matrix[r, c] = count
            
        im = axes[idx].imshow(matrix, cmap='viridis', interpolation='nearest')
        axes[idx].set_title(name, fontsize=12, fontweight='bold')
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
    plt.savefig(os.path.join(images_dir, "projection_heatmap.png"), dpi=150)
    plt.close()
    print("Wrote projection_heatmap.png")

    # 3. Fan-in / Out-degree Histograms
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    fig.suptitle("Degree Distributions (Balanced Candidate)", fontsize=14, fontweight='bold')
    
    fan_in = {s['soma_id']: 0 for s in somas}
    fan_out = {s['soma_id']: 0 for s in somas}
    
    for s in balanced_syn:
        fan_in[s['target']] += 1
        fan_out[s['source']] += 1
        
    axes[0].hist(list(fan_in.values()), bins=20, color='#8338ec', alpha=0.8, edgecolor='black')
    axes[0].set_title("Post-Synaptic Fan-In (In-Degree)", fontsize=12, fontweight='bold')
    axes[0].set_xlabel("Synapses per Soma")
    axes[0].set_ylabel("Soma Count")
    axes[0].grid(True, alpha=0.3)
    
    axes[1].hist(list(fan_out.values()), bins=20, color='#fb5607', alpha=0.8, edgecolor='black')
    axes[1].set_title("Pre-Synaptic Fan-Out (Out-Degree)", fontsize=12, fontweight='bold')
    axes[1].set_xlabel("Synapses per Axon")
    axes[1].set_ylabel("Soma Count")
    axes[1].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "fanin_out_degree.png"), dpi=150)
    plt.close()
    print("Wrote fanin_out_degree.png")

    # 4. Layer Firing Rate Time Series
    fig, axes = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
    fig.suptitle("Layer-Wise Mean Firing Rates Over Ticks", fontsize=15, fontweight='bold')
    
    for idx, (name, c) in enumerate(layer_colors.items()):
        lname = layer_names[idx]
        axes[0].plot(b_static_firing[lname], color=c, label=lname, alpha=0.8)
        axes[1].plot(b_gsop_firing[lname], color=c, label=lname, alpha=0.8)
        
    axes[0].set_title("Static Replay (No Plasticity)", fontsize=12, fontweight='bold')
    axes[0].set_ylabel("Firing Rate (Hz/Soma)")
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    
    axes[1].set_title("Plastic Replay (GSOP Enabled)", fontsize=12, fontweight='bold')
    axes[1].set_xlabel("Time (Ticks)")
    axes[1].set_ylabel("Firing Rate (Hz/Soma)")
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "layer_firing_rate.png"), dpi=150)
    plt.close()
    print("Wrote layer_firing_rate.png")

    # 5. Pseudo-Raster Plot by Layer
    fig, axes = plt.subplots(4, 1, figsize=(12, 8), sharex=True)
    fig.suptitle("Layer Firing Spectrograms (Balanced Candidate, First 2000 Ticks)", fontsize=15, fontweight='bold')
    
    for idx, lname in enumerate(layer_names):
        # We plot a 2D intensity strip representing firing rate per tick
        firing_strip = np.array(b_gsop_firing[lname][:2000]).reshape(1, -1)
        im = axes[idx].imshow(firing_strip, aspect='auto', cmap='plasma', extent=[0, 2000, 0, 1])
        axes[idx].set_yticks([])
        axes[idx].set_ylabel(lname, fontsize=12, fontweight='bold', rotation=0, labelpad=30)
        if idx == 3:
            axes[idx].set_xlabel("Time (Ticks)")
            
    fig.colorbar(im, ax=axes.tolist(), orientation='horizontal', shrink=0.6, pad=0.08, label="Mean Firing Rate")
    plt.savefig(os.path.join(images_dir, "raster_plot.png"), dpi=150)
    plt.close()
    print("Wrote raster_plot.png")

    # 6. Active Fraction Over Time
    fig, axes = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
    fig.suptitle("Fraction of Recruited Active Somas Over Time", fontsize=15, fontweight='bold')
    
    for idx, (name, c) in enumerate(layer_colors.items()):
        lname = layer_names[idx]
        axes[0].plot(b_static_active[lname], color=c, label=lname, alpha=0.8)
        axes[1].plot(b_gsop_active[lname], color=c, label=lname, alpha=0.8)
        
    axes[0].set_title("Static Replay", fontsize=12, fontweight='bold')
    axes[0].set_ylabel("Active Soma Fraction")
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    
    axes[1].set_title("Plastic Replay", fontsize=12, fontweight='bold')
    axes[1].set_xlabel("Time (Ticks)")
    axes[1].set_ylabel("Active Soma Fraction")
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "active_fraction.png"), dpi=150)
    plt.close()
    print("Wrote active_fraction.png")

    # 7. Vm Health / Threshold Distance Plot
    fig, axes = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
    fig.suptitle("Mean Somatic Membrane-to-Spike Distance ($V_{th} + V_{offset} - V_m$)", fontsize=15, fontweight='bold')
    
    for idx, (name, c) in enumerate(layer_colors.items()):
        lname = layer_names[idx]
        axes[0].plot(b_static_vm[lname], color=c, label=lname, alpha=0.8)
        axes[1].plot(b_gsop_vm[lname], color=c, label=lname, alpha=0.8)
        
    axes[0].set_title("Static Replay", fontsize=12, fontweight='bold')
    axes[0].set_ylabel("Distance (uV)")
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    
    axes[1].set_title("Plastic Replay", fontsize=12, fontweight='bold')
    axes[1].set_xlabel("Time (Ticks)")
    axes[1].set_ylabel("Distance (uV)")
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "vm_health.png"), dpi=150)
    plt.close()
    print("Wrote vm_health.png")

    # 8. Mean Synaptic Fatigue over Time (static vs plastic)
    fig = plt.figure(figsize=(10, 6))
    b_static_fatigue = data.get('balanced_static_fatigue', [])
    b_gsop_fatigue = data.get('balanced_gsop_fatigue', [])
    
    plt.plot(b_static_fatigue, label='Static Replay (No Plasticity)', color='#3a86c8', alpha=0.8, linewidth=1.5)
    plt.plot(b_gsop_fatigue, label='Plastic Replay (GSOP)', color='#ff006e', alpha=0.8, linewidth=1.5)
    plt.title("Mean Synaptic Fatigue Dynamics over Time (Balanced Candidate)", fontsize=14, fontweight='bold')
    plt.xlabel("Time (Ticks)")
    plt.ylabel("Mean Synaptic Fatigue (Capacity = 15)")
    plt.legend()
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "fatigue_distribution.png"), dpi=150)
    plt.close()
    print("Wrote fatigue_distribution.png")

    # 9. Weight Delta Histogram
    fig = plt.figure(figsize=(10, 6))
    initial_w = [s['weight'] / (1 << 16) for s in balanced_syn]
    final_w = [s['weight'] / (1 << 16) for s in balanced_gsop_syn]
    weight_deltas = np.array(final_w) - np.array(initial_w)
    plt.hist(weight_deltas, bins=30, color='#8338ec', alpha=0.8, edgecolor='black')
    plt.title(r"Synaptic Weight Changes Histogram ($\Delta W$ in Mass Domain)", fontsize=14, fontweight='bold')
    plt.xlabel(r"Weight Change Value ($\Delta W$)")
    plt.ylabel("Synapse Count")
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "weight_delta_histogram.png"), dpi=150)
    plt.close()
    print("Wrote weight_delta_histogram.png")

    # 10. Matched vs Unmatched Bar Chart
    fig = plt.figure(figsize=(8, 6))
    plt.bar(["Matched (Co-active)", "Unmatched (Control)"], [matched_mean / (1 << 16), unmatched_mean / (1 << 16)], color=['#4CAF50', '#F44336'], alpha=0.8, edgecolor='black', width=0.5)
    plt.title("Plasticity Matched vs. Unmatched Group Separation", fontsize=14, fontweight='bold')
    plt.ylabel(r"Mean Synaptic Weight Change ($\Delta W$)")
    plt.grid(True, alpha=0.3, axis='y')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "matched_vs_unmatched.png"), dpi=150)
    plt.close()
    print("Wrote matched_vs_unmatched.png")

    # 11. Per-Projection Plasticity Heatmap
    fig = plt.figure(figsize=(8, 6))
    proj_deltas = {p: [] for p in projection_pairs}
    for s_i, s_f in zip(balanced_syn, balanced_gsop_syn):
        ptype = s_i['type']
        if ptype in proj_deltas:
            proj_deltas[ptype].append((s_f['weight'] - s_i['weight']) / (1 << 16))
            
    mean_proj_deltas = {p: np.mean(val) if len(val) > 0 else 0.0 for p, val in proj_deltas.items()}
    
    matrix_deltas = np.zeros((4, 4))
    for p, val in mean_proj_deltas.items():
        r, c = proj_map[p]
        matrix_deltas[r, c] = val
        
    im = plt.imshow(matrix_deltas, cmap='coolwarm', interpolation='nearest')
    plt.title(r"Mean Weight Delta ($\Delta W$) by Projection", fontsize=14, fontweight='bold')
    plt.xticks(range(4), layer_names)
    plt.yticks(range(4), layer_names)
    plt.xlabel("Target Layer")
    plt.ylabel("Source Layer")
    for r in range(4):
        for c in range(4):
            if proj_map.get(f"{layer_names[r]}->{layer_names[c]}") is not None or r == c == 2:
                plt.text(c, r, f"{matrix_deltas[r, c]:+.2f}", ha='center', va='center', color='black', fontweight='bold')
    plt.colorbar(im, label="Mean Weight Delta")
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "projection_plasticity_heatmap.png"), dpi=150)
    plt.close()
    print("Wrote projection_plasticity_heatmap.png")

    print("=== All 11 figures generated successfully ===")

if __name__ == "__main__":
    main()
