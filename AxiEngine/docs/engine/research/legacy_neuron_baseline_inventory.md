# Legacy Neuron Baseline Inventory

This document presents an analysis of the legacy neuron library located in `W:\Workspace\axicor-master\Axicor_Neuron-Lib`. We scanned all 2,333 TOML configurations to extract parameters, categorize cells, identify potential baseline pairs, and document mapping alignments, limitations, and domain/scale conflicts with AxiEngine.

---

## Direct Parameter Mapping to AxiEngine

Several parameters in the legacy TOML files correspond directly to the runtime configurations of modern spiking neural network simulators (like AxiEngine):

| Legacy Field | AxiEngine Target Component / Field | Description | Directness |
|:---|:---|:---|:---|
| `name` | `NeuronType::name` / Identifier | Human-readable configuration name. | Direct string mapping. |
| `is_inhibitory` | Connection/Synapse Type determination | Sets whether target synaptic connections have positive (excitatory) or negative (inhibitory) weights. | Direct boolean mapping. |
| `leak_shift` | `MembraneParams::leak_shift` | Exponential leakage bit-shift factor. | Direct integer mapping. |
| `refractory_period` | `MembraneParams::refractory_period` | Number of ticks after firing during which the soma ignores inputs. | Direct integer mapping. |
| `synapse_refractory_period` | Synaptic refractory duration | Minimum ticks between consecutive signal updates at a single synapse. | Direct integer mapping. |
| `adaptive_leak_min_shift` | Adaptive leak base bit-shift | Minimal leakage shift value during high activity. | Direct integer mapping. |
| `adaptive_leak_gain` | Adaptive leak gain factor | Coefficient scaling leakage increase with activity. | Direct integer mapping. |
| `prune_threshold` | Connection pruning threshold | Synapse weight below which a connection is pruned. | Direct integer mapping. |

---

## Domain & Scale Conflicts (Requires Manual Resolution)

### 1. Membrane Threshold & Rest Potential Interpretation
* **Legacy scale**: Threshold and rest potential values are defined as absolute negative integers (in microvolts, $\mu V$). For example, a rest potential of `-70161` $\mu V$ represents $-70.161\text{ mV}$, and a threshold of `-48031` $\mu V$ represents $-48.031\text{ mV}$.
* **AxiEngine interpretation**: The modern AxiEngine physics model supports storing voltage as absolute signed integers (`i32`), where the initial soma voltage is set to `rest_potential`, and a spike fires when `voltage >= threshold` (with optional threshold adaptation offsets).
* **Resolution**: The legacy absolute negative microvolt integers (e.g. `-81161` and `-47781`) can be mapped directly to the engine without normalization or shifting, allowing direct validation in their native scale.

### 2. Spontaneous Firing Period Range Limitation
* **Legacy range**: Several neurons have `spontaneous_firing_period_ticks = 100000`.
* **AxiEngine limit**: If spontaneous firing periods are stored in a standard unsigned 16-bit register (`u16`), the maximum representable value is `65535`. A value of `100000` will overflow.
* **Resolution**: Ticks greater than `65535` should either be clamped to `65535` or mapped to `0` (which typically represents "spontaneous firing disabled").

### 3. Excitatory vs. Inhibitory Pruning Threshold Scales
* **Legacy asymmetry**: Excitatory neurons consistently utilize `prune_threshold = 5` (very low pruning barrier). Inhibitory neurons consistently utilize a high `prune_threshold` ranging from `235` to `384`.
* **AxiEngine mapping**: If the engine assumes a unified global pruning threshold, this asymmetry is lost. The engine or assembly tools must support type-specific pruning rules.

### 4. Negative Relative Thresholds (Edge Case)
* **Legacy anomaly**: In some configurations, such as `Cortex/L1/aspiny/MTG/3.toml`, the absolute threshold (`-61218`) is lower than the rest potential (`-60420`), leading to a negative relative difference (`-798`).
* **Simulation impact**: Under standard absolute threshold integrate-and-fire equations (`voltage >= threshold`), this neuron is immediately active at rest potential, causing it to fire constantly on every available tick (once refractory periods end). This indicates either a biological modeling edge case (constitutive pacemaker activity) or a legacy calibration artifact.

### 5. Learning & Plasticity Parameters (GSOP)
* **Legacy fields**: `gsop_potentiation` and `gsop_depression` regulate Hebbian/Sprouting plasticity.
* **AxiEngine mapping**: AxiEngine's core simulation loop does not evaluate these fields in VRAM during execution, as network plasticity/growth is managed out-of-band by the assembly generator tools.

---

## Selected Baseline Candidates

To build a baseline validation pair representing typical cortical behavior, we selected **5 Excitatory** and **5 Inhibitory** candidates representing different layers and visual cortex regions:

### Excitatory Candidates (Spiny Pyramidal Somateypes)

1. **L23_spiny_VISp23_1** (Layer 2/3 Primary Visual Cortex)
   * **Path**: `Cortex/L23/spiny/VISp23/1.toml`
   * **Threshold**: `-47781`
   * **Rest Potential**: `-81161`
   * **Relative Threshold ($\Delta$)**: `33380`
   * **Leak Shift**: `7`
   * **Spontaneous Firing Period**: `743` ticks
   * **Initial Synapse Weight**: `667`
   * **Refractory Period**: `14` ticks
   * **Prune Threshold**: `5`

