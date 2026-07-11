# Night Phase MVP Consolidation (v1.6)

This research experiment evaluates the consolidated MVP Night Policy (`mvp_night_policy`) across 10 day/night cycles on a single shard topology, assessing stability, bounded dormant bank eviction, target under-recruitment sprouting, and invariant safety checks.

## Key Goals
- **Day/Night Lifecycle**: Network dynamics are simulated under standard Hebbian learning/STDP during day phases.
- **Trace Decay**: Saturating trace decay rules are applied to both active and dormant synapses.
- **Pruning**: Weak and inactive synapses are pruned to the Dormant Bank under target and projection coverage constraints to prevent structural collapse.
- **Bounded Eviction**: Double-bounded (per-target and global caps) eviction ensures the Dormant Bank does not grow monotonically or leak memory.
- **Sprouting**: Target under-recruitment triggers stochastic geometry sprouting into target headroom to stabilize network structure.
- **Safety Gates**: Asserts zero Dale, dense, duplicate, or runaway violations.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 10 cycles of the consolidated MVP lifecycle.
- `scripts/generate_plots_v1_6.py`: Python script utilizing `matplotlib` to generate the 2 required lifecycle plots.
- `reports/report_v1_6.md`: Detailed scientific report answering the key evaluation questions.
- `images/lifecycle_counts.png`: Active/Dormant/Dead synapse counts over cycles.
- `images/network_stability.png`: Firing dynamics, silence/runaway ticks, fan-in Gini coefficient, and projection coverage.
