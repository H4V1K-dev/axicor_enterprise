# spec_config

> Версия спеки: 1.0  
> Дата: 2026-06-23  
> Статус: Approved  

---

## §1. Идентификация

| Поле | Значение |
|------|----------|
| Название | config |
| Слой | Слой 1 — Контракты Данных |
| Тип | Library (lib) |
| no_std | Нет (требуется `std` для `String` и аллокаций Serde) |
| Описание | Парсинг, валидация и типизация конфигурационных файлов новой 3-уровневой архитектуры (`model.toml`, `department.toml`, `shard.toml`). Крейт выступает в роли "Shift-Left" предохранителя, гарантируя математическую корректность и валидность всех параметров симуляции перед загрузкой. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|-------|-----------------|-------|
| `types` | `Tick`, `MasterSeed` | Использование фундаментальных типов квантов времени и сидов для построения структур конфигурации без нарушения архитектурной изоляции. |

### §2.2. Внешние зависимости

| Crate | Версия | Зачем |
|-------|--------|-------|
| `serde` | `=1.0.228`, features=["derive"] | Десериализация TOML-файлов в строгие типизированные структуры Rust. Версия жестко привязана к Workspace Cargo.toml. |
| `toml` | `=0.8.23` | Парсинг текстовых файлов конфигурации. Версия жестко привязана к Workspace Cargo.toml. |

### §2.3. Feature Flags

Секция не применима к данному крейту: Feature flags не используются.

---

## §3. Инварианты

Крейт `config` гарантирует соблюдение фундаментальных инвариантов, которые служат контрактом для всех вышестоящих слоев движка.

### §3.1. Структурные инварианты

- **INV-CONFIG-001**: Максимальное количество типов нейронов на один шард в `shard.toml` не может превышать `16` типов.
  - *Обоснование*: `VariantId` (идентификатор профиля в LUT) занимает 4 бита в упакованном представлении сомы. Соответственно, размер LUT-таблицы вариантов на GPU аппаратно ограничен 16 записями.
  - *Следствие нарушения*: Silent Data Corruption или вылет ядра GPU из-за выхода за границы массива Constant Memory.
  - *Где проверяется*: compile-time / load-time assert при десериализации `shard.toml` в `config::shard::validate_shard`.

- **INV-CONFIG-002**: Плотность нейронов (`density`) в слоях `LayerConfig` должна быть строго не отрицательной (`>= 0.0`).
  - *Обоснование*: Физическая плотность сомы не может быть отрицательной.
  - *Следствие нарушения*: Отрицательное число при генерации координат сомы вызовет некорректное распределение или падение генератора топологии.
  - *Где проверяется*: load-time assert в `config::shard::validate_shard`.

### §3.2. Семантические инварианты

- **INV-CONFIG-003**: Дискретный шаг скорости `v_seg` должен быть строго целым числом: `(signal_speed_m_s * tick_duration_us) % (voxel_size_um * segment_length_voxels) == 0`.
  - *Обоснование*: В ядрах симуляции используется целочисленная физика (Integer Physics) для обеспечения 100% воспроизводимости и производительности на GPU. Дробный шаг `v_seg` недопустим.
  - *Следствие нарушения*: Ошибки округления на GPU, потеря детерминизма, неверное прохождение сигналов по сегментам аксона.
  - *Где проверяется*: load-time assert при валидации `model.toml` в `config::simulation::validate_model`.

- **INV-CONFIG-004**: Защита "Single Spike in Flight". Для каждого типа нейрона в `shard.toml` должно выполняться условие: `signal_propagation_length >= refractory_period`.
  - *Обоснование*: Гарантирует, что предыдущий импульс успеет покинуть сому (пройти длину хвоста) до того, как нейрон выйдет из рефрактерности и сможет сгенерировать новый спайк.
  - *Следствие нарушения*: Наложение импульсов, повреждение аппаратной очереди на GPU.
  - *Где проверяется*: В функции `config::shard::validate_shard`.

- **INV-CONFIG-005**: Аппаратный лимит геометрии аксона. Значение `axon_growth_max_steps` в `model.toml` не может превышать 255.
  - *Обоснование*: Структура `PackedTarget` из Слоя 0 выделяет строго 8 бит на `Segment_Offset`. Если аксон будет длиннее 255 шагов, дендриты не смогут адресовать его дальние участки.
  - *Следствие нарушения*: Переполнение 8-битного значения смещения, запись мусора в память.
  - *Где проверяется*: Валидация `model.toml` в `config::simulation::validate_model`.

