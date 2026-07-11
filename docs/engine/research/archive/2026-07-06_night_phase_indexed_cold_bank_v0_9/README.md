# Night Phase Indexed Cold Bank Evidence (v0.9)

This research experiment evaluates production-like reactivation methods for the Dormant/Cold Storage Bank that do not require scanning the dormant synapse set during day-phase simulation ticks.

## Objective
We compare the exact per-tick scanning oracle (v0.8) with two hashset-based day-phase event indexing strategies:
1. **`dormant_indexed_any_day`**: Records coarse any-day pre-axon segment hits and post-soma target spikes.
2. **`dormant_indexed_bucketed`**: Records hits and spikes inside 8-tick time buckets, matching them on Night 2.
3. **`dormant_indexed_bucketed_plus_trace`**: Enhances the bucketed policy by requiring that the synapse also has a positive short or long trace (`short_trace > 0 || long_trace > 0`).

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics for all 5 policies, including overlap precision, recall, Jaccard scores, and computational check costs.
- `scripts/generate_plots_v0_9.py`: Python visualization script utilizing Matplotlib.
- `reports/night_phase_indexed_cold_bank_v0_9.md`: Scientific report documenting findings.
- `images/reactivation_comparison.png`: Chart showing reactivated counts and overlap metrics.
- `images/cost_comparison.png`: Log-scale cost bar chart comparing scan checks vs. indexed checks.
