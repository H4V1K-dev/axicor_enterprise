# All-to-All STDP & Refractory Timer Penalty Research Specification

## 1. Context & Motivation

During biocalibration research on MVP CPU replay, single-sided spatial cooling was replaced with a symmetric, continuous **Bidirectional STDP Gradient**. To avoid missing cumulative multi-spike temporal dynamics, the learning engine was further upgraded from Nearest-Neighbor search to **All-to-All STDP** summation.

Furthermore, a legacy limitation where the dendritic refractory timer blocked all synaptic learning was resolved. Now, learning remains open for all dendrites, but "tired" synapses (with `timer > 0`) receive a supplementary **Linear Refractory Penalty** proportional to the timer state and refractory period.

---

## 2. Mathematical Formulation

### 2.1 Head Partitioning & Distance Calculations

For each active head in the 8-head ring buffer (excluding `AXON_SENTINEL`):
- `diff = head.wrapping_sub(seg_idx)`
- **Causal Distance** ($d_{\text{ltp}}$): If `diff < 0x8000_0000`, the spike has already passed the segment ($d_{\text{ltp}} = \text{diff}$).
- **Anti-Causal Distance** ($d_{\text{ltd}}$): If `diff >= 0x8000_0000`, the spike is approaching the segment ($d_{\text{ltd}} = \text{seg\_idx.wrapping\_sub}(head)$).

---

### 2.2 All-to-All STDP Summation

Instead of finding the closest spike, the total synaptic plasticity delta is calculated by summing the contributions of all active axon heads within the propagation length window ($L_{\text{prop}}$):

$$\Delta_{\text{ltp}}^{\text{total}} = \sum_{h \in \text{heads}, d_{\text{ltp}}(h) \le L_{\text{prop}}} \left\lfloor \frac{\text{final\_pot} \cdot \text{inertia} \cdot \text{burst\_mult} \cdot (L_{\text{prop}} - d_{\text{ltp}}(h))}{128 \cdot L_{\text{prop}}} \right\rfloor$$

$$\Delta_{\text{ltd}}^{\text{total}} = \sum_{h \in \text{heads}, d_{\text{ltd}}(h) \le L_{\text{prop}}} \left\lfloor \frac{\text{final\_dep} \cdot \text{inertia} \cdot \text{burst\_mult} \cdot (L_{\text{prop}} - d_{\text{ltd}}(h))}{128 \cdot L_{\text{prop}}} \right\rfloor$$

---

### 2.3 Refractory Timer Penalty

If the dendritic timer is active (`timer > 0`) and the refractory period is non-zero (`refractory_period > 0`), an additional linear depression penalty is subtracted:

$$\text{base\_ltd} = \left\lfloor \frac{\text{final\_dep} \cdot \text{inertia} \cdot \text{burst\_mult}}{128} \right\rfloor$$

$$\Delta_{\text{penalty}} = \left\lfloor \frac{\text{timer} \cdot \text{base\_ltd}}{\text{refractory\_period}} \right\rfloor$$

---

### 2.4 Final Delta

$$\Delta = \Delta_{\text{ltp}}^{\text{total}} - \Delta_{\text{ltd}}^{\text{total}} - \Delta_{\text{penalty}}$$

The weight is updated by clamping the raw value:
$$\text{weight}_{\text{new}} = \text{clamp}(\text{weight}_{\text{old}} + \Delta, \text{MIN\_WEIGHT\_LIMIT}, \text{MAX\_WEIGHT\_LIMIT})$$

---

## 3. Code & Test Reference

- **Research Module**: `crates/test-harness/src/mvp_cpu_replay.rs` (`research_apply_gsop_plasticity`)
- **Sandbox Integration**: `cpu_apply_gsop` in `crates/test-harness/src/mvp_cpu_replay.rs`
- **Unit Tests**:
  - `test_research_apply_gsop_plasticity_bidirectional_stdp` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_all_to_all_stdp_summation` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_refractory_timer_penalty_linear` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_bidirectional_stdp_six_scenarios` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_cpu_apply_gsop_timer_skip` (verifying penalty application) in `crates/test-harness/tests/mvp_cpu_replay.rs`

