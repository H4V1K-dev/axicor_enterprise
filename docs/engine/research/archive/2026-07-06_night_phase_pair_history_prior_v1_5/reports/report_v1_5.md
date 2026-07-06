# Night Phase Pair-History Prior Probe (v1.5) Scientific Report

**Date**: 2026-07-06  
**Status**: DIAGNOSTIC / NEGATIVE RESULT (Un-gated Pair-History Prior Cannot Differentiate Returned vs. Absent Context)  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Executive Summary

Night Phase Pair-History Prior Probe (v1.5) evaluated the hypothesis that slow soma-to-soma structural history (`pair_history: HashMap<(u32, u32), PairHistory>`) could serve as an initialization prior/bias (weight, traces, sprouting probability) to accelerate the recovery of previously established memory connections.

To rigorously test for false recovery (hallucination), v1.5 implemented a **Negative Control Experimental Protocol** comparing two branches across 5 policies over a 5-cycle stimulus schedule:
1. **`returned_branch`**: Cycles 1-2 (Context A+B), Cycles 3-4 (Context A only), **Cycle 5 (Context A+B returned)**.
2. **`absent_branch` (Negative Control)**: Cycles 1-2 (Context A+B), Cycles 3-4 (Context A only), **Cycle 5 (Context A only, Context B remains absent)**.

### Key Scientific Findings:

1. **Un-Gated Sprouting is Inherent Context-Blind**:
   - Even in `baseline_fresh_sprout` (without pair-history bias), sprouting produced identical rare-labeled cohort sizes on Cycle 5 (**4,242 in `returned_branch` vs 4,242 in `absent_branch`**).
   - This proves that target under-recruitment and geometric candidate selection during sprouting are inherently **context-blind** if operating without daytime co-activity gating.

2. **Pair-History Prior Fails to Solve Context-Blindness**:
   - `pair_history.mass` accumulates during Cycles 1-2 for pairs active under Context A+B.
   - When sprouting occurs on Cycle 5, `pair_history_init_weight`, `pair_history_init_trace`, and `pair_history_weight_plus_trace` bias sprouting probability toward historical pairs.
   - However, because `pair_history` is a static structural memory, it biases sprouting toward historical pairs **equally in both `returned_branch` (4,237) and `absent_branch` (4,237)**.
   - Pair-history prior alone fails to differentiate a context that has returned from one that remains absent.

3. **Safety Gate Failure Boundary in Overstrong Stress Policy**:
   - The 4 standard policies (`baseline_fresh_sprout`, `pair_history_init_weight`, `pair_history_init_trace`, `pair_history_weight_plus_trace`) maintained **100% clean safety gates across 40 cycle evaluations**.
   - `pair_history_overstrong_stress` (exaggerated gain `gain = 50`, $W_{\text{bias}} = \text{mass} \times 200 \ll 16$) demonstrated the structural failure boundary, producing **7,820 total Dale violations** (1,839 on C4 returned, 2,092 on C5 returned, 1,839 on C4 absent, 2,050 on C5 absent).

4. **Metric Definition Note**:
   - The metric `rare_sprouted_active_cohort` (4,242 at Cycle 5) represents the **cumulative active rare-labeled sprouted cohort** present in the network at Cycle 5, rather than the single-cycle sprouting delta (1,855 total sprouts in Cycle 5).

5. **Correct Takeaway & Roadmap**:
   - v1.5 proved that structural/pair-history prior without a current activity gate cannot be used as an independent recovery mechanism. It does not differentiate between returned vs absent context.
   - Future versions must evaluate pair-history ONLY in conjunction with **current source activity / co-gating / target under-recruitment**, rather than as an un-gated structural prior.

---

## 2. Experimental Setup & Evaluated Policies

### Evaluated Policies (5 Policies $\times$ 2 Branches = 10 Runs)
1. `baseline_fresh_sprout`: Sprouted/reactivated synapses receive default $W_{\text{init}}$ and default traces. `pair_history` is logged but does not bias growth.
2. `pair_history_init_weight`: Initial weight receives bias $W_{\text{init}} += f(\text{mass})$.
3. `pair_history_init_trace`: Initial `short_trace` and `long_trace` receive bias from $\text{mass}$.
4. `pair_history_weight_plus_trace`: Both initial weight and traces receive bias from $\text{mass}$.
5. `pair_history_overstrong_stress`: Exaggerated gain (`gain = 50`, $W_{\text{bias}} = \text{mass} \times 200 \ll 16$) to test failure boundaries.

---

## 3. Results & Metrics Summary

### 3.1 Cycle 5 Policy Comparison (Returned Branch vs. Absent Negative Control Branch)

| Policy | Branch | Rare Sprouted Active Cohort (C5) | Rare Reactivated (C5) | Total Rare Cohort (C5) | PH Mass P50 | PH Mass P90 | Dale Violations (C5) | Status |
|---|---|---|---|---|---|---|---|---|
| `baseline_fresh_sprout` | `returned_branch` | 4,242 | 0 | 4,242 | 35 | 52 | 0 | Diagnostic |
| `baseline_fresh_sprout` | `absent_branch` | 4,242 | 0 | 4,242 | 35 | 52 | 0 | Context-Blind Baseline |
| `pair_history_init_weight` | `returned_branch` | 4,237 | 0 | 4,237 | 35 | 52 | 0 | Diagnostic |
| `pair_history_init_weight` | `absent_branch` | 4,237 | 0 | 4,237 | 35 | 52 | 0 | Context-Blind Prior |
| `pair_history_init_trace` | `returned_branch` | 4,242 | 0 | 4,242 | 35 | 52 | 0 | Diagnostic |
| `pair_history_init_trace` | `absent_branch` | 4,242 | 0 | 4,242 | 35 | 52 | 0 | Context-Blind Prior |
| `pair_history_weight_plus_trace` | `returned_branch` | 4,237 | 0 | 4,237 | 35 | 52 | 0 | Diagnostic |
| `pair_history_weight_plus_trace` | `absent_branch` | 4,237 | 0 | 4,237 | 35 | 52 | 0 | Context-Blind Prior |
| `pair_history_overstrong_stress` | `returned_branch` | 3,937 | 0 | 3,937 | 175 | 219 | 2,092 | Boundary Failure |
| `pair_history_overstrong_stress` | `absent_branch` | 3,972 | 0 | 3,972 | 175 | 219 | 2,050 | Boundary Failure |

### 3.2 Invariant Safety Gates Summary
- **Standard Policies (40 Evaluations)**: `0` Dale, `0` dense target, `0` duplicate (`count > 2`), `0` runaway ticks across all 4 standard policies.
- **Overstrong Stress Policy (10 Evaluations)**: Triggered **7,820 total Dale violations** across Cycles 4-5 due to exaggerated weight bias overwhelming inhibitory polarity.

---

## 4. Analytical Conclusions

1. **Is pair-history prior alone sufficient for memory recovery?**
   - No. Un-gated sprouting is inherently context-blind, and pair-history prior alone fails to differentiate whether a context has returned or remains absent.

2. **Architectural Takeaway**:
   - Structural history (`pair_history`) must **never** be used as an independent, un-gated prior for memory recovery.
   - Future research must test pair-history ONLY in combination with **current daytime activity / co-gating / target under-recruitment**.
