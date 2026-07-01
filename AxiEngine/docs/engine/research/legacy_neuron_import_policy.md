# Legacy Neuron Import Policy

This policy document defines the guidelines and translation rules for importing neuron configurations from the legacy library (`W:\Workspace\axicor-master\Axicor_Neuron-Lib`) into the nested configuration formats of AxiEngine.

---

## 1. Flat Legacy [[neuron_type]] Format

The legacy format is a flat TOML file representing neuron configurations under the `[[neuron_type]]` array. 

* **File Layout**: Although the parser/importer must be capable of processing a list containing multiple profiles, the current legacy library inventory contains exactly one `[[neuron_type]]` block per file.
* **Invariance Rule**: The source legacy TOML files are read-only and must never be modified in-place. Any adjustments, sanitization, or parameter clamping must be applied exclusively to the generated configuration outputs and the import manifests.

An example of this format is shown below:

```toml
[[neuron_type]]
name = "L1_aspiny_VISam1"
threshold = -48031
rest_potential = -70161
leak_shift = 7
homeostasis_penalty = 1000
spontaneous_firing_period_ticks = 289
initial_synapse_weight = 1326
gsop_potentiation = 200
gsop_depression = 2
homeostasis_decay = 8
refractory_period = 18
synapse_refractory_period = 15
signal_propagation_length = 18
is_inhibitory = true
inertia_curve = [128, 125, 123, 121, 119, 117, 115, 114]
ahp_amplitude = 5000
adaptive_leak_min_shift = 5
adaptive_leak_gain = 0
adaptive_mode = 0
d1_affinity = 64
d2_affinity = 255

# --- Baker Sprouting Params (Not in VRAM) ---
steering_fov_deg = 147.2
steering_radius_um = 33.0
growth_vertical_bias = 0.11
dendrite_radius_um = 150.0
type_affinity = 0.2
sprouting_weight_distance = 0.7
sprouting_weight_power = 0.3
sprouting_weight_explore = 0.3
sprouting_weight_type = 0.3
steering_weight_inertia = 0.2
steering_weight_sensor = 0.6
steering_weight_jitter = 0.2
prune_threshold = 331
```

---

## 2. Layout of Fields in the Nested Configuration

AxiEngine groups properties logically into nested structures within `crates/config/src/dto.rs` under the `NeuronType` DTO. Legacy fields must be mapped to their nested targets as follows:

| Legacy Field | AxiEngine Nested Target | Type / Notes |
|:---|:---|:---|
| `name` | `NeuronType::name` | `String` (Subject to sanitization policy). |
| **Membrane Parameters** | | |
| `threshold` | `NeuronType::membrane.threshold` | `i32` |
| `rest_potential` | `NeuronType::membrane.rest_potential` | `i32` |
| `leak_shift` | `NeuronType::membrane.leak_shift` | `u32` (Cast from `u8`) |
| `ahp_amplitude` | `NeuronType::membrane.ahp_amplitude` | `u16` (Cast from `i32`) |
| **Timing Parameters** | | |
| `refractory_period` | `NeuronType::timing.refractory_period` | `u8` |
| `synapse_refractory_period` | `NeuronType::timing.synapse_refractory_period` | `u8` |
| **Signal Parameters** | | |
| `signal_propagation_length` | `NeuronType::signal.signal_propagation_length` | `u8` (Cast from `u32`) |
| **Homeostasis Parameters** | | |
| `homeostasis_penalty` | `NeuronType::homeostasis.homeostasis_penalty` | `i32` |
| `homeostasis_decay` | `NeuronType::homeostasis.homeostasis_decay` | `u16` (Cast from `u8`) |
| **Adaptive Leak Parameters** | | |
| `adaptive_leak_min_shift` | `NeuronType::adaptive_leak.adaptive_leak_min_shift` | `i32` (Cast from `u8`) |
| `adaptive_leak_gain` | `NeuronType::adaptive_leak.adaptive_leak_gain` | `u16` (Cast from `u8`) |
| `adaptive_mode` | `NeuronType::adaptive_leak.adaptive_mode` | `u8` |
| **Dopamine Parameters** | | |
| `d1_affinity` | `NeuronType::dopamine.d1_affinity` | `u8` |
| `d2_affinity` | `NeuronType::dopamine.d2_affinity` | `u8` |
| **GSOP (Plasticity) Parameters** | | |
| `gsop_potentiation` | `NeuronType::gsop.gsop_potentiation` | `u16` |
| `gsop_depression` | `NeuronType::gsop.gsop_depression` | `u16` |
| `initial_synapse_weight` | `NeuronType::gsop.initial_synapse_weight` | `u16` |
| `is_inhibitory` | `NeuronType::gsop.is_inhibitory` | `bool` |
| `inertia_curve` | `NeuronType::gsop.inertia_curve` | `Vec<u8>` |
| **Growth (Baker) Parameters** | | |
| `steering_fov_deg` | `NeuronType::growth.steering_fov_deg` | `f32` |
| `steering_radius_um` | `NeuronType::growth.steering_radius_um` | `f32` |
| `steering_weight_inertia` | `NeuronType::growth.steering_weight_inertia` | `f32` |
| `steering_weight_sensor` | `NeuronType::growth.steering_weight_sensor` | `f32` |
| `steering_weight_jitter` | `NeuronType::growth.steering_weight_jitter` | `f32` |
| `dendrite_radius_um` | `NeuronType::growth.dendrite_radius_um` | `f32` |
| `growth_vertical_bias` | `NeuronType::growth.growth_vertical_bias` | `f32` |
| `type_affinity` | `NeuronType::growth.type_affinity` | `f32` |
| `sprouting_weight_distance` | `NeuronType::growth.sprouting_weight_distance` | `f32` |
| `sprouting_weight_power` | `NeuronType::growth.sprouting_weight_power` | `f32` |
| `sprouting_weight_explore` | `NeuronType::growth.sprouting_weight_explore` | `f32` |
| `sprouting_weight_type` | `NeuronType::growth.sprouting_weight_type` | `f32` |
| **Spontaneous Parameters** | | |
| `spontaneous_firing_period_ticks` | `NeuronType::spontaneous.spontaneous_firing_period_ticks` | `u32` (Subject to overflow policy) |

