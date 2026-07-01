# Legacy Baseline Calibration Status

This document summarizes the calibration state of the legacy biological neuron baseline within AxiEngine.

---

## 1. Selected Baseline E/I Pair

To evaluate biological parameters in a recurrent network, we utilize the following canonical pair from the legacy library (`W:\Workspace\axicor-master\Axicor_Neuron-Lib`):
* **Excitatory Type**: `Cortex/L23/spiny/VISp23/1.toml`
  - Name: `L23_spiny_VISp23_1`
  - Threshold: `-47781` $\mu V$
  - Rest Potential: `-81161` $\mu V$
* **Inhibitory Type**: `Cortex/L23/aspiny/VISp23/1.toml`
  - Name: `L23_aspiny_VISp23_1`
  - Threshold: `-47093` $\mu V$
  - Rest Potential: `-72712` $\mu V$

---

## 2. Parameter Categorization

### Trusted Parameters (High Confidence)
These represent experimentally measured biophysical values mapped directly from the Allen Cell Types Database electrophysiology sweeps:
* **Membrane Potential Bounds**: `threshold`, `rest_potential`, and `ahp_amplitude`. Stored directly as absolute signed microvolt integers (`i32`), allowing native integer arithmetic.
* **Basic Durations**: `refractory_period` and `is_inhibitory`.
* **Pacemaking Periods**: `spontaneous_firing_period_ticks` (values $> 65535$ are clamped or treated as disabled).

### Heuristic Parameters (Calibrated/Engine-Dependent)
These represent algorithmic weights and scales designed to regulate routing, structural growth, and learning plasticity:
* **Baker Layout Dimensions**: `steering_fov_deg`, `steering_radius_um`, `growth_vertical_bias`, and `dendrite_radius_um`. These control spatial dendritic/axonal routing during synapse creation.
* **Connection Strength & Pruning**: `initial_synapse_weight` and `prune_threshold`.
* **Structural Plasticity Rate**: `gsop_potentiation`, `gsop_depression`.
* **Adaptation & Homeostasis**: `homeostasis_penalty` and `homeostasis_decay`.

---

## 3. Stability Boundary Zone

Through a 30-run grid sweep mapping network outcomes under external stimulation (`single_pulse_2`), we identified a narrow stability boundary region separating rapid extinction from infinite sustained activity:

* **Soma Density**: `0.09..0.10`
* **Inhibitory Share**: `0.25..0.30`

Below a density of `0.09`, the network is too sparse to propagate activity. Above a density of `0.10` with less than `0.25` inhibitory share, recurrent excitation leads to infinite sustained activity.

---

## 4. Representative Regimes

Our temporal trace simulations successfully isolated three distinct response shapes within this stability boundary:

1. **Immediate Transient** (`density = 0.08, inhibitory_share = 0.20`):
   - Activates at tick 12 with only 2 spikes and immediately decays to 0. No recruitment occurs.
2. **Long Transient** (`density = 0.10, inhibitory_share = 0.25`):
   - Slow recruitment: Activates at tick 12, peaking at **21 spikes** on tick **328** due to slow recurrent build-up.
   - Gradual decay: Linear decline (slope $\approx$ `-0.12`) over 165 ticks, dying out completely at tick **493** (total spikes: 2,732).
3. **Sustained Activity** (`density = 0.09, inhibitory_share = 0.225`):
   - Activates at tick 12, peaking at **34 spikes** on tick **498**.
   - Slow decay (slope $\approx$ `-0.06`) but remains active until the end of the simulation at tick **999** (total spikes: 12,761).

---

## 5. Conclusions & Next Steps

### Conclusion
The AxiEngine simulation pipeline (config $\rightarrow$ baker $\rightarrow$ bootstrap $\rightarrow$ compute runtime) responds meaningfully and deterministically to connectivity damping and E/I ratio changes. However, direct porting of legacy TOML files yields runaway activity or silence unless the growth, synapse weights, and pruning parameters are adapted to align with AxiEngine's current `topology` and synchronization batch boundaries.

### Next Practical Step
Establish a target baseline regime for sensory cortical modeling. Specifically, calibrate the network to achieve a **long transient response** (e.g. peaking at 15–30 spikes with a decay window of 300–500 ticks following sensory stimulation), preventing runaway sustained loops while enabling temporary recurrent signal integration.
