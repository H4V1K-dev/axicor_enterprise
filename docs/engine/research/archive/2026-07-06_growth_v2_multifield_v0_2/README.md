# Growth v2 Biology-Aligned Multifield Prototype v0.2

Status: finished
Started: 2026-07-06
Completed: 2026-07-06

## Question
Позволяет ли многополевой непрерывный рост аксонов (multifield continuous growth) с двухфазной моделью (Morphology Phase 1 + Touch Detection/Pruning Phase 2) сформировать направленные, визуально похожие на тракты волокна, огибающие сомы и образующие разветвленные концевые древа (arbors), при полном исключении геометрических нарушений и снижении fan-in pressure?

## Expectation
1. Будет реализован новый режим роста `multifield_v0_2` с стейт-машиной (Pathfinding, TractFollowing, TargetZoneCapture, TerminalArborization, Terminated) и 6 силами.
2. Будет реализовано непрерывное отталкивание сомы (`v_repulse`) и разветвление терминалей (1-3 ветки) без спирального зацикливания.
3. Вторая фаза (Touch Detection + Pruning с режимом `one_per_source_target`) полностью отсечет дубликаты и ограничит число синапсов до 128.
4. Будет достигнуто 0 out-of-bounds, 0 soma-core и 0 whitelist/radius нарушений.
5. Индекс терминального узла (Terminal Knot Index) снизится по сравнению с MVP.

## Inputs
- Конфигурация слоев и соматических типов из Baker Spatial Growth Audit v1.
- Placed somas для семени 12345.

## Method
1. Реализовать симуляцию и метрики в `baker_growth_v2.rs`.
2. Экспортировать полную статистику путей, ветвлений и синапсов в `artifacts/growth_v2_comparison_data.json`.
3. Разработать Python-скрипт `plot_growth_v2_multifield.py` для генерации 10 панелей визуализации (3D сравнения, проекции по слоям, огибание сомы, ветвление арборов, тепловые карты окончаний, графики прунинга и переходов).

## Commands
```bash
# Rust test target
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test baker_growth_v2 run_growth_v2_multifield_v0_2 -- --nocapture

# Python plotting
.venv/bin/python3 docs/engine/research/archive/2026-07-06_growth_v2_multifield_v0_2/scripts/plot_growth_v2_multifield.py
```

## Outputs
- `reports/growth_v2_multifield_v0_2.md` — детальный научный отчет и ответы на вопросы гейта.
- `scripts/plot_growth_v2_multifield.py` — скрипт визуализации.
- Изображения в `images/`:
  - `comparison_panel_3d.png` — 3D-атлас сравнения путей.
  - `projections_and_fasciculation.png` — проекции XZ по слоям и Bundling.
  - `soma_repulsion_and_arbors.png` — детали обтекания соматических тел и структура концевого ветвления.
  - `endpoint_density_heatmaps.png` — тепловые карты плотности окончаний.
  - `synapses_and_saturation.png` — сравнение синапсов и гистограмма насыщения входов.
  - `state_transitions.png` — гистограмма активности стейт-машины.

## Result
Эксперимент признан полностью успешным (PASS по всем гейтам):
- Достигнуто ровно 0 invariants нарушений.
- Layer success rate Virtual->L4 составил **82.8%** (+36% улучшения по сравнению с v1).
- Раннее ветвление вместо петель снизило Terminal Knot Index до **1.17** (против 2.67 в MVP).
- Pruning сократил accepted synapses с **29,021** (Hybrid после cap) до **25,496** (-12.1% apples-to-apples) и убрал все дубликаты source-target. Raw-кандидатов остается много (**191,320**), поэтому fan-in pressure считается частично решенной задачей и требует sweep.

## Interpretation
Разделение роста на Morphology Phase (где силы steering обеспечивают визуальную красоту и биологичность) и Touch Detection/Pruning Phase 2 (где hard invariants отсекают все лишнее) является наиболее устойчивой архитектурой для Growth v2.

Физическое ветвление относится к AOT baker/topology phase, а не к runtime GPU payload. Production-вопрос не в том, можно ли ветвить аксон, а в том, как скомпилировать branch morphology в текущий плоский runtime contract targets/weights/segment offsets.

## Next Step
`Growth v2 parameter sweep & pruning policy`: подобрать параметры ветвления/fasciculation/repulsion и выбрать production compile policy для branch segments.
