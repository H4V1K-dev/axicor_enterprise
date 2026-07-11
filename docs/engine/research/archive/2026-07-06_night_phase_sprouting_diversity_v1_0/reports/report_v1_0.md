# Sprouting Diversity Research (v1.0) Scientific Report

**Date**: 2026-07-06  
**Status**: NEGATIVE / DIAGNOSTIC RESULT: Dense C17 Topology Saturated Under Strict Invariants  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase Sprouting Diversity (v1.0) investigated whether homeostatic sprouting could recruit under-active target somas without causing sprout monopoly or violating network invariants.

The key finding of v1.0 is **diagnostic**:
1. **Safety gates hold under sprouting**: Deterministic sprouting policies successfully add 417 new synapses while strictly satisfying all safety gates (0 Dale violations, 0 dense violations, 0 duplicate violations, 0 runaway ticks).
2. **Topology saturation blocks stochastic sprouting**: Stochastic geometry sampling yields 0 sprouted connections under the strict per-soma source-target pair cap of 2 (`pair_count < 2`). In the dense C17 benchmark network, geometry-compatible pairs already saturate the cap of 2.
3. **No invariant relaxation**: Weakening invariants (such as raising pair cap to 4 or disabling duplicate checks) is rejected. The correct architectural progression is **v1.1 Sprouting Headroom**, which explores topology pruning/slot freeing to create structural capacity without compromising physical constraints.

---

## 2. Evaluated Sprouting Policies

All policies run during Night 2 after Day 1 learning, Night 1 trace merge, Day 2 replay, and Day 3 returned context:

1. **`no_sprouting_baseline`**: Baseline run without any sprouting pass (pruning + dormant reactivation only).
2. **`active_source_greedy_sprouting`**: Connects under-recruited target somas to the closest active sources within spatial candidate pool.
3. **`under_recruited_target_sprouting`**: Prioritizes target somas with negative activity pressure (activity < target layer rate), sorting by distance.
4. **`under_recruited_plus_projection_diversity`**: Adds projection class diversity filter (`current_proj_count <= mean_proj_count`) to prevent single-projection flooding.
5. **`under_recruited_plus_diversity_plus_stochastic_geometry`**: Uses distance-weighted stochastic sampling ($w \propto e^{-\beta d^2}$) with soft diversity multiplier (3x weight for under-represented projections).

---

## 3. Physical Invariants & Constraints

- **Strict Source-Target Pair Cap**: Maximum 2 synapses per `(source_soma, target_soma)` pair.
- **Duplicate Check**: Duplicate violation triggered if `count > 2` per `(source_soma, target_soma)` pair or if exact `(source_soma, flat_segment_idx, target_soma)` triplet recurs.
- **Fan-in Limit**: Maximum 96 active synapses per target soma.
- **Dale's Law**: Strict separation of excitatory ($w > 0$) and inhibitory ($w < 0$) weights.

---

## 4. Results & Metrics Summary

| Metric / Policy | `no_sprouting_baseline` | `active_source_greedy` | `under_recruited_target` | `under_recruited_plus_proj_div` | `stochastic_geometry` |
|---|---|---|---|---|---|
| **Active Synapses (Day 4)** | 21,314 | 21,731 | 21,731 | 21,731 | 21,314 |
| **Dormant Synapses** | 723 | 723 | 723 | 723 | 723 |
| **Sprouted Synapses** | **0** | **417** | **417** | **417** | **0** |
| **Fan-in Gini** | 0.4097 | 0.3987 | 0.3987 | 0.3987 | 0.4097 |
| **Sprout Monopoly (Top 5%)** | 0.0000 | **0.3645** | **0.3645** | **0.3645** | 0.0000 |
| **Under-Recruited Activity (After)** | 10.04 | 9.82 | 10.18 | 10.51 | 10.04 |
| **Sprouted (L4 $\to$ L5)** | 0 | 151 | 53 | 89 | 0 |
| **Sprouted (L23 $\to$ L5)** | 0 | 266 | 364 | 328 | 0 |
| **Dale / Dense / Duplicate / Runaway** | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |

---

## 5. Key Diagnostic Insights

### 5.1 Deterministic Policies Exhibit Sprout Monopoly
- Policies 2, 3, and 4 successfully sprouted 417 synapses into under-recruited targets.
- However, all three deterministic policies showed a high **top 5% sprout monopoly of 36.45%**, meaning a small fraction of under-recruited targets absorbed a disproportionate share of new connections.
- Projection diversity (`under_recruited_plus_proj_div`) successfully shifted the composition toward L4 $\to$ L5 (89 sprouts vs 53 in spatial target sprouting), but did not eliminate monopoly because spatial candidate availability dominates.

### 5.2 Stochastic Geometry is Saturated by Topology Density
- When evaluated under strict invariant `pair_cap = 2`, `stochastic_geometry` sprouted **0 synapses**.
- Analysis of the candidate pool reveals that for almost every geometry-compatible source-target pair in the C17 benchmark network, 2 synapses already exist across existing active or dormant segments.
- Attempting to bypass `pair_cap` led to duplicate checker panics (160 violations), confirming that the restriction is an active structural constraint of the C17 topology rather than an algorithmic bug.

---

## 6. Research Conclusion & Next Steps

1. **v1.0 Status**: Closed as a diagnostic result. Deterministic sprouting works and respects safety gates, but stochastic sprouting requires topological headroom.
2. **Invariant Integrity**: Source-target pair cap = 2 and duplicate checking remain strictly non-negotiable.
3. **v1.1 Direction (Sprouting Headroom)**: The next iteration will focus on generating structural headroom (e.g. targeted pruning of inactive multi-synapses, spatial expansion of candidate search radii, or slot dynamic un-slotting) to enable stochastic sprouting under strict physical invariants.
