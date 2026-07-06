# Night Phase Passive Recovery (v0.2) Scientific Report

**Date**: 2026-07-06  
**Status**: PASS  
**Workspace**: AxiEngine Test-Harness  

---

## 1. Research Question

During active daytime learning, neural networks accumulate significant homeostatic threshold offsets, dendritic fatigue, refractory state, active propagation tails, and transient voltages. If these fast states are carried into the next day unchanged, the network can become over-clamped or poorly responsive.

In this research audit, we address the following questions:
1. Does passive homeostatic recovery (relaxing voltages, resetting refractory timers, clearing dendritic fatigue, and decaying threshold offsets back to rest) preserve the co-activation-induced matched-bias without upping silence or runaway rates?
2. Is a sign-preserving light weight decay (0.1%) stable, or does it erode matched-bias retention?
3. What is the impact of skipping the night phase completely (no_night_control)?

---

## 2. Method

We ran a day/night/day simulation protocol using the Growth v2 C17 topology winner (`Radius_9_Cap_96_Pair_2_ProjAware`), which contains **22,037 synapses** across 384 somatic neurons:
1. **Day 1 Learning (10,000 ticks)**: Learning is active (`is_learning = true`). Co-activation stimulus is applied to matched pathways, establishing a pre-night selection matched-bias (+273,784.71 weight units).
2. **Night Phase**: One of three night policies is executed:
   - `no_night_control`: Direct carryover of final Day 1 states (no reset, no decay).
   - `passive_recovery_only`: voltages relaxed to rest, threshold offsets reset to 0, refractory timers reset to 0, dendritic fatigue reset to 0. Weights and topology remain unchanged.
   - `passive_recovery_plus_light_weight_decay`: Passive recovery as above, plus a 0.1% sign-preserving synaptic weight decay.
3. **Day 2 Replay (10,000 ticks)**: Learning is disabled (`is_learning = false`). The network is simulated starting from the post-night state.

For each policy, we recorded synapse counts, expected projection preservation, matched/unmatched weight deltas, Dale/sign violations, silence ticks, and runaway ticks.

---

## 3. Results

The simulation metrics for all three policies are summarized in the table below:

| Policy | Synapses (Pre/Post) | Pre-Night Matched Bias | Post-Night Matched Bias | Retention Ratio | Dale Violations | Silence Ticks | Runaway Ticks |
|---|---|---|---|---|---|---|---|
| **no_night_control** | 22,037 / 22,037 | 273,784.71 | 273,784.71 | 1.0000 | 0 | 2,623 | 0 |
| **passive_recovery_only** | 22,037 / 22,037 | 273,784.71 | 273,784.71 | 1.0000 | 0 | 2,036 | 0 |
| **passive_recovery_plus_light_decay** | 22,037 / 22,037 | 273,784.71 | 273,510.95 | 0.9990 | 0 | 2,055 | 0 |

### Key Findings:
1. **Topology & Dale Preservation**: In all policies, the topology remains unchanged (22,037 synapses, expected projections fully intact) and Dale violations remain at exactly **0**.
2. **Retention Interpretation**: Day 2 runs with learning disabled, so retention primarily measures whether night preserves the learned weight structure rather than whether additional learning occurs. `passive_recovery_only` leaves weights unchanged and therefore retains matched-bias exactly.
3. **Excitability Recovery**: Skipping the night phase (`no_night_control`) results in **2,623 silence ticks** during Day 2 because high threshold offsets and other fast states accumulated during Day 1 carry over, dampening neural responsiveness. Running `passive_recovery_only` relaxes these states, dropping silence ticks to **2,036** (a 22.4% recovery in excitability).
4. **Decay Safety**: The 0.1% weight decay policy is dynamically stable. It results in a retention ratio of **0.9990** (matching the expected 0.1% scale reduction), has no sign flips, and keeps silence ticks low at **2,055**.

---

## 4. Verdict

- **Is passive night safe?**  
  **Yes, highly safe and beneficial**. Passive recovery does not degrade the co-activation matched-bias (retention = 1.0000) and dramatically improves network health by relaxing threshold offsets and reducing next-day silence ticks.
- **Is light weight decay safe?**  
  **Yes, safe**. The 0.1% sign-preserving decay preserves the sign structure perfectly (0 violations) and has a predictable, linear effect on matched-bias retention (0.9990) without causing dynamic instability or collapse.

---

## 5. Next Step Recommendation

The passive day/night recovery cycle has been successfully validated. The next planned step is **Night Phase Weight Maintenance / Prune-Compact (v0.3)**, where we will implement:
1. Milder homeostatic synaptic decay.
2. Synaptic pruning of weak connections that drop below a threshold.
3. Dendritic array compaction (moving remaining active synapses left to keep the arrays dense and gapless) before enabling sprouting in future phases.
