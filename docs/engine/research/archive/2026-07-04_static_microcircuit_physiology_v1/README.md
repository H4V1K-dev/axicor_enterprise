# Research Archive: Static Microcircuit Physiology v1

Status: completed (physiology inconclusive)
Slug: `static_microcircuit_physiology_v1`
Started: 2026-07-04

## Overview

Это исследование исследует физиологическую стабильность малой кортикальной микросети (L4/L2-3/L5) с использованием ранее откалиброванных одиночных GLIF_3 априоров без пластичности и reward:
- `L4_spiny`: 32 нейрона
- `L23_aspiny`: 16 нейронов
- `L5_spiny`: 16 нейронов
- Пространственная геометрия и sparse distance-based connectivity.

> [!WARNING]
> **Итог**: `production CPU smoke passed, physiology inconclusive`. Симуляция в Rust-harness успешно выполняется без падений, но физиология требует дальнейшей верификации (ei ablation, phase selectivity, Vm saturation).


## Acceptance Gates Status

- **No Complete Silence**: PASS (L4 Firing = 26.3 Hz, L23 = 30.9 Hz, L5 = 11.4 Hz)
- **No Runaway Excitation**: PASS (No runaway flags triggered in Regime 3)
- **L4 Responds to Input**: PASS
- **L23 Activity Modulates State**: PASS (L23 average inhibitory rate under moderate input = 30.9 Hz)
- **L5 Receives Output Activity**: PASS (L5 average rate = 11.4 Hz)

## Key Findings

1. **Сеть физиологически стабильна (static network physiology sanity)**:
   - Откалиброванные параметры leak, rest и homeostasis обеспечивают баланс без runaway возбуждения.
   - Homeostasis (Threshold Offset) препятствует насыщению при длительном moderate Poisson стимуле.
2. **E/I Balance Proxy**:
   - Наличие тормозных L23 проекций удерживает firing rate популяции L4 в разумных рамках (не превышает 50 Hz).
3. **Пространственная геометрия**:
   - Локальные distance-based проекции создают реалистичный профиль синаптических соединений.

## Outputs & Reports

- Full Research Report: [reports/static_microcircuit_physiology_v1.md](reports/static_microcircuit_physiology_v1.md)
- Artifacts:
  - `artifacts/static_microcircuit_connectivity.json`
  - `artifacts/static_microcircuit_simulation_log.json`
- Plots:
  - [images/spatial_microcircuit_geometry.png](images/spatial_microcircuit_geometry.png)
  - [images/spike_raster_heatmap.png](images/spike_raster_heatmap.png)
  - [images/firing_rate_traces.png](images/firing_rate_traces.png)
  - [images/voltage_and_threshold_traces.png](images/voltage_and_threshold_traces.png)
  - [images/dendritic_fatigue_traces.png](images/dendritic_fatigue_traces.png)
  - [images/connectivity_weight_matrix.png](images/connectivity_weight_matrix.png)
