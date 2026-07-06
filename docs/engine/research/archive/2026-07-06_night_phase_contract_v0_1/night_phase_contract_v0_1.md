# Night Phase Contract & MVP Extraction (v0.1)

**Date**: 2026-07-06  
**Status**: DESIGN APPROVED  
**Workspace**: AxiEngine  

---

## 1. Short MVP Audit Summary

The legacy night phase implementation in `axicor-master` consists of two distinct components:
1. **Runtime Node (`axicor-node/src/node/shard_thread.rs`)**: Runs the hot simulation tick loop during the day. When the night tick interval is reached, it triggers the night phase. It copies the network state (soma voltages, flags, thresholds, timers, weights, and targets) from device memory (VRAM) to host memory, executes a sorting and pruning GPU/CPU kernel, sends data over Unix/TCP sockets to the Baker daemon, and copies the updated state back to device memory.
2. **Baker Daemon (`axicor-baker/src/bin/daemon.rs` & `src/bake/sprouting.rs`)**: Listens on Unix sockets (or TCP on Windows). Upon receiving a bake request, it uses memory-mapped files (SHM) to read and write targets and weights. It grows axons by updating their tips/paths, builds a local spatial grid, performs synaptogenesis (sprouting) by searching for nearby axons within a radius, and routes inter-shard axon handovers.

### Limitations Identified in MVP:
- **Momentary Spike Triggers**: Sprouting was triggered based on the momentary spiking flag at the final day tick, rather than accumulated day activity, making connections highly sensitive to single noise spikes.
- **IPC Overhead**: The daemon/IPC structure via SHM and Unix sockets introduced significant system configuration complexity and overhead, which is undesirable for early-stage test-harness research validation.
- **Flat-Runtime Limits**: Sprouting was performed directly in the baker with complex reconstruction of paths, which is decoupled from the runtime's flat axon streams.

---

## 2. Keep / Reject / Postpone Matrix

| Aspect / Mechanic | Decision | Rationale |
|---|---|---|
| **Strict Day/Night Separation** | **KEEP** | Day ticks must be kept highly optimized; structural pruning/sprouting should only occur offline between batches to prevent runtime interference. |
| **Prune/Compact before Sprouting** | **KEEP** | Targets must be pruned and compacted to the front of the array so that new sprouting can fill empty slots from left to right. |
| **Dense Dendrite Target Array** | **KEEP** | The target array must remain dense (gapless). A zero target represents the end of active synapses, ensuring GPU/CPU compute threads do not waste branches. |
| **Dale's Law / Sign Preservation** | **KEEP** | Excitation and inhibition signs must be preserved strictly based on source neuron types. |
| **Mass-Domain Weights** | **KEEP** | Maintains high precision without floating-point overhead (`<< 16` bit shift). |
| **Survival Capital for New Synapses** | **KEEP** | Newly sprouted synapses must receive initial weight protection to avoid immediate pruning in the next cycle. |
| **AOT/Baker-side Sprouting Geometry** | **KEEP** | Axonal paths, segment lengths, and branching nodes must be read from the AOT/baker geometry. Sprouting cannot be reconstructed from flat runtime arrays alone. |
| **Duplicate / Per-Pair Control** | **KEEP, UPDATED** | The old one-connection uniqueness rule must become a configurable per-pair cap. Current C17 allows up to 2 source-target contacts, so night must enforce the active cap policy rather than hard-code one synapse per pair. |
| **Spiking Flag as Sprouting Trigger** | **REJECT** | Momentary day-end spikes are too noisy. We must use accumulated activity logs or average firing rates. |
| **IPC Socket Daemon/SHM Contract** | **POSTPONE** | Avoid IPC setup in early research validation. The test-harness should run the night phase in-process using direct Rust memory calls. |
| **Cross-Shard Handover Routing** | **POSTPONE** | Handover queueing and RCU routing should be deferred. Focus initial testing on single-shard local networks. |
| **Research Matched/Unmatched Labels** | **REJECT** | The night phase must never read or write research-specific `matched` or `unmatched` labels. Plasticity must be fully self-organizing. |

