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

experiments = data["experiments"]
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

cycles = [1, 2, 3, 4, 5]

policy_colors = {
    "baseline_fresh_sprout": "#79706e",
    "pair_history_init_weight": "#4e79a7",
    "pair_history_init_trace": "#f28e2b",
    "pair_history_weight_plus_trace": "#59a14f",
    "pair_history_overstrong_stress": "#e15759",
}

policy_labels = {
    "baseline_fresh_sprout": "Baseline (Fresh Sprout)",
    "pair_history_init_weight": "Init Weight Bias",
    "pair_history_init_trace": "Init Trace Bias",
    "pair_history_weight_plus_trace": "Weight + Trace Bias",
    "pair_history_overstrong_stress": "Overstrong Stress",
}

# 1. Recovery vs False Recovery by Policy (Cycle 5)
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

policies = ["baseline_fresh_sprout", "pair_history_init_weight", "pair_history_init_trace", "pair_history_weight_plus_trace", "pair_history_overstrong_stress"]
labels = [policy_labels[p] for p in policies]

returned_recovery = []
absent_false_recovery = []

for p in policies:
    # Find returned branch
    exp_ret = next(e for e in experiments if e["policy"] == p and e["branch"] == "returned_branch")
    c5_ret = exp_ret["cycles"][-1]
    ret_rec = c5_ret["rare_reactivated_count"] + c5_ret["rare_sprouted_new_count"]
    returned_recovery.append(ret_rec)

    # Find absent branch
    exp_abs = next(e for e in experiments if e["policy"] == p and e["branch"] == "absent_branch")
    c5_abs = exp_abs["cycles"][-1]
    abs_rec = c5_abs["rare_reactivated_count"] + c5_abs["rare_sprouted_new_count"]
    absent_false_recovery.append(abs_rec)

x = np.arange(len(labels))
width = 0.35

rects1 = ax.bar(x - width/2, returned_recovery, width, label="True Rare Recovery (Context B Returned)", color="#59a14f", edgecolor="black")
rects2 = ax.bar(x + width/2, absent_false_recovery, width, label="False Recovery / Hallucination (Context B Absent)", color="#e15759", edgecolor="black")

ax.set_ylabel("Rare Cohort Synapse Count (Cycle 5)", fontsize=11, fontweight="bold")
ax.set_title("Cycle 5 Rare Cohort Recovery vs False Recovery Across Pair-History Policies", fontsize=13, fontweight="bold", pad=15)
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
plt.savefig(os.path.join(images_dir, "recovery_vs_false_recovery.png"), dpi=300)
plt.close()
print("recovery_vs_false_recovery.png generated.")

# 2. Pair-History Mass Percentiles Over 5 Cycles
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

for p in policies:
    exp_ret = next(e for e in experiments if e["policy"] == p and e["branch"] == "returned_branch")
    p50_mass = [c["ph_stats"]["mass_p50"] for c in exp_ret["cycles"]]
    p90_mass = [c["ph_stats"]["mass_p90"] for c in exp_ret["cycles"]]
    ax.plot(cycles, p50_mass, marker='o', linewidth=2, label=f"P50: {policy_labels[p]}", color=policy_colors[p])
    ax.plot(cycles, p90_mass, marker='^', linestyle='--', linewidth=1.5, label=f"P90: {policy_labels[p]}", color=policy_colors[p], alpha=0.7)

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Pair History Mass", fontsize=11, fontweight="bold")
ax.set_title("Pair-History Mass P50 & P90 Dynamics Over 5 Cycles", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=8, ncol=2)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "pair_history_mass_distribution.png"), dpi=300)
plt.close()
print("pair_history_mass_distribution.png generated.")

# 3. Rare Cohort Lifecycle Comparison: Returned vs Absent Branch
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

target_policy = "pair_history_weight_plus_trace"
exp_ret = next(e for e in experiments if e["policy"] == target_policy and e["branch"] == "returned_branch")
exp_abs = next(e for e in experiments if e["policy"] == target_policy and e["branch"] == "absent_branch")

rare_act_ret = [c["rare_initial_active_count"] for c in exp_ret["cycles"]]
rare_new_ret = [c["rare_sprouted_new_count"] + c["rare_reactivated_count"] for c in exp_ret["cycles"]]

rare_act_abs = [c["rare_initial_active_count"] for c in exp_abs["cycles"]]
rare_new_abs = [c["rare_sprouted_new_count"] + c["rare_reactivated_count"] for c in exp_abs["cycles"]]

ax.plot(cycles, rare_act_ret, marker='o', linewidth=2, label="Active Cohort (Returned Branch)", color="#4e79a7")
ax.plot(cycles, rare_new_ret, marker='s', linewidth=2, label="New/Reactivated Rare (Returned Branch)", color="#59a14f")
ax.plot(cycles, rare_new_abs, marker='^', linestyle='--', linewidth=2, label="False Recovery (Absent Branch)", color="#e15759")

ax.set_xlabel("Night Phase Cycle", fontsize=10, fontweight="bold")
ax.set_ylabel("Rare Cohort Synapse Count", fontsize=11, fontweight="bold")
ax.set_title(f"Rare Cohort Dynamics: Returned vs Absent Branch ({policy_labels[target_policy]})", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(cycles)
ax.grid(True, linestyle='--', alpha=0.5)
ax.legend(loc="upper right", fontsize=9)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "rare_cohort_lifecycle_comparison.png"), dpi=300)
plt.close()
print("rare_cohort_lifecycle_comparison.png generated.")
