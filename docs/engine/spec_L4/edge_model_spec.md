# spec_edge_model

> Версия спеки: 2.0  
> Дата: 2026-06-29  
> Статус: Draft (Architecture Pass 1)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `edge-model` |
| **Слой** | Слой 4 — Geometry, Growth & Connectome Generation (`L4`) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — требуется доступ к файловой системе для записи отчетов, работа с векторами, системным аллокатором и сериализация JSON |
| **Описание** | Библиотека оффлайн-конвертации и проекции собранных десктопных `.axic` моделей в артефакты для встраиваемых платформ (Edge-Inference Only). Крейт осуществляет Winner-Take-All (WTA / top-K) срез и уплотнение дендритных слотов, разделение графа памяти на Read-Write (SRAM) и Read-Only (Flash) артефакты, выравнивание Flash-памяти под границы MMU-страниц целевой платформы (64 KB на ESP32-S3), а также генерацию манифеста отчуждения и проверочных C-заголовков. Крейт не владеет рантаймом микроконтроллера, прошивкой, циклом исполняющего устройства, сетевыми интерфейсами или обучением на устройстве. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `PackedTarget`, `EMPTY_PIXEL`, `AXON_SENTINEL`, базовые типы весов | Использование фундаментальных типов координат, упаковки синапсов и маркеров пустых слотов. |
| `layout` (Слой 1) | `StateFileHeader`, `AxonsFileHeader`, `VariantParameters`, `BurstHeads8`, `StateOffsets`, `MAX_DENDRITES`, математика размеров блобов | Чтение C-ABI макетов SoA-плоскостей, выравниваний и смещений полей. |
| `vfs` (Слой 2) | `AxicArchive`, `require_file`, `get_file`, `list_files` | Чтение скомпилированных файлов `.state`, `.axons` и таблицы вариантов из архива `.axic` без распаковки. |

### §2.2. Зависимые Компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `baker-cli` (Слой 4) | Вызывает `edge-model` при сборке с включенной Cargo feature `edge` для выполнения подкоманды `edge-convert`. |
| Edge Runtime / Bare-Metal Firmwares | Принимают сгенерированные `shard.sram` и `shard.flash` для исполнения нейронной сети в режиме только инференса. |

### §2.3. Внешние Зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `thiserror` | `=1.0.69` | Формирование строгой иерархии публичных ошибок библиотеки (`EdgeError`). |
| `bytemuck` | `=1.25.0` | Безопасный нуль-копийный каст байтовых слайсов при нарезке соа-плоскостей. |
| `serde` | `=1.0.228` | Сериализация метаданных отчетов и манифеста. |
| `serde_json` | `=1.0.117` | Форматирование машиночитаемого манифеста `EdgeManifest`. |
| `tracing` | `=0.1.40` | Логирование этапов конвертации и прореживания связей. |
| `sha2` | `=0.10.8` | Вычисление побитовых хэш-сумм SHA-256 сгенерированных edge-блобов. |

> [!IMPORTANT]
> Настоящая спецификация запрещает прямое использование `anyhow` в публичном API крейта `edge-model` (использование `anyhow` допустимо только на бинарном уровне CLI). Все внешние утилитарные версии зафиксированы как рабочие флаги библиотеки (централизованный учет версий вынесен в review debt, §12). Прямые зависимости от вычислительных бэкендов (`compute`, `compute-api`, `compute-cuda`, `compute-hip`), IPC (`ipc`), сетевых модулей (`wire`, `net`, `transport`), рантайма (`boot`) и компилятора (`baker`) категорически запрещены.

### §2.4. Feature Flags

Секция публичных feature flags не используется. Крейт собирается как единая библиотека конвертации.

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`edge-model`** (Слой 4) | **Оффлайн-Конвертация и Производный Edge-Формат**: Правила WTA/top-K прореживания дендритных слотов, разделение памяти на Read-Write (SRAM) и Read-Only (Flash) блобы, выравнивание Flash-блоба до границы MMU-страниц целевой платформы, формирование манифеста `EdgeManifest` и производных проверочных C-заголовков (`edge_model.h`). | Запрещены объявление и изменение C-ABI полей исходных десктопных `.state`, `.axons`, `.paths` (владелец `layout`), формат архива `.axic` и его оглавление TOC (владелец `vfs`), TOML DTO и DSL-валидация (владелец `config`), формулы физики GLIF/GSOP (владелец `physics`), упаковка синапсов `PackedTarget` (владелец `types`), CLI-грамматика (владелец `baker-cli`), а также рантайм-цикл исполнения устройств (будущий edge runtime). |
| **`layout`** (Слой 1) | **Макеты Памяти и Полей**: Исходный макет SoA-плоскостей десктопной модели (`ShardStateSoA`) и лимит `MAX_DENDRITES` (128). | Запрещено выполнение алгоритмов прореживания синапсов под конкретные железячные лимиты. |
| **`vfs`** (Слой 2) | **Доступ к Контейнерам**: Извлечение файлов из `.axic` архива. | Запрещен анализ и перекомпоновка семантического содержимого памяти. |

