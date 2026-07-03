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

## 3. Что сейчас известно

- **Эталонные данные есть**: создан пакет биологических признаков из Allen/NWB для дальнейшей калибровки.
- **Specimen 314900022 выбран как первый рабочий якорь**: по нему уже есть trace-match, passive-first, balanced, membrane sandbox и adaptive leak probes.
- **Homeostasis + adaptive leak + AHP выглядят перспективно**: в probe-режиме они дают лучший результат по SFA/f-I среди проверенных вариантов.
- **RC / membrane_v2 пока не обязательна**: RC улучшала отдельные метрики, но не дала очевидного выигрыша перед штатной адаптацией.
- **Мембранные probes были слишком узкими**: дальнейшие выводы должны строиться через full-neuron replay.

## 4. Живые гипотезы

| Гипотеза | Текущий уровень |
| :--- | :--- |
| Штатная адаптация AxiEngine способна дать биологически похожую SFA. | supported by probe, not confirmed |
| Главный конфликт одиночного нейрона связан не только с формулой мембраны, но и с полным tick-loop. | supported |
| DDS / спонтанное событие должно быть stateful и влиять на восстановление нейрона, а не быть бесплатным output-флагом. | hypothesis |
| Спайковая инерция от накопленного штрафа может дать уникальные отрицательные пики и лучшее восстановление. | hypothesis |
| Старые legacy-параметры роста и связности могут быть полезны как priors для будущих сетевых экспериментов. | deferred |

## 5. Ослабленные подходы

- **Homeostasis-free GLIF**: ослаблен, потому что без пороговой адаптации плохо воспроизводит форму разряда под длительным током.
- **Чистый brute force параметров**: отложен. Сначала нужен аудит полного нейронного цикла и понятные критерии, иначе перебор просто найдет красивую цифру без смысла.
- **Выводы только по membrane sandbox**: недостаточны. Они полезны для отладки математики, но не закрывают поведение нейрона.

## 6. Открытые вопросы

1. **Единицы и масштабы**: где именно production Rust использует микровольты, а где исследовательские scripts могли маскировать ошибки через mV.
2. **AHP и refractory shape**: должен ли нейрон восстанавливаться во время refractory или напряжение должно удерживаться плоско.
3. **DDS / spontaneous events**: это output-событие, внутренний разряд, шумовой recovery-kick или отдельный pacemaker-контур.
4. **Penalty-driven inertia**: как безопасно связать накопленный threshold offset с глубиной post-spike trough.
5. **EPHYS_PROBE_01**: какой именно старый контекст дал sawtooth-график с привыканием.
6. **Переход к популяции**: когда одиночный нейрон будет достаточно понятен, нужно проверить перенос на мини-сеть.

## 7. Активные и следующие исследования

### [Active] MVP CPU Replay v1 (`archive/_active/mvp_cpu_replay_v1/`)

- **Вопрос**: Можем ли изолированно воспроизвести MVP CPU tick-loop 1:1.
- **Зачем**: Нужен технический baseline перед изменением физики.
- **Что подтвердит**: Побитовое совпадение перенесенной логики с MVP-поведением на fixtures.
- **Что ослабит**: Расхождения в state planes, которые нельзя объяснить адаптацией контрактов.
- **Planned outputs**: README, test-only runner, parity tests, mismatch report.

### Следующий шаг

```text
full-neuron-replay-314900022-v1
```

Цель: прогнать 314900022 не через обрезанную мембранную песочницу, а через полный нейронный цикл с AHP, refractory, homeostasis, adaptive leak и будущими экспериментальными режимами DDS / inertia.

Ожидание: если probe-улучшения настоящие, full-neuron replay должен сохранить улучшение SFA/f-I и показать осмысленную форму восстановления после спайка. Если результат развалится, проблема находится не в подборе параметров, а в полном tick-loop.

## 8. Ключевые архивы

- [MVP CPU Replay v1 (Active)](archive/_active/mvp_cpu_replay_v1/README.md)
- [Legacy baseline import](archive/2026-07-01_legacy_baseline_import/README.md)
- [Biocalibration bootstrap](archive/2026-07-02_biocalibration_bootstrap/README.md)
- [Идеи полной физики нейрона](archive/2026-07-02_biocalibration_bootstrap/full_neuron_physics_ideas_v1.md)

## 9. Ключевые артефакты

### Базовые данные

- [biological_calibration_pack_v1.csv](../../../artifacts/biological_calibration_pack_v1.csv)
- [biological_calibration_pack_v1.json](../../../artifacts/biological_calibration_pack_v1.json)

### Specimen 314900022

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
