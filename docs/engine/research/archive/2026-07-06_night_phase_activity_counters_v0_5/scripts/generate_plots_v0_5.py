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

ratios = np.array(data["coactivity_ratios"])
long_traces = np.array(data["long_traces"])
labels = np.array(data["labels"])

matched_ratios = ratios[labels == "matched"]
unmatched_ratios = ratios[labels == "unmatched"]
other_ratios = ratios[labels == "other"]

matched_long = long_traces[labels == "matched"]
unmatched_long = long_traces[labels == "unmatched"]
other_long = long_traces[labels == "other"]

plt.style.use('seaborn-v0_8-whitegrid' if 'seaborn-v0_8-whitegrid' in plt.style.available else 'default')

fig, axes = plt.subplots(1, 2, figsize=(14, 5))

# Subplot 1: Coactivity Ratio Distribution
axes[0].hist(other_ratios, bins=40, alpha=0.4, label='Other Synapses (Background)', color='#7f7f7f', density=True)
axes[0].hist(unmatched_ratios, bins=40, alpha=0.6, label='Unmatched (Virtual B -> L4)', color='#d62728', density=True)
axes[0].hist(matched_ratios, bins=40, alpha=0.6, label='Matched (Virtual A -> L4)', color='#2ca02c', density=True)
axes[0].set_title('Coactivity Eligibility Ratio (coactivity_hits / pre_hits)', fontsize=11, fontweight='bold')
axes[0].set_xlabel('Coactivity Eligibility Ratio', fontsize=10)
axes[0].set_ylabel('Density', fontsize=10)
axes[0].legend(fontsize=9)

# Subplot 2: Long Trace Distribution
axes[1].hist(other_long, bins=40, alpha=0.4, label='Other Synapses (Background)', color='#7f7f7f', density=True)
axes[1].hist(unmatched_long, bins=40, alpha=0.6, label='Unmatched (Virtual B -> L4)', color='#d62728', density=True)
axes[1].hist(matched_long, bins=40, alpha=0.6, label='Matched (Virtual A -> L4)', color='#2ca02c', density=True)
axes[1].set_title('Long Structural Trace Distribution (long_trace)', fontsize=11, fontweight='bold')
axes[1].set_xlabel('Long Trace Value', fontsize=10)
axes[1].set_ylabel('Density', fontsize=10)
axes[1].legend(fontsize=9)

plt.suptitle('Night Phase Activity Counters: Signal Quality & Separation', fontsize=13, fontweight='bold')
plt.tight_layout()

# Save image
img_dir = os.path.join(script_dir, "..", "images")
os.makedirs(img_dir, exist_ok=True)
plt.savefig(os.path.join(img_dir, "coactivity_signal_distribution.png"), dpi=150)
plt.close()

print("Plot coactivity_signal_distribution.png generated successfully.")
