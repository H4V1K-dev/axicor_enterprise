#!/usr/bin/env python3
"""Baker Spatial Growth Audit v1 — Analysis and Visualization.

Reads topology stats JSON from the Rust test runner and produces:
1. soma_positions_2d.png — 2D projections (XY, XZ) colored by layer/type
2. projection_matrix_heatmap.png — source_type × target_type synapse count heatmap
3. fanin_fanout_distribution.png — box plots per layer
4. distance_distribution_by_projection.png — histogram per projection pair
5. ei_balance_by_layer.png — stacked bar: E/I counts and weight mass per target layer
6. dendrite_slot_usage.png — histogram of used slots per soma, by layer
7. segment_offset_distribution.png — histogram of synapse segment offsets per projection
Also writes:
- baker_spatial_growth_summary.json (verdict + key numbers)
- baker_spatial_growth_audit_v1.md (report)
"""
import json
import os
import sys
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from pathlib import Path

# ---- Paths ----
SCRIPT_DIR = Path(__file__).resolve().parent
ARCHIVE_DIR = SCRIPT_DIR.parent
IMAGES_DIR = ARCHIVE_DIR / "images"
REPORTS_DIR = ARCHIVE_DIR / "reports"
ARTIFACTS_DIR_ARCHIVE = ARCHIVE_DIR / "artifacts"

# Source artifacts from workflow/artifacts
WORKFLOW_DIR = SCRIPT_DIR
for _ in range(6):  # scripts -> date_dir -> archive -> research -> engine -> docs -> workflow
    WORKFLOW_DIR = WORKFLOW_DIR.parent
ARTIFACTS_SRC = WORKFLOW_DIR / "artifacts"

IMAGES_DIR.mkdir(parents=True, exist_ok=True)
REPORTS_DIR.mkdir(parents=True, exist_ok=True)
ARTIFACTS_DIR_ARCHIVE.mkdir(parents=True, exist_ok=True)

# ---- Load Data ----
topo_path = ARTIFACTS_SRC / "baker_spatial_growth_topology_stats.json"
variance_path = ARTIFACTS_SRC / "baker_spatial_growth_seed_variance.json"

if not topo_path.exists():
    print(f"ERROR: {topo_path} not found. Run the Rust test first.")
    sys.exit(1)

with open(topo_path) as f:
    data = json.load(f)

variance_data = None
if variance_path.exists():
    with open(variance_path) as f:
        variance_data = json.load(f)

# ---- Constants ----
TYPE_NAMES = ["VirtualInput", "L4_spiny", "L23_aspiny", "L5_spiny"]
LAYER_NAMES = ["Virtual", "L4", "L23", "L5"]
COLORS = ["#7b68ee", "#ff6347", "#20b2aa", "#ffa500"]  # medslateblue, tomato, lightseagreen, orange
EXPECTED_PROJECTIONS = {
    ("VirtualInput", "L4_spiny"),
    ("L4_spiny", "L23_aspiny"),
    ("L4_spiny", "L5_spiny"),
    ("L23_aspiny", "L4_spiny"),
    ("L23_aspiny", "L23_aspiny"),
    ("L23_aspiny", "L5_spiny"),
    ("L5_spiny", "L23_aspiny"),
}

# ---- Extract data ----
soma_positions = data["soma_positions"]
proj_matrix = data["projection_matrix"]
fan_in_by_type = data["fan_in_by_type"]
fan_out_by_type = data["fan_out_by_type"]
slot_usage_by_type = data["dendrite_slot_usage_by_type"]
ei_balance = data["ei_balance"]
total_somas = data["total_somas"]
total_synapses = data["total_synapses"]
dropped = data["dropped_candidates"]

print(f"Total somas: {total_somas}, Total synapses: {total_synapses}, Dropped: {dropped}")

# ============================================================
# Plot 1: Soma Positions 2D projections (XY and XZ)
# ============================================================
fig, axes = plt.subplots(1, 2, figsize=(14, 6))
fig.suptitle("Soma Positions — Baker Spatial Growth (seed 12345)", fontsize=14, fontweight="bold")

