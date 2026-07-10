# spec_compute_api

> Версия спеки: 2.3  
> Дата: 2026-07-10  

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `compute-api` |
| **Слой** | Слой 3 — Абстракция Вычислений (Compute Hardware Abstraction Layer / HAL) |
| **Тип** | Library (`lib`) |
| **no_std** | Да (`true`) — строго изолированный легкий контрактный крейт без динамических аллокаций (`no_std` / `no_alloc`) |
| **Описание** | Аппаратно-независимый HAL-контракт вычислительных бэкендов (CPU, CUDA, HIP, Mock) для движка `AxiEngine`. Крейт определяет объектно-безопасный трейт `ComputeBackend`, непрозрачные дескрипторы VRAM (`VramHandle`), структуры команд DTO (`DayBatchCmd`), DTO результатов (`BatchResult`), отладочные снимки состояния (`ShardSnapshotMut`), структуры обслуживания (`BackendMaintenanceMut`, `BackendMaintenanceRef`) и типизированную иерархию ошибок (`ComputeApiError`). `compute-api` не владеет C-ABI макетами памяти, физическими формулами симуляции и низкоуровневыми FFI-вызовами конкретных ускорителей. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | Скалярные идентификаторы, константы типов | Использование фундаментальных типов (`Tick`, `Voltage`) в DTO вызовов. |
| `layout` (Слой 1) | Расчет размеров блобов состояния и аксонов | Валидация входных байтовых массивов на соответствие контрактам C-ABI выравнивания и размеров. |

> [!IMPORTANT]
> Крейт `compute-api` **не зависит** от крейтов `physics` и `alloc`. Вычислительные DTO принимают уже готовые скалярные значения (например, pre-calculated `v_seg`), не вычисляя физику нейронов внутри API-слоя и не требуя кучи.

### §2.2. Зависимые компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `compute` (Слой 3) | Фасад вычислений (`ShardEngine`) использует трейт `ComputeBackend` (через `Box<dyn ComputeBackend>`) для динамического выбора и управления бэкендами. |
| `compute-cpu` (Слой 3) | Реализация вычислительного бэкенда для многопоточного CPU на базе SIMD. |
| `compute-cuda` (Слой 3) | Реализация вычислительного бэкенда для ускорителей NVIDIA CUDA. |
| `compute-hip` (Слой 3) | Реализация вычислительного бэкенда для ускорителей AMD ROCm/HIP. |
| `test-harness` (Слой 3) | Тестовый комплекс для верификации контракта и снимков состояний (`debug_snapshot`). |

> [!NOTE]
> Weaver (демон биологических шагов и топологии) напрямую **не зависит** от крейта `compute-api`. Weaver работает с хост-резидентными структурами и не использует HAL-интерфейсы ускорителей.

### §2.3. Внешние зависимости

Внешние зависимости отсутствуют. Крейт собирается в `#![no_std]` окружении.

### §2.4. Feature Flags и Вендорская Независимость

Крейт не содержит вендорских feature flags (`cuda`, `hip`). Публичный контракт является единым и нейтральным для всех аппаратных платформ.

