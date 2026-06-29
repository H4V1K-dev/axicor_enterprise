# spec_runtime

> Версия спеки: 2.0  
> Дата: 2026-06-29  
> Статус: Draft (Architecture Pass 1 - Refined)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `runtime` |
| **Слой** | Слой 6 — Runtime Orchestration (`L6` / Управляющий оркестратор) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется `std` для управления OS-потоками (`std::thread`), каналами коммуникации и системной памятью |
| **Описание** | Системный оркестратор AxiEngine, отвечающий за координацию жизненного цикла симуляции. Крейт управляет продвижением биологического времени (`Tick`), запуском и синхронизацией изолированных OS-потоков вычислительных шардов (`ShardWorker`), обработкой событий сетевого рантайма (`NetRuntime`), а также фазовыми переходами Дневного и Ночного циклов. Крейт координирует переходы Ночной фазы через атомарный автомат состояний разделяемой памяти (`ShmStateMachine`), используя переданный абстрактный интерфейс управления `WeaverControl`. Крейт не выполняет сокетных I/O вызовов сетевого уровня, не парсит конфигурации TOML, не делает прямых вызовов C-FFI видеокарты, не выполняет фрагментацию сетевых пакетов и не пишет чекпоинты напрямую на диск. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `Tick`, `MasterSeed` | Работа с биологическим временем ноды и инициализация детерминированных ГПСЧ для генерации шума. |
| `compute` (Слой 3) | `ShardEngine`, `DayBatchCmd`, `BatchResult`, `ComputeError` | Оркестрация вычислений на GPU/CPU через фасады вычислительных шардов. |
| `ipc` (Слой 2) | `ShmStateMachine`, `InputSwapchain`, `OutputSwapchain`, `ShmState`, `IpcError` | Межпроцессная синхронизация буферов обмена и координация переходов Ночной фазы. |
| `net` (Слой 5) | `NetRuntime`, `NetConfig`, `RouteUpdateOp`, `NeighborStatus`, `NetError`, `NetEvent` | Маршрутизация пакетов, опрос высокоуровневых сетевых событий и обновление маршрутов. |

> [!IMPORTANT]
> Настоящая спецификация категорически запрещает крейту `runtime` зависеть от сетевого транспорта (`transport`), L7-протокола (`protocol`), бинарных DTO-пакетов (`wire`), компиляторов (`baker`), контейнеров (`vfs`) и геометрических алгоритмов (`topology`). Взаимодействие со смежными компонентами происходит исключительно через высокоуровневые интерфейсы `ShardEngine`, `NetRuntime`, `ShmStateMachine` и `WeaverControl`. Любые зависимости от `layout` ослаблены; рантайм не валидирует выравнивания и смещения полей в SHM.

### §2.2. Зависимые Компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `node` (Слой 6) | Исполняемый демон (bin) ноды. Настраивает аппаратное окружение (`boot`), запускает рантайм (`runtime`) и координирует системные сигналы ОС (SIGINT/SIGTERM) для безопасной остановки. |

### §2.3. Внешние Зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `crossbeam` | `=0.8.4` | Быстрые lock-free bounded каналы (`crossbeam::channel`) для передачи команд (`ShardCommand`) и результатов воркеров. |
| `thiserror` | `=1.0.69` | Декларативная типизация системных ошибок рантайма (`RuntimeError`). |
| `tracing` | `=0.1.40` | Логирование переходов состояний и сбоев. |

### §2.4. Feature Flags

Секция feature flags не используется. Крейт собирается монолитно.

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`runtime`** | **Управление жизненным циклом и расписанием**: Продвижение тиков (`Tick`), переключение высокоуровневых состояний симуляции, жизненный цикл OS-потоков шардов (`ShardWorker`), координация фаз Дневного и Ночного циклов, обработка высокоуровневых сетевых событий `NetEvent`. | Запрещены прямые аллокации VRAM, сокетные вызовы физического I/O, чтение UDP/TCP пакетов, прямая мутация SoA-плоскостей в mmap, выполнение OS-level `kill` процессов демона и прямая запись чекпоинтов на диск. |
| **`compute`** | **Жизненный цикл вычислений шарда**: Обертка GPU контекстов, выполнение математики симуляции на GPU/CPU. | Запрещено продвижение глобального времени и реакция на сетевые эпохи. |
| **`net`** | **Маршруты и Сетевые события**: Таблица маршрутов RCU, BSP барьеры кластера, сборка/нарезка L7 пакетов. | Запрещено управление OS-потоками симуляции шардов и логика Дневного/Ночного циклов. |
| **`ipc`** | **Буферы SHM и Swapchain**: Жизненный цикл общих сегментов памяти и атомарное переключение буферов. | Запрещен контроль запуска вычислительного цикла. |

