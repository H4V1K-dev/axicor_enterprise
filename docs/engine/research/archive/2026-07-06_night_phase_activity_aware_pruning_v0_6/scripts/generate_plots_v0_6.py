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
scatter_data = data["scatter"]

# Setup directories
images_dir = os.path.join(script_dir, "..", "images")
os.makedirs(images_dir, exist_ok=True)

# 1. Bar Chart: Cohort Survival / Deletion rates
policy_names = []
high_long_trace_survival = []
high_coactivity_survival = []
rare_useful_survival = []
low_evidence_deletion = []

for p in policies_data:
    policy_names.append(p["name"])
    high_long_trace_survival.append(p["high_long_trace_survival"] * 100)
    high_coactivity_survival.append(p["high_coactivity_survival"] * 100)
    rare_useful_survival.append(p["rare_useful_survival"] * 100)
    low_evidence_deletion.append(p["low_evidence_deletion"] * 100)

x = np.arange(len(policy_names))
width = 0.2

fig, ax = plt.subplots(figsize=(10, 6), dpi=300)
rects1 = ax.bar(x - 1.5*width, high_long_trace_survival, width, label="High Long Trace Survival %", color="#4e79a7")
rects2 = ax.bar(x - 0.5*width, high_coactivity_survival, width, label="High Coactivity Survival %", color="#f28e2b")
rects3 = ax.bar(x + 0.5*width, rare_useful_survival, width, label="Rare Useful Survival %", color="#e15759")
rects4 = ax.bar(x + 1.5*width, low_evidence_deletion, width, label="Low Evidence Deletion %", color="#76b7b2")

ax.set_ylabel("Percentage (%)", fontsize=11, fontweight="bold")
ax.set_title("Cohort Survival & Deletion Rates across Pruning Policies", fontsize=13, fontweight="bold", pad=15)
ax.set_xticks(x)
ax.set_xticklabels(policy_names, rotation=15, fontsize=10, fontweight="bold")
ax.set_ylim(0, 105)
ax.grid(axis='y', linestyle='--', alpha=0.5)
ax.legend(loc="upper right", frameon=True, shadow=True)

# Add values above bars
def autolabel(rects):
    for rect in rects:
        height = rect.get_height()
        ax.annotate(f"{height:.1f}%",
                    xy=(rect.get_x() + rect.get_width() / 2, height),
                    xytext=(0, 3),  # 3 points vertical offset
                    textcoords="offset points",
                    ha="center", va="bottom", fontsize=8)

autolabel(rects1)
autolabel(rects2)
autolabel(rects3)
autolabel(rects4)

plt.tight_layout()
plt.savefig(os.path.join(images_dir, "survival_matrix.png"), dpi=300)
plt.close()
print("survival_matrix.png generated successfully.")

# 2. Scatter Plot: abs(weight) vs long_trace (trace_score_projection_budget)
weights = []
traces = []
colors = []

for item in scatter_data:
    weights.append(item["weight_mass"])
    traces.append(item["long_trace"])
    
    if item["survived"]:
        colors.append("#2ca02c")
    else:
        colors.append("#d62728")

weights = np.array(weights)
traces = np.array(traces)
colors = np.array(colors)

plt.figure(figsize=(12, 7), dpi=300)

labels_set = ["matched", "unmatched", "other"]
label_display = {"matched": "Matched (Stimulated)", "unmatched": "Unmatched Control", "other": "Background"}
marker_map = {"matched": "o", "unmatched": "x", "other": "."}
alpha_map = {"matched": 0.8, "unmatched": 0.6, "other": 0.3}
size_map = {"matched": 50, "unmatched": 40, "other": 15}

for lbl in labels_set:
    mask = np.array([item["label"] == lbl for item in scatter_data])
    if not np.any(mask):
        continue
    
    for survived_state, state_lbl, col in [(True, "Survived", "#2ca02c"), (False, "Deleted", "#d62728")]:
        sub_mask = mask & np.array([item["survived"] == survived_state for item in scatter_data])
        if not np.any(sub_mask):
            continue
            
        plt.scatter(
            weights[sub_mask],
            traces[sub_mask],
            color=col,
            marker=marker_map[lbl],
            s=size_map[lbl],
            alpha=alpha_map[lbl],
            label=f"{label_display[lbl]} ({state_lbl})"
        )

plt.xlabel("Weight Magnitude (Mass)", fontsize=11, fontweight="bold")
plt.ylabel("Long structural trace value", fontsize=11, fontweight="bold")
plt.title("Synapse Survival under Trace-Score Projection-Budget Policy\n(Weight vs. Long Structural Trace)", fontsize=12, fontweight="bold", pad=15)
plt.grid(True, linestyle='--', alpha=0.5)
plt.legend(bbox_to_anchor=(1.02, 1), loc='upper left', frameon=True)
plt.tight_layout()
plt.savefig(os.path.join(images_dir, "weight_vs_trace_survival.png"), dpi=300)
plt.close()
print("weight_vs_trace_survival.png generated successfully.")