| Feature | Default | Назначение |
|---|---|---|
| `default` | `[]` | Строго изолированное `no_std` / `no_alloc` окружение. |
| `std` | `[]` | Опциональный флаг исключительно для dev/test интеграций с `std::error::Error`. |

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`compute-api`** (Слой 3) | **Публичные Rust-контракты вычислений**: Трейт бэкенда (`ComputeBackend`), непрозрачные дескрипторы ресурсов (`VramHandle`), структуры команд DTO (`DayBatchCmd`), DTO результатов (`BatchResult`), снимки состояния (`ShardSnapshotMut`), характеристики оборудования (`BackendCapabilities`), перечисление ошибок (`ComputeApiError`) и правила жизненного цикла ресурсов. | Запрещено утверждение владения макетами `ShardVramPtrs` (целевой владелец `layout`), `StateOffsets`, `.state`, `.axons`, `VariantParameters`, `BurstHeads8` (владелец `layout`), физическими формулами (владелец `physics`), вендорскими FFI-символами и стримами (владельцы `compute-cuda`/`hip`/`cpu`), авто-выбором бэкендов и `Box<dyn ComputeBackend>` (владелец `compute`), а также Pinned Host буферами. |
| **`layout`** (Слой 1) | **Макеты Памяти и C-ABI**: Физическая структура SoA-плоскостей памяти, C-ABI выравнивание (`ShardVramPtrs`, `VariantParameters`, `BurstHeads8`, заголовки файлов). | Запрещено управление вызовами выполнения батчей на вычислительных ускорителях. |
| **`physics`** (Слой 0) | **Физическая Математика**: Чистые формулы интеграции потенциалов и пластичности. | Запрещена привязка к буферам памяти и структурам вызовов бэкендов. |
| **Бэкенды** (`compute-cuda` / `hip` / `cpu`) | **Аппаратные Реализации**: Аллокация физической VRAM, владение Pinned Host staging буферами, вызовы CUDA/HIP API, FFI-указатели, стримы execution, трансляция ошибок вендора. | Запрещено изменение публичных DTO структур и нарушение объектной безопасности трейта. |

---

## §4. Основные Аппаратные Понятия и DTO Структуры

### §4.1. Перечисление Бэкендов и Характеристики
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cpu,
    Cuda,
    Hip,
    Mock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub lane_count: u32,             // Размер Warp (32 NVIDIA, 64 AMD, 1/SIMD CPU)
    pub supports_async: bool,        // Внутренняя способность бэкенда к асинхронным стримам/DMA (зарезервировано)
    pub supports_ephys: bool,        // Поддержка съема осциллограмм в горячем цикле
    pub max_batch_ticks: u32,        // Максимальный размер батча тиков за один вызов
    pub alignment_bytes: usize,      // Требование выравнивания буферов (64B)
    pub pinned_host_required: bool,  // Флаг рекомендации/требования Pinned Host memory для DMA
}
```

### §4.2. Безопасный Непрозрачный Дескриптор VRAM (`VramHandle`)
Адресация выделенной памяти на ускорителе выполняется строго через непрозрачный дескриптор. Бэкенды конструируют дескриптор из своих локальных идентификаторов через безопасный фабричный метод:
```rust
use core::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VramHandle {
    kind: BackendKind,
    id: NonZeroU64,
    generation: u32,
}

impl VramHandle {
    /// Публичный безопасный конструктор дескриптора для бэкендов вычислений.
    #[inline(always)]
    pub const fn from_raw_parts(kind: BackendKind, id: NonZeroU64, generation: u32) -> Self {
        Self { kind, id, generation }
    }

    #[inline(always)]
    pub const fn kind(&self) -> BackendKind { self.kind }

    #[inline(always)]
    pub const fn id(&self) -> NonZeroU64 { self.id }

    #[inline(always)]
    pub const fn generation(&self) -> u32 { self.generation }
}
```
*Примечание*: Структурное создание `VramHandle` не означает его валидность в рантайме. Каждый бэкенд обязан проверять, что `id` принадлежит ему, физически аллоцирован и совпадает по `generation`.

### §4.3. Спецификация Аллокации и Загрузки
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardAllocSpec {
    pub padded_n: u32,
    pub total_axons: u32,
    pub total_ghosts: u32,
    pub virtual_offset: u32,
}

#[derive(Debug)]
pub struct ShardUpload<'a> {
    pub state_blob: &'a [u8],
    pub axons_blob: &'a [u8],
    pub variant_table: &'a [layout::VariantParameters; layout::VARIANT_LUT_LEN],
}
```