---

## §4. Публичная API-Модель (Public API Model)

Публичный интерфейс предоставляет структуры для конфигурации рантайма, управления рабочими потоками шардов, обработки событий и команд:

```rust
use std::time::Duration;
use types::Tick;
use net::{NetRuntime, RouteUpdateOp};
use ipc::ShmStateMachine;
use compute::{ShardEngine, DayBatchCmd, BatchResult};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub zone_hash: u32,
    pub tick_interval: Duration,
    pub night_interval_ticks: u64,
    pub warmup_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFault {
    DaemonTimeout,
    GpuLost,
    UnstableWarmup,
    ChannelDisconnected,
    NetworkBarrierFailure,
    PoisonedSharedState,
    InvalidStateTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeState {
    Created,
    Starting,
    RunningDay,
    RecoveryWarmup { ticks_elapsed: u64 },
    WaitingNetworkBarrier,
    NightPrepare,
    NightRunning,
    NightCommit,
    Draining,
    Shutdown,
    Faulted { reason: RuntimeFault },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    StateChanged { from: RuntimeState, to: RuntimeState },
    TickCommitted { tick: Tick },
    FastForwardRequested { target_epoch: u32 },
    NeighborStatusChanged { neighbor_id: u32, status: NeighborStatus },
    CheckpointRequested { tick: Tick },
    CheckpointCompleted { id: u64 },
    SystemFault { reason: RuntimeFault },
    NightPhaseCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCommand {
    StartSimulation,
    RequestPause,
    RequestShutdown,
    TriggerNightPhase,
    RequestCheckpoint,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RuntimeStats {
    pub committed_ticks: u64,
    pub completed_batches: u64,
    pub failed_batches: u64,
    pub night_runs: u64,
    pub night_timeouts: u64,
    pub worker_panics: u64,
    pub recovery_warmup_runs: u64,
    pub dropped_outbound_during_warmup: u64,
    pub compute_errors: u64,
    pub net_errors: u64,
    pub ipc_errors: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Runtime state machine fault: {0:?}")]
    SystemFault(RuntimeFault),
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: RuntimeState, to: RuntimeState },
}

/// Абстрактный интерфейс управления внешним процессом Ночной фазы
pub trait WeaverControl: Send + Sync {
    /// Проверить, запущен ли демон Ночной фазы
    fn is_active(&self) -> bool;
    /// Отправить сигнал к началу перестройки связей
    fn signal_night_start(&self) -> Result<(), String>;
    /// Запросить принудительную остановку демона
    fn request_daemon_shutdown(&self);
}

/// Сообщение, отправляемое в OS-поток шарда (ShardWorker)
#[derive(Debug)]
pub enum ShardCommand {
    /// Выполнить шаг вычислений Дневной фазы
    RunDayBatch {
        batch_cmd: DayBatchCmd,
    },
    /// Применить дельту структурных изменений, полученных в Ночную фазу
    ApplyNightDelta,
    /// Корректно завершить работу вычислительного ядра на стороне потока-владельца
    Shutdown,
}

/// Результат выполнения команды в ShardWorker
#[derive(Debug)]
pub enum ShardResult {
    /// Шаг Дневной фазы успешно завершен
    BatchCompleted {
        tick: Tick,
        result: BatchResult,
    },
    /// Патчи успешно применены к VRAM
    NightDeltaApplied,
    /// Подтверждение корректного завершения работы вычислительного ядра
    TeardownComplete,
    /// Ошибка вычислений на GPU/CPU
    Error(String),
}

/// Дескриптор управления потоком ShardWorker
pub struct ShardWorkerHandle {
    pub shard_id: u32,
    pub cmd_tx: crossbeam::channel::Sender<ShardCommand>,
    pub result_rx: crossbeam::channel::Receiver<ShardResult>,
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

/// Главный оркестратор рантайма ноды
pub struct RuntimeOrchestrator {
    config: RuntimeConfig,
    state: RuntimeState,
    tick_counter: Tick,
    stats: RuntimeStats,
    shard_workers: Vec<ShardWorkerHandle>,
    net_runtime: NetRuntime,
    shm_state: ShmStateMachine,
    weaver_control: Box<dyn WeaverControl>,
}

impl RuntimeOrchestrator {
    /// Инициализировать оркестратор на основе подготовленных boot-ресурсов
    pub fn new(
        config: RuntimeConfig,
        net_runtime: NetRuntime,
        shm_state: ShmStateMachine,
        weaver_control: Box<dyn WeaverControl>,
    ) -> Self;

    /// Запустить инициализацию рабочих потоков шардов
    pub fn initialize_workers(
        &mut self,
        engines_or_plan: ShardInitMode,
    ) -> Result<(), RuntimeError>;

    /// Выполнить одну итерацию управляющего цикла (Non-blocking step)
    pub fn step(&mut self) -> Result<Vec<RuntimeEvent>, RuntimeError>;

    /// Отправить высокоуровневую команду управления
    pub fn process_command(&mut self, cmd: RuntimeCommand) -> Result<(), RuntimeError>;

    /// Безопасно остановить все потоки и ресурсы (Graceful Teardown)
    pub fn shutdown(&mut self) -> Result<(), RuntimeError>;

    pub fn get_state(&self) -> RuntimeState;
    pub fn get_stats(&self) -> RuntimeStats;
    pub fn get_current_tick(&self) -> Tick;
}
```

