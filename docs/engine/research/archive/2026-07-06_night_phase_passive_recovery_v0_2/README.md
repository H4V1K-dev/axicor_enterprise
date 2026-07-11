# Night Phase Passive Recovery (v0.2)

Status: finished
Started: 2026-07-06
Completed: 2026-07-06

## Question
Does passive homeostatic recovery (relaxing voltages, resetting refractory timers, clearing dendritic fatigue, and decaying threshold offsets back to rest) preserve the co-activation-induced matched-bias without upping silence or runaway rates under a day/night cycle on the Growth v2 C17 topology winner (`Radius_9_Cap_96_Pair_2_ProjAware`), and is a sign-preserving light weight decay (0.1%) dynamically stable?

## Expectation
- **Selectivity**: Passive recovery preserves matched-bias (retention ratio = 1.0000).
- **Stability**: Light weight decay (0.1%) doesn't cause Dale or sign violations, and keeps matched-bias retention high.
- **Excitability**: Resetting fatigue and threshold offsets recovers layer excitability, reducing Day 2 silence ticks compared to the no-night baseline.

## Inputs
- Growth v2 C17 winner configuration: `Radius_9_Cap_96_Pair_2_ProjAware`.
- Layer profiles: VISl4, VISp5, VISp23 from `Axicor_Neuron-Lib/modernized/`.

## Method
1. Construct the Growth v2 C17 winner topology.
2. Run Day 1 Learning (10,000 ticks) with learning active.
3. Apply 3 night policies:
   - `no_night_control` (no reset, no decay).
   - `passive_recovery_only` (reset membrane, threshold, fatigue; no decay).
   - `passive_recovery_plus_light_weight_decay` (reset membrane, threshold, fatigue; 0.1% decay).
4. Run Day 2 Replay (10,000 ticks) without learning and record CV, LV, firing rates, silence/runaway ticks, and matched-bias.

## Commands
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test night_phase_passive_recovery_v0_2 -- --nocapture
```

## Outputs
- Scientific report: `night_phase_passive_recovery_v0_2.md`

## Result
- **Excitability**: Skipping night (`no_night_control`) results in **2,623 silence ticks** on Day 2 due to carried-over threshold offsets. Passive recovery recovers excitability, dropping silence ticks to **2,036** (22.4% recovery).
- **Retention**: Passive recovery preserves matched-bias perfectly (retention ratio = **1.0000**).
- **Weight Decay**: 0.1% sign-preserving decay is stable, produces **0 Dale/sign violations**, and maintains matched-bias retention at **0.9990** (exactly matching the scale reduction).

## Interpretation
Passive recovery is completely safe, does not erode learned matched-bias, and is highly beneficial for network health. By relaxing homeostatic thresholds, it restores next-day excitability and prevents high layer silence. A light 0.1% sign-preserving decay is dynamically stable and does not cause sign flips.

## Next Step
Implement and test `Night Phase Weight Maintenance / Prune-Compact v0.3` to verify weak synapse pruning and dendritic target compaction.
