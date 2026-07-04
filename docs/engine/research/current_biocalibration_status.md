# Текущая карта биологической калибровки AxiEngine

Status: active research index, not a final report.

Этот файл является короткой картой исследований. Он не должен превращаться в очередной большой отчет. Подробности, скрипты, картинки и сырые выводы живут в датированных папках внутри [archive/](archive/). Правила ведения исследований описаны в [RULES.md](RULES.md).

## 1. Общая цель

Свести поведение нейрона AxiEngine с реальным биологическим нейроном настолько близко, насколько позволяют текущая физика и доступные данные.

Главный принцип текущей ветки: проверять не только мембранную формулу, а полный нейронный цикл:

- входной ток и синаптический ток;
- обычная и адаптивная утечка;
- AHP;
- refractory;
- threshold offset;
- homeostasis penalty / decay;
- DDS / спонтанные события;
- финализация спайка и output-события.

Мембранные probes остаются полезным микроскопом, но не считаются доказательством поведения нейрона целиком.

## 2. Завершенные этапы

| Этап | Статус | Короткий итог |
| :--- | :--- | :--- |
| [2026-07-01 legacy baseline import](archive/2026-07-01_legacy_baseline_import/README.md) | archived | Просканирована legacy-библиотека, зафиксированы правила импорта и риски. Legacy-параметры полезны как стартовые гипотезы, но не как финальная биологическая истина. |
| [2026-07-02 biocalibration bootstrap](archive/2026-07-02_biocalibration_bootstrap/README.md) | archived | Собраны Allen/NWB данные, сделаны первые калибровочные пакеты, probes по 314900022, adaptive leak и EPHYS replay. Получены сильные сигналы, но полный нейронный контур еще не закрыт. |
| [2026-07-04 biology metrics verification](archive/2026-07-04_biology_metrics_verification/README.md) | archived | Мигрированы каноничные профили (VISl4, VISp5, VISp23), проведена длинная симуляция (1,000,000 тиков). Подтверждено плановое поведение спонтанной и синаптической физики (CV, LV, STA, усталость). |
| [2026-07-04 full neuron replay 314900022](archive/2026-07-04_full_neuron_replay_314900022/README.md) | archived | Выполнен полный нейронный replay с потиковым паритетом Python/Rust. Изучены AHP, рефрактерность, homeostasis, Bounded Inertia и Heartbeat Gating. Выявлено: Bounded Inertia не решает гипервозбудимость на малых токах; Heartbeat Gating устраняет рефрактерные коллизии; gated_discharge — единственный biophysical кандидат для продакшна. |

## 3. Что сейчас известно

- **Эталонные данные есть**: создан пакет биологических признаков из Allen/NWB для дальнейшей калибровки.
- **Specimen 314900022 выбран как первый рабочий якорь**: по нему уже есть trace-match, passive-first, balanced, membrane sandbox и adaptive leak probes.
- **Пассивная утечка `leak_shift = 4` решает гипевозбудимость на 30-40 pA**: снижение `leak_shift` с 8 до 4 при `rest = -70 mV` устраняет нефизичные спайки на 30 и 40 pA (spikes_30=0, spikes_40=0), сохраняя 35 спайков на 190 pA и улучшая Allen f-I RMSE с 12.89 до 1.89.
- **SFA / Homeostasis калибровка (Phase 5)**: при `leak_shift = 4`, `rest = -70 mV` подбор `homeostasis_penalty = 1940`, `homeostasis_decay = 4` дает устойчивую частотную адаптацию (ISI Growth Ratio = 2.05 на 190 pA) и дальнейшее улучшение f-I RMSE до 1.50.
- **AHP / Refractory калибровка (Phase 6)**: AHP sweep оказался weakly informative (5000..8000 uV даёт идентичный f-I RMSE 1.50). Базовые параметры `ahp_amplitude = 5000 uV` и `refractory_period = 14 ticks` удержаны (baseline retained; no improvement found) по биологическому априору ~5 mV и принципу минимального отклонения.
- **RC / membrane_v2 пока не обязательна**: RC улучшала отдельные метрики, но не дала очевидного выигрыша перед штатной адаптацией.
- **Мембранные probes были слишком узкими**: выводы зафиксированы через full-neuron replay.

