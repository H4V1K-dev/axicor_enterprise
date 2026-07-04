# Linear Spatial Cooling & Bidirectional STDP Research Specification

## 1. Context & Motivation

During biocalibration research on MVP CPU replay, single-sided spatial cooling with fixed unconditioned LTD penalty was replaced with a symmetric, continuous **Bidirectional STDP Gradient**.

In this model, passed spikes produce distance-attenuated Causal LTP (Long-Term Potentiation), while approaching spikes produce distance-attenuated Anti-Causal LTD (Long-Term Depression). Axons without active spikes in range experience zero weight modification ($\Delta = 0$).

---

## 2. Mathematical Formulation

### 2.1 Head Partitioning & Distance Calculations

For each active head in the 8-head ring buffer (excluding `AXON_SENTINEL`):
- `diff = head.wrapping_sub(seg_idx)`
- **Causal Distance** ($d_{\text{ltp}}$): If `diff < 0x8000_0000`, the spike has already passed the segment ($d_{\text{ltp}} = \text{diff}$).
- **Anti-Causal Distance** ($d_{\text{ltd}}$): If `diff >= 0x8000_0000`, the spike is approaching the segment ($d_{\text{ltd}} = \text{seg\_idx.wrapping\_sub}(head)$).

---

### 2.2 Superposition Gradient Model

$$\Delta = \Delta_{\text{ltp}} - \Delta_{\text{ltd}}$$

$$\Delta_{\text{ltp}} = \begin{cases} \left\lfloor \frac{\text{final\_pot} \cdot \text{inertia} \cdot \text{burst\_mult} \cdot (L_{\text{prop}} - d_{\text{ltp}})}{128 \cdot L_{\text{prop}}} \right\rfloor & \text{if } d_{\text{ltp}} \le L_{\text{prop}} \land L_{\text{prop}} > 0 \\ 0 & \text{otherwise} \end{cases}$$

$$\Delta_{\text{ltd}} = \begin{cases} \left\lfloor \frac{\text{final\_dep} \cdot \text{inertia} \cdot \text{burst\_mult} \cdot (L_{\text{prop}} - d_{\text{ltd}})}{128 \cdot L_{\text{prop}}} \right\rfloor & \text{if } d_{\text{ltd}} \le L_{\text{prop}} \land L_{\text{prop}} > 0 \\ 0 & \text{otherwise} \end{cases}$$

If both $d_{\text{ltp}} > L_{\text{prop}}$ and $d_{\text{ltd}} > L_{\text{prop}}$ (Complete Miss), $\Delta = 0$.

---

## 3. Boundary Conditions & Phase Table

| Signal State | Distance Condition | Effective Delta $\Delta$ | Plasticity Effect |
| :--- | :--- | :--- | :--- |
| **Causal Peak** | $d_{\text{ltp}} = 0, d_{\text{ltd}} = \infty$ | $+\Delta_{\text{pot}}^{\text{max}}$ | **100% LTP Potentiation** |
| **Causal Gradient** | $d_{\text{ltp}} = L_{\text{prop}} / 2$ | $+\frac{1}{2} \Delta_{\text{pot}}^{\text{max}}$ | **50% LTP Potentiation** |
| **Anti-Causal Peak** | $d_{\text{ltp}} = \infty, d_{\text{ltd}} = 0$ | $-\Delta_{\text{dep}}^{\text{max}}$ | **100% LTD Depression** |
| **Anti-Causal Gradient** | $d_{\text{ltd}} = L_{\text{prop}} / 2$ | $-\frac{1}{2} \Delta_{\text{dep}}^{\text{max}}$ | **50% LTD Depression** |
| **Complete Miss** | $d_{\text{ltp}} > L_{\text{prop}} \land d_{\text{ltd}} > L_{\text{prop}}$ | $0$ | **No Change** (Weight Preserved) |

---

## 4. Code & Test Reference

- **Research Module**: `crates/test-harness/src/mvp_cpu_replay.rs` (`research_apply_gsop_plasticity`)
- **Sandbox Integration**: `cpu_apply_gsop` in `crates/test-harness/src/mvp_cpu_replay.rs`
- **Unit Tests**:
  - `test_research_apply_gsop_plasticity_bidirectional_stdp` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_cpu_apply_gsop_spatial_cooling_attenuation` in `crates/test-harness/tests/mvp_cpu_replay.rs`
