# spec_config

> Версия спеки: 2.1  
> Дата: 2026-07-01  
> Статус: Approved v2.1 / Ready for Implementation

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| Название | `config` |
| Слой | Слой 1 — Контракты Данных и Десериализация (Data Contracts & Configuration) |
| Тип | Library (`lib`) |
| no_std | config является std-only крейтом; использует String, Vec, HashMap, файловый ввод-вывод и Serde. |
| Описание | Единый источник истины для парсинга, десериализации (Serde/TOML) и валидации декларативного биологического DSL движка `AxiEngine` и редактора AxiCAD (`model.toml`, `department.toml`, `shard.toml`). Крейт выступает в роли "Shift-Left" предохранителя, обеспечивая математическую и топологическую корректность всех параметров симуляции до начала стадии компиляции (`baker`) и аллокации VRAM (`compute`). |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` | `Tick`, `MasterSeed`, `Microns`, `Fraction` | Использование фундаментальных типов и лимитов для построения структур конфигурации без нарушения архитектурной изоляции. |
| `physics` | `compute_v_seg` | Вызов центральной математической функции деривации дискретного шага сигнала для проверок валидации без дублирования формул. |

### §2.2. Зависимые компоненты (outbound consumers)

Компилятор топологии (`baker`), рантайм-оркестратор и редактор AxiCAD обязаны десериализовать и валидировать конфигурационные TOML-файлы исключительно через крейт `config`. Самостоятельный парсинг TOML-строк в других компонентах **строжайше запрещен**.

### §2.3. Внешние зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `serde` | `=1.0.228`, features=`["derive"]` | Декларативная десериализация TOML-файлов в типизированные Rust DTO структуры с зафиксированными правилами наименования полей. |
| `toml` | `=0.8.23` | Парсинг текстовых документов конфигурации в промежуточные Serde-деревья. |
| `thiserror` | `=1.0.69` | Иерархическое строгое представление ошибок парсинга и валидации. |

### §2.4. Feature Flags

Крейт собирается как стандартная `std`-библиотека. Отдельные `no_std`-сборки отсутствуют, так как крейт `config` используется исключительно на этапе компиляции Ahead-of-Time (AOT) и оффлайн-инструментария (`baker`, `baker-cli`, редактор AxiCAD) и не задействуется во время выполнения горячего цикла симуляции на встраиваемых устройствах.

### §2.5. Запрещенные операции и зависимости

В крейте `config` физически и архитектурно запрещены:
- Аллокация VRAM/RAM для симуляции, FFI-вызовы CUDA/HIP и управление жизненным циклом буферов (принадлежат `compute`).
- Генерация 3D-топологии нейронов, рост аксонов, трассировка путей и бинарный bake дампов (принадлежат `baker`).
- Операции с C-ABI структурами `#[repr(C, align)]` и расчет SoA-смещений в бинарных файлах (принадлежат `layout`).
- Самостоятельная реализация математических формул деривации физики (делегируется в `physics`).
- Зависимость от `bytemuck` (десериализация выполняется исключительно через `serde`/`toml`).

---

## §3. Ownership Boundaries (Границы Владения)

| Крейт / Модуль | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`config`** (Слой 1) | **Serde/TOML DTO и валидация DSL**: Парсинг `model.toml`, `department.toml`, `shard.toml`, проверка синтаксиса, диапазонов полей, уникальности имен внутри документов, проверка целочисленности `v_seg` (через вызов `physics`) и локальная валидация связей. | Запрещен генератор топологии, сборка бинарников `.state`, mmap, GPU upload, FFI и дублирование C-ABI макетов `layout`. Крейт не занимается файловым резолвингом относительных путей или проверкой существования файлов на диске. |
| **`types`** (Слой 0) | Базовые типы данных (`Tick`, `MasterSeed`, `Microns`) и фундаментальные доменные лимиты. | Запрещен парсинг TOML-строк и Serde-атрибуты. |
| **`layout`** (Слой 1) | C-ABI макеты физической памяти (`VariantParameters`), выравнивание плоскостей SoA и заголовки файлов. | Запрещены Serde-структуры и парсинг текстовых конфигураций. |
| **`physics`** (Слой 0) | Математические формулы GLIF, GSOP, Active Tail, DDS и функцию деривации `compute_v_seg`. | Запрещена деривация параметров из TOML. |
| **`vfs`** (Слой 2) | Формат архива `.axic`, таблицы оглавления TOC и низкоуровневая mmap упаковка/распаковка. | Запрещен семантический разбор TOML-схем. |
| **`baker`** (Слой 4) | Компиляция 3D-пространства, генерация воксельных координат, процедурный рост аксонов, междокументный/межшардовый граф-резолвинг (включая поиск файлов на диске и связывание Shard/Department ссылок), а также сборка `.state` и вызов `vfs::pack_directory`. | Запрещен самостоятельный парсинг TOML без вызова `config`. |
| **`topology`** (Слой 4) | Алгоритмы и структуры данных 3D размещения нейронов, геометрического роста аксонов, расчета длин путей и начальной синаптической связности. | Запрещен парсинг TOML-схем. |