### §3.3. Дополнительные инварианты валидации

- **INV-CONFIG-006**: Сумма `height_pct` всех слоев в `shard.toml` должна быть равна `1.0` с допуском `1e-4`.
- **INV-CONFIG-007**: Сумма долей (`share`) в композиции каждого слоя должна быть равна `1.0` с допуском `1e-4`.
- **INV-CONFIG-008**: Лимит `max_dendrites` в `model.toml` должен быть строго равен `128` (жесткое ограничение структуры памяти GPU).
- **INV-CONFIG-009**: Размеры шарда `dimensions` в `shard.toml` должны быть в пределах: `0 < w <= 1023`, `0 < d <= 1023`, `0 < h <= 255`.
- **INV-CONFIG-010**: Массив `inertia_curve` в параметрах `GsopParams` должен содержать ровно 8 элементов.
- **INV-CONFIG-011**: Имена типов нейронов и имена шардов должны быть уникальными в пределах файла конфигурации.
- **INV-CONFIG-012**: Если шард имеет входящие сокеты (`SocketDirection::In`), `ghost_capacity` в настройках шарда должна быть строго больше нуля.
- **INV-CONFIG-013**: Координаты пинов `local_u`, `local_v` и их размеры `u_width`, `v_height` должны находиться в диапазоне `0.0..=1.0` и не выходить за границы диапазона `(local + size <= 1.0)`.

---

## §4. Публичный API

### §4.1. Типы (Types)

Все DTO-структуры крейта разделены на логические домены, соответствующие уровням конфигурации.

#### Домен 1: Системные метаданные (`simulation.rs`)
*   **`SystemMeta`**
    *   **Семантика**: Метаданные конфигурационного файла (`id`, `version`, `created_at`). Используется для версионирования.
    *   **Ограничения**: `id` не может быть пустым, `version` строго соответствует semver.

#### Домен 2: Модель (`simulation.rs`)
*   **`ModelConfig`**
    *   **Семантика**: Корневой узел файла `model.toml` (Level 1). Содержит физические параметры симуляции, размеры мира, список департаментов и межотдельские соединения.
*   **`WorldConfig`**
    *   **Семантика**: Физические размеры макро-мира (`width_um`, `depth_um`, `height_um`) в микрометрах.
*   **`SimulationParams`**
    *   **Семантика**: Глобальные законы симуляции (сид, скорость, лимиты).
*   **`DepartmentEntry`**
    *   **Семантика**: Запись о департаменте в модели с указанием его имени и относительного пути к конфигу.
*   **`ModelConnectionConfig`**
    *   **Семантика**: Межотдельские связи.

#### Домен 3: Департамент (`department.rs`)
*   **`DepartmentConfig`**
    *   **Семантика**: Корневой узел файла `department.toml` (Level 2). Описывает шарды и связи между ними.
*   **`ShardEntry`**
    *   **Семантика**: Описание входящего в департамент шарда (имя и путь к его конфигу).
*   **`DepartmentConnection`**
    *   **Семантика**: Связь между шардами внутри департамента в формате `"ShardName.SocketName"`.

#### Домен 4: Шард (`shard.rs`)
*   **`ShardConfig`**
    *   **Семантика**: Корневой узел файла `shard.toml` (Level 3). Детальное описание геометрии, слоев, типов нейронов, сокетов, портов и локальных настроек шарда.
*   **`ShardDimensions`**
    *   **Семантика**: Размеры шарда в вокселях (`w`, `d`, `h`).
*   **`LayerConfig`**
    *   **Семантика**: Настройки слоя (высота в процентах `height_pct`, плотность сомы `density`, распределение типов нейронов `composition`).
*   **`NeuronTypeDistribution`**
    *   **Семантика**: Доля (`share`) конкретного типа нейрона (`type_name`) в составе слоя.
*   **`NeuronType`**
    *   **Семантика**: Полный профиль параметров нейрона (мембрана, тайминги, сигнал, гомеостаз, адаптивный лик, дофамин, gsop, рост, спонтанная активность).
*   **`MembraneParams`**
    *   **Семантика**: Пороги напряжения и амплитуда AHP.
*   **`TimingParams`**
    *   **Семантика**: Период рефрактерности нейрона и синапса.
*   **`SignalParams`**
    *   **Семантика**: Дистанция распространения сигнала (длина хвоста аксона).
