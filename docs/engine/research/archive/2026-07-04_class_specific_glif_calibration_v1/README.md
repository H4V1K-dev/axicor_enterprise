# Archived Research: Class-Specific GLIF Calibration v1

Status: completed
Slug: `class_specific_glif_calibration_v1`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование исследует возможность вывода класс-специфичных априоров (`class-specific priors`) для калибровки GLIF_3 нейронов различных типов взамен единого глобального пресета:
- `L4_spiny`: Excitatory control class (Exact Allen bio target for 314900022)
- `L5_spiny`: Layer 5 pyramidal excitatory class (Single-profile qualitative target)
- `L23_aspiny`: Layer 2/3 aspiny interneuron-like class (Single-profile qualitative target)

## Class Inventory Summary

| Class Name | Representative Profile | Target Type | Candidate leak_shift | Candidate rest_potential (mV) | Candidate Penalty | Candidate Decay | Candidate Spikes 190pA | Class Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **L4_spiny** | `L4_spiny_VISl4_4` | Exact Allen Bio | **4** | **-70.0** | 1940 | 4 | 35 | class-specific priors supported |
| **L5_spiny** | `L5_spiny_VISp5_7` | Qualitative Class | **4** | **-76.0** | 1940 | 9 | 35 | single-profile qualitative only |
| **L23_aspiny** | `L23_aspiny_VISp23_218` | Qualitative Class | **2** | **-66.0** | 500 | 4 | 40 | single-profile qualitative only |

## Key Findings

1. **Класс-специфичные априоры поддержаны (class-specific priors supported)**:
   - Единая глобальная константа не может накрыть все слои. Различные пороговые потенциалы (`-45.6 mV` у L4, `-49.7 mV` у L5, `-55.4 mV` у L2/3) требуют разграничения калибровочных априоров.
2. **L4_spiny является наиболее сильным калиброванным классом**:
   - Априор `leak_shift = 4`, `rest = -70.0 mV`, `penalty = 1940`, `decay = 4` точно воссоздает поведение контрольного нейрона (0 спайков на 30–40 pA, 35 спайков на 190 pA, ISI growth = 2.05).
3. **L5_spiny и L23_aspiny квалифицированы как `single-profile qualitative only`**:
   - Для L5_spiny выведен априор (`leak_shift = 4`, `rest = -76.0 mV`, `penalty = 1940`, `decay = 9`), устраняющий ложные спайки 30–40 pA до 0 при удержании 35 спайков на 190 pA.
   - Для L23_aspiny выведен априор (`leak_shift = 2`, `rest = -66.0 mV`, `penalty = 500`, `decay = 4`), устраняющий ложные спайки до 0 при удержании 40 спайков на 190 pA. В частности, `rest = -66.0 mV` является значением кандидатного априора, которое нейробиологам нужно подтвердить отдельно при расширении выборки.
   - Из-за наличия всего 1 профиля в модернизированной библиотеке для данных классов производственная миграция остается отложенной (`needs biological target expansion`).

## Outputs & Reports

- Full Research Report: [reports/class_specific_calibration_v1.md](reports/class_specific_calibration_v1.md)
- Artifacts:
  - `artifacts/class_specific_glif_inventory.json`
  - `artifacts/class_specific_glif_baseline_replay.json`
  - `artifacts/class_specific_glif_passive_sweep.json`
  - `artifacts/class_specific_glif_passive_selected.json`
  - `artifacts/class_specific_glif_homeostasis_sweep.json`
  - `artifacts/class_specific_glif_homeostasis_selected.json`
  - `artifacts/class_specific_glif_ahp_refractory_sanity.json`
- Plots:
  - [images/baseline_vs_calibrated_fi_by_class.png](images/baseline_vs_calibrated_fi_by_class.png)
  - [images/false_low_spikes_by_profile.png](images/false_low_spikes_by_profile.png)
  - [images/spikes_190_by_profile.png](images/spikes_190_by_profile.png)
  - [images/isi_growth_by_profile.png](images/isi_growth_by_profile.png)
  - [images/selected_passive_params_distribution.png](images/selected_passive_params_distribution.png)
  - [images/selected_homeostasis_params_distribution.png](images/selected_homeostasis_params_distribution.png)
