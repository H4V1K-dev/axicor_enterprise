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

policies_data = data["policies"]
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

cycles = [1, 2, 3, 4, 5]
policy_colors = {
    "passive_night_baseline": "#79706e",
    "dormant_reactivation_only": "#4e79a7",
    "sprouting_only": "#f28e2b",
    "full_lifecycle": "#e15759",
}

policy_labels = {
    "passive_night_baseline": "Passive Baseline",
    "dormant_reactivation_only": "Dormant Reactivation Only",
    "sprouting_only": "Sprouting Only",
    "full_lifecycle": "Full Lifecycle (v1.2)",
}

# 1. Lifecycle Counts Over Cycles for Full Lifecycle Policy
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)
full_policy = next(p for p in policies_data if p["policy"] == "full_lifecycle")
c_data = full_policy["cycles"]

active = [c["active_count"] for c in c_data]
dormant = [c["dormant_count"] for c in c_data]
dead = [c["dead_count"] for c in c_data]
pruned = [c["pruned_count"] for c in c_data]
reactivated = [c["reactivated_count"] for c in c_data]
sprouted = [c["sprouted_count"] for c in c_data]

ax.plot(cycles, active, marker='o', linewidth=2.5, label="Active Synapses", color="#4e79a7")
ax.plot(cycles, dormant, marker='s', linewidth=2.5, label="Dormant Bank", color="#f28e2b")
ax.plot(cycles, dead, marker='x', linestyle=':', linewidth=2, label="Dead / Expired", color="#79706e")
ax.plot(cycles, pruned, marker='^', linestyle='--', linewidth=2, label="Pruned (Cycle)", color="#e15759")
ax.plot(cycles, reactivated, marker='d', linestyle='--', linewidth=2, label="Reactivated (Cycle)", color="#76b7b2")
ax.plot(cycles, sprouted, marker='*', linestyle='-', linewidth=2.5, label="Sprouted (Cycle)", color="#59a14f")

ax.set_xlabel("Night Phase Cycle (Cycles 1-2: A+B, Cycles 3-4: A only, Cycle 5: A+B returned)", fontsize=10, fontweight="bold")
ax.set_ylabel("Synapse Count", fontsize=11, fontweight="bold")
ax.set_title("Full Lifecycle (v1.2) Synapse Population Dynamics & Eviction Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="center right", fontsize=9)

ax.axvspan(2.5, 4.5, color='#feebe2', alpha=0.5)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "lifecycle_counts.png"), dpi=300)
plt.close()
print("lifecycle_counts.png generated.")

# 2. Rare Context B Initial Cohort Active Weight Over Cycles
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies_data:
    p_name = p["policy"]
    weights = [c["rare_initial_active_weight"] for c in p["cycles"]]
    ax.plot(cycles, weights, marker='o', linewidth=2.5, label=policy_labels[p_name], color=policy_colors[p_name])

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Original Rare Context B Active Cohort Mean Weight", fontsize=11, fontweight="bold")
ax.set_title("Original Rare Cohort (Context B) Active Weight Across 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=9)

ax.axvspan(2.5, 4.5, color='#feebe2', alpha=0.5)
ax.text(3.5, ax.get_ylim()[1] * 0.95, "Context B Absent\n(Trace Decay Phase)", ha='center', va='top', fontsize=9, bbox=dict(boxstyle='round', facecolor='white', alpha=0.8))

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "rare_path_retention.png"), dpi=300)
plt.close()
print("rare_path_retention.png generated.")

# 3. Structural Health: Fan-in Gini Coefficient Over Cycles
fig, ax1 = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies_data:
    p_name = p["policy"]
    ginis = [c["fan_in_gini"] for c in p["cycles"]]
    ax1.plot(cycles, ginis, marker='o', linewidth=2, label=f"Gini: {policy_labels[p_name]}", color=policy_colors[p_name])

ax1.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax1.set_ylabel("Fan-in Gini Coefficient", fontsize=11, fontweight="bold")
ax1.set_title("Structural Health: Fan-in Gini Coefficient Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax1.set_xticks(cycles)
ax1.grid(True, linestyle='--', alpha=0.5)
ax1.legend(loc="upper right", fontsize=9)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "structural_health.png"), dpi=300)
plt.close()
print("structural_health.png generated.")

# 4. Sprouting & Reactivation Blocker Breakdown (Full Lifecycle)
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

p_full = next(p for p in policies_data if p["policy"] == "full_lifecycle")
pair_blocked = [c["blocker_breakdown"]["pair_cap_blocked"] for c in p_full["cycles"]]
dup_blocked = [c["blocker_breakdown"]["exact_duplicate_blocked"] for c in p_full["cycles"]]
react_trace_blocked = [c["reactivation_blocker"]["react_trace_failed"] for c in p_full["cycles"]]

x = np.arange(len(cycles))
width = 0.25

ax.bar(x - width, pair_blocked, width, label="Sprout Pair Cap Blocked (>=2)", color="#e15759", edgecolor="black")
ax.bar(x, dup_blocked, width, label="Sprout Duplicate Blocked", color="#f28e2b", edgecolor="black")
ax.bar(x + width, react_trace_blocked, width, label="Reactivation Trace/Context Failed", color="#4e79a7", edgecolor="black")

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Candidate Rejection Count", fontsize=11, fontweight="bold")
ax.set_title("Rejection Blocker Breakdown per Cycle (Full Lifecycle)", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels([f"Cycle {c}" for c in cycles], fontweight="bold")
ax.legend(loc="upper right", fontsize=9)
ax.grid(axis='y', linestyle='--', alpha=0.5)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "blocker_breakdown.png"), dpi=300)
plt.close()
print("blocker_breakdown.png generated.")
