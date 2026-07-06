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

# Setup directories
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

# Filter policies that support reactivation (exclude hard_delete_trace_aware)
comparison_policies = [p for p in policies_data if p["name"] != "hard_delete_trace_aware"]

names = [p["name"].replace("dormant_", "").replace("_context_reactivation", "") for p in comparison_policies]
counts = [p["reactivated_total"] for p in comparison_policies]
precisions = [p["precision"] for p in comparison_policies]
recalls = [p["recall"] for p in comparison_policies]
jaccards = [p["jaccard"] for p in comparison_policies]

# 1. Reactivation Comparison: Counts (Bars) + Overlap Metrics (Lines on Y2)
fig, ax1 = plt.subplots(figsize=(10, 6), dpi=300)

x = np.arange(len(names))
width = 0.4

bars = ax1.bar(x, counts, width, color="#4e79a7", edgecolor="black", label="Reactivated Count", alpha=0.85)
ax1.set_ylabel("Reactivated Synapses Count", color="#4e79a7", fontsize=11, fontweight="bold")
ax1.tick_params(axis="y", labelcolor="#4e79a7")
ax1.set_title("Reactivation Performance & Overlap vs Oracle Scan", fontsize=13, fontweight="bold", pad=15)
ax1.set_xticks(x)
ax1.set_xticklabels(names, rotation=15, fontsize=9, fontweight="bold")
ax1.grid(axis='y', linestyle='--', alpha=0.5)

# Y2 Axis for metrics
ax2 = ax1.twinx()
line1, = ax2.plot(x, precisions, marker="o", color="#e15759", linewidth=2, label="Precision")
line2, = ax2.plot(x, recalls, marker="s", color="#f28e2b", linewidth=2, label="Recall")
line3, = ax2.plot(x, jaccards, marker="^", color="#59a14f", linewidth=2, label="Jaccard")

ax2.set_ylabel("Overlap Metric Value (Ratio)", color="black", fontsize=11, fontweight="bold")
ax2.tick_params(axis="y", labelcolor="black")
ax2.set_ylim(-0.05, 1.05)

# Label bars
for bar in bars:
    height = bar.get_height()
    ax1.annotate(f"{height}",
                xy=(bar.get_x() + bar.get_width() / 2, height),
                xytext=(0, 3),
                textcoords="offset points",
                ha="center", va="bottom", fontsize=8, fontweight="bold")

# Legend mapping
lines = [bars, line1, line2, line3]
labels = [l.get_label() for l in lines]
ax1.legend(lines, labels, loc="upper left", frameon=True, shadow=True)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "reactivation_comparison.png"), dpi=300)
plt.close()
print("reactivation_comparison.png generated successfully.")

# 2. Cost Comparison: Scan vs Indexed Check Count (Log Scale)
# Only compare the three context policies (scan, indexed_any_day, indexed_bucketed, indexed_bucketed_plus_trace)
cost_policies = [p for p in comparison_policies if p["name"] != "dormant_no_reactivation"]
cost_names = [p["name"].replace("dormant_", "").replace("_context_reactivation", "") for p in cost_policies]
scan_checks = [p["cost"]["scan_checks"] for p in cost_policies]
indexed_checks = [p["cost"]["indexed_checks"] for p in cost_policies]

x = np.arange(len(cost_names))
width = 0.35

fig, ax = plt.subplots(figsize=(9, 6), dpi=300)
rects1 = ax.bar(x - width/2, scan_checks, width, label="Scan Checks", color="#e15759", edgecolor="black")
rects2 = ax.bar(x + width/2, indexed_checks, width, label="Indexed (Summary + Night) Checks", color="#59a14f", edgecolor="black")

ax.set_ylabel("Operation Count (Log Scale)", fontsize=11, fontweight="bold")
ax.set_yscale("log")
ax.set_title("Computational Cost Comparison: Scan vs Indexed Policies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(cost_names, rotation=15, fontsize=9, fontweight="bold")
ax.grid(axis='y', which='both', linestyle='--', alpha=0.5)
ax.legend(loc="upper right", frameon=True, shadow=True)

# Add values above bars
def autolabel_log(rects):
    for rect in rects:
        height = rect.get_height()
        if height > 0:
            ax.annotate(f"{height:,}",
                        xy=(rect.get_x() + rect.get_width() / 2, height),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha="center", va="bottom", fontsize=8)

autolabel_log(rects1)
autolabel_log(rects2)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "cost_comparison.png"), dpi=300)
plt.close()
print("cost_comparison.png generated successfully.")
