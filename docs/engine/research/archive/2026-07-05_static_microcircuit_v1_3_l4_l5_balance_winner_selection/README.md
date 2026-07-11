# Research Archive: Static Microcircuit v1.3 L4/L5 Balance & Winner Selection

Status: completed
Slug: `static_microcircuit_v1_3_l4_l5_balance_winner_selection`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование частично закрывает задачу одновременной балансировки слоев L4/L23/L5 в статической микросети:
- Внедрена winner selection policy с жесткой приоритизацией `passed_all_gates`.
- Расширен диапазон sweep L23 feedback inhibition split.
- Оценены все жесткие gates: Vm health, threshold recovery, selectivity, E/I ablation.

## Key Findings

1. **L4/L5 Balance Gate Passed**: Найдены конфигурации, полностью удовлетворяющие Moderate Activity на N=256 (например, L23->L4 = -1500, L23->L5 = -1000).
2. **N=512 Borderline**: Winner-конфигурация показывает 2.76 Hz на L4 при hard gate 3..25 Hz и 8.14 Hz на L5, поэтому масштабирование почти стабильно, но формально еще не прошло.
3. **Physiology Gate Not Closed**: Переход к пластичности заблокирован до минимального N=512 fine-tuning pass.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_v1_3_l4_l5_balance_winner_selection.md](reports/static_microcircuit_v1_3_l4_l5_balance_winner_selection.md)
- Plots: [images/](images/)
