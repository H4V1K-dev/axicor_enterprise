# spec_runtime

> Версия спеки: 2.1  
> Дата: 2026-07-01  
> Статус: Approved / Ready for Implementation (Stage A: Single-Shard Local Day Loop)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `runtime` |
| **Слой** | Слой 6 — Runtime Orchestration & Node Startup (`L6`) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется `std` для работы с динамической памятью хоста |
| **Описание** | Локальный управляющий оркестратор дневного цикла для одного шарда (Single-Shard Local Day Loop) симуляции AxiEngine. В рамках Stage A крейт принимает предварительно инициализированный и запущенный вычислительный движок `compute::ShardEngine`, владеет счетчиком тиков времени симуляции, последовательно выполняет синхронные дневные батчи через `engine.run_day_batch`, управляет хост-буферами входов/выходов, накапливает статистику и обеспечивает безопасное завершение работы движка при останове. |

---

## §1.1. Scope of Stage A (Границы реализации Stage A)

### Входит в Stage A:
1. **Владение движком**: Принятие и удержание владения над уже запущенным (`Running`) `compute::ShardEngine`.
2. **Владение временем**: Хранение и продвижение счетчика тиков симуляции `current_tick`.
3. **Диспетчеризация батчей**: Подготовка команды `compute_api::DayBatchCmd` и выполнение вычислений через синхронный вызов `engine.run_day_batch`.
4. **Управление DTO**: Использование структуры конфигурации `LocalRuntimeConfig`, отслеживание состояния `RuntimeState` и сбор `RuntimeStats`.
5. **Владение буферами выгрузки**: Рантайм сам владеет внутренними векторами для выходных спайков, ресайзя их размер под максимальный лимит батча.
6. **Безопасная очистка (Teardown)**: Вызов метода `teardown`/освобождения ресурсов движка при выгрузке.

### НЕ входит в Stage A (Отложено / Deferred):
1. **Запуск воркер-потоков**: Создание OS-потоков (`std::thread`) и каналов обмена (`crossbeam`).
2. **Мультишардовое распределение**: Координация и обмен спайками между несколькими шардами.
3. **Сетевое взаимодействие**: Чтение/отправка спайковых пакетов в сеть, опрос событий `NetRuntime`, RCU-маршруты.
4. **Межпроцессная IPC/SHM синхронизация**: Атомарный автомат `ShmStateMachine`, буферы обмена `Swapchain`.
5. **Ночная фаза симуляции**: Сигнализирование и координация перестройки с `weaver-daemon`.
6. **Восстановление и Чекпоинты**: Прямая интеграция со службой чекпоинтов VRAM и диска.
7. **Загрузка архивов**: Вызов `boot` или `vfs` для разбора TOC архивов `.axic`.

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `Tick` | Идентификация шагов биологического времени симуляции. |
| `compute` (Слой 3) | `ShardEngine`, `ComputeError`, `LifecycleState` | Выполнение биологических вычислений на хосте/ускорителе, контроль состояния движка. |
| `compute-api` (Слой 3) | `DayBatchCmd`, `BatchResult`, validation functions | Описание команд батча, результатов вычислений и функции валидации. |

> [!IMPORTANT]
> Настоящая спецификация Stage A запрещает крейту `runtime` зависеть в production-зависимостях от:
> `boot`, `baker`, `vfs`, `config`, `topology`, `ipc`, `net`, `wire`, `protocol`, `transport`, `node`.
> Крейт `boot` разрешается подключать исключительно в качестве `dev-dependency` для сборки тестовых архивов в юнит-тестах.

---

## §3. Публичная API-Модель Stage A

### §3.1. Структуры данных и Перечисления