### §4.1. Две модели управления жизненным циклом вычислительного ядра (Thread Affinity Ownership)

Спецификация оставляет открытым выбор между двумя моделями инициализации вычислительных ядер GPU/CPU (см. Open Debt §8):

*   **Модель A (Send Compute)**: `boot` возвращает полностью инициализированные экземпляры `ShardEngine` (они реализуют маркерный трейт `Send`). Оркестратор `runtime` принимает `Vec<ShardEngine>` и перемещает (moves) их в создаваемые OS-потоки `ShardWorker`.
*   **Модель B (Thread-Affine Compute)**: Вычислительное ядро жестко привязано к конкретному OS-потоку хоста и не может быть создано в одном потоке, а использовано в другом. `boot` возвращает легковесную структуру `ShardBootPlan`. Оркестратор запускает OS-потоки воркеров, передает им `ShardBootPlan`, и каждый воркер инициализирует `ShardEngine` локально внутри своего потока.

```rust
pub enum ShardInitMode {
    /// Модель A: Готовые экземпляры движка передаются в потоки
    DirectEngines(Vec<(u32, ShardEngine)>),
    /// Модель B: Движки создаются воркерами локально по плану
    BootPlan(Vec<(u32, ShardBootPlan)>),
}

pub struct ShardBootPlan {
    pub shard_id: u32,
    pub config_bytes: Vec<u8>,
}
```

---

## §5. Доменная Логика и Поведение (Domain Logic & Behavior)

Жизненный цикл ноды строго подчинен переходам стейт-машины. Прямой запуск бесконечных процедурных циклов без проверки состояний запрещен.

### §5.1. Жизненный цикл и Машина Состояний (Lifecycle State Machine)

Переходы стейт-машины выполняются по строго определенным правилам. Любая попытка несанкционированного перехода возвращает ошибку `RuntimeError::InvalidStateTransition`.