for tid in range(4):
    pts = [(s["x"], s["y"], s["z"]) for s in soma_positions if s["type_id"] == tid]
    if not pts:
        continue
    xs, ys, zs = zip(*pts)
    axes[0].scatter(xs, ys, c=COLORS[tid], label=LAYER_NAMES[tid], s=12, alpha=0.7, edgecolors="none")
    axes[1].scatter(xs, zs, c=COLORS[tid], label=LAYER_NAMES[tid], s=12, alpha=0.7, edgecolors="none")

axes[0].set_xlabel("X (voxels)")
axes[0].set_ylabel("Y (voxels)")
axes[0].set_title("XY Projection (top-down)")
axes[0].legend(fontsize=9)
axes[0].set_aspect("equal")
axes[0].grid(True, alpha=0.3)

axes[1].set_xlabel("X (voxels)")
axes[1].set_ylabel("Z (voxels)")
axes[1].set_title("XZ Projection (side view)")
axes[1].legend(fontsize=9)
axes[1].grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig(IMAGES_DIR / "soma_positions_2d.png", dpi=150)
plt.close()
print("Wrote soma_positions_2d.png")

# ============================================================
# Plot 2: Projection Matrix Heatmap
# ============================================================
mat = np.zeros((4, 4), dtype=int)
for p in proj_matrix:
    si = TYPE_NAMES.index(p["source"])
    ti = TYPE_NAMES.index(p["target"])
    mat[si][ti] = p["synapse_count"]

fig, ax = plt.subplots(figsize=(8, 6))
im = ax.imshow(mat, cmap="YlOrRd", aspect="auto")
ax.set_xticks(range(4))
ax.set_xticklabels(LAYER_NAMES, fontsize=10)
ax.set_yticks(range(4))
ax.set_yticklabels(LAYER_NAMES, fontsize=10)
ax.set_xlabel("Target Layer", fontsize=11)
ax.set_ylabel("Source Layer", fontsize=11)
ax.set_title("Projection Matrix — Synapse Count\n(Baker Spatial Growth, seed 12345)", fontsize=12, fontweight="bold")

for i in range(4):
    for j in range(4):
        val = mat[i][j]
        pair = (TYPE_NAMES[i], TYPE_NAMES[j])
        marker = "✓" if pair in EXPECTED_PROJECTIONS else ("✗" if val > 0 else "")
        color = "white" if val > mat.max() * 0.6 else "black"
        ax.text(j, i, f"{val}\n{marker}", ha="center", va="center", fontsize=9, color=color)

plt.colorbar(im, ax=ax, label="Synapse Count")
plt.tight_layout()
plt.savefig(IMAGES_DIR / "projection_matrix_heatmap.png", dpi=150)
plt.close()
print("Wrote projection_matrix_heatmap.png")

# ============================================================
# Plot 3: Fan-In / Fan-Out Distribution
# ============================================================
fig, axes = plt.subplots(1, 2, figsize=(14, 6))
fig.suptitle("Fan-In / Fan-Out Distribution by Layer", fontsize=14, fontweight="bold")

# Collect per-soma fan-in/fan-out from raw data
fi_data = []
fo_data = []
fi_labels = []
fo_labels = []

for i, entry in enumerate(fan_in_by_type):
    stats = entry["stats"]
    n = stats["count"]
    if n == 0:
        continue
    # Reconstruct approximate distribution from stats
    fi_labels.append(LAYER_NAMES[i])
    fo_labels.append(LAYER_NAMES[i])
    # Use the stats summary values for a box-like representation
    fi_data.append({
        "mean": stats["mean"], "median": stats["median"],
        "p10": stats["p10"], "p90": stats["p90"],
        "min": stats["min"], "max": stats["max"],
        "zi": entry["zero_input_fraction"]
    })