2. **L23_spiny_SSp-bfd23** (Layer 2/3 Somatosensory Barrel Field)
   * **Path**: `Cortex/L23/spiny/SSp-bfd23.toml`
   * **Threshold**: `-43031`
   * **Rest Potential**: `-75216`
   * **Relative Threshold ($\Delta$)**: `32185`
   * **Leak Shift**: `6`
   * **Spontaneous Firing Period**: `405` ticks
   * **Initial Synapse Weight**: `643`
   * **Refractory Period**: `16` ticks
   * **Prune Threshold**: `5`

3. **L23_spiny_VISp23_10** (Layer 2/3 Primary Visual Cortex - Non-Spontaneous Variant)
   * **Path**: `Cortex/L23/spiny/VISp23/10.toml`
   * **Threshold**: `-42000`
   * **Rest Potential**: `-79324`
   * **Relative Threshold ($\Delta$)**: `37324`
   * **Leak Shift**: `7`
   * **Spontaneous Firing Period**: `100000` ticks (Requires range clamping)
   * **Initial Synapse Weight**: `746`
   * **Refractory Period**: `13` ticks
   * **Prune Threshold**: `5`

4. **L4_spiny_VISp4_1** (Layer 4 Primary Visual Cortex Star Pyramid)
   * **Path**: `Cortex/L4/spiny/VISp4/1.toml`
   * **Threshold**: `-44468`
   * **Rest Potential**: `-75066`
   * **Relative Threshold ($\Delta$)**: `30598`
   * **Leak Shift**: `8`
   * **Spontaneous Firing Period**: `1132` ticks
   * **Initial Synapse Weight**: `611`
   * **Refractory Period**: `15` ticks
   * **Prune Threshold**: `5`

5. **L5_spiny_VISp5_1** (Layer 5 Primary Visual Cortex Large Pyramid)
   * **Path**: `Cortex/L5/spiny/VISp5/1.toml`
   * **Threshold**: `-49000`
   * **Rest Potential**: `-76928`
   * **Relative Threshold ($\Delta$)**: `27928`
   * **Leak Shift**: `8`
   * **Spontaneous Firing Period**: `558` ticks
   * **Initial Synapse Weight**: `558`
   * **Refractory Period**: `16` ticks
   * **Prune Threshold**: `5`

---

### Inhibitory Candidates (Aspiny Interneurons)

1. **L1_aspiny_VISam1** (Layer 1 Visual Anteromedial Area Neurogliaform)
   * **Path**: `Cortex/L1/aspiny/VISam1.toml`
   * **Threshold**: `-48031`
   * **Rest Potential**: `-70161`
   * **Relative Threshold ($\Delta$)**: `22130`
   * **Leak Shift**: `7`
   * **Spontaneous Firing Period**: `289` ticks
   * **Initial Synapse Weight**: `1326`
   * **Refractory Period**: `18` ticks
   * **Prune Threshold**: `331`

2. **L23_aspiny_VISp23_1** (Layer 2/3 Visual Cortex Basket Cell)
   * **Path**: `Cortex/L23/aspiny/VISp23/1.toml`
   * **Threshold**: `-47093`
   * **Rest Potential**: `-72712`
   * **Relative Threshold ($\Delta$)**: `25619`
   * **Leak Shift**: `6`
   * **Spontaneous Firing Period**: `789` ticks
   * **Initial Synapse Weight**: `1536`
   * **Refractory Period**: `15` ticks
   * **Prune Threshold**: `384`

3. **L1_aspiny_MTG_3** (Layer 1 Middle Temporal Gyrus - Negative Relative Offset Anomaly)
   * **Path**: `Cortex/L1/aspiny/MTG/3.toml`
   * **Threshold**: `-61218`
   * **Rest Potential**: `-60420`
   * **Relative Threshold ($\Delta$)**: `-798` (Negative difference)
   * **Leak Shift**: `8`
   * **Spontaneous Firing Period**: `260` ticks
   * **Initial Synapse Weight**: `1200`
   * **Refractory Period**: `18` ticks
   * **Prune Threshold**: `300`

4. **L4_aspiny_VISp4_1** (Layer 4 Visual Cortex Basket/SST Interneuron)
   * **Path**: `Cortex/L4/aspiny/VISp4/1.toml`
   * **Threshold**: `-55687`
   * **Rest Potential**: `-72042`
   * **Relative Threshold ($\Delta$)**: `16355`
   * **Leak Shift**: `5`
   * **Spontaneous Firing Period**: `125` ticks
   * **Initial Synapse Weight**: `981`
   * **Refractory Period**: `24` ticks
   * **Prune Threshold**: `245`

5. **L5_aspiny_VISp5_1** (Layer 5 Visual Cortex SST Interneuron - Non-Spontaneous Variant)
   * **Path**: `Cortex/L5/aspiny/VISp5/1.toml`
   * **Threshold**: `-59187`
   * **Rest Potential**: `-74926`
   * **Relative Threshold ($\Delta$)**: `15739`
   * **Leak Shift**: `7`
   * **Spontaneous Firing Period**: `100000` ticks (Requires range clamping)
   * **Initial Synapse Weight**: `942`
   * **Refractory Period**: `22` ticks
   * **Prune Threshold**: `235`
