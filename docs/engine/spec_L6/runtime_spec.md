# spec_runtime

> Версия спеки: 2.2  
> Дата: 2026-07-10  

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `runtime` |
| **Слой** | Слой 6 — Runtime Orchestration & Node Startup (`L6`) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется `std` для работы с динамической памятью хоста |
| **Описание** | Оркестратор циклов симуляции (Day Loop & Night Phase) для одного шарда AxiEngine. В рамках дневного цикла крейт принимает вычислительный движок `compute::ShardEngine`, владеет счетчиком тиков времени и последовательно выполняет синхронные дневные батчи через `engine.run_day_batch`. Во время Ночной Фазы (Stage N) рантайм координирует переходы жизненного цикла (Day $\to$ Maintenance $\to$ Night $\to$ Day), экспортирует/импортирует VRAM состояние и вызывает `weaver-daemon` для перестройки коннектома. |

---

## §1.1. Этапы реализации (Staged Scope)

### Входит в Stage A:
1. **Владение движком**: Принятие и удержание владения над уже запущенным (`Running`) `compute::ShardEngine`.
2. **Владение временем**: Хранение и продвижение счетчика тиков симуляции `current_tick`.
3. **Диспетчеризация батчей**: Подготовка команды `compute_api::DayBatchCmd` и выполнение вычислений через синхронный вызов `engine.run_day_batch`.
4. **Управление DTO**: Использование структуры конфигурации `LocalRuntimeConfig`, отслеживание состояния `RuntimeState` и сбор `RuntimeStats`.

### Входит в Stage N (Night Phase Coordination):
1. **Оркестрация переходов**: Приостановка дневного цикла по условию `tick % night_interval == 0` и перевод движка в состояние `Maintenance`.
2. **Владение рабочей копией путей**: Хранение и передача мутабельного буфера трассировки путей (`paths_blob`) в составе `HostWorkingState`.
3. **Экспорт и импорт**: Выгрузка VRAM-данных в хост-буферы и последующий импорт модифицированных данных обратно в вычислительный бэкенд.
4. **Запуск биологических мутаций**: In-process вызов библиотеки `weaver-daemon` в режиме `HostSlices` (T8 вертикальный срез).
5. **Fail-Closed дисциплина**: Блокировка возврата из обслуживания в случае сбоя импорта или возврата демоном ошибки.

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `Tick`, `SomaFlags`, `MasterSeed` | Идентификация шагов времени, семена PRNG и флаги активности. |
| `layout` (Слой 1) | `NightWorkingViewMut`, `NightWorkingViewRef`, `StateOffsets` | Структуры представлений Ночной Фазы для обмена с weaver. |
| `compute-api` (Слой 3) | `DayBatchCmd`, `BatchResult`, `BackendMaintenanceMut`, `BackendMaintenanceRef` | Команды батча, результаты и DTO буферов фазы обслуживания. |
| `compute` (Слой 3) | `ShardEngine`, `ComputeError`, `LifecycleState` | Выполнение вычислений, управление фазой обслуживания. |
| `weaver-daemon` (Слой 4) | `run_night`, `NightBufferSource`, `WeaverReport`, `WeaverJobRequest` | Вызов биологического планировщика в режиме `HostSlices` (in-proc). |

> [!IMPORTANT]
> Настоящая спецификация запрещает крейту `runtime` зависеть от тяжелых сетевых транспортов (`net`, `transport`, `protocol`) и компилятора `baker`.
> Прямые алгоритмы пространственной геометрии, ранжирования или скоринга синапсов выполняются в `topology` и запрещены в `runtime`.

---

## §3. Публичная API-Модель

### §3.1. Структуры данных и Перечисления

```rust
/// Конфигурация локального оркестратора Дневного цикла шарда.
#[derive(Debug, Clone)]
pub struct LocalRuntimeConfig {
    pub sync_batch_ticks: u32,
    pub v_seg: u32,
    pub dopamine: i16,
    pub max_spikes_per_tick: u32,
    pub virtual_offset: u32,
    pub num_virtual_axons: u32,
    pub input_words_per_tick: u32,
    pub mapped_soma_ids: Vec<u32>,
    /// Интервал тиков между запусками Ночной Фазы (0 - отключено).
    pub night_interval: u64,
}

/// Состояния жизненного цикла рантайма.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Created,
    Running,
    /// Движок переведен в режим обслуживания для выполнения Ночной Фазы.
    Maintenance,
    Stopped,
    Faulted,
}

/// Параметры запуска Ночной Фазы.
#[derive(Debug, Clone)]
pub struct NightJobParams {
    pub night_epoch: u64,
    pub master_seed: [u8; 32],
    /// Порог прунинга (в типе i32, на этапе сборки рантайм проверяет >= 0 и конвертирует в u32).
    pub prune_threshold: i32,
    pub max_sprouts: u32,
    pub w_distance: u32,
    pub w_power: u32,
    pub w_explore: u32,
    pub initial_synapse_weight: i32,
}

/// Хост-ориентированные буферы и рабочие копии данных обслуживания.
pub struct HostWorkingState {
    /// Временный буфер для экспорта плоскостей соматического состояния сом и синапсов.
    pub state_blob: Vec<u8>,
    /// Временный буфер для экспорта буфера головок аксонов.
    pub axons_blob: Vec<u8>,
    /// Мутабельная рабочая копия путей аксонов (никогда не пишется обратно в .axic).
    pub paths_blob: Vec<u8>,
}
```

