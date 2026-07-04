# Research Archive: Static Microcircuit Scale-Up v1

Status: completed
Slug: `static_microcircuit_scale_up_v1`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование оценивает стабильность физиологии и производительность симулятора при масштабировании малой пространственной микросети от 128 до 1,000,000 нейронов.

## Key Findings

1. **Успешный release Load Test (1,000,000 нейронов)**: 10-тиковый perf/load-only сценарий со 128 миллионами синапсов запускается на CPU без OOM и переполнений.
2. **Физиология inconclusive**: N=128/256/512 не уходят в runaway, но Vm health падает, а L5 почти молчит на N=128/256.
3. **Переход к plasticity заблокирован**: нужен отдельный v1.1 прогон с input scaling, E/I ablation и жесткими Vm/threshold gates.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_scale_up_v1.md](reports/static_microcircuit_scale_up_v1.md)
- Plots: [images/](images/)
