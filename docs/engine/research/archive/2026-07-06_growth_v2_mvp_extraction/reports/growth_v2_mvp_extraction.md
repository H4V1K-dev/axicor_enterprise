# Growth v2 MVP Source Audit & Terminal Knot Design

**Date**: 2026-07-06
**Status**: Completed Source Audit / Design Proposal
**Experiment**: `2026-07-06_growth_v2_mvp_extraction`

---

## 1. Introduction

This report documents the architectural and algorithmic audit of the legacy MVP Baker engine (`axicor-master/axicor-baker`) as a reference for the upcoming **Growth v2** design.

The goal of this audit is to:
1. Compare the features of the legacy MVP Baker with the current production baseline (Baker v1).
2. Formulate the "terminal knot" problem where growing axons form excessive local tangles near targets.
3. Propose a suite of metrics and algorithmic fixes to resolve this issue in Growth v2.

---

## 2. MVP vs. Current Baker v1 Comparison

### A. Features in Legacy MVP (Absent in Baker v1)

The legacy MVP Baker implements a continuous, biologically guided vector growth simulation:
- **`v_attract` (Attraction Force)**: Continuous chemical gravity vector pulling the axon tip toward nearby somas.
- **Cone/FOV Sensing**: The axon tip only detects somas situated within a forward-facing field-of-view cone (e.g., cos(FOV/2) cosine culling), mimicking biological growth cone constraints.
- **`type_affinity` (Type-Specific Affinity)**: Adjusts the attraction multiplier based on whether the target soma type matches the presynaptic type (e.g. homophilic vs. heterophilic attraction).
- **Steering Vector Mixture (`v_global + v_attract + v_noise`)**: Calculates the final step vector as a weighted blend of a global direction (inertia or target), chemical attraction, and stochastic noise (random jitter).
- **Target Position / Target Layer Z-Boundaries**: Axons have a designated Z target or target coordinates in the target layer and stop growing when they cross this plane.
- **Continuous Coordinates & Quantization**: Axon growth runs in continuous space (f32 in micrometers) and only quantizes (rounds-to-nearest) coordinates to voxel indices for storage, collision checking, and boundary detection.
- **`GrowthEvent` State Machine**: Tracks step-level transitions (`Advanced`, `TargetReached`, `Stagnated`, `OutOfBounds`).
- **`GhostPacket` / Inter-Shard Handover**: Axons crossing shard boundaries are serialized as ghost packets and continued in adjacent shards to maintain global network connectivity.
- **Spatial Grid Acceleration**: Uses spatial hashing (`SpatialGrid` for somas and `AxonSegmentGrid` for segments) to speed up culling from $O(N)$ to $O(1)/O(K)$ lookups.
- **Uniqueness Per Axon**: The synapse formation step (`connect_dendrites`) enforces that a target soma forms at most **one** synapse with a given presynaptic axon, preventing duplicate synapses from the same source.
- **Cell-Radius Dendrite Scan**: The legacy dendrite connector scans nearby `AxonSegmentGrid` cells using `radius_cells`, then applies whitelist, self-exclusion, uniqueness, and Dale sign rules. It does **not** perform a final exact Euclidean segment-to-soma radius check; Growth v2 should keep the current production exact radius validation after any spatial-grid prefilter.
- **Whitelist Bypass for Virtual/Ghost Axons**: Legacy axons represented with `soma_idx == usize::MAX` bypass target whitelists to ensure external sensor/ghost inputs can connect and establish zone-level connectivity.

### B. Features in Current Baker v1

Production Baker v1 implements a discrete, grid-based pathfinding simulation:
- **26-Neighbor Discrete Step**: The axon moves step-by-step to adjacent voxels in the 26-neighbor Moore neighborhood.
- **Score-Based Selection**: The next step is selected by maximizing a score function: `score = inertia + vertical_bias + jitter` (jitter is computed using wrapping FNV-1a hashes for determinism).
- **Soma Collision Avoidance**: The path strictly avoids stepping on any voxel occupied by a soma in the shard.
- **Self-Intersection Avoidance**: The path strictly avoids stepping on voxels already visited by the same axon.
- **`BoundaryReached` Stop**: The growth halts immediately when the path hits the 3D boundary of the shard.
- **Post-Growth Radius-Based Synapse Formation**: Growth and synapse formation are separated. After growth is complete, the engine finds all segment voxels situated within the target soma's `dendrite_radius_um`, groups them, sorts them by proximity, and caps them at `MAX_DENDRITES=128`.

### C. Major Architectural Comparison

| Feature | Legacy MVP Baker | Current Baker v1 |
|---|---|---|
| **Space** | Continuous `f32` (quantized to voxel indices) | Discrete voxel grid (`u32`) |
| **Steering Force** | Blended vector: `v_global + v_attract + v_noise` | Neighborhood score: `inertia + bias + jitter` |
| **Sensing** | FOV cone-tracing with chemical gradients | None (pure local bias score) |
| **Soma Collisions** | Not explicitly checked during growth | Strictly avoided (acts as obstacle) |
| **Synapse Formation** | Spatial-grid cell-radius scan; uniqueness per axon; no exact final radius check | Exact segment-to-soma radius check of finished segments + deterministic post-sort/cap |
| **Uniqueness** | Enforced (at most 1 synapse per axon) | Not enforced (multiple synapses per axon allowed) |
| **Whitelist Bypass** | Enabled for legacy external axons (`soma_idx == usize::MAX`) | Disabled by default (Virtual must follow config targets) |

