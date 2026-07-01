# Biological Calibration Plan

This plan describes the methodology and validation steps for the reference biological calibration of AxiEngine's neuron types on a selected subset of well-characterized neurons.

---

## 1. Required Empirical Biophysical Data
To calibrate the simulated integrate-and-fire model against actual biological recordings, we require the following data per neuron profile:
* **Resting Membrane Potential ($V_{\text{rest}}$)**: The baseline voltage of the cell in the absence of input currents.
* **Spike Threshold ($V_{\text{thresh}}$)**: The voltage level at which an action potential is triggered.
* **Input Resistance ($R_{\text{in}}$)**: Determines the steady-state voltage change in response to injected current.
* **Membrane Time Constant ($\tau_m$)**: The exponential decay rate of the membrane potential.
* **Rheobase Current ($I_{\text{rheo}}$)**: The minimum current amplitude of infinite duration (typically a long step pulse) required to trigger a single action potential.
* **Spike Frequency Adaptation (SFA) Trajectory**: The progressive lengthening of inter-spike intervals under sustained step current.
* **f-I Curve Data**: The firing frequency ($f$) as a function of injected step current ($I$).

---

## 2. Parameter Status in Legacy TOML
Several biophysical properties are already encoded directly or indirectly in the flat legacy TOML configs:
* **Directly Encoded**:
  - `rest_potential` (in $\mu V$, e.g. `-81161`)
  - `threshold` (in $\mu V$, e.g. `-47781`)
  - `refractory_period` (ticks)
  - `ahp_amplitude` (in $\mu V$, e.g. `5000`)
  - `spontaneous_firing_period_ticks`
* **Indirectly Encoded**:
  - `leak_shift`: Calibrated to approximate $\tau_m$ using exponential powers of 2.

---

## 3. Metadata to Retrieve from Allen NWB/GLIF
To resolve scaling mismatches and calibrate current/weight conversions, the following parameters should be pulled from the **Allen Cell Types Database (Neurodata Without Borders / NWB files)** or the **GLIF (Generalized Leak-Integrate-and-Fire) models metadata**:
* **Membrane Capacity ($C_m$) and Input Resistance ($R_m$)**: Essential to translate physical nanoamperes (nA) or picoamperes (pA) of injected current into digital voltage steps.
* **Experimental f-I Curves**: Raw experimental sweep files containing exact spike times under varied step current levels.
* **GLIF Model Coefficients**: For advanced integrate-and-fire configurations, GLIF metadata contains exact fit constants for after-spike currents (ASC) and voltage/threshold adaptation dynamics.

---

## 4. Required Harness Test Suite
The new test harness must support automated single-cell simulation protocols:
1. **No-Input Resting Stability**:
   - Run the simulation with zero input current. Verify that the membrane potential remains stable at $V_{\text{rest}}$ indefinitely.
2. **Step Current Injection**:
   - Inject a constant current block for a specified duration and record the voltage trace and spike timings.
3. **Rheobase Search**:
   - Perform binary search over step current intensities to find the minimum current threshold ($I_{\text{rheo}}$) that evokes exactly one spike within a defined window.
4. **f-I Curve Calibration**:
   - Sweep injected currents from sub-threshold to saturating levels. Record the steady-state firing frequency and compare it to experimental f-I sweeps.
5. **Membrane Decay Test**:
   - Inject a short sub-threshold pulse, then terminate the input. Measure the exponential voltage decay back to rest to validate the simulated $\tau_m$ (leakage shift).
6. **Refractory & Adaptation Verification**:
   - Inject super-rheobase step currents to analyze inter-spike intervals, validating that the simulated AHP and homeostasis penalties replicate biological Spike Frequency Adaptation.

---

## 5. Engine Parameter Calibration Plan

Calibration is divided into two sequential layers:

### Primary Calibration (Single-Cell Biophysics)
* **Trusted Constants**:
  - `rest_potential`: Initial membrane voltage.
  - `threshold`: Base spike voltage threshold.
  - `leak_shift`: Mapped to approximate the empirical $\tau_m$.
* **Adjustable Knobs**:
  - `current_scale`: Scalar matching input current units (e.g. pA) to internal digital voltage increments.
  - `weight_scale`: Scaling factor for postsynaptic potential increments.
  - `refractory_period`: Absolute clamp duration after a spike.
  - `homeostasis_penalty` / `homeostasis_decay`: Adapts threshold or subtracts potential to model Spike Frequency Adaptation.
  - `ahp_amplitude`: Post-spike hyperpolarization magnitude.
  - `spontaneous_firing_period_ticks`: Set to 0 if spontaneous firing is not experimentally observed.

### Secondary Calibration (Structural/Network Dynamics - Later Stage)
* **Adjustable Knobs**:
  - `steering_fov_deg` / `steering_radius_um` / `growth_vertical_bias` / `dendrite_radius_um` (Baker routing rules).
  - `type_affinity` and sprout weights (objective routing functions).
  - `initial_synapse_weight` and `prune_threshold` (structural plasticity stability).

---

## 6. Library-Wide Generalization Rules
Once rules are discovered and validated on the reference sample:
1. **Establish a Conversion Formula**: Formulate analytical scaling rules (e.g., mapping $R_{\text{in}}$ and $\tau_m$ directly to equivalent `leak_shift` and `current_scale` parameters).
2. **Batch Import Manifest**: Implement the translation rules within the import script or generated manifests (avoiding manual, file-by-file adjustments).
3. **Automated Validation Suite**: Run the generalized configuration profiles through the single-cell harness test suite to ensure that all imported profiles fall within standard biological bounds (e.g. correct rheobase ranges) prior to network baking.