> [!NOTE]
> **Pruning Threshold Mapping Mismatch**: In the legacy neuron library, each `[[neuron_type]]` can define its own `prune_threshold` (e.g. `5` or `331`). In AxiEngine's schema, pruning threshold is set globally at the shard level via `ShardSettings::prune_threshold` (inside `dto.rs` line 196). It must not be added to the individual `NeuronType` configurations.

---

## 3. Name Sanitization Policy

To prevent parsing errors or configuration serialization issues, the `name` attribute must undergo sanitization **only during the generation of the exported configuration**.

* **Standard**: Names should only contain alphanumeric characters (`A-Za-z0-9`), hyphens (`-`), and underscores (`_`). Hyphens are explicitly allowed by AxiEngine's schema validator regex.
* **Replacement Rules**: All spaces (` `), slashes (`/`, `\`), and other invalid symbols must be replaced with an underscore (`_`).
* **Uniqueness Validation**: The importer tool must perform a post-sanitization check to ensure that all sanitized names remain globally unique. If a name collision occurs after sanitization (e.g. if two original names differed only by a space vs an underscore), the importer must raise a validation error and abort the configuration generation.
* **Example**:
  * Original Name: `L23_spiny_SSp-bfd23` -> Sanitized Name: `L23_spiny_SSp-bfd23` (No change, hyphen preserved).
  * Original Name: `L1 aspiny VISam1` -> Sanitized Name: `L1_aspiny_VISam1` (Spaces replaced).
* **Reference Logging**: The original legacy name must be kept in the import reference log (e.g. CSV manifests) to maintain traceability.

---

## 4. Spontaneous Firing Overflow Policy

In the legacy configuration, a very high spontaneous firing period value is used to disable spontaneous firing.
* **Condition**: If `spontaneous_firing_period_ticks > 65536`.
* **Action**: Set `SpontaneousParams::spontaneous_firing_period_ticks = 0` (or `disabled` per the target engine spec).
* **Rationale**: Registers storing spontaneous firing ticks are optimized. Period values larger than 65536 ticks represent a period longer than normal simulation epochs and must be cleanly set to `0` (disabled) to avoid numeric overflows or unintended behavior.

---

## 5. Negative Threshold-Rest Potential Policy (Edge Cases)

* **Condition**: If `threshold - rest_potential < 0` (the activation threshold is lower than the rest potential).
* **Action**:
  1. Flag the configuration file as a **non-standard biological edge case** in the import manifests.
  2. **Do not select** these configurations as candidates for the initial baseline validation run.
* **Rationale**: Under integrate-and-fire simulation rules, a neuron with a threshold lower than its rest potential will be in a state of constant excitation, firing continuously on every available tick (once refractory periods end). This creates unstable, self-exciting network loops that interfere with baseline validation.

---

## 6. Dendrite Whitelisting Policy

* **Legacy Status**: The `dendrite_whitelist` parameter does not exist in the flat legacy `[[neuron_type]]` configuration.
* **AxiEngine Requirement**: `GrowthParams::dendrite_whitelist` (a `Vec<String>`) must be defined in the nested configuration of postsynaptic targets.
* **Phase Application**: The whitelist constraint is evaluated during the **local synapse formation** phase (establishing active connections when axon terminals overlap with target dendrites), **not** during the structural pathfinding growth steps.
* **Whitelist Semantics**: The whitelist defined inside a postsynaptic target neuron type defines the allowed **source (presynaptic) neuron type names** that can form synaptic inputs onto its dendritic tree.
* **Import Rules**:
  1. **Default**: Populate as an empty vector (`[]`), representing no restriction (all source type connections allowed) or disabled constraints.
  2. **Composition-Based Overlay**: If selective targeting is required, connection mapping rules must be injected via an external configuration overlay during compilation rather than being hardcoded into the imported neuron profiles.