```rust
/// Конфигурация локального оркестратора Дневного цикла шарда.
#[derive(Debug, Clone)]
pub struct LocalRuntimeConfig {
    /// Шаг тиков симуляции, выполняемый за один батч.
    pub sync_batch_ticks: u32,
    /// Количество сегментов на один аксон.
    pub v_seg: u32,
    /// Базовый уровень концентрации допамина в биологической среде.
    pub dopamine: i16,
    /// Максимальное число спайков, обрабатываемое за один тик.
    pub max_spikes_per_tick: u32,
    /// Число виртуальных/входных аксонов.
    pub num_virtual_axons: u32,
    /// Количество слов маски входных каналов на один тик.
    pub input_words_per_tick: u32,
    /// Отображение soma_id в глобальные индексы.
    pub mapped_soma_ids: Vec<u32>,
}

/// Состояния жизненного цикла рантайма.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    /// Рантайм создан, но симуляция не запущена.
    Created,
    /// Вычислительный цикл активен и готов к выполнению батчей.
    Running,
    /// Рантайм безопасно остановлен, движок выгружен.
    Stopped,
    /// Аварийное состояние из-за ошибки вычислений на GPU/CPU.
    Faulted,
}

/// Накапливаемая статистика работы.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RuntimeStats {
    /// Текущий тик биологического времени.
    pub current_tick: u64,
    /// Количество успешно выполненных батчей.
    pub batches_executed: u64,
    /// Суммарное количество обработанных тиков.
    pub ticks_executed: u64,
    /// Количество биологических спайков, сгенерированных шардом.
    pub generated_spikes: u64,
    /// Количество записанных выходных спайков во внешние структуры.
    pub output_spikes_written: u64,
    /// Количество отброшенных спайков из-за переполнения лимитов.
    pub dropped_spikes: u64,
    /// Счетчик произошедших ошибок вычислений.
    pub compute_errors: u64,
}

/// Временный заимствованный пакет входных сигналов на батч.
pub struct RuntimeBatchInput<'a> {
    /// Опциональная маска активности входных каналов (размер: sync_batch_ticks * input_words_per_tick).
    pub input_bitmask: Option<&'a [u32]>,
    /// Буфер индексов входящих спайков.
    pub incoming_spikes: Option<&'a [u32]>,
    /// Количество спайков на каждый тик батча (размер: sync_batch_ticks).
    pub incoming_spike_counts: &'a [u32],
}

/// Отчет о результатах выполнения батча.
pub struct RuntimeBatchReport {
    /// Низкоуровневый результат вычислений из compute-api.
    pub batch_result: compute_api::BatchResult,
    /// Выходные индексы сгенерированных спайков биологических нейронов.
    pub output_spikes: Vec<u32>,
    /// Количество выходных спайков по тикам батча.
    pub output_spike_counts: Vec<u32>,
    /// Стартовый тик времени, с которого начался батч.
    pub tick_base: u64,
    /// Количество тиков, фактически рассчитанных в батче.
    pub ticks_executed: u32,
}
```

### §3.2. Возможные ошибки

```rust
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Ошибка при вызове методов вычислительного движка.
    #[error("Compute execution error: {0}")]
    Compute(#[from] compute::ComputeError),

    /// Вызов операции в недопустимом состоянии жизненного цикла.
    #[error("Invalid state transition from {from:?} (expected {expected})")]
    InvalidState {
        /// Текущее состояние рантайма.
        from: RuntimeState,
        /// Ожидаемое состояние для вызова.
        expected: &'static str,
    },

    /// Ошибка при передаче движка с неподходящим жизненным циклом.
    #[error("Invalid engine lifecycle state: expected Running, found {actual:?}")]
    InvalidEngineLifecycle {
        /// Текущее состояние движка.
        actual: compute::LifecycleState,
    },

    /// Ошибка переполнения биологических тиков.
    #[error("Biological tick overflow: current={current}, sync={sync}")]
    TickOverflow {
        /// Текущее биологическое время.
        current: u64,
        /// Количество добавляемых тиков.
        sync: u32,
    },

    /// Ошибка валидации размеров входных массивов до отправки на расчет.
    #[error("Invalid input buffer dimensions for {field}: expected {expected}, found {actual}")]
    InvalidInputDimensions {
        /// Имя некорректного входного поля.
        field: &'static str,
        /// Ожидаемая размерность.
        expected: usize,
        /// Фактический размер буфера.
        actual: usize,
    },
}
```

