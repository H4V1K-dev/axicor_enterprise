# Balanced-калибровка одиночного нейрона 314900022

Цель прогона - проверить, можно ли текущей GLIF-математикой AxiEngine одновременно удержать пассивный отклик, реобазу, f-I кривую и отсутствие ложного молчания на активных sweep.

## Лучший найденный набор

| Параметр | Значение |
|:---|:---|
| leak_shift | 4 |
| current_scale | 0.035 |
| refractory_period | 24 ms |
| threshold | -41 mV |
| ahp_amplitude | 10 mV |
| loss | 60.0100 |

## Scoreboard

| Метрика | Значение |
|:---|:---|
| Passive RMSE | 5.9298 mV |
| Passive steady-state error | 6.4432 mV |
| f-I RMSE | 4.3359 spikes |
| Bio rheobase | 50.0 pA |
| Sim rheobase | 70.0 pA |
| Rheobase error | 20.0 pA |
| False silent sweeps | 1 |
| False silent missing spikes | 7 |
| False positive sweeps | 0 |
| False positive spikes | 0 |
| Subthreshold false spikes | 0 |
| Latency MAE | 5.4950 ms |
| ISI MAE | 12.2161 ms |
| ISI adaptation error | 3.4615 |

## Candidate comparison

| Кандидат | loss | passive RMSE | f-I RMSE | sim rheobase | false silent | false positive |
|:---|---:|---:|---:|---:|---:|---:|
| overall_best | 60.0100 | 5.9298 | 4.3359 | 70.0 | 1 | 0 |
| exact_rheobase_best | 101.1724 | 12.2710 | 6.5422 | 50.0 | 0 | 1 |
| no_false_silent_best | 101.1724 | 12.2710 | 6.5422 | 50.0 | 0 | 1 |
| good_passive_rheobase_le_70_best | 60.0100 | 5.9298 | 4.3359 | 70.0 | 1 | 0 |

## Sweep table

| Sweep | pA | Bio spikes | Sim spikes | Error | Bio latency | Sim latency | Passive peak err | Passive SS err | Voltage RMSE |
|:---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 24 | -110.0 | 0 | 0 | 0 | n/a | n/a | 2.03 | -3.41 | 2.82 |
| 25 | -90.0 | 0 | 0 | 0 | n/a | n/a | -2.47 | -7.52 | 6.47 |
| 29 | -10.0 | 0 | 0 | 0 | n/a | n/a | 9.06 | 8.40 | 8.50 |
| 31 | 30.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | 6.49 |
| 40 | 40.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | 8.09 |
| 32 | 50.0 | 7 | 0 | -7 | 51.58 | n/a | n/a | n/a | 11.67 |
| 41 | 50.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | 11.90 |
| 33 | 70.0 | 11 | 20 | 9 | 30.74 | 23.00 | n/a | n/a | 30.03 |
| 34 | 90.0 | 20 | 25 | 5 | 22.09 | 12.00 | n/a | n/a | 34.37 |
| 35 | 110.0 | 22 | 25 | 3 | 15.89 | 12.00 | n/a | n/a | 35.88 |
| 36 | 130.0 | 26 | 28 | 2 | 12.91 | 9.00 | n/a | n/a | 37.82 |
| 37 | 150.0 | 29 | 31 | 2 | 10.63 | 6.00 | n/a | n/a | 40.56 |
| 39 | 190.0 | 36 | 32 | -4 | 7.70 | 5.00 | n/a | n/a | 42.73 |

## Вывод

Balanced-прогон все еще показывает конфликт: параметры, которые держат пассивный отклик, не дают корректно стартовать спайкам на части биологически активных sweep. Это указывает не только на подбор конфига, но и на возможное ограничение текущей формулы мембраны/масштабирования тока.
