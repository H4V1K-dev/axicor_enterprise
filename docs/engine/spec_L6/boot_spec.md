# spec_boot

> Версия спеки: 2.1  
> Дата: 2026-07-01  

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `boot` |
| **Слой** | Слой 6 — Runtime Orchestration & Node Startup (`L6`) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется `std` для работы с файловой системой ОС, путями и динамической аллокацией |
| **Описание** | Загрузчик локальных архивов шарда (Local Shard Archive Loader) для AxiEngine. В рамках Stage A крейт отвечает за открытие локального контейнера `.axic` (созданного `baker` Stage B) через `vfs::AxicArchive`, извлечение 4 обязательных бинарных файлов локального шарда, валидацию их C-ABI заголовков (`layout`) и размеров, а также подготовку owned-структур загрузки (`LocalShardBootBundle`) для передачи их фасаду вычислительного ядра `compute`. |

---

## §1.1. Scope of Stage A (Границы реализации Stage A)

### Входит в Stage A:
1. **Загрузка архива**: Открытие `.axic` файла через `vfs::AxicArchive`.
2. **Проверка состава файлов**: Контейнер обязан содержать минимальный обязательный набор из 4 файлов: `state.bin`, `axons.bin`, `paths.bin`, `variant_table.bin`. Любые дополнительные файлы в `.axic` Stage A игнорирует, не валидирует и не считает ошибкой.
3. **Валидация заголовков**: Парсинг и верификация magic-чисел, версий форматов и внутренних размерностей в заголовках `StateFileHeader`, `AxonsFileHeader`, `PathsFileHeader` с использованием alignment-safe механизмов разбора.
4. **Валидация соответствия размеров**: Проверка соответствия размеров извлеченных сырых буферов вычисленным по формулам `layout`.
5. **Подготовка структуры спецификаций**: Сборка структуры `compute_api::ShardAllocSpec` на основе проверенных заголовков.
6. **Вызов внешней валидации**: Вызов `compute_api::validation::validate_upload` на подготовленной спецификации и данных.
7. **Копирование данных в owned buffers**: Чтение данных из архива во владение (`Vec<u8>` и LUT-массив для параметров вариантов) с обеспечением безопасного выравнивания (alignment-safe).
8. **Optional Helper Bootstrap**: Вспомогательный метод `bootstrap_local_shard_engine` для инициализации движка `compute::ShardEngine` на том же потоке.

### НЕ входит в Stage A (Отложено):
1. **Обработка манифеста**: Чтение и парсинг `manifest.toml` или `department.toml`.
2. **Парсинг TOML**: Использование TOML/JSON парсеров.
3. **Обход host-директорий**: Логика directory walking.
4. **Временные директории**: Распаковка файлов в RAM-диск / tmpfs.
5. **IPC / SHM ресурсы**: Создание общих сегментов памяти, Swapchains или автоматов состояний IPC.
6. **Сетевой стек**: Таблицы маршрутизации RCU и BSP-барьеры `net`.
7. **Оркестрация рантайма**: Day/Night переходы и управление тиками в `runtime`.
8. **Мультишардовый макет**: Маршрутизация путей между шардами.
9. **Контрольные суммы и сжатие**: Хэширование файлов или сжатие TOC.
10. **GPU бэкенды**: Прямые зависимости от конкретных крейтов реализации движков `compute-cpu`/`compute-cuda`/`compute-hip`.

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `vfs` (Слой 2) | `AxicArchive`, `VfsError` | Открытие контейнера и безопасное извлечение сырых байтовых срезов файлов. |
| `layout` (Слой 1) | `VariantParameters`, `StateFileHeader`, `AxonsFileHeader`, `PathsFileHeader`, константы magic/версий, `VARIANT_LUT_LEN`, `MAX_SEGMENTS_PER_AXON`, математика размеров. | Парсинг C-ABI заголовков, проверка соответствия размерностей и выравниваний. |
| `compute-api` (Слой 3) | `ShardAllocSpec`, `ShardUpload`, `validation::validate_upload` | Сборка спецификации выделения VRAM, представление загружаемых ресурсов и верификация. |
| `compute` (Слой 3) | `BackendPreference`, `ShardEngine`, `ComputeError` | Фасадный запуск движка вычислений (только для optional helper). |

