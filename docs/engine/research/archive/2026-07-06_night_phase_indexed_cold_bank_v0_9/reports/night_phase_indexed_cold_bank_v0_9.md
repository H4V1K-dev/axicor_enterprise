# Indexed Cold Bank Evidence (v0.9) Scientific Report

**Date**: 2026-07-06  
**Status**: PASS: Cold Bank reactivation indexing validated  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Research Question

Can we design a production-viable reactivation mechanism that avoids per-tick scanning of dormant synapses during the active Day phase, while maintaining high reactivation precision and recall compared to the v0.8 scanning oracle?

---

## 2. Indexed Reactivation Policies

Instead of checking dormant synapses every tick, the Day 3 runner compiles active summaries:
- **`dormant_indexed_any_day`**: Records `(source_soma_id, flat_segment_idx)` and spiked `target_soma_id`. Reactivates if both hit at least once during the day.
- **`dormant_indexed_bucketed`**: Records hits and spikes inside 8-tick buckets. Reactivates if source hit and target spike occur in the same bucket $b$ or $b+1$.
- **`dormant_indexed_bucketed_plus_trace`**: Requires the bucketed condition AND that the dormant synapse has a positive short or long trace (`short_trace > 0 || long_trace > 0`).

---

## 3. Results Comparison

### 3.1 Policy Metrics

| Metric / Policy | `hard_delete_trace_aware` | `dormant_scan_context_reactivation` | `dormant_indexed_any_day` | `dormant_indexed_bucketed` | `dormant_indexed_bucketed_plus_trace` |
|---|---|---|---|---|---|
| **Reactivated Synapses** | 0 | **444** | **864** | **845** | **539** |
| **Precision vs Oracle** | 0.0000 | 1.0000 | 0.5139 | 0.5254 | **0.8237** |
| **Recall vs Oracle** | 0.0000 | 1.0000 | 1.0000 | 1.0000 | **1.0000** |
| **Jaccard vs Oracle** | 0.0000 | 1.0000 | 0.5139 | 0.5254 | **0.8237** |
| **Total Ops (Day + Night)** | 0 | 8,640,000 | 1,125,238 | 1,217,628 | **1,217,628** |
| **Cost Reduction Factor** | 1.00x | 1.00x | 7.68x | 7.10x | **7.10x** |
| **Day 4 Full Cohort Retention**| -113.6663 | -62.2555 | -64.4449 | -63.5049 | **-61.9472** |
| **Dale / Dense / Duplicate Violations** | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |

### 3.2 Summary Set Sizes (Day 3)

- **`source_segment_hit_set`**: 5,027
- **`target_spike_set`**: 384 (all somas spiked)
- **`source_segment_buckets`**: 500,906
- **`target_spike_buckets`**: 38,325

---

## 4. Key Findings

1. **High false positives in raw indexed policies**:
   - `dormant_indexed_any_day` reactivated **864/864 (100%)** synapses.
   - `dormant_indexed_bucketed` reactivated **845/864 (97.8%)** synapses.
   - *Why*: In a highly active 10,000-tick simulation, almost every segment-target pair co-occurs in some 8-tick window by chance. Thus, time-bucketing alone cannot prevent random false-positive reactivations.
2. **Trace-gating solves the precision problem**:
   - `dormant_indexed_bucketed_plus_trace` requires the bucketed context match AND a positive trace (`short_trace > 0 || long_trace > 0`).
   - This policy achieves **0.8237 precision** and **1.0000 recall** (perfect coverage of the oracle set).
3. **Massive computational reduction**:
   - Total operations drop from **8,640,000** (oracle scan) to **1,217,628** (7.10x reduction).
   - More importantly, **dormant synapses are never iterated during the active day-phase tick loop**. This removes cold storage access from the active runtime hot path completely.
4. **Physical invariants preserved**:
   - Zero Dale, dense target, or duplicate violations on both Day 2 and Day 4 replays across all policies.

---

## 5. Visualizations

The generated graphics are located in the `images/` directory:
1. `reactivation_comparison.png`: Plots reactivation counts (bars) against precision/recall/Jaccard (lines) vs the scan oracle.
2. `cost_comparison.png`: Log-scale bar chart illustrating the 7x computational cost reduction.

---

## 6. Verdict

> [!IMPORTANT]
> **Verdict**: Indexed summaries with trace-gating approximate scan reactivation well enough for production design. By combining coarse 8-tick day summaries with a trace check during the night phase, the network achieves 82% precision and 100% recall of candidate reactivation, while completely isolating the Dormant Bank from the active simulation loop.