1.  **Created**: Начальное состояние. Воркеры не запущены.
2.  **Starting**: Запускаются OS-потоки воркеров и инициализируются вычислительные ядра (по модели А или В).
3.  **RecoveryWarmup**: Фаза **Восстановления (Resurrection)**. Запускается холостой цикл прогрева биологической модели на `warmup_ticks` (по умолчанию 100 тиков). На протяжении всей фазы отправка исходящих спайковых пакетов в `NetRuntime` полностью глушится (Muted). По завершении фазы Sentinel проверяет уровень биологического шума. Если шум превышает установленный порог, рантайм переходит в `Faulted { reason: RuntimeFault::UnstableWarmup }`. При успешной стабилизации система переходит в `RunningDay`.
4.  **RunningDay**: Активный вычислительный цикл. Рассылаются команды `ShardCommand::RunDayBatch`, синхронизируются результаты воркеров, продвигается биологическое время `Tick`.
5.  **WaitingNetworkBarrier**: Ожидание BSP барьера от сетевого рантайма, если текущий тик требует синхронизации.
6.  **NightPrepare**: Дневной цикл приостанавливается. Новые шаги вычислений воркерам не отправляются. Оркестратор ждет завершения обработки всех текущих очередей и переводит `ShmStateMachine` в `NightStart`.
7.  **NightRunning**: Ожидание выполнения задач демоном `weaver-daemon`. Оркестратор проверяет состояние `ShmStateMachine` (переходы `NightStart` $\to$ `Sprouting` $\to$ `NightDone`). При превышении лимита времени (10 секунд) оркестратор переходит в `Faulted { reason: RuntimeFault::DaemonTimeout }` без самостоятельного вызова `kill`.
8.  **NightCommit**: Рантайм считывает изменения, вызывает безопасные методы `apply_night_delta` у вычислительных ядер через воркеры и `NetRuntime::apply_route_update(...)` для маршрутов. После коммита рантайм возвращает `ShmStateMachine` в `Idle` и переходит в `RunningDay`.
9.  **Draining**: Ожидание завершения текущего тика при запросе выключения.
10. **Shutdown**: Состояние безопасного выключения. Оркестратор рассылает воркерам `ShardCommand::Shutdown`.
11. **Faulted**: Аварийное состояние. Фиксирует причину сбоя `RuntimeFault`, генерирует событие `RuntimeEvent::SystemFault` и прекращает симуляцию.

---

### §5.2. Диспетчеризация вычислительных шардов (Compute Dispatcher & Shard Isolation)

Для исключения вмешательства планировщика ОС (Context Thrashing) каждый воркер привязан к выделенному OS-потоку (`std::thread`). Схема работы потока воркера по Модели B (Thread-Affine):

```rust
pub fn run_worker_thread(
    shard_id: u32,
    boot_plan: ShardBootPlan,
    cmd_rx: crossbeam::channel::Receiver<ShardCommand>,
    result_tx: crossbeam::channel::Sender<ShardResult>,
) {
    // 1. Инициализация вычислительного ядра внутри OS-потока владельца
    let mut engine = match ShardEngine::initialize_affine(boot_plan) {
        Ok(eng) => eng,
        Err(e) => {
            let _ = result_tx.send(ShardResult::Error(format!("Affinitized initialization failed: {:?}", e)));
            return;
        }
    };

    // 2. Цикл обработки сообщений
    loop {
        match cmd_rx.recv() {
            Ok(ShardCommand::RunDayBatch { batch_cmd }) => {
                match engine.run_day_batch(batch_cmd) {
                    Ok(result) => {
                        let _ = result_tx.send(ShardResult::BatchCompleted {
                            tick: batch_cmd.tick,
                            result,
                        });
                    }
                    Err(e) => {
                        let _ = result_tx.send(ShardResult::Error(format!("Compute error: {:?}", e)));
                        break;
                    }
                }
            }
            Ok(ShardCommand::ApplyNightDelta) => {
                // Применение патчей памяти хостом (Open Debt)
                match engine.apply_night_delta(/* future safe delta handle */) {
                    Ok(_) => {
                        let _ = result_tx.send(ShardResult::NightDeltaApplied);
                    }
                    Err(e) => {
                        let _ = result_tx.send(ShardResult::Error(format!("Delta application error: {:?}", e)));
                        break;
                    }
                }
            }
            Ok(ShardCommand::Shutdown) => {
                // Вызов teardown выполняется на потоке воркере перед его завершением
                engine.teardown();
                let _ = result_tx.send(ShardResult::TeardownComplete);
                break;
            }
            Err(_) => {
                // Канал закрылся (родительский поток аварийно завершился)
                engine.teardown();
                break;
            }
        }
    }
}
```

---

### §5.3. Дневной цикл вычислений (Day Loop)

Дневной цикл выполняется в состоянии `RunningDay` и координируется неблокирующим вызовом метода `step()`:

1.  **Рассылка DayBatchCmd**: Оркестратор подготавливает `DayBatchCmd` (содержит текущий `tick_counter` и метаданные) и отправляет воркерам команду `ShardCommand::RunDayBatch`.
2.  **Опрос сетевых событий (Decoupled Ingress)**: Вместо прямого вызова I/O сокетов или освобождения буферов, рантайм вызывает `net_runtime.poll_events()` / `net_runtime.drain_events()`. Сетевой слой сам обрабатывает события транспорта, вызывает L7-парсер протокола и выполняет освобождение буферов (`release_ingress`). Рантайм получает от сетевого слоя только очищенные события `NetEvent`.
3.  **Сбор результатов вычислений**: Рантайм ожидает ответов `ShardResult` от всех воркеров.
    *   Если канал возвращает ошибку (воркер упал с паникой), генерируется сбой `RuntimeFault::ChannelDisconnected`.
    *   Если воркер возвращает `ShardResult::Error(err)`, рантайм фиксирует `RuntimeFault::GpuLost` и переходит в `Faulted`.