---

## §3. API Stage A

### §3.1. Структуры данных

```rust
/// Входные параметры для загрузки архива локального шарда.
pub struct LocalShardBootInput {
    /// Путь к .axic контейнеру на хосте.
    pub archive_path: std::path::PathBuf,
    /// Виртуальное смещение идентификаторов сом/аксонов в глобальной сети.
    pub virtual_offset: u32,
    /// Количество ghost-аксонов, заведенных для шарда.
    pub total_ghosts: u32,
}

/// Параметры для автоматического бутстрапа вычислительного фасада.
pub struct LocalShardComputeInput {
    /// Путь к .axic контейнеру.
    pub archive_path: std::path::PathBuf,
    /// Предпочтительный вычислительный бэкенд (CUDA, HIP, CPU).
    pub backend_preference: compute::BackendPreference,
    /// Виртуальное смещение идентификаторов.
    pub virtual_offset: u32,
    /// Количество ghost-аксонов.
    pub total_ghosts: u32,
}

/// Владеющий пакет скомпилированных данных шарда в памяти.
pub struct LocalShardBootBundle {
    /// Спецификация размещения ресурсов VRAM.
    pub spec: compute_api::ShardAllocSpec,
    /// Owned байты SoA-состояния сом и синапсов.
    pub state_blob: Vec<u8>,
    /// Owned байты заголовков импульсов аксонов.
    pub axons_blob: Vec<u8>,
    /// Owned байты координатных сеток путей.
    pub paths_blob: Vec<u8>,
    /// Выровненный owned LUT параметров вариантов нейронов.
    pub variant_table: [layout::VariantParameters; layout::VARIANT_LUT_LEN],
}

impl LocalShardBootBundle {
    /// Возвращает заимствованное представление (borrowed view) для загрузки в compute.
    pub fn upload(&self) -> compute_api::ShardUpload<'_> {
        compute_api::ShardUpload {
            state_blob: &self.state_blob,
            axons_blob: &self.axons_blob,
            variant_table: &self.variant_table,
        }
    }
}
```

### §3.2. Возможные ошибки

```rust
#[derive(Debug, thiserror::Error)]
pub enum BootError {
    /// Ошибки при чтении или валидации VFS контейнера.
    #[error("VFS error: {0}")]
    Vfs(#[from] vfs::VfsError),

    /// Сбои при вызове валидации compute-api.
    #[error("Compute API validation error: {0}")]
    ComputeApi(#[from] compute_api::ComputeApiError),

    /// Ошибки инициализации или аллокации вычислительного движка.
    #[error("Compute engine bootstrap error: {0}")]
    Compute(#[from] compute::ComputeError),

    /// В оглавлении архива отсутствует один из 4 обязательных файлов.
    #[error("Missing required file in archive: {path}")]
    MissingRequiredFile { path: &'static str },

    /// Нарушение спецификации заголовка или размерности конкретного блоба.
    #[error("Invalid artifact header/content for {path}: {reason}")]
    InvalidArtifact { path: &'static str, reason: &'static str },

    /// Размер извлеченного файла variant_table.bin не соответствует LUT.
    #[error("Variant parameters table size mismatch (expected {expected}, found {actual})")]
    VariantTableSizeMismatch { expected: usize, actual: usize },
}
```

### §3.3. Точки входа

```rust
/// Открывает архив шарда, считывает данные в owned buffers и выполняет ABI-валидацию.
pub fn load_local_shard_archive(
    input: &LocalShardBootInput,
) -> Result<LocalShardBootBundle, BootError>;

/// Склеенный вызов: считывает архив и сразу инициализирует движок вычислений ShardEngine.
pub fn bootstrap_local_shard_engine(
    input: &LocalShardComputeInput,
) -> Result<(compute::ShardEngine, LocalShardBootBundle), BootError>;
```

---

## §4. Правила валидации заголовков и данных (Header & Data Validation)

