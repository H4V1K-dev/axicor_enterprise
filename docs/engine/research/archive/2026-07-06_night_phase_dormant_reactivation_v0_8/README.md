# Night Phase Dormant Reactivation (v0.8)

This research experiment validates the reactivation mechanism of a Dormant/Cold Storage Bank prototype under budget-matched pruning pressure.

## Objective
Rather than reactivating dormant synapses based solely on historical long trace values, we implement a **side-channel day-phase candidate evidence collection** mechanism. If a dormant synapse receives concurrent pre-synaptic spikes and target post-synaptic activity during the returned context (Day 3), it registers context hits. At Night 2, these context hits are used to gate reactivation.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics for all 5 policies, including blocker breakdowns and reactivated synapses weight/age distribution.
- `scripts/generate_plots_v0_8.py`: Python visualization script utilizing Matplotlib/Numpy.
- `reports/night_phase_dormant_reactivation_v0_8.md`: Scientific report documenting the reactivation results.
- `images/counts_funnel.png`: Funnel chart showing reactivation rates and blockers.
- `images/retention_comparison.png`: Comparison of memory retention ratios across policies.
- `images/reactivated_synapses_distribution.png`: Weight and age distribution of reactivated synapses.
