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

# 1. Lifecycle counts: active/dormant/dead over cycles under stress
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
# Dynamic scale for active synapses under stress
ax_act.set_ylim(min(active) - 500, max(active) + 500)
ax_act.grid(True, linestyle='--', alpha=0.5)

ax_act_twin = ax_act.twinx()
ax_act_twin.plot(cycles, active_delta, marker='x', linestyle='--', color='#e15759', linewidth=1.5, label="Active Delta (Step)")
ax_act_twin.set_ylabel("Active Delta (Count Change)", fontsize=10, color='#e15759', fontweight="bold")
ax_act_twin.tick_params(axis='y', labelcolor='#e15759')
ax_act_twin.set_ylim(min(active_delta) - 100, max(active_delta) + 100)

# Combine Panel A legends
lines1, labels1 = ax_act.get_legend_handles_labels()
lines2, labels2 = ax_act_twin.get_legend_handles_labels()
ax_act.legend(lines1 + lines2, labels1 + labels2, loc="lower left", fontsize=8)

# Panel B: dormant_count / pruned_to_dormant / sprouted / dead
dormant = [c["dormant_count"] for c in cycles_data]
pruned = [c["pruned_to_dormant_count"] for c in cycles_data]
sprouted = [c["sprouted_count"] for c in cycles_data]
dead = [c["dead_count"] for c in cycles_data]

ax_dorm.plot(cycles, dormant, marker='s', color='#f28e2b', linewidth=2.0, label="Dormant Bank (Total)")
ax_dorm.plot(cycles, pruned, marker='P', linestyle=':', color='#76b7b2', linewidth=1.5, label="Pruned to Dormant (Step)")
ax_dorm.plot(cycles, sprouted, marker='*', linestyle='-.', color='#59a14f', linewidth=1.5, label="Sprouted (Step)")
ax_dorm.plot(cycles, dead, marker='^', color='#e15759', linewidth=2.0, label="Dead (Cumulative Evicted)")

ax_dorm.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax_dorm.set_ylabel("Synapse Count", fontsize=10, fontweight="bold")
ax_dorm.set_title("Panel B: Dormant, Pruning, Sprouting & Dead", fontsize=11, fontweight="bold")
ax_dorm.set_xticks(cycles)
# Log scale for y-axis in Panel B because dead_count gets very large (15,000+)
ax_dorm.set_yscale('log')
ax_dorm.grid(True, which="both", linestyle='--', alpha=0.5)
ax_dorm.legend(loc="upper left", fontsize=8)

fig.suptitle("Synapse Lifecycle Counts & Turnover under Eviction Stress (v1.6b)", fontsize=13, fontweight="bold", y=0.98)
plt.tight_layout()
plt.savefig(os.path.join(images_dir, "lifecycle_counts_v1_6b.png"), dpi=300)
plt.close()
print("lifecycle_counts_v1_6b.png generated.")

# 2. Eviction metrics: showing active eviction reasons and global/target bounds
fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 10), dpi=300)

dormant_evicted = [c["dormant_evicted_count"] for c in cycles_data]
dormant_age_max = [c["dormant_age_max"] for c in cycles_data]

# 2a. Dormant Count and Max Age
ax1.plot(cycles, dormant, marker='s', color='#f28e2b', linewidth=2.0, label="Dormant Count (Current)")
ax1.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax1.set_ylabel("Dormant Synapse Count", fontsize=10, color='#f28e2b', fontweight="bold")
ax1.tick_params(axis='y', labelcolor='#f28e2b')
ax1.set_title("Dormant Bank Count & Max Age under Stress", fontsize=11, fontweight="bold")
ax1.set_xticks(cycles)
ax1.set_ylim(-10, 250)
ax1.grid(True, linestyle='--', alpha=0.5)

ax1_twin = ax1.twinx()
ax1_twin.plot(cycles, dormant_age_max, marker='o', linestyle='--', color='#4e79a7', linewidth=1.5, label="Max Dormant Age")
ax1_twin.set_ylabel("Max Age in Bank (Cycles)", fontsize=10, color='#4e79a7', fontweight="bold")
ax1_twin.tick_params(axis='y', labelcolor='#4e79a7')
ax1_twin.set_ylim(-0.5, 6.0)

# Combine legends
lines1, labels1 = ax1.get_legend_handles_labels()
lines2, labels2 = ax1_twin.get_legend_handles_labels()
ax1.legend(lines1 + lines2, labels1 + labels2, loc="upper left", fontsize=9)

