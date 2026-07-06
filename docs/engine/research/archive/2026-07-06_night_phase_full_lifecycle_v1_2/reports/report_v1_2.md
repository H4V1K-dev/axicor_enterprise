# Night Phase Full Lifecycle Research (v1.2) Scientific Report

**Date**: 2026-07-06  
**Status**: PARTIAL / DIAGNOSTIC RESULT (Sprouting Churn Stable, Dormant Reactivation Not Yet Validated)  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase Full Lifecycle (v1.2) assembled the complete multi-stage night-phase pipeline into a unified research harness across 5 day/night cycles under a varied stimulus schedule (Context A+B in Cycles 1-2, Context A only in Cycles 3-4, Context A+B returned in Cycle 5) with explicit cohort tracking, real dormant bank eviction, and reactivation blocker breakdown.

Key diagnostic findings:
1. **Sprouting Churn is Stable (`full_lifecycle` & `sprouting_only`)**:
   - `full_lifecycle` starts from the full active topology and then settles into a post-prune/sprout regime: active synapses follow **22,037 -> 16,793 -> 3,640 -> 4,574 -> 5,560** across cycles.
   - Stochastic geometry sprouting adds new connections into available headroom without pair-cap or duplicate violations: **1,639**, **1,937**, **1,856**, and **1,912** sprouts on Cycles 2-5.
   - Fan-in Gini coefficient stays bounded between **0.3917** and **0.4579**.
2. **Dormant Bank Eviction Functions**:
   - Age-based eviction (`dormant_age > 3`) successfully prevents monotonic growth of the Dormant Bank. In Cycle 5, 6,883 expired dormant synapses were evicted to `dead_count`, reducing Dormant Bank size from 22,895 to 16,938.
3. **Network Collapse Without Sprouting (`dormant_reactivation_only`)**:
   - Without sprouting, `dormant_reactivation_only` suffers complete network collapse by Cycle 4 (`active_count = 0`), proving that dormant reactivation alone cannot sustain the active network.
4. **Dormant Reactivation of Rare Memory is NOT Yet Validated**:
   - When Context B returns in Cycle 5, only **12 dormant synapses** reactivated in `full_lifecycle`. The primary blocker was `react_trace_failed` (14,320 candidates), because long traces decayed during the 2-cycle absence of Context B.
   - Consequently, **v1.2 is categorized as PARTIAL / DIAGNOSTIC RESULT**. Dormant memory recovery requires side-channel trace preservation or lowered reactivation threshold to prevent decay during absent cycles.

---

## 2. Experimental Setup & Cohort Tracking

### Topology & Stimulus Protocol
- **Topology**: Baker C17 (`post_prune_headroom`) initialized with 22,037 synapses.
- **Cycles 1–2**: Context A + B active (learning phase).
- **Cycles 3–4**: Context A active only (Context B absent; testing STDP decay and trace demotion).
- **Cycle 5**: Context A + B returned (testing reactivation).

### Explicit Cohort Tracking
Each synapse carries metadata:
- `initial_triple`: `(source_soma_id, flat_segment_idx, target_soma_id)`
- `origin_kind`: `Initial`, `Reactivated`, `Sprouted`
- `context_label`: `ContextA`, `ContextB`, `General` (used exclusively for research metrics).

---

## 3. Results & Metrics Summary

### 3.1 Full Lifecycle Policy Performance

| Cycle | Stimulus | Active | Dormant | Dead | Pruned | Reactivated | Sprouted | Rare Active Cohort (Context B) | Rare Dormant | Rare Dead | React Trace Fail | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Cycle 1** | A+B | 22,037 | 0 | 0 | 0 | 0 | 0 | 17,082 | 0 | 0 | 0 | `STABLE` |
| **Cycle 2** | A+B | 16,793 | 6,883 | 0 | 7,337 | 454 | 1,639 | 11,501 | 5,301 | 0 | 6,844 | `STABLE` |
| **Cycle 3** | A only | 3,640 | 21,973 | 0 | 15,090 | 0 | 1,937 | 37 | 17,045 | 0 | 20,771 | `STABLE` |
| **Cycle 4** | A only | 4,574 | 22,895 | 0 | 922 | 0 | 1,856 | 0 | 17,082 | 0 | 21,077 | `STABLE` |
| **Cycle 5** | A+B | 5,560 | 16,938 | 6,883 | 938 | **12** | 1,912 | 0 | 11,773 | 5,301 | **14,320** | `PARTIAL` |

### 3.2 Policy Comparison Across 5 Cycles

| Policy | Cycle 5 Active | Cycle 5 Dormant | Cycle 5 Dead | Cycle 5 Reactivated | Status |
|---|---|---|---|---|---|
| `passive_night_baseline` | 22,037 | 0 | 0 | 0 | Stable (No pruning/sprouting) |
| `dormant_reactivation_only` | **0** | 15,154 | 6,883 | 0 | **COLLAPSED (Active Count = 0)** |
| `sprouting_only` | 5,614 | 0 | 0 | 0 | Stable Sprouting (No Dormant Bank) |
| `full_lifecycle` | 5,560 | 16,938 | 6,883 | 12 | **PARTIAL DIAGNOSTIC (Sprouting Stable, Reactivation Low)** |

---

## 4. Activity Health & Blocker Attribution

1. **Driven Ticks vs. Silence Ticks**:
   - `driven_tick_count` = 40 ticks per 2,000-tick day (stimulus injected every 50 ticks).
   - High `silence_ticks` (~1,940 / 2,000) is expected due to the sparse stimulus schedule by design.
2. **Reactivation Blocker Breakdown**:
   - In Cycle 5, out of 16,938 dormant candidates, **14,320 failed due to `react_trace_failed`**.
   - During the 2-cycle absence of Context B (Cycles 3-4), long traces decayed below the reactivation threshold (`long_trace < 20`).
3. **Eviction Mechanism**:
   - Age-based eviction (`dormant_age > 3`) successfully evicted 6,883 expired dormant synapses in Cycle 5, halting unbounded dormant bank growth.

---

## 5. Conclusion & Architectural Status

1. **Overall Status**: **PARTIAL / DIAGNOSTIC RESULT**.
2. **Validated Mechanisms**:
   - Homeostatic stochastic sprouting into freed headroom is **stable and robust**.
   - Dormant bank age eviction prevents memory leak / unbounded bank growth.
   - Physical invariants (`pair_cap = 2`, duplicate check `count > 2`, Dale=0) remain 100% intact.
3. **Open Research Challenge for v1.3**:
   - Dormant reactivation of rare context memories after extended absence requires trace decay protection (e.g. side-channel reactivation tagging or trace floor preservation) to prevent trace expiration before context return.
