# Спецификация доменной модели AxiCAD (Domain Model Spec)

> Этот документ определяет фундаментальную доменную модель AxiCAD — структуру данных, иерархию владения, правила адресации (имен), внешние ссылки и инварианты для ключевых биологических и структурных сущностей сети Axicor, синхронизируемые с TOML/Rust контрактами.

## Status: Draft

---

## 1. Введение и общие концепции

Доменная модель AxiCAD служит единым словарем (Ubiquitous Language) для 3D-редактора, парсеров/сериализаторов TOML и валидатора, обеспечивая бинарную совместимость со спецификациями компилятора сети (Baker) и вычислительного ядра на Rust (`dev-rust-api`).

### 1.1 Координатная система и сетка (Grid)
- **Воксельная сетка (Voxel Grid)**: Все пространственные расчеты производятся в дискретной целочисленной 3D-сетке. Шаг сетки равен 1 вокселю.
- **Оси координат**:
  - **X** — Ширина (Width, `w`). Диапазон значений: `0..1023` (10 бит). Направление: слева направо.
  - **Y** — Глубина (Depth, `d`). Диапазон значений: `0..1023` (10 бит). Направление: от наблюдателя вглубь.
  - **Z** — Высота (Height, `h`). Диапазон значений: `0..255` (8 бит). Направление: снизу вверх. Это **вертикальная ось** для анатомических слоев.
- **Упакованное представление**: Координаты нейрона упаковываются в один 32-битный регистр `PackedPosition` со следующей структурой:
  `[Type_ID(4b) | Z(8b) | Y(10b) | X(10b)]`

### 1.2 Концепция адресации (Имена)
- **Имя (Name)**: Каждая сущность в TOML-файлах идентифицируется строковым именем (ASCII slug). Имя должно быть уникальным в пределах непосредственного родителя.
  - Формат имени должен удовлетворять регулярному выражению `^[a-zA-Z0-9_-]+$`. `[Tier: AxiCAD editor validator]`

### 1.3 Path grammar (Грамматика путей)
Для описания топологии сети и внутренних адресов редактора используются три различные категории путей:

1. **TOML endpoint**: Адресация соединений в конфигурационных файлах TOML.
   - Внутри `department.toml`: `<ShardName>.<SocketName>` (например, `Retina.cross_modal`).
   - Внутри `model.toml`: `<DepartmentName>.<ShardName>.<SocketName>` (например, `SensoryCortex.Retina.motor_commands`).
2. **Store key**: Уникальный строковый ключ записи во внутреннем реактивном Store AxiCAD, использующий сгенерированный UUID для однозначной идентификации объекта в памяти:
   - Формат: `entities.<UUID>` (например, `entities.e5c2b3d4-f6a8-4c9e-9d8f-7b6a5d4c3b2a`).
3. **DomainRecord.path**: Динамически вычисляемый (derived/recomputed) путь в дереве владения домена, отражающий текущее иерархическое положение сущности (например, `SensoryCortex.Retina`). Не является первичным идентификатором (primary identity) объекта и автоматически пересчитывается при перемещении сущности по дереву владения.

### 1.4 Первичность стабильных идентификаторов (Persistent Stable IDs)

> [!IMPORTANT]
> **Принцип первичного связывания (Primary Binding vs Human-Readable Labels)**:
> - **Connection stable id/ref**: Любое логическое соединение оперирует персистентным стабильным идентификатором (`connection id/ref`).
> - **Tract geometry binding**: Внешняя геометрия тракта (описываемая как сплайновый тоннель роста с контрольными фреймами `control frames`) привязывается строго к `connection id/ref`, сохраняя целостность при любых переименованиях шардов и департаментов.
> - **Endpoint stable id/ref**: Междепартаментные эндпоинты имеют стабильные идентификаторы для надёжной моделируемой привязки графа.
> - **Names / Typed paths**: Строковые имена и пути являются исключительно отображаемыми метками (UI display labels) и вторичным диагностическим слоем (`human-readable diagnostic paths`), но не первичным механизмом связывания сущностей.

