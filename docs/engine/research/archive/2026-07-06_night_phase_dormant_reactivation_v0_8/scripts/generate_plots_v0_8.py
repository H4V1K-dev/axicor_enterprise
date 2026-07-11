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
reactivated_synapses = data.get("reactivated_synapses", [])

# Setup directories
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

# Find target policy for funnel
target_policy = None
for p in policies_data:
    if p["name"] == "dormant_context_reactivation":
        target_policy = p
        break

# 1. Reactivation Funnel Bar Chart
if target_policy:
    blockers = target_policy["blockers"]
    funnel_stages = [
        "Total Dormant",
        "Context/Trace OK",
        "Slot OK",
        "Diversity OK",
        "Reactivated"
    ]
    # In our policy, the counts passing each check (non-cumulatively evaluated in Rust, but we can sequence them)
    # Total dormant = 864
    total_dormant = target_policy["dormant_day2"]
    funnel_counts = [
        total_dormant,
        blockers["context_ok"] + blockers["trace_ok"], # approximate candidates passing evidence check
        blockers["slot_ok"],
        blockers["diversity_ok"],
        blockers["all_ok"]
    ]
    # Let's clean up counts: we shouldn't have candidates exceeding total_dormant
    funnel_counts[1] = min(funnel_counts[1], total_dormant)
    funnel_counts[2] = min(funnel_counts[2], funnel_counts[1])
    funnel_counts[3] = min(funnel_counts[3], funnel_counts[2])
    funnel_counts[4] = min(funnel_counts[4], funnel_counts[3])

    fig, ax = plt.subplots(figsize=(8, 5), dpi=300)
    colors = ["#4e79a7", "#76b7b2", "#f28e2b", "#e15759", "#59a14f"]
    bars = ax.barh(funnel_stages[::-1], funnel_counts[::-1], color=colors[::-1], edgecolor="black", height=0.5)
    ax.set_xlabel("Synapse Count", fontsize=11, fontweight="bold")
    ax.set_title("Reactivation Filter Funnel (dormant_context_reactivation)", fontsize=12, fontweight="bold", pad=15)
    ax.grid(axis="x", linestyle="--", alpha=0.5)

    for bar in bars:
        width = bar.get_width()
        ax.annotate(f"{width}",
                    xy=(width, bar.get_y() + bar.get_height() / 2),
                    xytext=(5, 0),
                    textcoords="offset points",
                    ha="left", va="center", fontsize=9, fontweight="bold")

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "counts_funnel.png"), dpi=300)
    plt.close()
    print("counts_funnel.png generated successfully.")
else:
    print("Warning: dormant_context_reactivation policy not found for funnel plot.")

# 2. Day 4 Full-Cohort Retention Comparison
policy_names = []
retention_ratios = []
for p in policies_data:
    policy_names.append(p["name"])
    retention_ratios.append(p["matched_retention_day4_full"])

fig, ax = plt.subplots(figsize=(10, 6), dpi=300)
x_coords = np.arange(len(policy_names))
colors_map = {
    "hard_delete_trace_aware": "#e15759",
    "dormant_no_reactivation": "#76b7b2",
    "dormant_trace_only": "#f28e2b",
    "dormant_context_reactivation": "#59a14f",
    "dormant_context_reactivation_conservative": "#af7aa1"
}
colors = [colors_map.get(name, "#4e79a7") for name in policy_names]

bars = ax.bar(x_coords, retention_ratios, color=colors, edgecolor="black", width=0.5)
ax.set_ylabel("Day 4 Full-Cohort Memory Retention Ratio", fontsize=11, fontweight="bold")
ax.set_title("Memory Retention Comparison across Policies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x_coords)
ax.set_xticklabels(policy_names, rotation=15, fontsize=9, fontweight="bold")
ax.grid(axis="y", linestyle="--", alpha=0.5)

for bar in bars:
    height = bar.get_height()
    # Format retention nicely
    ax.annotate(f"{height:.4f}",
                xy=(bar.get_x() + bar.get_width() / 2, height),
                xytext=(0, 3 if height >= 0 else -12),
                textcoords="offset points",
                ha="center", va="bottom", fontsize=8, fontweight="bold")

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "retention_comparison.png"), dpi=300)
plt.close()
print("retention_comparison.png generated successfully.")

# 3. Reactivated synapse weight distribution & age distribution (winner policy only)
winner_syns = [s for s in reactivated_synapses if s["policy_name"] == "dormant_context_reactivation"]
if winner_syns:
    weights = [s["weight_mass"] for s in winner_syns]
    ages = [s["dormant_age"] for s in winner_syns]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5), dpi=300)

    # Weights
    ax1.hist(weights, bins=10, color="#59a14f", edgecolor="black", alpha=0.7)
    ax1.set_xlabel("Synaptic Weight (Absolute magnitude)", fontsize=10, fontweight="bold")
    ax1.set_ylabel("Count", fontsize=10, fontweight="bold")
    ax1.set_title("Weight Distribution of Reactivated Synapses", fontsize=11, fontweight="bold")
    ax1.grid(True, linestyle="--", alpha=0.5)

    # Ages
    unique_ages, age_counts = np.unique(ages, return_counts=True)
    ax2.bar(unique_ages, age_counts, color="#af7aa1", edgecolor="black", width=0.4)
    ax2.set_xlabel("Dormant Age (in Nights)", fontsize=10, fontweight="bold")
    ax2.set_ylabel("Count", fontsize=10, fontweight="bold")
    ax2.set_title("Dormant Age of Reactivated Synapses", fontsize=11, fontweight="bold")
    ax2.set_xticks(unique_ages)
    ax2.grid(True, linestyle="--", alpha=0.5)

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "reactivated_synapses_distribution.png"), dpi=300)
    plt.close()
    print("reactivated_synapses_distribution.png generated successfully.")
else:
    print("No reactivated synapses found to plot.")
