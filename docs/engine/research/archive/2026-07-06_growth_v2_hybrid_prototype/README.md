# Growth v2 Hybrid Prototype & 3D Atlas

Status: finished
Started: 2026-07-06
Completed: 2026-07-06

## Question
Как построить гибридный алгоритм роста аксонов (Growth v2), который сочетает направленное векторное притяжение (continuous gradient, FOV cone, type affinity) из MVP с жесткими физическими гарантиями дискретной решетки AxiEngine (0 collisions, 0 self-intersections, 0 out-of-bounds, strict whitelists), и предотвращает образование концевых клубков (terminal knots)?

## Expectation
1. Будет реализован гибридный режим шага: непрерывный расчет сил steering blend (`v_global + v_attract + v_noise`) используется для оценки и ранжирования дискретных 26-соседних шагов.
2. Гибридный режим полностью пройдет жесткие invariants (0 нарушений).
3. Успешность проецирования в целевые слои (Virtual -> L4) значительно вырастет по сравнению с дискретным v1 baseline.
4. Концевая плотность (density) снизится по сравнению с MVP благодаря механизмам `capture stop`, `attraction damping` и `monotonicity stop`.

## Inputs
- Конфигурация слоев и соматических типов из Baker Spatial Growth Audit v1.
- Placed somas для семян 12345, 12346, 12347.

## Method
1. Реализовать три режима роста в тесте `run_growth_v2_hybrid_prototype` в `baker_growth_v2.rs`.
2. Экспортировать полную статистику сомиков, путей и синапсов в `artifacts/growth_v2_comparison_data.json`.
3. Разработать Python-скрипт `plot_growth_v2_atlas.py` для построения 3D атласа путей, XZ-проекций, тепловых карт окончаний и гистограмм синаптической плотности.

## Commands
```bash
# Rust test target
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test baker_growth_v2 run_growth_v2_hybrid_prototype -- --nocapture

# Python plotting
.venv/bin/python3 docs/engine/research/archive/2026-07-06_growth_v2_hybrid_prototype/scripts/plot_growth_v2_atlas.py
```

## Outputs
- `reports/growth_v2_hybrid_prototype.md` — детальный научный отчет и ответы на вопросы гейта.
- `scripts/plot_growth_v2_atlas.py` — скрипт построения 3D атласа и графиков.
- `images/comparison_panel_3d.png` — 3D-атлас сравнения путей трех моделей.
- `images/side_view_projections.png` — проекции путей XZ по слоям.
- `images/endpoint_density_heatmaps.png` — тепловые карты плотности окончаний XY.
- `images/stop_reasons_and_lengths.png` — гистограммы длин и причины останова.
- `images/terminal_knot_analysis.png` — сравнение концевой извилистости и локальной плотности.
- `images/synapse_candidate_distributions.png` — число сформированных связей и дубликатов.

## Result
Гибридный режим (Hybrid Growth v2) полностью прошел все жесткие invariants (0 out-of-bounds, 0 self-intersections, 0 collisions, 0 whitelist/radius violations).
Показана высокая эффективность направленного роста:
- Успешность проецирования Virtual -> L4 выросла с 60.9% (discrete v1) до **83.6%** (+37% улучшения).
- Концевая плотность снизилась с 2.67 (MVP) до **1.65** (-38% уменьшения) благодаря раннему захвату (90.6% axons успешно завершились по `TargetReached`).
- Fan-in caveat: Hybrid v2 породил **112,261 raw contacts**, но после production-style `MAX_DENDRITES=128` cap осталось **29,021 accepted synapses**, а **83,240 candidates** были отброшены. Это требует отдельного sweep по плотности/уникальности перед production migration.

## Interpretation
Гибридный подход соединяет лучшие стороны обоих миров: физическую строгость и детерминизм дискретного автомата с биологической направленностью и красотой трактов векторного поля. Опробованные анти-knot механизмы улучшают окончание роста, но высокая raw contact density показывает, что это еще research atlas, а не готовый production-порт.

## Next Step
Дождаться биологического аудита формы траекторий, затем провести sweep по `w_global/w_attract/w_noise`, `R_capture`, `R_damping`, fan-in cap pressure и source-target uniqueness. Только после этого принимать решение о production Growth v2.
