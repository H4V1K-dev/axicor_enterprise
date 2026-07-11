# Night Phase Activity Counters Baseline (v0.5)

This directory contains research code and reports for validating local synapse and soma activity counters during active day phase learning and nightly structural trace merge.

## Research Goal
Validate whether we can collect cheap, local activity/coactivity counters during the day phase and merge them at night into short/long structural traces to capture structural signal, without modifying network dynamics.

## Methodology
- **Winner Topology**: Growth v2 C17 winner (`Radius_9_Cap_96_Pair_2_ProjAware`).
- **Simulations**:
  - `baseline_no_counters`: control run, no counters collected.
  - `counters_collect_only`: counters are collected but not merged at night.
  - `counters_collect_and_merge`: counters are collected and trace merge is executed at night.
- **Verification**: Ensure all three runs have identical weights and firing rates on Day 2 replay (hard gate).

## Contents
- `reports/night_phase_activity_counters_v0_5.md`: The detailed scientific report.
- `scripts/generate_plots_v0_5.py`: Python script to generate coactivity and long trace distribution plots.
- `artifacts/plot_data.json`: Raw metrics output from the test run.
- `images/coactivity_signal_distribution.png`: Signal separation plot.

## How to Run
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test night_phase_activity_counters_v0_5 -- --nocapture
```