---

## §4. Публичная API-Модель (Public API Model)

Ядро конвертации работает исключительно с байтовыми слайсами и типизированными структурами в памяти. Физическая запись на диск вынесена в отдельный утилитарный хелпер:

```rust
pub struct EdgeConversionOptions {
    pub target_profile: EdgeTargetProfile,
    pub override_dendrite_slots: Option<usize>,
    pub override_flash_page_size: Option<usize>,
    pub sram_budget_bytes: usize,
    pub emit_c_headers: bool,
}

impl EdgeConversionOptions {
    /// Получение итогового числа дендритных слотов K с учетом пресета и оверрайда
    pub fn effective_dendrite_slots(&self) -> usize {
        self.override_dendrite_slots
            .unwrap_or_else(|| self.target_profile.default_dendrite_slots())
    }

    /// Получение итогового размера страницы MMU Flash с учетом пресета и оверрайда
    pub fn effective_flash_page_size(&self) -> usize {
        self.override_flash_page_size
            .unwrap_or_else(|| self.target_profile.default_flash_page_size())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeTargetProfile {
    Esp32S3,
    GenericMcu {
        default_slots: usize,
        default_page_size: usize,
    },
}

impl EdgeTargetProfile {
    pub fn default_dendrite_slots(&self) -> usize {
        match self {
            Self::Esp32S3 => 32,
            Self::GenericMcu { default_slots, .. } => *default_slots,
        }
    }

    pub fn default_flash_page_size(&self) -> usize {
        match self {
            Self::Esp32S3 => 65_536,
            Self::GenericMcu { default_page_size, .. } => *default_page_size,
        }
    }
}

pub struct EdgeSourceBlobs<'a> {
    pub state_blob: &'a [u8],
    pub axons_blob: &'a [u8],
    pub paths_blob: Option<&'a [u8]>,
    pub variant_table_blob: &'a [u8],
}

pub struct EdgeBundle {
    pub sram_blob: Vec<u8>,
    pub flash_blob: Vec<u8>,
    pub manifest: EdgeManifest,
    pub c_header: Option<String>,
}

pub struct EdgeWriteReport {
    pub sram_path: PathBuf,
    pub flash_path: PathBuf,
    pub manifest_path: PathBuf,
    pub c_header_path: Option<PathBuf>,
    pub bytes_written: usize,
}

pub fn convert_archive(
    archive: &AxicArchive,
    options: &EdgeConversionOptions,
) -> Result<EdgeBundle, EdgeError>;

pub fn convert_blobs(
    sources: EdgeSourceBlobs<'_>,
    options: &EdgeConversionOptions,
) -> Result<EdgeBundle, EdgeError>;

pub fn write_bundle(
    bundle: &EdgeBundle,
    out_dir: &Path,
) -> Result<EdgeWriteReport, EdgeError>;
```

---

## §5. Алгоритм WTA / Top-K Прореживания Дендритов (Trimming & Output Stride)

