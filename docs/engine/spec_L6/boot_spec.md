# spec_boot

> Версия спеки: 2.0  
> Дата: 2026-06-29  
> Статус: Draft (Architecture Pass 1)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `boot` |
| **Слой** | Слой 6 — Runtime Orchestration (`L6` / Загрузочный пайплайн) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется `std` для работы с файловой системой ОС, путями и динамической аллокацией |
| **Описание** | Загрузочный конвейер ноды (Stateful Boot Pipeline), подготавливающий аппаратное и программное окружение для рантайма. Крейт отвечает за монтирование архивов `.axic` через `vfs`, Shift-Left парсинг конфигурации через `config`, проверку целостности и выравнивания бинарных файлов через `layout`, извлечение рабочих файлов в RAM-диск, инициализацию IPC-ресурсов (Swapchains, SHM) через `ipc`, составление планов маршрутизации через `net` и подготовку вычислительного оборудования (Send-движков или thread-affine планов) через `compute`. Крейт не продвигает системное время, не запускает вычислительный цикл и не обрабатывает сигналы ОС. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `Tick`, `MasterSeed` | Базовые скалярные константы и типы для инициализации времени и ГПСЧ. |
| `vfs` (Слой 2) | `AxicArchive`, `require_file`, `extract_file` | Read-only доступ к виртуальному архиву, извлечение рабочих файлов без прямого парсинга TOC. |
| `config` (Слой 1) | `SimulationConfig`, `ShardConfig`, `ZoneManifest`, парсеры | Чтение и Shift-Left валидация TOML конфигураций симуляции. |
| `layout` (Слой 1) | `VariantParameters`, `StateFileHeader`, `validate_state`, `validate_axons`, `validate_paths` | Валидация выравниваний, смещений и размеров бинарных файлов. |
| `compute` (Слой 3) | `ShardEngine`, `BackendPreference`, `ShardBootPlan` | Stage-инициализация, автовыбор вычислительного бэкенда и подготовка планов. |
| `compute-api` (Слой 3) | `ShardAllocSpec` | Передача спецификаций аллокации (если не реэкспортируется из `compute`). |
| `ipc` (Слой 2) | `ShmStateMachine`, `InputSwapchain`, `OutputSwapchain`, `RuntimeIpcHandles` | Инициализация общих сегментов памяти и примитивов Swapchain. |
| `net` (Слой 5) | `NetConfig`, `RouteProfile`, `NetInitPlan` | Подготовка параметров сетевого стека и стартовых маршрутов. |
| `runtime` (Слой 6) | `RuntimeConfig`, `ShardInitMode` | Сборка конфигурационных DTO запуска симуляционного оркестратора. |

### §2.2. Зависимые Компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `node` (Слой 6) | Исполняемый файл ноды. Настраивает CLI, вызывает `boot` для создания `BootOutput` и инициализирует `runtime` на полученных ресурсах. |

### §2.3. Внешние Зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `thiserror` | `=1.0.69` | Строгая типизация сбоев конвейера (`BootError`). |
| `tracing` | `=0.1.40` | Логирование фаз загрузки и ошибок. |
| `tempfile` | `=3.10.1` | RAII-управление временной директорией (`tmpfs`) на хосте. |

> [!IMPORTANT]
> Крейту `boot` категорически запрещено импортировать или зависеть напрямую от:
> 1. Физических бэкендов вычислений: `compute-cuda`, `compute-hip`, `compute-cpu`.
> 2. Сетевых библиотек нижнего уровня: `transport`, `protocol`, `wire`.
> 3. Компонентов компилятора и роста: `baker`, `topology`, `weaver-daemon`.
> 4. Прямых системных FFI-вызовов `libc` / `windows-sys` для создания SHM, сокетов или вызовов API видеокарт.
> 5. Библиотек парсинга TOML/JSON (`serde`, `toml`) напрямую — парсинг инкапсулирован в `config`.
> 6. Использования `anyhow` в публичных интерфейсах.

### §2.4. Feature Flags

