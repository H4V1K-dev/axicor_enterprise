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

evaluations = data["evaluations"]
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

# Helper label formatting
def format_label(eval_item):
    topo_short = {
        "saturated_C17_control": "Control (Full C17)",
        "headroom_C17_pair1": "Pair-1 Headroom",
        "post_prune_headroom": "Post-Prune Headroom",
    }.get(eval_item["topology"], eval_item["topology"])
    
    policy_short = {
        "no_sprouting_baseline": "Baseline",
        "deterministic_under_recruited_projection_diversity": "Deterministic",
        "stochastic_geometry_projection_diversity": "Stochastic",
    }.get(eval_item["policy"], eval_item["policy"])
    
    return f"{topo_short}\n[{policy_short}]"

labels = [format_label(e) for e in evaluations]
x = np.arange(len(labels))
width = 0.25

# 1. Blocker Breakdown Plot
fig, ax = plt.subplots(figsize=(12, 6), dpi=300)

pair_cap_blocked = [e["blocker_breakdown"]["pair_cap_blocked"] for e in evaluations]
exact_dup_blocked = [e["blocker_breakdown"]["exact_duplicate_blocked"] for e in evaluations]
fan_in_blocked = [e["blocker_breakdown"]["target_fan_in_blocked"] for e in evaluations]
proj_div_blocked = [e["blocker_breakdown"]["projection_diversity_blocked"] for e in evaluations]

b1 = ax.bar(labels, pair_cap_blocked, label="Pair Cap Blocked (>=2)", color="#e15759", edgecolor="black", alpha=0.85)
b2 = ax.bar(labels, exact_dup_blocked, bottom=pair_cap_blocked, label="Exact Triplet Duplicate", color="#f28e2b", edgecolor="black", alpha=0.85)
bottoms1 = np.array(pair_cap_blocked) + np.array(exact_dup_blocked)

b3 = ax.bar(labels, fan_in_blocked, bottom=bottoms1, label="Target Fan-In Blocked (>=96)", color="#4e79a7", edgecolor="black", alpha=0.85)
bottoms2 = bottoms1 + np.array(fan_in_blocked)

b4 = ax.bar(labels, proj_div_blocked, bottom=bottoms2, label="Projection Diversity Filter", color="#76b7b2", edgecolor="black", alpha=0.85)

ax.set_ylabel("Candidate Rejection Count", fontsize=11, fontweight="bold")
ax.set_title("Sprouting Candidate Rejection Blocker Breakdown by Topology & Policy", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=15, fontsize=8, fontweight="bold")
ax.legend(title="Rejection Reason", loc="upper right")
ax.grid(axis='y', linestyle='--', alpha=0.5)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "blocker_breakdown.png"), dpi=300)
plt.close()
print("blocker_breakdown.png generated.")

# 2. Sprouted Projection Composition Stacked Bars
fig, ax = plt.subplots(figsize=(12, 6), dpi=300)

proj_names = ["Virtual->L4", "L4->L23", "L4->L5", "L23->L4", "L23->L23", "L23->L5", "L5->L23"]
proj_colors = ["#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1"]

bottoms = np.zeros(len(evaluations))

for idx, proj in enumerate(proj_names):
    proj_vals = [e["sprouted_by_proj"].get(proj, 0) for e in evaluations]
    ax.bar(labels, proj_vals, bottom=bottoms, label=proj, color=proj_colors[idx], edgecolor="black", alpha=0.85)
    bottoms += np.array(proj_vals)

ax.set_ylabel("Number of Sprouted Synapses", fontsize=11, fontweight="bold")
ax.set_title("Composition of Sprouted Synapses by Projection Class Across Topologies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(labels, rotation=15, fontsize=8, fontweight="bold")
ax.legend(title="Projection Classes", loc="upper left", bbox_to_anchor=(1.02, 1))
ax.grid(axis='y', linestyle='--', alpha=0.5)

for idx, val in enumerate(bottoms):
    if val > 0:
        ax.annotate(f"Total: {int(val)}", xy=(idx, val), xytext=(0, 5), textcoords="offset points", ha="center", va="bottom", fontweight="bold", fontsize=8)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "projection_composition.png"), dpi=300)
plt.close()
print("projection_composition.png generated.")

# 3. Fan-in Gini & Monopoly Comparison Plot
fig, ax1 = plt.subplots(figsize=(12, 6), dpi=300)

ginis = [e["gini_coefficient"] for e in evaluations]
monopolies = [e["monopoly_top_5pct_share"] for e in evaluations]

color_gini = "#4e79a7"
color_monopoly = "#e15759"

bar_width = 0.35
bars1 = ax1.bar(x - bar_width/2, ginis, bar_width, label="Fan-in Gini", color=color_gini, edgecolor="black", alpha=0.85)
ax1.set_ylabel("Fan-in Gini Coefficient", color=color_gini, fontsize=11, fontweight="bold")
ax1.tick_params(axis="y", labelcolor=color_gini)
ax1.set_title("Topology Metrics: Fan-in Gini & Top 5% Sprout Monopoly Share", fontsize=13, fontweight="bold", pad=15)
ax1.set_xticks(x)
ax1.set_xticklabels(labels, rotation=15, fontsize=8, fontweight="bold")
ax1.grid(axis='y', linestyle='--', alpha=0.5)

ax2 = ax1.twinx()
bars2 = ax2.bar(x + bar_width/2, monopolies, bar_width, label="Top 5% Target Sprout Share", color=color_monopoly, edgecolor="black", alpha=0.85)
ax2.set_ylabel("Top 5% Target Sprout Share", color=color_monopoly, fontsize=11, fontweight="bold")
ax2.tick_params(axis="y", labelcolor=color_monopoly)
ax2.set_ylim(-0.05, 1.05)

for bar in bars1:
    h = bar.get_height()
    ax1.annotate(f"{h:.4f}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=7)

for bar in bars2:
    h = bar.get_height()
    if h > 0:
        ax2.annotate(f"{h:.1%}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=7)

lines1, labels1 = ax1.get_legend_handles_labels()
lines2, labels2 = ax2.get_legend_handles_labels()
ax1.legend(lines1 + lines2, labels1 + labels2, loc="upper right")

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "fan_in_gini.png"), dpi=300)
plt.close()
print("fan_in_gini.png generated.")
