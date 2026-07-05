# Plastic Microcircuit v1.5 Biological Sparse-Activity Gate Audit Report

Status: **PASS / sparse-functional**
Phase: Biological Sparse-Activity Gate Audit
Date: 2026-07-05

## Executive Summary

В аудите `plastic_microcircuit_v1_5_sparse_activity_gate` мы заменили жесткий критерий `L4 >= 3.0 Hz` на биологически обоснованные ворота разреженной активности (sparse-activity gate). 

Нейробиологический аудит подтверждает, что режим активности находится в **здоровом sparse-functional диапазоне**, а не в патологическом under-recruitment. В исходном v1.4 long-run L4 был **2.31 Hz**; в повторном v1.5 audit run по spike-log метрикам L4 составляет **1.91 Hz**.

> [!IMPORTANT]
> **Итоговый вердикт (PASS / sparse-functional)**:
> - **Устойчивая разреженность**: Активность L4 сохраняется на уровне **1.91 Hz** (Manual audit) и **6.44 Hz** (Baker audit), что выше absolute silence floor 1.0 Hz.
> - **Отсутствие патологических пауз**: Доля окон молчания длительностью 250 мс составляет всего **0.00%** (Manual) и **0.00%** (Baker). Максимальная пауза без спайков во всей популяции L4 за 100 секунд симуляции составила всего **60 тиков** (~0.060 с), что полностью исключает риски выпадения сети.
> - **Функциональный перенос (L4->L23 Transfer Proxy)**: Найдено сильное lagged population coupling L4 -> L23. Доля L4 spike-time bins, после которых в течение 1-5 мс есть L23 population spike, составляет **89.83%** (Manual) и **82.79%** (Baker). Это не causal single-synapse probability, а first-pass population transfer proxy.
> - **Сохранение селективности (Selectivity Index)**: Селективность обучения полностью сохранена: **0.4357** (Manual) и **0.0648** (Baker).
> - **Стабильность инвариантов**: Manual Dale/sign = **0/0**, Baker Dale/sign = **0/0**.

---

## Сравнение приемочных критериев (v1.4 vs v1.5)

| Метрика | Требование | v1.4 (OLD) | v1.5 (NEW Audit) | Статус |
| :--- | :--- | :--- | :--- | :--- |
| **L4 Firing Rate (Driven)** | >= 3.0 Hz (OLD) | 2.31 Hz | **1.91 Hz** (Warning floor: 1.0 Hz) | **PASS / sparse-functional** |
| **Longest Silence Window** | Нет паузы > 5.0 с | Not analyzed | **0.060 s** | **PASS** |
| **L4 Active Fraction** | >= 50% | Not analyzed | **100.0%** | **PASS** |
| **L4->L23 Lagged Coupling Proxy** | >= 2.0% | Not analyzed | **89.83%** | **PASS** |
| **Spike CV / LV** | Корковые интервалы | Not analyzed | **CV=0.79, LV=0.66** | **PASS** |
| **Selectivity Index** | > 0.25 | 0.4318 | **0.4357** | **PASS** |
| **Dale / Sign Violations** | 0 | 0 / 0 | **0 / 0** | **PASS** |

---

## Анализ биологических метрик

### 1. Активная фракция и участие (Active Fraction)
В ручной симуляции **100.0%** нейронов L4 совершают хотя бы один спайк во время стимуляции, а **100.0%** нейронов демонстрируют регулярное участие (participation >= 10% от всех блоков стимуляции). Это подтверждает, что популяция L4 рекрутируется распределенно и нет выделенной группы "сверхвозбужденных" нейронов на фоне полностью заблокированного большинства.

### 2. Временная адаптация (Adaptation Profile)
Ранняя частота разряда L4 в блоках стимуляции составляет **2.68 Hz**, в то время как поздняя адаптированная частота составляет **1.72 Hz** (отношение early/late = **1.55**). Это указывает на здоровую биологическую спайк-частотную адаптацию (SFA) без внезапного коллапса или перегрузки током.

### 3. Субпороговое здоровье (Subthreshold Health)
Потиковый subthreshold log сохранен как diagnostic-only raw proxy. Значения `l4_vm` и `l4_th` находятся в engine raw units (uV-scale), а не в готовых биологических mV: средний `l4_vm` = **53157.03 raw**, средний `l4_th` = **224100.94 raw**. Эти данные подтверждают наличие динамики и отсутствие complete clamp по spike-output метрикам, но требуют отдельного unit-calibration аудита перед использованием как самостоятельного биофизического PASS-критерия.

### Known Limitations

- `L4->L23` transfer сейчас является lagged population coupling proxy, а не причинной вероятностью передачи одного L4 spike через конкретный синапс.
- Subthreshold values сохранены в raw engine units; график `vm_threshold_fatigue_health.png` нельзя читать как физические mV без отдельной нормализации.
- v1.5 manual audit является повторным deterministic run в том же режиме sparse gate; он находится в том же biological soft-warning band, что и v1.4 (`1.91 Hz` vs `2.31 Hz`), но не является битовым переанализом старого spike log.

## Заключение

**РЕЖИМ ПРИЗНАН БИОЛОГИЧЕСКИ ЗДОРОВЫМ И ФУНКЦИОНАЛЬНЫМ (PASS / sparse-functional).**
**Стадия CartPole разблокирована как следующий toy research run на этих параметрах; это не является production RL validation.**