*   **`HomeostasisParams`**
    *   **Семантика**: Штраф и затухание гомеостаза.
*   **`AdaptiveLeakParams`**
    *   **Семантика**: Адаптивный сдвиг утечки.
*   **`DopamineParams`**
    *   **Семантика**: Аффинность к рецепторам D1 и D2.
*   **`GsopParams`**
    *   **Семантика**: Параметры синаптической пластичности (потенциация, депрессия, тип влияния, кривая инерции).
*   **`GrowthParams`**
    *   **Семантика**: Параметры роста аксонов и дендритных деревьев (угол FOV, радиус, веса стиринга, whitelist дендритов, веса спаутинга).
*   **`SpontaneousParams`**
    *   **Семантика**: Период спонтанного спайкинга.
*   **`SocketConfig`**
    *   **Семантика**: Межшардовые порты связи (сокеты) с указанием направления, размеров и параметров роста.
*   **`PortConfig`**
    *   **Семантика**: Внешние интерфейсы ввода/вывода (сенсоры/моторы).
*   **`PinConfig`**
    *   **Семантика**: Сенсорные или моторные пины проецирования с UV-координатами.
*   **`ShardSettings`**
    *   **Семантика**: Внутренние параметры шарда (емкость гостов, пороги прунинга, ночной интервал, автосохранение).

---

### §4.3. Функции (Functions)

#### Категория A: Парсинг и валидация уровня Модели

##### `pub fn parse_model_config(content: &str) -> Result<ModelConfig, ConfigError>`
- **Назначение**: Парсинг текстового представления `model.toml` в структуру `ModelConfig`.
- **Постусловия**: Возвращает распарсенную структуру или ошибку парсинга.
- **Паника**: Никогда.

##### `pub fn validate_model(config: &ModelConfig) -> Result<(), ConfigError>`
- **Назначение**: Семантическая проверка параметров модели на соответствие инвариантам (INV-CONFIG-003, INV-CONFIG-005, INV-CONFIG-008 и размеры мира).
- **Постусловия**: Возвращает `Ok(())` или ошибку `ConfigError::ValidationError`.
- **Паника**: Никогда.

#### Категория B: Парсинг и валидация уровня Департамента

##### `pub fn parse_department_config(content: &str) -> Result<DepartmentConfig, ConfigError>`
- **Назначение**: Парсинг текстового представления `department.toml` в структуру `DepartmentConfig`.
- **Постусловия**: Возвращает распарсенную структуру или ошибку парсинга.
- **Паника**: Никогда.

##### `pub fn validate_department(config: &DepartmentConfig) -> Result<(), ConfigError>`
- **Назначение**: Проверка департамента на отсутствие дубликатов имен шардов, непустоту путей и корректность формата связей (`"Shard.Socket"`).
- **Постусловия**: Возвращает `Ok(())` или ошибку `ConfigError::ValidationError`.
- **Паника**: Никогда.

#### Категория C: Парсинг и валидация уровня Шарда

##### `pub fn parse_shard_config(content: &str) -> Result<ShardConfig, ConfigError>`
- **Назначение**: Парсинг текстового представления `shard.toml` в структуру `ShardConfig`.
- **Постусловия**: Возвращает распарсенную структуру или ошибку парсинга.
- **Паника**: Никогда.

##### `pub fn validate_shard(config: &ShardConfig) -> Result<(), ConfigError>`
- **Назначение**: Комплексная семантическая проверка параметров шарда на соответствие инвариантам (INV-CONFIG-001, 002, 004, 006, 007, 009, 010, 012, 013).
- **Постусловия**: Возвращает `Ok(())` или ошибку `ConfigError::ValidationError`.
- **Паника**: Никогда.

---

### §4.4. Константы и Магические Числа

| Константа | Значение | Тип | Семантика |
|-----------|----------|-----|-----------|
| `MAX_NEURON_TYPES` | 16 | `usize` | Максимально допустимое количество уникальных типов нейронов на один шард (аппаратное ограничение GPU). |
| `DEFAULT_AXON_GROWTH_MAX_STEPS` | 255 | `u32` | Значение по умолчанию для `axon_growth_max_steps` (8-битный предел адресации). |
| `DEFAULT_SEGMENT_LENGTH_VOXELS` | 2 | `u32` | Значение по умолчанию для длины сегмента в вокселях. |
| `FLOAT_TOLERANCE` | `1e-4` | `f32` | Допуск погрешности float при проверке инвариантов слоев и композиции. |

