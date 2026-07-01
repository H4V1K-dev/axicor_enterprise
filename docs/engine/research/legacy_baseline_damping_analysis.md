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

## Conclusion & Next Steps
To establish a stable "boundary regime" between transient and sustained activity, future research should explore the following parameters:
- **Inhibitory Share**: `0.20..0.30`
- **Soma Density**: `0.05..0.10`
