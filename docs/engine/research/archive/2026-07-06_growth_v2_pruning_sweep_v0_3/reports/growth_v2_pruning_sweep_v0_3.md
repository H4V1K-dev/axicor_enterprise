# Growth v2 Parameter Sweep & Pruning Policy v0.3 — Report

**Date**: 2026-07-06
**Status**: COMPILE-CANDIDATE / Structural invariants pass / Functional projection caveat
**Experiment**: `2026-07-06_growth_v2_pruning_sweep_v0_3`
**Seed**: 12345 (primary comparison), verified consistency rerun

---

## 1. Executive Summary

This report evaluates the **Growth v2 Parameter Sweep & Pruning Policy (v0.3)**. The goal was to systematically explore the parameter space of the continuous multifield growth model, evaluate how pruning methods affect synapse density and target neuron input saturation (fan-in), and identify a low-pressure candidate suitable for the next AOT-to-flat runtime compile parity gate.

### Summary Metrics Panel

| Config Name | Raw Candidates | Accepted Synapses | Uniqueness Pruned | Cap Pruned (128) | Saturated Target Somas | Virtual Axon L4-Zone Reach Rate | Mean Terminal Knot Index |
|---|---|---|---|---|---|---|---|
| **1. Baseline (v0.2)** | 192,317 | 25,615 | 161,582 | 5,120 | 128 / 256 | 84.38% | 1.23 |
| **2. No Uniqueness** | 192,317 | 30,485 | 0 | 161,832 | 217 / 256 | 84.38% | 1.23 |
| **3. Softmax Cap 1 (beta=2)** | 192,317 | 25,615 | 161,582 | 5,120 | 128 / 256 | 84.38% | 1.23 |
| **4. Softmax Cap 2 (beta=2)** | 192,317 | 28,438 | 132,835 | 31,044 | 200 / 256 | 84.38% | 1.23 |
| **5. Softmax Cap 2 (beta=0.5)** | 192,317 | 28,438 | 132,835 | 31,044 | 200 / 256 | 84.38% | 1.23 |
| **6. Softmax Cap 3 (beta=5)** | 192,317 | 29,316 | 106,712 | 56,289 | 206 / 256 | 84.38% | 1.23 |
| **7. Low Branching (max 1)** | 131,277 | 24,977 | 101,860 | 4,440 | 113 / 256 | 75.78% | 1.22 |
| **8. High Branching (max 5)** | 271,119 | 26,013 | 239,619 | 5,487 | 132 / 256 | 83.59% | 1.31 |
| **9. Low Branch Length (max 1)** | 137,714 | 24,905 | 108,376 | 4,433 | 114 / 256 | 76.56% | 0.98 |
| **10. High Fasciculation (w=0.9)** | 192,387 | 25,598 | 161,581 | 5,208 | 125 / 256 | 80.47% | 1.23 |
| **11. No Fasciculation (w=0)** | 190,619 | 25,478 | 160,122 | 5,019 | 121 / 256 | 82.81% | 1.23 |
| **12. High Repulsion (R=1.8)** | 201,729 | 25,284 | 171,781 | 4,664 | 125 / 256 | 72.66% | 1.33 |
| **13. Low Repulsion (R=0.6)** | 182,521 | 25,541 | 151,929 | 5,051 | 125 / 256 | 87.50% | 1.20 |
| **14. Tight Dendrite (1.0 um)** | 443 | 323 | 120 | 0 | 0 / 256 | 84.38% | 1.23 |
| **15. Large Dendrite (2.5 um)** | 4,377 | 1,298 | 3,079 | 0 | 0 / 256 | 84.38% | 1.23 |
| **16. Compile Candidate** | **1,088** | **876** | **212** | **0** | **0 / 256** | **80.47%** | **1.20** |

---

## 2. In-Depth Metrics & Invariant Analysis

- **Hard Invariant Compliance**: All 16 configurations achieved **exactly 0 out-of-bounds, 0 self-intersection, 0 soma-core violation, and 0 whitelist/radius violations** in their final connectomes. This confirms the baseline safety mechanics are highly stable.
- **The Core Cause of Candidate Explosion**: Configurations 1-13 used the default dendrite capture radius (10.0 to 12.0 um). This extremely wide radius spans almost the entire local volume of the shard, causing axons to make contact with dozens of somas and inflating raw candidates to **~192,000**.
- **Taming the Saturation via Biological Parameters**: Overriding the dendrite capture radius to realistic biological dimensions (1.0 um, 1.5 um, or 2.5 um) in Configs 14-16 immediately resolved this. In the selected low-pressure candidate (Config 16), the dendrite radius is set to **1.5 um**, reducing raw candidates from 192,317 to **1,088** (a **-99.43%** reduction). Saturated target somas dropped from 128 to **0**, preventing any capping losses.
- **Functional Projection Caveat**: Config 16 is not yet a final functional replay topology. It preserves the key `VirtualInput -> L4_spiny` path with 320 accepted synapses and keeps `L4_spiny -> L23_aspiny` with 251 accepted synapses, but `L4_spiny -> L5_spiny` is absent at the strict 1.5 um capture radius. The reported 80.47% metric is an axon reach metric, not a direct synapse projection success metric.