### §3.3. Публичный интерфейс оркестратора

```rust
/// Локальный оркестратор дневного цикла для одного шарда.
pub struct LocalRuntime {
    engine: compute::ShardEngine,
    config: LocalRuntimeConfig,
    state: RuntimeState,
    stats: RuntimeStats,
    current_tick: u64,
    // Внутренние буферы для переиспользования аллокаций
    cached_output_spikes: Vec<u32>,
    cached_output_spike_counts: Vec<u32>,
}

impl LocalRuntime {
    /// Создает рантайм, принимая Running движок и конфигурацию.
    pub fn new(
        engine: compute::ShardEngine,
        config: LocalRuntimeConfig,
    ) -> Result<Self, RuntimeError>;

    /// Выполняет один биологический батч с входными сигналами.
    pub fn run_batch(
        &mut self,
        input: RuntimeBatchInput<'_>,
    ) -> Result<RuntimeBatchReport, RuntimeError>;

    /// Выполняет один батч без входных сигналов (холостой тик).
    pub fn run_empty_batch(&mut self) -> Result<RuntimeBatchReport, RuntimeError>;

    /// Идемпотентно останавливает рантайм, вызывая teardown движка.
    pub fn shutdown(&mut self) -> Result<(), RuntimeError>;

    /// Возвращает снимок накопленной статистики.
    pub fn stats(&self) -> RuntimeStats;

    /// Возвращает текущее состояние жизненного цикла.
    pub fn state(&self) -> RuntimeState;
}
```

---

## §4. Жизненный цикл и Доменная логика (Lifecycle & Buffers)

1. **Инициализация**:
   Конструктор `LocalRuntime::new` принимает движок `ShardEngine`, который обязан находиться в рабочем состоянии `engine.state() == compute::LifecycleState::Running`. Если состояние иное, возвращается `RuntimeError::InvalidEngineLifecycle`. При успехе рантайм переходит в состояние `RuntimeState::Running`, устанавливает `current_tick = 0` и сбрасывает статистику.
2. **Владение буферами**:
   Для исключения накладных расходов от постоянных аллокаций в горячем цикле симуляции, `LocalRuntime` содержит внутренние векторы `cached_output_spikes` и `cached_output_spike_counts`. Перед запуском каждого батча рантайм ресайзит и обнуляет их:
   - `cached_output_spikes` принудительно изменяет размер до `sync_batch_ticks * max_spikes_per_tick`.
   - `cached_output_spike_counts` изменяет размер до `sync_batch_ticks`.
   Эти буферы передаются в методы выгрузки движка. После расчёта в результирующий DTO `RuntimeBatchReport` возвращается owned copy (копия) актуально выгруженных данных.
3. **Продвижение времени**:
   Перед сборкой `DayBatchCmd` рантайм обязан проверить, не приводит ли запуск вычислений к переполнению тиков времени: `current_tick + sync_batch_ticks` не должно превышать `u64::MAX`. При угрозе переполнения возвращается `RuntimeError::TickOverflow` без вызова вычислительного ядра. При успешном выполнении батча значение `current_tick` инкрементируется на `ticks_executed` (берется из `BatchResult`). Статистика пополняется счетчиками выполненных батчей, тиков, сгенерированных и отброшенных спайков.
4. **Валидация размерностей входов**:
   Рантайм выполняет проверку соответствия размеров входных массивов перед сборкой команды:
   - Если предоставлен `input_bitmask`, его длина должна быть строго равна `sync_batch_ticks * input_words_per_tick`.
   - Длина `incoming_spike_counts` должна быть строго равна `sync_batch_ticks`.
   - Каждый элемент `incoming_spike_counts[tick]` должен быть `<= max_spikes_per_tick`.
   - Если `incoming_spikes = Some(slice)`, длина `incoming_spikes` должна быть не меньше `sync_batch_ticks * max_spikes_per_tick`.
   - Если `incoming_spikes = None`, все `incoming_spike_counts` должны быть равны 0.
   При нестыковке возвращается `RuntimeError::InvalidInputDimensions` без паники процесса. (Правило жесткого совпадения длины с суммой `incoming_spike_counts` не используется, так как `DayBatchCmd` принимает плоский tick-major буфер фиксированного объема).
