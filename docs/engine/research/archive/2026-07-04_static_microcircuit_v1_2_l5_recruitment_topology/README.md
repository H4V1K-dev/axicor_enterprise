# Research Archive: Static Microcircuit v1.2 L5 Recruitment & Topology

Status: completed
Slug: `static_microcircuit_v1_2_l5_recruitment_topology`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование частично закрывает задачу вывода L5 пирамидного класса в физиологический целевой диапазон 1..15 Hz под полной сетью:
- Проведен пошаговый sweep L4->L5 excitation силы и fan-in.
- Реализовано разделение тормозного действия L23 на L4 и L5.
- Оценены все жесткие gates: Vm health, threshold recovery, selectivity, E/I ablation.

## Key Findings

1. **L5 Recruitment Gate Passed**: Winner-конфигурация дает L5 в целевом диапазоне в full network.
2. **Inhibition Split Crucial**: Снижение тормозного влияния L23->L5 при сохранении сильного L23->L4 является ключевым фактором рекрутирования.
3. **L4 Gate Failed**: L4 переторможен ниже целевого диапазона, поэтому переход к STDP пока преждевременен.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_v1_2_l5_recruitment_topology.md](reports/static_microcircuit_v1_2_l5_recruitment_topology.md)
- Plots: [images/](images/)
