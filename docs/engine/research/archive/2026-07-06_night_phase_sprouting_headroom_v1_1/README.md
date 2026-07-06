# Night Phase Sprouting Headroom (v1.1)

This research experiment evaluates homeostatic night phase sprouting policies under corrected topology semantics to isolate un-pruned dense topology saturation from post-pruning headroom, while strictly preserving physical invariants (source-target pair cap = 2).

## Objective & Topologies Evaluated
We evaluate 3 topology variations across 3 sprouting policies under strict physical invariants (Dale's law, max fan-in capacity 96, and strict per-soma source-target cap of 2):

1. **`saturated_C17_control`**: Standard un-pruned dense C17 benchmark topology (22,037 active synapses). Sprouting is run directly without pruning to measure raw topological saturation.
2. **`headroom_C17_pair1`**: Initial topology initialization restricted to at most 1 synapse per `(source_soma, target_soma)` pair (11,648 active synapses). Sprouting is run directly without pruning under runtime invariant `pair_cap = 2`.
3. **`post_prune_headroom`**: Standard C17 topology evaluated after Night 1 pruning/Dormant Bank demotion, testing whether natural night pruning frees sufficient headroom for sprouting.

## Sprouting Policies
1. **`no_sprouting_baseline`**: Baseline without sprouting pass.
2. **`deterministic_under_recruited_projection_diversity`**: Distance-ordered spatial selection with hard projection diversity filtering.
3. **`stochastic_geometry_projection_diversity`**: Distance-weighted stochastic sampling ($w \propto e^{-\beta d^2}$) with soft diversity multiplier (3x weight for under-represented projections).

## Key Findings
- **Un-pruned Saturation**: `stochastic_geometry` on un-pruned `saturated_C17_control` yields **0 sprouted synapses** due to 18,265 `pair_cap_blocked` candidate rejections.
- **Headroom Enables Stochastic Sprouting**: In `headroom_C17_pair1` (initial pair count = 1, runtime cap = 2), `pair_cap_blocked` drops to 0 and `stochastic_geometry` successfully sprouts **1,956 synapses**.
- **Post-Prune Headroom**: In `post_prune_headroom`, Night 1 pruning frees active capacity allowing `stochastic_geometry` to sprout **1,540 synapses**.
- **Physical Invariants Preserved**: All 9 evaluation runs strictly satisfy all safety gates (0 Dale violations, 0 dense target violations, 0 duplicate violations, 0 runaway ticks).

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across all 9 evaluation runs.
- `scripts/generate_plots_v1_1.py`: Matplotlib script generating visual comparisons.
- `reports/report_v1_1.md`: Scientific report documenting findings.
- `images/blocker_breakdown.png`: Rejection reason breakdown across topologies and policies.
- `images/projection_composition.png`: Composition of sprouted synapses by projection class.
- `images/fan_in_gini.png`: Comparison of Fan-in Gini coefficient and Top 5% Sprout Monopoly Share.
