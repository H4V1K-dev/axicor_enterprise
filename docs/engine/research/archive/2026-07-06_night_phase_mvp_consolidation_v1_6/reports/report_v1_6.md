# Night Phase MVP Consolidation (v1.6) Scientific Report

**Date**: 2026-07-06  
**Status**: SMOKE PASS / PARTIAL MVP VALIDATION  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase MVP Consolidation (v1.6) ran a smoke validation of the day/night lifecycle. While the active structures and safety invariants remained clean, the test only partially validated the system because pruning was small (44 synapses) and occurred only at Cycle 10. Consequently, the Dormant Bank was never heavily loaded, and eviction mechanics did not trigger.

### Verified in v1.6 (Partial Validation):
1. **Active Structure is Stable**: No immediate structural collapse or explosion occurred (synapse count went from 20,467 to 20,443).
2. **Safety Invariants are Clean**: Asserted 0 Dale, dense, duplicate, or runaway violations.
3. **Pruning/Sprouting Path Activated**: The pruning and sprouting mechanisms successfully executed at Cycle 10.

### NOT Verified in v1.6 (Requires Stress Test):
1. **Dormant Eviction Mechanics**: Since dormant synapse count (44) remained below thresholds, the per-target and global eviction logic was never tested.
2. **Dormant Bounding Under Load**: Memory footprint bounding and eviction leakage prevention under real load remain unproven.
3. **Long-Run Lifecycle Stability**: Behavior over multiple cycles of high pruning/sprouting is unproven.
4. **Anti-Monopoly & Sprouting Diversity**: The `top_5pct_fan_in_share` Gini/monopoly metrics are unreliable because the sprout count (20) was too small.

---

## 2. Experimental Setup

The experiment ran a 10-cycle lifecycle on a C17-like shard topology (384 somas).
- **Stimulus Schedule**:
  - Cycles 1-2: Mixed structured drive (Context A and Context B active).
  - Cycles 3-6: Sparse/reduced drive (only Context A active).
  - Cycles 7-10: Mixed structured drive returns.
- **Knobs & Thresholds**:
  - `PRUNE_WEIGHT_THRESHOLD` = $500 \ll 16$
  - `PRUNE_COACTIVITY_THRESHOLD` = 2 hits
  - `MIN_TARGET_ACTIVE_COUNT` = 5
  - `MIN_PROJECTION_ACTIVE_COUNT` = 2
  - `MAX_DORMANT_AGE` = 5 cycles
  - `MAX_DORMANT_TOTAL` = 500
  - `MAX_DORMANT_PER_TARGET` = 10
  - `MAX_SPROUTS_PER_TARGET` = 8

---

## 3. Results & Metrics Summary

### 3.1 Cycle-by-Cycle Metrics

| Cycle | Active Count | Dormant Count | Dead Count | Pruned Count | Sprouted Count | Silence Ticks | Runaway Ticks | Proj Coverage | Under-Recruited (Before) | Under-Recruited (After) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 20,467 | 0 | 0 | 0 | 0 | 1937 | 0 | 70% | 384 | 384 |
| 2 | 20,467 | 0 | 0 | 0 | 0 | 1940 | 0 | 70% | 384 | 384 |
| 3 | 20,467 | 0 | 0 | 0 | 0 | 1960 | 0 | 70% | 384 | 384 |
| 4 | 20,467 | 0 | 0 | 0 | 0 | 1960 | 0 | 70% | 384 | 384 |
| 5 | 20,467 | 0 | 0 | 0 | 0 | 1960 | 0 | 70% | 384 | 384 |
| 6 | 20,467 | 0 | 0 | 0 | 0 | 1960 | 0 | 70% | 384 | 384 |
| 7 | 20,467 | 0 | 0 | 0 | 0 | 1941 | 0 | 70% | 384 | 384 |
| 8 | 20,467 | 0 | 0 | 0 | 0 | 1941 | 0 | 70% | 384 | 384 |
| 9 | 20,467 | 0 | 0 | 0 | 0 | 1942 | 0 | 70% | 384 | 384 |
| 10 | 20,443 | 44 | 0 | 44 | 20 | 1942 | 0 | 70% | 384 | 373 |

### 3.2 Invariant Safety Gates
- **Dale Violations**: 0 (Excitatory/Inhibitory polarity strictly respected)
- **Dense/Fan-in Violations**: 0 (Synapse dendrite indices are contiguous and dense)
- **Duplicate Violations**: 0 (No duplicate synapses in active list)
- **Invalid Geometry**: 0 (All synapses respect axon/dendrite whitelists)
- **Runaway Ticks**: 0 (Activity never exploded beyond 50% somas spiking)

---

## 4. Design Validation Answers

### 1. Does MVP night policy maintain network structure over 8-10 cycles?
Yes, under low activity conditions, the structure remains stable. However, long-run stability under continuous structural turnover is **not** yet proven.

### 2. Is Dormant Bank bounded or leaking?
Unproven in v1.6. The bank size (44) remained below capacity bounds (`max_dormant_total = 500`), meaning the eviction code was never executed. Bounded eviction must be verified under high pruning pressure.

### 3. Does sprouting refill useful headroom without breaking Dale/fan-in/duplicates?
Partial validation. Sprouting refilled 20 synapses without safety violations, but the code needs validation under high sprouting rates.

### 4. Does active synapse count stabilize instead of collapsing or exploding?
Yes, in the short term. However, long-term stabilization under high load/turnover has not been tested.

### 5. What minimal knobs need to be carried toward production design?
1. `PRUNE_WEIGHT_THRESHOLD`
2. `MIN_TARGET_ACTIVE_COUNT` & `MIN_PROJECTION_ACTIVE_COUNT`
3. `MAX_DORMANT_AGE`, `MAX_DORMANT_TOTAL` & `MAX_DORMANT_PER_TARGET` (Eviction bounds)
4. `MAX_SPROUTS_PER_TARGET` & `under-recruitment` targeting.
