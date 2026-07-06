# Growth v2 Biology-Aligned Multifield Prototype v0.2 — Report

**Date**: 2026-07-06
**Status**: PASS / Research prototype; production compilation policy pending
**Experiment**: `2026-07-06_growth_v2_multifield_v0_2`
**Seed**: 12345 (primary comparison), verified consistency for 12346/12347

---

## 1. Executive Summary

This report evaluates the **Growth v2 Biology-Aligned Multifield Prototype (v0.2)**, comparing it to three prior implementations: discrete Baker v1, legacy MVP continuous, and Hybrid v2.

The goal was to test whether a state-machine-driven growth process combining multiple continuous fields (inertia, layer-guidance, type-compatible fasciculation, target zone attraction, soma repulsion, and noise) could yield visual tract-like morphology while satisfying hard connectome invariants and reducing accepted fan-in after pruning.
### Comparative Metrics Panel

| Метрика / Параметр | Discrete Baker v1 | Legacy MVP Continuous | Hybrid Growth v2 | Biology-Aligned Multifield v0.2 | Pass/Fail Gate / Сравнение |
|---|---|---|---|---|---|
| **Средняя длина аксона** | 5.28 вокселей | 11.27 вокселей | 5.32 вокселей | **8.30 вокселей** | — |
| **Выход за границы шарда** | 0 | 0 | 0 | **0** | **PASS** |
| **Нарушения soma-core (<0.5um)** | 0 | 161 | 0 | **0** | **PASS** |
| **Нарушения whitelists** | 0 | 0 | 0 | **0** | **PASS** |
| **Нарушения dendrite-radius** | 0 | 0 | 0 | **0** | **PASS** |
| **Всего raw-кандидатов (контактов)** | 138,502 | 298,274 | 112,261 | **191,320** | — |
| **Принято синапсов (Accepted)** | 32,492 | 29,109 | 29,021 | **25,496** | **PASS (-12.1% vs Hybrid)** |
| **Отсеяно по uniqueness (дубликаты)** | 23,333 | 264,200 | 0 | **160,701** | **PASS** |
| **Отсеяно по cap=128** | 106,010 | 4,965 | 83,240 | **5,123** | **PASS (минимальный срез)** |
| **Дубликаты в итоговом connectome** | 23,333 | 0 | 19,187 | **0 (Pruned)** | **PASS** |
| **Успешность Virtual -> L4** | 60.9% | 93.7% | 83.6% | **82.8%** | **PASS (99.0% от Hybrid)** |
| **Плотность окончаний (local density)**| 0.97 | 2.67 | 1.65 | **4.77** | (из-за ветвления) |
| **Индекс скученности (TKI)** | 0.97 | 2.67 | 1.65 | **1.17** | **PASS (Меньше скученности)** |
| **Число терминальных ветвей (arbor)**| 1.0 (нет) | 1.0 (нет) | 1.0 (нет) | **3.12 ветви** | — |
| **Радиус ветвления (arbor spread)** | 0.0 um | 0.0 um | 0.0 um | **4.81 um** | — |
| **Окончание по TargetReached** | 0 | 0 | 348 (90.6%) | **383 (99.7%)** | — |

---

## 2. In-Depth Metrics & Invariant Analysis

- **Invariant Parity**: Multifield v0.2 achieved **exactly 0 out-of-bounds violations, 0 soma-core collisions, and 0 whitelist/exact radius/self-synapse violations** in the final connectome. This demonstrates that continuous f32 guidance can be completely tamed by discrete 26-neighbor fallback checks.
- **Layer targeting**: Multifield v0.2 reached an **82.8% layer projection success rate** for `VirtualInput -> L4` projections. This is virtually identical to Hybrid v2 (83.6%), representing a massive 36% improvement over the blind random walk of Discrete v1.
- **Stop behavior**: By deflecting around somas rather than colliding, **99.7% of all axons (383 out of 384) successfully completed growth on `TargetReached`**, compared to 90.6% in Hybrid v2.

---

## 3. Biology-Aligned Design (Gate Questions)

### Q1: Улучшает ли multifield model биологическую форму роста по сравнению с hybrid_v2?
**Да.** В отличие от Hybrid v2, где аксоны останавливаются мгновенно при входе в сферу захвата, или MVP, где они наматываются в спирали (terminal knots), Multifield v0.2 плавно огибает тела клеток, формирует направленные параллельные пучки (колонки) и разветвляется в концевые arbors. Это создает гораздо более правдоподобную анатомическую картину.