for i, entry in enumerate(fan_out_by_type):
    stats = entry["stats"]
    fo_data.append({
        "mean": stats["mean"], "median": stats["median"],
        "p10": stats["p10"], "p90": stats["p90"],
        "min": stats["min"], "max": stats["max"],
        "zo": entry["zero_output_fraction"]
    })

# Fan-In bar chart
x = np.arange(len(fi_labels))
means = [d["mean"] for d in fi_data]
medians = [d["median"] for d in fi_data]
p10s = [d["p10"] for d in fi_data]
p90s = [d["p90"] for d in fi_data]
mins = [d["min"] for d in fi_data]
maxs = [d["max"] for d in fi_data]

axes[0].bar(x - 0.15, means, 0.3, label="Mean", color=COLORS[:len(fi_labels)], alpha=0.8)
axes[0].bar(x + 0.15, medians, 0.3, label="Median", color=COLORS[:len(fi_labels)], alpha=0.5)
yerr_lo = np.maximum(np.array(means) - np.array(p10s), 0)
yerr_hi = np.maximum(np.array(p90s) - np.array(means), 0)
axes[0].errorbar(x, means, yerr=[yerr_lo, yerr_hi],
                 fmt="none", ecolor="black", capsize=4, label="p10-p90")
axes[0].set_xticks(x)
axes[0].set_xticklabels(fi_labels)
axes[0].set_ylabel("Fan-In (synapses)")
axes[0].set_title("Fan-In per Target Layer")
axes[0].legend(fontsize=8)
axes[0].grid(True, alpha=0.3, axis="y")

# Fan-Out bar chart
fo_means = [d["mean"] for d in fo_data]
fo_medians = [d["median"] for d in fo_data]
fo_p10s = [d["p10"] for d in fo_data]
fo_p90s = [d["p90"] for d in fo_data]

axes[1].bar(x - 0.15, fo_means, 0.3, label="Mean", color=COLORS[:len(fo_labels)], alpha=0.8)
axes[1].bar(x + 0.15, fo_medians, 0.3, label="Median", color=COLORS[:len(fo_labels)], alpha=0.5)
fo_yerr_lo = np.maximum(np.array(fo_means) - np.array(fo_p10s), 0)
fo_yerr_hi = np.maximum(np.array(fo_p90s) - np.array(fo_means), 0)
axes[1].errorbar(x, fo_means, yerr=[fo_yerr_lo, fo_yerr_hi],
                 fmt="none", ecolor="black", capsize=4, label="p10-p90")

axes[1].set_xticks(x)
axes[1].set_xticklabels(fo_labels)
axes[1].set_ylabel("Fan-Out (synapses)")
axes[1].set_title("Fan-Out per Source Layer")
axes[1].legend(fontsize=8)
axes[1].grid(True, alpha=0.3, axis="y")

plt.tight_layout()
plt.savefig(IMAGES_DIR / "fanin_fanout_distribution.png", dpi=150)
plt.close()
print("Wrote fanin_fanout_distribution.png")

# ============================================================
# Plot 4: Distance Distribution by Projection
# ============================================================
active_projs = [(p["source"], p["target"], p["distances"]) for p in proj_matrix if p["synapse_count"] > 0]
n_projs = len(active_projs)

if n_projs > 0:
    cols = min(3, n_projs)
    rows = (n_projs + cols - 1) // cols
    fig, axes = plt.subplots(rows, cols, figsize=(5 * cols, 4 * rows), squeeze=False)
    fig.suptitle("Soma-to-Soma Distance Distribution by Projection", fontsize=14, fontweight="bold")

    for idx, (src, tgt, dists) in enumerate(active_projs):
        r, c = divmod(idx, cols)
        ax = axes[r][c]
        if len(dists) > 0:
            ax.hist(dists, bins=30, color=COLORS[TYPE_NAMES.index(src) % 4], alpha=0.8, edgecolor="white")
        is_expected = (src, tgt) in EXPECTED_PROJECTIONS
        marker = " ✓" if is_expected else " ✗"
        short_src = src.replace("_spiny", "").replace("_aspiny", "").replace("Input", "")
        short_tgt = tgt.replace("_spiny", "").replace("_aspiny", "").replace("Input", "")
        ax.set_title(f"{short_src}→{short_tgt}{marker}", fontsize=10)
        ax.set_xlabel("Distance (voxels)", fontsize=8)
        ax.set_ylabel("Count", fontsize=8)
        ax.grid(True, alpha=0.3)

    # Hide unused axes
    for idx in range(n_projs, rows * cols):
        r, c = divmod(idx, cols)
        axes[r][c].set_visible(False)

    plt.tight_layout()
    plt.savefig(IMAGES_DIR / "distance_distribution_by_projection.png", dpi=150)
    plt.close()
    print("Wrote distance_distribution_by_projection.png")

