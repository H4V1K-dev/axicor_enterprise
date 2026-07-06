# AxiEngine MVP Night Phase: Production Freeze Notes

**Date**: 2026-07-06  
**Status**: `PROPOSED FREEZE`  
**Related Research**:  
- [v1.6 (Smoke Lifecycle)](file:///home/alex/AI_Home/workflow/docs/engine/research/archive/2026-07-06_night_phase_mvp_consolidation_v1_6/reports/report_v1_6.md)  
- [v1.6b (Eviction Stress)](file:///home/alex/AI_Home/workflow/docs/engine/research/archive/2026-07-06_night_phase_mvp_eviction_stress_v1_6b/reports/report_v1_6b.md)  
- [v1.6c (Age+Trace Micro-Gate)](file:///home/alex/AI_Home/workflow/docs/engine/research/archive/2026-07-06_night_phase_age_trace_eviction_v1_6c/reports/report_v1_6c.md)

---

## 1. Final Consolidated MVP Night Policy

The MVP Night Phase acts as a homeostatic stabilizer, balancing network growth and pruning. The verified state machine executes sequentially at the end of each Day Interval:

```
                  +--------------------------+
                  |  Day Active Synapses     |
                  +-------------+------------+
                                |
                   Hebbian learning & decay
                                |
                                v
                  +-------------+------------+
                  | Pruning Safety Gates     |
                  | - Target active > 5      |
                  | - Proj active > 2        |
                  +-------------+------------+
                                |  Weak & Grace done
                                v
                  +-------------+------------+
                  |  Dormant Synapse Bank    |
                  +-------------+------------+
                                |
                  Double-Bounded Eviction:
                  - age > 2 & trace == 0
                  - target cap > 3
                  - global cap > 200
                                |
                                v
                  +-------------+------------+
                  |  Dead Synapse (Pruned)   |
                  +--------------------------+
```

1. **Trace Decay & Age Increment**: 
   - Age is incremented for all dormant synapses.
   - Long trace ($k_{long}=7$) and short trace ($k_{short}=2$) decay.
2. **Pruning (Active $\rightarrow$ Dormant)**:
   - Active synapses with weights below threshold (e.g. $500 \times 2^{16}$) and low coactivity are pruned.
   - **Safety Gates**: Active synapse counts per target soma cannot drop below `min_target_active_count` (5) and target projection counts cannot drop below `min_projection_active_count` (2).
3. **Double-Bounded Eviction (Dormant $\rightarrow$ Dead)**:
   - **Age-out branch**: Evict if `dormant_age > MAX_DORMANT_AGE` AND `long_trace == 0`.
   - **Target cap branch**: Cap at `MAX_DORMANT_PER_TARGET` (3) per target, evicting lowest long-trace / oldest.
   - **Global cap branch**: Cap at `MAX_DORMANT_TOTAL` (200), evicting lowest long-trace / oldest.
4. **Homeostatic Sprouting**:
   - Targets with spike rates below layer-specific targets are sprouted with new active synapses (initially with `age_or_grace = 3` cycles protection).
   - Sprouting uses a stochastic distance/density probability and enforces E/I ratio limits.

---

## 2. Production State Planes & Counters Needed

To implement this policy in production, the following data planes and counters must be introduced in the engine runtime (`compute-api` / `compute-cpu` / `compute-cuda`):

### 2.1. Synapse State Plane Additions
To handle pruning, decay, and grace periods, the flat synapse structure needs additional fields:
```rust
struct FlatSynapse {
    // Existing fields: source, target, weight, fatigue, etc.
    // ...
    pub short_trace: u16,      // 16-bit decay trace for coactivity
    pub long_trace: u16,       // 16-bit long-term retention trace
    pub age_or_grace: u8,      // Cycles remaining for sprout grace or age
    pub origin_kind: u8,       // 0 = Initial, 1 = Sprouted
}
```

### 2.2. Shard-Level Dormant Bank Buffer
A flat pre-allocated array at the shard level is needed to hold dormant synapses.
* **Allocation size**: `max_dormant_total` elements (e.g. 500 in standard config).
* **Element structure**:
```rust
struct DormantSynapse {
    pub source_soma_id: u32,
    pub target_soma_id: u32,
    pub flat_segment_idx: u32,
    pub weight: i32,
    pub long_trace: u16,
    pub short_trace: u16,
    pub dormant_age: u8,
}
```

### 2.3. Soma-Level Activity & Target Counters
* **Somatic Spike Register**: A rolling average or simple counter of spikes registered during the day interval for each soma.
* **Active Synapse Headroom Counters**: Track the current number of active synapses per target soma and per projection class to dynamically evaluate safety gates during pruning and sprouting.
* **Dormant Synapse Target Index**: Keep count of how many dormant connections point to each target soma to enforce target caps.

---

## 3. Research-Only Boundaries (Out of Scope for Production)

The following items are designated as research-only and should **NOT** be built into the production codebase:

1. **Pair-History and Oracle Labels**: Complex history tracking or semantic "original connection" tags are replaced by simple trace/age indicators.
2. **Soma-level Spiking Micro-Replay**: Simulating detailed somatic replay schedules or night spikes is omitted; passive decay and sprouting are sufficient.
3. **Axonal Branch Pathfinding during Sprouting**: Sprouting does not trigger full multifield axon growth simulations at runtime. Instead, the engine selects new synapses from a pre-calculated cache of compatible target zones generated at topology creation time.
4. **Fine-grained Biological Timescales**: Calibrating cycles to exact clock hours is simplified to deterministic tick-based intervals (`night_interval_ticks`).