## 4. Живые гипотезы

| Гипотеза | Текущий уровень |
| :--- | :--- |
| Корректировка пассивной утечки (`leak_shift = 4`) приводит реобазу нейрона к биологическому порогу (~50 pA) без сложной адаптивной математики. | confirmed |
| Штатная адаптация AxiEngine (`homeostasis_penalty=1940, decay=4`) способна дать биологически похожую SFA (ISI growth 2.05). | confirmed |
| Пост-спайковый сброс (`ahp_amplitude=5000 uV`, `refractory_period=14`) обеспечивает правдоподобную форму спайка и AHP глубину (~5.0 mV). | retained / supported by conservative tie-break |
| Главный конфликт одиночного нейрона связан не только с формулой мембраны, но и с полным tick-loop. | supported |
| DDS / спонтанное событие должно быть stateful и начислять гомеостатический штраф (`gated_discharge`). | supported (plausible candidate) |
| Спайковая инерция от накопленного штрафа может улучшить восстановление на низких частотах. | weakened (ineffective at low frequencies) |
| Старые legacy-параметры роста и связности могут быть полезны как priors для будущих сетевых экспериментов. | deferred |

## 5. Ослабленные подходы

- **Bounded Spike Inertia (shift 3-5)**: ослаблена/отклонена для подавления гипервозбудимости на низких токах, так как релаксация порогового смещения между спайками делает инерционный сдвиг нулевым.
- **Heartbeat Production Control (без gating)**: ослаблен/отклонен, так как допускает генерацию спонтанных спайков во время рефрактерного периода, искажая ISI.
- **Heartbeat Gated (без discharge)**: классифицирован как diagnostic / free-spike control, так как генерирует спайки без AHP-сброса и рефрактерности.
- **Homeostasis-free GLIF**: ослаблен, потому что без пороговой адаптации плохо воспроизводит форму разряда под длительным током.
- **Чистый brute force параметров**: отложен. Сначала нужен аудит полного нейронного цикла и понятные критерии.
- **Выводы только по membrane sandbox**: недостаточны. Они полезны для отладки математики, но не закрывают поведение нейрона.

## 6. Открытые вопросы

1. **Единицы и масштабы**: где именно production Rust использует микровольты, а где исследовательские scripts могли маскировать ошибки через mV.
2. **AHP и refractory shape**: должен ли нейрон восстанавливаться во время refractory или напряжение должно удерживаться плоско.
3. **DDS / spontaneous events**: детализация спецификации `gated_discharge` для перевода в production CPU ядра.
4. **Переход к популяции**: когда одиночный нейрон достаточно понятен, проверить перенос на мини-сеть.

## 7. Лестница сетевых исследований

Следующий блок работ идет строго по gate-лестнице. CartPole и reward-задачи не запускаются, пока сеть не пройдет физиологические sanity-гейты.

| Порядок | Исследование | Статус | Gate для перехода дальше |
| :--- | :--- | :--- | :--- |
| 1 | **Single-cell calibration anchor** | completed | Есть воспроизводимый GLIF_3/current-clamp якорь: passive membrane, SFA/homeostasis, AHP/refractory sanity и class-specific priors без production migration. |
| 2 | **Static microcircuit physiology** | completed | Маленькая L4/L2-3/L5 сеть без пластичности не уходит в silence/runaway, показывает осмысленные firing rates, E/I balance, fatigue, spatial connectivity и визуализируемую геометрию. |
| 2.1 | **Static microcircuit scale-up** | completed | Оценена стабильность и CPU производительность при масштабировании до 1,000,000 нейронов. Выявлена Vm saturation (> -25mV) из-за избыточного homeostasis offset под Poisson-шумом. Физиология inconclusive. |
| 3 | **Plastic microcircuit** | blocked on step 2.1 | GSOP/STDP/fatigue включаются только после статической сетевой стабильности; веса должны оставаться bounded, коррелированные пути усиливаться, шумовые не разрушать сеть. |
| 4 | **Sensorimotor toy / CartPole** | blocked on step 3 | CartPole запускается только после microcircuit physiology + plasticity sanity, чтобы не смешивать ошибки topology, кодирования сенсоров, reward и моторного декодера. |

