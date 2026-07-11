# Research Archive: Plastic Microcircuit v1.3 Control-Preserving Potentiation

Status: partial / activity gate failed / positive-ratio tie
Slug: `plastic_microcircuit_v1_3_control_preserving_potentiation`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование проверяет селективность пластичности GSOP/STDP с помощью введения контрольной unmatched группы:
- Введена topology с 8 matched и 4 unmatched Virtual->L4 связями.
- Доказано relative matched bias: matched растет сильнее unmatched (mean delta mass: 178125.0 vs 96391.2; exact charge: +2.7180 uV vs +1.4708 uV).
- Unmatched контрольные связи тоже растут, поэтому binary positive-ratio gate не закрыт.
- N=512 sanity проходит activity gate, но N=256 learning L4 остается ниже hard gate.

## Key Findings

1. **Virtual->L4 Selective Potentiation**: matched mean 2.7180 uV vs unmatched 1.4708 uV.
2. **Physiology Status**: partial; N=256 learning L4=2.62 Hz ниже gate.
3. **CartPole Blocked**: переход к RL остается закрыт до full activity + pathway gates.

## Reports & Outputs

- Full Report: [reports/plastic_microcircuit_v1_3_control_preserving_potentiation.md](reports/plastic_microcircuit_v1_3_control_preserving_potentiation.md)
- Plots: [images/](images/)
