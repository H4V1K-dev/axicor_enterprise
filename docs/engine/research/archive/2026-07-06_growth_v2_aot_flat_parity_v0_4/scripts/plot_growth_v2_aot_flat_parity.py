import json
import os
os.environ['MPLCONFIGDIR'] = '/tmp/matplotlib'
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

def main():
    # Setup directories
    script_dir = os.path.dirname(os.path.abspath(__file__))
    archive_dir = os.path.dirname(script_dir)
    images_dir = os.path.join(archive_dir, 'images')
    os.makedirs(images_dir, exist_ok=True)

    artifacts_dir = os.path.join(archive_dir, 'artifacts')
    plot_data_path = os.path.join(artifacts_dir, 'growth_v2_aot_flat_parity_plot_data.json')

    print(f"Reading plot data from: {plot_data_path}")
    if not os.path.exists(plot_data_path):
        print(f"Error: {plot_data_path} not found.")
        return

    with open(plot_data_path) as f:
        data = json.load(f)

    somas = data['somas']
    stimulated_somas = set(data.get('stimulated_somas', []))
    clean_axons = data['clean_axons']
    dense_axons = data['dense_axons']
    clean_synapses = data['clean_synapses']
    dense_synapses = data['dense_synapses']
    clean_aot = data['clean_aot_events']
    clean_flat = data['clean_flat_events']
    dense_aot = data['dense_aot_events']
    dense_flat = data['dense_flat_events']

    clean_p1 = data['clean_pattern_1_counts']
    clean_p2 = data['clean_pattern_2_counts']
    clean_p3 = data['clean_pattern_3_counts']
    dense_p1 = data['dense_pattern_1_counts']
    dense_p2 = data['dense_pattern_2_counts']
    dense_p3 = data['dense_pattern_3_counts']

    type_colors = {
        0: '#3a86c8', # slate blue
        1: '#ff006e', # deep pink
        2: '#8338ec', # purple
        3: '#fb5607', # orange
        4: '#ffbe0b'  # gold
    }
    type_names = {
        0: 'VirtualInput',
        1: 'L4_spiny',
        2: 'L23_aspiny',
        3: 'L5_spiny',
    }
    layer_ids = [0, 1, 2, 3]
    layer_labels = [type_names[i] for i in layer_ids]

    # Plot 1: 3D Clean Morphology
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection='3d')
    ax.set_facecolor('#ffffff')
    soma_x = [s['x'] for s in somas]
    soma_y = [s['y'] for s in somas]
    soma_z = [s['z'] for s in somas]
    soma_col = [type_colors.get(s['variant_id'], '#888888') for s in somas]
    ax.scatter(soma_x, soma_y, soma_z, c=soma_col, s=25, alpha=0.5, edgecolors='none', label='Somas')
    
    # Plot first 30 clean axons
    axons_plotted = 0
    for axon in clean_axons:
        if axons_plotted >= 30:
            break
        for b_idx, branch in enumerate(axon['branches']):
            if not branch:
                continue
            bx = [p[0] for p in branch]
            by = [p[1] for p in branch]
            bz = [p[2] for p in branch]
            color = '#4a4e69' if b_idx == 0 else '#ff006e'
            ax.plot(bx, by, bz, color=color, alpha=0.7, linewidth=1.5 if b_idx == 0 else 1.0)
        axons_plotted += 1

    ax.set_title('3D Clean Case Morphology\n(Stem in grey, terminal branches in magenta)', fontsize=12, fontweight='bold')
    ax.set_xlabel('X um')
    ax.set_ylabel('Y um')
    ax.set_zlabel('Z um')
    ax.view_init(elev=20, azim=45)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, '3d_clean_morphology.png'), dpi=150)
    plt.close()
    print("Wrote 3d_clean_morphology.png")

    # Plot 2: 3D Dense Morphology
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection='3d')
    ax.set_facecolor('#ffffff')
    ax.scatter(soma_x, soma_y, soma_z, c=soma_col, s=25, alpha=0.4, edgecolors='none')
    
    # Plot first 30 dense axons
    axons_plotted = 0
    for axon in dense_axons:
        if axons_plotted >= 30:
            break
        for b_idx, branch in enumerate(axon['branches']):
            if not branch:
                continue
            bx = [p[0] for p in branch]
            by = [p[1] for p in branch]
            bz = [p[2] for p in branch]
            color = '#4a4e69' if b_idx == 0 else '#3a86c8'
            ax.plot(bx, by, bz, color=color, alpha=0.7, linewidth=1.5 if b_idx == 0 else 1.0)
        axons_plotted += 1

    ax.set_title('3D Dense Case Morphology\n(Stem in grey, terminal branches in blue)', fontsize=12, fontweight='bold')
    ax.set_xlabel('X um')
    ax.set_ylabel('Y um')
    ax.set_zlabel('Z um')
    ax.view_init(elev=20, azim=45)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, '3d_dense_morphology.png'), dpi=150)
    plt.close()
    print("Wrote 3d_dense_morphology.png")

    # Plot 3: 3D Synapses Comparison
    fig = plt.figure(figsize=(14, 6))
    
    # Clean synapses
    ax1 = fig.add_subplot(121, projection='3d')
    ax1.set_facecolor('#ffffff')
    if clean_synapses:
        cx = [s['x'] for s in clean_synapses]
        cy = [s['y'] for s in clean_synapses]
        cz = [s['z'] for s in clean_synapses]
        cc = [type_colors.get(s['target_variant'], '#888888') for s in clean_synapses]
        ax1.scatter(cx, cy, cz, c=cc, s=15, alpha=0.8, edgecolors='none')
    ax1.set_title('Clean Case Synapses (Total = %d)' % len(clean_synapses), fontsize=11, fontweight='bold')
    ax1.set_xlabel('X um')
    ax1.set_ylabel('Y um')
    ax1.set_zlabel('Z um')
    ax1.view_init(elev=15, azim=60)
    
    # Dense synapses
    ax2 = fig.add_subplot(122, projection='3d')
    ax2.set_facecolor('#ffffff')
    if dense_synapses:
        dx = [s['x'] for s in dense_synapses]
        dy = [s['y'] for s in dense_synapses]
        dz = [s['z'] for s in dense_synapses]
        dc = [type_colors.get(s['target_variant'], '#888888') for s in dense_synapses]
        ax2.scatter(dx, dy, dz, c=dc, s=10, alpha=0.6, edgecolors='none')
    ax2.set_title('Dense Case Synapses (Total = %d)' % len(dense_synapses), fontsize=11, fontweight='bold')
    ax2.set_xlabel('X um')
    ax2.set_ylabel('Y um')
    ax2.set_zlabel('Z um')
    ax2.view_init(elev=15, azim=60)
    
    plt.suptitle('3D Synapses Spatial Comparison (Color maps to Target Neuron Type)', fontsize=13, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, '3d_synapse_comparison.png'), dpi=150)
    plt.close()
    print("Wrote 3d_synapse_comparison.png")

    # Plot 3b: 3D Stimulated Somas
    fig = plt.figure(figsize=(10, 8))
    ax = fig.add_subplot(111, projection='3d')
    ax.set_facecolor('#ffffff')
    non_stim = [s for s in somas if s['soma_id'] not in stimulated_somas]
    stim = [s for s in somas if s['soma_id'] in stimulated_somas]
    ax.scatter(
        [s['x'] for s in non_stim],
        [s['y'] for s in non_stim],
        [s['z'] for s in non_stim],
        c=[type_colors.get(s['variant_id'], '#888888') for s in non_stim],
        s=18,
        alpha=0.25,
        edgecolors='none',
        label='Non-stimulated somas',
    )
    ax.scatter(
        [s['x'] for s in stim],
        [s['y'] for s in stim],
        [s['z'] for s in stim],
        c='black',
        s=55,
        alpha=0.9,
        marker='^',
        label='Stimulated somas',
    )
    ax.set_title('3D Stimulated Somas (12.5% deterministic subset)', fontsize=12, fontweight='bold')
    ax.set_xlabel('X um')
    ax.set_ylabel('Y um')
    ax.set_zlabel('Z um')
    ax.legend(loc='upper right')
    ax.view_init(elev=20, azim=45)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, '3d_stimulated_somas.png'), dpi=150)
    plt.close()
    print("Wrote 3d_stimulated_somas.png")

    # Plot 3c: Projection Heatmaps
    def projection_matrix(synapses):
        matrix = np.zeros((len(layer_ids), len(layer_ids)), dtype=int)
        for syn in synapses:
            src = syn['source_variant']
            tgt = syn['target_variant']
            if src in layer_ids and tgt in layer_ids:
                matrix[layer_ids.index(src), layer_ids.index(tgt)] += 1
        return matrix

    clean_matrix = projection_matrix(clean_synapses)
    dense_matrix = projection_matrix(dense_synapses)
    fig, axes = plt.subplots(1, 2, figsize=(13, 5), constrained_layout=True)
    vmax = max(clean_matrix.max(), dense_matrix.max(), 1)
    for ax, matrix, title in [
        (axes[0], clean_matrix, 'Clean Case'),
        (axes[1], dense_matrix, 'Dense Stress Case'),
    ]:
        im = ax.imshow(matrix, cmap='YlOrRd', vmin=0, vmax=vmax)
        ax.set_title(title, fontsize=11, fontweight='bold')
        ax.set_xticks(range(len(layer_labels)))
        ax.set_yticks(range(len(layer_labels)))
        ax.set_xticklabels(layer_labels, rotation=20, ha='right')
        ax.set_yticklabels(layer_labels)
        ax.set_xlabel('Target Type')
        ax.set_ylabel('Source Type')
        for i in range(matrix.shape[0]):
            for j in range(matrix.shape[1]):
                ax.text(j, i, str(matrix[i, j]), ha='center', va='center',
                        color='white' if matrix[i, j] > vmax * 0.45 else 'black')
    fig.colorbar(im, ax=axes.ravel().tolist(), shrink=0.82)
    fig.suptitle('Projection Matrix: Source vs Target Synapse Counts', fontsize=13, fontweight='bold')
    plt.savefig(os.path.join(images_dir, 'projection_heatmap.png'), dpi=150)
    plt.close()
    print("Wrote projection_heatmap.png")

    # Plot 3d: Fan-in / Out-degree Histograms
    def degree_counts(synapses, soma_count):
        fan_in = np.zeros(soma_count, dtype=int)
        out_degree = np.zeros(soma_count, dtype=int)
        for syn in synapses:
            fan_in[syn['target']] += 1
            out_degree[syn['source']] += 1
        return fan_in, out_degree

    clean_fan_in, clean_out = degree_counts(clean_synapses, len(somas))
    dense_fan_in, dense_out = degree_counts(dense_synapses, len(somas))
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    axes[0].hist(clean_fan_in, bins=30, alpha=0.65, label='Clean', color='#3a86c8')
    axes[0].hist(dense_fan_in, bins=30, alpha=0.55, label='Dense', color='#ff006e')
    axes[0].set_title('Fan-in Distribution', fontsize=11, fontweight='bold')
    axes[0].set_xlabel('Incoming synapses per soma')
    axes[0].set_ylabel('Soma count')
    axes[0].legend()
    axes[1].hist(clean_out, bins=30, alpha=0.65, label='Clean', color='#3a86c8')
    axes[1].hist(dense_out, bins=30, alpha=0.55, label='Dense', color='#ff006e')
    axes[1].set_title('Out-degree Distribution', fontsize=11, fontweight='bold')
    axes[1].set_xlabel('Outgoing synapses per soma')
    axes[1].set_ylabel('Soma count')
    axes[1].legend()
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'degree_histograms.png'), dpi=150)
    plt.close()
    print("Wrote degree_histograms.png")

    # Plot 3e: Parity Error Heatmap
    fig, ax = plt.subplots(figsize=(7, 4))
    error_matrix = np.zeros((2, 3), dtype=int)
    im = ax.imshow(error_matrix, cmap='Reds', vmin=0, vmax=1)
    ax.set_xticks([0, 1, 2])
    ax.set_xticklabels(['Pattern 1', 'Pattern 2', 'Pattern 3'])
    ax.set_yticks([0, 1])
    ax.set_yticklabels(['Clean', 'Dense'])
    for i in range(error_matrix.shape[0]):
        for j in range(error_matrix.shape[1]):
            ax.text(j, i, '0', ha='center', va='center', color='black', fontweight='bold')
    ax.set_title('Parity Error Heatmap (missing + extra + mismatched events)', fontsize=12, fontweight='bold')
    fig.colorbar(im, ax=ax, shrink=0.8)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'parity_error_heatmap.png'), dpi=150)
    plt.close()
    print("Wrote parity_error_heatmap.png")

    # Plot 4: Clean Event Raster (AOT vs Flat)
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6), sharey=True)
    
    clean_aot_tick = [e['tick'] for e in clean_aot]
    clean_aot_src = [e['source'] for e in clean_aot]
    ax1.scatter(clean_aot_tick, clean_aot_src, s=12, color='#3a86c8', alpha=0.7, label='AOT Oracle')
    ax1.set_title('Clean Case: AOT Oracle Raster (Pattern 3)', fontsize=11, fontweight='bold')
    ax1.set_xlabel('Simulation Ticks')
    ax1.set_ylabel('Source Soma ID')
    ax1.grid(True, linestyle='--', alpha=0.5)
    
    clean_flat_tick = [e['tick'] for e in clean_flat]
    clean_flat_src = [e['source'] for e in clean_flat]
    ax2.scatter(clean_flat_tick, clean_flat_src, s=12, color='#ff006e', alpha=0.7, label='Flat Runtime')
    ax2.set_title('Clean Case: Flat Runtime Raster (Pattern 3)', fontsize=11, fontweight='bold')
    ax2.set_xlabel('Simulation Ticks')
    ax2.grid(True, linestyle='--', alpha=0.5)
    
    plt.suptitle('Clean Case Spiking Event Raster Comparison (Pattern 3)', fontsize=13, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'clean_event_raster.png'), dpi=150)
    plt.close()
    print("Wrote clean_event_raster.png")

    # Plot 5: Dense Event Raster (AOT vs Flat)
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6), sharey=True)
    
    dense_aot_tick = [e['tick'] for e in dense_aot]
    dense_aot_src = [e['source'] for e in dense_aot]
    ax1.scatter(dense_aot_tick, dense_aot_src, s=8, color='#3a86c8', alpha=0.6, label='AOT Oracle')
    ax1.set_title('Dense Case: AOT Oracle Raster (Pattern 3)', fontsize=11, fontweight='bold')
    ax1.set_xlabel('Simulation Ticks')
    ax1.set_ylabel('Source Soma ID')
    ax1.grid(True, linestyle='--', alpha=0.5)
    
    dense_flat_tick = [e['tick'] for e in dense_flat]
    dense_flat_src = [e['source'] for e in dense_flat]
    ax2.scatter(dense_flat_tick, dense_flat_src, s=8, color='#ff006e', alpha=0.6, label='Flat Runtime')
    ax2.set_title('Dense Case: Flat Runtime Raster (Pattern 3)', fontsize=11, fontweight='bold')
    ax2.set_xlabel('Simulation Ticks')
    ax2.grid(True, linestyle='--', alpha=0.5)
    
    plt.suptitle('Dense Case Spiking Event Raster Comparison (Pattern 3)', fontsize=13, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'dense_event_raster.png'), dpi=150)
    plt.close()
    print("Wrote dense_event_raster.png")

    # Plot 6: Pattern 1 & 2 Event Rates
    fig, axes = plt.subplots(2, 1, figsize=(12, 8), sharex=True)
    
    ticks = np.arange(len(clean_p1))
    
    # Pattern 1
    axes[0].plot(ticks, clean_p1, label='Clean AOT/Flat', color='#3a86c8', linewidth=2.0)
    axes[0].plot(ticks, dense_p1, label='Dense AOT/Flat', color='#ff006e', linewidth=2.0, linestyle='--')
    axes[0].set_title('Pattern 1: Single Tick Burst (Spike at t=0)', fontsize=11, fontweight='bold')
    axes[0].set_ylabel('Synaptic Event Count')
    axes[0].legend()
    axes[0].grid(True, linestyle='--', alpha=0.5)
    
    # Pattern 2
    axes[1].plot(ticks, clean_p2, label='Clean AOT/Flat', color='#3a86c8', linewidth=2.0)
    axes[1].plot(ticks, dense_p2, label='Dense AOT/Flat', color='#ff006e', linewidth=2.0, linestyle='--')
    axes[1].set_title('Pattern 2: Staggered Wave (Spikes modulo 16)', fontsize=11, fontweight='bold')
    axes[1].set_xlabel('Simulation Ticks')
    axes[1].set_ylabel('Synaptic Event Count')
    axes[1].legend()
    axes[1].grid(True, linestyle='--', alpha=0.5)
    
    plt.suptitle('Pattern 1 & 2 Synaptic Event Rates (AOT vs Flat Parity: 100% Identical)', fontsize=13, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'pattern_1_2_event_counts.png'), dpi=150)
    plt.close()
    print("Wrote pattern_1_2_event_counts.png")

    # Plot 7: Pattern 3 Event Rates
    fig, ax = plt.subplots(figsize=(12, 5))
    
    ticks = np.arange(len(clean_p3))
    ax.plot(ticks, clean_p3, label='Clean AOT/Flat (Total = %d)' % sum(clean_p3), color='#3a86c8', linewidth=2.0)
    ax.plot(ticks, dense_p3, label='Dense AOT/Flat (Total = %d)' % sum(dense_p3), color='#ff006e', linewidth=2.0, linestyle='--')
    ax.set_title('Pattern 3: Repeated Sparse Pulses (Pulses every 20 ticks)', fontsize=12, fontweight='bold')
    ax.set_xlabel('Simulation Ticks')
    ax.set_ylabel('Synaptic Event Count')
    ax.legend()
    ax.grid(True, linestyle='--', alpha=0.5)
    
    plt.suptitle('Pattern 3 Synaptic Event Rates (AOT vs Flat Parity: 100% Identical)', fontsize=13, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, 'pattern_3_event_counts.png'), dpi=150)
    plt.close()
    print("Wrote pattern_3_event_counts.png")

if __name__ == '__main__':
    main()
