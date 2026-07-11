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
    "baseline_v1_2": "#79706e",
    "dormant_trace_floor": "#4e79a7",
    "dormant_slow_decay": "#f28e2b",
    "dormant_age_hysteresis": "#76b7b2",
    "combined_preservation": "#59a14f",
}

policy_labels = {
    "baseline_v1_2": "Baseline (v1.2)",
    "dormant_trace_floor": "Trace Floor (>=15)",
    "dormant_slow_decay": "Slow Decay (>>10)",
    "dormant_age_hysteresis": "Age Hysteresis (MaxAge 5)",
    "combined_preservation": "Combined Preservation",
}

# 1. Cycle 5 Reactivation Comparison
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

p_names = [p["policy"] for p in policies_data]
labels = [policy_labels[name] for name in p_names]

rare_react_c5 = []
total_react_c5 = []

for p in policies_data:
    c5 = p["cycles"][-1]
    rare_react_c5.append(c5["rare_reactivated_count"])
    total_react_c5.append(c5["dormant_reactivated"])

x = np.arange(len(labels))
width = 0.35

rects1 = ax.bar(x - width/2, total_react_c5, width, label="Total Reactivated Synapses (Cycle 5)", color="#4e79a7", edgecolor="black")
rects2 = ax.bar(x + width/2, rare_react_c5, width, label="Original Rare Context B Cohort Reactivated", color="#59a14f", edgecolor="black")

ax.set_ylabel("Reactivated Synapse Count", fontsize=11, fontweight="bold")
ax.set_title("Cycle 5 Memory Reactivation Comparison Across Trace Preservation Policies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=15, fontsize=8, fontweight="bold")
ax.legend(loc="upper left", fontsize=9)
ax.grid(axis='y', linestyle='--', alpha=0.5)

for bar in rects1:
    h = bar.get_height()
    if h > 0:
        ax.annotate(f"{h}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=8, fontweight="bold")

for bar in rects2:
    h = bar.get_height()
    if h > 0:
        ax.annotate(f"{h}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=8, fontweight="bold")

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "cycle5_reactivation_comparison.png"), dpi=300)
plt.close()
print("cycle5_reactivation_comparison.png generated.")

# 2. Reactivation Blocker Breakdown in Cycle 5
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

trace_fail = [p["cycles"][-1]["reactivation_blocker"]["react_trace_failed"] for p in policies_data]
context_fail = [p["cycles"][-1]["reactivation_blocker"]["react_context_failed"] for p in policies_data]
slot_fail = [p["cycles"][-1]["reactivation_blocker"]["react_slot_failed"] for p in policies_data]
div_fail = [p["cycles"][-1]["reactivation_blocker"]["react_diversity_failed"] for p in policies_data]

b1 = ax.bar(labels, trace_fail, label="Trace/Context Missing (long_trace < 20 & no bucket hit)", color="#e15759", edgecolor="black", alpha=0.85)
b2 = ax.bar(labels, slot_fail, bottom=trace_fail, label="Slot / Duplicate Pair Cap Blocked", color="#f28e2b", edgecolor="black", alpha=0.85)
bottoms1 = np.array(trace_fail) + np.array(slot_fail)
b3 = ax.bar(labels, div_fail, bottom=bottoms1, label="Projection Diversity Blocked", color="#76b7b2", edgecolor="black", alpha=0.85)

ax.set_ylabel("Candidate Rejection Count", fontsize=11, fontweight="bold")
ax.set_title("Cycle 5 Dormant Reactivation Blocker Breakdown by Policy", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=15, fontsize=8, fontweight="bold")
ax.legend(loc="upper right", fontsize=9)
ax.grid(axis='y', linestyle='--', alpha=0.5)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "reactivation_blockers.png"), dpi=300)
plt.close()
print("reactivation_blockers.png generated.")

# 3. Dormant Trace Percentiles (p50 / p90 / max) Over Cycles
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies_data:
    p_name = p["policy"]
    p90_vals = [c["long_trace_p90"] for c in p["cycles"]]
    ax.plot(cycles, p90_vals, marker='o', linewidth=2, label=f"P90: {policy_labels[p_name]}", color=policy_colors[p_name])

ax.axhline(20, color='red', linestyle='--', linewidth=1.5, label='Reactivation Threshold (long_trace = 20)')
ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Dormant Long Trace P90 Value", fontsize=11, fontweight="bold")
ax.set_title("Dormant Bank P90 Long Trace Trajectory Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "dormant_trace_distribution.png"), dpi=300)
plt.close()
print("dormant_trace_distribution.png generated.")

# 4. Dormant Lifecycle (Dormant Bank Size vs Evicted Dead vs Reactivated)
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies_data:
    p_name = p["policy"]
    bank_size = [c["dormant_bank_size"] for c in p["cycles"]]
    ax.plot(cycles, bank_size, marker='s', linewidth=2, label=policy_labels[p_name], color=policy_colors[p_name])

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Dormant Bank Size (Synapses)", fontsize=11, fontweight="bold")
ax.set_title("Dormant Bank Population Dynamics & Eviction Boundedness Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "dormant_lifecycle.png"), dpi=300)
plt.close()
print("dormant_lifecycle.png generated.")
