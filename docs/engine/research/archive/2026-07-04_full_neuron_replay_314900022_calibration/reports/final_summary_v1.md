# Final Research Summary Report: Full Neuron Replay 314900022

Status: closure_pending
Specimen: 314900022 (L4_spiny_VISl4_4)
Date: 2026-07-04

## Executive Summary

Данный отчёт подводит итоговое резюме биокалибровки одиночного нейрона спесимена `314900022` в рамках исследования `full_neuron_replay_314900022`.

Исследование состояло из 6 последовательных фаз и успешно решило ключевую проблему: устранило нефизичную ложную гипервозбудимость на малых токах (30–40 pA) без разрушения высокотоковой f-I кривой и биологической частотной адаптации (SFA).

---

## Что было протестировано и изменено

| Компонент / Фаза | Исходное значение (Baseline) | Калиброванное значение (Final Candidate) | Результат | Статус |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 4: Passive Membrane** | `leak_shift = 8`, `rest = -73.4 mV` | `leak_shift = 4`, `rest = -70.0 mV` | Снята гипервозбудимость 30–40 pA (spikes=0), f-I RMSE упал с 12.89 до 1.89 | **CONFIRMED** |
| **Phase 5: SFA Adaptation** | `penalty = 1940`, `decay = 4` | `penalty = 1940`, `decay = 4` | ISI Growth = 2.05 на 190 pA; f-I RMSE упал до **1.50** | **CONFIRMED** |
| **Phase 6: AHP / Refractory** | `ahp = 5000 uV`, `refr = 14 ticks` | `ahp = 5000 uV`, `refr = 14 ticks` | Null-result по AHP amplitude (5-8 mV RMSE=1.50); baseline retained by conservative tie-break | **RETAINED** |

---

## Итоговый калиброванный GLIF_3+ профиль

Для спесимена `314900022` зафиксирован следующий набор параметров:
- `leak_shift`: **4** (сила пассивной утечки)
- `rest_potential`: **-70000 uV** (-70.0 mV)
- `threshold`: **-45656 uV** (-45.656 mV)
- `homeostasis_penalty`: **1940**
- `homeostasis_decay`: **4**
- `ahp_amplitude`: **5000 uV** (5.0 mV)
- `refractory_period`: **14** (14 ms / 14 ticks)
- `adaptive_leak_gain`: **0** (отключен)
- `adaptive_mode`: **0**
- `heartbeat_m`: **0** (для чистого current-clamp replay)

---

## Основные выводы и ограничения (Remaining Risks)

1. **Биологическая согласованность**: GLIF_3+ модель воспроизводит 0 спайков на 30–40 pA, 4 спайка на 50 pA (реобаза) и 35 спайков на 190 pA (биологическая цель Allen: 36 спайков).
2. **Ограничения исследования (Risks)**:
   - **Single-specimen overfit**: Параметры подобраны строго под трассу спесимена 314900022.
   - **Current-clamp only**: Модель проверена на ступенчатых токах длительностью 1000 ms, без синаптического шума и in vivo флуктуаций.
   - **AHP Shape Metric**: Форма пост-спайкового сброса в GLIF_3 является структурным сбросом ядра; для более детального подбора формы спайка в будущем могут потребоваться метрики совмещения трасс по времени (trace-aligned kinetics).

---

## Следующий рекомендованный шаг

Завершить и архивировать исследование `full_neuron_replay_314900022`. Перенести методику калибровки на **Cross-Profile Validation** (популяционный набор профилей Allen Cell Types).