4.  **Атомарный Swap**: После сбора всех результатов выполняется атомарный обмен указателей входного и выходного буферов обмена (`InputSwapchain::swap()`).
5.  **Сетевая координация и продвижение тика**:
    *   Если текущий тик совпадает с интервалом BSP, оркестратор переводит состояние в `WaitingNetworkBarrier` и проверяет барьер через `NetRuntime`.
    *   При обнаружении таймаута соседа (`NetEvent::NeighborTimeout`) рантайм принимает политическое решение: запрашивает обновление RCU-таблицы маршрутов через `NetRuntime::apply_route_update(...)`. Рантайм не мутирует внутренности таблицы маршрутизации напрямую.
    *   Инкрементируется `tick_counter`. При достижении интервала `night_interval_ticks` планируется переход в `NightPrepare`.

---

### §5.4. Координация Ночной Фазы (Night state flow)

Координация Ночной фазы построена на строгой изоляции и не возобновляет симуляцию в случае отравления или сбоя разделяемой памяти.

```
[RunningDay] 
     │  (ticks_elapsed >= night_interval_ticks)
     ▼
[NightPrepare] ──> Ожидание завершения текущих команд воркеров (idle/drain)
     │  (ShmStateMachine: Idle -> NightStart)
     ▼
[NightRunning] ──> wait (weaver_control / ShmStateMachine: NightStart -> Sprouting -> NightDone)
     │
     ├─── [Успех: NightDone] ──> [NightCommit] ──> Применение результатов ──> [RunningDay]
     │                                             (apply_night_delta)
     └─── [Сбой / Тайм-аут] ──> [Faulted { reason: NightTimeout / PoisonedSharedState }]
                                  (Симуляция останавливается, Day-фаза НЕ запускается)
```

1.  **NightPrepare**: Оркестратор дожидается перевода воркеров в режим ожидания. Затем атомарно меняет состояние `ShmStateMachine` на `NightStart`.
2.  **NightRunning**: Период работы демона.
    *   Если `ShmStateMachine` переходит в состояние `Error` или сигнализирует об отравлении сегмента, рантайм переходит в `Faulted { reason: RuntimeFault::PoisonedSharedState }` и останавливается. Возобновление дневного цикла из отравленной памяти запрещено.
    *   Если демон не завершил работу за установленное время, рантайм переходит в `Faulted { reason: RuntimeFault::DaemonTimeout }` и генерирует событие `RuntimeEvent::SystemFault`. Оркестратор не делает прямых вызовов `SIGKILL` или чистки SHM; эти действия возлагаются на внешний контролирующий процесс `node` или супервизор ОС.
3.  **NightCommit**: Рантайм рассылает воркерам `ShardCommand::ApplyNightDelta` для безопасного коммита изменений в VRAM. После получения `ShardResult::NightDeltaApplied` рантайм вызывает RCU-обновление маршрутов в `NetRuntime`, сбрасывает состояние `ShmStateMachine` в `Idle` и переходит в `RunningDay`.

---

### §5.5. Sentinel, Recovery Warmup и Обработка Сбоев (Fault Handling & Core Events)

1.  **Recovery Warmup**:
    *   При старте системы из сохраненного чекпоинта Sentinel переводит рантайм в состояние `RecoveryWarmup`.
    *   Система работает 100 тиков вхолостую. Все сетевые отправки спайков в `NetRuntime` глушатся.
    *   Sentinel отслеживает уровень генерируемого шума. Превышение порога биологической нестабильности переводит рантайм в `Faulted { reason: RuntimeFault::UnstableWarmup }`.
2.  **Эвакуация при сбоях (Fault Evacuation)**:
    *   При сбое GPU или панике канала рантайм переводит воркеры в Shutdown, генерирует `RuntimeEvent::SystemFault` и останавливает симуляцию.
