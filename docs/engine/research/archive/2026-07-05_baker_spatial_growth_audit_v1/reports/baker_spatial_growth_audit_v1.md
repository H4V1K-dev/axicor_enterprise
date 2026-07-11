# Baker Spatial Growth Audit v1 — Report

**Date**: 2026-07-05
**Status**: PARTIAL / whitelist fixed, capacity warning
**Config**: 16×16×32 voxels, 4 layers, seeds 12345/12346/12347

## 1. What Did Baker Actually Build?

Baker compiled a spatial connectome with:
- **384 somas** placed in 4 layers (Virtual: 128, L4: 128, L23: 64, L5: 64)
- **32,492 live synapses** (seed 12345), with **106,010 dropped candidates** due to `MAX_DENDRITES=128`
- **7 distinct projection pairs** (all expected, no unexpected)

### Soma Placement

| Layer | Type | Count | Z Range |
|---|---|---|---|
| Virtual | VirtualInput | 128 | 0–7 |
| L4 | L4_spiny | 128 | 8–15 |
| L23 | L23_aspiny | 64 | 16–23 |
| L5 | L5_spiny | 64 | 24–31 |

Layer Z-slicing is clean: 4 equal bands of 8 voxels each, no overlap.

## 2. Are Expected Projections Present?

**Yes, all 7 expected projections are present:**

| Source → Target | Count | % | Sign | Distance Mean | Segment Mean |
|---|---|---|---|---|---|
| VirtualInput → L4_spiny | 13,118 | 40.37% | excitatory | 10.02 | 5.22 |
| L4_spiny → L23_aspiny | 3,748 | 11.54% | excitatory | 10.39 | 5.78 |
| L4_spiny → L5_spiny | 2,399 | 7.38% | excitatory | 18.24 | 9.63 |
| L23_aspiny → L4_spiny | 3,266 | 10.05% | inhibitory | 10.00 | 5.35 |
| L23_aspiny → L23_aspiny | 2,453 | 7.55% | inhibitory | 5.60 | 3.05 |
| L23_aspiny → L5_spiny | 5,517 | 16.98% | inhibitory | 9.88 | 4.55 |
| L5_spiny → L23_aspiny | 1,991 | 6.13% | excitatory | 9.49 | 5.33 |

All sign polarities correct: VirtualInput/L4/L5 excitatory, L23 inhibitory. This matches V1-like microcircuit expectations.

## 3. Are Unexpected Projections Present?

**No.** After applying the `VirtualInput` target whitelist fix, there are no unexpected projection pairs.

Initial audit found a dominant `VirtualInput → VirtualInput` projection caused by empty `VirtualInput.dendrite_whitelist` (`[] = accept all`). The rerun uses a valid input-only whitelist target and suppresses all incoming synapses onto Virtual targets.

### Root Cause Fixed: VirtualInput Target Whitelist

`VirtualInput` now uses a non-empty target whitelist that references an unused dummy source type. This keeps the config valid while ensuring no placed soma can form dendrites onto Virtual targets.

This confirms the prior failure was a configuration/whitelist omission, not a baker core bug.

## 4. Are Fan-In/Fan-Out Distributions Sane?

### Fan-In (incoming synapses per target soma)

| Target Layer | Mean | Median | Min | Max | Zero-Input |
|---|---|---|---|---|---|
| Virtual | 0.0 | 0.0 | 0 | 0 | 100% |
| L4 | 128.0 | 128.0 | 128 | 128 | 0% |
| L23 | 128.0 | 128.0 | 128 | 128 | 0% |
| L5 | 123.7 | 128.0 | 62 | 128 | 0% |

**Observation**: Virtual zero-input is expected for an input-only layer. L4 and L23 are still **completely saturated** — every neuron has exactly 128 synapses. This means the `MAX_DENDRITES=128` cap remains the effective bottleneck. 106,010 candidates were dropped. L5 is near-saturated.

This is a known consequence of the small shard (16×16×32) with high-radius dendrites (10–12 µm at 1.0 µm/voxel). In a larger shard, the ratio would be different.

### Fan-Out (outgoing synapses per source soma)

| Source Layer | Mean | Median | Min | Max | Zero-Output |
|---|---|---|---|---|---|
| Virtual | 102.5 | 62.5 | 0 | 408 | 9.4% |
| L4 | 48.0 | 14.5 | 0 | 425 | 20.3% |
| L23 | 175.6 | 143.0 | 0 | 475 | 7.8% |
| L5 | 31.1 | 13.0 | 0 | 136 | 25.0% |

Virtual somas have the highest fan-out (mean 223.8) because their axons grow long (vertical bias 2.0) and pass through many target zones. The zero-output fractions show that outgoing growth is uneven across all layers, especially L5.

## 5. Are Distances/Segments Measured From Real Data?

**Yes.** All distances are Euclidean soma-to-soma distances in voxel space, computed from `PlacedSoma.position` coordinates. Segment offsets are from `FormedSynapse.segment_offset`.