### §4.4. Команда Выполнения Дневного Батча (`DayBatchCmd`)
Структура DTO содержит геометрию I/O, разделенные входные и выходные буферы для автономного выполнения батча тиков симуляции в горячем цикле. В v2.1 передача RNG seed не используется (горячий цикл строго детерминирован):
```rust
#[derive(Debug)]
pub struct DayBatchCmd<'a> {
    pub tick_base: u64,
    pub sync_batch_ticks: u32,
    pub v_seg: u32,                   // Значение передается посчитанным из physics (1..=255)
    pub dopamine: i16,
    pub input_words_per_tick: u32,    // Количество 32-битных слов входного битмаска на тик
    pub max_spikes_per_tick: u32,     // Емкость спайков за один тик (stride)
    pub num_outputs: u32,             // Количество опрашиваемых соматических выходов
    pub virtual_offset: u32,
    pub num_virtual_axons: u32,
    pub input_bitmask: Option<&'a [u32]>,
    pub incoming_spikes: Option<&'a [u32]>,
    pub incoming_spike_counts: &'a [u32],   // Длина строго равна sync_batch_ticks
    pub mapped_soma_ids: &'a [u32],
    pub output_spikes: &'a mut [u32],        // Буфер выходящих спайковых ID (емкость >= sync_batch_ticks * max_spikes_per_tick)
    pub output_spike_counts: &'a mut [u32], // Длина строго равна sync_batch_ticks (заполняется бэкендом)
}
```

### §4.5. Результат Выполнения Батча (`BatchResult`)
Сгенерированные идентификаторы спайков записываются напрямую в `cmd.output_spikes`, а потиковое количество сгенерированных спайков записывается бэкендом в `cmd.output_spike_counts`. Структура `BatchResult` возвращает сводную телеметрию и счетчики:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchResult {
    pub ticks_executed: u32,
    pub generated_spikes_count: u32,
    pub output_spikes_written: u32,
    pub dropped_spikes_count: u32,
    pub execution_time_us: u64,
}
```

### §4.6. Отладочный Снимок Состояния (`ShardSnapshotMut`)
Используется для диагностического выгружения текущего состояния VRAM в тестовом комплексе `test-harness`:
```rust
#[derive(Debug)]
pub struct ShardSnapshotMut<'a> {
    pub state_blob: &'a mut [u8],
    pub axons_blob: &'a mut [u8],
}
```

### §4.7. Структуры Обслуживания Бэкенда (Backend Maintenance DTOs)
Используются для моста переноса данных между хостом и устройством в период фазы обслуживания (Maintenance):
```rust
/// Буферы для выгрузки (экспорта) состояния устройства на хост.
#[derive(Debug)]
pub struct BackendMaintenanceMut<'a> {
    pub state_blob: &'a mut [u8],
    pub axons_blob: &'a mut [u8],
}

