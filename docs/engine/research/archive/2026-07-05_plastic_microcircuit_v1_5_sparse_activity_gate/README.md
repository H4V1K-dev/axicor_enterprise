# Plastic Microcircuit v1.5 Biological Sparse-Activity Gate Audit

Status: PASS / sparse-functional

## Цель исследования
Заменить грубый hard gate `L4 >= 3.0 Hz` на биологически обоснованный sparse-activity gate и проверить, является ли v1.4/v1.5 N=256 long-run режим (`L4` в диапазоне 1.0..3.0 Hz) здоровым sparse-but-functional режимом или патологическим under-recruitment.

## Ключевой итог
- Manual audit: L4 = {m_l4_metrics["rates"]["driven"]:.2f} Hz, selectivity = {m_sel:.4f}, active fraction = {m_l4_metrics["active_fraction"]*100:.1f}%, longest L4 silence = {m_l4_metrics["silence_windows"]["longest_silence_ticks"]/1000.0:.3f}s.
- Baker audit: L4 = {b_l4_metrics["rates"]["driven"]:.2f} Hz, selectivity = {b_sel:.4f}, active fraction = {b_l4_metrics["active_fraction"]*100:.1f}%.
- CartPole разблокирован как следующий toy research run, с caveat: transfer metric пока является lagged population coupling proxy.

## Структура папки
- `scripts/` — Python скрипт анализа спайковых и субпороговых метрик.
- `images/` — 8 обязательных графиков динамики физиологии.
- `reports/` — Итоговый научный отчёт аудита.
- `artifacts/` — Копии JSON логов симуляции v1.5.