3.  **Disk Checkpoints**:
    *   Рантайм не пишет файлы на диск самостоятельно и не владеет файловыми путями на диске. При необходимости создания чекпоинта оркестратор генерирует событие `RuntimeEvent::CheckpointRequested { tick }`, а после подтверждения от внешнего сервиса принимает `RuntimeEvent::CheckpointCompleted { id }` (или реагирует на соответствующий сигнал). Рантайм только запрашивает и наблюдает за службой чекпоинтов.

---

## §6. Требуемые Инварианты

### §6.1. Структурные инварианты

*   **INV-RUN-001**: *Изоляция потоков шардов (Thread Isolation)*.
    *   *Обоснование*: Каждый вычислительный шард (`ShardWorker`) привязан строго к одному выделенному системному потоку (`std::thread`), чтобы исключить накладные расходы на переключение контекста ОС в горячем цикле вычислений и предотвратить конкуренцию за контекст GPU.
    *   *Следствие нарушения*: Рост латентности p99 из-за context thrashing, неявные гонки за FFI-вызовы бэкендов в `compute`.
    *   *Где проверяется*: Проверка уникальности системного потока (thread ID) при старте вычислительного цикла.

*   **INV-RUN-002**: *Асинхронный диспетчер (Lock-Free Command Delivery)*.
    *   *Обоснование*: Главный управляющий поток оркестратора отправляет вычислительные команды (`ShardCommand`) в поток шарда строго через lock-free очереди `crossbeam::channel` без использования системных мьютексов.
    *   *Следствие нарушения*: Блокировка управляющего потока Control Plane отстающим вычислительным потоком.
    *   *Где проверяется*: Юнит-тесты на неблокирующую отправку команд.

### §6.2. Семантические инварианты

*   **INV-RUN-003**: *Owner-Thread Teardown Sequence*.
    *   *Обоснование*: Деаллокация контекста GPU (`teardown()`) бэкенда должна выполняться строго внутри того же OS-потока `ShardWorker`, в котором происходила инициализация и расчет ядер. Попытка вызвать `teardown()` из родительского управляющего потока после join воркера ведет к краху драйвера.
    *   *Следствие нарушения*: Use-After-Free на уровне GPU, Segmentation Fault при выгрузке динамической библиотеки драйвера.
    *   *Где проверяется*: Интеграционный тест завершения работы, контролирующий вызов `teardown()` до слияния потока.

*   **INV-RUN-004**: *Sentinel Mute during Recovery Warmup*.
    *   *Обоснование*: Во время прогрева после восстановления из чекпоинта (Recovery Warmup, 100 тиков) отправка сетевых пакетов спайков наружу полностью глушится в `NetRuntime`.
    *   *Следствие нарушения*: Засорение кластера фантомными спайками из прошлого, рассинхронизация распределенных узлов.
    *   *Где проверяется*: Тест фазы Recovery Warmup с сетевым моком.

*   **INV-RUN-005**: *Night Phase Lock*.
    *   *Обоснование*: Методы модификации памяти хостом (`apply_night_delta`) и RCU-подмена таблиц маршрутизации выполняются строго при замороженном вычислительном цикле в состояниях `NightPrepare`, `NightRunning` и `NightCommit`.
    *   *Следствие нарушения*: Race condition в VRAM между вычислительными ядрами GPU и хост-записью, повреждение данных.
    *   *Где проверяется*: Runtime assert блокировки состояния вычислительного цикла в фазах Ночной фазы.

### §6.3. Межкрейтовые инварианты

*   **INV-CROSS-013**: *Day/Night SHM State Compliance (Night Phase Sync)*.
    *   *Участники*: `ipc`, `runtime`.
    *   *Кто владелец проверки*: `runtime`.
    *   *Обоснование*: `runtime` координирует Ночную Фазу, переключая состояние `ipc::ShmStateMachine` в режим ожидания `weaver-daemon`. `runtime` не возобновляет симуляцию, пока `ipc::ShmStateMachine` не перейдет в состояние готовности после применения патчей.
    *   *Следствие нарушения*: Data Race в разделяемой памяти между рабочим потоком и демоном.
    *   *Где проверяется*: Интеграционные тесты перехода фаз с `weaver-daemon`.

