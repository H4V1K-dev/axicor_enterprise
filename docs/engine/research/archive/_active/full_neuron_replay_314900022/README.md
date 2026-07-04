# Full Neuron Replay 314900022

Status: running
Started: 2026-07-04
Completed: N/A (Phase 4 Rheobase Calibration finished)

Slug: `full_neuron_replay_314900022`

## Question

Можно ли изолированно подобрать параметры пассивной возбудимости (`leak_shift` и `rest_potential`) для specimen `314900022` так, чтобы полностью устранить ложную гипервозбудимость на малых токах (30–40 pA), не разрушив высокотоковый отклик на 190 pA (~36 спайков) и сохраняя монотонность f-I кривой?

## Expectation

- 30 pA stimulus: 0 spikes (1000 ms window)
- 40 pA stimulus: 0 spikes (1000 ms window)
- 50 pA stimulus: около биологического начала разряда (3-4 спайка)
- 190 pA stimulus: высокотоковый разряд в диапазоне 30–42 спайка (биологическая норма ~36)
- Monotonic f-I curve
- Biologically defensible `rest_potential` (±3000 uV от baseline -70 mV)

## Inputs

- Specimen parameters profile: `Axicor_Neuron-Lib/modernized/L4_spiny_VISl4_4.toml`
- Profile Baseline: `rest_potential = -73443 uV`, `threshold = -45656 uV`, `leak_shift = 8`, `homeostasis_penalty = 1940`, `homeostasis_decay = 2`, `ahp_amplitude = 5000 uV`, `current_scale = 35.0`
- Stimulus duration: 1000 ms (ticks 1000..2000 in 3000-tick run)
- Allen Cell Types Reference f-I curve: `[-10, 30, 40, 50, 70, 90, 110, 130, 150, 190] pA` -> `[0.0, 0.0, 0.0, 3.5, 11.0, 20.0, 22.0, 26.0, 29.0, 36.0]` spikes

## Method

1. **Rust Test Harness Execution**:
   - Integrated `run_full_neuron_replay_phase4_experiments` into `crates/test-harness/tests/full_neuron_replay.rs`.
   - Conducted grid sweep across `leak_shift` ∈ [1, 2, 3, 4, 5, 6, 7, 8, 10] and `rest_potential` ∈ [-70000, -71000, -72000, -73000, -73443] uV.
   - Conducted control sweep over `current_scale` ∈ [15.0, 20.0, 25.0, 30.0, 35.0, 40.0].
   - Conducted adaptive leak subphase grid search across `adaptive_leak_gain`, `adaptive_leak_min_shift`, `adaptive_mode`.
2. **Python Metrics Audit & Plot Generation**:
   - `docs/engine/research/archive/_active/full_neuron_replay_314900022/scripts/rheobase_leak_rest_calibration.py`.
   - Evaluated Acceptance Gate criteria and generated heatmaps, Pareto plot, f-I curves, and trace comparison.

## Commands

```bash
# Rust test harness execution
cargo test -p test-harness --features "mvp-cpu-replay,baker-probe" --test full_neuron_replay -- run_full_neuron_replay_phase4_experiments --nocapture
cargo clippy -p test-harness --features "mvp-cpu-replay,baker-probe" --test full_neuron_replay -- -D warnings
cargo fmt --check

# Python analysis & report generation
.venv/bin/python3 docs/engine/research/archive/_active/full_neuron_replay_314900022/scripts/rheobase_leak_rest_calibration.py
```

## Outputs

- Phase 4 Report: [reports/rheobase_leak_rest_calibration_v1.md](reports/rheobase_leak_rest_calibration_v1.md)
- Images:
  - Heatmap 40 pA: [images/heatmap_leak_rest_40pa.png](images/heatmap_leak_rest_40pa.png)
  - Heatmap 190 pA: [images/heatmap_leak_rest_190pa.png](images/heatmap_leak_rest_190pa.png)
  - Pareto Plot: [images/pareto_false_low_vs_high_error.png](images/pareto_false_low_vs_high_error.png)
  - f-I Curves: [images/fi_curves_best_candidates.png](images/fi_curves_best_candidates.png)
  - Trace Comparison: [images/trace_comparison_best_vs_baseline.png](images/trace_comparison_best_vs_baseline.png)
- Repository Artifacts:
  - `artifacts/full_neuron_replay_314900022_phase4_static_sweep.json`
  - `artifacts/full_neuron_replay_314900022_phase4_control_scale_sweep.json`
  - `artifacts/full_neuron_replay_314900022_phase4_adaptive_sweep.json`
  - `artifacts/full_neuron_replay_314900022_phase4_trace_baseline_190.csv`
  - `artifacts/full_neuron_replay_314900022_phase4_trace_candidate_190.csv`

## Result

- Baseline (`leak_shift = 8`, `rest = -73443 uV`): `spikes_30 = 16`, `spikes_40 = 19`, `spikes_190 = 40`, Allen f-I RMSE = 12.89. **FAIL**.
- **Winner Candidate (`leak_shift = 4`, `rest = -70000 uV`)**:
  - `spikes_30` = 0 (target 0)
  - `spikes_40` = 0 (target 0)
  - `spikes_50` = 3 (bio target 3.5)
  - `spikes_190` = 35 (bio target 36)
  - Allen f-I RMSE = **1.89** (dropped from 12.89 to 1.89)
  - Monotonicity: True
  - Acceptance Gate Status: **PASS**

## Interpretation

Увеличение силы пассивной утечки через параметр `leak_shift = 4` (повышение константы сброса потенциала в `update_glif_voltage`) полностью устраняет нефизичную гипервозбудимость на малых токах 30–40 pA. При этом мембранный потенциал успевает накапливать достаточный заряд при сильном внешнем токе 190 pA, сохраняя высокотоковый разряд (35 спайков) и SFA.

## Next Step

Обновить параметры профиля `L4_spiny_VISl4_4.toml` (установить `leak_shift = 4`) и применить результат в последующих калибровочных тестах.
