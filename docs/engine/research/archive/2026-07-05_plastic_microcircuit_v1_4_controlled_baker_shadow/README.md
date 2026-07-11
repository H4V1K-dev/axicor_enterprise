# Research Archive: Plastic Microcircuit v1.4 Controlled + Baker Shadow

Status: partial / activity gate failed
Slug: `plastic_microcircuit_v1_4_controlled_baker_shadow`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование проверяет pre-CartPole gate на управляемой manual topology и bakers-compiled spatial connectome:
- В Phase A найден кандидат, проходящий selectivity index >= 0.25, но финальный 100k-tick learning run не проходит L4 learning gate >= 3.0 Hz.
- В Phase B доказано успешное прохождение компиляции (baker) и симуляции (compute-cpu), а также сохранение тренда matched bias на пространственной топологии.
- Блокировка CartPole RL-стадии остается до восстановления L4 activity на финальном manual long-run.

## Outputs
- Отчёт: [plastic_microcircuit_v1_4_controlled_baker_shadow.md](reports/plastic_microcircuit_v1_4_controlled_baker_shadow.md)
- Графики: [images/](images/)
- Артефакты симуляций: [artifacts/](artifacts/)