---

## §4. Целевая Иерархия TOML-Файлов

Проект `AxiEngine` описывается строгой 3-уровневой иерархией конфигурационных файлов.

```text
model.toml                                 # Глобальный мир и симуляция
├── DepartmentName/
│   ├── DepartmentName.toml                # Группа шардов и внутренние связи
│   ├── ShardNameA/
│   │   └── ShardNameA.toml                # Геометрия, слои и типы нейронов
│   └── ShardNameB/
│       └── ShardNameB.toml
```

---

## §5. Публичные Rust DTO и Свойства Сериализации

### §5.1. Правила Serde и Строгая Защита
Все DTO-структуры в крейте `config` обязаны использовать атрибут `#[serde(deny_unknown_fields)]`. Наличие любого незадокументированного или постороннего поля в TOML-файле вызывает немедленную ошибку десериализации.

### §5.2. Публичные Свойства и Перечисления (Enums)

#### Direction (Направление подключения)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    In,  // "in"
    Out, // "out"
}
```

#### EmptyPixelMode (Поведение при нулевом сигнале)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmptyPixelMode {
    Skip, // "skip"
    Zero, // "zero"
}
```

#### EntryZ (Высотная привязка аксонов)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EntryZ {
    Top,    // "Top"
    Mid,    // "Mid"
    Bottom, // "Bottom"
}
```

---

### §5.3. Полный Реестр DTO Структур

#### 1. Метаданные и Модель (`model.toml`)
- **`SystemMeta`**: `id: String`, `version: String`, `created_at: String`.
- **`ModelConfig`**: `meta: Option<SystemMeta>`, `world: WorldConfig`, `simulation: SimulationParams`, `departments: Vec<DepartmentEntry>`, `connections: Vec<ModelConnectionConfig>`.
- **`WorldConfig`**: `width_um: f64`, `depth_um: f64`, `height_um: f64`.
- **`SimulationParams`**: `tick_duration_us: u32`, `total_ticks: u64`, `master_seed: String`, `voxel_size_um: f32`, `segment_length_voxels: u32`, `signal_speed_m_s: f32`, `sync_batch_ticks: u32`, `axon_growth_max_steps: u32`.
- **`DepartmentEntry`**: `name: String`, `config: String`, `meta: Option<SystemMeta>`.
- **`ModelConnectionConfig`**: `id: String`, `from: String`, `to: String`.

#### 2. Департамент (`department.toml`)
- **`DepartmentConfig`**: `meta: Option<SystemMeta>`, `shards: Vec<ShardEntry>`, `connections: Vec<DepartmentConnection>`.
- **`ShardEntry`**: `name: String`, `config: String`.
- **`DepartmentConnection`**: `id: String`, `from: String`, `to: String`.

#### 3. Шард и Анатомия (`shard.toml`)
- **`ShardConfig`**: `meta: Option<SystemMeta>`, `dimensions: ShardDimensions`, `settings: ShardSettings`, `layers: Vec<LayerConfig>`, `neuron_types: Vec<NeuronType>`, `sockets: Option<Vec<SocketConfig>>`, `ports: Option<Vec<PortConfig>>`.
- **`ShardDimensions`**: `w: u32`, `d: u32`, `h: u32`.
- **`ShardSettings`**: `ghost_capacity: u32`, `prune_threshold: i32`, `max_sprouts: u32`, `night_interval_ticks: u32`, `save_checkpoints_interval_ticks: u32`.
- **`LayerConfig`**: `name: String`, `height_pct: f32`, `density: f32`, `composition: Vec<NeuronTypeDistribution>`.
- **`NeuronTypeDistribution`**: `type_name: String`, `share: f32`.

#### 4. Профиль Нейрона (`NeuronType` и подструктуры)
`NeuronType` состоит из следующих вложенных секций:
- **`MembraneParams`**: `threshold: i32`, `rest_potential: i32`, `leak_shift: u32`, `ahp_amplitude: u16`.
- **`TimingParams`**: `refractory_period: u8`, `synapse_refractory_period: u8`.
- **`SignalParams`**: `signal_propagation_length: u8`.
- **`HomeostasisParams`**: `homeostasis_penalty: i32`, `homeostasis_decay: u16`.
- **`AdaptiveLeakParams`**: `adaptive_leak_min_shift: i32`, `adaptive_leak_gain: u16`, `adaptive_mode: u8`.
- **`DopamineParams`**: `d1_affinity: u8`, `d2_affinity: u8`.
- **`GsopParams`**: `gsop_potentiation: u16`, `gsop_depression: u16`, `initial_synapse_weight: u16`, `is_inhibitory: bool`, `inertia_curve: Vec<u8>`.
- **`GrowthParams`**: `steering_fov_deg: f32`, `steering_radius_um: f32`, `steering_weight_inertia: f32`, `steering_weight_sensor: f32`, `steering_weight_jitter: f32`, `dendrite_radius_um: f32`, `growth_vertical_bias: f32`, `type_affinity: f32`, `dendrite_whitelist: Vec<String>`, `sprouting_weight_distance: f32`, `sprouting_weight_power: f32`, `sprouting_weight_explore: f32`, `sprouting_weight_type: f32`.
- **`SpontaneousParams`**: `spontaneous_firing_period_ticks: u32`.

#### 5. Сокеты, Порты и Пины
- **`SocketConfig`**: `name: String`, `direction: Direction`, `width: u32`, `height: u32`, `entry_z: Option<EntryZ>`, `target_type: Option<String>`, `growth_steps: Option<u32>`.
- **`PortConfig`**: `name: String`, `direction: Direction`, `entry_z: Option<EntryZ>`, `pins: Vec<PinConfig>`.
- **`PinConfig`**: `name: String`, `width: u32`, `height: u32`, `local_u: f32`, `local_v: f32`, `u_width: f32`, `v_height: f32`, `target_type: String`, `stride: u32`, `growth_steps: Option<u32>`, `empty_pixel: Option<EmptyPixelMode>`.

---

## §6. Правила Идентичности, Имен и Адресации

1. **Доменное Имя (`name`) и Регулярное Выражение**:
   - Поле `name` является человекочитаемым доменным идентификатором сущности.
   - Имена должны быть уникальными внутри своего списка (`departments`, `shards`, `layers`, `neuron_types`, `sockets`, `ports`, `pins`).
   - Во избежание поломки грамматики путей эндпоинтов (где точка `.` служит разделителем), имя должно соответствовать регулярному выражению `^[a-zA-Z0-9_-]+$`. Точки и пустые строки в именах **запрещены**.
2. **Идентификация Связей (`id`)**:
   - Массивы `[[connections]]` содержат обязательное целевое поле `id` (стабильный идентификатор связи), к которому привязывается геометрия тракта.
3. **Грамматика Путей Связей (`from` / `to`)**:
   - В `department.toml`: строго формат `ShardName.SocketName` (2 компонента).
   - В `model.toml`: строго формат `DepartmentName.ShardName.SocketName` (3 компонента).
4. **Отсутствие `typeId` в `NeuronType`**:
   - Явное поле `typeId` в TOML не требуется. Бинарный `VariantId` (0..15) рассчитывается строго по индексу элемента в массиве `[[neuron_types]]`.

---

## §7. Валидация `model.toml`

При вызове `validate_model()` выполняются следующие обязательные проверки:
1. **Размеры Мира**: `world.width_um > 0.0`, `world.depth_um > 0.0`, `world.height_um > 0.0`.
2. **Параметры Симуляции**:
   - `tick_duration_us > 0`.
   - `total_ticks == 0` трактуется как бесконечная симуляция.
   - `master_seed` не является пустой строкой.
   - `voxel_size_um > 0.0`.
   - `segment_length_voxels > 0`.
   - `signal_speed_m_s > 0.0`.
   - `sync_batch_ticks > 0`.
   - `axon_growth_max_steps <= 255`.
3. **Департаменты**: Имена `departments[i].name` уникальны и соответствуют регулярному выражению `^[a-zA-Z0-9_-]+$`. Пути `config` не пусты.
4. **Связи**: Идентификаторы `connections[i].id` уникальны. Строки `from` и `to` успешно парсятся грамматикой (3 компонента, разделенных точкой).

---

## §8. Формула Валидации Дискретного Шага `v_seg`

Десериализатор/валидатор `config` **не переопределяет математику физики у себя**, а вызывает функцию `physics::compute_v_seg(...)` из крейта `physics`.

### Вызов функции из `physics`
Валидатор `config` передает входные физические параметры в модуль `physics`:
```rust
let v_seg = physics::compute_v_seg(
    sim.signal_speed_m_s,
    sim.tick_duration_us,
    sim.voxel_size_um,
    sim.segment_length_voxels,
)?;
```
Если `compute_v_seg` возвращает ошибку (например, значение $v_{\text{seg}}$ не является точным целым числом или выходит за диапазон $1 \dots 255$), вызов `validate_model()` завершается с сообщением об ошибке валидации дискретного шага.

---

## §9. Валидация `department.toml`

При вызове `validate_department()` выполняются локальные проверки одного файла:
1. **Шарды**: Имена `shards[i].name` уникальны в департаменте и валидны по regex. Пути `config` не пусты.
2. **Связи**: Идентификаторы `connections[i].id` уникальны.
3. **Локальная Грамматика Эндпоинтов**: Строки `from` и `to` проверяются исключительно на соответствие текстовому формату `ShardName.SocketName` (2 компонента). Проверка существования целевых шардов и сокетов в графе не входит в компетенцию `validate_department()` и выполняется на этапе межшардового резолвинга в компиляторе `baker`.

---

## §10. Валидация `shard.toml`

При вызове `validate_shard()` выполняются проверки:
1. **Воксельные Размеры (`dimensions`)**:
   - `w`: в диапазоне $1 \dots 1023$.
   - `d`: в диапазоне $1 \dots 1023$.
   - `h`: в диапазоне $1 \dots 255$.
2. **Анатомические Слои (`layers`)**:
   - Массив `layers` не пуст. Имена `layers[i].name` уникальны и соответствуют regex.
   - Каждый `height_pct > 0.0`. Сумма всех `height_pct` равна `1.0` ($\pm 1e-4$).
   - Плотность `density >= 0.0` и `density <= 1.0` (верхняя граница набивки вокселей нейронами).
   - Массив `composition` не пуст. Каждый `share >= 0.0`. Сумма всех `share` в слое равна `1.0` ($\pm 1e-4$).
   - Каждый `composition.type_name` ссылается на существующее имя из `neuron_types`.

---

## §11. Валидация Профилей Нейронов (`neuron_types`)

- **Лимит Типов**: Длина массива `neuron_types` в диапазоне $1 \dots 16$ (`INV-CONFIG-001`).
- **Уникальность**: Имена `neuron_types[i].name` уникальны и соответствуют regex.
- **Тайминги**: `refractory_period > 0`, `signal_propagation_length > 0`.
- **Инвариант Длины Хвоста (`INV-CONFIG-004`)**: `signal_propagation_length >= refractory_period` и `signal_propagation_length <= 255`.
- **Разграничение Serde и Validator проверок**: Поля с типами `u8` (`synapse_refractory_period`, `d1_affinity`, `d2_affinity`) аппаратно ограничены типом. Если в TOML передать значение $> 255$ (например, `256`), это вызывает ошибку десериализации на уровне Serde (`Serde Range Rejection`), а не рантайм-ошибку `ValidationError`.
- **Кривая Инерции**: Массив `inertia_curve` содержит ровно **8 элементов**.
- **Адаптивная Утечка**: `adaptive_mode` принимает значения `0`, `1` или `2`. Значение `adaptive_leak_min_shift` может быть отрицательным (рантайм физика выполняет безопасный clamp).
- **Валидация Вайтлистов (`dendrite_whitelist`)**: Если в `growth.dendrite_whitelist` указаны имена типов нейронов, каждый элемент списка обязан ссылаться на объявленное имя из `neuron_types`.
- **Спонтанный Спайкинг**: `spontaneous_firing_period_ticks == 0` означает выключено. Значение `1` запрещено на уровне TOML DSL (вызывает ошибку валидации), минимально разрешенный период — `2` тика. Хотя низкоуровневые физические примитивы `physics` аппаратно поддерживают период 1 для гибкости расчетов, пользовательский DSL config намеренно вводит это ограничение для предотвращения истощения синапсов. Это не является архитектурным конфликтом: физика шире, config строже.
- **Веса Синапсов**: Начальный вес синапсов `initial_synapse_weight` в `GsopParams` должен быть в диапазоне `0..=32767`.

---

## §12. Валидация Сокетов, Портов и Пинов

1. **Сокеты (`sockets`)**:
   - Имена сокетов уникальны в шарде и соответствуют regex. `direction` принимает значение `Direction::In` или `Direction::Out`.
   - `width > 0`, `height > 0`. `growth_steps <= 255` (если задан).
   - **Проверка Ссылок Сокета**: Если у сокета задано поле `target_type`, оно обязано ссылаться на объявленное имя из `neuron_types`.
   - **Инвариант VRAM**: Если у шарда есть входящий сокет (`direction == Direction::In`), поле `settings.ghost_capacity` должно быть $> 0$.
2. **Порты и Пины (`ports`, `pins`)**:
   - Имена портов и пинов уникальны и соответствуют regex.
   - Разрешение пина: `width > 0`, `height > 0`, `stride > 0`.
   - Координаты проекции: `local_u` и `local_v` в диапазоне `0.0 ..= 1.0`.
   - Размеры проекции: `u_width > 0.0`, `v_height > 0.0`.
   - **Инвариант Границ Проекции**: `local_u + u_width <= 1.0` и `local_v + v_height <= 1.0`.
   - `target_type` пина обязан ссылаться на объявленный тип нейрона из `neuron_types`.

---

## §13. API Поверхность Крейта

Крейт `config` предоставляет строго типизированный функциональный API:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("I/O error: {0}")]
    IoError(String),
}

pub fn parse_model_str(toml_content: &str) -> Result<ModelConfig, ConfigError>;
pub fn parse_department_str(toml_content: &str) -> Result<DepartmentConfig, ConfigError>;
pub fn parse_shard_str(toml_content: &str) -> Result<ShardConfig, ConfigError>;

pub fn validate_model(config: &ModelConfig) -> Result<(), ConfigError>;
pub fn validate_department(config: &DepartmentConfig) -> Result<(), ConfigError>;
pub fn validate_shard(config: &ShardConfig) -> Result<(), ConfigError>;
```