/// Буферы для загрузки (импорта) состояния с хоста на устройство.
#[derive(Debug)]
pub struct BackendMaintenanceRef<'a> {
    pub state_blob: &'a [u8],
    pub axons_blob: &'a [u8],
}
```
*Примечание*: Буфер путей аксонов (`paths_blob`) отсутствует в HAL-контракте обслуживания, так как пути являются хост-резидентными данными и никогда не загружаются на физические ускорители (устройства).

Размеры буферов обслуживания рассчитываются на основе геометрии шарда с использованием формул из `layout`:
- `state_blob`: `layout::calculate_state_blob_size(padded_n)`
- `axons_blob`: `expected_axons_blob_size(total_axons)`

Функция `expected_axons_blob_size` возвращает ожидаемый размер байтового буфера аксонов в зависимости от их количества:
```rust
pub fn expected_axons_blob_size(total_axons: u32) -> Result<usize, ComputeApiError> {
    // В будущем это тонкая обертка над layout::calculate_axons_blob_size (landed in T002b).
    // Любой сбой переполнения или None преобразуется в ComputeApiError::InvalidShape.
    let num_axons = total_axons as usize;
    num_axons.checked_mul(32)
        .and_then(|axons_size| axons_size.checked_add(16))
        .ok_or(ComputeApiError::InvalidShape)
}
```

---

## §5. Требования к Трейту Бэкенда (Trait Requirements)

Публичный контракт вычислительного бэкенда утвержден под именем `ComputeBackend`.

### §5.1. Концептуальный Скелет Трейта (`ComputeBackend`)
```rust
pub trait ComputeBackend {
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> BackendCapabilities;
    fn alloc_shard(&mut self, spec: ShardAllocSpec) -> Result<VramHandle, ComputeApiError>;
    fn upload_shard(&mut self, handle: VramHandle, upload: ShardUpload<'_>) -> Result<(), ComputeApiError>;
    fn run_day_batch(&mut self, handle: VramHandle, cmd: DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError>;
    fn free_shard(&mut self, handle: VramHandle) -> Result<(), ComputeApiError>;
    fn teardown(&mut self) -> Result<(), ComputeApiError>;

    /// Диагностический метод для выгрузки состояния в тестовом комплексе test-harness.
    /// Возвращает UnsupportedFeature по умолчанию.
    fn debug_snapshot(&mut self, _handle: VramHandle, _snapshot: ShardSnapshotMut<'_>) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }

    /// Экспорт текущего рабочего состояния (state + axons) во внешние host-буферы в режиме Maintenance.
    /// Возвращает UnsupportedFeature по умолчанию.
    fn export_maintenance_state(&mut self, _handle: VramHandle, _out: BackendMaintenanceMut<'_>) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }

    /// Импорт измененного рабочего состояния (state + axons) из host-буферов в режиме Maintenance.
    /// Возвращает UnsupportedFeature по умолчанию.
    fn import_maintenance_state(&mut self, _handle: VramHandle, _src: BackendMaintenanceRef<'_>) -> Result<(), ComputeApiError> {
        Err(ComputeApiError::UnsupportedFeature)
    }
}
```

### §5.2. Правила Проектирования Трейта
1. **Объектная Безопасность (Object Safety)**: Трейт не содержит generic-методов или ассоциированных типов. Это позволяет инстанцировать бэкенд как динамический объект (`Box<dyn ComputeBackend>`) на уровне фасада `compute`.
2. **Безопасность Типов (Safe API)**: Ни один публичный метод трейта не принимает и не возвращает сырые указатели (`*mut u8`, `*const u8`). Сырой FFI-код разрешен только внутри конкретных бэкендов.
3. **Отсутствие Паник**: Все методы возвращают `Result<T, ComputeApiError>`. Паники внутри бэкендов запрещены.
4. **Синхронность Выполнения в v2.1**: Метод `run_day_batch` является блокирующим синхронным вызовом. Управление возвращается только после полного завершения всех тиков батча и готовности выходных буферов.
5. **Батчевая Диспетчеризация (Batch-Level Dispatch)**: Вызов метода выполнения производится единоразово на весь батч тиков (`sync_batch_ticks`), а не на каждый тик отдельно.
6. **Явный Жизненный Цикл Ресурсов**: Ресурсы создаются и уничтожаются через методы `alloc_shard` / `free_shard` / `teardown`.
7. **Разделение Диагностики и Обслуживания**: Метод `debug_snapshot` предназначен исключительно для диагностических целей в рамках тестового окружения (`test-harness`). Рабочий экспорт и импорт состояния шарда на хост осуществляется методами `export_maintenance_state` и `import_maintenance_state`.
8. **Отсутствие Биологических Политик в HAL**: В HAL-методах бэкенда запрещено выполнение любых операций биологических политик (уплотнение синапсов, сортировка, спраутинг, прунинг). Weaver (планировщик биологических шагов) не зависит от `compute-api`.
9. **Синхронное копирование в v1 (memcpy/DMA)**: Методы `export_maintenance_state` и `import_maintenance_state` в v1 всегда выполняют синхронный перенос байт (host memcpy для CPU бэкенда, синхронный DMA H2D/D2H для ускорителей). Использование zero-copy монопольного заимствования через `Box<dyn ComputeBackend>` в v1 запрещено.

---

## §6. Правила Валидации Параметров (Validation Rules)

Бэкенд обязан выполнять строгую валидацию входных DTO перед запуском вычислений:

1. **Alignment & Shape**: Параметр `spec.padded_n` должен быть кратен 64 (`PADDED_N_ALIGNMENT`).
2. **Limits**: Параметры `total_axons` и `total_ghosts` не должны превышать аппаратно допустимые лимиты.
3. **State Blob Size**: Размер `upload.state_blob.len()` должен строго совпадать с расчитанным размером состояния из `layout::calculate_state_blob_size(padded_n)`.
4. **Axons Blob Size**: Размер `upload.axons_blob.len()` валидируется на соответствие полному размеру файла аксонов по формуле `16 + total_axons * core::mem::size_of::<layout::BurstHeads8>()` (т.е. `16 + total_axons * 32`).
5. **v_seg Range**: Значение `cmd.v_seg` проверяется на физический диапазон `1 <= v_seg <= 255`. Значение передается уже посчитанным из `physics`.
6. **Spike Array Lengths**: Длины массивов `cmd.incoming_spike_counts.len()` and `cmd.output_spike_counts.len()` должны быть строго равны `cmd.sync_batch_ticks`.
7. **Incoming Spikes Validation**:
   - Если `cmd.incoming_spikes.is_some()`, то длина среза `incoming_spikes.unwrap().len()` должна быть не менее `cmd.sync_batch_ticks * cmd.max_spikes_per_tick`.
   - Если `cmd.incoming_spikes.is_none()`, то все элементы массива `cmd.incoming_spike_counts` должны быть строго равны `0`.
8. **Output Spikes Buffer Capacity**: Длина выходящего буфера `cmd.output_spikes.len()` должна быть не менее `cmd.sync_batch_ticks * cmd.max_spikes_per_tick`. Бэкенд обязан детерминированно перезаписывать и заполнять массив `cmd.output_spike_counts` количеством сгенерированных спайков для каждого тика.
9. **Input Bitmask Bounds**: Длина `cmd.input_bitmask` (при наличии `Some`) должна быть не менее `cmd.input_words_per_tick * cmd.sync_batch_ticks`.
10. **Mapped Soma IDs**: Длина `cmd.mapped_soma_ids` должна быть строго равна `cmd.num_outputs`.
11. **Handle Validation**: Использование недействительного, ранее освобожденного или чужого дескриптора должно мгновенно возвращать ошибку `InvalidHandle` или `ForeignHandle` без попыток разыменования памяти.
12. **Размер Буфера Экспорта Обслуживания (Maintenance Export)**: Длина `out.state_blob.len()` должна быть строго равна `layout::calculate_state_blob_size(padded_n)`, а `out.axons_blob.len()` — `expected_axons_blob_size(total_axons)` (которая должна возвращать `Ok`).
13. **Размер Буфера Импорта Обслуживания (Maintenance Import)**: Длина `src.state_blob.len()` должна быть строго равна `layout::calculate_state_blob_size(padded_n)`, а `src.axons_blob.len()` — `expected_axons_blob_size(total_axons)` (которая должна возвращать `Ok`).

### §6.1. Публичные Функции Валидации Обслуживания (Maintenance Validation APIs)
Для обеспечения унифицированной проверки буферов обслуживания во всех бэкендах, `compute-api` предоставляет следующие публичные функции:

```rust
pub fn validate_maintenance_export(
    spec: &ShardAllocSpec,
    out: &BackendMaintenanceMut<'_>,
) -> Result<(), ComputeApiError> {
    // 1. Валидация выравнивания и геометрии
    if spec.padded_n % 64 != 0 {
        return Err(ComputeApiError::AlignmentViolation);
    }
    // 2. Валидация размера блоба состояния
    let expected_state = layout::calculate_state_blob_size(spec.padded_n);
    if out.state_blob.len() != expected_state {
        return Err(ComputeApiError::SizeMismatch);
    }
    // 3. Валидация размера блоба аксонов
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if out.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::SizeMismatch);
    }
    Ok(())
}

pub fn validate_maintenance_import(
    spec: &ShardAllocSpec,
    src: &BackendMaintenanceRef<'_>,
) -> Result<(), ComputeApiError> {
    // 1. Валидация выравнивания и геометрии
    if spec.padded_n % 64 != 0 {
        return Err(ComputeApiError::AlignmentViolation);
    }
    // 2. Валидация размера блоба состояния
    let expected_state = layout::calculate_state_blob_size(spec.padded_n);
    if src.state_blob.len() != expected_state {
        return Err(ComputeApiError::SizeMismatch);
    }
    // 3. Валидация размера блоба аксонов
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if src.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::SizeMismatch);
    }
    Ok(())
}
```
*Примечание*: Данные методы осуществляют проверки размерностей и структуры C-ABI. Сбои валидации возвращают ошибки `SizeMismatch` или `InvalidShape`, в отличие от функции `validate_snapshot_buffers` для диагностических снимков состояния, которая оперирует границами отладчика и возвращает `InvalidDebugProbeBounds`.

---

## §7. Иерархия Ошибок (`ComputeApiError`)

Все ошибки вычислительного слоя транслируются в единый типизированный enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeApiError {
    InvalidHandle,
    ForeignHandle,
    AlreadyFreed,
    InvalidShape,
    AlignmentViolation,
    SizeMismatch,
    CapacityExceeded,
    OutOfMemory,
    DeviceLost,
    VendorError { code: i32 },
    DmaFailed,
    KernelLaunchFailed,
    SynchronizeFailed,
    UnsupportedBackend,
    UnsupportedFeature,
    BackendNotInitialized,
    InvalidBatch,
    InvalidDebugProbeBounds,
}
```

