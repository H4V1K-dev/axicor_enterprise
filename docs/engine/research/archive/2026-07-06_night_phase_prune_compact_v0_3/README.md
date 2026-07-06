# Night Phase Prune & Compact (v0.3)

Status: finished
Started: 2026-07-06
Completed: 2026-07-06

## Question
Can we prune weak synapses during the night phase and compact the target dendritic arrays (reassigning dense indices consecutive from 0 to $k-1$) without violating Dale's Law, dense target constraints, or duplicate per-pair limits, and how does this affect the learned matched-bias and Day 2 replay dynamics on the Growth v2 C17 topology winner (`Radius_9_Cap_96_Pair_2_ProjAware`)?

## Expectation
- **Invariants**: 0 Dale violations, 0 dense target violations, and 0 duplicate per-pair cap violations.
- **Selectivity**: Weak pruning preserves or improves matched-bias selectivity by selectively removing noise/unmatched/depressed synapses.
- **Dynamics**: Replay doesn't collapse into complete silence or trigger runaway firing.

## Inputs
- Growth v2 C17 winner configuration: `Radius_9_Cap_96_Pair_2_ProjAware`.
- Layer profiles: VISl4, VISp5, VISp23 from `Axicor_Neuron-Lib/modernized/`.

## Method
1. Construct the Growth v2 C17 winner topology.
2. Run Day 1 Learning (10,000 ticks) with co-activation stimulus.
3. Apply 4 night policies (no decay/no prune, decay/no prune, decay/weak floor prune, decay/moderate floor prune).
4. Perform compaction by sorting remaining synapses descending by absolute weight and reindexing `dendrite_idx = 0..k-1`.
5. Run Day 2 Replay (10,000 ticks) without learning and evaluate ISI, CV, LV, silence/runaway ticks, matched-bias, and retention ratios.
6. Export telemetry to `artifacts/plot_data.json` and generate matplotlib plots.

## Commands
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test night_phase_prune_compact_v0_3 -- --nocapture
uv run generate_plots.py
```

## Outputs
- Scientific report: `reports/night_phase_prune_compact_v0_3.md`
- Plotting script: `scripts/generate_plots.py`
- Analysis plots: `images/` (weight distribution, fan-in compaction, delta updates, pruned synapse map, Day 2 timeline)
- Raw simulation data: `artifacts/plot_data.json` (git-ignored)

## Result
- **Compaction**: 0 dense target violations and 0 duplicate violations across all policies.
- **Selectivity**: Moderate pruning (`floor = 1498 << 16`) pruned 1,750 synapses, increasing matched-bias from 273k to 918k (retention = 3.3542) by removing depressed unmatched connections.
- **Dynamics**: Stable replay, Day 2 silence ticks slightly increased to 2,159 under moderate pruning (better than no-night's 2,623), 0 runaway ticks.

## Interpretation
Pruning and compaction are mechanically safe and dynamically stable in the flat-tree runner. Pruning selectively removes depressed, unmatched synapses, which significantly increases matched-bias selectivity at the cost of a minor increase in Day 2 silence (which is still a major recovery over the no-night baseline). Pruning floors should be carefully calibrated or guided by activity counters to avoid over-pruning.

## Next Step
Prepare `Night Phase Activity Counters Review Package v0.4` for biological validation of activity counters and cold memory bank mechanisms.
