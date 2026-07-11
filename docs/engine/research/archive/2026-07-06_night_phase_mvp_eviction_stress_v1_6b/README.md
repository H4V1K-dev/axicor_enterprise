# Night Phase MVP Eviction Stress (v1.6b)

This research experiment stress tests target-cap and global-cap Dormant Bank eviction under high pruning load over 12 cycles on a single shard topology.

## Key Goals
- **High Pruning Pressure**: `PRUNE_WEIGHT_THRESHOLD` is set to $1490 \times 2^{16}$ (close to initial weight $1500 \times 2^{16}$), causing thousands of synapses to prune.
- **Tight Bounded Eviction**: Dormant Bank caps are set to stress limits:
  - Global cap: 200 synapses (originally 500)
  - Target cap: 3 synapses per target (originally 10)
  - Max age: 2 cycles (originally 5)
- **Age/Trace Note**: The age+trace eviction path is present but did not fire in this run; retained dormant entries kept non-zero `long_trace` and were displaced by target/global caps first.
- **Homeostatic Sprouting**: Stochastic sprouting refuels target headroom under safety bounds.
- **Safety Gate Verification**: Ensures 0 Dale, dense, duplicate, runaway, or geometry violations.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 12 cycles of eviction stress.
- `scripts/generate_plots_v1_6b.py`: Python script utilizing `matplotlib` to render metrics.
- `reports/report_v1_6b.md`: Scientific report detailing stress results.
- `images/lifecycle_counts_v1_6b.png`: Synapse count and turnover under stress.
- `images/eviction_metrics_v1_6b.png`: Eviction counts by reason and dormant bank bounding.
- `images/sprouting_gini_v1_6b.png`: Sprouting distribution and Gini indices.