---

## §8. Требуемые Инварианты

- **INV-COMPUTE-API-001**: Трейт бэкенда `ComputeBackend` является объектно-безопасным (`Object Safe`) для использования через `Box<dyn ComputeBackend>`.
- **INV-COMPUTE-API-002**: Публичный API адресации VRAM использует непрозрачные дескрипторы `VramHandle` с приватными полями и фабричным методом `from_raw_parts`.
- **INV-COMPUTE-API-003**: Выделение и освобождение памяти VRAM выполняется через явные методы `alloc_shard` и `free_shard`.
- **INV-COMPUTE-API-004**: Все публичные методы бэкендов возвращают `Result<T, ComputeApiError>` and гарантируют отсутствие паник.
- **INV-COMPUTE-API-005**: Передача невалидного или освобожденного `VramHandle` мгновенно бракуется без обращения к C-ABI.
- **INV-COMPUTE-API-006**: Диспетчеризация вычислений выполняется пакетами на уровне батчей (`DayBatchCmd`), без вызова трейта на каждый отдельный тик.
- **INV-COMPUTE-API-007**: Временные границы срезов памяти внутри DTO жестко ограничены временами жизни кредитов Rust.

---

## §9. Golden Tests / Обязательная Матрица Тестирования

Крейт `compute-api` обязан быть покрыт набором тестов компиляции и валидации контрактов:

1. **Проверка Объектной Безопасности (`test_trait_object_safety`)**: Компиляционная проверка возможности создания `Box<dyn ComputeBackend>`.
2. **Браковка Невалидного Дескриптора (`test_reject_invalid_vram_handle`)**: Проверка возврата `InvalidHandle` при передаче неинициализированного `VramHandle`.
3. **Браковка Освобожденного Дескриптора (`test_reject_freed_vram_handle`)**: Проверка повторного вызова `free_shard` или `run_day_batch` на освобожденном дескрипторе.
4. **Проверка Неверного Выравнивания (`test_reject_misaligned_padded_n`)**: Браковка `padded_n`, не кратного 64.
5. **Проверка Валидации v_seg (`test_reject_invalid_v_seg`)**: Браковка вызова при `v_seg == 0` или `v_seg > 255`.
6. **Проверка Неверного Размера Блоба Состояния (`test_reject_bad_state_blob_size`)**: Проверка браковки `upload_shard` при несоответствии длины массива.
7. **Проверка Размера Блоба Аксонов (`test_validate_axons_blob_size_formula`)**: Проверка валидации длины `axons_blob` по формуле `16 + total_axons * 32`.
8. **Проверка Валидации Срезов Батча (`test_reject_insufficient_batch_slices`)**: Браковка вызова при недостаточной емкости `output_spikes` или расхождении длин `incoming_spike_counts`.
9. **Поведение `debug_snapshot` по умолчанию (`test_default_debug_snapshot_returns_unsupported`)**: Проверка возврата `Err(ComputeApiError::UnsupportedFeature)` базовым методом трейта.
10. **Валидация Буферов Снимок Состояния (`test_debug_snapshot_buffer_validation`)**: Проверка валидации границ и размеров срезов в `ShardSnapshotMut`.
11. **Гарантия Отсутствия Паник (`test_api_returns_result_never_panics`)**: Проверка возврата `Result` при любых некорректных параметрах рантайма.
12. **Автономность Реализации Mock-Бэкенда (`test_mock_backend_implementation`)**: Проверка полной реализации трейта `ComputeBackend` на Mock-бэкенде без внешних CUDA/HIP библиотек.
13. **Отсутствие Вендорских Флагов (`test_no_vendor_feature_flags`)**: Гарантия сборки крейта без флагов компиляции.
14. **Поведение `export_maintenance_state` по умолчанию (`test_default_export_maintenance_returns_unsupported`)**: Проверка возврата `Err(ComputeApiError::UnsupportedFeature)` базовым методом трейта.
15. **Поведение `import_maintenance_state` по умолчанию (`test_default_import_maintenance_returns_unsupported`)**: Проверка возврата `Err(ComputeApiError::UnsupportedFeature)` базовым методом трейта.
16. **Валидация буферов экспорта обслуживания (`test_validate_maintenance_export_buffers`)**: Проверка валидации размеров буферов в `BackendMaintenanceMut`.
17. **Валидация буферов импорта обслуживания (`test_validate_maintenance_import_buffers`)**: Проверка валидации размеров буферов в `BackendMaintenanceRef`.