> [!IMPORTANT]
> **Требование к выравниванию (Alignment-Safe Parsing)**:
> Так как байтовые срезы файлов, извлекаемые из `vfs::AxicArchive`, возвращаются в виде ссылок на `mmap` область памяти, они не гарантируют выравнивания по границам структур `StateFileHeader`, `AxonsFileHeader`, `PathsFileHeader` или `VariantParameters`.
> Загрузчик **категорически обязан** выполнять alignment-safe разбор всех заголовков и данных (например, через `bytemuck::pod_read_unaligned`, копирование во временную локальную переменную типа заголовка или побайтовое чтение `from_le_bytes`).
> Использование `bytemuck::from_bytes` или `bytemuck::cast_slice` напрямую на байтах из архива без предварительного выравнивания/копирования **запрещено** и приводит к неопределенному поведению.

При разборе файлов в `load_local_shard_archive` загрузчик обязан применить следующие проверки:

### 1. Файл `state.bin`
- **Размер**: Сырой размер буфера должен быть `>= 16` байт.
- **Magic**: Первые 4 байта равны `layout::STATE_MAGIC`.
- **Версия**: Версия формата равна `layout::STATE_FILE_VERSION`.
- **padded_n**: Выровненное число сом `padded_n` из заголовка должно быть строго `> 0` и кратно границе выравнивания `layout::PADDED_N_ALIGNMENT`.
- **Размер буфера**: Длина буфера `state_blob.len()` должна быть в точности равна вычисленному значению `layout::calculate_state_blob_size(padded_n)`.

### 2. Файл `axons.bin`
- **Размер**: Сырой размер буфера должен быть `>= 16` байт.
- **Magic**: Первые 4 байта равны `layout::AXONS_MAGIC`.
- **Версия**: Версия формата равна `layout::AXONS_FILE_VERSION`.
- **total_axons**: Число аксонов `total_axons` в заголовке должно строго совпадать с `total_axons` из ранее проверенного заголовка `state.bin`.
- **Размер буфера**: Длина буфера `axons_blob.len()` должна быть в точности равна `compute_api::validation::expected_axons_blob_size(total_axons)` (или аналогичной формуле размера из `layout`).

### 3. Файл `paths.bin`
- **Размер**: Сырой размер буфера должен быть `>= 16` байт.
- **Magic**: Первые 4 байта равны `layout::PATHS_MAGIC`.
- **Версия**: Версия формата равна `layout::PATHS_FILE_VERSION`.
- **total_axons**: Должно строго совпадать с `total_axons` из заголовка `state.bin`.
- **max_segments**: Значение `max_segments` в заголовке должно строго быть равно `layout::MAX_SEGMENTS_PER_AXON`.
- **Размер буфера**: Длина буфера `paths_blob.len()` должна быть в точности равна `layout::calculate_paths_file_size(total_axons)`.

### 4. Файл `variant_table.bin`
- **Размер**: Длина буфера должна быть строго равна `std::mem::size_of::<layout::VariantParameters>() * layout::VARIANT_LUT_LEN`. Любое отклонение вызывает `BootError::VariantTableSizeMismatch`.
- **Безопасное копирование**: Для предотвращения UB на невыровненных срезах (из-за прямого mmap VFS-буфера), загрузчик обязан использовать alignment-safe чтение. Байты копируются во внутренний стек/кучу в локальный массив `[VariantParameters; VARIANT_LUT_LEN]` (например, через `bytemuck::pod_read_unaligned` или прямое копирование байт в локальный выровненный буфер).

---

## §5. Сборка AllocSpec и Сквозная Валидация

На основе проверенных заголовков `boot` собирает промежуточную структуру:

```rust
let spec = compute_api::ShardAllocSpec {
    padded_n: state_header.padded_n,
    total_axons: state_header.total_axons,
    total_ghosts: input.total_ghosts,
    virtual_offset: input.virtual_offset,
};
```

Далее загрузчик собирает временное представление `ShardUpload` (вызовом `bundle.upload()`) и вызывает:
```rust
compute_api::validation::validate_upload(&spec, &upload)?;
```
Только после прохождения этой верификации буфер с данными считается валидным для возврата вызывающему или инициализации бэкенда.