---

## 2. Иерархия владения и ссылок (Domain Hierarchy)

Архитектурная структура доменной модели делится на два перекрывающихся слоя:

1. **Модель владения (Ownership Model)**: Строгое дерево (**strict tree**), где каждая дочерняя сущность принадлежит ровно одному контейнеру (без разделяемого владения). Уничтожение родителя каскадно уничтожает потомков.
2. **Модель связей (Connections)**: Ссылочный граф (**reference graph**), наложенный поверх дерева владения с помощью относительных или абсолютных путей (Path grammar).

```
[Strict Tree of Ownership]                 [Reference Graph]
ModelConfig (Root)
 ├── ModelConnectionConfig ───────────────────────► Ссылается на Department.Shard.Socket
 └── DepartmentConfig
      ├── DepartmentConnection ───────────────────► Ссылается на Shard.Socket
      └── ShardConfig
           ├── LayerConfig
           ├── NeuronType
           ├── SocketConfig
           └── PortConfig
                └── PinConfig
```

---

## 3. Уровни валидации (Validation Tiers)

Все инварианты доменной модели разделены по 4 уровням валидации в зависимости от этапа жизненного цикла модели и проверяющего компонента:

1. **Serde/Rust contract (TOML Parse)**:
   - Проверяется автоматически при разборе файлов конфигурации.
   - Ограничения: обязательность полей, строгое соответствие типов, запрет неизвестных полей (`deny_unknown_fields`).
2. **Current Rust validator**:
   - Проверяется методами `validate_model`, `validate_department`, `validate_shard` в `dev-rust-api`.
   - Ограничения: семантическая непротиворечивость биологических и математических параметров в рамках отдельного файла конфигурации.
3. **AxiCAD editor validator**:
   - Интерактивная валидация в UI редактора во время проектирования до экспорта в TOML.
   - Ограничения: редакторские UX-политики, визуальные предупреждения (например, непересечение физических AABB-границ шардов на 3D-сцене, несохраненный dirty-стейт, проверка формата имен).
4. **Baker resolver / ABI checks**:
   - Статические проверки компилятора (`baker-cli`) на этапе линковки и генерации бинарных прошивок.
   - Ограничения: физические лимиты оборудования, глобальная связность сокетов, генерация UDP-портов и хэшей пинов, проверка лимитов памяти.

---

## 4. Спецификация сущностей (Rust-контракты)

### 4.1 ModelConfig (Модель)

Модель является корневым контейнером для всего проекта нейросети Axicor (`model.toml`).

- **Зона ответственности**:
  - Агрегирует все департаменты сети.
  - Владеет глобальными физическими законами пространства и симуляции.
  - Владеет междепартаментными соединениями (`ModelConnectionConfig`).
- **Владение**: Корневой объект дерева владения.
- **Инварианты**:
  - В системе может существовать только одна активная `ModelConfig`. `[Tier: AxiCAD editor validator]`
  - В списке `departments` имена департаментов уникальны. `[Tier: AxiCAD editor validator]`
  - Физический шаг сигнала `segment_length_voxels` должен быть строго целым положительным числом. `[Tier: Rust Validator]`
  - Длительность одного тика `tick_duration_us` должна быть строго больше нуля. `[Tier: Rust Validator]`
  - Размер вокселя `voxel_size_um` и скорость сигнала `signal_speed_m_s` должны быть строго больше нуля. `[Tier: Rust Validator]`
  - Физические размеры мира `width_um`, `depth_um`, `height_um` должны быть строго больше нуля. `[Tier: Rust Validator]`
  - Коэффициент дискретной скорости распространения сигнала ($v_{seg} = \frac{speed\_um\_tick}{segment\_length\_um}$) должен точно оцениваться как целое число. `[Tier: Rust Validator]`
  - Лимит максимального количества дендритов `max_dendrites` должен быть равен ровно 128. `[Tier: Rust Validator]`
  - Глобальное ограничение шагов роста аксона `axon_growth_max_steps` должно быть строго $\le 255$. `[Tier: Rust Validator]`
