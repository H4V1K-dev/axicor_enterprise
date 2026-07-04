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
- **Homeostasis + adaptive leak + AHP выглядят перспективно**: в probe-режиме они дают лучший результат по SFA/f-I среди проверенных вариантов.
- **RC / membrane_v2 пока не обязательна**: RC улучшала отдельные метрики, но не дала очевидного выигрыша перед штатной адаптацией.
- **Мембранные probes были слишком узкими**: выводы зафиксированы через full-neuron replay.

## 4. Живые гипотезы

| Гипотеза | Текущий уровень |
| :--- | :--- |
| Корректировка пассивной утечки (`leak_shift = 4`) приводит реобазу нейрона к биологическому порогу (~50 pA) без сложной адаптивной математики. | confirmed |
| Штатная адаптация AxiEngine способна дать биологически похожую SFA. | supported |
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

## 7. Активные и следующие исследования

### [Active] Phase 4 Rheobase Calibration 314900022 (`archive/_active/full_neuron_replay_314900022/`)

- **Вопрос**: Можно ли изолированно подобрать параметры пассивной утечки (`leak_shift` и `rest_potential`) для устранения нефизичной гипервозбудимости на малых токах (30–40 pA) без ухудшения высокотокового отклика на 190 pA?
- **Зачем**: Убрать фальшивые спайки на малых токах и приблизить реобазу нейрона к биологическому порогу (~50 pA).
- **Что подтвердило**: `leak_shift = 4` и `rest_potential = -70 mV` полностью устраняют спайки на 30 pA и 40 pA (`spikes_30 = 0`, `spikes_40 = 0`), сохраняют 35 спайков на 190 pA и снижают Allen f-I RMSE с 12.89 до 1.89. Кандидат прошёл Acceptance Gate.
- **Outputs**: Rust runner (`run_full_neuron_replay_phase4_experiments`), Python скрипт `rheobase_leak_rest_calibration.py`, отчёт [rheobase_leak_rest_calibration_v1.md](archive/_active/full_neuron_replay_314900022/reports/rheobase_leak_rest_calibration_v1.md).

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

## 8. Ключевые архивы

- [Phase 4 Rheobase Calibration 314900022](archive/_active/full_neuron_replay_314900022/README.md)
- [Full Neuron Replay 314900022 v1](archive/2026-07-04_full_neuron_replay_314900022/README.md)
- [Biological Physics Verification](archive/2026-07-04_biology_metrics_verification/README.md)
- [GSOP STDP Fatigue v1](archive/gsop_stdp_fatigue_v1/README.md)
- [Legacy baseline import](archive/2026-07-01_legacy_baseline_import/README.md)
- [Biocalibration bootstrap](archive/2026-07-02_biocalibration_bootstrap/README.md)
- [Идеи полной физики нейрона](archive/2026-07-02_biocalibration_bootstrap/full_neuron_physics_ideas_v1.md)

## 9. Ключевые артефакты

### Базовые данные

- [biological_calibration_pack_v1.csv](../../../artifacts/biological_calibration_pack_v1.csv)
- [biological_calibration_pack_v1.json](../../../artifacts/biological_calibration_pack_v1.json)

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

## 10. Визуальные ориентиры

### Adaptive leak probe

![Adaptive leak probe](archive/2026-07-02_biocalibration_bootstrap/images/single_neuron_314900022_adaptive_leak_probe.png)

### EPHYS replay

![EPHYS replay](archive/2026-07-02_biocalibration_bootstrap/images/ephys_probe_01_replay.png)