Feature flags отсутствуют. Сборка монолитная.

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`boot`** | **Жизненный цикл инициализации**: Оркестрация фаз загрузки (`BootPhase`), структуры `BootInput` / `BootOutput`, политика отката при сбоях (Rollback), защита мутабельного рабочего пространства (`BootWorkdirGuard`). | Запрещен парсинг TOC архива, объявление схем TOML (владелец `config`), расчет смещений C-ABI (владелец `layout`), FFI-вызовы видеокарт (владелец `compute`), прямой вызов `shm_open` (владелец `ipc`). |
| **`vfs`** | **Доступ к архивам**: Контейнер `.axic`, распаковка в tmpfs, Read-Only mmap. | Запрещена валидация биологического содержимого файлов. |
| **`config`** | **Схемы конфигураций**: Десериализация манифестов и TOML-файлов. | Запрещено создание IPC-каналов и системные аллокации VRAM. |
| **`layout`** | **Бинарные форматы**: Смещения полей SoA, валидация выравнивания. | Запрещено монтирование архивов и запуск OS-потоков. |
| **`compute`** | **Видеопамять и Движки**: Аллокация VRAM, управление GPU-бэкендами. | Запрещен парсинг дисковых файлов и проверка сетевых маршрутов. |
| **`ipc`** | **OS IPC ресурсы**: Создание сегментов SHM,Swapchains, UDS-сокеты. | Запрещен AOT-процессинг и компиляция связей. |
| **`net`** | **Сетевые структуры**: Инициализация RCU-маршрутов и BSP. | Запрещено прямое открытие сокетов до вызова оркестратора рантайма. |

---

## §4. Публичная API-Модель (Public API Model)

> [!NOTE]
> `NetInitPlan` и `RuntimeIpcHandles` являются композиционными DTO-структурами, которыми владеет крейт `boot`. Они собираются на этапе подготовки из внутренних типов крейтов `net` и `ipc` соответственно.
> Крейт `boot` также возвращает параметры запуска `WeaverControlInit` для демона `weaver-daemon`. Прослушивание сокетов и непосредственный запуск процесса осуществляются внешним демоном `node`/супервизором, сам `boot` сокеты управления не открывает.

Публичный интерфейс предоставляет структуры для настройки входных параметров загрузки, возврата готовых ресурсов симуляции и отслеживания фаз конвейера:

```rust
use std::path::PathBuf;
use compute::{BackendPreference, ShardEngine};
use ipc::{ShmStateMachine, InputSwapchain, OutputSwapchain};
use net::{NetConfig, RouteProfile};
use runtime::{RuntimeConfig, ShardInitMode};
use vfs::AxicArchive;

#[derive(Debug, Clone)]
pub struct BootOverrides {
    pub force_backend: Option<BackendPreference>,
    pub port_override: Option<u16>,
}

/// Легковесный план инициализации шарда для thread-affine модели (Model B)
pub struct BootShardPlan {
    pub shard_id: u32,
    pub config_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardInitPolicy {
    /// Модель A: Движки инициализируются boot и передаются в runtime (требуется ShardEngine: Send)
    DirectEnginesIfSend,
    /// Модель B: Передается легковесный план, воркеры инициализируют движки сами
    ThreadAffineBootPlan,
}

pub struct BootInput {
    pub archive_path: PathBuf,
    pub workdir_root: PathBuf,
    pub backend_preference: BackendPreference,
    pub shard_init_policy: ShardInitPolicy,
    pub runtime_dir: PathBuf,
    pub node_overrides: BootOverrides,
}

pub struct NetInitPlan {
    pub config: NetConfig,
    pub initial_routes: Vec<RouteProfile>,
}

pub struct RuntimeIpcHandles {
    pub shm_state: ShmStateMachine,
    pub input_swapchain: InputSwapchain,
    pub output_swapchain: OutputSwapchain,
}

/// Абстрактный инициализатор интерфейса управления демоном Ночной фазы
pub struct WeaverControlInit {
    pub shm_path: PathBuf,
    pub socket_path: PathBuf,
}

/// RAII-защитник временной директории извлеченных мутабельных файлов
pub struct BootWorkdirGuard {
    pub path: PathBuf,
    _temp_dir: Option<tempfile::TempDir>,
}

pub struct BootOutput {
    pub runtime_config: RuntimeConfig,
    pub shard_init_mode: ShardInitMode,
    pub net_init: NetInitPlan,
    pub ipc_handles: RuntimeIpcHandles,
    pub weaver_control_init: WeaverControlInit,
    pub workdir_guard: BootWorkdirGuard,
    pub archive_guard: Option<AxicArchive>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    Created,
    ArchiveOpened,
    ConfigParsed,
    RequiredFilesResolved,
    MutableArtifactsExtracted,
    LayoutValidated,
    ShardInitPrepared,
    IpcPrepared,
    NetPlanPrepared,
    RuntimeConfigPrepared,
    Ready,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum BootError {
    #[error("VFS error: {0}")]
    Vfs(vfs::VfsError), // typed source error, exact path/name follows owner crate
    #[error("Config parsing/validation error: {0}")]
    Config(config::ConfigError),
    #[error("Layout alignment/offset error: {0}")]
    Layout(layout::LayoutError),
    #[error("Compute allocation/upload error: {0}")]
    Compute(compute::ComputeError),
    #[error("IPC initialization error: {0}")]
    Ipc(ipc::IpcError),
    #[error("Invalid network configuration: {0}")]
    Net(net::NetError),
    #[error("Missing required file in archive: {path}")]
    MissingRequiredFile { path: String },
    #[error("Invalid phase transition from {from:?} to {to:?}")]
    InvalidPhaseTransition { from: BootPhase, to: BootPhase },
    #[error("Workdir is not memory-backed (tmpfs required)")]
    WorkdirNotMemoryBacked,
    #[error("Unsupported shard initialization policy: {0}")]
    UnsupportedShardInitPolicy(String),
    #[error("Dangling boot resource detected during cleanup")]
    DanglingBootResource,
    #[error("Resource rollback failed during cleanup")]
    RollbackFailed,
    #[error("Internal invariant violated: {0}")]
    InternalInvariant(String),
}

pub struct BootPipeline {
    input: BootInput,
    phase: BootPhase,
    archive: Option<AxicArchive>,
    workdir_guard: Option<BootWorkdirGuard>,
    // Внутренние промежуточные ресурсы для отката
    ipc_guard: Option<ipc::IpcResourceGuard>,
    allocated_engines: Vec<ShardEngine>,
}

impl BootPipeline {
    pub fn new(input: BootInput) -> Self;
    pub fn run(self) -> Result<BootOutput, BootError>;
    pub fn phase(&self) -> BootPhase;
}
```