- **Связанные структуры Rust**:
  - `ModelConfig` содержит `WorldConfig`, `SimulationParams`, `Vec<DepartmentEntry>` и `Vec<ModelConnectionConfig>`.
- **Пример TOML (`model.toml`)**:
  ```toml
  [world]
  width_um  = 25000.0
  depth_um  = 25000.0
  height_um = 6375.0

  [simulation]
  tick_duration_us       = 100
  total_ticks            = 0
  master_seed            = "AXICOR"
  voxel_size_um          = 25.0
  signal_speed_m_s       = 0.5
  sync_batch_ticks       = 100
  segment_length_voxels  = 2
  axon_growth_max_steps  = 250
  max_dendrites          = 128

  [[departments]]
  name   = "SensoryCortex"
  config = "SensoryCortex/SensoryCortex.toml"

  [[departments]]
  name   = "MotorGanglion"
  config = "MotorGanglion/MotorGanglion.toml"

  [[connections]]
  from = "SensoryCortex.Retina.motor_commands"
  to   = "MotorGanglion.SpinalRelay.motor_feed"
  ```

---

### 4.2 DepartmentConfig (Департамент)

Департамент — это функционально-структурная группа нейросети (`department.toml`).

- **Зона ответственности**:
  - Группирует шарды по логическому признаку.
  - Владеет внутридепартаментными соединениями (`DepartmentConnection`).
- **Владение**: Принадлежит `ModelConfig`.
- **Инварианты**:
  - В списке `shards` имена шардов строго уникальны. `[Tier: Rust Validator]`
  - Соединения `connections` ссылаются только на существующие шарды и сокеты внутри этого департамента. `[Tier: Baker resolver / ABI checks]`
  - Пути соединений `from` и `to` должны строго соответствовать формату `Shard.Socket` из 2 элементов. `[Tier: Rust Validator]`
- **Связанные структуры Rust**:
  - `DepartmentConfig` содержит `Option<SystemMeta>`, `Vec<ShardEntry>` и `Vec<DepartmentConnection>`.
- **Пример TOML (`SensoryCortex/SensoryCortex.toml`)**:
  ```toml
  [[shards]]
  name   = "Retina"
  config = "Retina/Retina.toml"

  [[shards]]
  name   = "Auditory"
  config = "Auditory/Auditory.toml"

  [[connections]]
  from = "Retina.cross_modal"
  to   = "Auditory.cross_feed"
  ```

---

### 4.3 ShardConfig (Шард)

Шард — это базовый автономный трехмерный вычислительный блок (GPU Unit), описываемый файлом `shard.toml`.

- **Зона ответственности**:
  - Определяет физические границы (объем в вокселях) для размещения нейронов.
  - Владеет анатомическими слоями (`LayerConfig`), типами нейронов (`NeuronType`), сокетами (`SocketConfig`), портами (`PortConfig`) и локальными вычислительными настройками.
- **Владение**: Принадлежит `DepartmentConfig`.
- **Инварианты**:
  - Размеры шарда `dimensions` должны строго укладываться в лимиты `PackedPosition`: `w > 0 && w <= 1023`, `d > 0 && d <= 1023`, `h > 0 && h <= 255`. `[Tier: Rust Validator]`
  - Количество уникальных `NeuronType` ограничено: **максимум 16**. `[Tier: Rust Validator]`
  - Если у шарда есть входящие сокеты (`direction = "in"`), то его `settings.ghost_capacity` должен быть строго больше нуля. `[Tier: Rust Validator]`