В режиме `std` крейт предоставляет утилитарные методы загрузки файлов с диска:
```rust
pub fn load_model_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<ModelConfig, ConfigError>;
pub fn load_department_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<DepartmentConfig, ConfigError>;
pub fn load_shard_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<ShardConfig, ConfigError>;
```
Файловые хелперы выполняют исключительно чтение и парсинг по указанному пути. Они не занимаются разрешением относительных путей соседних файлов, path containment, include-графом или валидацией целостности проекта. Вся оркестрация сборки и проверка структуры проекта на диске является монопольной зоной ответственности `baker`.

---

## §14. Golden Tests / Обязательная Матрица Тестирования

Крейт `config` должен содержать unit-тесты для верификации правил валидации:

1. **Минимальный Валидный Парсинг (`test_valid_minimal_configs_parse`)**: Проверка успешного парсинга эталонных файлов `model.toml`, `department.toml`, `shard.toml`.
2. **Запрет Неизвестных Полей (`test_unknown_field_rejected`)**: Попытка десериализации TOML с посторонним полем вызывает ошибку Serde.
3. **Уникальность Имен (`test_duplicate_names_rejected`)**: Проверка дубликатов имен в `departments`, `shards`, `layers`, `neuron_types`, `sockets`, `ports`, `pins`.
4. **Валидация Regex Имен (`test_entity_name_regex_validation`)**: Отклонение имен с точками или специальными символами (`"Retina.1"` -> ошибка).
5. **Грамматика Путей (`test_bad_endpoint_path_rejected`)**: Отклонение некорректных строк `from`/`to`.
6. **Превышение Лимита Типов (`test_too_many_neuron_types_rejected`)**: Массив `neuron_types` из 17 элементов вызывает ошибку (`INV-CONFIG-001`).
7. **Отсутствующий Тип Нейрона (`test_missing_referenced_type_rejected`)**: `composition.type_name`, `socket.target_type` или `dendrite_whitelist` ссылаются на несуществующий тип.
8. **Выход за Границы Вокселей (`test_bad_dimensions_rejected`)**: Размеры `w=1024` или `h=256` вызывают ошибку.
9. **Сумма Высот Слоев (`test_bad_layer_height_sum_rejected`)**: Сумма `height_pct != 1.0` вызывает ошибку.
10. **Сумма Долей Слоя (`test_bad_composition_sum_rejected`)**: Сумма `composition.share != 1.0` вызывает ошибку.
11. **Дробный `v_seg` (`test_fractional_v_seg_rejected`)**: Физические параметры, при которых вызов `physics::compute_v_seg` возвращает ошибку, отклоняются.
12. **Валидация Периода Спонтанного Спайкинга (`test_spontaneous_firing_period_validation`)**: Значение `spontaneous_firing_period_ticks == 1` отклоняется при валидации; `0` или `>= 2` разрешены.
13. **Лимит Шагов Роста (`test_axon_growth_max_steps_limit`)**: Значение `axon_growth_max_steps > 255` отклоняется.
14. **Переполнение UV Проекции Пина (`test_pin_uv_overflow_rejected`)**: Условие `local_u + u_width > 1.0` вызывает ошибку.
15. **Входящий Сокет при Ghost Capacity 0 (`test_input_socket_zero_ghost_capacity_rejected`)**: Отклонение входящего сокета при `ghost_capacity == 0`.
16. **Serde Range Rejection (`test_serde_u8_range_rejection`)**: Проверка, что значения $> 255$ для полей `synapse_refractory_period`, `d1_affinity`, `d2_affinity` отклоняются на этапе Serde десериализации.
17. **Валидация Веса Синапсов (`test_initial_synapse_weight_validation`)**: Значение `initial_synapse_weight > 32767` в `GsopParams` отклоняется при валидации.
18. **Предельная Плотность Слая (`test_density_out_of_bounds`)**: Значение `density > 1.0` в `LayerConfig` отклоняется при валидации.

