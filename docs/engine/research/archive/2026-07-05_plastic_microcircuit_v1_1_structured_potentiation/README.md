# Research Archive: Plastic Microcircuit v1.1 Structured Potentiation

Status: partial pass
Slug: `plastic_microcircuit_v1_1_structured_potentiation`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование проверяет структурированное обучение и downstream перенос изменений пластичности на последующие слои:
- Проведен sweep параметров стимуляции, выбран победитель `structured_p=0.075`, `background_p=0.003`.
- Доказано селективное удержание Virtual->L4 коррелированных связей от LTD (-0.0167 uV vs -0.6111 uV).
- Положительная потенциация Virtual->L4 пока не доказана, так как matched mean delta остается ниже 0.
- Downstream перенос частичный: L4->L23 положительный, L4->L5 только менее депрессивный.

## Key Findings

1. **Virtual->L4 Protection**: matched delta -0.0167 uV против unmatched -0.6111 uV.
2. **Downstream Bias**: L4->L23 matched bias +0.1142 uV, L4->L5 matched bias +0.0269 uV.
3. **Physiology Status**: runaway/sign violations отсутствуют, но L4 rate ниже hard gate 3 Hz.
4. **CartPole Blocked**: переход к RL остается закрыт до positive potentiation + activity pass.

## Reports & Outputs

- Full Report: [reports/plastic_microcircuit_v1_1_structured_potentiation.md](reports/plastic_microcircuit_v1_1_structured_potentiation.md)
- Plots: [images/](images/)