---

## §5. Доменная Логика и Поведение (Domain Logic & Behavior)

### §5.1. Жизненный цикл и Конвейер Фаз (Boot Phase State Machine)

Конвейер загрузки состоит из 12 последовательных фаз. Пропуск фаз или их перестановка категорически запрещены. При попытке выполнить недопустимый переход метод `run` возвращает `BootError::InvalidPhaseTransition` и инициирует экстренный откат.

```
Успешный путь (Success path):
[Created] ──> [ArchiveOpened] ──> [ConfigParsed] ──> [RequiredFilesResolved]
                                                               │
[LayoutValidated] <── [MutableArtifactsExtracted] <────────────┘
       │
       └──> [ShardInitPrepared] ──> [IpcPrepared] ──> [NetPlanPrepared]
                                                             │
[Ready] <────────────────────────────────────── [RuntimeConfigPrepared] ◄──┘

Путь сбоя (Error path):
[Любая фаза / Any Phase] ──(Ошибка валидации/системы)──> Откат (Rollback) ──> [Failed] (терминальное состояние)
```

1.  **Created**: Конвейер инициализирован входными данными `BootInput`.
2.  **ArchiveOpened**: Вызов `AxicArchive::open(input.archive_path)`. Полученный хэндл сохраняется в конвейере.
3.  **ConfigParsed**: Извлечение TOML-файлов манифеста (`manifest.toml`, `department.toml`). Передача сырых байт в парсер `config`.
4.  **RequiredFilesResolved**: Проверка обязательного списка файлов в архиве (в качестве примера v2/минимального набора, задаваемого манифестом: `manifest.toml`, `department.toml`, `depart.state`, `synapses.axons`, `axon.paths`). Если обязательные файлы отсутствуют, переход завершается с ошибкой `BootError::MissingRequiredFile`.
5.  **MutableArtifactsExtracted**: Копирование файлов состояния (`.state`, `.axons`, `.paths`) во временную директорию RAM-диска (tmpfs) через VFS API.
6.  **LayoutValidated**: Вызов валидаторов `layout` для проверки выравнивания и размеров заголовков файлов `.state`, `.axons` и `.paths`.
7.  **ShardInitPrepared**: Подготовка вычислительных шардов.
    *   **При ShardInitPolicy::DirectEnginesIfSend**: Boot вызывает методы фасада `ShardEngine::allocate_vram` и `ShardEngine::upload_shard` (или `ShardEngine::bootstrap`) в `compute`, создавая готовые экземпляры `ShardEngine`.
    *   **При ShardInitPolicy::ThreadAffineBootPlan**: Движки не создаются. Формируются только дескрипторы `BootShardPlan` (передаются в runtime через `ShardInitMode`), которые упаковывают необходимые для последующей аллокации данные.
