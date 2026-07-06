# Night Phase Pair-History Prior Probe (v1.5)

This research experiment evaluates slow soma-to-soma structural history (`pair_history: HashMap<(u32, u32), PairHistory>`) as an initialization prior/bias (weight, traces, sprouting probability) across 5 policies (`baseline_fresh_sprout`, `pair_history_init_weight`, `pair_history_init_trace`, `pair_history_weight_plus_trace`, `pair_history_overstrong_stress`) and 2 branches (`returned_branch` vs `absent_branch` negative control).

## Status: DIAGNOSTIC / NEGATIVE RESULT
- **Safety Gates**: Standard policies maintain 0 Dale/dense/dup/runaway violations across 40 cycle evaluations. Overstrong stress triggers 7,820 total Dale violations on Cycles 4-5 as a failure boundary test.
- **Key Discovery**: Un-gated sprouting is inherently context-blind (sprouting 4,242 rare-labeled links in both returned and absent branches). Pair-history prior alone fails to solve this context-blindness.
- **Verdict**: Pair-history priors cannot be used as an independent recovery mechanism without active daytime co-gating.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 5 policies, 2 branches, and 5 cycles.
- `scripts/generate_plots_v1_5.py`: Visualization script for 3 comparison figures.
- `reports/report_v1_5.md`: Detailed scientific report.
- `images/recovery_vs_false_recovery.png`: True rare recovery vs False recovery on Cycle 5 across policies.
- `images/pair_history_mass_distribution.png`: P50 and P90 `pair_history.mass` over 5 cycles.
- `images/rare_cohort_lifecycle_comparison.png`: Rare cohort dynamics comparing `returned_branch` vs `absent_branch`.