---

## §15. Open Questions / Review Debt (Открытые Вопросы и Противоречия)

Все архитектурные и структурные вопросы по крейту `config` были полностью разрешены в рамках прохода Pass 2.1. Новые блокирующие вопросы отсутствуют.

---

## §16. Resolved Architectural Decisions (Принятые Решения Pass 2.1)

1. **[RESOLVED] Тип Физических Размеров `WorldConfig` (REV-CFG-001)**:
   - *Решение*: Фиксируется `f64` как единый целевой тип в TOML DTO для размеров мира (`width_um`, `depth_um`, `height_um`).

2. **[RESOLVED] Регистр Символов в `EntryZ` (REV-CFG-002)**:
   - *Решение*: Оставляем PascalCase написание (`"Top"`, `"Mid"`, `"Bottom"`). Настраиваем Serde через `#[serde(rename_all = "PascalCase")]` для `EntryZ` для совместимости со схемами AxiCAD.

3. **[RESOLVED] Верхняя Граница Плотности `density` (REV-CFG-003)**:
   - *Решение*: Плотность `density` в `LayerConfig` строго ограничена диапазоном `0.0..=1.0`.

4. **[RESOLVED] Формат Стабильного Идентификатора Связи `connections.id` (REV-CFG-004)**:
   - *Решение*: Разрешается любой текстовый слаг по regex `^[a-zA-Z0-9_-]+$`. UUID v4 не требуется на уровне TOML.

