# /// script
# dependencies = [
#   "matplotlib",
#   "numpy",
# ]
# ///

import json
import matplotlib.pyplot as plt
import numpy as np

# Load JSON data
with open("plot_data.json", "r") as f:
    data = json.load(f)

# Set style
plt.style.use('seaborn-v0_8-whitegrid' if 'seaborn-v0_8-whitegrid' in plt.style.available else 'default')

# 1. Weight distribution before/after prune floor
plt.figure(figsize=(10, 6))
plt.hist(data['pre_weights'], bins=50, alpha=0.6, label='Pre-night', color='#3182bd')
plt.hist(data['post_weights'], bins=50, alpha=0.6, label='Post-night (After Prune & Decay)', color='#e6550d')
plt.axvline(1498.0, color='red', linestyle='--', label='Prune Floor (1498)')
plt.title('Weight Distribution Before and After Pruning & Decay', fontsize=14, fontweight='bold')
plt.xlabel('Synaptic Weight Magnitude', fontsize=12)
plt.ylabel('Synapse Count', fontsize=12)
plt.legend(fontsize=11)
plt.tight_layout()
plt.savefig('weight_distribution.png', dpi=150)
plt.close()

# 2. Fan-in histogram before/after compaction
plt.figure(figsize=(10, 6))
plt.hist(data['pre_fan_in'], bins=np.arange(0, 105, 2), alpha=0.6, label='Pre-night', color='#3182bd')
plt.hist(data['post_fan_in'], bins=np.arange(0, 105, 2), alpha=0.6, label='Post-night', color='#e6550d')
plt.axvline(96, color='black', linestyle=':', label='Max Cap (96)')
plt.title('Fan-In Distribution Before and After Compaction', fontsize=14, fontweight='bold')
plt.xlabel('In-degree (Synapses per Target Neuron)', fontsize=12)
plt.ylabel('Neuron Count', fontsize=12)
plt.legend(fontsize=11)
plt.tight_layout()
plt.savefig('fan_in_distribution.png', dpi=150)
plt.close()

# 3. Matched/unmatched weight delta distribution
plt.figure(figsize=(10, 6))
plt.hist(data['matched_deltas'], bins=40, alpha=0.6, label='Matched (L4->L5)', color='#2ca02c')
plt.hist(data['unmatched_deltas'], bins=40, alpha=0.6, label='Unmatched', color='#d62728')
plt.title('Distribution of Synaptic Weight Deltas During Learning', fontsize=14, fontweight='bold')
plt.xlabel('Weight Delta (Post-Learning - Initial Weight)', fontsize=12)
plt.ylabel('Synapse Count', fontsize=12)
plt.legend(fontsize=11)
plt.tight_layout()
plt.savefig('delta_distribution.png', dpi=150)
plt.close()

# 4. 3D/2D map of deleted/pruned synapses
plt.figure(figsize=(10, 8))
coords = data['pruned_synapses_coords']
xs = [c['x'] for c in coords]
ys = [c['y'] for c in coords]
zs = [c['z'] for c in coords]
layers = [c['src_layer'] for c in coords]

color_map = {
    'Virtual': '#1f77b4',
    'L4': '#ff7f0e',
    'L23': '#2ca02c',
    'L5': '#d62728'
}

for layer in ['Virtual', 'L4', 'L23', 'L5']:
    lx = [xs[i] for i in range(len(coords)) if layers[i] == layer]
    lz = [zs[i] for i in range(len(coords)) if layers[i] == layer]
    if lx:
        plt.scatter(lx, lz, alpha=0.7, label=f'Pruned from {layer}', color=color_map.get(layer, '#7f7f7f'), edgecolors='none', s=15)

plt.title('Spatial Map of Pruned Synapses (X-Z Projection)', fontsize=14, fontweight='bold')
plt.xlabel('X Position (um)', fontsize=12)
plt.ylabel('Z Position (um)', fontsize=12)
plt.legend(fontsize=11)
plt.tight_layout()
plt.savefig('pruned_synapses_map.png', dpi=150)
plt.close()

# 5. Timeline silence/runaway/activity by Day 2
plt.figure(figsize=(12, 6))
rates = data['day2_firing_rates']
ticks = np.arange(len(rates['Virtual']))

plt.plot(ticks, rates['Virtual'], label='Virtual Input', color='#1f77b4', alpha=0.7)
plt.plot(ticks, rates['L4'], label='L4 Spiny', color='#ff7f0e', alpha=0.8)
plt.plot(ticks, rates['L23'], label='L23 Aspiny', color='#2ca02c', alpha=0.8)
plt.plot(ticks, rates['L5'], label='L5 Spiny', color='#d62728', alpha=0.8)

plt.title('Day 2 Layer Firing Activity Timeline (Post-Pruning)', fontsize=14, fontweight='bold')
plt.xlabel('Simulation Ticks', fontsize=12)
plt.ylabel('Firing Rate (Hz)', fontsize=12)
plt.legend(fontsize=11, loc='upper right')
plt.tight_layout()
plt.savefig('day2_timeline.png', dpi=150)
plt.close()

print("All plots generated successfully!")
