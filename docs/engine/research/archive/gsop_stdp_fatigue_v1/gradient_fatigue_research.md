# Gradient Synaptic Fatigue (Short-Term Plasticity)

## 1. Binary Refractory Limitations

In the legacy physics implementation, synaptic refractoriness (`dendrite_timer`) acted as a rigid binary toggle:
- If `timer == 0`, a spike transmitted 100% of the synaptic weight.
- If `timer > 0`, the spike was completely blocked (0% weight).

This approach caused abrupt, step-like energy transmission jumps and prevented synapses from properly responding to high-frequency spike bursts. This created the requirement for a smoother mechanism that accounts for synaptic activity intensity.

## 2. Core Concept: Gradient Synaptic Fatigue (Leaky Integrator)

To achieve smooth (gradient) transmission, the binary timer is replaced with a **Fatigue Integrator (Leaky Integrator)**.

Each synapse maintains a state variable `fatigue` (ranging from 0 to 255) representing resource depletion.

### Operational Mechanics:

1. **Recovery Phase (Tick):**
   Every simulation tick, fatigue continuously decays. This simulates natural resource replenishment during rest:
   `fatigue = fatigue.saturating_sub(recovery_rate)`

2. **Signal Transmission Phase (Spike):**
   When a spike traverses a synapse, the transmitted post-synaptic charge is no longer strictly equal to `weight`. It is attenuated proportionally to current fatigue:
   `let modified_weight = weight * (255 - fatigue) / 255`

   Immediately after signal transmission, the synapse incurs additional depletion:
   `fatigue = fatigue.saturating_add(spike_cost)`

### Absolute Refractoriness Handling

Under continuous spike trains, if a synapse cannot recover in time, `fatigue` rapidly reaches the ceiling of `255`. At this point, the multiplier `(255 - fatigue)` becomes zero. The synapse enters **absolute refractoriness**, ceasing energy transmission until it cools down through the recovery phase.

## 3. Conclusions & Advantages

Transitioning to a 255-step fatigue gradient provides fundamental benefits:
- A single isolated spike is transmitted at full strength (100%).
- High-frequency spike bursts decay smoothly (e.g., 100% -> 80% -> 60% -> 40%).
- Continuous high-frequency noise rapidly drives the synapse into dormancy (`fatigue = 255`), protecting target somas from hyperexcitation and runaway activity.

This mechanism implements hardware-efficient Short-Term Synaptic Depression (STD) strictly using fast integer operations, preserving peak engine throughput.
