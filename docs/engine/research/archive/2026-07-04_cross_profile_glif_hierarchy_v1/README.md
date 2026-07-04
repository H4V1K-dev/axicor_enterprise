# Archived Research: Cross-Profile Validation of GLIF Calibration Hierarchy v1

Status: completed
Slug: `cross_profile_glif_hierarchy_v1`
Started: 2026-07-04
Completed: 2026-07-04

## Overview

Это исследование проверяет переносимость иерархии калибровки GLIF_3 (`Stage C1: Passive Membrane` -> `Stage C2: Homeostasis/SFA`, с `Stage C3: AHP Deferred/Sanity Artifact`), разработанной на спесимене `314900022`, на другие канонические профили репозитория:
- `L4_spiny_VISl4_4` (Control, L4 Spiny Excitatory, Exact Allen Bio Target)
- `L5_spiny_VISp5_7` (Layer 5 Spiny Pyramidal Excitatory, Qualitative Target)
- `L23_aspiny_VISp23_218` (Layer 2/3 Aspiny Inhibitory Interneuron, Qualitative Target)

## Profile Inventory Summary

| Profile Name | Class | Target Status | Threshold (uV) | Rest Baseline (uV) | Calib Leak | Calib Rest (mV) | Calib Penalty | Calib Decay | AHP Stage Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **L4_spiny_VISl4_4** | L4 Spiny Excitatory | Exact Allen Target | -45656 | -73443 | **4** | **-70.0** | 1940 | 4 | Deferred / Sanity Artifact |
| **L5_spiny_VISp5_7** | L5 Pyramidal Excitatory | Qualitative Class | -49718 | -71105 | **4** | **-73.0** | 1940 | 4 | Deferred / Sanity Artifact |
| **L23_aspiny_VISp23_218** | L2/3 Aspiny Interneuron | Qualitative Class | -55406 | -73862 | **2** | **-68.0** | 500 | 4 | Deferred / Sanity Artifact |

## Key Findings

1. **Вердикт исследования: Partial Success / Class-Specific Calibration Required**:
   - 2-этапный иерархический подход (`Passive C1` -> `Homeostasis C2`) убирает малотоковую гипервозбудимость без убоя высокотокового разряда.
   - Однако единый глобальный пресет не накрывает все слои: для каждого класса требуется индивидуальный пассивный сдвиг и гомеостатическое пенальти.
2. **Контрольный профиль `L4_spiny_VISl4_4`**:
   - `leak_shift = 4`, `rest_potential = -70000 uV`, `homeostasis_penalty = 1940`, `decay = 4` дают полное совпадение с биотреком Allen (0 спайков на 30–40 pA, 35 спайков на 190 pA, ISI growth = 2.05).
3. **Различие пороговых потенциалов между слоями**:
   - Из-за различий встроенных пороговых потенциалов (-45656 uV для L4, -49718 uV для L5, -55406 uV для L2/3) калибровка должна опираться на класс-специфичные априоры (class-specific priors).
4. **Статус AHP (Stage C3)**:
   - Сгенерирован как sanity артефакт, но не оптимизируется отдельно в данном исследовании (`deferred`).

## Outputs & Reports

- Full Research Report: [reports/cross_profile_validation_v1.md](reports/cross_profile_validation_v1.md)
- Artifacts:
  - `artifacts/cross_profile_glif_inventory.json`
  - `artifacts/cross_profile_glif_baseline_replay.json`
  - `artifacts/cross_profile_glif_passive_sweep.json`
  - `artifacts/cross_profile_glif_homeostasis_sweep.json`
  - `artifacts/cross_profile_glif_ahp_refractory_sweep.json`
- Plots:
  - [images/baseline_vs_calibrated_fi.png](images/baseline_vs_calibrated_fi.png)
  - [images/false_low_current_spikes_comparison.png](images/false_low_current_spikes_comparison.png)
  - [images/sfa_isi_growth_comparison.png](images/sfa_isi_growth_comparison.png)
