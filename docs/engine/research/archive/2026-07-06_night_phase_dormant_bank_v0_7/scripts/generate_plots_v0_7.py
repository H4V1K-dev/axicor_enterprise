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
dormant_traces = data["dormant_traces"]

# Setup directories
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

# 1. Bar Chart: Active / Dormant / Deleted counts on Day 4
policy_names = []
active_counts = []
dormant_counts = []
deleted_counts = []

for p in policies_data:
    policy_names.append(p["name"])
    active_counts.append(p["active_day4"])
    dormant_counts.append(p["dormant_day4"])
    deleted_counts.append(p["deleted_day4"])

x = np.arange(len(policy_names))
width = 0.25

fig, ax = plt.subplots(figsize=(10, 6), dpi=300)
rects1 = ax.bar(x - width, active_counts, width, label="Active Synapses", color="#4e79a7")
rects2 = ax.bar(x, dormant_counts, width, label="Dormant Bank", color="#f28e2b")
rects3 = ax.bar(x + width, deleted_counts, width, label="Hard Deleted", color="#e15759")

ax.set_ylabel("Synapse Count", fontsize=11, fontweight="bold")
ax.set_title("Synapse Distribution at Day 4 across Policies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(policy_names, rotation=15, fontsize=9, fontweight="bold")
ax.grid(axis='y', linestyle='--', alpha=0.5)
ax.legend(loc="upper right", frameon=True, shadow=True)

# Add values above bars
def autolabel(rects):
    for rect in rects:
        height = rect.get_height()
        if height > 0:
            ax.annotate(f"{height}",
                        xy=(rect.get_x() + rect.get_width() / 2, height),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha="center", va="bottom", fontsize=8)

autolabel(rects1)
autolabel(rects2)
autolabel(rects3)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "counts_comparison.png"), dpi=300)
plt.close()
print("counts_comparison.png generated successfully.")

# 2. Line Plot: Retention & Recovery over Day 1 -> Day 2 -> Day 4
# Let's plot both full cohort retention (solid) and survivor cohort retention (dashed)
plt.figure(figsize=(10, 6), dpi=300)

colors = {"hard_delete_absolute_floor": "#4e79a7", 
          "hard_delete_trace_aware": "#e15759", 
          "dormant_trace_aware": "#76b7b2", 
          "dormant_trace_aware_with_return": "#59a14f"}

days_labels = ["Day 1\n(Baseline)", "Day 2\n(Pruned / Demoted)", "Day 4\n(Reactivation / Replay)"]
x_coords = [1, 2, 4]

for p in policies_data:
    name = p["name"]
    col = colors.get(name, "#333333")
    
    # Full cohort retention
    full_ret = [1.0, p["matched_retention_day2_full"], p["matched_retention_day4_full"]]
    plt.plot(x_coords, full_ret, marker="o", linestyle="-", color=col, linewidth=2, label=f"{name} (Full)")
    
    # Survivor cohort retention
    surv_ret = [1.0, p["matched_retention_day2_surv"], p["matched_retention_day4_surv"]]
    plt.plot(x_coords, surv_ret, marker="x", linestyle="--", color=col, linewidth=1.5, alpha=0.7, label=f"{name} (Survivor)")

plt.ylabel("Matched Memory Retention Ratio", fontsize=11, fontweight="bold")
plt.title("Memory Retention & Recovery Path across Policies", fontsize=13, fontweight="bold", pad=15)
plt.xticks(x_coords, days_labels, fontsize=10, fontweight="bold")
plt.grid(True, linestyle='--', alpha=0.5)
plt.legend(bbox_to_anchor=(1.02, 1), loc='upper left', frameon=True)
plt.tight_layout()
plt.savefig(os.path.join(images_dir, "retention_recovery.png"), dpi=300)
plt.close()
print("retention_recovery.png generated successfully.")

# 3. Histogram: Dormant Trace Distribution
if dormant_traces:
    traces = [item["long_trace"] for item in dormant_traces]
    plt.figure(figsize=(8, 5), dpi=300)
    plt.hist(traces, bins=15, color="#f28e2b", edgecolor="black", alpha=0.7)
    plt.xlabel("Long Structural Trace Value", fontsize=11, fontweight="bold")
    plt.ylabel("Frequency", fontsize=11, fontweight="bold")
    plt.title("Long Trace Distribution inside the Dormant Bank\n(dormant_trace_aware_with_return on Night 2)", fontsize=12, fontweight="bold", pad=15)
    plt.grid(True, linestyle='--', alpha=0.5)
    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "dormant_trace_distribution.png"), dpi=300)
    plt.close()
    print("dormant_trace_distribution.png generated successfully.")
else:
    print("No dormant traces available to plot.")