5. **[RESOLVED] Размещение и Тестирование `initial_synapse_weight` в TOML-схеме (REV-CFG-005)**:
   - *Решение*: Поле `initial_synapse_weight` добавлено в секцию `GsopParams` (`initial_synapse_weight: u16`) в `NeuronType`. Валидация контролирует значение по порогу `<= 32767`.

6. **[RESOLVED] Крайний Случай DDS Heartbeat (REV-CFG-006)**:
   - *Решение*: Спонтанный период `spontaneous_firing_period_ticks == 1` запрещен, так как спайкинг на каждом тике ломает синаптический рефрактерный период. Минимально допустимый период — `2` тика. Хотя низкоуровневые физические примитивы `physics` аппаратно поддерживают период 1 для гибкости расчетов, пользовательский DSL config намеренно вводит это ограничение для предотвращения истощения синапсов. Это не является архитектурным конфликтом: физика шире, config строже.

7. **[RESOLVED] Политика Точности Валидации `v_seg` (REV-CFG-007)**:
   - *Решение*: Валидация выполняется путем вызова функции `physics::compute_v_seg(...)` из крейта `physics`. Для параметров в `model.toml` проверяется, что они строго положительные конечные `f32`/`f64` числа.

8. **[RESOLVED] Будущее Поля `max_dendrites` в TOML (REV-CFG-008)**:
   - *Решение*: Поле `max_dendrites` полностью удалено из TOML-схем. Движок неявно оперирует константой `MAX_DENDRITES = 128` из `layout`.