5. **Аварийный переход**:
   При возврате движком `ComputeError`, рантайм увеличивает счетчик `compute_errors` статистики, переводит состояние в `RuntimeState::Faulted` и возвращает ошибку наверх. Дальнейшие попытки запуска батчей в состоянии `Faulted` пресекаются возвратом `RuntimeError::InvalidState`.
6. **Graceful Shutdown**:
   Метод `shutdown` переводит рантайм в состояние `RuntimeState::Stopped` и освобождает ресурсы движка. Схема переходов:
   - `Running` $\to$ `shutdown()` вызывает `engine.teardown()`. При успехе состояние переходит в `Stopped`.
   - `Faulted` $\to$ `shutdown()` также пытается вызвать `engine.teardown()`. При успехе состояние переходит в `Stopped`.
   - Если `teardown()` возвращает ошибку, рантайм остается (или переходит) в состоянии `Faulted` и возвращает ошибку `RuntimeError::Compute`.
   - Если рантайм находится в состоянии `Stopped`, вызов `shutdown()` является no-op и успешно возвращает `Ok(())`.

---

## §5. Обязательная Матрица Тестирования Stage A

1. **Инициализация рантайма (`test_runtime_stage_a_create_with_running_engine`)**: Создание рантайма с Running CPU-движком и проверка корректного перехода состояния в `Running`, а также сброса тиков.
2. **Запрет батча после останова (`test_runtime_stage_a_reject_batch_after_shutdown`)**: Попытка вызвать `run_batch` после выполнения `shutdown()` приводит к ошибке `RuntimeError::InvalidState`.
3. **Продвижение тиков симуляции (`test_runtime_stage_a_tick_advancement`)**: Проверка, что после выполнения батча `current_tick` рантайма сдвигается ровно на значение `ticks_executed` из отчета.
4. **Накопление статистики (`test_runtime_stage_a_stats_accumulation`)**: Выполнение серии батчей должно приводить к накоплению соответствующих значений в `RuntimeStats` (число тиков, батчей, спайков).
5. **Выполнение пустого батча (`test_runtime_stage_a_empty_batch_creation`)**: Проверка, что `run_empty_batch()` корректно инициализирует команду `DayBatchCmd` с пустыми входами и возвращает успешный отчет.
6. **Валидация размеров входных буферов (`test_runtime_stage_a_invalid_input_lengths`)**: Передача некорректных размеров `incoming_spikes` или `input_bitmask` возвращает `RuntimeError::InvalidInputDimensions` и не приводит к панике.
7. **Авария вычислений (`test_runtime_stage_a_compute_error_to_faulted`)**: Имитация сбоя вычислений движка (например, возврат ошибки VRAM или некорректной разметки). Проверка перехода рантайма в `Faulted` и запрета последующих вызовов.
8. **Идемпотентность shutdown (`test_runtime_stage_a_shutdown_idempotency`)**: Многократный вызов `shutdown()` подряд на одном и том же рантайме возвращает `Ok(())` без сбоев и паник.
9. **Архитектурная чистота (`test_runtime_stage_a_no_forbidden_production_dependencies`)**: Проверка Cargo.toml: в списке `dependencies` отсутствуют любые крейты кроме `compute`, `compute-api`, `types` и `thiserror`.
10. **Проверка жизненного цикла движка при создании (`test_runtime_stage_a_invalid_initial_engine_state`)**: Передача в `LocalRuntime::new` движка в состоянии `LifecycleState::Created` или `Stopped` возвращает ошибку `RuntimeError::InvalidEngineLifecycle`.
11. **Проверка переполнения тиков (`test_runtime_stage_a_tick_overflow_prevention`)**: Установка `current_tick` близко к лимиту и запуск батча возвращает `RuntimeError::TickOverflow` без отправки задач в движок.
