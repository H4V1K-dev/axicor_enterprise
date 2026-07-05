# Baker Spatial Growth Audit v1

Status: completed audit / whitelist fixed, capacity warning

## Цель исследования
Провести изолированный аудит baker spatial growth: какие слои соединяются, какие projection пары доминируют, какие fan-in/fan-out distributions получаются, как выглядят расстояния/segment indices, и нет ли topology pathologies.

## Ключевой итог
- `VirtualInput` переведен в input-only режим через target whitelist: входящие синапсы на Virtual targets подавлены.
- Все 7 expected V1-like projections присутствуют, unexpected projections отсутствуют.
- Собрано 32,492 live synapses при 106,010 dropped candidates на seed 12345.
- Virtual layer имеет 100% zero-input — это ожидаемо для input-only слоя.
- L4 и L23 полностью насыщены (128/128 dendrite slots), L5 близок (mean 123.7).
- Fan-out остается неоднородным: zero-output источники есть во всех слоях (Virtual 9.4%, L4 20.3%, L23 7.8%, L5 25.0%).
- Seed variance мала: total synapses 31978..32492 (~±1%).

## Вердикт

Аудит успешно показал, что baker реально строит пространственный коннектом и сохраняет все expected projections. Корневая whitelist-ошибка исправлена: `VirtualInput` больше не принимает синапсы. Текущий статус — не блокер whitelist, а **capacity warning**: малый shard все еще насыщает L4/L23 dendrite slots, поэтому следующий functional replay можно запускать только с явной пометкой о saturation caveat.

## Воспроизведение

```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test full_neuron_replay run_baker_spatial_growth_audit_v1 -- --nocapture
```

## Структура папки
- `scripts/` — Python скрипт анализа topology.
- `images/` — 7 обязательных графиков topology.
- `reports/` — Итоговый научный отчёт аудита.
- `artifacts/` — JSON топологические артефакты и summary.
