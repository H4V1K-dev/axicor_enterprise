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
    "segment_index_baseline_v1_3": "#79706e",
    "soma_bucket_cofire": "#4e79a7",
    "soma_bucket_plus_trace": "#59a14f",
    "soma_bucket_plus_trace_plus_slot_pressure": "#e15759",
}

policy_labels = {
    "segment_index_baseline_v1_3": "Segment Baseline (v1.3)",
    "soma_bucket_cofire": "Soma Bucket Cofire",
    "soma_bucket_plus_trace": "Soma Cofire + Trace",
    "soma_bucket_plus_trace_plus_slot_pressure": "Soma Cofire + Slot Pressure (Cap 64)",
}

# 1. Reactivation Comparison by Policy (Cycle 5)
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
ax.set_title("Cycle 5 Dormant Reactivation Comparison Across Evidence Indexing Rules", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=12, fontsize=8, fontweight="bold")
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
plt.savefig(os.path.join(images_dir, "reactivation_comparison.png"), dpi=300)
plt.close()
print("reactivation_comparison.png generated.")

# 2. Reactivation Blocker Breakdown in Cycle 5
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

evidence_fail = [p["cycles"][-1]["reactivation_blocker"]["react_evidence_failed"] for p in policies_data]
slot_fail = [p["cycles"][-1]["reactivation_blocker"]["react_slot_failed"] for p in policies_data]
div_fail = [p["cycles"][-1]["reactivation_blocker"]["react_diversity_failed"] for p in policies_data]

b1 = ax.bar(labels, evidence_fail, label="Trace/Context Evidence Failed", color="#e15759", edgecolor="black", alpha=0.85)
b2 = ax.bar(labels, slot_fail, bottom=evidence_fail, label="Slot / Duplicate Pair Cap Blocked", color="#f28e2b", edgecolor="black", alpha=0.85)
bottoms1 = np.array(evidence_fail) + np.array(slot_fail)
b3 = ax.bar(labels, div_fail, bottom=bottoms1, label="Projection Diversity Blocked", color="#76b7b2", edgecolor="black", alpha=0.85)

ax.set_ylabel("Candidate Rejection Count", fontsize=11, fontweight="bold")
ax.set_title("Cycle 5 Dormant Reactivation Blocker Breakdown by Policy", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=12, fontsize=8, fontweight="bold")
ax.legend(loc="upper right", fontsize=9)
ax.grid(axis='y', linestyle='--', alpha=0.5)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "blocker_breakdown.png"), dpi=300)
plt.close()
print("blocker_breakdown.png generated.")

# 3. Rare Context B Cohort Population Dynamics
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies_data:
    p_name = p["policy"]
    rare_act = [c["rare_initial_active_count"] for c in p["cycles"]]
    rare_react = [c["rare_reactivated_count"] for c in p["cycles"]]
    ax.plot(cycles, rare_act, marker='o', linewidth=2, label=f"Active: {policy_labels[p_name]}", color=policy_colors[p_name])
    ax.plot(cycles, rare_react, marker='^', linestyle='--', linewidth=2, label=f"Reactivated: {policy_labels[p_name]}", color=policy_colors[p_name], alpha=0.7)

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Rare Cohort Synapse Count", fontsize=11, fontweight="bold")
ax.set_title("Original Rare Cohort (Context B) Active & Reactivated Dynamics Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "rare_cohort_lifecycle.png"), dpi=300)
plt.close()
print("rare_cohort_lifecycle.png generated.")