---

## §10. Resolved Architectural Decisions (Принятые Решения Pass 2)

Все открытые вопросы архитектуры Layer 3 HAL успешно закрыты и зафиксированы в процессе системного прохода Pass 2:

1. **[RESOLVED] Легковесный no_std контракт (REV-COMPUTE-API-007 / Pass 2)**:
   - *Решение*: Крейт `compute-api` переведен в режим `#![no_std]`. Для DTO и трейта не требуется `alloc` и `std`. Создание `Box<dyn ComputeBackend>` является ответственностью фасада `compute`.
2. **[RESOLVED] Единый стандарт имен методов (REV-COMPUTE-API-001)**:
   - *Решение*: В трейте `ComputeBackend` утверждены строго имена `alloc_shard`, `upload_shard`, `run_day_batch`, `free_shard`, `teardown`. Имя `teardown` означает деинициализацию контекста бэкенда, а `free_shard` — освобождение VRAM конкретного шарда.
3. **[RESOLVED] Конструктор и фабрика `VramHandle` (REV-COMPUTE-CPU-001)**:
   - *Решение*: В `VramHandle` добавлен публичный безопасный конструктор `from_raw_parts(kind, id, generation)` и геттеры `kind()`, `id()`, `generation()`. Валидация принадлежности и жизни дескриптора выполняется бэкендом.
