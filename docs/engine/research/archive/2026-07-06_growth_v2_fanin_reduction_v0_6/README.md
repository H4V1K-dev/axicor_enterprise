# Growth v2 Fan-in Pressure Reduction (v0.6) Research Archive

This archive contains the code, data, plots, and scientific report for the Growth v2 Fan-in Pressure Reduction (v0.6) research audit.

## Purpose
The purpose of this audit was to systematically reduce the post-synaptic fan-in pressure and target soma saturation observed in the v0.5 Balanced topology candidate (which had 168 fully saturated somas at the hardware cap of 128 synapses), while preserving all functional projections, static stability, and GSOP matched-bias learning.

## Contents
- `reports/growth_v2_fanin_reduction_v0_6.md` — The detailed scientific report answering the 8 core research questions.
- `scripts/plot_growth_v2_fanin_reduction.py` — The Python visualization script that reads the exported JSON and generates 10 comparison plots.
- `images/` — Contains the 10 generated PNG plots:
  1. `projection_heatmap_comparison.png` — Synapse count heatmaps for Baseline vs Winner 1 vs Winner 2.
  2. `fanin_histogram_comparison.png` — In-degree distribution comparison showing hardware cap safety.
  3. `saturated_target_count.png` — Saturation count across all 24 swept configurations.
  4. `synapses_projection_comparison.png` — Synapse counts and critical L4->L5 projections across all sweep configurations.
  5. `stream_compile_audit.png` — Stream counts (total, active, dropped) and memory segment counts.
  6. `layer_firing_rates.png` — Spiking rates over 10,000 ticks for Baseline vs Winner 1 vs Winner 2.
  7. `active_fractions.png` — Recruitment active fraction over time.
  8. `matched_vs_unmatched.png` — GSOP learning matched vs unmatched delta bar chart.
  9. `weight_delta_histogram.png` — Synaptic weight changes delta distribution.
  10. `pareto_fanin_vs_matched_bias.png` — Pareto frontier comparing fan-in pressure against learning matched bias.
- `artifacts/` — Primary JSON logs and data files:
  - `growth_v2_fanin_reduction_plot_data.json` — Swept configuration stats and replay trajectories.
  - `baker_growth_v2_summary.json` — Summary verdict and final metrics.

## Key Outcomes
- **Successful Pressure Reduction**: Saturated target somas (fan-in = 128) reduced from **168 to 0**.
- **p90 Fan-in Safety**: p90 fan-in dropped from **128 to 96**.
- **Projection Preservation**: All 7 expected projections are preserved. Crucially, the **projection-aware capping policy** maintained **606 L4->L5 synapses** (Winner 2) compared to only 399 in the standard cap (Winner 1).
- **Stability and Learning**: Spiking replays are active and stable, and GSOP matched bias remains strongly positive (+303k vs +29k).