---

## §6. Требуемые Инварианты Stage A

- **INV-BOOT-A01**: `boot` не парсит оглавление архива (TOC) `.axic` напрямую на байтовом уровне, а делегирует эту задачу библиотеке `vfs` (через метод `AxicArchive::open` и вызовы `get_file`).
- **INV-BOOT-A02**: Перечень обязательных файлов Stage A (`Required Files`) равен минимальному набору: `state.bin`, `axons.bin`, `paths.bin`, `variant_table.bin`. Любые другие/дополнительные файлы в архиве (например, манифесты) полностью игнорируются на этой стадии.
- **INV-BOOT-A03**: `boot` копирует извлеченные данные в owned buffers и возвращает `LocalShardBootBundle`. После возврата бандла хэндл `AxicArchive` закрывается, и в памяти не остается borrowed-ссылок на исходный файл архива.
- **INV-BOOT-A04**: Представление `compute_api::ShardUpload` создается исключительно как временное заимствованное отображение (borrowed view) поверх живой структуры `LocalShardBootBundle`.
- **INV-BOOT-A05**: Валидация загрузки (`validate_upload`) должна быть выполнена и завершена успешно до вызова `compute::ShardEngine::bootstrap`.
- **INV-BOOT-A06**: Координатная сетка `paths.bin` считывается и валидируется на этапе загрузки, но не передается в `ShardUpload` при инициализации вычислительного ядра (так как вычислительный цикл не использует геометрические траектории сегментов в Stage A).
- **INV-BOOT-A07**: Независимость от `baker`. Крейт `boot` не импортирует `baker` и не зависит от него напрямую. Пути файлов Stage A объявляются локальными константами внутри `boot` и должны по строковым значениям совпадать с константами baker Stage B:
  - `state.bin`
  - `axons.bin`
  - `paths.bin`
  - `variant_table.bin`

---

## §7. Обязательные тесты Stage A

1. **Успешная загрузка архива (`test_boot_stage_a_load_baker_axic_success`)**: Проверка полной загрузки валидного `.axic` контейнера в `LocalShardBootBundle` и сверка полей.
2. **Сбой при отсутствии файла (`test_boot_stage_a_missing_required_file`)**: Проверка возврата ошибки `MissingRequiredFile`, если из архива удален, например, `variant_table.bin`.
3. **Браковка битого заголовка state.bin (`test_boot_stage_a_reject_bad_state_header`)**: Проверка возврата ошибки `InvalidArtifact` при несовпадении magic/версии или невыровненном `padded_n`.
4. **Браковка битого заголовка axons.bin (`test_boot_stage_a_reject_bad_axons_header`)**: Проверка возврата ошибки `InvalidArtifact` при несовпадении `total_axons` с ранее разобранным `state.bin`.
5. **Браковка битого заголовка paths.bin (`test_boot_stage_a_reject_bad_paths_header`)**: Проверка возврата ошибки `InvalidArtifact` при неверном `max_segments` или `total_axons`.
6. **Браковка некорректного размера LUT таблицы (`test_boot_stage_a_reject_variant_table_size`)**: Проверка возврата `VariantTableSizeMismatch` при коротком файле вариантов.
7. **Сборка AllocSpec по заголовкам (`test_boot_stage_a_alloc_spec_from_headers`)**: Проверка точного переноса `padded_n` и `total_axons` из заголовков файлов в спецификацию `ShardAllocSpec`.
8. **Равенство временного представления owned буферам (`test_boot_stage_a_upload_view_matches_owned_buffers`)**: Проверка, что байты в `bundle.upload()` совпадают с байтами owned-полей в бандле.
9. **Сквозная валидация compute-api (`test_boot_stage_a_compute_api_validation`)**: Проверка вызова `validate_upload` и выявления несоответствия размерностей.
10. **Тестовый бутстрап движка вычислений (`test_boot_stage_a_bootstrap_mock_or_cpu_engine`)**: Проверка успешной инициализации `compute::ShardEngine` через вспомогательный хелпер `bootstrap_local_shard_engine`.
