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

# Policy name mappings for cleaner labels
name_mapping = {
    "no_sprouting_baseline": "Baseline (No Sprout)",
    "active_source_greedy_sprouting": "Active Src Greedy",
    "under_recruited_target_sprouting": "Target Spatial",
    "under_recruited_plus_projection_diversity": "Target + Proj Div",
    "under_recruited_plus_diversity_plus_stochastic_geometry": "Stochastic Geometry (Cap Saturated)",
}

names = [name_mapping.get(p["name"], p["name"]) for p in policies_data]
ginis = [p["gini_coefficient"] for p in policies_data]
sprouted_counts = [p["sprouted_count"] for p in policies_data]
monopolies = [p["monopoly_top_5pct_share"] for p in policies_data]
ur_before = [p["under_recruited_activity_before"] for p in policies_data]
ur_after = [p["under_recruited_activity_after"] for p in policies_data]

# 1. Fan-in Gini Coefficient and Monopoly Indicator by Policy
fig, ax1 = plt.subplots(figsize=(10, 6), dpi=300)

x = np.arange(len(names))
width = 0.35

color_gini = "#4e79a7"
color_monopoly = "#e15759"

bars1 = ax1.bar(x - width/2, ginis, width, label="Fan-in Gini", color=color_gini, edgecolor="black", alpha=0.85)
ax1.set_ylabel("Fan-in Gini Coefficient", color=color_gini, fontsize=11, fontweight="bold")
ax1.tick_params(axis="y", labelcolor=color_gini)
ax1.set_title("Network Topology Metrics: Gini Coefficient & Sprout Monopoly", fontsize=13, fontweight="bold", pad=15)
ax1.set_xticks(x)
ax1.set_xticklabels(names, rotation=15, fontsize=8, fontweight="bold")
ax1.grid(axis='y', linestyle='--', alpha=0.5)

# Dual Y-axis for monopoly share
ax2 = ax1.twinx()
bars2 = ax2.bar(x + width/2, monopolies, width, label="Top 5% Target Sprout Share", color=color_monopoly, edgecolor="black", alpha=0.85)
ax2.set_ylabel("Top 5% Target Sprout Share", color=color_monopoly, fontsize=11, fontweight="bold")
ax2.tick_params(axis="y", labelcolor=color_monopoly)
ax2.set_ylim(-0.05, 1.05)

# Label values
for bar in bars1:
    h = bar.get_height()
    ax1.annotate(f"{h:.4f}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=8)

for bar in bars2:
    h = bar.get_height()
    ax2.annotate(f"{h:.1%}", xy=(bar.get_x() + bar.get_width()/2, h), xytext=(0, 3), textcoords="offset points", ha="center", va="bottom", fontsize=8)

# Combine legends
lines1, labels1 = ax1.get_legend_handles_labels()
lines2, labels2 = ax2.get_legend_handles_labels()
ax1.legend(lines1 + lines2, labels1 + labels2, loc="upper right")

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "fan_in_gini.png"), dpi=300)
plt.close()
print("fan_in_gini.png generated.")

# 2. Sprouted Projection Composition by Policy
fig, ax = plt.subplots(figsize=(10, 6), dpi=300)

proj_names = ["Virtual->L4", "L4->L23", "L4->L5", "L23->L4", "L23->L23", "L23->L5", "L5->L23"]
proj_colors = ["#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1"]

# Filter out baseline which has 0 sprouts
sprouted_policies = [p for p in policies_data if p["sprouted_count"] > 0]
sp_names = [name_mapping.get(p["name"], p["name"]) for p in sprouted_policies]

bottoms = np.zeros(len(sp_names))

for idx, proj in enumerate(proj_names):
    proj_vals = [p["sprouted_by_proj"].get(proj, 0) for p in sprouted_policies]
    ax.bar(sp_names, proj_vals, bottom=bottoms, label=proj, color=proj_colors[idx], edgecolor="black", alpha=0.85)
    bottoms += np.array(proj_vals)

ax.set_ylabel("Number of Sprouted Synapses", fontsize=11, fontweight="bold")
ax.set_title("Composition of Sprouted Synapses by Projection Class", fontsize=13, fontweight="bold", pad=15)
ax.legend(title="Projection Classes", loc="upper left", bbox_to_anchor=(1.02, 1))
ax.grid(axis='y', linestyle='--', alpha=0.5)

# Label total count on top of stacks
for idx, val in enumerate(bottoms):
    ax.annotate(f"Total: {int(val)}", xy=(idx, val), xytext=(0, 5), textcoords="offset points", ha="center", va="bottom", fontweight="bold")

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "projection_composition.png"), dpi=300)
plt.close()
print("projection_composition.png generated.")

# 3. 3D Spatial Scatter of New Synapses for Winner Policy
winner_policy = next((p for p in policies_data if p["name"] == "under_recruited_plus_diversity_plus_stochastic_geometry"), None)
if winner_policy and winner_policy["sprouted_coords"]:
    coords = winner_policy["sprouted_coords"]
    xs = [c["x"] for c in coords]
    ys = [c["y"] for c in coords]
    zs = [c["z"] for c in coords]
    projs = [c["proj"] for c in coords]

    fig = plt.figure(figsize=(10, 8), dpi=300)
    ax = fig.add_subplot(111, projection='3d')

    # Assign color to each projection class for consistency
    proj_color_map = {name: color for name, color in zip(proj_names, proj_colors)}
    colors = [proj_color_map.get(p, "grey") for p in projs]

    scatter = ax.scatter(xs, ys, zs, c=colors, s=50, edgecolor='k', alpha=0.8)

    ax.set_xlabel("X (um)", fontsize=10, fontweight="bold")
    ax.set_ylabel("Y (um)", fontsize=10, fontweight="bold")
    ax.set_zlabel("Z (um)", fontsize=10, fontweight="bold")
    ax.set_title("3D Spatial Distribution of Sprouted Synapses\n(Stochastic Geometry Winner Policy)", fontsize=12, fontweight="bold", pad=15)

    # Create manual legend handles for the projections plotted
    import matplotlib.patches as mpatches
    legend_handles = []
    unique_projs = sorted(list(set(projs)))
    for up in unique_projs:
        legend_handles.append(mpatches.Patch(color=proj_color_map.get(up, "grey"), label=up))
    ax.legend(handles=legend_handles, title="Projection Class", loc="upper left")

    plt.tight_layout()
    plt.savefig(os.path.join(images_dir, "sprout_spatial.png"), dpi=300)
    plt.close()
    print("sprout_spatial.png generated.")
else:
    print("No sprouted coordinates found for winner policy or winner policy not present. Skipping 3D scatter.")
