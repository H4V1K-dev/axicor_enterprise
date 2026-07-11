# Plastic Microcircuit v1.4 Controlled + Baker Shadow Report

Status: partial / activity gate failed
Phase: GSOP/STDP Controlled + Baker Shadow
Started: 2026-07-05
Completed: 2026-07-05

## Executive Summary

В исследовании `plastic_microcircuit_v1_4_controlled_baker_shadow` мы проверили пластическую микросеть в ручной controlled topology и на baker-compiled shadow shard. Результат сильнее v1.3: selectivity index в manual run прошел целевой порог, а baker shadow сохранил положительный matched-bias trend. Однако pre-CartPole gate не закрыт, потому что финальный 100k-tick manual learning run просел по L4 activity ниже hard gate.

> [!IMPORTANT]
> **Итоговый вердикт (PARTIAL / activity gate failed)**:
> - **Phase A (Manual)**: Selectivity прошла gate, но activity gate не закрыт на финальном long-run:
>   - N=256 learning: L4=**2.31 Hz**, L23=**6.40 Hz**, L5=**1.39 Hz**.
>   - N=512 sanity: L4=**8.71 Hz**, L23=**18.29 Hz**, L5=**10.57 Hz**.
>   - Selectivity index: **0.4318** (target >= 0.25, PASS).
> - **Phase B (Baker Shadow)**: Spatial connectome скомпилирован и запущен успешно:
>   - Somas: **384**, Synapses: **48786**.
>   - Baker selectivity index: **0.0648** (positive matched-bias trend, PASS).
>   - Invariants: 0 нарушений Dale's Law, 0 sign flips.
> - **CartPole**: blocked; RL-стадия не должна запускаться до закрытия final manual activity gate.

---

## Статус приемочных критериев

| Критерий | Требование | Результат (Manual) | Результат (Baker) | Статус |
| :--- | :--- | :--- | :--- | :--- |
| **Dale's Law** | Веса не пересекают 0 | 0 нарушений | 0 нарушений | **PASS** |
| **Sign Integrity** | Исключены случайные перескоки знака | 0 перескоков | 0 перескоков | **PASS** |
| **Manual L4 Learning Rate** | >= 3.0 Hz | **2.31 Hz** | - | **FAIL** |
| **Manual L23/L5 activity** | L23: 3..35Hz, L5: 1..15Hz | L23=6.40Hz, L5=1.39Hz | - | **PASS** |
| **Manual Selectivity Index** | >= 0.25 | **0.4318** | - | **PASS** |
| **Baker Activity Smoke** | no silence/runaway trend | - | L4=7.18Hz, L23=5.89Hz, L5=1.00Hz | **PASS / smoke** |
| **Baker Transfer Trend** | selectivity > 0 | - | **0.0648** | **PASS** |

---

## Параметры победителя (Winner Parameters)

- `fatigue_capacity` = **18**
- `gsop_potentiation` = **240**
- `gsop_depression` = **68**
- `virt_w` = **3500**
- `inh_l23_l4` = **-900**
- `structured_p` = **0.1100**

---

## Результаты пространственной компиляции (Phase B Baker)

Спецификация шарда `16x16x32` успешно скомпилирована бейкером за счет пространственного роста отростков:
- **VirtualInput** (128 somas) выросли вертикально вверх (vertical bias = 2.0) и сформировали плотный синаптический пучок с **L4_spiny** (128 somas).
- Пост-хок анализ показал, что L4 нейроны образовали селективные matched-связи с пространственно близкими группами виртуальных входов.
- В ходе 100k ticks пластического обучения matched-связи показали устойчивый рост относительно unmatched-контроля (selectivity = **0.0648**).
- Это подтверждает положительный shadow-transfer trend, но не снимает блокировку CartPole из-за manual L4 activity failure.

### Known Limitation

`baker_segment_distance_distribution.png` пока является synthetic placeholder: runner не экспортирует реальные segment distances из baker artifacts. Его нельзя использовать как доказательство распределения физических расстояний до добавления measured distance logging.