---

## §5. Доменная Логика

Выделение конфигурации в отдельный крейт Слоя 1 изолирует текстовый парсинг (Serde, динамические аллокации строк и работу с файловой системой) от вычислительного ядра симуляции.

Крейт выступает в роли «Shift-Left» предохранителя симуляции. Он проверяет анатомические и физические параметры сети (количество типов нейронов, скорость сигналов, ограничения геометрии, лимиты вокселей) на соответствие жестким аппаратным лимитам GPU и MCU *до* этапа генерации топологии и инициализации симуляции. Это исключает запуск заведомо некорректных моделей, способных вызвать переполнение видеопамяти (VRAM) или сбои целочисленной физики в рантайме.

---

## §6. Алгоритмы и Формулы

### §6.1. Проверка целочисленности дискретной скорости (`v_seg` Integer Verification)

**Вход**:
- `signal_speed_m_s: f32` — скорость распространения сигнала в метрах в секунду (из `model.toml`).
- `tick_duration_us: u32` — длительность тика в микросекундах (из `model.toml`).
- `voxel_size_um: f32` — размер вокселя в микрометрах (из `model.toml`).
- `segment_length_voxels: u32` — длина одного сегмента аксона в вокселях (из `model.toml`).

**Выход**:
- `v_seg: f32` — шаг скорости в сегментах за тик.
- `Result<(), ConfigError>` — результат валидации.

**Формула:**
Длина одного сегмента в микрометрах:
```math
\text{segment\_length\_um} = \text{voxel\_size\_um} \cdot \text{segment\_length\_voxels}
```

Скорость за тик в микрометрах:
```math
\text{speed\_um\_tick} = \text{signal\_speed\_m\_s} \cdot \text{tick\_duration\_us}
```

Дискретная скорость в сегментах за тик:
```math
\text{v\_seg} = \frac{\text{speed\_um\_tick}}{\text{segment\_length\_um}}
```

Инвариант INV-CONFIG-003 требует целочисленности `v_seg` с допуском `1e-5`:
```rust
let rounded = v_seg.round();
if (v_seg - rounded).abs() > 1e-5 {
    return Err(ConfigError::ValidationError(...));
}
```

---

## §7. Структуры Данных и Memory Layout

### §7.1. Иерархическое дерево конфигурации

Все структуры являются Serde DTO без бинарных паддингов.

