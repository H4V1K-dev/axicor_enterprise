# Legacy Neuron Parameter Provenance Map

This document tracks the origins, mapping formulas, confidence levels, and integration risks for all parameters in the legacy neuron library (`W:\Workspace\axicor-master\Axicor_Neuron-Lib`).

---

## Parameter Provenance Table

| Parameter Field | Provenance Category | Formula / Scale | Confidence | Risk for AxiEngine |
|:---|:---|:---|:---|:---|
| **`name`** | Derived from Allen | Layers (`L1`-`L6`), morphology (`spiny`/`aspiny`), and region. | **High** | **Low**. Requires character sanitization (replacing spaces/slashes with `_`) to meet AxiEngine name regex rules. |
| **`threshold`** | Allen | Absolute potential ($\mu V$, e.g. `-48031`). | **High** | **Low**. Supported directly as absolute signed integers (`i32`). Relative threshold differences must be monitored. |
| **`rest_potential`** | Allen | Absolute potential ($\mu V$, e.g. `-70161`). | **High** | **Low**. Maps directly as the baseline voltage. |
| **`leak_shift`** | Old-Engine Heuristic | $\tau_{\text{membrane}} \approx 2^{\text{leak\_shift}}$ ticks. | **High** | **Medium**. Exponential bit-shift means small adjustments drastically scale membrane leakage rate. |
| **`refractory_period`** | Derived from Allen | Millisecond sweeps duration mapped to ticks. | **High** | **Low**. Clamps soma firing after an active spike. |
| **`synapse_refractory_period`** | Old-Engine Heuristic | Time interval (ticks) between updates. | **Medium** | **Medium**. Controls maximum signal transmission frequency per synapse. |
| **`spontaneous_firing_period_ticks`** | Allen / Heuristic | Pacemaker sweep rates (clamped/zeroed > 65535). | **High** | **Low**. Needs range checks/clamping to prevent `u16` buffer overflow when spontaneous is disabled (`100000`). |
| **`initial_synapse_weight`** | Old-Engine Heuristic | Starting connection weight scale. | **Medium-Low** | **High**. Directly sets initial postsynaptic potential impact. If miscalibrated, leads to instant network runaway or silence. Requires recalibration. |
| **`gsop_potentiation` / `gsop_depression`** | Old-Engine Heuristic | Growth sprouting plasticity update rates. | **Medium-Low** | **High**. Controls Hebbian synapse strengthening and weakening. Requires recalibration. |
| **`homeostasis_penalty` / `homeostasis_decay`** | Old-Engine Heuristic | Activity-dependent firing threshold penalty. | **Medium** | **Medium**. Regulates firing frequency under long-term stimulation. |
| **`signal_propagation_length`** | Old-Engine Heuristic | Max path length for signal updates. | **Medium** | **Medium**. Limits transmission distances. |
| **`is_inhibitory`** | Allen | Morphology / Transgenic line properties. | **High** | **Low**. Dictates whether connection weights are applied as positive or negative. |
| **`inertia_curve`** | Old-Engine Heuristic | Vector of directional pathfinding weights. | **Medium** | **Medium**. Influences axon layout topology; engine-dependent. |
| **`ahp_amplitude`** | Allen | After-hyperpolarization voltage drop ($\mu V$). | **High** | **Low**. Directly influences threshold adaptation offset. |
| **`adaptive_leak_min_shift` / `adaptive_leak_gain`** | Old-Engine Heuristic | Acceleration of leakage during high activity. | **Medium** | **Medium**. Alters local integration behavior under high-frequency inputs. |
| **`adaptive_mode`** | Old-Engine Heuristic | Flag selecting adaptation equation logic. | **Medium** | **Medium**. Requires alignment with AxiEngine's implementation of adaptive leaks. |
| **`d1_affinity` / `d2_affinity`** | Derived from Allen | Dopamine receptor density indicators. | **Medium** | **Medium**. Regulates reinforcement learning sensitivity. |
| **`steering_fov_deg` / `steering_radius_um` / `growth_vertical_bias` / `dendrite_radius_um`** | Derived from Allen / Heuristic | Anatomical volume dimensions and biased orientation. | **Medium** | **High**. Alters the spatial probability and density of synapses during baking. Minor changes can disrupt network ignition. |
| **`type_affinity` / `sprouting_weight_distance` / `sprouting_weight_power` / `sprouting_weight_explore` / `sprouting_weight_type` / `steering_weight_inertia` / `steering_weight_sensor` / `steering_weight_jitter`** | Old-Engine Heuristic | Axonal growth pathfinding objective scoring weights. | **Medium** | **High**. Directly determines connectome matrix topology; engine-dependent and highly sensitive. |
| **`prune_threshold`** | Old-Engine Heuristic | Minimum synapse weight below which deletion occurs. | **Medium-Low** | **High**. Directly governs structural plasticity stability. Requires recalibration. |

---

## Detailed Provenance Notes

### 1. High Confidence Parameters
* **Electrophysiology Core (`threshold`, `rest_potential`, `ahp_amplitude`, `refractory_period`)**:
  - **Source**: Directly matching the patch-clamp sweep recordings of the Allen Cell Types Database.
  - **AxiEngine Mapping**: Stored as native negative integer values representing microvolts, matching the simulator's capability to run absolute integer arithmetic without normalization.
* **Basal Firing (`spontaneous_firing_period_ticks`)**:
  - **Source**: Extracted from spontaneous pacemaking protocols. Values greater than `65535` are designated as disabled or mapped to `0`.

### 2. Medium Confidence Parameters (Engine-Dependent)
* **Baker Growth Layouts (`steering_fov_deg`, `steering_radius_um`, `growth_vertical_bias`, `dendrite_radius_um`, `inertia_curve`)**:
  - **Source**: Calibrated from biological morphology reconstruction data (e.g. neuron reconstructions on NeuroMorpho.org or Allen Brain Map) to replicate typical axonal/dendritic arbor spans.
  - **AxiEngine Mapping**: These govern the spatial distribution of axonal pathfinding during the Baker routing stage. Changes in the routing algorithm will shift the total synapse counts.

### 3. Medium-Low Confidence Parameters (Recalibration Required)
* **Hebbian Plasticity & Connectivity Constraints (`initial_synapse_weight`, `gsop_potentiation`, `gsop_depression`, `prune_threshold`)**:
  - **Source**: Calibrated via numerical heuristics in the legacy Axicor engine to stabilize large-scale network loops.
  - **AxiEngine Mapping**: Because AxiEngine uses optimized compute kernels, integer scales, and synchronization batches, these parameters are highly sensitive to numerical differences. Direct porting without calibration runs a high risk of inducing either absolute silence or runaway excitation.