8.  **IpcPrepared**: Инициализация SHM и Swapchains. Если при вызове `ipc` обнаруживается отравленный (poisoned) сегмент от предыдущего запуска, конвейер требует его принудительного удаления и пересоздания через `ipc` API.
9.  **NetPlanPrepared**: Сборка сетевой конфигурации `NetConfig` и стартовых маршрутов. Физические сокеты не открываются.
10. **RuntimeConfigPrepared**: Сборка `RuntimeConfig` на основе полученных измерений.
11. **Ready**: Ресурсы успешно собраны и упакованы в `BootOutput` (терминальное успешное состояние).
12. **Failed**: Системная ошибка. Выполняется экстренный откат (Rollback) и перевод конвейера в терминальное ошибочное состояние.

---

### §5.2. Правила Безопасности Владения и Времени Жизни (Lifetimes & Ownership Guards)

1.  **Запрет Утечки Ссылок**: Структуры `BootOutput`, `BootShardPlan`, `RuntimeConfig` и `NetInitPlan` не должны содержать висячих заимствованных срезов (dangling borrowed slices) из `AxicArchive`. Все данные, время жизни которых превышает фазу работы `boot`, обязаны быть либо принадлежащими (owned), либо защищены явным охранником времени жизни (explicit guard/lifetime). В случае, если в будущем разрешается использование zero-copy срезов, возвращаемый тип должен быть параметризован временем жизни и удерживать `archive_guard`.
2.  **Защита Временной Папки**: Все извлеченные мутабельные файлы уничтожаются при сбоях или нормальном завершении работы ноды. `BootOutput` возвращает `BootWorkdirGuard`. Пока этот объект жив, временная папка в tmpfs гарантированно удерживается в системе. При вызове деструктора `Drop` для `BootWorkdirGuard` временные папки неявно стираются через RAII.

---

### §5.3. Подготовка Вычислительной Среды (Model A vs Model B)

1.  **Принудительный Выбор Бэкенда**: Если в `BootInput` передан флаг `force_backend` (например, `BackendPreference::Cuda`), конвейер обязан вызвать инициализацию именно этого бэкенда. Автоматический тихий откат к CPU при отсутствии видеокарты запрещен и вызывает `BootError::Compute`.
2.  **Layout-контроль**: Валидация файлов `layout` должна выполняться строго до вызова аллокаций VRAM. Спецификация запрещает путать смещения дисковых `.state` файлов и смещения живых SHM сегментов в оперативной памяти хоста.

---

### §5.4. Fail-Fast Rollback Логика (Откат ресурсов)

Если любая из фаз загрузки возвращает ошибку `Err`, конвейер немедленно прерывает выполнение и запускает процедуру каскадной очистки ресурсов:

*   Если ошибка произошла до инициализации IPC и Compute: конвейер очищает временную директорию `BootWorkdirGuard`.
*   Если были аллоцированы вычислительные ресурсы (в Модели А): для каждого созданного `ShardEngine` вызывается безопасный метод очистки `engine.teardown()`.
*   Если были созданы IPC сегменты: вызывается `ipc` API для удаления созданных файлов SHM (`unlink_shm`).
*   Если во время отката возникает вторичная ошибка, она логируется на уровне `ERROR`, но исходная ошибка `BootError`, вызвавшая сбой конвейера, остается первичной и возвращается пользователю.

---

## §6. Требуемые Инварианты

*   **INV-BOOT-001**: *Строгая очередность фаз конвейера*.
    *   *Обоснование*: Инициализационный конвейер состоит из 12 фаз. Фазы физически не могут выполняться вне очереди (например, аллокация VRAM невозможна без выровненных байт из `LayoutValidated` и метаданных `ConfigParsed`).
    *   *Следствие нарушения*: Паника ОС, обращение по невалидным указателям, неопределенное поведение (UB) GPU-драйвера.
    *   *Где проверяется*: Проверка переходов внутренней стейт-машины при вызове `run()`.

*   **INV-BOOT-002**: *RAII Fail-Fast (Изоляция ресурсов при сбоях)*.
    *   *Обоснование*: Любой сбой до перехода в состояние `Ready` гарантирует неявный откат всех частично созданных ОС ресурсов (SHM, VRAM) и удаление временных файлов tmpfs.
    *   *Следствие нарушения*: Утечка VRAM, дескрипторов и "осиротевшие" временные файлы, блокирующие повторный запуск ноды.
    *   *Где проверяется*: Тест отката ресурсов при ошибке на промежуточной фазе.