### [Next] Static Microcircuit v1.1 Input Scale & E/I Ablation

- **Вопрос**: Можно ли устранить Vm saturation и избыточный homeostasis offset за счет снижения весов Poisson-входов и введения жестких Vm/homeostasis ограничений (hard gates)?
- **Зачем**: Убедиться в физиологической стабильности мембраны перед STDP.
- **Первый scope**: CPU/test-harness; уменьшенный Poisson drive; ablation L23 прогон; расчет фазовой селективности.
- **Gate**: Vm и threshold_offset остаются стабильными; торможение L23 снижает firing rate L4/L5; количественно подтверждена phase selectivity.

## 8. Активные и следующие исследования

### [Completed] Static Microcircuit Scale-Up v1 (`archive/2026-07-04_static_microcircuit_scale_up_v1/`)

- **Вопрос**: Переносится ли статическая L4/L2-3/L5 микросеть с 64 нейронов на существенно больший размер без silence/runaway, без перегрева homeostasis threshold и без деградации CPU tick-loop?
- **Итоговый вердикт (Performance Passed / Physiology Inconclusive)**: CPU симулятор в release-сборке успешно масштабируется до 1,000,000 нейронов со 128 миллионами синапсов (около 8.8 секунды на тик). Однако физиология признана **inconclusive** из-за перегрева гомеостаза и Vm saturation (> -25mV) под сильным шумом.
- **Следующий шаг**: Исследование `Static Microcircuit v1.1 Input Scale & E/I Ablation` для стабилизации Vm.
- **Outputs**: Rust runner (`run_static_microcircuit_scale_up_experiments`), Python скрипты анализа и визуализации, отчёт [static_microcircuit_scale_up_v1.md](archive/2026-07-04_static_microcircuit_scale_up_v1/reports/static_microcircuit_scale_up_v1.md).

### [Completed] Static Microcircuit Physiology v1 (`archive/2026-07-04_static_microcircuit_physiology_v1/`)

- **Вопрос**: Дают ли откалиброванные одиночные GLIF-профили устойчивую пространственную сеть без обучения и reward?
- **Итоговый вердикт (Static Network Physiology Sanity Passed)**: Откалиброванные параметры leak, rest и homeostasis обеспечивают стабильное функционирование сети (без ухода в silence или runaway excitation), с выраженным E/I балансом и нормальной динамикой синаптического утомления (fatigue). Все приемочные гейты успешно пройдены.
- **Следующий шаг**: Переход к `GSOP STDP Plasticity` на базе этой структуры.
- **Outputs**: Rust runner (`run_static_microcircuit_physiology_experiments`), Python скрипты анализа и визуализации, отчёт [static_microcircuit_physiology_v1.md](archive/2026-07-04_static_microcircuit_physiology_v1/reports/static_microcircuit_physiology_v1.md).

### [Completed] Class-Specific GLIF Calibration v1 (`archive/2026-07-04_class_specific_glif_calibration_v1/`)

- **Вопрос**: Можно ли вывести устойчивые class-specific priors для разных типов нейронов (`L4_spiny`, `L5_spiny`, `L23_aspiny`) взамен единого глобального пресета?
- **Итоговый вердикт (Partial Success / Class-Specific Priors Supported)**: Класс-специфичные априоры поддержаны. L4_spiny удержан как точный калиброванный класс (`4/-70.0 mV`, `1940/4`). L5_spiny и L23_aspiny получили качественные кандидаты (`4/-76.0 mV` и `2/-66.0 mV`), устраняющие ложную гипервозбудимость (0 спайков), но имеют статус `single-profile qualitative only`.
- **Следующий шаг**: Сбор биологических NWB мишеней для L5 и L2/3 профилей перед производственной миграцией (`needs biological target expansion`).
- **Outputs**: Rust runner (`run_class_specific_glif_calibration_experiments`), Python скрипты анализа и визуализации, отчёт [class_specific_calibration_v1.md](archive/2026-07-04_class_specific_glif_calibration_v1/reports/class_specific_calibration_v1.md).