---

## §3.2. Возможные ошибки

```rust
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Compute execution error: {0}")]
    Compute(#[from] compute::ComputeError),

    #[error("Invalid state transition from {from:?} (expected {expected})")]
    InvalidState {
        from: RuntimeState,
        expected: &'static str,
    },

    #[error("Invalid engine lifecycle state: expected Running, found {actual:?}")]
    InvalidEngineLifecycle {
        actual: compute::LifecycleState,
    },

    #[error("Biological tick overflow: current={current}, sync={sync}")]
    TickOverflow {
        current: u64,
        sync: u32,
    },

    #[error("Invalid input buffer dimensions for {field}: expected {expected}, found {actual}")]
    InvalidInputDimensions {
        field: &'static str,
        expected: usize,
        actual: usize,
    },

    /// Ошибка при невалидных параметрах Ночной Фазы.
    #[error("Invalid prune threshold: {0} (must be >= 0)")]
    InvalidPruneThreshold(i32),

    /// Ошибка при выполнении шага биологической перестройки демоном.
    #[error("Weaver execution failed: {0}")]
    WeaverFailed(String),

    /// Ситуация, когда импорт модифицированных данных в VRAM завершился со сбоем.
    #[error("Engine import failed, memory segment is poisoned")]
    ImportPoisoned,
}
```

---

## §3.3. Публичный интерфейс оркестратора

```rust
pub struct LocalRuntime {
    engine: compute::ShardEngine,
    config: LocalRuntimeConfig,
    state: RuntimeState,
    stats: RuntimeStats,
    current_tick: u64,
    cached_output_spikes: Vec<u32>,
    cached_output_spike_counts: Vec<u32>,
    /// Хранилище рабочих буферов обслуживания Ночной Фазы
    working_state: HostWorkingState,
    /// Флаг отравления рантайма из-за сбоя импорта
    import_poisoned: bool,
}

impl LocalRuntime {
    /// Создает рантайм, инициализирует HostWorkingState на основе ShardEngine/AllocSpec.
    pub fn new(
        engine: compute::ShardEngine,
        config: LocalRuntimeConfig,
        // Исходные пути из boot загружаются во владение working_state
        initial_paths_blob: Vec<u8>,
    ) -> Result<Self, RuntimeError>;

    /// Выполняет один биологический батч с автоматическим триггером Ночной Фазы.
    pub fn run_batch(
        &mut self,
        input: RuntimeBatchInput<'_>,
    ) -> Result<RuntimeBatchReport, RuntimeError>;

    /// Явный ручной вызов перестройки коннектома (Ночной Фазы).
    pub fn run_night_phase(
        &mut self,
        params: NightJobParams,
    ) -> Result<weaver_daemon::WeaverReport, RuntimeError>;

    pub fn shutdown(&mut self) -> Result<(), RuntimeError>;
    pub fn stats(&self) -> RuntimeStats;
    pub fn state(&self) -> RuntimeState;
}
```

---

## §4. Жизненный цикл и Доменная логика (Lifecycle & Night boundary)

### §4.1. Автоматический триггер Ночной Фазы
В процессе вызова `run_batch` рантайм продвигает внутреннее время `current_tick`.
- Если `config.night_interval > 0` и `current_tick > 0` и `current_tick % config.night_interval == 0`:
  Рантайм приостанавливает дневной цикл и автоматически инициирует Ночную Фазу, вызывая `run_night_phase` с дефолтными/накопленными параметрами. При успешном завершении дневной цикл возобновляется.

### §4.2. Последовательность оркестрации Ночной Фазы (`run_night_phase`)
При вызове `run_night_phase` рантайм выполняет строго следующую последовательность шагов:

1. **Проверка состояния**: Рантайм обязан находиться в состоянии `RuntimeState::Running`. Любое другое состояние вызывает ошибку `RuntimeError::InvalidState`.
2. **Проверка параметров**: Порог `params.prune_threshold` проверяется на валидность. Если `prune_threshold < 0`, то вычисления немедленно останавливаются, рантайм переходит в состояние `RuntimeState::Faulted`, возвращая `RuntimeError::InvalidPruneThreshold`.
3. **Вход в обслуживание**: Вызывается `engine.enter_maintenance()`. Рантайм переходит в `RuntimeState::Maintenance`.
4. **Экспорт данных**: Рантайм подготавливает структуру `compute_api::BackendMaintenanceMut` со ссылками на буферы `state_blob` и `axons_blob` из состава `HostWorkingState`. Вызывается метод `engine.export_maintenance_state(maintenance_mut)`. Данные выгружаются из VRAM ускорителя в оперативную память хоста.
5. **Вызов планировщика (Weaver)**: Рантайм подготавливает `weaver_daemon::WeaverJobRequest` (сводя `prune_threshold` к `u32` в Mass Domain).
   Вызывается библиотечная функция `weaver_daemon::run_night` в режиме `HostSlices`:
   ```rust
   let source = weaver_daemon::NightBufferSource::HostSlices(layout::NightWorkingViewMut {
       padded_n: spec.padded_n,
       total_axons: spec.total_axons,
       total_ghosts: spec.total_ghosts,
       state_blob: &mut working_state.state_blob,
       axons_blob: &mut working_state.axons_blob,
       paths_blob: Some(&mut working_state.paths_blob),
       offsets: layout::compute_state_offsets(spec.padded_n),
   });
   let report = weaver_daemon::run_night(job_request, source)?;
   ```
6. **Импорт данных**: Рантайм подготавливает структуру `compute_api::BackendMaintenanceRef` со ссылками на измененные буферы `state_blob` и `axons_blob`. Вызывается метод `engine.import_maintenance_state(maintenance_ref)`.
7. **Fail-Closed дисциплина**:
   - Если импорт или запуск `run_night` завершается сбоем (`Err`), рантайм устанавливает флаг `import_poisoned = true`, переходит в состояние `RuntimeState::Faulted` и возвращает `RuntimeError::ImportPoisoned`.
   - При установленном флаге `import_poisoned` вызов `engine.exit_maintenance()` жестко блокируется, и рантайм никогда не возвращается в режим `Running`. Симуляция признается необратимо поврежденной.
8. **Выход из обслуживания**: При успешном импорте вызывается `engine.exit_maintenance()`. Рантайм переходит обратно в состояние `RuntimeState::Running` и готов к приему дневных батчей.

---

## §5. Обязательная Матрица Тестирования (Stage A & Stage N)

### Тесты этапа Stage A
1. **Инициализация рантайма (`test_runtime_stage_a_create_with_running_engine`)**: Создание рантайма с Running CPU-движком и проверка корректного перехода состояния в `Running`.
2. **Запрет батча после останова (`test_runtime_stage_a_reject_batch_after_shutdown`)**: Вызов `run_batch` после выполнения `shutdown()` приводит к ошибке `RuntimeError::InvalidState`.
3. **Продвижение тиков симуляции (`test_runtime_stage_a_tick_advancement`)**: Проверка, что после выполнения батча `current_tick` рантайма сдвигается ровно на значение `ticks_executed` из отчета.
4. **Валидация размеров входных буферов (`test_runtime_stage_a_invalid_input_lengths`)**: Передача некорректных размеров `incoming_spikes` или `input_bitmask` возвращает `RuntimeError::InvalidInputDimensions`.

### Тесты этапа Stage N
5. **Выполнение пустой Ночной Фазы (`test_runtime_stage_n_empty_night_success`)**: Проверка прохождения полной цепочки переходов `Day -> Maintenance -> Night -> Day` с вызовом `weaver_daemon::run_night` без мутаций (пустой прунинг).
6. **Браковка отрицательного порога прунинга (`test_runtime_stage_n_reject_negative_prune_threshold`)**: Передача `prune_threshold < 0` в `NightJobParams` на этапе `run_night_phase` немедленно блокирует старт, переводит рантайм в `Faulted` и возвращает `InvalidPruneThreshold`.
7. **Блокировка возобновления при сбое импорта (`test_runtime_stage_n_fail_closed_prevents_resume`)**: Имитация сбоя при вызове `import_maintenance_state`. Верификация перевода в `Faulted`, блокировки `exit_maintenance()` и запрета последующих запусков батчей.
8. **Изоляция изменений архива (`test_runtime_stage_n_never_mutates_axic_archive`)**: Проверка, что выполнение `run_night_phase` изменяет только буферы в памяти `HostWorkingState` и не выполняет запись в исходный RO файл `.axic`.