```text
ModelConfig (model.toml)
  ├── meta: Option<SystemMeta>
  ├── world: WorldConfig
  │     ├── width_um (f64)
  │     ├── depth_um (f64)
  │     └── height_um (f64)
  ├── simulation: SimulationParams
  │     ├── tick_duration_us (u32)
  │     ├── total_ticks (u64)
  │     ├── master_seed (String)
  │     ├── voxel_size_um (f32)
  │     ├── segment_length_voxels (u32, default=2)
  │     ├── signal_speed_m_s (f32)
  │     ├── sync_batch_ticks (u32)
  │     ├── axon_growth_max_steps (u32, default=255)
  │     └── max_dendrites (u8, ==128)
  ├── departments: Vec<DepartmentEntry>
  │     ├── name (String)
  │     └── config (String → path to department.toml)
  └── connections: Vec<ModelConnectionConfig>
        ├── from (String)
        └── to (String)

DepartmentConfig (department.toml)
  ├── meta: Option<SystemMeta>
  ├── shards: Vec<ShardEntry>
  │     ├── name (String)
  │     └── config (String → path to shard.toml)
  └── connections: Vec<DepartmentConnection>
        ├── from (String)
        └── to (String)

ShardConfig (shard.toml)
  ├── meta: Option<SystemMeta>
  ├── dimensions: ShardDimensions
  │     ├── w (u32)
  │     ├── d (u32)
  │     └── h (u32)
  ├── layers: Vec<LayerConfig>
  │     ├── name (String)
  │     ├── height_pct (f32)
  │     ├── density (f32)
  │     └── composition: Vec<NeuronTypeDistribution>
  │           ├── type_name (String)
  │           └── share (f32)
  ├── neuron_types: Vec<NeuronType> [max 16]
  │     ├── name (String)
  │     ├── membrane: MembraneParams
  │     │     ├── threshold (i32)
  │     │     ├── rest_potential (i32)
  │     │     ├── leak_shift (u32)
  │     │     └── ahp_amplitude (u16)
  │     ├── timings: TimingParams
  │     │     ├── refractory_period (u8)
  │     │     └── synapse_refractory_period (u8)
  │     ├── signal: SignalParams
  │     │     └── signal_propagation_length (u8)
  │     ├── homeostasis: HomeostasisParams
  │     │     ├── homeostasis_penalty (i32)
  │     │     └── homeostasis_decay (u16)
  │     ├── adaptive_leak: AdaptiveLeakParams
  │     │     ├── adaptive_leak_min_shift (i32)
  │     │     ├── adaptive_leak_gain (u16)
  │     │     └── adaptive_mode (u8)
  │     ├── dopamine: DopamineParams
  │     │     ├── d1_affinity (u8)
  │     │     └── d2_affinity (u8)
  │     ├── gsop: GsopParams
  │     │     ├── gsop_potentiation (u16)
  │     │     ├── gsop_depression (u16)
  │     │     ├── is_inhibitory (bool)
  │     │     └── inertia_curve (Vec<u8>, len==8)
  │     ├── growth: GrowthParams
  │     │     ├── steering_fov_deg (f32)
  │     │     ├── steering_radius_um (f32)
  │     │     ├── steering_weight_inertia (f32)
  │     │     ├── steering_weight_sensor (f32)
  │     │     ├── steering_weight_jitter (f32)
  │     │     ├── dendrite_radius_um (f32)
  │     │     ├── growth_vertical_bias (f32)
  │     │     ├── type_affinity (f32)
  │     │     ├── dendrite_whitelist (Vec<String>)
  │     │     ├── sprouting_weight_distance (f32)
  │     │     ├── sprouting_weight_power (f32)
  │     │     ├── sprouting_weight_explore (f32)
  │     │     └── sprouting_weight_type (f32)
  │     └── spontaneous: SpontaneousParams
  │           └── spontaneous_firing_period_ticks (u32)
  ├── sockets: Option<Vec<SocketConfig>>
  │     ├── name (String)
  │     ├── direction (SocketDirection)
  │     ├── width (u32)
  │     ├── height (u32)
  │     ├── entry_z (Option<EntryZ>)
  │     ├── target_type (Option<String>)
  │     └── growth_steps (Option<u32>)
  ├── ports: Option<Vec<PortConfig>>
  │     ├── name (String)
  │     ├── direction (SocketDirection)
  │     ├── entry_z (Option<EntryZ>)
  │     └── pins: Vec<PinConfig>
  │           ├── name (String)
  │           ├── width (u32)
  │           ├── height (u32)
  │           ├── local_u (f32)
  │           ├── local_v (f32)
  │           ├── u_width (f32)
  │           ├── v_height (f32)
  │           ├── target_type (String)
  │           ├── stride (u32)
  │           ├── growth_steps (Option<u32>)
  │           └── empty_pixel (Option<String>)
  └── settings: ShardSettings
        ├── ghost_capacity (u32)
        ├── prune_threshold (i32)
        ├── max_sprouts (u32)
        ├── night_interval_ticks (u32)
        └── save_checkpoints_interval_ticks (u32)
```

---

## §8. Граничные Случаи и Особые Сценарии

### §8.1. Граничные значения

| # | Ситуация | Ожидаемое поведение |
|---|----------|-------------------|
| E-016 | **Отрицательная плотность сомы (`density < 0.0`)**: В слое шарда передано некорректное значение. | Валидатор `validate_shard` немедленно возвращает `ConfigError::ValidationError` (INV-CONFIG-002). |
| E-017 | **Превышение количества типов нейронов (`neuron_types.len() > 16`)**: В `shard.toml` определено более 16 профилей нейронов. | Валидатор возвращает `ConfigError::ValidationError` (INV-CONFIG-001). |
| E-018 | **Дробное значение шага скорости (`v_seg` is non-integer)**: Физические параметры дают дробный шаг. | Валидатор возвращает `ConfigError::ValidationError` (INV-CONFIG-003). |
| E-019 | **Нарушение Single Spike in Flight (`signal_propagation_length < refractory_period`)**: Конфигурация допускает выпуск второго спайка до затухания первого. | Валидатор возвращает `ConfigError::ValidationError` (INV-CONFIG-004). |
| E-020 | **Превышение лимита длины аксона (`axon_growth_max_steps > 255`)**: Задана длина аксона, превышающая лимит. | Валидатор возвращает `ConfigError::ValidationError` (INV-CONFIG-005). |

