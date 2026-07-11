# Dormant/Cold Storage Bank Stress Test (v0.7) Scientific Report

**Date**: 2026-07-06  
**Status**: PARTIAL PASS: Cold Bank isolation/storage passed, reactivation not validated  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Research Question

Pruning synapses permanently (hard delete) can lead to the loss of useful pathways that are temporarily inactive or depressed. We evaluate if storing pruned synapses in a **Dormant Bank (Cold Storage)**:
1. Safely preserves structural option value (traces and weights) under equal budget-matched pruning pressure.
2. Allows functional recovery and reactivation when a previously learned context returns.
3. Protects matched pathways from permanent deletion.

---

## 2. Multi-Day Experiment Protocol

We construct a multi-day schedule:
- **Day 1**: Run structured stimulation training with learning enabled.
- **Night 1**: reset voltages, merge traces, and execute pruning. Demoted synapses under dormant policies are moved to the Dormant Bank. Active synapses are compacted.
- **Day 2**: Run replay without learning to assess initial memory retention. Assertions are placed here to ensure Dale, dense, and duplicate invariants are intact.
- **Day 3**: Re-expose the network to the original structured stimulation context with learning enabled to accumulate pre/post spike evidence. A snapshot of spikes is captured at the end of the day.
- **Night 2**: reset voltages, merge active traces. Increment dormant synapse age and decay their traces. Execute reactivation pass using Day 3 activity evidence.
- **Day 4**: Run replay without learning to verify final recovery.

---

## 3. Policy Comparison Metrics

Pruning budget limit is set to exactly **864** synapses.

| Metric / Policy | `hard_delete_absolute_floor` | `hard_delete_trace_aware` | `dormant_trace_aware` | `dormant_trace_aware_with_return` |
|---|---|---|---|---|
| **Day 2 Active / Dormant / Deleted** | 21,173 / 0 / 864 | 21,173 / 0 / 864 | 21,173 / 864 / 0 | 21,173 / 864 / 0 |
| **Day 4 Active / Dormant / Deleted** | 21,173 / 0 / 864 | 21,173 / 0 / 864 | 21,173 / 864 / 0 | 21,173 / 864 / 0 |
| **Reactivated Synapses (Night 2)** | 0 | 0 | 0 | 0 |
| **Day 2 Full Cohort Retention Ratio**| -186.1928 | -114.9637 | -114.9637 | -114.9637 |
| **Day 4 Full Cohort Retention Ratio**| -184.7963 | -113.6663 | -113.6663 | -113.6663 |
| **Day 2 Survivor Retention Ratio**| 3.1884 | 2.1592 | 2.1592 | 2.1592 |
| **Day 4 Survivor Retention Ratio**| 6.3003 | 4.3128 | 4.3128 | 4.3128 |
| **Day 2 / Day 4 Silence Ticks** | 2,150 / 2,121 | 2,115 / 2,110 | 2,115 / 2,110 | 2,115 / 2,110 |
| **Dormant Protected Matched Count** | 0 | 0 | 577 | 577 |
| **Dormant Protected High-Trace** | 0 | 0 | 0 | 0 |
| **Dale / Dense / Duplicate Violations** | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |

---

## 4. Reactivation Blocker Breakdown

To identify the precise bottleneck preventing functional recovery, we analyzed the criteria evaluation for all 864 synapses in the Dormant Bank under the `dormant_trace_aware_with_return` policy:

- **`trace_ok`** (`long_trace >= 20`): **0 / 864**
- **`slot_ok`** (target active input count < 96): **864 / 864**
- **`diversity_ok`** (projection input count < initial count): **864 / 864**
- **`activity_ok`** (source and target soma spiked in Day 3): **864 / 864**
- **`all_ok / reactivated`**: **0 / 864**

### Blocker Analysis
1. **Activity evidence correctly validated**: The fix to snapshot Day 3 activity before Night 2 reset confirms that **100% of the dormant synapses (864/864)** had active source and target neurons during the Day 3 learning block.
2. **Trace bottleneck**: The trace-aware pruning score targets synapses with low weights and low trace values for demotion. Thus, 100% of the demoted cohort starts with `long_trace == 0`. After Day 3 trace decay, they remain 0, making them unable to pass the threshold of 20.

---

## 5. Analysis and Findings

- **Option Value Preservation**:
  - The Dormant Bank successfully preserved **577 matched synapses** in inactive storage without any functional transmission leakage during replay, matching the active metrics of `hard_delete_trace_aware` exactly.
- **Dual Day 2 / Day 4 Assertions**:
  - Hard gate checks (Dale, duplicate, dense target) passed with **0 violations** on both Day 2 and Day 4 across all policies.
- **Limitation**:
  - "Dormant preserves traces but reactivation rule is not yet sufficient" or "Dormant adds storage without functional recovery".

---

## 6. Verdict

> [!IMPORTANT]
> **Verdict: PARTIAL PASS**
> Cold Bank isolation/storage has passed (synapses are isolated, active slots are cleared, and option value is preserved), but reactivation was not functionally validated because trace-aware pruning demotes the low-trace tail, preventing any synapses from meeting the reactivation threshold.
