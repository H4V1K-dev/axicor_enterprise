# Full Neuron Replay 314900022 v1

Status: active
Start date: 2026-07-04
Slug: `full-neuron-replay-314900022-v1`

## Purpose

Проверить, сохраняются ли улучшения, найденные в membrane/adaptive probes по specimen `314900022`, когда нейрон прогоняется не через обрезанную песочницу, а через полный production CPU tick-loop AxiEngine.

Это прямое продолжение [2026-07-02 biocalibration bootstrap](../../2026-07-02_biocalibration_bootstrap/README.md) и особенно заметки [full_neuron_physics_ideas_v1.md](../../2026-07-02_biocalibration_bootstrap/full_neuron_physics_ideas_v1.md).

## Core Question

Если probe-улучшения настоящие, full-neuron replay должен сохранить улучшение SFA/f-I и показать осмысленную форму восстановления после спайка.

Если результат развалится, проблема находится не в подборе отдельных мембранных параметров, а в полном tick-loop: входы, AHP, refractory, adaptive leak, homeostasis, heartbeat/DDS, финализация спайка и запись output-событий.

## Scope

- Использовать production `compute-cpu` path.
- Не использовать membrane sandbox как источник истины.
- Не менять CUDA в этом исследовании.
- Не переходить к cortical microcircuit, пока одиночный full-neuron replay не стал понятным.
- Экспериментальные режимы DDS / inertia допускаются только как отдельные сравниваемые варианты, не как молчаливое изменение baseline.

## Inputs

- Allen/NWB specimen: `314900022`.
- Биологические признаки из calibration pack:
  - resting potential;
  - input resistance;
  - tau;
  - rheobase;
  - f-I / firing rate;
  - adaptation / SFA-related признаки;
  - spike timing and ISI where available.
- Предыдущие артефакты:
  - `artifacts/single_neuron_314900022_balanced_best.csv`;
  - `artifacts/single_neuron_314900022_passive_first_best.csv`;
  - `artifacts/single_neuron_314900022_membrane_sandbox_model_comparison.csv`;
  - `artifacts/single_neuron_314900022_adaptive_leak_best.csv`;
  - `artifacts/ephys_probe_01_replay_summary.csv`;
  - `artifacts/ephys_probe_01_replay_trace.csv`.

## Planned Phases

### Phase 0: Production Path Audit

Цель: подготовить replay так, чтобы Phase 1 мерила полный production CPU tick-loop, а не новую скрытую песочницу.

Checklist:

1. Зафиксировать точный tick-order `compute-cpu`:
   - virtual input injection;
   - incoming spike injection;
   - axon head propagation;
   - homeostasis decay;
   - dendritic fatigue recovery;
   - dendritic charge integration;
   - refractory branch;
   - membrane candidate update;
   - GLIF spike evaluation;
   - heartbeat/DDS evaluation;
   - spike finalization;
   - GSOP pass;
   - local spike axon emission.
2. Зафиксировать причинность входов:
   - external/incoming spikes становятся видимы на segment 0 в этот же тик;
   - local spikes пишутся в axon heads в конце тика и начинают путь на следующем propagation.
3. Зафиксировать список логируемых полей:
   - `tick`;
   - `voltage_pre`;
   - `voltage_candidate`;
   - `voltage_post`;
   - `timer_before`;
   - `timer_after`;
   - `was_refractory`;
   - `threshold_offset`;
   - `effective_threshold`;
   - `i_syn`;
   - `i_ext` or explicit `no_i_ext_plane`;
   - `is_glif_spike`;
   - `is_heartbeat_spike`;
   - `final_spike`;
   - `spike_cause`;
   - `burst_count`;
   - fatigue aggregate for live dendrites.
4. Решить, как Phase 1 воспроизводит EPHYS current injection:
   - либо через named synaptic approximation на virtual axons;
   - либо через research-only replay runner с `i_ext[tick]`;
   - либо через отдельный production API proposal for external-current plane.
5. Зафиксировать heartbeat baseline policy:
   - Phase 1 baseline запускается с `heartbeat_m = 0`;
   - текущее heartbeat-during-refractory поведение записывается как audit finding;
   - варианты heartbeat gating / DDS discharge проверяются только после baseline как named research variants.
