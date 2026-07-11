# Growth v2 MVP Extraction

Status: finished / source audit
Started: 2026-07-06
Completed: 2026-07-06

## Question
Как устроен старый MVP Baker в `axicor-master/axicor-baker`, какие ключевые механики в нем присутствуют и отсутствуют в текущем Baker v1, как формализовать проблему "terminal knot" (образование избыточных завихрений на концах аксонов) и как безопасно заложить основы для новой ветки исследований Growth v2 без раздувания существующего тест-сьюта?

## Expectation
1. Будет выделен новый независимый файл интеграционных тестов `baker_growth_v2.rs` с флагами `baker-probe`.
2. Будет проведен детальный аудит исходного кода MVP Baker (`cone_tracing.rs`, `axon_growth.rs`, `spatial_grid.rs`, `dendrite_connect.rs`, `sprouting.rs`).
3. Будут предложены четкие математические и геометрические метрики для выявления terminal knot и алгоритмические методы борьбы с ними.
4. Будет подтверждена сборка нового тестового таргета с помощью легковесного inventory-теста.
5. Индекс исследований будет обновлен, а `Baker Functional Topology Replay` отложен до выработки решений по Growth v2.

## Inputs
- Исходный код legacy MVP Baker в `axicor-master/axicor-baker/src/bake/`.
- Текущая кодовая база `AxiEngine`.

## Method
1. Создать `AxiEngine/crates/test-harness/tests/baker_growth_v2.rs` с cfg-гейтами.
2. Написать легковесный тест `run_growth_v2_mvp_extraction_inventory`, записывающий JSON-инвентарь в `artifacts/growth_v2_mvp_extraction_inventory.json`.
3. Зафиксировать в отчете разницу в механике непрерывного и дискретного роста, проблемы обхода whitelist для legacy external axons и отличие старого cell-radius prefilter от текущего exact radius gate.

## Commands
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test baker_growth_v2 run_growth_v2_mvp_extraction_inventory -- --nocapture
```

## Outputs
- `reports/growth_v2_mvp_extraction.md` — подробный аудит и дизайн terminal knot метрик.
- `artifacts/growth_v2_mvp_extraction_inventory.json` — JSON-файл инвентаря.

## Result
Новый тестовый таргет успешно изолирован и собирается. Написан подробный аналитический отчет. Разработан дизайн метрик (tortuosity, local density, angle variance) и методов исправления (capture radius, damping, terminal straightening) для проблемы terminal knot в будущей Growth v2.

## Interpretation
Current Baker v1 является стабильным и геометрически корректным дискретным решением. MVP Baker содержит биологически релевантные непрерывные векторные механики, которые послужат основой для Growth v2, но его legacy-код содержит два риска, которые нельзя переносить вслепую: whitelist bypass для `soma_idx == usize::MAX` и cell-radius dendrite scan без финальной точной проверки расстояния. Growth v2 должен брать из MVP навигацию роста, но сохранять production exact radius gate и явную whitelist-политику.

## Next Step
Принятие решения по дизайну Growth v2: переносить ли непрерывный векторный рост с конусами или дорабатывать дискретный Baker v1.
