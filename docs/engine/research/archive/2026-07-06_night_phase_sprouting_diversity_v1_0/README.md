# Night Phase Sprouting Diversity Research (v1.0)

This research experiment evaluates homeostatic night phase sprouting policies designed to recruit under-active somas while maintaining network invariants and preventing sprout monopoly.

## Objective
We evaluate 5 sprouting policies under strict physical invariants (Dale's law, max fan-in capacity 96, and strict per-soma source-target cap of 2):
1. **`no_sprouting_baseline`**: Baseline with pruning and dormant reactivation, but no sprouting.
2. **`active_source_greedy_sprouting`**: Connects under-recruited targets to active sources greedily.
3. **`under_recruited_target_sprouting`**: Sprouting directed at under-recruited targets ordered by spatial proximity.
4. **`under_recruited_plus_projection_diversity`**: Enforces target recruitment with projection class representation diversity.
5. **`under_recruited_plus_diversity_plus_stochastic_geometry`**: Distance-weighted stochastic sampling with soft diversity bonus.

## Diagnostic Summary
- **Safety Compliance**: All policies maintain 0 Dale violations, 0 dense target violations, 0 duplicate violations, and 0 runaway ticks.
- **Topology Saturation**: Stochastic geometry yields 0 sprouted connections under the strict `pair_cap = 2` constraint. In the dense C17 topology, almost all geometry-compatible source-target pairs already reach the cap of 2 synapses.
- **Conclusion**: v1.0 serves as a diagnostic result proving that invariant relaxation (e.g., raising pair cap to 4) is unnecessary and unphysical. The path forward is **v1.1 Sprouting Headroom**, creating structural capacity within the topology rather than weakening invariants.

## Directory Structure
- `artifacts/plot_data.json`: Metrics across all 5 evaluated policies.
- `scripts/generate_plots_v1_0.py`: Visualization script generating metric comparison charts.
- `reports/report_v1_0.md`: Scientific report documenting diagnostic findings.
- `images/fan_in_gini.png`: Comparison of Fan-in Gini coefficient and Sprout Monopoly share across policies.
- `images/projection_composition.png`: Stacked composition of sprouted synapses by projection class.