*   **INV-BOOT-003**: *RAM-Disk Mutability (Защита от износа SSD)*.
    *   *Обоснование*: Мутабельные файлы (`.state`, `.axons`), извлекаемые из архива для работы рантайма, обязаны распаковываться строго в директорию, смонтированную в оперативной памяти ОС (tmpfs / RAM-диск), чтобы исключить износ физических накопителей SSD.
    *   *Следствие нарушения*: Физическая смерть NVMe/SSD накопителей на серверах кластера.
    *   *Где проверяется*: Проверка пути `workdir_root` при инициализации пайплайна.

*   **INV-BOOT-004**: *C-ABI Layout Guard (Аппаратный барьер выравнивания)*.
    *   *Обоснование*: Перед передачей спецификаций аллокаций в вычислительный бэкенд, файлы состояния должны пройти проверку выравнивания по границам 64 байт для `.state` и 32 байт для `.axons`.
    *   *Следствие нарушения*: Падение пропускной способности PCIe шины или аппаратный крах драйвера GPU (Misaligned Address).
    *   *Где проверяется*: Вызов функций валидации `layout` на фазе `LayoutValidated`.

*   **INV-BOOT-005**: *No silent backend fallback*.
    *   *Обоснование*: Если пользователь явно запросил аппаратный ускоритель (CUDA/HIP), конвейер не имеет права переключаться на CPU при возникновении сбоя инициализации GPU.
    *   *Следствие нарушения*: Скрытая деградация производительности симуляции.
    *   *Где проверяется*: Проверка возврата `BootError::Compute` при сбое аллокации VRAM.

*   **INV-CROSS-017**: *Zero Direct Dependency on Transport, Protocol, and Wire*.
    *   *Участники*: `boot`, `net`, `ipc`.
    *   *Кто владелец проверки*: `boot` (архитектурная изоляция).
    *   *Обоснование*: Конвейер инициализации не имеет права содержать прямые зависимости, импорты или непосредственные вызовы API от крейтов `transport`, `protocol` и `wire`. Вся сетевая конфигурация подготавливается через типы крейта `net`.
    *   *Следствие нарушения*: Утечка сетевых сокетных абстракций в загрузочный слой, усложнение графа компиляции.
    *   *Где проверяется*: Проверка непосредственных зависимостей (`Cargo.toml`).

---

## §7. Golden Tests / Обязательная Матрица Тестирования

Крейт `boot` обязан быть покрыт набором автоматических тестов:

1.  **Контроль фаз конвейера (`test_boot_phase_order_enforced`)**: Верификация того, что все фазы выполняются строго в заданной последовательности.
2.  **Запрет неверных переходов (`test_invalid_phase_transition_rejected`)**: Попытка вызвать выполнение фазы вне очереди возвращает `BootError::InvalidPhaseTransition`.
3.  **Сбой при отсутствии обязательного файла (`test_missing_required_file_fails_before_compute`)**: Отсутствие в архиве файла `synapses.axons` прерывает загрузку до выделения ресурсов Compute.
4.  **Обработка ошибок манифеста (`test_bad_manifest_returns_config_error_without_panic`)**: Передача битого TOML-файла возвращает `BootError::Config` без паники процесса.
5.  **Изоляция архива (`test_archive_open_uses_vfs_api_only`)**: Проверка того, что открытие архива происходит строго через API `vfs` без прямого парсинга заголовков `.axic`.
6.  **Границы экстракции (`test_mutable_extraction_stays_inside_workdir`)**: Верификация того, что все распакованные файлы находятся строго внутри директории `workdir_root`.
7.  **Неизменяемость исходного архива (`test_original_axic_never_modified`)**: Проверка отсутствия операций записи в исходный `.axic` файл.
8.  **Валидация разметки до аллокации (`test_layout_validation_runs_before_compute_prepare`)**: Проверка того, что вызовы валидаторов `layout` завершаются до начала подготовки вычислительных ресурсов.
9.  **Сбой при неверном выравнивании (`test_bad_state_alignment_rejected`)**: Файл состояния с неверной кратностью выравнивания (например, 63 байта) вызывает ошибку `BootError::Layout`.
10. **Изоляция Compute в Модели А (`test_model_a_uses_shard_engine_facade_only`)**: Проверка того, что подготовка движков идет строго через фасад `compute::ShardEngine` без сырого FFI.
11. **Отсутствие аллокаций в Модели B (`test_model_b_returns_boot_plan_without_allocating_engine`)**: Верификация возврата `BootShardPlan` без создания живого экземпляра движка и выделения VRAM.
12. **Запрет тихого отката бэкенда (`test_explicit_backend_failure_does_not_silent_fallback`)**: Сбой аллокации CUDA при явном выборе GPU возвращает ошибку, не переключаясь на CPU.
13. **Изоляция IPC ресурсов (`test_ipc_created_only_through_ipc_api`)**: Проверка создания сегментов SHM и Swapchains исключительно через функции крейта `ipc`.
14. **Обработка отравленной SHM (`test_poisoned_shm_rejected_or_explicitly_recreated`)**: Проверка того, что конвейер падает с ошибкой или требует явного пересоздания отравленного сегмента SHM.
15. **Отсутствие сетевых зависимостей (`test_net_plan_uses_net_types_without_wire_dependency`)**: Верификация сборки `NetInitPlan` с использованием типов `net` без импортов из `wire`.
16. **Отсутствие прямых транспортных зависимостей (`test_no_direct_transport_protocol_wire_dependencies`)**: Проверка Cargo.toml: прямые зависимости от `transport`, `protocol` и `wire` отсутствуют.
17. **Отсутствие прямых зависимостей от GPU-бэкендов (`test_no_direct_backend_crate_dependencies`)**: Проверка Cargo.toml: зависимости от `compute-cuda`/`compute-hip`/`compute-cpu` отсутствуют.
18. **Запрет запуска симуляции (`test_no_runtime_tick_started_by_boot`)**: Загрузчик не вызывает методы рассылки тиков рантайма.
19. **Удержание ресурсов в BootOutput (`test_boot_output_keeps_archive_and_workdir_guards_alive`)**: Проверка того, что удаление `BootOutput` приводит к освобождению архива и папки, но пока объект жив, они активны.
20. **Консистентность очистки при сбоях (`test_fail_fast_rollback_cleans_created_resources`)**: Проверка удаления созданных сегментов SHM и teardown движков при ошибке на фазе RuntimeConfig.
21. **Детерминированность загрузки (`test_repeated_boot_same_archive_produces_same_normalized_plan`)**: Повторный запуск с одним архивом выдает побитово идентичную конфигурацию запуска.

---

## §8. Open Debt (Открытые Вопросы Проектирования)

1.  **Точный список обязательных файлов в `.axic`**: Точный список обязательных файлов в фазе `RequiredFilesResolved` не зафиксирован в спецификациях baker/vfs/config и может определяться динамически на основе манифеста.
2.  **Окончательное владение файлами Ghost-связей**: Архитектурный слой для `.gxi`, `.gxo` и `.ghosts` не определен (layout vs topology).
3.  **Разделение заголовка SHM**: Окончательное владение `ShmHeader`, `ShmState` и `EphysShm` находится на согласовании (layout vs ipc).
4.  **Модель инициализации воркеров**: Решение о выборе между Моделью А (Send) и Моделью B (Thread-Affine) зависит от технических возможностей compute бэкендов.
5.  **Физическое размещение и контракт BootShardPlan / ShardBootPlan**: Окончательное определение места владения и контракта обмена планами между `boot` и `runtime`.
6.  **Инициализация сетевого рантайма**: Определить, должен ли `boot` возвращать живой `NetRuntime` или только спецификацию `NetInitPlan` (предпочтительно второе).
7.  **RAM-диск на Windows**: Определение системного механизма памяти для Windows-платформ (virtual RAM-drive).
8.  **Точки интеграции службы чекпоинтов**: Правила восстановления из чекпоинта VRAM при холодном старте.
9.  **Недостающие параметры TOML**: Определение полей `initial_synapse_weight`, а также физических координат сетевых сокетов.
10. **Протоколы сетевого автообнаружения**: Начальная геометрия распределения соседей по зонам.
6.  **Инициализация сетевого рантайма**: Определить, должен ли `boot` возвращать живой `NetRuntime` или только спецификацию `NetInitPlan` (предпочтительно второе).
7.  **RAM-диск на Windows**: Определение системного механизма памяти для Windows-платформ (virtual RAM-drive).
8.  **Точки интеграции службы чекпоинтов**: Правила восстановления из чекпоинта VRAM при холодном старте.
9.  **Недостающие параметры TOML**: Определение полей `initial_synapse_weight`, а также физических координат сетевых сокетов.
10. **Протоколы сетевого автообнаружения**: Начальная геометрия распределения соседей по зонам.
