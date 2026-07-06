# Night Phase Dormant Trace Preservation (v1.3) Scientific Report

**Date**: 2026-07-06  
**Status**: DIAGNOSTIC / RESEARCH RESULT (Mechanism Discovered: Disuse Pruning & Segment-Trace Indexing Gap Identified)  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase Dormant Trace Preservation (v1.3) systematically evaluated 5 trace preservation policies (`baseline_v1_2`, `dormant_trace_floor`, `dormant_slow_decay`, `dormant_age_hysteresis`, `combined_preservation`) across a 5-cycle stimulus schedule (Cycles 1-2: Context A+B, Cycles 3-4: Context A only, Cycle 5: Context A+B returned) to resolve the `react_trace_failed = 14,320` blocker identified in v1.2.

### Key Scientific Findings:
1. **The Disuse Pruning Trace Paradox**:
   - Synapses do not enter the Dormant Bank immediately when Context B becomes absent. Instead, STDP depresses their weight over 1–2 cycles of non-coactivity.
   - By the time a synapse's weight drops below the prune threshold (`abs(weight) < 500 << 16`), Context B has been silent for ~1,500 – 3,000 ticks. Consequently, `long_trace` has already decayed to `0` **before** the synapse enters the Dormant Bank.
   - As a result, passive trace preservation rules (trace floor, slow decay, age hysteresis) operate on `initial_dormant_trace = 0` and cannot preserve a non-existent trace across absent cycles.

2. **The Dormant Segment-Level Indexing Gap**:
   - In v1.2/v1.3, `indexed_evidence` checked segment-level hits (`source_segment_buckets.contains(&(ds.source_soma_id, ds.flat_segment_idx, b))`).
   - Because dormant synapses are unhooked from the active signal pipeline during the day, dormant axon segments register `0` segment hits even when the source soma fires.
   - When Context B returns on Cycle 5, source somas fire vigorously, but dormant links are blocked by `trace/context evidence failed` (19,487 candidate rejections) because segment-level active hits were required.

3. **Production Design Recommendation (Path to v1.4)**:
   - To convert the Dormant Bank into functional associative memory, dormant reactivation must match **Source Soma Spiking Buckets** (`(ds.source_soma_id, bucket)`) with **Target Soma Spiking Buckets** (`(ds.target_soma_id, bucket)`), rather than requiring segment-level active traces from dormant links.

---

## 2. Experimental Setup & Trace Policies

### Evaluated Policies (5 Policies)
1. `baseline_v1_2`: Standard decay `long_trace -= long_trace >> 7`, eviction at `dormant_age > 3`.
2. `dormant_trace_floor`: Preserves a trace floor (`TRACE_FLOOR = 15`) if `initial_dormant_trace >= 20`. Eviction at `dormant_age > 3`.
3. `dormant_slow_decay`: Slower decay rate (`long_trace -= long_trace >> 10`). Eviction at `dormant_age > 3`.
4. `dormant_age_hysteresis`: Variable grace period (`max_age = 5` if `long_trace >= 20`, else `3`).
5. `combined_preservation`: Slower decay (`>> 9`) + trace floor (`TRACE_FLOOR = 12`) + age hysteresis (`max_age = 5` if `long_trace >= 15`, else `3`).

---

## 3. Results & Metrics Summary

### 3.1 Cycle 5 Policy Comparison

| Policy | Active Count | Dormant Count | Dead Count | Reactivated | Sprouted | Rare Context B Reactivated | Trace / Context Evidence Failed | Status |
|---|---|---|---|---|---|---|---|---|
| `baseline_v1_2` | 5,478 | 22,028 | 0 | 9 | 1,855 | **2** | 19,487 | Diagnostic |
| `dormant_trace_floor` | 5,478 | 22,028 | 0 | 9 | 1,855 | **2** | 19,487 | Diagnostic |
| `dormant_slow_decay` | 5,478 | 22,028 | 0 | 9 | 1,855 | **2** | 19,487 | Diagnostic |
| `dormant_age_hysteresis` | 5,478 | 22,028 | 0 | 9 | 1,855 | **2** | 19,487 | Diagnostic |
| `combined_preservation` | 5,478 | 22,028 | 0 | 9 | 1,855 | **2** | 19,487 | Diagnostic |

### 3.2 Invariant Safety Gates
Across all 5 policies and all 25 cycle evaluations (5 policies × 5 cycles):
- **Dale Violations**: `0`
- **Dense Violations**: `0`
- **Duplicate Violations (`count > 2`)**: `0`
- **Runaway Ticks**: `0`

---

## 4. Analytical Findings & Answers to Prompt Questions

1. **Can Dormant preserve rare memory across 2 absent cycles via trace preservation alone?**
   - No. Trace preservation rules operating solely on post-prune `long_trace` cannot preserve memory because disuse pruning occurs *after* trace decay has already occurred during absent cycles.

2. **Which trace preservation rule gives best recovery without memory leak?**
   - Under segment-level trace indexing, all passive trace rules yield identical performance (`trace/context evidence failed = 19,487`). An eviction policy (`dormant_age > max_age`) is implemented in code, but in this 5-cycle protocol dormant synapses did not reach expiration age because pruning occurred on Cycles 3–4 (age = 1–2 on Cycle 5, `dead_count = 0`).

3. **Does reactivation beat pure sprouting replacement?**
   - Currently, homeostatic sprouting replaces pruned capacity (~1,855 new sprouts per cycle). For reactivation to outperform sprouting, dormant reactivation must use **Source Soma Co-Spiking Evidence** rather than segment-level trace activity.

---

## 5. Conclusion & Next Steps (v1.4 Roadmap)

v1.3 successfully isolated the precise mathematical and structural cause of `trace/context evidence failed`. In v1.4 (Night Phase Soma-Indexed Reactivation), indexed evidence will index source soma spiking events directly (`(source_soma_id, bucket)`), enabling dormant synapses to reactivate whenever their original source and target somas co-fire upon context return.
