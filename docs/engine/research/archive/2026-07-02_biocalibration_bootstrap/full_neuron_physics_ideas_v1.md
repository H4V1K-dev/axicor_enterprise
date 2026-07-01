# Full Neuron Physics Ideas v1

Status: research hypotheses, not accepted engine changes yet.
Date: 2026-07-02

## Context

The current calibration work must not treat the membrane equation as the whole neuron.
Single-neuron validation should exercise the full tick loop:

- synaptic/current input,
- ordinary leak,
- adaptive leak,
- refractory timer,
- AHP,
- threshold offset,
- homeostasis penalty and decay,
- spontaneous/DDS-like events,
- spike finalization and output emission.

Component probes are useful for isolating a mechanism, but conclusions about biological behavior must come from full-neuron replay.

## Idea 1: DDS / spontaneous event as a discharge, not self-excitation

Earlier thought: a rare DDS event could inject charge into the same neuron and sometimes push an almost charged membrane over threshold.

Updated hypothesis: a DDS/spontaneous event should behave more like a full internal discharge.
Instead of adding charge to the membrane, it should subtract energy from the current membrane state as if a real spike happened.

Expected effect:

- the neuron can produce rare spontaneous output events;
- the event also opens a negative recovery phase instead of being a free output flag;
- membrane voltage drops below rest after the event;
- the cell temporarily becomes harder to excite because it must climb back to rest and then to threshold;
- repeated spontaneous events can bury the neuron into a deeper recovery state if the timing is unlucky;
- spontaneous activity becomes stateful and history-dependent instead of a detached random emitter.

Research question:

Should this event reuse the normal spike finalization path exactly, or should it have a separate amplitude/timer policy?

Candidate policies:

- `DDS_AS_SPIKE`: same AHP, refractory, threshold penalty, output spike.
- `DDS_AS_SUBSPIKE`: weaker AHP/refractory, may or may not emit output.
- `DDS_AS_RECOVERY_KICK`: no output spike, only voltage discharge/noise.

## Idea 2: spike inertia / penalty-driven negative overshoot

At a normal spike, the neuron should not only reset. The accumulated adaptation penalty can push the post-spike voltage below the normal AHP floor.

Current simple shape:

```text
after_spike_voltage = rest_potential - ahp_amplitude
```

Candidate shape:

```text
penalty_inertia = f(threshold_offset, homeostasis_penalty, inertia_params)
after_spike_voltage = rest_potential - ahp_amplitude - penalty_inertia
```

Expected effect:

- every spike can have a unique negative trough;
- stronger recent activity creates deeper post-spike recovery;
- the threshold-offset "bank" affects timing through two channels:
  - higher effective threshold;
  - deeper voltage recovery after spike;
- the neuron spends time climbing back to the mathematical floor/rest area before ordinary integration dominates;
- this can reduce runaway behavior without simply clamping output;
- spike trains gain more biological-looking ISI adaptation and post-spike shape diversity.

Important constraint:

The inertia term must be bounded. It should not allow permanent burial, integer overflow, or unstable negative runaway.

Possible formula families:

```text
linear:
penalty_inertia = min(max_inertia, threshold_offset >> inertia_shift)

saturating:
penalty_inertia = max_inertia * threshold_offset / (threshold_offset + k)

piecewise:
penalty_inertia = 0 below small threshold_offset,
                  linear in mid range,
                  saturated at max_inertia
```

The first engine candidate should probably be the linear bounded form because it is deterministic, cheap on CPU/CUDA, and easy to reason about.

## What These Ideas Would Give

Together, DDS discharge and penalty-driven spike inertia would make the neuron less like a threshold counter and more like a stateful excitable cell:

- rare spontaneous events are no longer free spikes;
- spike timing depends on recent voltage history;
- repeated activity leaves a real recovery footprint;
- constant stimulation can produce habituation without pure hard clamps;
- negative peaks carry information about recent activity intensity;
- borderline inputs can sometimes trigger and sometimes fail depending on the current membrane state.

## Validation Plan

Do not start by changing production code.

First, test these ideas in a standalone replay/probe:

1. Reproduce the current full-neuron behavior for specimen `314900022`.
2. Replay the old `EPHYS_PROBE_01` habituation scenario.
3. Add experimental modes one by one:
   - baseline current engine,
   - DDS discharge only,
   - spike inertia only,
   - DDS discharge + spike inertia.
4. Compare:
   - spike count,
   - rheobase / f-I,
   - first spike latency,
   - first/last ISI,
   - ISI growth ratio,
   - post-spike trough depth,
   - trough diversity,
   - threshold_offset max/mean,
   - recovery time to rest,
   - runaway/no-response boundary.

Only if these probes improve behavior should the formulas be promoted into a formal physics spec change.

## Current Decision

These ideas are recorded as high-priority physics hypotheses.
They should be evaluated after the two currently running experiments finish, before doing large parameter brute force.