---

## 3. Biology-Aligned Design Questions (Audit Responses)

### Q1: Какая конфигурация выбрана и почему?
**Low-Pressure Compile Candidate (Config 16)**. Parameters:
- `one_per_source_target = false`
- `softmax_cap_per_pair = Some((2, 2.0))` (allowing up to 2 synapses per source-target pair using distance-softmax selection)
- `max_branches = 2` (max terminal branches)
- `max_branch_len = 2` (max branch length)
- `w_fascicle = 0.5`
- `r_repulsion = 1.0`
- `dendrite_radius_um = 1.5`

**Reasoning**: Config 16 achieves a biologically sparse connectome (876 synapses total) with 0 target neuron input saturation. It allows up to 2 synapses per source-target pair while reducing raw candidate pressure to a minimum (1,088 candidates). This makes it a good candidate for compile-parity testing, but not yet a final functional topology candidate because one expected whitelisted projection (`L4_spiny -> L5_spiny`) disappears under the strict capture radius.

### Q2: Какие параметры сильнее всего влияют на raw candidate explosion?
**Dendrite Capture Radius (`dendrite_radius_um`)** is the primary driver. In the 3D volume, candidate contact volume scales cubically with the search radius. Reducing the radius from 10-12 um to 1.5 um drops raw candidate volume by **170x**. Secondarily, **high terminal branching factors** (e.g. Config 8 with up to 5 branches of length 4) increase axon segment volume and raw candidates by **41%** (from 192,317 to 271,119).

### Q3: Что лучше работает: pruning by uniqueness, soft cap, one-per-source-target, fasciculation tuning или repulsion?
- **Touch-based culling (Dendrite Radius Limit)** is the most effective proactive way to prevent raw candidate explosion.
- **Uniqueness Pruning (`one_per_source_target` or `softmax_cap_per_pair`)** is necessary to prevent duplicate-synapse waste. In Config 2 (No Uniqueness), the connectome contains **22,585 duplicate synapses** (74% of the accepted set), which consumes slots without adding projection diversity.
- **`softmax_cap_per_pair`** is superior to simple `one_per_source_target` because it allows multi-synapse links (e.g. 2 synapses per pair) based on physical proximity, which is biologically more authentic while still placing a strict bound on input slots.

### Q4: Есть ли tradeoff между красивой morphology и runtime-friendly flat compile?
No structural mapping blocker was found. The flat segment prefix-sum index mapping verification checks out for this test: **100% of the generated synapses across all configurations successfully mapped to flat `(axon_id, flat_segment_idx, target_id, dendrite_idx)` tuples**.
This confirms that we do not need to restrict morphology merely to create flat tuples. It does not yet prove tick-level runtime parity for spike propagation over flattened branch segments; that remains the explicit purpose of the next AOT-to-flat runtime parity gate.

### Q5: Как branch morphology предлагается компилировать дальше?
We propose **flattening branch segments into one segment namespace** using a prefix-sum offset map.
- The axon is represented as a contiguous array of segments of length $L_{total} = \sum L_{branch}$.
- A segment offset $K$ in branch $b$ maps to `flat_segment_idx = (sum of lengths of branches < b) + K - 1`.
- A synapse formed at that segment can refer to the flat segment index directly.
This allows the GPU runtime to process axons as flat, continuous segments, while preserving the branched spatial contacts formed during AOT growth.

### Q6: Готова ли выбранная конфигурация к следующему gate: AOT-to-Flat Runtime Compile Parity?
**Yes, as a compile-parity candidate; no, not as a final functional topology candidate.** The selected configuration is deterministic, passes hard structural invariants, and validates flat-tuple mapping. Its sparse density and missing `L4_spiny -> L5_spiny` projection mean it still needs parity and functional replay before production migration.

The next step is the **AOT-to-Flat Runtime Compile Parity Gate**, which will verify on a dense shard with 10-15% stimulated somas that the compiled flat runtime matches the AOT oracle predictions with zero discrepancies.