# ============================================================
# Plot 5: E/I Balance by Layer
# ============================================================
fig, axes = plt.subplots(1, 2, figsize=(14, 6))
fig.suptitle("E/I Balance by Target Layer", fontsize=14, fontweight="bold")

layers_with_synapses = [e for e in ei_balance if e["total_synapses"] > 0]
labels = [e["target_layer"] for e in layers_with_synapses]
exc_counts = [e["excitatory_count"] for e in layers_with_synapses]
inh_counts = [e["inhibitory_count"] for e in layers_with_synapses]
exc_mass = [e["excitatory_weight_mass"] for e in layers_with_synapses]
inh_mass = [e["inhibitory_weight_mass"] for e in layers_with_synapses]

x = np.arange(len(labels))
width = 0.35

axes[0].bar(x - width/2, exc_counts, width, label="Excitatory", color="#4CAF50", alpha=0.8)
axes[0].bar(x + width/2, inh_counts, width, label="Inhibitory", color="#F44336", alpha=0.8)
axes[0].set_xticks(x)
axes[0].set_xticklabels(labels)
axes[0].set_ylabel("Synapse Count")
axes[0].set_title("E/I Synapse Count")
axes[0].legend()
axes[0].grid(True, alpha=0.3, axis="y")

# Add E/I ratio text
for i, e in enumerate(layers_with_synapses):
    ratio = e["ei_count_ratio"]
    ratio_str = f"E/I={ratio:.2f}" if ratio != float('inf') and ratio > 0 else "E only"
    axes[0].text(i, max(exc_counts[i], inh_counts[i]) * 1.05, ratio_str,
                 ha="center", fontsize=8, fontweight="bold")

axes[1].bar(x - width/2, [m / 1e6 for m in exc_mass], width, label="Excitatory", color="#4CAF50", alpha=0.8)
axes[1].bar(x + width/2, [m / 1e6 for m in inh_mass], width, label="Inhibitory", color="#F44336", alpha=0.8)
axes[1].set_xticks(x)
axes[1].set_xticklabels(labels)
axes[1].set_ylabel("Weight Mass (×10⁶)")
axes[1].set_title("E/I Weight Mass")
axes[1].legend()
axes[1].grid(True, alpha=0.3, axis="y")

plt.tight_layout()
plt.savefig(IMAGES_DIR / "ei_balance_by_layer.png", dpi=150)
plt.close()
print("Wrote ei_balance_by_layer.png")

# ============================================================
# Plot 6: Dendrite Slot Usage
# ============================================================
fig, ax = plt.subplots(figsize=(10, 6))
fig.suptitle("Dendrite Slot Usage by Target Layer", fontsize=14, fontweight="bold")

for i, entry in enumerate(slot_usage_by_type):
    stats = entry["stats"]
    if stats["count"] == 0:
        continue
    ax.bar(i, stats["mean"], color=COLORS[i], alpha=0.8, label=LAYER_NAMES[i])
    yerr_lo = max(stats["mean"] - stats["p10"], 0)
    yerr_hi = max(stats["p90"] - stats["mean"], 0)
    ax.errorbar(i, stats["mean"],
                yerr=[[yerr_lo], [yerr_hi]],
                fmt="none", ecolor="black", capsize=6)
    ax.text(i, stats["max"] + 1, f"max={stats['max']}", ha="center", fontsize=8)

