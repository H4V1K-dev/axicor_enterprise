# Baker Axon Growth & Synapse Geometry Audit v1

Status: finished
Started: 2026-07-06
Completed: 2026-07-06

## Question
Как именно растут аксоны в 3D, куда они доходят, почему останавливаются (stop reasons), корректно ли dendrite-radius ловит контакты по сегментам, соблюдаются ли все геометрические инварианты, и как выглядит 3D визуализация шарда?

## Expectation
1. `VirtualInput` с положительным вертикальным смещением (+2.0) должен расти вверх.
2. `L5_spiny` с отрицательным вертикальным смещением (-1.5) должен чаще расти вниз.
3. `L23_aspiny` с нейтральным смещением (0.0) должен быть более латеральным/возвратным.
4. `L4_spiny` (+1.0) должен расти вверх/латерально.
5. Все геометрические invariants (out-of-bounds, self-intersection, soma voxel intersection, whitelist, segment reference, dendrite radius, self-synapse, determinism) должны строго соблюдаться (hard pass).
6. Из-за геометрии малого шарда (16x16x32) большинство аксонов должны останавливаться по `BoundaryReached` из-за близости боковых границ X/Y.

## Inputs
- Конфигурация fixed-whitelist из Baker Spatial Growth Audit v1.
- Shard 16x16x32, seeds 12345, 12346, 12347.
- 384 somas.

## Method
1. Реализовать тест-раннер `run_baker_axon_growth_synapse_geometry_v1` в `AxiEngine/crates/test-harness/tests/full_neuron_replay.rs`.
2. Встроить в раннер автоматическую верификацию детерминизма и геометрических инвариантов.
3. Экспортировать полную статистику контактов и ростовых путей в JSON.
4. Разработать Python-скрипт `baker_axon_geometry_analysis.py` для вычисления распределений и генерации 3D-графиков (soma positions, axon paths, synapse contacts, endpoint directions, candidate density).

## Commands
```bash
# Rust test-runner
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test full_neuron_replay run_baker_axon_growth_synapse_geometry_v1 -- --nocapture

# Python analysis & rendering
.venv/bin/python3 docs/engine/research/archive/2026-07-06_baker_axon_growth_synapse_geometry_v1/scripts/baker_axon_geometry_analysis.py
```

## Outputs
- `reports/baker_axon_growth_synapse_geometry_audit_v1.md` — подробный научный отчет.
- `scripts/baker_axon_geometry_analysis.py` — скрипт анализа.
- `images/soma_positions_3d.png` — 3D scatter сомиков по слоям.
- `images/axon_paths_3d_by_type.png` — 3D полилинии путей аксонов.
- `images/synapse_contacts_3d.png` — 3D визуализация связей.
- `images/axon_endpoint_3d.png` — 3D векторы роста аксонов.
- `images/candidate_density_3d.png` — 3D плотность кандидатов.
- `images/axon_length_distribution.png` — распределение длин аксонов.
- `images/axon_tortuosity_distribution.png` — извилистость путей аксонов.
- `images/candidate_metrics.png` — сравнение принятых и сброшенных синапсов.
- `artifacts/baker_axon_geometry_summary.json` — сохраненный JSON с variance-статистикой.

## Result
Все жесткие геометрические инварианты успешно пройдены:
- 0 segment out-of-bounds violations.
- 0 axon self-intersections.
- 0 segment soma voxel intersections.
- 0 whitelist violations.
- 0 missing axon segment references.
- 0 dendrite radius violations.
- 0 self-synapses.
Детерминизм подтвержден (seed 12345 дает 100% идентичные структуры при повторном запуске).
Выявлена и физически обоснована причина относительно коротких аксонов (mean ~5.2 voxels) и 100% stop reason `BoundaryReached`: в узком шард-пространстве (16x16 в сечении) среднее расстояние до боковых границ X/Y не превышает 4-5 вокселей, что приводит к быстрой остановке роста при касании стенок.

## Interpretation
Baker строит геометрически честный и верифицированный 3D коннектом. Вертикальные смещения нейронов отрабатывают в полной мере: восходящие пути VirtualInput и L4 направлены вверх, нисходящие L5 направлены вниз, а L23 меандрирует латерально. Синапсы образуются строго в точках пересечения путей с dendrite radius target-нейронов.

## Next Step
Перейти к `Baker Functional Topology Replay` (4.2 в дорожной карте) для проверки функциональной динамики активности и пластичности на полученном пространственном коннектоме.
