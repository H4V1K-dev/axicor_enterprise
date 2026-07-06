import json
import os
os.environ['MPLCONFIGDIR'] = '/tmp/matplotlib'
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

def main():
    data_path = 'artifacts/growth_v2_comparison_data.json'
    if not os.path.exists(data_path):
        print(f"Data file {data_path} not found.")
        return

    with open(data_path) as f:
        db = json.load(f)

    somas = db['somas']
    v1_axons = db['v1_axons']
    mvp_axons = db['mvp_axons']
    hybrid_axons = db['hybrid_axons']
    multifield_axons = db['multifield_axons']
    metrics = db['metrics']

    os.makedirs('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images', exist_ok=True)

    type_colors = {0: '#8A2BE2', 1: '#1E90FF', 2: '#008080', 3: '#FF8C00'}
    type_names = {0: 'VirtualInput', 1: 'L4_spiny', 2: 'L23_aspiny', 3: 'L5_spiny'}

    soma_x = [s['x'] for s in somas]
    soma_y = [s['y'] for s in somas]
    soma_z = [s['z'] for s in somas]
    soma_c = [type_colors[s['variant_id']] for s in somas]

    # --- Plot 1: 3D Grid Atlas Comparison Panel ---
    fig = plt.figure(figsize=(24, 10))
    fig.suptitle('3D Axon Growth Atlas: V1 vs. MVP vs. Hybrid vs. Multifield v0.2', fontsize=18, fontweight='bold')

    # Subplot 1: Baker V1
    ax1 = fig.add_subplot(1, 4, 1, projection='3d')
    ax1.scatter(soma_x, soma_y, soma_z, c=soma_c, s=15, alpha=0.2)
    for a in v1_axons[:20]:
        ap = np.array(a['points'])
        if len(ap) >= 2:
            ax1.plot(ap[:, 0], ap[:, 1], ap[:, 2], color=type_colors[somas[a['soma_id']]['variant_id']], linewidth=1.5, alpha=0.8)
            ax1.scatter(ap[-1, 0], ap[-1, 1], ap[-1, 2], color='red', s=8)
    ax1.set_title('Baker v1 (Discrete)', fontsize=13, fontweight='bold')
    ax1.set_xlim(0, 16); ax1.set_ylim(0, 16); ax1.set_zlim(0, 32)

    # Subplot 2: MVP
    ax2 = fig.add_subplot(1, 4, 2, projection='3d')
    ax2.scatter(soma_x, soma_y, soma_z, c=soma_c, s=15, alpha=0.2)
    for a in mvp_axons[:20]:
        ap = np.array(a['points'])
        if len(ap) >= 2:
            ax2.plot(ap[:, 0], ap[:, 1], ap[:, 2], color=type_colors[somas[a['soma_id']]['variant_id']], linewidth=1.5, alpha=0.8)
            ax2.scatter(ap[-1, 0], ap[-1, 1], ap[-1, 2], color='red', s=8)
    ax2.set_title('MVP Continuous', fontsize=13, fontweight='bold')
    ax2.set_xlim(0, 16); ax2.set_ylim(0, 16); ax2.set_zlim(0, 32)

    # Subplot 3: Hybrid v2
    ax3 = fig.add_subplot(1, 4, 3, projection='3d')
    ax3.scatter(soma_x, soma_y, soma_z, c=soma_c, s=15, alpha=0.2)
    for a in hybrid_axons[:20]:
        ap = np.array(a['points'])
        if len(ap) >= 2:
            ax3.plot(ap[:, 0], ap[:, 1], ap[:, 2], color=type_colors[somas[a['soma_id']]['variant_id']], linewidth=1.5, alpha=0.8)
            ax3.scatter(ap[-1, 0], ap[-1, 1], ap[-1, 2], color='red', s=8)
    ax3.set_title('Hybrid v2 (Target Damped)', fontsize=13, fontweight='bold')
    ax3.set_xlim(0, 16); ax3.set_ylim(0, 16); ax3.set_zlim(0, 32)

    # Subplot 4: Multifield v0.2 Detailed Render with branches
    ax4 = fig.add_subplot(1, 4, 4, projection='3d')
    ax4.scatter(soma_x, soma_y, soma_z, c=soma_c, s=15, alpha=0.2)
    for a in multifield_axons[:20]:
        color = type_colors[somas[a['soma_id']]['variant_id']]
        for b_idx, branch in enumerate(a['branches']):
            bp = np.array(branch)
            if len(bp) >= 2:
                # Main branch is solid, terminal branches are thinner/dashed or different alpha
                lw = 2.0 if b_idx == 0 else 1.0
                alpha = 0.9 if b_idx == 0 else 0.6
                ls = '-' if b_idx == 0 else '--'
                ax4.plot(bp[:, 0], bp[:, 1], bp[:, 2], color=color, linewidth=lw, alpha=alpha, linestyle=ls)
                if b_idx > 0:
                    ax4.scatter(bp[-1, 0], bp[-1, 1], bp[-1, 2], color='green', s=6) # tip of terminal arbor
    ax4.set_title('Multifield v0.2 (Branching + Repulsion)', fontsize=13, fontweight='bold')
    ax4.set_xlim(0, 16); ax4.set_ylim(0, 16); ax4.set_zlim(0, 32)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/comparison_panel_3d.png', dpi=150)
    plt.close()
    print("Wrote comparison_panel_3d.png")


    # --- Plot 2: Projections & Fasciculation Overlay ---
    fig, axes = plt.subplots(1, 4, figsize=(20, 5), sharey=True)
    fig.suptitle('XZ Layer Projections: Tract Formation & Fasciculation Coherence', fontsize=14, fontweight='bold')

    modes = [
        ('Baker v1', v1_axons, False),
        ('MVP Continuous', mvp_axons, False),
        ('Hybrid v2', hybrid_axons, False),
        ('Multifield v0.2', multifield_axons, True)
    ]

    for idx, (title, axons, is_multi) in enumerate(modes):
        ax = axes[idx]
        ax.scatter(soma_x, soma_z, c=soma_c, s=10, alpha=0.15)

        for a in axons[:40]:
            color = type_colors[somas[a['soma_id']]['variant_id']]
            if is_multi:
                for branch in a['branches']:
                    bp = np.array(branch)
                    if len(bp) >= 2:
                        ax.plot(bp[:, 0], bp[:, 2], color=color, linewidth=0.8, alpha=0.6)
            else:
                ap = np.array(a['points'])
                if len(ap) >= 2:
                    ax.plot(ap[:, 0], ap[:, 2], color=color, linewidth=0.8, alpha=0.6)

        ax.set_title(title)
        ax.set_xlabel('X')
        if idx == 0:
            ax.set_ylabel('Z (Layer height)')
        ax.axhline(8, color='gray', linestyle='--', alpha=0.4)
        ax.axhline(16, color='gray', linestyle='--', alpha=0.4)
        ax.axhline(24, color='gray', linestyle='--', alpha=0.4)
        ax.set_xlim(0, 16)
        ax.set_ylim(0, 32)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/projections_and_fasciculation.png', dpi=150)
    plt.close()
    print("Wrote projections_and_fasciculation.png")


    # --- Plot 3: Soma Repulsion Zoom and Arbor Zoom ---
    fig = plt.figure(figsize=(15, 6))
    fig.suptitle('Soma Repulsion & Terminal Arbor Details (Multifield v0.2)', fontsize=14, fontweight='bold')

    # Subplot 1: Repulsion Zoom (XZ View near X=8, Z=12)
    ax1 = fig.add_subplot(1, 2, 1)
    ax1.set_title('Axons Deflecting/Bending Around Somas', fontsize=12)
    # Filter somas in this window
    window_somas = [s for s in somas if 4 <= s['x'] <= 12 and 6 <= s['z'] <= 18]
    for s in window_somas:
        circ = plt.Circle((s['x'], s['z']), 0.5, color=type_colors[s['variant_id']], alpha=0.5)
        ax1.add_patch(circ)
        # repulsion circle
        rep_circ = plt.Circle((s['x'], s['z']), 1.2, color='gray', fill=False, linestyle=':', alpha=0.3)
        ax1.add_patch(rep_circ)

    for a in multifield_axons:
        for branch in a['branches']:
            bp = np.array(branch)
            if len(bp) >= 2:
                # Plot parts within window
                ax1.plot(bp[:, 0], bp[:, 2], color=type_colors[somas[a['soma_id']]['variant_id']], linewidth=1.2, alpha=0.7)

    ax1.set_xlim(4, 12)
    ax1.set_ylim(6, 18)
    ax1.set_xlabel('X')
    ax1.set_ylabel('Z')

    # Subplot 2: Terminal Arbor Zoom
    ax2 = fig.add_subplot(1, 2, 2, projection='3d')
    ax2.set_title('Terminal Arbor Branching Structure', fontsize=12)
    # Pick a few axons that reached target
    arbor_axons = [a for a in multifield_axons if len(a['branches']) > 1][:4]
    for a in arbor_axons:
        color = type_colors[somas[a['soma_id']]['variant_id']]
        for b_idx, branch in enumerate(a['branches']):
            bp = np.array(branch)
            if len(bp) >= 2:
                lw = 2.5 if b_idx == 0 else 1.2
                ls = '-' if b_idx == 0 else '--'
                ax2.plot(bp[:, 0], bp[:, 1], bp[:, 2], color=color, linewidth=lw, alpha=0.9, linestyle=ls)
                # Plot somas near target
                target_soma_pos = bp[-1]
                for s in somas:
                    d = np.linalg.norm(np.array([s['x'], s['y'], s['z']]) - target_soma_pos)
                    if d <= 3.0:
                        ax2.scatter(s['x'], s['y'], s['z'], color=type_colors[s['variant_id']], s=60, alpha=0.6)

    ax2.set_xlabel('X'); ax2.set_ylabel('Y'); ax2.set_zlabel('Z')
    ax2.view_init(elev=25, azim=30)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/soma_repulsion_and_arbors.png', dpi=150)
    plt.close()
    print("Wrote soma_repulsion_and_arbors.png")


    # --- Plot 4: Endpoint Density Heatmaps ---
    fig, axes = plt.subplots(1, 4, figsize=(20, 5))
    fig.suptitle('Axon Endpoint Density Heatmaps (XY Projections)', fontsize=14, fontweight='bold')

    modes_heat = [
        ('Baker v1', v1_axons, False),
        ('MVP Continuous', mvp_axons, False),
        ('Hybrid v2', hybrid_axons, False),
        ('Multifield v0.2', multifield_axons, True)
    ]

    for idx, (title, axons, is_multi) in enumerate(modes_heat):
        ax = axes[idx]
        endpoints = []
        for a in axons:
            if is_multi:
                # Use the end point of the main stem (branch 0)
                if a['branches'] and a['branches'][0]:
                    endpoints.append(a['branches'][0][-1])
            else:
                if a['points']:
                    endpoints.append(a['points'][-1])
        if endpoints:
            eps = np.array(endpoints)
            h = ax.hexbin(eps[:, 0], eps[:, 1], gridsize=10, cmap='YlOrRd', mincnt=1)
            fig.colorbar(h, ax=ax)
        ax.set_title(title)
        ax.set_xlabel('X')
        ax.set_ylabel('Y')
        ax.set_xlim(0, 16)
        ax.set_ylim(0, 16)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/endpoint_density_heatmaps.png', dpi=150)
    plt.close()
    print("Wrote endpoint_density_heatmaps.png")


    # --- Plot 5: Synapses & Saturation ---
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Synapses comparison
    labels = ['Discrete v1', 'MVP Continuous', 'Hybrid v2', 'Multifield v0.2']
    syn_counts = [
        metrics['v1']['accepted_synapses'],
        metrics['mvp']['accepted_synapses'],
        metrics['hybrid']['accepted_synapses'],
        metrics['multifield']['accepted_synapses']
    ]

    ax1.bar(labels, syn_counts, color=['blue', 'orange', 'green', 'purple'], alpha=0.8)
    ax1.set_title('Total Established Synapses (Seed 12345)\n(Multifield shows post-pruning uniqueness efficiency)', fontsize=11, fontweight='bold')
    ax1.set_ylabel('Count')

    # Fan-in saturation histogram (synapses per target soma)
    target_syn_counts = [0] * len(somas)
    for s in db.get('multifield_synapses', []):
        target_syn_counts[s['target']] += 1

    ax2.hist(target_syn_counts, bins=15, color='purple', alpha=0.7, rwidth=0.85)
    ax2.set_title('Fan-In Saturation Histogram (Multifield v0.2)\n(Max Cap = 128)', fontsize=11, fontweight='bold')
    ax2.set_xlabel('Number of Synapses on Target Soma')
    ax2.set_ylabel('Soma Count')
    ax2.axvline(128, color='red', linestyle='--', label='Cap=128')
    ax2.legend()

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/synapses_and_saturation.png', dpi=150)
    plt.close()
    print("Wrote synapses_and_saturation.png")


    # --- Plot 6: State Transitions ---
    fig, ax = plt.subplots(figsize=(8, 5))
    hist = metrics['multifield']['state_transitions_histogram']
    states_sorted = ['Pathfinding', 'TractFollowing', 'TargetZoneCapture', 'TerminalArborization', 'Terminated']
    counts_sorted = [hist.get(s, 0) for s in states_sorted]

    ax.bar(states_sorted, counts_sorted, color='teal', alpha=0.8, width=0.7)
    ax.set_title('Growth State Transition Activity (Multifield v0.2)', fontsize=12, fontweight='bold')
    ax.set_ylabel('State Visit / Activation Count')
    ax.set_xticks(range(len(states_sorted)))
    ax.set_xticklabels(states_sorted, rotation=15)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/images/state_transitions.png', dpi=150)
    plt.close()
    print("Wrote state_transitions.png")

if __name__ == '__main__':
    main()