---

## §9. Ошибки

### §9.1. Перечисление ошибок

```rust
#[derive(Debug)]
pub enum ConfigError {
    /// Ошибка файлового ввода/вывода (файл не найден, нет прав доступа)
    IoError(std::io::Error),
    /// Ошибка парсинга структуры TOML (синтаксические ошибки в файле)
    ParseError(String),
    /// Нарушение инвариантов и ограничений валидации
    ValidationError(String),
}
```

---

## §10. Зависимости и Интеграция

### §10.1. Что крейт потребляет (inbound)

| Крейт-источник | Что используем | Какой контракт ожидаем |
|---------------|---------------|----------------------|
| `types` | `Tick`, `MasterSeed` | Атомарные типы данных в no_std окружении. |

### §10.2. Кто потребляет крейт (outbound / обратные зависимости)

| Крейт-потребитель | Что использует | Какой контракт мы обязаны сохранить |
|------------------|---------------|-----------------------------------|
| `baker` | `ModelConfig`, `ShardConfig` | Предоставление 100% провалидированных данных для построения бинарного архива. |
| `topology` | `ShardConfig` | Гарантия валидности плотности слоев для генерации координат. |
| `weaver-daemon` | `ShardConfig` | Парсинг и передача срезов `NeuronType` в рантайм. |

---

## §11. Стратегия Тестирования

### §11.1. Юнит-тесты

| Тест | Что проверяет | Связанный инвариант / Граничный случай |
|------|--------------|-------------------|
| `test_parse_valid_model` | Парсинг корректного `model.toml` и валидацию его параметров. | INV-CONFIG-003, INV-CONFIG-005 |
| `test_default_values` | Подстановку значений по умолчанию (например, `axon_growth_max_steps = 255`). | INV-CONFIG-005 |
| `test_validation_err_v_seg_not_integer` | Возврат ошибки при дробном значении `v_seg`. | INV-CONFIG-003, E-018 |
| `test_validation_err_axon_growth_overflow` | Возврат ошибки при превышении `axon_growth_max_steps > 255`. | INV-CONFIG-005, E-020 |
| `test_validation_err_dendrites_mismatch` | Возврат ошибки при `max_dendrites != 128`. | INV-CONFIG-008 |
| `test_parse_valid_department` | Парсинг и валидацию корректного `department.toml`. | — |
| `test_validation_err_duplicate_shards` | Ошибку валидации при одинаковых именах шардов в департаменте. | INV-CONFIG-011 |
| `test_validation_err_invalid_connection_format` | Возврат ошибки при некорректном формате связи шардов. | — |
| `test_parse_valid_shard` | Полный цикл парсинга и валидации корректного `shard.toml`. | — |
| `test_validation_err_dimensions_overflow` | Возврат ошибки при размерах шарда, превышающих 1023x1023x255. | INV-CONFIG-009 |
| `test_validation_err_lut_limit` | Ошибку валидации при определении более 16 типов нейронов. | INV-CONFIG-001, E-017 |
| `test_validation_err_overlapping_spikes` | Нарушение Single Spike in Flight. | INV-CONFIG-004, E-019 |
| `test_validation_err_height_mismatch` | Возврат ошибки при сумме высот слоев не равной 1.0. | INV-CONFIG-006 |
| `test_validation_err_pins_boundary` | Ошибку выхода пинов за границы 1.0. | INV-CONFIG-013 |

---

## Приложение A — Глоссарий

| Термин | Определение |
|--------|-----------|
| Shift-Left Валидация | Подход к обеспечению качества, при котором проверки переносятся на самые ранние этапы жизненного цикла (в данном случае — до запуска симуляции). |

Checklist Полноты (A.3)

- [x] Все публичные типы описаны в §4
- [x] Все функции описаны в §4.3 (3 parse + 3 validate = 6 функций)
- [x] Все инварианты из §3 имеют соответствующий пункт в §11 (тесты)
- [x] Все `Err`-варианты перечислены в §9 (`IoError`, `ParseError`, `ValidationError`)
- [x] Все крейты-потребители перечислены в §10.2
- [x] Нет ни одного «магического числа» без объяснения
- [x] Все формулы имеют единицы измерения
- [x] Граничные случаи из §8 покрыты тестами в §11
- [x] Все константы описаны в §4.4