### [Completed] Cross-Profile Validation of GLIF Hierarchy v1 (`archive/2026-07-04_cross_profile_glif_hierarchy_v1/`)

- **Вопрос**: Переносится ли 2-этапная иерархия калибровки GLIF_3 (`passive` -> `homeostasis`, с `AHP deferred/sanity`) на другие канонические профили репозитория (`L4_spiny_VISl4_4`, `L5_spiny_VISp5_7`, `L23_aspiny_VISp23_218`)?
- **Итоговый вердикт (Partial Success / Class-Specific Calibration Required)**: Иерархический метод калибровки полностью валидирован как верный workflow (ликвидирует 100% ложной 30–40 pA гипервозбудимости без провала 190 pA отклика). Однако единый глобальный пресет не накрывает все слои из-за различий пороговых потенциалов (L4 `-45.6 mV`, L5 `-49.7 mV`, L2/3 `-55.4 mV`).
- **Следующий шаг**: Разработка исследований класс-специфичной калибровки (`class-specific calibration research`) отдельно для слоев L5_spiny и L23_aspiny. Никакой производственной миграции на данном этапе не проводится.
- **Outputs**: Rust runner (`run_cross_profile_glif_hierarchy_experiments`), Python скрипты анализа и визуализации, отчёт [cross_profile_validation_v1.md](archive/2026-07-04_cross_profile_glif_hierarchy_v1/reports/cross_profile_validation_v1.md).

### [Completed] Single-Specimen Biocalibration 314900022 (`archive/2026-07-04_full_neuron_replay_314900022_calibration/`)

- **Вопрос**: Каков итоговый calibrated GLIF_3+ профиль для specimen 314900022 после подбора пассивной утечки (Phase 4), SFA (Phase 5) и аудита AHP/рефрактерности (Phase 6)?
- **Итоговый вердикт**: Исследование успешно выполнено. Снижение `leak_shift` с 8 до 4 при `rest = -70 mV` устранило ложную 30–40 pA гипервозбудимость (Phase 4). Подбор `homeostasis_penalty = 1940`, `decay = 4` зафиксировал биологичную SFA (ISI growth 2.05) и снизил Allen f-I RMSE с 12.89 до 1.50 (Phase 5). Phase 6 показала null-result по `ahp_amplitude` (retained `ahp_amplitude=5000 uV`, `refractory_period=14 ticks` по принципу minimal-change).
- **Следующий шаг**: Перенос методологии на Cross-Profile Validation (популяционный suite из нескольких профилей Allen Cell Types).
- **Outputs**: Rust runner (`run_full_neuron_replay_phase6_experiments`), Python скрипты анализа и визуализации, отчёты Phase 4–6 и итоговый [final_summary_v1.md](archive/2026-07-04_full_neuron_replay_314900022_calibration/reports/final_summary_v1.md).

### [Completed] Full Neuron Replay 314900022 v1 (`archive/2026-07-04_full_neuron_replay_314900022/`)

- **Вопрос**: Переносится ли калибровочный выигрыш membrane/adaptive probes на production CPU tick-loop и экспериментальные гипотезы (inertia, heartbeat gating).
- **Зачем**: Это gate перед сетевыми и microcircuit-экспериментами.
- **Что подтвердило**: Потиковый паритет Rust с Python; Homeostasis — главный драйвер SFA; Heartbeat Gating устраняет рефрактерные коллизии; Gated Discharge — единственный biophysical кандидат. Bounded Inertia ослаблена на низких частотах.
- **Outputs**: Rust test-runner (`full_neuron_replay.rs`), Python скрипты анализа и визуализации, детальные отчеты v1 в архиве.

### [Completed] Biological Physics Verification (`archive/2026-07-04_biology_metrics_verification/`)