ax.axhline(y=128, color="red", linestyle="--", alpha=0.5, label="MAX_DENDRITES=128")
ax.set_xticks(range(4))
ax.set_xticklabels(LAYER_NAMES)
ax.set_ylabel("Used Dendrite Slots")
ax.set_title("Mean Dendrite Slot Usage (p10-p90 bars)")
ax.legend(fontsize=9)
ax.grid(True, alpha=0.3, axis="y")

plt.tight_layout()
plt.savefig(IMAGES_DIR / "dendrite_slot_usage.png", dpi=150)
plt.close()
print("Wrote dendrite_slot_usage.png")

# ============================================================
# Plot 7: Segment Offset Distribution by Projection
# ============================================================
if n_projs > 0:
    active_segs = [(p["source"], p["target"], p["segment_offsets"]) for p in proj_matrix if p["synapse_count"] > 0]
    n_seg_projs = len(active_segs)
    cols = min(3, n_seg_projs)
    rows = (n_seg_projs + cols - 1) // cols
    fig, axes = plt.subplots(rows, cols, figsize=(5 * cols, 4 * rows), squeeze=False)
    fig.suptitle("Segment Offset Distribution by Projection", fontsize=14, fontweight="bold")

    for idx, (src, tgt, segs) in enumerate(active_segs):
        r, c = divmod(idx, cols)
        ax = axes[r][c]
        if len(segs) > 0:
            ax.hist(segs, bins=range(0, max(segs) + 2), color=COLORS[TYPE_NAMES.index(src) % 4],
                    alpha=0.8, edgecolor="white")
        short_src = src.replace("_spiny", "").replace("_aspiny", "").replace("Input", "")
        short_tgt = tgt.replace("_spiny", "").replace("_aspiny", "").replace("Input", "")
        ax.set_title(f"{short_src}→{short_tgt}", fontsize=10)
        ax.set_xlabel("Segment Offset", fontsize=8)
        ax.set_ylabel("Count", fontsize=8)
        ax.grid(True, alpha=0.3)

    for idx in range(n_seg_projs, rows * cols):
        r, c = divmod(idx, cols)
        axes[r][c].set_visible(False)

    plt.tight_layout()
    plt.savefig(IMAGES_DIR / "segment_offset_distribution.png", dpi=150)
    plt.close()
    print("Wrote segment_offset_distribution.png")

# ============================================================
# Verdict Logic
# ============================================================
print("\n=== Verdict Computation ===")

# Check 1: All expected projections present
present_pairs = set()
for p in proj_matrix:
    if p["synapse_count"] > 0:
        present_pairs.add((p["source"], p["target"]))

missing_expected = EXPECTED_PROJECTIONS - present_pairs
unexpected = present_pairs - EXPECTED_PROJECTIONS

# Check 2: Zero-input pathology
zero_input_issues = []
for entry in fan_in_by_type:
    if entry["type_name"] == "VirtualInput":
        # VirtualInput is an input-only source layer in this audit. Zero fan-in is expected.
        continue
    if entry["zero_input_fraction"] > 0.5:
        zero_input_issues.append(f"{entry['type_name']}: {entry['zero_input_fraction']*100:.1f}% zero-input")

# Check 3: Fan-in/fan-out concentration
fanin_pathology = []
for entry in fan_in_by_type:
    stats = entry["stats"]
    if stats["count"] > 0 and stats["max"] >= 128:
        fanin_pathology.append(f"{entry['type_name']}: max fan-in={stats['max']} (saturated)")

# Check 4: Dominant unexpected
dominant_unexpected = []
for p in proj_matrix:
    pair = (p["source"], p["target"])
    if pair not in EXPECTED_PROJECTIONS and p["synapse_count"] > 0:
        pct = float(p["percent_of_all"])
        if pct > 10.0:
            dominant_unexpected.append(f"{p['source']}→{p['target']}: {pct:.1f}%")

