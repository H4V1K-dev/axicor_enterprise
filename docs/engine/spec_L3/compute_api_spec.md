# spec_compute_api

> Версия спеки: 2.0  
> Дата: 2026-06-29  
> Статус: Draft (Architecture Pass 1)

---

## §1. Идентификация

| Поле | Значение |
|------|----------|
| **Имя крейта** | `compute-api` |
| **Слой** | Слой 3 — Абстракция Вычислений (Compute Hardware Abstraction Layer / HAL) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуются стандартные примитивы синхронизации и управление коллекциями |
| **Описание** | Аппаратно-независимый контракт вычислительных бэкендов (CPU, CUDA, HIP, Mock) для движка `AxiEngine`. Крейт определяет объектно-безопасный трейт `ComputeBackend`, непрозрачные дескрипторы VRAM, структуры команд DTO и типизированную иерархию ошибок. `compute-api` не владеет C-ABI макетами памяти, физическими формулами симуляции и низкоуровневыми FFI-вызовами конкретных ускорителей. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|-------|-----------------|-------|
| `types` (Слой 0) | Скалярные идентификаторы, константы типов | Использование фундаментальных типов (`Tick`, `Voltage`) в DTO вызовов. |
| `layout` (Слой 1) | Правила выравнивания и расчет размеров SoA-плоскостей | Валидация входных байтовых массивов на соответствие контрактам C-ABI выравнивания. |

> [!IMPORTANT]
> Крейт `compute-api` **не зависит** от крейта `physics`. Вычислительные DTO принимают уже готовые скалярные значения (например, pre-calculated `v_seg`), не вычисляя физику нейронов внутри API-слоя.

### §2.2. Зависимые компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|-------------------|---------------------------------|
| `compute` (Слой 3) | Фасад вычислений (`ShardEngine`) использует трейт `ComputeBackend` для динамического выбора и управления бэкендами. |
| `compute-cpu` (Слой 3) | Реализация вычислительного бэкенда для многопоточного CPU на базе SIMD. |
| `compute-cuda` (Слой 3) | Реализация вычислительного бэкенда для ускорителей NVIDIA CUDA. |
| `compute-hip` (Слой 3) | Реализация вычислительного бэкенда для ускорителей AMD ROCm/HIP. |

### §2.3. Внешние зависимости

Внешние зависимости отсутствуют. Сторонний код (вендорские SDK CUDA/HIP, Rayon) изолирован внутри конкретных реализаций бэкендов. Зависимость `anyhow` полностью выведена из публичного API.

### §2.4. Feature Flags и Вендорская Независимость

Крейт не содержит вендорских feature flags (`cuda`, `hip`). Публичный контракт является единым и нейтральным для всех аппаратных платформ.

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|----------------|-----------------------------------------------------|--------------------------------------------------------|
| **`compute-api`** (Слой 3) | **Публичные Rust-контракты вычислений**: Трейт бэкенда (`ComputeBackend`), непрозрачные дескрипторы ресурсов (`VramHandle`), структуры команд DTO (`DayBatchCmd`), DTO результатов (`BatchResult`), характеристики оборудования (`BackendCapabilities`), перечисление ошибок (`ComputeApiError`) и правила жизненного цикла ресурсов. | Запрещено утверждение владения макетами `ShardVramPtrs` (целевой владелец должен быть закреплен в `layout`, пока является межспецификационным долгом), `StateOffsets`, `.state`, `.axons`, `VariantParameters`, `BurstHeads8` (владелец `layout`), физическими формулами (владелец `physics`), вендорскими FFI-символами и стримами (владельцы `compute-cuda`/`hip`/`cpu`), авто-выбором бэкендов (владелец `compute`), а также парсингом сетевых пакетов или архивов. |
| **`layout`** (Слой 1) | **Макеты Памяти**: Физическая структура SoA-плоскостей памяти, C-ABI выравнивание (Целевой владелец `ShardVramPtrs`). | Запрещено управление вызовами выполнения батчей на вычислительных ускорителях. |
| **`physics`** (Слой 0) | **Физическая Математика**: Чистые формулы интеграции потенциалов и пластичности. | Запрещена привязка к буферам памяти и структурам вызовов бэкендов. |
| **Бэкенды** (`compute-cuda` / `hip` / `cpu`) | **Аппаратные Реализации**: Аллокация физической VRAM, вызовы CUDA/HIP API, FFI-указатели, стримы execution, трансляция ошибок вендора. | Запрещено изменение публичных DTO структур и нарушение объектной безопасности трейта. |

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
    pub supports_async: bool,        // Поддержка асинхронных стримов и DMA
    pub supports_ephys: bool,        // Поддержка съема осциллограмм в горячем цикле
    pub max_batch_ticks: u32,        // Максимальный размер батча тиков за один вызов
    pub alignment_bytes: usize,      // Требование выравнивания буферов (например, 64B)
    pub pinned_host_required: bool,  // Требование Pinned Host memory для DMA
}
```

### §4.2. Действительно Непрозрачный Дескриптор VRAM (`VramHandle`)
Адресация выделенной памяти на ускорителе выполняется строго через непрозрачный дескриптор с приватным полем. Внешний код не может вручную конструировать произвольные указатели:
```rust
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VramHandle(NonZeroU64);