# 2b. Eviction reasons per cycle
age_trace_ev = [c["eviction_reason_counts"].get("age_trace", 0) for c in cycles_data]
target_cap_ev = [c["eviction_reason_counts"].get("target_cap", 0) for c in cycles_data]
global_cap_ev = [c["eviction_reason_counts"].get("global_cap", 0) for c in cycles_data]

x = np.array(cycles)
width = 0.25

rects1 = ax2.bar(x - width, age_trace_ev, width, label="Age > MAX & Trace == 0", color='#bab0ac', edgecolor='black')
rects2 = ax2.bar(x, target_cap_ev, width, label="Target Cap Exceeded", color='#499894', edgecolor='black')
rects3 = ax2.bar(x + width, global_cap_ev, width, label="Global Cap Exceeded", color='#86bcb6', edgecolor='black')

ax2.set_xlabel("Cycle", fontsize=11, fontweight="bold")
ax2.set_ylabel("Evicted Synapses per Cycle", fontsize=11, fontweight="bold")
ax2.set_title("Dormant Eviction Reasons Over Cycles (Log Scale)", fontsize=12, fontweight="bold")
ax2.set_xticks(cycles)
ax2.set_yscale('log')
ax2.grid(True, which="both", linestyle='--', alpha=0.5)
ax2.legend(loc="upper right", fontsize=9)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "eviction_metrics_v1_6b.png"), dpi=300)
plt.close()
print("eviction_metrics_v1_6b.png generated.")

# 3. Sprouting metrics and Gini indices
fig, (ax_gini, ax_sprouts) = plt.subplots(1, 2, figsize=(14, 5), dpi=300)

fan_in_gini = [c["fan_in_gini"] for c in cycles_data]
sprout_gini = [c["sprout_target_gini"] for c in cycles_data]

ax_gini.plot(cycles, fan_in_gini, marker='o', color='#e15759', linewidth=2.0, label="Fan-in Gini Index")
ax_gini.plot(cycles, sprout_gini, marker='s', color='#f28e2b', linewidth=2.0, label="Sprout Gini Index")
ax_gini.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax_gini.set_ylabel("Gini Coefficient", fontsize=10, fontweight="bold")
ax_gini.set_title("Structural Fan-in & Sprouting Inequality (Gini)", fontsize=11, fontweight="bold")
ax_gini.set_xticks(cycles)
ax_gini.set_ylim(-0.05, 1.05)
ax_gini.grid(True, linestyle='--', alpha=0.5)
ax_gini.legend(loc="lower right", fontsize=9)

sprouted_target_count = [c["sprouted_target_count"] for c in cycles_data]
max_sprouts_target = [c["max_sprouts_on_single_target"] for c in cycles_data]

ax_sprouts.plot(cycles, sprouted_target_count, marker='P', color='#76b7b2', linewidth=2.0, label="Sprouted Target Count")
ax_sprouts.set_xlabel("Cycle", fontsize=10, fontweight="bold")
ax_sprouts.set_ylabel("Target Count", fontsize=10, color='#76b7b2', fontweight="bold")
ax_sprouts.tick_params(axis='y', labelcolor='#76b7b2')
ax_sprouts.set_title("Sprouted Target Somas and Intensity", fontsize=11, fontweight="bold")
ax_sprouts.set_xticks(cycles)
ax_sprouts.set_ylim(-5, 100)
ax_sprouts.grid(True, linestyle='--', alpha=0.5)

ax_sprouts_twin = ax_sprouts.twinx()
ax_sprouts_twin.plot(cycles, max_sprouts_target, marker='*', linestyle='--', color='#59a14f', linewidth=1.5, label="Max Sprouts on Single Target")
ax_sprouts_twin.set_ylabel("Max Sprouts count", fontsize=10, color='#59a14f', fontweight="bold")
ax_sprouts_twin.tick_params(axis='y', labelcolor='#59a14f')
ax_sprouts_twin.set_ylim(-0.5, 10.0)

# Combine legends
lines1, labels1 = ax_sprouts.get_legend_handles_labels()
lines2, labels2 = ax_sprouts_twin.get_legend_handles_labels()
ax_sprouts.legend(lines1 + lines2, labels1 + labels2, loc="upper left", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "sprouting_gini_v1_6b.png"), dpi=300)
plt.close()
print("sprouting_gini_v1_6b.png generated.")
