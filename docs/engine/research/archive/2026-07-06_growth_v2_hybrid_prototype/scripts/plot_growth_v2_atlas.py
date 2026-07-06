import json
import os

os.environ.setdefault("MPLCONFIGDIR", "/tmp/axi_matplotlib")

import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

def main():
    # Load raw JSON data
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
    metrics = db['metrics']

    os.makedirs('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images', exist_ok=True)

    # Color map for neuron types/layers
    # 0: VirtualInput (Purple), 1: L4_spiny (Blue), 2: L23_aspiny (Teal), 3: L5_spiny (Orange)
    type_colors = {0: '#8A2BE2', 1: '#1E90FF', 2: '#008080', 3: '#FF8C00'}
    type_names = {0: 'VirtualInput', 1: 'L4_spiny', 2: 'L23_aspiny', 3: 'L5_spiny'}

    soma_x = [s['x'] for s in somas]
    soma_y = [s['y'] for s in somas]
    soma_z = [s['z'] for s in somas]
    soma_c = [type_colors[s['variant_id']] for s in somas]

    # --- Plot 1: 3D Grid Atlas Comparison Panel ---
    fig = plt.figure(figsize=(18, 8))
    fig.suptitle('3D Axon Growth Atlas: V1 vs. MVP vs. Hybrid v2', fontsize=16, fontweight='bold')

    modes = [('Discrete Baker v1', v1_axons), ('Legacy MVP Continuous', mvp_axons), ('Hybrid Growth v2 (Candidate)', hybrid_axons)]
    for idx, (title, axons) in enumerate(modes, 1):
        ax = fig.add_subplot(1, 3, idx, projection='3d')
        ax.scatter(soma_x, soma_y, soma_z, c=soma_c, s=15, alpha=0.3, label='Somas')

        # Plot a subset of 20 axons for clarity
        for i, axon in enumerate(axons[:20]):
            points = axon['points']
            if len(points) < 2:
                continue
            ap = np.array(points)
            source_soma = somas[axon['soma_id']]
            color = type_colors[source_soma['variant_id']]
            ax.plot(ap[:, 0], ap[:, 1], ap[:, 2], color=color, linewidth=1.5, alpha=0.9)
            ax.scatter(ap[-1, 0], ap[-1, 1], ap[-1, 2], color='red', s=10) # tip

        ax.set_title(title, fontsize=12, fontweight='bold')
        ax.set_xlim(0, 16)
        ax.set_ylim(0, 16)
        ax.set_zlim(0, 32)
        ax.set_xlabel('X')
        ax.set_ylabel('Y')
        ax.set_zlabel('Z')
        ax.view_init(elev=20, azim=45)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/comparison_panel_3d.png', dpi=150)
    plt.close()
    print("Wrote comparison_panel_3d.png")

    # --- Plot 2: Side View Projections (XZ) ---
    fig, axes = plt.subplots(1, 3, figsize=(18, 6), sharey=True)
    fig.suptitle('XZ Projections (Side Views by Layer)', fontsize=14, fontweight='bold')

    for idx, (title, axons) in enumerate(modes):
        ax = axes[idx]
        ax.scatter(soma_x, soma_z, c=soma_c, s=10, alpha=0.2)
        for axon in axons[:40]:
            points = axon['points']
            if len(points) < 2:
                continue
            ap = np.array(points)
            source_soma = somas[axon['soma_id']]
            color = type_colors[source_soma['variant_id']]
            ax.plot(ap[:, 0], ap[:, 2], color=color, linewidth=1.0, alpha=0.7)
            ax.scatter(ap[-1, 0], ap[-1, 2], color='red', s=8)

        ax.set_title(title, fontsize=12)
        ax.set_xlabel('X')
        if idx == 0:
            ax.set_ylabel('Z (Layer height)')
        ax.axhline(8, color='gray', linestyle='--', alpha=0.5)
        ax.axhline(16, color='gray', linestyle='--', alpha=0.5)
        ax.axhline(24, color='gray', linestyle='--', alpha=0.5)
        ax.set_xlim(0, 16)
        ax.set_ylim(0, 32)

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/side_view_projections.png', dpi=150)
    plt.close()
    print("Wrote side_view_projections.png")

    # --- Plot 3: Endpoint Density XY Heatmaps ---
    fig, axes = plt.subplots(1, 3, figsize=(18, 5))
    fig.suptitle('Axon Endpoint Spatial Density (XY Projections)', fontsize=14, fontweight='bold')

    for idx, (title, axons) in enumerate(modes):
        ax = axes[idx]
        endpoints = []
        for axon in axons:
            if axon['points']:
                endpoints.append(axon['points'][-1])
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
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/endpoint_density_heatmaps.png', dpi=150)
    plt.close()
    print("Wrote endpoint_density_heatmaps.png")

    # --- Plot 4: Stop Reasons & Axon Lengths ---
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Axon lengths
    lengths_v1 = [len(a['points']) for a in v1_axons]
    lengths_mvp = [len(a['points']) for a in mvp_axons]
    lengths_hybrid = [len(a['points']) for a in hybrid_axons]

    ax1.hist(lengths_v1, bins=10, alpha=0.5, label=f'V1 (mean={metrics["v1"]["mean_length"]:.1f})', color='blue')
    ax1.hist(lengths_mvp, bins=10, alpha=0.5, label=f'MVP (mean={metrics["mvp"]["mean_length"]:.1f})', color='orange')
    ax1.hist(lengths_hybrid, bins=10, alpha=0.5, label=f'Hybrid (mean={metrics["hybrid"]["mean_length"]:.1f})', color='green')
    ax1.set_title('Axon Length Distribution', fontsize=12, fontweight='bold')
    ax1.set_xlabel('Path Length (segments)')
    ax1.set_ylabel('Count')
    ax1.legend()

    # Stop reasons
    reasons = sorted(list(set(
        list(metrics['v1']['stop_reasons'].keys()) +
        list(metrics['mvp']['stop_reasons'].keys()) +
        list(metrics['hybrid']['stop_reasons'].keys())
    )))
    v1_reasons = [metrics['v1']['stop_reasons'].get(r, 0) for r in reasons]
    mvp_reasons = [metrics['mvp']['stop_reasons'].get(r, 0) for r in reasons]
    hybrid_reasons = [metrics['hybrid']['stop_reasons'].get(r, 0) for r in reasons]

    x = np.arange(len(reasons))
    width = 0.25

    ax2.bar(x - width, v1_reasons, width, label='V1', color='blue')
    ax2.bar(x, mvp_reasons, width, label='MVP', color='orange')
    ax2.bar(x + width, hybrid_reasons, width, label='Hybrid', color='green')
    ax2.set_title('Growth Stop Reasons Comparison', fontsize=12, fontweight='bold')
    ax2.set_xticks(x)
    ax2.set_xticklabels(reasons, rotation=15)
    ax2.set_ylabel('Count')
    ax2.legend()

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/stop_reasons_and_lengths.png', dpi=150)
    plt.close()
    print("Wrote stop_reasons_and_lengths.png")

    # --- Plot 5: Terminal Knot Analysis ---
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Endpoint density comparison
    labels = ['Discrete v1', 'MVP Continuous', 'Hybrid v2']
    densities = [metrics['v1']['mean_endpoint_density'], metrics['mvp']['mean_endpoint_density'], metrics['hybrid']['mean_endpoint_density']]
    tortuosities = [metrics['v1']['mean_last_n_tortuosity'], metrics['mvp']['mean_last_n_tortuosity'], metrics['hybrid']['mean_last_n_tortuosity']]

    ax1.bar(labels, densities, color=['blue', 'orange', 'green'], alpha=0.8)
    ax1.set_title('Endpoint Local Segment Density\n(Anti-Knot Indicator: Lower is Better)', fontsize=11, fontweight='bold')
    ax1.set_ylabel('Mean Segments within R=2.0 voxels')

    ax2.bar(labels, tortuosities, color=['blue', 'orange', 'green'], alpha=0.8)
    ax2.set_ylim(1.0, 1.05)
    ax2.set_title('Last-5 Segment Tortuosity\n(Straightness: Closer to 1.0 is Better)', fontsize=11, fontweight='bold')
    ax2.set_ylabel('Mean Tortuosity')

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/terminal_knot_analysis.png', dpi=150)
    plt.close()
    print("Wrote terminal_knot_analysis.png")

    # --- Plot 6: Synapse Candidate Distributions ---
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Total accepted synapses comparison after production-style per-target cap.
    syn_counts = [metrics['v1']['accepted_synapses'], metrics['mvp']['accepted_synapses'], metrics['hybrid']['accepted_synapses']]
    ax1.bar(labels, syn_counts, color=['blue', 'orange', 'green'], alpha=0.8)
    ax1.set_title('Accepted Synapses After 128-Cap (Seed 12345)', fontsize=12, fontweight='bold')
    ax1.set_ylabel('Count')

    # Uniqueness violations
    uniqueness = [metrics['v1']['uniqueness_violations'], metrics['mvp']['uniqueness_violations'], metrics['hybrid']['uniqueness_violations']]
    ax2.bar(labels, uniqueness, color=['blue', 'orange', 'green'], alpha=0.8)
    ax2.set_title('Uniqueness Violations\n(Duplicate Synapses from Same Axon)', fontsize=12, fontweight='bold')
    ax2.set_ylabel('Count')

    plt.tight_layout()
    fig.savefig('docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/images/synapse_candidate_distributions.png', dpi=150)
    plt.close()
    print("Wrote synapse_candidate_distributions.png")

if __name__ == '__main__':
    main()