- **Связанные структуры Rust**:
  - `ShardConfig` содержит `Option<SystemMeta>`, `ShardDimensions`, `Vec<LayerConfig>`, `Vec<NeuronType>`, `Option<Vec<SocketConfig>>`, `Option<Vec<PortConfig>>` и `ShardSettings`.
- **Пример TOML (`SensoryCortex/Retina/Retina.toml`)**:
  ```toml
  [dimensions]
  w = 256
  d = 256
  h = 63

  # Дальнейшее содержимое shard.toml приведено в подразделах ниже
  ```

---

### 4.4 LayerConfig (Анатомический слой)

Анатомический слой делит внутренний объем шарда на подобъемы вдоль вертикальной оси **Z**.

- **Зона ответственности**:
  - Задает распределение типов нейронов на определенном уровне высоты шарда.
- **Владение**: Принадлежит `ShardConfig`.
- **Инварианты**:
  - Сумма относительных высот всех слоев `height_pct` должна быть строго равна `1.0` (±1e-4). `[Tier: Rust Validator]`
  - Сумма долей распределения нейронов `composition.share` внутри каждого слоя должна быть строго равна `1.0` (±1e-4). `[Tier: Rust Validator]`
  - Все типы нейронов в `composition.type_name` должны быть объявлены в `[[neuron_types]]` этого шарда. `[Tier: Rust Validator]`
  - Плотность `density` должна быть строго неотрицательным числом (`INV-CONFIG-002`). `[Tier: Rust Validator]`
- **Связанные структуры Rust**:
  - `LayerConfig` содержит `name` (String), `height_pct` (f32), `density` (f32), `composition` (Vec<NeuronTypeDistribution>).
- **Пример TOML (внутри `shard.toml`)**:
  ```toml
  [[layers]]
  name       = "L4_Sensory"
  height_pct = 0.6
  density    = 0.8
  composition = [
      { type_name = "Stellate_Exc",  share = 0.7 },
      { type_name = "Basket_Inh",    share = 0.3 },
  ]

  [[layers]]
  name       = "L5_Output"
  height_pct = 0.4
  density    = 0.5
  composition = [
      { type_name = "Pyramidal_Exc", share = 0.9 },
      { type_name = "Basket_Inh",    share = 0.1 },
  ]
  ```

---

### 4.5 NeuronType (Тип нейрона)

Параметры биологического профиля и модели поведения нейронов, а также правила роста дендритных и аксональных деревьев.

- **Зона ответственности**:
  - Описывает уравнения мембранного потенциала, механизмы гомеостаза, адаптивной утечки и чувствительности к дофамину.
  - Описывает параметры ветвления синапсов (GSOP) и параметры управления ростом аксонов (Growth).
- **Владение**: Принадлежит `ShardConfig`.
- **Инварианты**:
  - Локальный индекс типа нейрона (определяемый порядком в списке `[[neuron_types]]` от 0 до 15) записывается в 4 старших бита `PackedPosition`. `[Tier: Baker ABI]`
  - Длина хвоста распространения сигнала `signal_propagation_length` должна быть больше или равна абсолютному рефрактерному периоду `refractory_period` (`INV-CONFIG-004`). `[Tier: Rust Validator]`
  - Длина массива `inertia_curve` в секции `gsop` должна составлять ровно **8 элементов**. `[Tier: Rust Validator]`
  - Параметры `refractory_period` и `signal_propagation_length` должны быть строго больше нуля. `[Tier: Rust Validator]`
- **Связанные структуры Rust**:
  - `NeuronType` содержит `name` (String), `membrane` (MembraneParams), `timings` (TimingParams), `signal` (SignalParams), `homeostasis` (HomeostasisParams), `adaptive_leak` (AdaptiveLeakParams), `dopamine` (DopamineParams), `gsop` (GsopParams), `growth` (GrowthParams) и `spontaneous` (SpontaneousParams).