# Determine verdict
issues = []
if missing_expected:
    issues.append(f"MISSING expected projections: {missing_expected}")
if dominant_unexpected:
    issues.append(f"DOMINANT unexpected projections: {dominant_unexpected}")
if zero_input_issues:
    issues.append(f"Zero-input pathology: {zero_input_issues}")

warnings = []
if unexpected:
    warnings.append(f"Unexpected (non-dominant) projections present: {unexpected}")
if fanin_pathology:
    warnings.append(f"Fan-in saturation: {fanin_pathology}")

if issues:
    verdict = "FAIL"
    verdict_detail = "; ".join(issues)
elif warnings:
    verdict = "PARTIAL / topology imbalance"
    verdict_detail = "; ".join(warnings)
else:
    verdict = "PASS"
    verdict_detail = "All expected projections present, no dominant unexpected, no zero-input pathology, distances and segment offsets measured from real data."

print(f"Verdict: {verdict}")
print(f"Detail: {verdict_detail}")

# ============================================================
# Write Summary JSON
# ============================================================
summary = {
    "verdict": verdict,
    "verdict_detail": verdict_detail,
    "total_somas": total_somas,
    "somas_by_type": data["somas_by_type"],
    "total_synapses": total_synapses,
    "dropped_candidates": dropped,
    "expected_projections_present": list(present_pairs & EXPECTED_PROJECTIONS),
    "missing_expected_projections": [list(p) for p in missing_expected],
    "unexpected_projections": [list(p) for p in unexpected],
    "projection_summary": [
        {
            "source": p["source"],
            "target": p["target"],
            "count": p["synapse_count"],
            "pct": p["percent_of_all"],
            "sign": p["sign"],
            "expected": p["expected"],
            "distance_mean": p["distance_mean"],
            "distance_median": p["distance_median"],
            "mean_weight": p["mean_initial_weight"],
            "segment_offset_mean": p["segment_offset_stats"]["mean"],
        }
        for p in proj_matrix
    ],
    "fan_in_summary": [
        {
            "layer": entry["layer"],
            "mean": entry["stats"]["mean"],
            "median": entry["stats"]["median"],
            "p10": entry["stats"]["p10"],
            "p90": entry["stats"]["p90"],
            "min": entry["stats"]["min"],
            "max": entry["stats"]["max"],
            "zero_input_fraction": entry["zero_input_fraction"],
        }
        for entry in fan_in_by_type
    ],
    "fan_out_summary": [
        {
            "layer": entry["layer"],
            "mean": entry["stats"]["mean"],
            "median": entry["stats"]["median"],
            "p10": entry["stats"]["p10"],
            "p90": entry["stats"]["p90"],
            "min": entry["stats"]["min"],
            "max": entry["stats"]["max"],
            "zero_output_fraction": entry["zero_output_fraction"],
        }
        for entry in fan_out_by_type
    ],
    "ei_balance_summary": [
        {
            "layer": e["target_layer"],
            "exc": e["excitatory_count"],
            "inh": e["inhibitory_count"],
            "ratio": e["ei_count_ratio"] if e["ei_count_ratio"] != float("inf") else "inf",
        }
        for e in ei_balance
    ],
    "seed_variance": variance_data,
    "warnings": warnings,
    "issues": issues,
}

summary_path = ARTIFACTS_DIR_ARCHIVE / "baker_spatial_growth_summary.json"
with open(summary_path, "w") as f:
    json.dump(summary, f, indent=2, default=str)
print(f"Wrote {summary_path}")

# Also copy source artifacts to archive
import shutil
for fname in ["baker_spatial_growth_topology_stats.json",
              "baker_spatial_growth_projection_matrix.json",
              "baker_spatial_growth_seed_variance.json"]:
    src = ARTIFACTS_SRC / fname
    if src.exists():
        shutil.copy2(src, ARTIFACTS_DIR_ARCHIVE / fname)
        print(f"Copied {fname} to archive")

print("\n=== Baker Spatial Growth Analysis Complete ===")
print(f"Verdict: {verdict}")
