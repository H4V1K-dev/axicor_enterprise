# Research Archive: Plastic Microcircuit v1.0 GSOP/STDP Spatial Weight Formation

Status: completed / partial
Slug: `plastic_microcircuit_v1_0_gsop_spatial_weight_formation`
Started: 2026-07-05
Completed: 2026-07-05

## Overview

Это исследование подтверждает включение правил пластичности GSOP и STDP на сбалансированной микросети v1.4, но не закрывает доказательство положительного пространственного укрепления:
- Проведены 9,000 tick sanity симуляции на N=256 и N=512, а также 50,000 tick learning симуляция на N=256.
- Подтвержден слабый корреляционный bias `Virtual -> L4`: коррелированные входы депрессируются меньше фоновых (+0.0686 uV), но средняя дельта остается отрицательной.
- Проверены и удовлетворены все структурные инварианты (Dale's Law, отсутствие sign flips).

## Key Findings

1. **GSOP/STDP Active**: Веса меняются, инварианты сохранены.
2. **Physiological Safety**: Сеть сохраняет устойчивость, runaway/silence не возникают.
3. **Pathway Formation Partial**: Положительная потенциация коррелированных пространственных дорожек пока не доказана; нужен v1.1 перед CartPole.

## Reports & Outputs

- Full Report: [reports/plastic_microcircuit_v1_0_gsop_spatial_weight_formation.md](reports/plastic_microcircuit_v1_0_gsop_spatial_weight_formation.md)
- Plots: [images/](images/)