impl VramHandle {
    // Внутренний конструктор доступен только внутри крейта вычислений
    pub(crate) fn new(id: NonZeroU64) -> Self {
        Self(id)
    }
}
```

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
}
```

### §4.4. Команда Выполнения Дневного Батча (`DayBatchCmd`)
Структура DTO содержит геометрию I/O и параметры для автономного выполнения батча тиков симуляции в горячем цикле:
```rust
#[derive(Debug)]
pub struct DayBatchCmd<'a> {
    pub tick_base: u64,
    pub sync_batch_ticks: u32,
    pub v_seg: u32,                   // Значение приходит уже посчитанным из physics (1..=255)
    pub dopamine: i16,
    pub input_words_per_tick: u32,    // Количество 32-битных слов входного битмаска на тик
    pub max_spikes_per_tick: u32,     // Максимальная емкость спайков за один тик
    pub num_outputs: u32,             // Количество опрашиваемых соматических выходов
    pub output_capacity_bytes: u32,   // Размер выделенного буфера результатов
    pub input_bitmask: Option<&'a [u32]>,
    pub incoming_spikes: Option<&'a [u32]>,
    pub spike_counts: &'a [u32],
    pub virtual_offset: u32,
    pub num_virtual_axons: u32,
    pub mapped_soma_ids: &'a [u32],
}
```