- **Пример TOML (внутри `shard.toml`)**:
  ```toml
  [[neuron_types]]
  name = "Stellate_Exc"

    [neuron_types.membrane]
    threshold      = 20000
    rest_potential  = -70000
    leak_shift      = 4
    ahp_amplitude   = 0

    [neuron_types.timings]
    refractory_period          = 5
    synapse_refractory_period  = 10

    [neuron_types.signal]
    signal_propagation_length  = 8

    [neuron_types.homeostasis]
    homeostasis_penalty  = 1500
    homeostasis_decay    = 990

    [neuron_types.adaptive_leak]
    adaptive_leak_min_shift = -5
    adaptive_leak_gain      = 2
    adaptive_mode           = 1

    [neuron_types.dopamine]
    d1_affinity = 80
    d2_affinity = 20

    [neuron_types.gsop]
    gsop_potentiation = 15
    gsop_depression   = 5
    is_inhibitory     = false
    inertia_curve     = [10, 20, 30, 40, 50, 60, 70, 80]

    [neuron_types.growth]
    steering_fov_deg          = 60.0
    steering_radius_um        = 100.0
    steering_weight_inertia   = 0.6
    steering_weight_sensor    = 0.3
    steering_weight_jitter    = 0.1
    dendrite_radius_um        = 150.0
    growth_vertical_bias      = 0.7
    type_affinity             = 0.5
    dendrite_whitelist        = []
    sprouting_weight_distance = 0.4
    sprouting_weight_power    = 0.4
    sprouting_weight_explore  = 0.1
    sprouting_weight_type     = 0.1

    [neuron_types.spontaneous]
    spontaneous_firing_period_ticks = 10000
  ```

---

### 4.6 SocketConfig (Сокет)

Интерфейсный порт на границе шарда для биологического подключения аксонов от других шардов.

- **Зона ответственности**:
  - Описывает геометрическую область ввода/вывода на грани шарда.
  - Инкапсулирует параметры шагов роста аксона при компиляции.
- **Владение**: Принадлежит `ShardConfig`.
- **Инварианты**:
  - Поле `direction` пишется в нижнем регистре: `"in"` (приемник) или `"out"` (передатчик). `[Tier: Serde/Rust]`
  - Входящие и исходящие сокеты, соединяемые через `Connections`, должны иметь совпадающие геометрические размеры: `from_socket.width == to_socket.width` и `from_socket.height == to_socket.height`. `[Tier: Baker resolver / ABI checks]`
  - Параметр `growth_steps` должен быть строго $\le 255$ из-за 8-битного аппаратного ограничения. `[Tier: Baker resolver / ABI checks]`
- **Связанные структуры Rust**:
  - `SocketConfig` содержит `name` (String), `direction` (SocketDirection), `width` (u32), `height` (u32), `entry_z` (Option<EntryZ>), `target_type` (Option<String>), `growth_steps` (Option<u32>).
  - `EntryZ` принимает значения: `"Top"`, `"Mid"`, `"Bottom"`.
- **Пример TOML (внутри `shard.toml`)**:
  ```toml
  [[sockets]]
  name      = "motor_commands"
  direction = "in"
  width     = 16
  height    = 16

  [[sockets]]
  name         = "cross_feed"
  direction    = "out"
  width        = 8
  height       = 8
  entry_z      = "Mid"
  target_type  = "Stellate_Exc"
  growth_steps = 250
  ```

---

### 4.7 PortConfig (Порт)

Интерфейс внешнего ввода-вывода (I/O) шарда для интеграции с внешним миром (UDP-сеть, сенсоры, симуляция).

- **Зона ответственности**:
  - Предоставляет абстракцию портов, мапируемых Baker-ом в бинарные UDP-потоки.
- **Владение**: Принадлежит `ShardConfig`.
- **Инварианты**:
  - Направление `direction` задается как `"in"` или `"out"`. `[Tier: Serde/Rust]`
- **Связанные структуры Rust**:
  - `PortConfig` содержит `name` (String), `direction` (SocketDirection), `entry_z` (Option<EntryZ>), `pins` (Vec<PinConfig>).