1. **Размерность Входа и Ограничения**: Исходная ширина дендритного ряда составляет `layout::MAX_DENDRITES` (128 слотов). Эффективное число слотов $K$ (`effective_dendrite_slots()`) должно находиться строго в диапазоне `1..=128`. Значение $K = 0$ вызывает немедленный возврат ошибки `EdgeError::InvalidTargetDendriteSlots`.
2. **Определяющий Стрид Выходного Блоба (Output Array Stride)**: Финальные массивы `dendrite_targets`, `dendrite_weights` и `dendrite_timers` в сжатых edge-блобах сжимаются до физического размерного стрида **$K$**, а не 128. Размерность выходных массивов составляет ровно `[padded_n * K]`.
3. **Определение Живого Слота**: Слот внутри исходного 128-слотового ряда считается живым, если его таргет не равен `PackedTarget(0)` (сырой нуль) и не равен `EMPTY_PIXEL` (`0xFFFF_FFFF`), а синаптический вес имеет ненулевую величину.
4. **Ранжирование по Силе (Sorting & Dale's Law)**: Синапсы сортируются по убыванию модулей весов:
   $$\text{Primary Key: } |\text{weight}| = \text{weight.unsigned\_abs() (descending)}$$
   Знак веса сохраняется в точности без изменений для соблюдения Закона Дейла.
5. **Разрешение Ничьих (Deterministic Tie-Breaker)**: При равенстве абсолютных весов используется детерминированный порядок:
   $$\text{Magnitude (descending)} \longrightarrow \text{Original Slot Index (ascending)} \longrightarrow \text{Raw Target Value (ascending)}$$
6. **Уплотнение и Маркировка Заполнения Слотов**: Отобранные верхние живые синапсы (не более $K$) уплотняются влево (`0..active_count-1`). Если количество живых синапсов у нейрона меньше $K$, неиспользованные слоты внутри сжатого ряда (от `active_count` до $K-1$) заполняются хард-маркером `EMPTY_PIXEL`, а их веса и таймеры зануляются.
7. **Правило Маркеров Пустоты**: В сгенерированных edge-блобах запрещено создавать новые сырые нули `PackedTarget(0)` в качестве маркеров пустого слота (сырой нуль принимается только при чтении legacy-данных на входе). Строго сохраняется синхронное выравнивание тройки колонок `targets`, `weights` и `dendrite_timers`.

---

## §6. Физическое Разделение Памяти (Memory Split Policy)

Десктопный массив SoA декомпозируется на два независимых бинарных домена в зависимости от требования к мутабельности в горячем цикле исполнения устройства:

1. **Read-Only Flash Domain (`shard.flash`)**:
   - `dendrite_targets` (сжатые до стрида $K$ слотов на нейрон, всего `padded_n * K`);
   - `dendrite_weights` (замороженные веса синапсов со стридом $K$);
   - `variant_parameters` (таблица физических типов нейронов);
   - Статические массивы топологии, необходимые для чистой инференс-маршрутизации.
2. **Read-Write SRAM Domain (`shard.sram`)**:
   - `soma_voltage` (текущие мембранные потенциалы сом);
   - `soma_flags` (флаги спайков и состояния);
   - `threshold_offset` (адаптивные пороги);
   - `timers` (рефрактерные таймеры сом);
   - `axon_heads` (указатели/индексы аксонных головок);
   - `dendrite_timers` (таймеры дендритных слотов со стридом $K$, если инференс-рантайм использует рефрактерность синапсов).

---

## §7. Выравнивание Памяти Flash и MMU (Execute-In-Place Padding)

1. **Требование XIP (Execute-In-Place)**: Для микроконтроллеров класса ESP32-S3 область внешней SPI Flash проецируется в виртуальное адресное пространство процессора страницами аппаратного MMU размером 64 KB (65 536 байт).
2. **Заполнение Нулями (Zero Padding)**: Итоговый размер `flash_blob` принудительно выравнивается вверх до ближайшей кратной границы `effective_flash_page_size()` путем добавления нулей (`0x00`).
3. **Метаданные Размеров**: В манифесте отчуждения `EdgeManifest` логический размер данных (`logical_byte_len`) и выровненный физический размер (`padded_byte_len`) фиксируются в виде отдельных независимых полей. Данное выравнивание является аппаратным требованием контроллера флеш-памяти и не путается со страницами архивации `vfs` (4096 байт).

---

## §8. Формат Edge-Артефактов и Манифест (`EdgeManifest`)

При конвертации формируются следующие логические типы артефактов (`ArtifactKind`):
- `EdgeSramBlob`: бинарный образ оперативной памяти (`shard.sram`).
- `EdgeFlashBlob`: выровненный бинарный образ флеш-памяти (`shard.flash`).
- `EdgeManifest`: машиночитаемый манифест отчуждения (`edge_manifest.json`).
- `EdgeCHeader`: опциональный C-заголовок с константами смещений (`edge_model.h`).

Манифест является типизированной Rust-структурой `EdgeManifest`, содержащей таблицы секций памяти:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeManifest {
    pub abi_version: String,
    pub engine_spec_version: String,
    pub endianness: String, // Строго "little"
    pub target_profile: String,
    pub total_neurons: usize,
    pub target_dendrite_slots: usize,
    pub flash_page_size: usize,
    pub sram_byte_len: usize,
    pub flash_logical_byte_len: usize,
    pub flash_padded_byte_len: usize,
    pub sram_hash_sha256: String,
    pub flash_hash_sha256: String,
    pub sections: Vec<EdgeSectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSectionEntry {
    pub name: String,
    pub domain: EdgeMemoryDomain,
    pub offset: usize,
    pub logical_byte_len: usize,
    pub padded_byte_len: usize,
    pub alignment: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeMemoryDomain {
    Sram,
    Flash,
}
```

Проверочный C-заголовок (`edge_model.h`) генерируется напрямую из типизированной структуры `EdgeManifest` и параметров конвертации. Он не является единым источником истины.

---

## §9. Иерархия Ошибок Конвертора (`EdgeError`)

Публичный API библиотеки возвращает строго типизированную ошибку `EdgeError` на базе трейта `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum EdgeError {
    #[error("Missing required input artifact: {0}")]
    MissingArtifact(&'static str),

    #[error("Invalid state file header: {0}")]
    InvalidStateHeader(&'static str),

    #[error("Invalid axons file header: {0}")]
    InvalidAxonsHeader(&'static str),

    #[error("Invalid variant table content: {0}")]
    InvalidVariantTable(&'static str),

    #[error("State blob size mismatch: expected {expected}, got {actual}")]
    StateSizeMismatch { expected: usize, actual: usize },

    #[error("Axons blob size mismatch: expected {expected}, got {actual}")]
    AxonsSizeMismatch { expected: usize, actual: usize },

    #[error("Invalid target dendrite slots: {slots} (must be 1..={max})")]
    InvalidTargetDendriteSlots { slots: usize, max: usize },

    #[error("Invalid Flash page size: {0}")]
    InvalidFlashPageSize(usize),

    #[error("SRAM memory budget exceeded: allocated {allocated} > budget {budget}")]
    SramBudgetExceeded { allocated: usize, budget: usize },

    #[error("Numeric overflow during byte calculations")]
    IntegerOverflow,

    #[error("VFS execution error: {0}")]
    Vfs(String),

    #[error("FileSystem I/O error: {0}")]
    Io(String),

    #[error("Manifest serialization failure: {0}")]
    ManifestSerialization(String),
}
```

---

## §10. Требуемые Инварианты

- **INV-EDGE-001**: `edge-model` ни при каких условиях не модифицирует исходный `.axic` архив или входные байтовые слайсы.
- **INV-EDGE-002**: При совпадении входных байтов и параметров `EdgeConversionOptions` результат конвертации побитово детерминирован.
- **INV-EDGE-003**: Все смещения и размеры SoA-плоскостей десктопной модели запрашиваются строго через формулы крейта `layout`.
- **INV-EDGE-004**: Все пустые слоты внутри рядов дендритов после WTA-прореживания заполняются строго маркером `EMPTY_PIXEL`.
- **INV-EDGE-005**: Размер `flash_blob` выравнивается нулями ровно до ближайшей кратной границы `effective_flash_page_size()`.
- **INV-EDGE-006**: Эффективное число target-слотов $K$ ограничено условием `1 <= K <= layout::MAX_DENDRITES`. Выходные массивы имеют стрид $K$ (`padded_n * K`).
- **INV-EDGE-007**: Прореживание синапсов сохраняет биологический знак веса в точности без изменений (Закон Дейла).
- **INV-EDGE-008**: Проверка превышения бюджета SRAM выводится в виде явной ошибки `SramBudgetExceeded`, блокирующей генерацию бинарников.

---

## §11. Golden Tests / Обязательная Матрица Тестирования

Крейт `edge-model` обязан быть покрыт набором юнитов и интеграционных тестов:

1. **Архитектурная Изоляция (`test_edge_model_does_not_depend_on_compute_ipc_wire_net_baker`)**: Проверка отсутствия внешних тяжелых зависимостей в графе компиляции.
2. **Браковка Отсутствующих Файлов (`test_edge_rejects_missing_state_or_axons`)**: Возврат `MissingArtifact` при отсутствии ключевых блобов.
3. **Валидация Размеров по Layout (`test_edge_validates_state_size_from_layout`)**: Проверка валидации размера `.state` файла по формулам `layout`.
4. **Контроль Диапазона K Слотов (`test_edge_rejects_k_zero_and_k_gt_max`)**: Отбраковка значений $K = 0$ и $K > 128$.
5. **Сохранение Знака Весов при WTA (`test_wta_keeps_top_k_by_abs_weight_and_preserves_sign`)**: Проверка отбора по модулю с удержанием исходного знака.
6. **Детерминизм Разрешения Ничьих (`test_wta_tie_breaker_is_stable`)**: Проверка работы tie-breaker алгоритма при равных весах.
7. **Принятие Нулей и EMPTY_PIXEL на Входе (`test_wta_accepts_zero_and_empty_pixel_as_empty_input`)**: Верификация распознавания различных маркеров на входе.
8. **Сжатие Выходного Стрида до K Слотов (`test_wta_compaction_compresses_stride_to_k`)**: Верификация размерности выходных массивов `padded_n * K` и заполнения неиспользованных слотов рядов маркером `EMPTY_PIXEL`.
9. **Синхронизация Тройки Колонок (`test_compaction_preserves_target_weight_timer_alignment`)**: Проверка сохранения выравнивания целей, весов и таймеров.
10. **MMU Выравнивание 64 KB для ESP32 (`test_flash_blob_padded_to_64kb_for_esp32`)**: Верификация кратности физического размера флеш-блоба 65 536 байтам.
11. **Фиксация Таблицы Секций в Манифесте (`test_manifest_records_logical_and_padded_sizes`)**: Верификация секций `EdgeSectionEntry`, `endianness` и независимых полей в `EdgeManifest`.
12. **Побитовая Воспроизводимость Блобов (`test_output_is_byte_reproducible`)**: Проверка побитового совпадения результатов при повторных запусках.
13. **Неизменяемость Исходного Архива (`test_source_archive_not_mutated`)**: Проверка неизменяемости входных байтов.
14. **Контроль Бюджета SRAM (`test_sram_budget_exceeded_is_error`)**: Проверка немедленного возврата `SramBudgetExceeded` при превышении лимита памяти.
15. **Соответствие Генерации C-Заголовка Из Манифеста (`test_generated_c_header_matches_manifest_offsets`)**: Верификация совпадения констант в `edge_model.h` с данными типизированной структуры `EdgeManifest`.

---

## §12. Open Questions / Review Debt (Открытые Вопросы и Противоречия)

1. **Размещение Конфигурации Edge-Профилей**:
   - *Контекст*: Параметры конвертации передаются через `EdgeConversionOptions`.
   - *Вопрос*: Должны ли профили edge-конвертации в будущем перенесены в TOML-конфигурации крейта `config`?

2. **Обязательный Перечень Файлов `.axic` для Edge-Конвертации**:
   - *Контекст*: Для инференса требуются `.state`, `.axons` и таблица вариантов.
   - *Вопрос*: Требуется ли обязательное наличие файла `.paths` для генерации edge-модели или он опционален?

3. **Общий Крейт C-ABI Заголовков Встраиваемых Устройств (Firmware-Facing Edge ABI)**:
   - *Контекст*: `layout` владеет десктопными `.state/.axons/.paths`, а `edge-model` владеет производными `shard.sram` и `shard.flash`.
   - *Вопрос*: Потребуется ли отдельный легкий крейт геометрических контрактов и C-структур для прошивок микроконтроллеров в будущем?

4. **Выделение Edge-Конвертора в Отдельный CLI-Исполняемый Файл**:
   - *Контекст*: Подкоманда `edge` активируется через Cargo feature в `baker-cli`.
   - *Вопрос*: Целесообразно ли выделение утилиты в отдельный бинарник `edge-cli`?

5. **Политика Таймеров Дендритов (`dendrite_timers`) при Запуске на Устройстве**:
   - *Контекст*: В режиме только инференса таймеры рефрактерности могут быть не нужны.
   - *Вопрос*: Следует ли обнулять или копировать исходные `dendrite_timers` при генерации образа SRAM?

6. **Перспективы Поддержки Пластичности на Устройстве (On-Device Plasticity)**:
   - *Контекст*: Текущая версия проектируется строго под Pure Inference.
   - *Вопрос*: Каковы архитектурные границы при будущей поддержке ночной фазы на микроконтроллерах?

7. **Квантование Синаптических Весов (Weight Quantization)**:
   - *Контекст*: Веса хранятся в Mass Domain (`i32`).
   - *Вопрос*: Требуется ли поддержка квантования весов до типов `i16`, `i8` или `i4` для сверхкомпактных устройств?

8. **Поддержка Внешней Памяти PSRAM и Нескольких Бюджетов SRAM**:
   - *Контекст*: Некоторые чипы (ESP32-WROVER) обладают внешней оперативной памятью PSRAM.
   - *Вопрос*: Должен ли `edge-model` поддерживать трехуровневое разделение (SRAM / PSRAM / Flash)?

9. **Разбиение Огромных Моделей на Каскад Микроконтроллеров**:
   - *Контекст*: Крупный шардированный граф может не влезать в один чип.
   - *Вопрос*: Каким образом будет осуществляться нарезка одного шарда на несколько MCU?

10. **Владелец Спецификации Bare-Metal Runtime (`axicor-lite`)**:
    - *Контекст*: `edge-model` является оффлайн-конвертором.
    - *Вопрос*: В каком документе и крейте будет описан рантайм-цикл исполнения устройств (device loop) и прошивки?
