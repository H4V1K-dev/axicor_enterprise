# Night Phase Soma-Indexed Dormant Reactivation (v1.4) Scientific Report

**Date**: 2026-07-06  
**Status**: DIAGNOSTIC / NEGATIVE RESULT (Soma Co-Spiking Evidence Alone Insufficient to Restore Dormant Memory)  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase Soma-Indexed Dormant Reactivation (v1.4) tested the hypothesis formulated in v1.3: that dormant reactivation was failing because evidence was indexed by `flat_segment_idx` (which is inactive for dormant synapses during the day).

To test this hypothesis, v1.4 introduced **Soma-Level Bucket Co-Spiking Evidence** (`soma_spike_buckets: HashSet<(u32, usize)>`), recording `(soma_id, tick / 8)` on every soma spike during the day, and evaluated 4 reactivation policies across a 5-cycle stimulus schedule (Cycles 1-2: Context A+B, Cycles 3-4: Context A only, Cycle 5: Context A+B returned).

### Key Scientific Findings:
1. **Soma Co-Spiking Yields Marginal Increase, Not Full Recovery**:
   - Switching from segment-level indexing (`segment_index_baseline_v1_3`) to soma-level co-firing (`soma_bucket_cofire`) increased total reactivated synapses on Cycle 5 from **9 to 17** (+88%) and rare Context B cohort reactivated synapses from **2 to 6** (+200%).
   - However, **19,475 out of 22,037 reactivation candidates (88.4%) still failed evidence (`react_evidence_failed`)**.

2. **The Causal Mechanism of Evidence Failure**:
   - When Context B returns on Cycle 5, stimulus drives source soma $S$, causing $S$ to fire.
   - However, because the synapses connecting $S$ to target soma $T$ are dormant, action potentials from $S$ cannot propagate to $T$.
   - Unless target soma $T$ happens to be independently driven by another stimulus in the exact same 8-tick bucket ($b$ or $b+1$), target soma $T$ does **not** spike.
   - Consequently, $S$ and $T$ do not co-fire in the same bucket, and `soma_spike_buckets` fails for 88.4% of dormant candidates.

3. **Classification**:
   - Per the diagnostic gate criteria, v1.4 is classified as **DIAGNOSTIC / NEGATIVE RESULT**. Soma-level co-firing alone cannot restore dormant memory without active signal propagation or structural history memory.

4. **Roadmap to v1.5 (Pair-History / Structural Mass Memory)**:
   - Because dormant links cannot demonstrate co-spiking while silent, dormant memory preservation cannot depend on daytime co-spiking evidence alone. v1.5 must evaluate **Structural Mass Memory / Pair-History**, preserving structural credit for previously established connections.

---

## 2. Experimental Setup & Reactivation Policies

### Evaluated Policies (4 Policies)
1. `segment_index_baseline_v1_3`: Segment-level evidence indexing `(source_soma_id, flat_segment_idx, bucket)` from v1.3.
2. `soma_bucket_cofire`: Pure source/target soma co-spiking evidence `(soma_spike_buckets.contains(&(source_id, b)) && target_id in b or b+1)`.
3. `soma_bucket_plus_trace`: Soma co-spiking OR `long_trace >= 20`.
4. `soma_bucket_plus_trace_plus_slot_pressure`: Soma co-spiking + trace, with strict slot limit (`target_count < 64` instead of `96`) to test slot contention.

---

## 3. Results & Metrics Summary

### 3.1 Cycle 5 Policy Comparison

| Policy | Active Count | Dormant Count | Dead Count | Total Reactivated | Sprouted | Rare Context B Reactivated | Evidence Failed | Slot Failed | Status |
|---|---|---|---|---|---|---|---|---|---|
| `segment_index_baseline_v1_3` | 5,478 | 22,028 | 0 | 9 | 1,855 | 2 | 19,487 | 2,540 | Diagnostic |
| `soma_bucket_cofire` | 5,486 | 22,020 | 0 | 17 | 1,855 | 6 | 19,475 | 2,541 | Diagnostic |
| `soma_bucket_plus_trace` | 5,486 | 22,020 | 0 | 17 | 1,855 | 6 | 19,475 | 2,541 | Diagnostic |
| `soma_bucket_plus_trace_plus_slot_pressure` | 5,486 | 22,020 | 0 | 17 | 1,855 | 6 | 19,475 | 2,541 | Diagnostic |

> [!NOTE]
> Enforcing stricter target slot pressure (`target_count < 64` in `soma_bucket_plus_trace_plus_slot_pressure`) yielded identical results to `soma_bucket_plus_trace`, confirming that evidence failure (`19,475`) remains the primary bottleneck long before target slot capacity limits are reached.

### 3.2 Invariant Safety Gates
Across all 4 policies and all 20 cycle evaluations:
- **Dale Violations**: `0`
- **Dense Violations**: `0`
- **Duplicate Violations (`count > 2`)**: `0`
- **Runaway Ticks**: `0`

---

## 4. Analytical Findings & Prompt Questions Answered

1. **Does soma-level co-spiking evidence resolve dormant reactivation failure?**
   - No. While total reactivated synapses increased modestly from 9 to 17 and rare cohort reactivation increased from 2 to 6, 88.4% of reactivation candidates (19,475 out of 22,037) still failed evidence.

2. **Why does soma-level co-firing fail for dormant links?**
   - Because dormant links do not conduct action potentials during the day, source soma firing does not cause target soma firing. Co-spiking only occurs if source and target are independently stimulated in the same 8-tick bucket.

3. **What is the structural takeaway for AxiEngine?**
   - Passive trace preservation (v1.3) and active co-spiking evidence (v1.4) are both insufficient to recover dormant memories after long absence. Dormant memory reactivation requires **Pair-History / Structural Mass Memory (v1.5)**, where initial structural memory weights persist directly in the Dormant Bank.
