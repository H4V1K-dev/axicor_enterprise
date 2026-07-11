# Research Archive: Static Microcircuit v1.4 N=512 Fine-Tuning

Status: completed
Slug: `static_microcircuit_v1_4_n512_fine_tuning`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование полностью закрывает задачу одновременной балансировки слоев L4/L23/L5 в статической микросети:
- Проведен тонкий sweep тормозных сплитов L23 на совместных размерах N=256 и N=512.
- Найдена оптимальная конфигурация, проходящая все жесткие ворота без Vm saturation и runaway.
- Разблокировано исследование пластичности (Plastic Microcircuit).

## Key Findings

1. **L4/L5 Balance Gate Passed**: Winner-конфигурация (`L23->L4 = -1200`, `L23->L5 = -1250`) дает L4 = 4.05 Hz / L5 = 4.30 Hz на N=256 и L4 = 3.64 Hz / L5 = 5.72 Hz на N=512.
2. **Physiology Gate Closed**: Все 10 приемочных критериев пройдены на обоих масштабах.
3. **Plasticity Ready**: Разблокирован шаг `Plastic microcircuit` (GSOP/STDP).

## Reports & Outputs

- Full Report: [reports/static_microcircuit_v1_4_n512_fine_tuning.md](reports/static_microcircuit_v1_4_n512_fine_tuning.md)
- Plots: [images/](images/)
