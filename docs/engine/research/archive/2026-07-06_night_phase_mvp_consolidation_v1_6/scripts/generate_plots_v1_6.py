# /// script
# dependencies = [
#   "matplotlib",
#   "numpy",
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

# 1. Lifecycle counts: active/dormant/dead over cycles
fig, (ax_act, ax_dorm) = plt.subplots(1, 2, figsize=(14, 5), dpi=300)

active = [c["active_count"] for c in cycles_data]
active_delta = [active[i] - active[i-1] if i > 0 else 0 for i in range(len(active))]

# Panel A: active_count and active_delta (twinx)
ax_act.plot(cycles, active, marker='o', color='#4e79a7', linewidth=2.5, label="Active Synapses")
ax_act.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax_act.set_ylabel("Active Synapse Count", fontsize=10, color='#4e79a7', fontweight="bold")
ax_act.tick_params(axis='y', labelcolor='#4e79a7')
ax_act.set_title("Panel A: Active Synapse Count & Delta", fontsize=11, fontweight="bold")
ax_act.set_xticks(cycles)
ax_act.set_ylim(20420, 20480)
ax_act.grid(True, linestyle='--', alpha=0.5)

ax_act_twin = ax_act.twinx()
ax_act_twin.plot(cycles, active_delta, marker='x', linestyle='--', color='#e15759', linewidth=1.5, label="Active Delta (Step)")
ax_act_twin.set_ylabel("Active Delta (Count Change)", fontsize=10, color='#e15759', fontweight="bold")
ax_act_twin.tick_params(axis='y', labelcolor='#e15759')
ax_act_twin.set_ylim(-30, 10)

# Combine Panel A legends
lines1, labels1 = ax_act.get_legend_handles_labels()
lines2, labels2 = ax_act_twin.get_legend_handles_labels()
ax_act.legend(lines1 + lines2, labels1 + labels2, loc="upper right", fontsize=8)

# Panel B: dormant_count / pruned_to_dormant / sprouted / dead
dormant = [c["dormant_count"] for c in cycles_data]
pruned = [c["pruned_to_dormant_count"] for c in cycles_data]
sprouted = [c["sprouted_count"] for c in cycles_data]
dead = [c["dead_count"] for c in cycles_data]

ax_dorm.plot(cycles, dormant, marker='s', color='#f28e2b', linewidth=2.0, label="Dormant Bank (Total)")
ax_dorm.plot(cycles, pruned, marker='P', linestyle=':', color='#76b7b2', linewidth=1.5, label="Pruned to Dormant (Step)")
ax_dorm.plot(cycles, sprouted, marker='*', linestyle='-.', color='#59a14f', linewidth=1.5, label="Sprouted (Step)")
ax_dorm.plot(cycles, dead, marker='^', color='#d3d3d3', linewidth=2.0, label="Dead (Cumulative Evicted)")

ax_dorm.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax_dorm.set_ylabel("Synapse Count", fontsize=10, fontweight="bold")
ax_dorm.set_title("Panel B: Dormant, Pruning, Sprouting & Dead", fontsize=11, fontweight="bold")
ax_dorm.set_xticks(cycles)
ax_dorm.set_ylim(-5, 60)
ax_dorm.grid(True, linestyle='--', alpha=0.5)
ax_dorm.legend(loc="upper left", fontsize=8)

fig.suptitle("Synapse Lifecycle Counts & Turnover Over 10 Cycles", fontsize=13, fontweight="bold", y=0.98)
plt.tight_layout()
plt.savefig(os.path.join(images_dir, "lifecycle_counts.png"), dpi=300)
plt.close()
print("lifecycle_counts.png generated.")

# 2. Dormant bank health: explicitly show that eviction did not trigger
fig, ax = plt.subplots(figsize=(10, 5), dpi=300)

dormant_evicted = [c["dormant_evicted_count"] for c in cycles_data]
dormant_age_max = [c["dormant_age_max"] for c in cycles_data]

ax.plot(cycles, dormant, marker='s', color='#f28e2b', linewidth=2.0, label="Dormant Count (Current)")
ax.plot(cycles, dormant_age_max, marker='o', linestyle='--', color='#e15759', linewidth=1.5, label="Max Dormant Age")
ax.plot(cycles, dormant_evicted, marker='x', linestyle=':', color='#bab0ac', linewidth=2.0, label="Dormant Evicted (Step)")

ax.set_xlabel("Cycle", fontsize=11, fontweight="bold")
ax.set_ylabel("Count / Age (Cycles)", fontsize=11, fontweight="bold")
ax.set_title("Dormant Bank Health (Eviction Mechanics Inactive)", fontsize=12, fontweight="bold")
ax.set_xticks(cycles)
ax.set_ylim(-2, 50)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper left", fontsize=10)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "dormant_bank_health.png"), dpi=300)
plt.close()
print("dormant_bank_health.png generated.")

# 3. Network stability: firing pressure, silence/runaway ticks, Gini index, projection coverage
fig, axs = plt.subplots(2, 2, figsize=(12, 10), dpi=300)