### §4.5. Результат Выполнения Батча (`BatchResult`)
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchResult {
    pub generated_spikes_count: u32,
    pub execution_time_us: u64,
}
```

### §4.6. Условные DTO Отладчика Ephys
*(Данные структуры являются условными и применяются строго при условии сохранения поддержки Ephys в scope бэкендов)*:
```rust
#[derive(Debug)]
pub struct EphysProbeCmd<'a> {
    pub target_soma_ids: &'a [u32],
    pub max_ticks: u32,
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
}
```

### §5.2. Правила Проектирования Трейта
1. **Объектная Безопасность (Object Safety)**: Трейт не содержит generic-методов или ассоциированных типов. Это позволяет инстанцировать бэкенд как динамический объект (`Box<dyn ComputeBackend>`).
2. **Безопасность Типов (Safe API)**: Ни один публичный метод трейта не принимает и не возвращает сырые указатели (`*mut u8`, `*const u8`). Сырой FFI-код разрешен только внутри конкретных бэкендов.
3. **Отсутствие Паник**: Все методы возвращают `Result<T, ComputeApiError>`. Паники внутри бэкендов запрещены.
4. **Батчевая Диспетчеризация (Batch-Level Dispatch)**: Вызов метода выполнения производится единоразово на весь батч тиков (`sync_batch_ticks`), а не на каждый тик отдельно.
5. **Явный Жизненный Цикл Ресурсов**: Ресурсы создаются и уничтожаются через явные методы API. Трейт `Drop` может присутствовать как защитный механизм от утечек, но не является основным путем освобождения и не должен вызывать паник.

### §5.3. Единый Жизненный Цикл Ресурсов (Resource Lifecycle)
Выполнение задач на бэкенде подчиняется строгой последовательности вызовов:
1. `init_backend()` — Инициализация контекста ускорителя.
2. `alloc_shard(spec)` — Аллокация буферов VRAM для шарда, возврат `VramHandle`.
3. `upload_shard(handle, upload)` — Zero-copy DMA перенос состояния в VRAM.
4. `run_day_batch(handle, cmd)` — Запуск автономного горячего цикла вычислений на `sync_batch_ticks`.
5. `download_output(handle)` / Доступ к результатам — Считывание сгенерированных спайков и телеметрии.
6. `free_shard(handle)` — Явное освобождение VRAM-ресурсов шарда.
7. `teardown()` — Деинициализация контекста бэкенда.

---

## §6. Правила Валидации Параметров (Validation Rules)

Бэкенд обязан выполнять строгую валидацию входных DTO перед запуском вычислений:

1. **Alignment & Shape**: Параметр `spec.padded_n` должен быть кратен 64 (`PADDED_N_ALIGNMENT`).
2. **Limits**: Параметры `total_axons` и `total_ghosts` не должны превышать аппаратно допустимые лимиты.
3. **State Blob Size**: Размер `upload.state_blob.len()` должен строго совпадать с расчитанным размером состояния из `layout`.
4. **Axons Blob Size**: Размер `upload.axons_blob.len()` валидируется на соответствие байтовому размеру таблицы аксонов.
5. **v_seg Range**: Значение `cmd.v_seg` проверяется на физический диапазон `1 <= v_seg <= 255` (ограничение задается размером сегментного поля в `PackedTarget`). Значение передается уже посчитанным из `physics`.
6. **Spike Array Lengths**: Длина массива `cmd.spike_counts.len()` должна быть строго равна `cmd.sync_batch_ticks`.
7. **Spike Stride Capacity**: Каждое значение внутри `cmd.spike_counts[i]` не должно превышать `cmd.max_spikes_per_tick`.
8. **Input Bitmask Bounds**: Длина `cmd.input_bitmask` (при наличии) должна строго соответствовать `cmd.input_words_per_tick * cmd.sync_batch_ticks`.
9. **Mapped Soma IDs**: Длина `cmd.mapped_soma_ids` должна быть строго равна `cmd.num_outputs`.
10. **Handle Validation**: Использование недействительного, ранее освобожденного или чужого дескриптора должно мгновенно возвращать ошибку `InvalidHandle` или `ForeignHandle` без попыток разыменования памяти.

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
- **INV-COMPUTE-API-002**: Публичный API адресации VRAM использует только непрозрачные дескрипторы `VramHandle` с приватными полями без раскрытия сырых указателей.
- **INV-COMPUTE-API-003**: Выделение и освобождение памяти VRAM выполняется через явные методы вызовов `alloc_shard` и `free_shard`.
- **INV-COMPUTE-API-004**: Все публичные методы бэкендов возвращают `Result<T, ComputeApiError>` и гарантируют отсутствие паник.
- **INV-COMPUTE-API-005**: Передача невалидного или освобожденного `VramHandle` мгновенно бракуется без обращения к C-ABI.
- **INV-COMPUTE-API-006**: Диспетчеризация вычислений выполняется пакетами на уровне батчей (`DayBatchCmd`), без вызова трейта на каждый отдельный тик.
- **INV-COMPUTE-API-007**: Временные границы срезов памяти внутри DTO жестко ограничены временам жизни кредитов Rust.

---

## §9. Golden Tests / Обязательная Матрица Тестирования

Крейт `compute-api` обязан быть покрыт набором тестов компиляции и валидации контрактов:

1. **Проверка Объектной Безопасности (`test_trait_object_safety`)**: Компиляционная проверка возможности создания `Box<dyn ComputeBackend>`.
2. **Браковка Невалидного Дескриптора (`test_reject_invalid_vram_handle`)**: Проверка возврата `InvalidHandle` при передаче неинициализированного `VramHandle`.
3. **Браковка Освобожденного Дескриптора (`test_reject_freed_vram_handle`)**: Проверка повторного вызова `free_shard` или `run_day_batch` на освобожденном дескрипторе.
4. **Проверка Неверного Выравнивания (`test_reject_misaligned_padded_n`)**: Браковка `padded_n`, не кратного 64.
5. **Проверка Валидации v_seg (`test_reject_invalid_v_seg`)**: Браковка вызова при `v_seg == 0` или `v_seg > 255`.
6. **Проверка Неверного Размера Блоба Состояния (`test_reject_bad_state_blob_size`)**: Проверка браковки `upload_shard` при несоответствии длины массива.
7. **Проверка Превышения Емкости Спайков (`test_reject_oversized_spike_count`)**: Браковка батча при превышении `max_spikes_per_tick`.
8. **Проверка Несоответствия Длин Массивов Батча (`test_reject_mismatched_batch_arrays`)**: Браковка вызова при несоответствии длин битмасков или `mapped_soma_ids`.
9. **Гарантия Отсутствия Паник (`test_api_returns_result_never_panics`)**: Проверка возврата `Result` при любых некорректных параметрах рантайма.
10. **Автономность Реализации Mock-Бэкенда (`test_mock_backend_implementation`)**: Проверка полной реализации трейта `ComputeBackend` на Mock-бэкенде без внешних CUDA/HIP библиотек.
11. **Единство DTO для всех Бэкендов (`test_shared_dto_names_across_backends`)**: Проверка использования единых структур DTO бэкендами CPU, CUDA и HIP.
12. **Проверка Границ Отладчика Ephys (`test_ephys_probe_bounds`)**: *(Условный тест)* Проверка валидации границ при использовании отладочных зондов.
13. **Отсутствие Вендорских Флагов (`test_no_vendor_feature_flags`)**: Гарантия сборки крейта без флагов компиляции.

---

## §10. Open Questions / Review Debt (Открытые Вопросы и Противоречия)

В процессе анализа спецификации Compute API выявлены следующие открытые вопросы для согласования:

1. **Поддержка Окружений `no_std + alloc`**:
   - *Контекст*: Крейт `compute-api` содержит только абстрактные контракты и DTO.
   - *Вопрос*: Требуется ли перевести `compute-api` в режим `no_std + alloc` для поддержки встраиваемых систем (Edge devices)?

2. **Модель Владения Pinned Host Буферами**:
   - *Контекст*: Для скоростного DMA переноса требуются закрепощенные страницы памяти хоста (Pinned Memory).
   - *Вопрос*: Кто должен монопольно владеть Pinned-буферами — DTO дескриптор API, сам бэкенд или вышележащий фасад `compute`?

3. **Модель Выполнения Батча (Синхронная vs Асинхронная)**:
   - *Контекст*: Метод `run_day_batch` может быть блокирующим синхронным вызовом или асинхронной моделью сабмита со сплитом `submit_batch` / `sync_batch`.
   - *Вопрос*: Зафиксировать ли строго синхронную модель выполнения батча на уровне API?

4. **Точная Форма DTO Результатов Телеметрии (`BatchResult`)**:
   - *Контекст*: Структура результатов пока содержит минимальный набор полей.
   - *Вопрос*: Какая точная структура массива сгенерированных спайков должна возвращаться в `BatchResult`?

5. **Допустимость Частичной Загрузки Таблицы Аксонов (`.axons`)**:
   - *Контекст*: Таблица аксонов может быть огромной.
   - *Вопрос*: Допускается ли частичная загрузка (partial upload) буфера аксонов, или требовать только полный единовременный блоб?

6. **Зона Владения Вспомогательных Команд Сортировки и Синхронизации**:
   - *Контекст*: Команды `sort_and_prune`, синхронизация Ghost-аксонов и отладочные вызовы Ephys пересекаются с различными слоями.
   - *Вопрос*: Относятся ли данные методы к `compute-api` или выносятся на уровень фасада `compute` / `runtime`?
