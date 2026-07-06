# /// script
# dependencies = [
#   "matplotlib",
# ]
# ///

import json
import os
import matplotlib.pyplot as plt
import numpy as np

script_dir = os.path.dirname(os.path.abspath(__file__))
json_path = os.path.join(script_dir, "..", "artifacts", "plot_data.json")

if not os.path.exists(json_path):
    print(f"Error: {json_path} not found.")
    exit(1)

with open(json_path, "r") as f:
    data = json.load(f)

cycles_data = data["cycles"]
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

cycles = [c["cycle"] for c in cycles_data]
dormant = [c["dormant_count"] for c in cycles_data]
dead = [c["dead_count"] for c in cycles_data]

age_ev = [c["age_trace_evicted"] for c in cycles_data]
target_ev = [c["target_cap_evicted"] for c in cycles_data]
global_ev = [c["global_cap_evicted"] for c in cycles_data]
age_max = [c["dormant_age_max"] for c in cycles_data]

fig, ax1 = plt.subplots(figsize=(9, 5), dpi=300)

# Left Axis: dormant_count and dead_count
ax1.plot(cycles, dormant, marker='s', color='#f28e2b', linewidth=2.5, label="Dormant Synapses")
ax1.plot(cycles, dead, marker='o', color='#e15759', linewidth=2.5, label="Dead Synapses (Cumulative)")

# Eviction bars (Grouped)
x = np.array(cycles)
width = 0.18
ax1.bar(x - width, age_ev, width, color='#499894', alpha=0.7, label="Age+Trace Evicted (Step)")
ax1.bar(x, target_ev, width, color='#bab0ac', alpha=0.7, label="Target Cap Evicted (Step)")
ax1.bar(x + width, global_ev, width, color='#86bcb6', alpha=0.7, label="Global Cap Evicted (Step)")

ax1.set_xlabel("Cycle", fontsize=11, fontweight="bold")
ax1.set_ylabel("Synapse Count", fontsize=11, fontweight="bold")
ax1.set_title("Age+Trace Eviction Micro-Gate Validation (v1.6c)", fontsize=12, fontweight="bold")
ax1.set_xticks(cycles)
ax1.set_ylim(-10, 120)
ax1.grid(True, linestyle='--', alpha=0.5)

# Right Axis: Max Dormant Age
ax2 = ax1.twinx()
ax2.plot(cycles, age_max, marker='x', linestyle='--', color='#b07aa1', linewidth=1.5, label="Max Dormant Age")
ax2.set_ylabel("Max Age in Bank (Cycles)", fontsize=11, color='#b07aa1', fontweight="bold")
ax2.tick_params(axis='y', labelcolor='#b07aa1')
ax2.set_ylim(-0.5, 4.0)

# Combine legends
lines1, labels1 = ax1.get_legend_handles_labels()
lines2, labels2 = ax2.get_legend_handles_labels()
ax1.legend(lines1 + lines2, labels1 + labels2, loc="center right", fontsize=9)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "age_trace_eviction_gate.png"), dpi=300)
plt.close()
print("age_trace_eviction_gate.png generated.")