### D. Main Conclusion
1. **Current Baker v1** serves as a **structurally correct, deterministic baseline** that guarantees zero self-intersections, zero out-of-bounds segments, and strict soma avoidance in a discrete grid.
2. **MVP Baker** acts as a **biological reference** showing how vector-based steering, attraction cones, and type affinity can guide axons to target layers, which forms the basis for the new **Growth v2** design.
3. **Growth v2 should be a selective merge**, not a direct port: use MVP-style continuous steering/FOV/affinity where it improves path biology, but retain production's exact radius gate and explicit whitelist policy.

---

## 3. Terminal Knot Audit Design

### A. The "Terminal Knot" Problem
In legacy continuous vector-guided growth models, when an axon reaches its target layer or target soma's vicinity, the attraction force towards the target becomes extremely dominant while the global/inertia direction decreases or undergoes frequent updates. Under strong noise or local culling, the axon tip starts circling or looping around the target coordinate. This creates a dense, coiled cluster of segment voxels near the endpoint—a **terminal knot**.

This "knotting" behavior is problematic because:
- It wastes segment capacity (axons hit step limits without exploring).
- It results in high redundant candidate density around a single soma, which is then capped anyway.
- It is biologically unrealistic (axons terminate with terminal boutons or branching, not dense circular coils).

### B. Proposed Audit Metrics for Growth v2
To detect and quantify terminal knots, we propose the following metrics:
1. **Last-$N$ Segment Tortuosity**:
   $$\text{Tortuosity}_{\text{last-}N} = \frac{\sum_{i=L-N}^{L-1} \text{step\_length}(i, i+1)}{\text{Euclidean}(\mathbf{x}_{L-N}, \mathbf{x}_L)}$$
   A high tortuosity value (e.g., $>1.5$) for the last $N$ segments (e.g. $N=5$ or $10$) indicates winding or circular tangles.
2. **Endpoint Local Segment Density**:
   The count of segment voxels belonging to the same axon that fall within a small sphere (e.g. radius $R = 2.0$ or $3.0$ voxels) centered at the endpoint $\mathbf{x}_L$.
3. **Final Angle Variance**:
   The standard deviation of angles between consecutive step vectors over the last $N$ segments. High variance indicates rapid, erratic changes in direction.
4. **Distance-to-Target Profile**:
   Evaluating the Euclidean distance to the target coordinates over the last $N$ steps. A profile that decreases to a minimum and then oscillates indicates a failure to terminate cleanly upon arrival.
5. **Stagnation Steps**:
   The number of steps spent inside the target capture zone without improving the distance to the target center.

### C. Proposed Algorithmic Fixes for Growth v2
To prevent terminal knots, we propose implementing one or more of the following mechanisms in Growth v2:
1. **Capture Radius Stop**:
   Terminate growth immediately as soon as the axon tip coordinates $\mathbf{x}$ enter a configured capture sphere:
   $$\text{Euclidean}(\mathbf{x}, \mathbf{t}) \le R_{\text{capture}}$$
2. **Target Attraction Damping**:
   Apply a damping factor to the attraction weight $w_{\text{attract}}$ as distance decreases, transitioning steering to pure inertia or a straight-line vector inside the terminal zone.
3. **Terminal Straightening / Straight Handover**:
   Force the final $M$ segments to align strictly with the direction vector at the entry point of the target zone, disabling noise ($w_{\text{noise}} = 0$) and attraction ($w_{\text{attract}} = 0$).
4. **Curvature Constraints**:
   Implement a maximum turning angle constraint between consecutive steps (e.g., $\theta \le 45^\circ$) to mathematically prevent tight loops and self-coiling.
5. **Distance-to-Target Monotonicity Check**:
   Halt growth if the distance to the target fails to improve (decrease) for $K$ consecutive steps.

---

## 4. Architectural Whitelist Note

In legacy MVP Baker, a bypass whitelist flag was implicitly enabled for Virtual and Ghost axons:
```rust
let is_virtual_or_ghost = axons[original_axon_index].soma_idx == usize::MAX;
if !is_virtual_or_ghost && !my_type.dendrite_whitelist.is_empty() && ...
```
> [!CAUTION]
> This whitelist bypass must **NOT** be ported blindly into Growth v2. In production AxiEngine setups, `VirtualInput` mapping must be strictly controlled through target whitelists (as done in the Baker Spatial Growth whitelist fix) to prevent unintended synapse leakage and feedback onto inputs. If inter-shard ghost continuation needs a bypass later, it should be an explicit configuration flag with separate invariants, not the old unconditional `soma_idx == usize::MAX` behavior.

## 5. Radius Gate Note

Legacy MVP `connect_dendrites` uses `AxonSegmentGrid::for_each_in_radius` as a cell-neighborhood prefilter. The function iterates hash-grid cells around the soma; it does not load each segment position and re-check exact Euclidean distance against `dendrite_radius_um`.

Current production topology formation is stricter: candidate synapses pass only when the segment-to-soma voxel distance satisfies `dist_sq <= radius_voxels_sq`, then candidates are sorted and capped. Growth v2 should preserve this exact final radius gate even if it reintroduces MVP-style spatial acceleration.