---

## 3. Night State-Plane Contract

To ensure strict safety boundaries, we define the allowed data access planes for the night phase.

### Read-Only Planes
- **Soma Position Map**: Coordinate space of all target somas.
- **Soma Type Metadata**: E/I classification and type-affinity parameters.
- **Axon Spatial Trajectories**: Physical segment locations and branching nodes (AOT/baker-side geometry).
- **Day Activity Accumulator**: Average firing rates or accumulated spike counts for each neuron.

### Mutable Planes
- **Dendrite Targets**: Mutated during prune (zeroing targets) and sprout (writing target axon + segment IDs).
- **Dendrite Weights**: Reset to 0 when pruned; initialized with survival capital when sprouted.
- **Dendrite Timers / Synaptic Fatigue**: Decayed or reset during passive recovery. This is a legitimate fast-state recovery plane, not a forbidden probe.
- **Soma Refractory Timers**: May be decayed/reset during passive recovery when the night window represents elapsed biological time.
- **Homeostatic Threshold Offsets**: Normalization or decay of thresholds back to rest state.
- **Soma Voltage**: Relaxation of membrane potential back to rest.

### Forbidden Inputs
- **Matched / Unmatched Labels**: The night phase **must not** read or write any labels indicating whether a pathway is a "matched" stimulus group or "unmatched" control.
- **Day Tick Physics Probes**: The night phase must not depend on instantaneous, tick-local probes such as `i_in`, transient membrane update intermediates, debug-only spike-cause labels, or research-only annotations. Persistent fast-state planes such as fatigue/timers are allowed only through the explicit mutable-plane contract above.

---

## 4. Proposed Operation Order

Every night phase iteration must run operations in this exact order:

1. **Passive Recovery / Reset**: Soma voltages, refractory timers, dendritic fatigue, and threshold offsets decay homeostatically towards their rest states.
2. **Synaptic Decay / Weight Maintenance**: Passive decay is applied to all synaptic weights.
3. **Prune & Compact**: Synapses with weights below the pruning threshold are deleted, and active targets/weights are shifted left to ensure the arrays are gapless.
4. **Sprouting**: New synapses are sprouted to fill empty dendritic slots, using local AOT-compiled geometry and type affinities.
5. **Hard Invariant Validation**: Verify all invariants are satisfied before resuming day ticks.

---

## 5. Hard Invariants

The night phase must preserve these strict invariants:
- **Duplicate / Per-Pair Cap Rule**: A target soma cannot exceed the configured source-target contact cap. For the current C17 winner this cap is 2, not 1. Exact duplicate target slots must still be avoided.
- **Dense Target Rule**: There must be no gaps (empty/zero slots) in the targets array before active synapses. The first zero target index represents the end of the connection list.
- **Dale's Law**: The sign of a synapse's weight must match the E/I type of its source neuron.
- **AOT Geometry Dependency**: Sprouting must use physical axonal path coordinates from the AOT compiler; it cannot be computed from flat runtime target arrays alone.
- **Label Independence**: Sprouting and pruning must operate without access to matched/unmatched stimulus categories.

---

## 6. Minimal Next Phase Task: Night Phase Passive Recovery v0.2

The direct next executable step is **Night Phase Passive Recovery (v0.2)**. 

### Objective:
Implement an in-process day/night simulation loop on the test-harness that runs passive recovery and weight decay on the Growth v2 C17 topology winner.

### Scope:
- Disable sprouting (sprouting = 0) to isolate recovery and decay dynamics.
- Implement homeostatic threshold offset decay and soma voltage relaxation.
- Implement homeostatic synaptic weight decay.
- Verify that repeated day/night cycles maintain learning stability and matched-bias without topology collapse or runaway dynamics.