- **Вопрос**: Соответствует ли поведение новой CPU-физики (Gradient Synaptic Fatigue и Stochastic Heartbeat) реальным биологическим показателям при калибровке на каноничных профилях?
- **Зачем**: Подтвердить корректность интеграции Leak, AHP, пороговой динамики и синаптической усталости на длинной симуляции (1,000,000 тиков).
- **Что подтвердило**: Реалистичные частоты спонтанного спайкирования (VISl4: 1.03 Hz, VISp5: 0.96 Hz, VISp23: 3.98 Hz) с CV/LV ~1.0. Под Poisson-шумом в 50 Hz получен регулярный эмерджентный разряд с CV ~0.15-0.31, синаптической усталостью 76-83% и плавными пост-спайковыми STA-профилями.
- **Outputs**: Скрипт миграции, интеграционный тест-раннер, отчет в архиве.

### [Completed] GSOP STDP Fatigue v1 (`archive/gsop_stdp_fatigue_v1/`)

- **Вопрос**: Можем ли изолированно воспроизвести MVP CPU tick-loop 1:1.
- **Зачем**: Нужен технический baseline перед изменением физики.
- **Что подтвердит**: Побитовое совпадение перенесенной логики с MVP-поведением на fixtures.
- **Что ослабит**: Расхождения в state planes, которые нельзя объяснить адаптацией контрактов.
- **Planned outputs**: README, test-only runner, parity tests, mismatch report.

## 9. Ключевые архивы

- [Static Microcircuit Scale-Up v1](archive/2026-07-04_static_microcircuit_scale_up_v1/README.md)
- [Static Microcircuit Physiology v1](archive/2026-07-04_static_microcircuit_physiology_v1/README.md)
- [Single-Specimen Biocalibration 314900022](archive/2026-07-04_full_neuron_replay_314900022_calibration/README.md)
- [Full Neuron Replay 314900022 v1](archive/2026-07-04_full_neuron_replay_314900022/README.md)
- [Biological Physics Verification](archive/2026-07-04_biology_metrics_verification/README.md)
- [GSOP STDP Fatigue v1](archive/gsop_stdp_fatigue_v1/README.md)
- [Legacy baseline import](archive/2026-07-01_legacy_baseline_import/README.md)
- [Biocalibration bootstrap](archive/2026-07-02_biocalibration_bootstrap/README.md)
- [Идеи полной физики нейрона](archive/2026-07-02_biocalibration_bootstrap/full_neuron_physics_ideas_v1.md)

## 10. Ключевые артефакты

### Базовые данные

- [biological_calibration_pack_v1.csv](../../../artifacts/biological_calibration_pack_v1.csv)
- [biological_calibration_pack_v1.json](../../../artifacts/biological_calibration_pack_v1.json)

### Static Microcircuit

- [static_microcircuit_scale_up_summary.json](../../../artifacts/static_microcircuit_scale_up_summary.json)
- [static_microcircuit_connectivity.json](../../../artifacts/static_microcircuit_connectivity.json)
- [static_microcircuit_simulation_log.json](../../../artifacts/static_microcircuit_simulation_log.json)

### Specimen 314900022

- [Phase 4 static sweep](../../../artifacts/full_neuron_replay_314900022_phase4_static_sweep.json)
- [Phase 4 winner 190 pA trace](../../../artifacts/full_neuron_replay_314900022_phase4_trace_candidate_190.csv)
- [balanced best](../../../artifacts/single_neuron_314900022_balanced_best.csv)
- [passive-first best](../../../artifacts/single_neuron_314900022_passive_first_best.csv)
- [membrane sandbox comparison](../../../artifacts/single_neuron_314900022_membrane_sandbox_model_comparison.csv)
- [adaptive leak best](../../../artifacts/single_neuron_314900022_adaptive_leak_best.csv)

### EPHYS replay

- [ephys_probe_01_replay_summary.csv](../../../artifacts/ephys_probe_01_replay_summary.csv)
- [ephys_probe_01_replay_trace.csv](../../../artifacts/ephys_probe_01_replay_trace.csv)

## 11. Визуальные ориентиры

### Adaptive leak probe

![Adaptive leak probe](archive/2026-07-02_biocalibration_bootstrap/images/single_neuron_314900022_adaptive_leak_probe.png)

### EPHYS replay

![EPHYS replay](archive/2026-07-02_biocalibration_bootstrap/images/ephys_probe_01_replay.png)