- **Пример TOML (внутри `shard.toml`)**:
  ```toml
  [[ports]]
  name      = "retina_feed"
  direction = "in"
  entry_z   = "Top"

    # Описание пинов приведено ниже
  ```

---

### 4.8 PinConfig (Пин)

Конкретная зона маппинга и UV-проекция внутри внешнего порта.

- **Зона ответственности**:
  - Привязывает каналы внешнего сигнала к внутренним вокселям или типам нейронов шарда.
- **Владение**: Принадлежит `PortConfig`.
- **Инварианты**:
  - Координаты нормализованной UV-проекции должны лежать строго в границах $0.0..1.0$: `local_u + u_width <= 1.0` и `local_v + v_height <= 1.0`. `[Tier: Rust Validator]`
  - Поле `empty_pixel` принимает значения `"skip"` или `"zero"`. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]`
  - Параметр `growth_steps` должен быть строго $\le 255$ из-за 8-битного аппаратного ограничения. `[Tier: Baker resolver / ABI checks]`
- **Связанные структуры Rust**:
  - `PinConfig` содержит `name` (String), `width` (u32), `height` (u32), `local_u` (f32), `local_v` (f32), `u_width` (f32), `v_height` (f32), `target_type` (String), `stride` (u32), `growth_steps` (Option<u32>), `empty_pixel` (Option<String>).
- **Пример TOML (внутри `[[ports]]` в `shard.toml`)**:
  ```toml
    [[ports.pins]]
    name         = "retina_left"
    width        = 28
    height       = 16
    local_u      = 0.0
    local_v      = 0.0
    u_width      = 0.5
    v_height     = 1.0
    target_type  = "Stellate_Exc"
    stride       = 1
    growth_steps = 255
    empty_pixel  = "skip"
  ```

---

### 4.9 ShardSettings (Настройки шарда)

Локальные конфигурационные лимиты симуляции шарда.

- **Связанные структуры Rust**:
  - `ShardSettings` содержит `ghost_capacity` (u32), `prune_threshold` (i32), `max_sprouts` (u32), `night_interval_ticks` (u32), `save_checkpoints_interval_ticks` (u32).
- **Пример TOML (в конце `shard.toml`)**:
  ```toml
  [settings]
  ghost_capacity                   = 1024
  prune_threshold                  = 15
  max_sprouts                      = 4
  night_interval_ticks             = 10000
  save_checkpoints_interval_ticks  = 100000
  ```

---

## 5. Контракты и аппаратные лимиты (Baker / ABI Limits)

Доменные спецификации транслируются компилятором Baker в высокопроизводительные бинарные структуры данных. Модель обязана соблюдать жесткие лимиты:

| Параметр | Лимит | Описание и обоснование (ABI / Hardware) | Уровень валидации |
|---|---|---|---|
| **Neuron Types per Shard** | Макс. 16 | 4-битная адресация типа нейрона в регистре `PackedPosition` | `[Tier: Baker ABI]` |
| **Max Dendrites** | ровно 128 | Константа `max_dendrites` в симуляторе для синапсов сомы | `[Tier: Rust Validator]` |
| **Global Axon Growth Steps** | Макс. 255 | Лимит `axon_growth_max_steps` в `SimulationParams` | `[Tier: Rust Validator]` |
| **Local Growth Steps** | Макс. 255 | Ограничение `growth_steps` для `SocketConfig` и `PinConfig` | `[Tier: Baker resolver / ABI checks]` / `[Tier: AxiCAD editor validator]` |
| **Shard Width (X)** | Макс. 1023 | 10-битный лимит координаты X в `PackedPosition` | `[Tier: Rust Validator]` |
| **Shard Depth (Y)** | Макс. 1023 | 10-битный лимит координаты Y в `PackedPosition` | `[Tier: Rust Validator]` |
| **Shard Height (Z)** | Макс. 255 | 8-битный лимит вертикальной координаты Z в `PackedPosition` | `[Tier: Rust Validator]` |
| **Inertia Curve length** | ровно 8 | Фиксированный размер массива в структуре `GsopParams` | `[Tier: Rust Validator]` |
| **Connection direction** | Strict `out` $\to$ `in` | Однонаправленная передача спайков в синапсах | `[Tier: Baker resolver / ABI checks]` / `[Tier: AxiCAD editor validator]` |

---

## 6. Разделение TOML-схемы и внутренней модели редактора (Editor Domain Model)

Спецификации файлов конфигурации Axicor описывают исключительно биологическую и вычислительную топологию сети. В них **запрещено** добавлять поля, предназначенные только для пользовательского интерфейса редактора AxiCAD (например, цвета, координаты окон, состояние выделения).

### 6.1 Каноническая TOML-схема (Axicor DSL)
- Очищена от UI-метаданных.
- Структуры данных полностью сериализуются/десериализуются в Rust через библиотеку `serde` в типы `ModelConfig`, `DepartmentConfig`, `ShardConfig`.
- Является единственным источником истины для симулятора и Baker.

### 6.2 Внутренняя доменная модель редактора AxiCAD (Store Domain Model)
- Внутри 3D-редактора AxiCAD канонические сущности оборачиваются в плоскую структуру `DomainRecord` реактивного хранилища `Store`:
  ```typescript
  interface DomainRecord<TData = unknown> {
    id: string;                    // Всегда стабильный сгенерированный UUID редактора (primary identity)
    type: DomainEntityType;        // 'model' | 'department' | 'shard' | 'layer' | ...
    path: string;                  // Вычисляемый/перевычисляемый путь на основе дерева владения (например, 'SensoryCortex.Retina')
    data: TData;                   // Чистые данные TOML-контракта (соответствующие Rust-схемам). TOML name хранится в data.name.
    meta?: Record<string, unknown>;// Метаданные редактора (UI, цвета, видимость)
  }
  ```
- **Размещение в пространстве сцены (`EditorPlacementData`)**: Положение шардов на 3D-сцене в viewport редактора хранится в поле `meta` в структуре `EditorPlacementData` или в отдельном файле проекта `axicad.project.json`, предотвращая засорение файлов конфигурации `shard.toml`.
  ```typescript
  interface EditorPlacementData {
    shardName: string;
    position: [number, number, number]; // Координаты в пространстве сцены редактора [X, Y, Z]
    rotation?: [number, number, number, number];
    visible?: boolean;
  }
  ```
- **Интерактивные свойства**: Применяются во внутреннем Store AxiCAD (такие как `selection` — список выбранных сущностей, `dirty` — флаг несохраненных изменений, стек команд `Undo/Redo`). Перед экспортом/сохранением данных в TOML-файлы редактор очищает все поля `meta` и сохраняет исключительно чистые ветки `data` в соответствии с Axicor DSL.

---

## Changelog

| Дата | Изменение |
|------|-----------|
| 2026-06-28 | Синхронизация доменной модели с зафиксированными архитектурными решениями Open Decisions #11-30: утверждены тракты как сплайновые тоннели с контрольными фреймами, внутренние сокеты в MVP, отмена обязательного typeId в домене, кэш позиций сом (`shard-soma-cache`) и глобальная библиотека пресетов. |
| 2026-06-28 | Синхронизация доменной модели с зафиксированными архитектурными решениями: утверждены стабильные UUID/id/ref как первичное средство связывания трактов/эндпоинтов, зафиксирован статус `white_matter` как канонического системного профиля нейронов для relay growth и отдельное хранение геометрии трактов. |
| 2026-06-27 | Создание спецификации доменной модели AxiCAD (Domain Model Spec), синхронизированной с TOML/Rust контрактами Axicor (определение осей X/Y/Z, структуры PackedPosition, сущностей, Path grammar, Validation Tiers и структуры DomainRecord). |