4. **[RESOLVED] Закрепление владения Pinned Host Memory (REV-COMPUTE-API-002)**:
   - *Решение*: В v2.1 не вводится отдельный DTO для pinned буферов. Владение Pinned Host буферами закрепляется за конкретными бэкендами (`compute-cuda`, `compute-hip`, `compute-cpu`). `ShardUpload` принимает обычные заимствованные срезы `&'a [u8]`.
5. **[RESOLVED] Синхронная модель выполнения батча (REV-COMPUTE-API-003)**:
   - *Решение*: Базовый метод `run_day_batch` строго синхронный (блокирующий). Асинхронная модель `submit_batch` / `poll_batch` оставлена как будущий extension-trait. Флаг `supports_async` в capabilities помечен как зарезервированный/внутренний.
6. **[RESOLVED] Разделение I/O буферов в `DayBatchCmd` и `BatchResult` (REV-COMPUTE-API-004)**:
   - *Решение*: В `DayBatchCmd` явно разделены входящие и выходящие данные. Выходящие спайковые ID записываются в `output_spikes: &'a mut [u32]`, а `BatchResult` возвращает только телеметрию и счетчики.
7. **[RESOLVED] Debug Snapshot API для test-harness (REV-TEST-001)**:
   - *Решение*: В `ComputeBackend` добавлен метод по умолчанию `debug_snapshot(&mut self, handle, snapshot: ShardSnapshotMut<'_>)`, возвращающий `UnsupportedFeature` по умолчанию.
8. **[RESOLVED] Детерминизм и отсутствие RNG Seed в `DayBatchCmd`**:
   - *Решение*: Подтверждено, что горячий цикл полностью детерминирован (Stochastic Heartbeat вычисляется от `tick_base`, neuron id и `heartbeat_m`). Передача RNG Seed в `DayBatchCmd` не требуется.
9. **[RESOLVED] Загрузка `.axons` блоба (Pass 2)**:
   - *Решение*: В v2.1 допускается только полная загрузка `ShardUpload`. Частичная загрузка аксонов является будущим расширением. Полный размер файла аксонов валидируется по формуле `16 + total_axons * 32`.
10. **[RESOLVED] Передача таблицы вариантов нейронов `variant_table` в `ShardUpload` (REV-COMPUTE-CUDA-002 / Pass 2.2)**:
    - *Решение*: В DTO `ShardUpload` добавлено фиксированное заимствованное поле `variant_table: &'a [layout::VariantParameters; layout::VARIANT_LUT_LEN]`. Структура `ShardUpload` предоставляет временное заимствованное представление (`borrowed view`) строго на время выполнения метода `upload_shard`. Каждый бэкенд обязан либо синхронно перенести таблицу в память устройства (`GPU Constant Memory`), либо скопировать её во внутреннее backend-owned хранилище ресурса шарда. Сохранять и удерживать ссылку из `ShardUpload` внутри бэкенда или ресурса после завершения вызова `upload_shard` категорически запрещено.
11. **[RESOLVED] Добавление контракта обслуживания для интеграции Night Phase (REV-MAINT-001)**:
    - *Решение*: В трейт `ComputeBackend` добавлен метод `export_maintenance_state` и `import_maintenance_state`, работающие в режиме жизненного цикла `Maintenance`. Состояние устройства передается через структуры `BackendMaintenanceMut` и `BackendMaintenanceRef`, содержащие исключительно `state_blob` и `axons_blob`. Пути аксонов (`paths_blob`) не входят в HAL-интерфейс, так как они никогда не загружаются на вычислительное устройство.