# 3a. Silence / Runaway ticks (use twinx because silence is around 1940 and runaway is 0)
axs[0, 0].plot(cycles, [c["silence_ticks"] for c in cycles_data], marker='o', color='#4e79a7', label="Silence Ticks")
axs[0, 0].set_ylabel("Silence Tick Count (Zoomed)", fontsize=10, color='#4e79a7', fontweight="bold")
axs[0, 0].set_ylim(1920, 1980)

axs00_twin = axs[0, 0].twinx()
axs00_twin.plot(cycles, [c["runaway_ticks"] for c in cycles_data], marker='s', color='#e15759', label="Runaway Ticks")
axs00_twin.set_ylabel("Runaway Tick Count", fontsize=10, color='#e15759', fontweight="bold")
axs00_twin.set_ylim(-0.5, 10.0)

axs[0, 0].set_xlabel("Cycle", fontsize=10, fontweight="bold")
axs[0, 0].set_title("Network Activity Dynamics (Silence vs Runaway)", fontsize=11, fontweight="bold")
axs[0, 0].set_xticks(cycles)
axs[0, 0].grid(True, linestyle='--', alpha=0.5)

# Combine legends
lines1, labels1 = axs[0, 0].get_legend_handles_labels()
lines2, labels2 = axs00_twin.get_legend_handles_labels()
axs[0, 0].legend(lines1 + lines2, labels1 + labels2, loc="upper right", fontsize=9)

# 3b. Firing rates per layer (Mean/Total spikes)
layers = ["Virtual", "L4", "L23", "L5"]
layer_colors = {"Virtual": "#76b7b2", "L4": "#59a14f", "L23": "#edc948", "L5": "#b07aa1"}
for lyr in layers:
    spikes = [c["spike_counts_per_layer"].get(lyr, 0) for c in cycles_data]
    axs[0, 1].plot(cycles, spikes, marker='o', color=layer_colors[lyr], label=f"{lyr} Layer Spikes")
axs[0, 1].set_xlabel("Cycle", fontsize=10, fontweight="bold")
axs[0, 1].set_ylabel("Spike Count", fontsize=10, fontweight="bold")
axs[0, 1].set_title("Spikes Per Layer Over Cycles", fontsize=11, fontweight="bold")
axs[0, 1].set_xticks(cycles)
axs[0, 1].set_ylim(-5, 130)
axs[0, 1].grid(True, linestyle='--', alpha=0.5)
axs[0, 1].legend(loc="upper left", fontsize=9)

# 3c. Fan-in Gini & top 5% share
axs[1, 0].plot(cycles, [c["fan_in_gini"] for c in cycles_data], marker='o', color='#e15759', label="Fan-in Gini Index")
axs[1, 0].plot(cycles, [c["top_5pct_fan_in_share"] for c in cycles_data], marker='s', color='#f28e2b', label="Top 5% Sprouted Share (unreliable when sprout_count is tiny)")
axs[1, 0].set_xlabel("Cycle", fontsize=10, fontweight="bold")
axs[1, 0].set_ylabel("Metric Value / Share", fontsize=10, fontweight="bold")
axs[1, 0].set_title("Fan-in Inequality & Sprouting Monopoly Share", fontsize=11, fontweight="bold")
axs[1, 0].set_xticks(cycles)
axs[1, 0].set_ylim(-0.05, 1.15)
axs[1, 0].grid(True, linestyle='--', alpha=0.5)
axs[1, 0].legend(loc="upper right", fontsize=7)

# 3d. Projection Coverage & Under-Recruited Count before/after sprouting
# Use twinx because projection coverage is [0, 1] and under-recruited is ~384
axs[1, 1].plot(cycles, [c["under_recruited_count_before"] for c in cycles_data], marker='s', color='#af7aa1', label="Under-Recruited (Before)")
axs[1, 1].plot(cycles, [c["under_recruited_count_after"] for c in cycles_data], marker='x', linestyle='--', color='#9c755f', label="Under-Recruited (After)")
axs[1, 1].set_ylabel("Under-Recruited Soma Count (Zoomed)", fontsize=10, color='#af7aa1', fontweight="bold")
axs[1, 1].set_ylim(360, 400)

axs11_twin = axs[1, 1].twinx()
axs11_twin.plot(cycles, [c["projection_coverage"] for c in cycles_data], marker='o', color='#59a14f', label="Proj Coverage Fraction")
axs11_twin.set_ylabel("Projection Coverage Fraction", fontsize=10, color='#59a14f', fontweight="bold")
axs11_twin.set_ylim(0.0, 1.0)

axs[1, 1].set_xlabel("Cycle", fontsize=10, fontweight="bold")
axs[1, 1].set_title("Structural Diversity & Recruitment Success", fontsize=11, fontweight="bold")
axs[1, 1].set_xticks(cycles)
axs[1, 1].grid(True, linestyle='--', alpha=0.5)

# Combine legends
lines1, labels1 = axs[1, 1].get_legend_handles_labels()
lines2, labels2 = axs11_twin.get_legend_handles_labels()
axs[1, 1].legend(lines1 + lines2, labels1 + labels2, loc="upper right", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "network_stability.png"), dpi=300)
plt.close()
print("network_stability.png generated.")