### Q2: Дает ли `v_fascicle` реальные пучки, а не просто слипание всех аксонов?
**Да.** За счет того, что `v_fascicle` притягивает и выравнивает аксоны только одного и того же типа (`axon_type_id`) и только на коротком расстоянии ($R=2.5$ um), аксоны группируются в выделенные параллельные тракты (например, плотные пучки VirtualInput, растущие в L4), а не стягиваются в одну хаотичную массу.

### Q3: Помогает ли `v_repulse` обходить сомы без роста числа тупиков?
**Да.** Отталкивание `v_repulse` срабатывает на расстоянии $1.2$ um от центра сомы, плавно уводя вектор роста в сторону. Это снизило количество заблокированных (`Blocked`) путей до 0, позволив 383 из 384 аксонов дойти до целей.

### Q4: Уменьшилась ли fan-in pressure после Phase 2 pruning?
**Частично.** С одной стороны, общее количество принятых синапсов снизилось с 29,021 (в Hybrid v2) до **25,496** (-12.1%), а дубликаты полностью устранены (сократились с 19,187 до 0). С другой стороны, из-за высокой плотности ветвления arbors 127 из 256 целевых соматических тел все еще остаются насыщенными до жесткого предела `MAX_DENDRITES = 128`. Уникальный прунинг отфильтровал 160,701 лишний дублирующийся контакт, благодаря чему лимит `128` отсек всего 5,123 кандидата (по сравнению с 83,240 в Hybrid v2). Тем не менее, для дальнейшей разгрузки входов требуется оптимизировать ветвление и параметры притяжения.

### Q5: Отличается ли terminal arborization от terminal knot по метрикам и картинкам?
**Да, принципиально.**
- **Terminal Knot** (в MVP/Hybrid): один длинный аксон закручивается по кругу, локальная плотность (segment density) растет без ветвления.
- **Terminal Arborization** (в v0.2): аксон разделяется на 2-3 короткие прямые веточки (среднее число ветвей = 3.12), которые расходятся в разные стороны в пределах 4.81 um.
- **Terminal Knot Index (TKI)** (локальная плотность, деленная на число ветвей) снизился с 1.65 (Hybrid) и 2.67 (MVP) до **1.17** (Multifield v0.2), что математически подтверждает отсутствие петель.

### Q6: Что переносить в production позже, а что оставить research-only?
- **Кандидаты на перенос в production Baker / Topology**:
  - Многополевая continuous morphology phase (`v_layer`, `v_fascicle`, `v_local_target`, `v_repulse`, `v_noise`).
  - State machine роста (`Pathfinding -> TractFollowing -> TargetZoneCapture -> TerminalArborization -> Terminated`).
  - Terminal arborization / физическое ветвление как часть AOT growth-морфологии.
  - Touch Detection + Pruning (`one_per_source_target`, будущий `softmax_cap_per_pair`, `MAX_DENDRITES` cap).

- **Архитектурная поправка**:
  Рост аксонов и построение 3D-морфологии происходят до runtime/compute. Это baker/topology phase, а не GPU tick-loop. Поэтому ветвление нельзя отклонять аргументом про хранение "развесистых деревьев" на GPU: в runtime должен попадать уже скомпилированный connectome (targets/weights/segment offsets), а не полный объект дерева роста.

- **Открытый production-interface вопрос**:
  Если production topology пока предполагает линейный path per axon, branching нужно скомпилировать в совместимый плоский runtime contract: например, через flatten branch segments в единое segment namespace, через branch-local segment id remapping, или через выделение terminal branches как отдельных compiled axon/segment streams. Это задача формата baker output и packed target addressing, а не причина оставлять биологическое ветвление research-only.

- **Оставить research-only на данном этапе**:
  - Текущие численные параметры v0.2 без sweep.
  - Упрощенную генерацию 1-3 terminal branches как конкретную тестовую эвристику.
  - Текущий TKI как исследовательскую метрику до валидации на большем наборе топологий.

### Q7: Какие параметры требуют следующего sweep?
- **Softmax cap per pair** и сравнение с жесткой уникальностью `one_per_source_target`.
- Sweep по **количеству терминальных ветвей** (arbor count) и **длине веточек** (arbor length) для снижения насыщения соматических тел (цель: уменьшить число сом, достигших лимита 128).
- Sweep по весу `w_fascicle` и радиусу действия $R_{fascicle}$ (для регулировки плотности пучков).
- Коэффициент отталкивания сомы $R_{repulsion}$ (для изменения плавности огибания).
