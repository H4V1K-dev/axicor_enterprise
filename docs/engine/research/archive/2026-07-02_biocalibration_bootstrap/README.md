# Архив этапа биологической калибровки (2026-07-02)
*(biocalibration-research-consolidation-v1)*

Этот каталог содержит детальные отчеты, материалы и промежуточные выводы начального этапа исследований (bootstrap) по калибровке AxiEngine на эталонных биологических нейронах Allen Institute.

## Индекс архивных отчетов

### 1. Эталонные биологические нейроны и сбор данных
*   [reference_neuron_audit.md](reference_neuron_audit.md) — Первичный аудит структуры биологических данных.
*   [reference_neuron_harvest_summary.md](reference_neuron_harvest_summary.md) — Сводка результатов сбора (harvest) параметров по популяциям клеток.
*   [reference_neuron_nwb_seed_probe_summary.md](reference_neuron_nwb_seed_probe_summary.md) — Исследование NWB файлов и структуры зондирования (seed probe).
*   [biological_calibration_pack_v1.md](biological_calibration_pack_v1.md) — Отчет по формированию первой версии пакета калибровки.
*   [Biological Reference Neuron Research.md](Biological%20Reference%20Neuron%20Research.md) — Общее описание исследований эталонных нейронов (EN).
*   [biological_reference_neuron_research_ru.md](biological_reference_neuron_research_ru.md) — Общее описание исследований эталонных нейронов (RU).
*   [biological_calibration_plan.md](biological_calibration_plan.md) — Исходный пошаговый план (roadmap) работ по биологической калибровке.


### 2. Калибровка одиночных нейронов
*   [single_neuron_calibration_probe_v1.md](single_neuron_calibration_probe_v1.md) — Первая итерация калибровки одиночного GLIF-нейрона.
*   [single_neuron_calibration_probe_v2.md](single_neuron_calibration_probe_v2.md) — Вторая итерация калибровки с разделением порогов.

### 3. Исследование нейрона 314900022 (Scnn1a)
*   [single_neuron_314900022_trace_match_v1.md](single_neuron_314900022_trace_match_v1.md) — Первое строгое trace-match сравнение с Allen.
*   [single_neuron_314900022_passive_first_v1.md](single_neuron_314900022_passive_first_v1.md) — Перекалибровка с приоритетом пассивных свойств (passive-first).
*   [single_neuron_314900022_balanced_v1.md](single_neuron_314900022_balanced_v1.md) — Оптимизация сбалансированного соответствия (balanced).
*   [single_neuron_314900022_membrane_sandbox_v1.md](single_neuron_314900022_membrane_sandbox_v1.md) — Песочница мембранной физики (RC_float vs RC_Q16).

### 4. Адаптивные механизмы и реплеи
*   [single_neuron_314900022_adaptive_leak_audit_v1.md](single_neuron_314900022_adaptive_leak_audit_v1.md) — Аудит влияния адаптивной утечки и гомеостаза.
*   [ephys_probe_01_replay_audit_v1.md](ephys_probe_01_replay_audit_v1.md) — Восстановление и реплей протокола EPHYS_PROBE_01.
*   [full_neuron_physics_ideas_v1.md](full_neuron_physics_ideas_v1.md) — Теоретические идеи расширения физики мембран.

---

## Архивные скрипты и инструменты

Исследовательские инструменты этого этапа сохранены в каталоге [scripts/](scripts/):
*   [allen_nwb_seed_probe.py](scripts/allen_nwb_seed_probe.py) — Скрипт парсинга NWB-файлов и извлечения признаков.
*   [allen_reference_harvest.py](scripts/allen_reference_harvest.py) — Скрипт массового сбора параметров популяций.
*   [trace_match_314900022.py](scripts/trace_match_314900022.py) — Базовый скрипт подбора параметров для trace-match.
*   [trace_match_314900022_balanced.py](scripts/trace_match_314900022_balanced.py) — Скрипт оптимизации сбалансированных параметров.
*   [trace_match_314900022_membrane_sandbox.py](scripts/trace_match_314900022_membrane_sandbox.py) — Песочница сравнения мембранных уравнений (GLIF vs RC).
*   [trace_match_314900022_adaptive_leak_probe.py](scripts/trace_match_314900022_adaptive_leak_probe.py) — Скрипт аудита адаптивной утечки и порогового гомеостаза.
*   [ephys_probe_01_replay_audit.py](scripts/ephys_probe_01_replay_audit.py) — Реплей и анализ протокола EPHYS_PROBE_01.

---

## Ссылки на связанные артефакты

Из этого расположения к ключевым файлам результатов ведут следующие пути:

*   **Калибровочный пакет**:
    *   [biological_calibration_pack_v1.csv](../../../../../artifacts/biological_calibration_pack_v1.csv)
    *   [biological_calibration_pack_v1.json](../../../../../artifacts/biological_calibration_pack_v1.json)
*   **Результаты trace-match 314900022**:
    *   [Сбалансированная сетка](../../../../../artifacts/single_neuron_314900022_balanced_grid.csv) | [Лучшие](../../../../../artifacts/single_neuron_314900022_balanced_best.csv)
    *   [Passive-first сетка](../../../../../artifacts/single_neuron_314900022_passive_first_grid.csv) | [Лучшие](../../../../../artifacts/single_neuron_314900022_passive_first_best.csv)
    *   [Сравнение моделей в песочнице](../../../../../artifacts/single_neuron_314900022_membrane_sandbox_model_comparison.csv)
    *   [Парные сравнения Float/Q16](../../../../../artifacts/single_neuron_314900022_membrane_sandbox_paired.csv)
*   **Результаты аудита адаптации и реплея**:
    *   [Сетка адаптивной утечки](../../../../../artifacts/single_neuron_314900022_adaptive_leak_grid.csv) | [Лучшие](../../../../../artifacts/single_neuron_314900022_adaptive_leak_best.csv)
    *   [Результаты реплея EPHYS_PROBE_01](../../../../../artifacts/ephys_probe_01_replay_summary.csv)
    *   [График реплея EPHYS_PROBE_01](images/ephys_probe_01_replay.png)