*   **INV-CROSS-016**: *Zero Direct Dependency on Transport, Protocol, and Wire*.
    *   *Участники*: `runtime`, `net`, `ipc`.
    *   *Кто владелец проверки*: `runtime` (архитектурная изоляция).
    *   *Обоснование*: Оркестратор рантайма не должен иметь прямых зависимостей, импортов или непосредственного вызова API от крейтов `transport`, `protocol` и `wire`. Транзитивные зависимости через крейт `net` допустимы, но вся прямая логика взаимодействия с сетью и сериализации абстрагирована для рантайма слоями `net` и `ipc`.
    *   *Следствие нарушения*: Нарушение принципа модульности, утечка сокетных абстракций в управляющий слой, усложнение тестирования.
    *   *Где проверяется*: Проверка списка непосредственных (direct) зависимостей крейта `runtime` (например, в `Cargo.toml`).

---

## §7. Golden Tests / Обязательная Матрица Тестирования

Крейт `runtime` обязан быть покрыт набором автоматических тестов:

1.  **Атомарный коммит биологического шага (`test_tick_commits_only_after_all_shards_complete`)**: Оркестратор рассылает задачи. Шаг времени (`Tick`) инкрементируется только после того, как все каналы `result_rx` вернут `BatchCompleted`.
2.  **Блокировка тика при сбое вычислений (`test_tick_not_committed_on_shard_error`)**: Один из воркеров имитирует сбой `ShardResult::Error`. Рантайм переходит в `Faulted`, тик не продвигается.
3.  **Корректный контракт передачи данных (`test_day_batch_dispatch_uses_day_batch_cmd`)**: Проверка отправки воркеру полноценного DTO `DayBatchCmd` (а не сырого тика) через канал.
4.  **Отсутствие прямых зависимостей от сетевого стека (`test_no_direct_transport_protocol_wire_dependencies`)**: Проверка непосредственных (direct) зависимостей крейта (глубина 1). Прямые зависимости от крейтов `transport`, `protocol` и `wire` должны отсутствовать.
5.  **Архитектурная изоляция L6 (`test_no_vfs_baker_topology_dependencies`)**: Проверка непосредственных (direct) зависимостей крейта (глубина 1). Крейты `vfs`, `baker`, `topology` должны отсутствовать.
6.  **Переход в аварийный режим без прямых OS-вызовов (`test_night_timeout_transitions_to_faulted_without_restart`)**: Имитация зависания Ночной фазы. Рантайм переходит в `Faulted { reason: RuntimeFault::DaemonTimeout }` и посылает событие `SystemFault`, не пытаясь самостоятельно завершить процесс демона.
7.  **Блокировка при отравлении SHM (`test_poisoned_ipc_never_resumes_day_phase`)**: Установка статуса SHM в `Error`. Рантайм переходит в `Faulted { reason: RuntimeFault::PoisonedSharedState }` и блокирует перезапуск Дневной фазы.
8.  **Заглушение сетевых отправлений при прогреве (`test_recovery_warmup_mutes_net_sends`)**: Инициализация в состоянии `RecoveryWarmup`. Проверка того, что сетевой фасад `NetRuntime` не генерирует внешних пакетов на протяжении 100 тиков.
9.  **Отсутствие C-ABI Teardown Race (`test_teardown_runs_on_owner_worker_before_join`)**: При вызове `shutdown()` воркер должен получить `Shutdown`, вызвать `engine.teardown()`, вернуть `TeardownComplete`, и только после этого родительский поток выполняет `join()`.
10. **Поддержка двух моделей инициализации воркеров (`test_thread_affinity_send_vs_boot_plan_mode_documented`)**: Валидация возможности инициализации как через прямой проброс `ShardEngine`, так и по плану `ShardBootPlan` с внутренней аллокацией.

---

## §8. Open Debt (Открытые Вопросы Проектирования)

1.  **Точная структура полезной нагрузки спайков**: Формат исходящего сетевого контекста доставки спайков не зафиксирован в рантайме. Временное решение: передавать данные пакета через интерфейс `NetRuntime` в виде обобщенных слайсов байт или абстрактного контракта обмена.
2.  **Механизм записи теневых чекпоинтов**: Архитектурные границы записи VRAM чекпоинтов на диск не утверждены. Не определено, должен ли рантайм генерировать событие внешней асинхронной записи, либо использовать специализированный системный сервис `CheckpointWriter`.
3.  **Окончательный выбор модели инициализации воркеров**: Решение о переходе на эксклюзивную Thread-Affine инициализацию (Модель B) будет принято после закрытия технических вопросов в спецификации `compute_spec.md`.
4.  **Синтаксис безопасного API применения ночных изменений**: Конкретные параметры метода `apply_night_delta` в `ShardEngine` остаются на стадии согласования с вычислительным слоем.
