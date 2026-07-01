# Legacy Baseline Connectivity Damping Analysis

## Overview
This document records the findings of the legacy baseline connectivity damping sweeps, which evaluated the impact of E/I share, soma density, presynaptic target restrictions (whitelisting), and growth sprouting limitations on local network dynamics using unnormalized L23 VISp1 biological parameters.

## Core Findings

1. **Sustained Activity in Baseline**:
   - The default baseline E/I pair (density = 0.20, inhibitory share = 20%, spontaneous heartbeat enabled, no stimulus) initiates a self-sustained state (`sustained-activity`) immediately once spontaneous firing triggers at tick 743. No spikes are dropped (`dropped_ratio = 0.0`), showing stable recurrent loop propagation.

2. **Inhibitory Share Threshold**:
   - Raising the inhibitory share to **30%** (`inh_share_30`) successfully dampens recurrent activity, transitioning the network into a transient state (`transient-response`) that decays after **374 ticks**.
   - Raising it further to **40%** (`inh_share_40`) almost completely extinguishes the response, decaying within **1 tick** (yielding only 5 spikes total).

3. **Soma Density Boundaries**:
   - A soma density of **10%** (`density_10`) is still dense enough to sustain active feedback loops to the end of the simulation.
   - Reducing the density to **5%** (`density_05`) extinguishes the response immediately within **1 tick**, showing that sparse spacing prevents recurrent loops from forming.

4. **Excitatory Recurrence and Whitelisting**:
   - Whitelist restrictions show that E-to-E recurrence is critical to maintaining the active state:
     - **E accepts E/I, I accepts E only** (`whitelist_E_all_I_E`): Restricting inhibitory targets causes the network activity to decay after **164 ticks**.
     - **E accepts I only, I accepts E only** (`whitelist_E_I_I_E`): Removing E-to-E feedback loops entirely extinguishes the response within **1 tick**, showing the sustained state cannot be maintained without direct recurrent excitatory feedback.

5. **Max Sprouts Limit**:
   - Changing `max_sprouts` to `4` or `2` has no effect on network activity (it behaves identically to `8` sprouts). In the current pipeline, limiting sprouts only throttles growth rate but does not reduce the `total_synapses` successfully formed (`204,800` synapses). Therefore, `max_sprouts` is not a valid knob for network damping without separate validation of growth/synapse formation thresholds in `topology`.

## Stability Band Mapping

A 30-run grid search sweep across densities `[0.05..0.10]` and inhibitory shares `[0.20..0.30]` mapped the narrow boundary regime between transient extinction and infinite sustained activity:

* **Sustained Propagation Minimum (Density >= 0.09)**:
  - Below `density = 0.09`, all configurations decay immediately within **1 tick** after the pulse. The recurrent network is too sparse to propagate activity.
* **Transition Bifurcation (Density = 0.09)**:
  - `inhibitory_share <= 0.225` triggers **sustained-activity** (active to tick 999).
  - `inhibitory_share >= 0.250` decays immediately within **5 ticks** or **1 tick**.
* **Long-Lived Transients (Density = 0.10)**:
  - `inhibitory_share = 0.200` triggers **sustained-activity** (active to tick 999).
  - `inhibitory_share = 0.250` triggers a long-lived **transient-response** of **482 ticks** (active from tick 12 to 493).
  - `inhibitory_share = 0.300` triggers a long-lived **transient-response** of **836 ticks** (active from tick 12 to 847).

## Representative Temporal Traces

To understand the trajectory shapes of these responses, we simulated three key configurations and exported their per-tick temporal traces (available at `artifacts/legacy_baseline_traces/`):

1. **Immediate Transient** (`density = 0.08, inhibitory_share = 0.20`):
   - Activates at tick 12 with only 2 spikes (both excitatory) and immediately decays to 0. No recruitment occurs.
2. **Long Transient** (`density = 0.10, inhibitory_share = 0.25`):
   - Activates at tick 12, then recruits slowly over 316 ticks to reach peak activity of **21 spikes** at tick `328` (18 excitatory, 3 inhibitory).
   - After the peak, activity decays linearly (decay slope $\approx$ `-0.12`) over 165 ticks until it dies at tick `493` (total spikes: 2,732).
3. **Sustained** (`density = 0.09, inhibitory_share = 0.225`):
   - Activates at tick 12, recruits slowly to reach a peak of **34 spikes** at tick `498` (28 excitatory, 6 inhibitory).
   - After the peak, it enters a very slow decay (decay slope $\approx$ `-0.06`), remaining active to the end of the simulation at tick `999` (total spikes: 12,761).

These trajectories show a characteristic **slow-recruitment** pattern: when a stimulus is presented, it takes hundreds of ticks for recurrent pathways to reach maximum firing intensity, followed by a gradual decay.

## Conclusion & Next Steps
The precise stability boundary zone where the network transitions from immediate decay to infinite sustained activity is located at:
- **Soma Density**: `0.09..0.10`
- **Inhibitory Share**: `0.25..0.30`

This parameter coordinate represents the critical damping region where the network can sustain long-lived transients, making it the primary target for biological baseline calibrations.