6. Подготовить структуру эксперимента:
   - `scripts/` for research scripts;
   - `images/` for committed plots;
   - generated CSV/JSON/traces under repository-level `artifacts/full_neuron_replay_314900022_*`.
7. Если понадобится изменить production-функцию для проверки гипотезы, сделать named research variant в `test-harness`, а не патчить production physics.

### Phase 0 Audit Findings & Design Decisions

- **Exact Semantic Delta from Production**:
  `full_neuron_replay` test-harness runner mirrors the exact production CPU tick-loop, plus explicit `i_ext[tick]` added to the somatic charge after dendritic integration and before GLIF membrane update:
  `i_total = i_syn + i_ext[tick]`.
- **Causality of Inputs**:
  - `incoming_spikes`/`virtual_inputs` have a 0-tick delay to target segment 0.
  - `local_spikes` are emitted at the end of the tick and propagate to targets starting on the next tick (1-tick delay).
- **Heartbeat Audit Finding**:
  In current `compute-cpu` production code, the spontaneous heartbeat evaluation is done outside the refractory check. Spontaneous spikes can fire during refractory period, resetting the refractory timer and accumulating homeostasis threshold offsets.
- **EPHYS current injection**:
  We use a custom `full_neuron_replay` runner in `test-harness` that replicates the production `compute-cpu` tick-loop, using exact production physics functions directly. This avoids modifying the production API or polluting it with test-only current inputs.
- **Logged Fields**:
  Every tick, the runner logs `tick`, `voltage_pre`, `voltage_candidate`, `voltage_post`, `timer_before`, `timer_after`, `was_refractory`, `threshold_offset`, `effective_threshold`, `i_syn`, `i_ext`, `is_glif_spike`, `is_heartbeat_spike`, `final_spike`, `burst_count` to `artifacts/full_neuron_replay_314900022_trace.csv`.

---

### Phase 1: Baseline Full-Neuron Replay

Прогнать `314900022` на текущей production CPU-физике без новых экспериментальных формул.

Measure:

- spike count;
- rheobase / f-I curve;
- first spike latency;
- first/last ISI;
- ISI growth ratio;
- CV / LV;
- voltage trace shape;
- post-spike trough depth;
- recovery time to rest;
- threshold offset max/mean;
- homeostasis penalty max/mean;
- silence/runaway boundary.

### Phase 2: EPHYS_PROBE_01 Replay

Восстановить старый контекст `EPHYS_PROBE_01` уже на полном production tick-loop.

Purpose:

- проверить, сохраняется ли sawtooth/habituation-поведение;
- понять, какие части формы создаются membrane/update логикой, а какие рождаются полным циклом спайка.

### Phase 3: Experimental Modes

Только после baseline replay добавить режимы как сравниваемые варианты:

- baseline current engine;
- DDS/spontaneous event as full discharge;
- bounded spike inertia;
- DDS discharge + bounded spike inertia.

Каждый режим должен иметь отдельный результат и не смешиваться с baseline.

### Phase 4: Decision Gate

Решить, что делать дальше:

- если production baseline уже сохраняет SFA/f-I улучшение, двигаться к population/motif tests;
- если DDS или inertia явно улучшает full replay, вынести формулу в отдельную physics-spec proposal;
- если все варианты разваливаются, остановить brute force и искать ошибку в tick-loop/единицах/масштабах.

## Verification Criteria

Подтверждает гипотезу:

- full replay сохраняет улучшение SFA/f-I относительно ранних probes;
- rheobase и f-I не уходят в физически бессмысленную область;
- восстановление после спайка имеет устойчивую и интерпретируемую форму;
- AHP/refractory/homeostasis не создают ложную тишину или runaway;
- одинаковый seed дает детерминированный результат.

Ослабляет гипотезу:

- параметры, хорошие в sandbox, ломаются в production tick-loop;
- heartbeat/DDS дает бесплатные или несогласованные output-события;
- post-spike recovery превращается в clamp без биологически осмысленной динамики;
- небольшие изменения входа вызывают резкий переход в silence/runaway.

## Planned Outputs

- production CPU replay runner path and exact command;
- raw CSV/JSON metrics;
- sampled voltage/state traces;
- summary report in this README or `reports/`;
- explicit decision: promote, defer, or reject DDS/inertia hypotheses.