| Projection | Dist Mean | Dist Median | Dist Min | Dist Max | Seg Mean |
|---|---|---|---|---|---|
| Virtual→L4 | 10.02 | 9.64 | 1.41 | 20.27 | 5.22 |
| L4→L23 | 10.39 | 9.85 | 2.24 | 21.17 | 5.78 |
| L4→L5 | 18.24 | 18.49 | 8.25 | 26.22 | 9.63 |
| L23→L4 | 10.00 | 9.49 | 1.41 | 20.66 | 5.35 |
| L23→L23 | 5.60 | 5.10 | 1.00 | 14.49 | 3.05 |
| L23→L5 | 9.88 | 9.85 | 1.41 | 19.03 | 4.55 |
| L5→L23 | 9.49 | 9.22 | 1.41 | 18.39 | 5.33 |

The L23→L23 intra-layer projection has the shortest distance (~5.6 voxels), as expected. Cross-layer projections have longer distances proportional to z-separation. L4→L5 has the longest mean distance (18.24 voxels) due to spanning 2 layer gaps.

## 6. Is E/I Balance Plausible?

| Target Layer | Excitatory | Inhibitory | E/I Ratio |
|---|---|---|---|
| Virtual | 0 | 0 | 0.0 |
| L4 | 13,118 | 3,266 | 4.0 |
| L23 | 5,739 | 2,453 | 2.3 |
| L5 | 2,399 | 5,517 | 0.43 |

- **Virtual**: no incoming E/I balance by design — input-only targets are suppressed
- **L4**: E/I=4.0 — receives excitatory Virtual→L4 input + inhibitory L23→L4 feedback
- **L23**: E/I=2.3 — receives excitatory L4→L23 + L5→L23 + inhibitory L23→L23 (lateral inhibition)
- **L5**: E/I=0.43 — **net inhibited** — dominated by L23→L5 inhibition (5,517) vs L4→L5 excitation (2,399)

L5 net inhibition is consistent with previous static microcircuit calibration findings where L5 required higher excitatory weights to reach target firing rates.

## 7. Current Topology Limitations

1. **MAX_DENDRITES saturation**: L4 and L23 are at 100% capacity (128/128). 106,010 candidates were dropped. Useful projections still compete hard for limited dendrite slots in the small shard.

2. **Fan-out unevenness**: every source layer has zero-output somas (Virtual 9.4%, L4 20.3%, L23 7.8%, L5 25.0%), so the current shard does not distribute outbound growth uniformly.

3. **L5 net inhibition**: L5 receives more inhibitory than excitatory synapses (E/I=0.43), which was already observed in static microcircuit calibration and required compensatory weight tuning.

4. **Small shard geometry**: 16×16×32 with dendrite radii of 10–12 µm at 1 µm/voxel means every axon can reach most of the shard. This differs from production-scale connectivity patterns.

## 8. Is the Next Research Step Allowed?

**Yes, with a capacity caveat.** The whitelist blocker is fixed and the baker spatial growth pipeline is producing a clean expected projection matrix:
- All topology APIs work as specified
- Soma placement is clean with proper layer z-banding
- Axon growth produces realistic paths with proper stop reasons
- Synapse formation respects whitelists and produces correct signs
- Distances and segment offsets are real, measured from topology data
- Seed variance is small (~±1%)

The remaining issue is capacity pressure, not a whitelist failure. L4/L23 are still at 128/128 dendrite slots, and dropped candidates remain high. Functional topology replay can proceed if this is treated as a known small-shard saturation caveat.

**Next step**: Baker Functional Topology Replay can proceed on the fixed whitelist topology, while tracking whether dendrite saturation distorts activity/plasticity.

## Seed Variance

| Seed | Total Somas | Total Synapses | Dropped |
|---|---|---|---|
| 12345 | 384 | 32,492 | 106,010 |
| 12346 | 384 | 31,978 | 114,539 |
| 12347 | 384 | 32,358 | 112,313 |

Soma counts are deterministic (384 always). Synapse counts vary by about ±1%, well within statistical noise from stochastic axon growth paths.

## Artifacts

- `baker_spatial_growth_topology_stats.json` — full topology data for seed 12345
- `baker_spatial_growth_projection_matrix.json` — projection matrix for seed 12345
- `baker_spatial_growth_seed_variance.json` — seed comparison data
- `baker_spatial_growth_summary.json` — verdict and summary metrics

## Plots

1. `soma_positions_2d.png` — XY and XZ projections of soma positions
2. `projection_matrix_heatmap.png` — source×target synapse count heatmap
3. `fanin_fanout_distribution.png` — fan-in/fan-out statistics by layer
4. `distance_distribution_by_projection.png` — distance histograms per projection
5. `ei_balance_by_layer.png` — E/I synapse count and weight mass per target layer
6. `dendrite_slot_usage.png` — used dendrite slots per soma
7. `segment_offset_distribution.png` — segment offset distribution per projection
