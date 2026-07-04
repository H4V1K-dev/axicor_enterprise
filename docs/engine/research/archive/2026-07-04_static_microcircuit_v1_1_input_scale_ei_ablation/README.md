# Research Archive: Static Microcircuit v1.1 Input Scale & E/I Ablation

Status: completed
Slug: `static_microcircuit_v1_1_input_scale_ei_ablation`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование проверяет физиологические проблемы первой версии статической микросети:
- L4 Vm Health: убран перегрев мембраны выше -25 mV.
- L5 Activity: проверено, почему класс L5 остается слабым.
- E/I Ablation: проверена роль торможения L23 в стабилизации сети.

## Key Findings

1. **Vm Health Gate Passed**: L4 мембрана удерживается в физиологических рамках без перегрева.
2. **E/I Ablation Informative**: Без торможения L23 активность L4/L23/L5 резко растет, но runaway не фиксируется.
3. **L5 Gate Failed**: В full network L5 остается ниже целевого диапазона 1..15 Hz.

## Reports & Outputs

- Full Report: [reports/static_microcircuit_v1_1_input_scale_ei_ablation.md](reports/static_microcircuit_v1_1_input_scale_ei_ablation.md)
- Plots: [images/](images/)