## Notes

Это исследование не является microcircuit/V1 validation. Оно должно закрыть одиночный full-neuron контур. Сетевые GSOP/microcircuit эксперименты начинаются только после того, как этот слой понятен.

---

## Phase 1 Results & Interpretation

Мы успешно выполнили Phase 1 (Baseline Full-Neuron Replay) на Rust-раннере, использующем производственную физику.

### 1. Подтверждение математического соответствия (Parity)
Запуск протокола `EPHYS_PROBE_01` (10,000 тиков, $I_{in} = 350$ µV/tick) показал **100% математическое совпадение трасс** во всех четырех режимах (с точностью до $10^{-4}$ mV):
*   **Mode A (no_homeostasis)**: 137 спайков.
*   **Mode B (homeostasis_only)**: 61 спайков.
*   **Mode C (ahp_only)**: 115 спайков.
*   **Mode D (ahp_plus_homeostasis)**: 58 спайков.

> [!NOTE]
> В ходе интеграции было обнаружено расхождение в порядке выполнения распада: в прототипе-песочнице распад порога выполнялся в конце шага (после проверки спайка), в то время как в производственном коде Rust распад `homeostasis_decay` выполняется в начале шага (до GLIF обновления и проверки спайка). Мы обновили прототип `ephys_probe_01_replay_audit.py`, приведя его к производственному виду, после чего достигнуто полное потиковое совпадение всех трасс.

График трассы напряжения и динамики эффективного порога для Mode D сохранен как:
![EPHYS Probe 01 Trace](images/ephys_probe_01_replay_rust.png)

### 2. f-I Sweep по specimen 314900022 (Scnn1a_L4_excitatory)
Используя базовые параметры `L4_spiny_VISl4_4.toml` (Balanced winner: $R_{\text{in}}$ scale = 35.0, $\tau$ leak_shift = 8, refractory_period = 14, threshold = -45.6 mV) и инжектируя токи от -100 pA до 200 pA во временном окне $[1000, 2000]$ тиков, мы получили следующие результаты:

| Ток (pA) | Число спайков (sim) | Число спайков (bio) | First ISI (ticks) | Last ISI (ticks) | ISI Growth Ratio |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **-100** | 0 | 0.0 | - | - | - |
| **-50** | 0 | 0.0 | - | - | - |
| **0** | 0 | 0.0 | - | - | - |
| **30** | 16 | 0.0 | 49 | 78 | 1.5918 |
| **40** | 19 | 0.0 | 40 | 66 | 1.6500 |
| **50** | 22 | 3.5 | 35 | 58 | 1.6571 |
| **70** | 26 | 11.0 | 29 | 49 | 1.6897 |
| **90** | 29 | 20.0 | 26 | 43 | 1.6538 |
| **110** | 32 | 22.0 | 24 | 39 | 1.6250 |
| **130** | 34 | 26.0 | 22 | 36 | 1.6364 |
| **150** | 36 | 29.0 | 21 | 34 | 1.6190 |
| **190** | 40 | 36.0 | 20 | 31 | 1.5500 |
| **200** | 41 | - | 19 | 30 | 1.5789 |

Сгенерированные графики сравнения f-I кривых и трассы на 190 pA сохранены как:
- ![f-I Curve Comparison](images/full_neuron_fi_curve.png)
- ![Sweep 190 Trace](images/sweep_190_replay_rust.png)

### 3. Выводы по Phase 1
- **Адаптация порога (Homeostasis)**: Накопление `threshold_offset` после каждого спайка успешно воспроизводит Spike Frequency Adaptation (SFA) на полном производственном контуре (коэффициент роста ISI ~1.55–1.68).
- **Плавный профиль**: STA / восстановление мембраны имеет гладкий биологически реалистичный вид без ухода в silence или runaway при небольших изменениях входа.
- **Точность f-I кривой и гипервозбудимость**: SFA появилась, наклон на высоких токах (high-current slope) похож на биологический, но чувствительность на низких токах (low-current excitability) существенно завышена (при 30-40 pA в симуляции уже регистрируется 16-19 спайков, тогда как в биологическом эксперименте нейрон молчит).
- **Детерминизм**: Результаты полностью воспроизводимы и стабильны во всех запусках.


