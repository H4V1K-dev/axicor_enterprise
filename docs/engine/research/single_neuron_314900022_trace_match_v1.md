# Trace Match 1-к-1: Нейрон 314900022 (Scnn1a L4 Excitatory) - Hardened
*(single-neuron-314900022-trace-match-hardening-v1)*

Этот отчет представляет расширенные результаты строгого 1-к-1 сопоставления трасс симуляции одиночной GLIF-мембраны AxiEngine с экспериментальными sweep-данными клетки **314900022** (возбуждающий нейрон 4-го слоя зрительной коры, линия Scnn1a-Tg3-Cre).

## 1. Сводные показатели калибровки

| Метрика | Значение |
|:---|:---|
| **f-I RMSE (ошибка количества спайков)** | 4.5166 |
| **Ошибка реобазы** | 40.0 pA (Bio: 50.0 pA vs GLIF: 90.0 pA) |
| **Latency MAE (ошибка задержки спайка)** | 1.99 ms |
| **ISI MAE (ошибка межспайковых интервалов)** | 9.65 ms |
| **Ошибка адаптации ISI (Adaptation Error)** | 3.8776 |
| **Средняя ошибка пика пассивного отклика** | 4.52 mV |
| **Средняя ошибка steady-state пассивного отклика** | 6.44 mV |
| **Всего проанализировано свипов** | 13 |

## 2. Подетальный анализ по свипам

| Sweep ID | Ток (pA) | Спайки (Bio) | Спайки (GLIF) | Ошибка спайков | Bio Latency (ms) | GLIF Latency (ms) | Ошибка латентности (ms) | Ошибка пика (mV) | Steady-State Err (mV) | Voltage RMSE (mV) | Примечания |
|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|:---|
| 24 | -110.0 | 0 | 0 | 0 | n/a | n/a | n/a | 2.03 | -3.41 | 2.85 | Passive response only, no spikes. |
| 25 | -90.0 | 0 | 0 | 0 | n/a | n/a | n/a | -2.47 | -7.52 | 6.36 | Passive response only, no spikes. |
| 29 | -10.0 | 0 | 0 | 0 | n/a | n/a | n/a | 9.06 | 8.40 | 8.50 | Passive response only, no spikes. |
| 31 | 30.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | n/a | 21.61 | Subthreshold response, correct no-spike matching. |
| 40 | 40.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | n/a | 8.31 | Subthreshold response, correct no-spike matching. |
| 32 | 50.0 | 7 | 0 | -7 | 51.6 | n/a | n/a | n/a | n/a | 9.56 | 50 pA Threshold Sweep (Mixed bio response: 7 or 0 vs GLIF 0 spikes). |
| 41 | 50.0 | 0 | 0 | 0 | n/a | n/a | n/a | n/a | n/a | 4.76 | 50 pA Threshold Sweep (Mixed bio response: 7 or 0 vs GLIF 0 spikes). |
| 33 | 70.0 | 11 | 0 | -11 | 30.7 | n/a | n/a | n/a | n/a | 7.88 | Underexcited: biology fired but GLIF remained silent. |
| 34 | 90.0 | 20 | 22 | 2 | 22.1 | 21.0 | -1.1 | n/a | n/a | 30.43 | Active spikes matching (bio=20, sim=22). |
| 35 | 110.0 | 22 | 22 | 0 | 15.9 | 21.0 | 5.1 | n/a | n/a | 33.03 | Active spikes matching (bio=22, sim=22). |
| 36 | 130.0 | 26 | 28 | 2 | 12.9 | 13.0 | 0.1 | n/a | n/a | 35.74 | Active spikes matching (bio=26, sim=28). |
| 37 | 150.0 | 29 | 28 | -1 | 10.6 | 13.0 | 2.4 | n/a | n/a | 36.77 | Active spikes matching (bio=29, sim=28). |
| 39 | 190.0 | 36 | 31 | -5 | 7.7 | 9.0 | 1.3 | n/a | n/a | 40.69 | Active spikes matching (bio=36, sim=31). |
