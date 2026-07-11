# Research Archive: Plastic Microcircuit v1.2 Positive Potentiation / Activity Recovery

Status: partial pass
Slug: `plastic_microcircuit_v1_2_positive_potentiation_activity_recovery`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование демонстрирует строгую положительную потенциацию сконструированных matched Virtual->L4 синаптических путей, но не закрывает все hard gates:
- Проведен 16-компонентный sweep, позволивший найти оптимальный набор параметров.
- Доказана положительная потенциация matched связей (mean delta mass: 68834.9, exact charge: +1.0503 uV).
- `Virtual->L4` unmatched-control отсутствует (matched n=1024, unmatched n=0), поэтому pathway selection не доказан.
- N=256 learning не проходит L4 activity gate (`1.54 Hz < 3.0 Hz`).

## Key Findings

1. **Virtual->L4 Potentiation**: matched mean 1.0503 uV vs unmatched 0.0000 uV.
2. **Pathway Control Gap**: unmatched Virtual->L4 count = 0; это делает matched/unmatched ratio невалидным.
3. **Physiology Status**: partial; N=512 sanity проходит, N=256 learning L4 ниже gate.
4. **CartPole Blocked**: переход к RL остается закрыт до control-preserving positive potentiation + activity pass.

## Reports & Outputs

- Full Report: [reports/plastic_microcircuit_v1_2_positive_potentiation_activity_recovery.md](reports/plastic_microcircuit_v1_2_positive_potentiation_activity_recovery.md)
- Plots: [images/](images/)
