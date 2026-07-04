# Linear Spatial Cooling (STDP Gradient) Research Specification

## 1. Context & Motivation

During biocalibration research on MVP CPU replay, the discrete bitwise shift model (`cooling_shift = min_dist >> 4`) produced artificial quantization step-downs in LTP magnitude every 16 segments.

To provide smooth distance-dependent plasticity without introducing floating-point math, an isolated linear STDP gradient function `research_apply_gsop_plasticity` was created inside `crates/test-harness`.

---

## 2. Mathematical Formulation

### 2.1 Legacy Stepwise Model (Discrete Shift)

$$\text{cooling\_shift} = \begin{cases} \lfloor d_{\text{min}} / 16 \rfloor & \text{if } d_{\text{min}} \le L_{\text{prop}} \\ 0 & \text{otherwise} \end{cases}$$

$$\Delta_{\text{pot}}^{\text{discrete}} = \left( \frac{\text{final\_pot} \cdot \text{inertia} \cdot \text{burst\_mult}}{128} \right) \gg \text{cooling\_shift}$$

---

### 2.2 Active Research Linear STDP Gradient Model

$$\text{decay\_factor} = \text{saturating\_sub}(L_{\text{prop}}, d_{\text{min}}) = \max(0, L_{\text{prop}} - d_{\text{min}})$$

$$\Delta_{\text{pot}}^{\text{linear}} = \begin{cases} \left\lfloor \frac{\text{final\_pot} \cdot \text{inertia} \cdot \text{burst\_mult} \cdot (L_{\text{prop}} - d_{\text{min}})}{128 \cdot L_{\text{prop}}} \right\rfloor & \text{if } d_{\text{min}} \le L_{\text{prop}} \land L_{\text{prop}} > 0 \\ 0 & \text{otherwise} \end{cases}$$

$$\Delta_{\text{dep}} = \left\lfloor \frac{\text{final\_dep} \cdot \text{inertia} \cdot \text{burst\_mult}}{128} \right\rfloor$$

$$\Delta = \begin{cases} \Delta_{\text{pot}}^{\text{linear}} & \text{if } d_{\text{min}} \le L_{\text{prop}} \quad (\text{Active Tail Hit}) \\ -\Delta_{\text{dep}} & \text{if } d_{\text{min}} > L_{\text{prop}} \quad (\text{Inactive Miss / LTD}) \end{cases}$$

---

## 3. Boundary Conditions & Phase Table

| Distance $d_{\text{min}}$ | $\text{decay\_factor}$ | Relative LTP Strength | Plasticity Mode |
| :--- | :--- | :--- | :--- |
| $0$ | $L_{\text{prop}}$ | **100%** | Peak Potentiation |
| $L_{\text{prop}} / 2$ | $L_{\text{prop}} / 2$ | **50%** | Midpoint Linear Attenuation |
| $L_{\text{prop}}$ | $0$ | **0%** | Zero Delta Boundary |
| $> L_{\text{prop}}$ | $0$ | N/A | **Full LTD Penalty** ($-\Delta_{\text{dep}}$) |

---

## 4. Code & Test Reference

- **Research Module**: `crates/test-harness/src/mvp_cpu_replay.rs` (`research_apply_gsop_plasticity`)
- **Sandbox Integration**: `cpu_apply_gsop` in `crates/test-harness/src/mvp_cpu_replay.rs`
- **Unit Tests**:
  - `test_research_apply_gsop_plasticity_linear_stdp_gradient` in `crates/test-harness/tests/mvp_cpu_replay.rs`
  - `test_cpu_apply_gsop_spatial_cooling_attenuation` in `crates/test-harness/tests/mvp_cpu_replay.rs`
