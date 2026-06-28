# Спецификация TOML-схемы Axicor (TOML Schema Spec)

> Этот документ содержит формальное техническое описание структуры, типов данных, ограничений и канонического синтаксиса конфигурационных файлов Axicor/AxiCAD (`model.toml`, `department.toml`, `shard.toml`).

## Status: Draft

---

## 1. Назначение документа

Данная спецификация описывает исключительно **чистый декларативный биологический DSL Axicor**, используемый компилятором Baker и симулятором на Rust (`dev-rust-api`).

### Вне зоны ответственности (Non-goals)
- Визуальные метаданные 3D-лейаута редактора (цвета элементов, пространственные координаты окон, флаги видимости, состояния выделения).
- Реактивное состояние Store AxiCAD (`DomainRecord`, UUID, флаги `dirty`).
- Выходные бинарные артефакты компиляции Baker (файлы `manifest.toml`, `.axic`, `.state`, генерируемые порты и т.д.).

---

## 2. Рекомендуемая структура каталогов (Canonical Layout)

Биологический проект Axicor рекомендуется организовывать в виде следующей структуры каталогов с относительными путями:

```
model.toml                                 # Корневой конфигурационный файл модели
├── DepartmentName/
│   ├── DepartmentName.toml                # Конфигурация департамента
│   ├── ShardNameA/
│   │   └── ShardNameA.toml                # Конфигурация шарда А
│   └── ShardNameB/
│       └── ShardNameB.toml                # Конфигурация шарда Б
```

- Пути к конфигурациям департаментов в `model.toml` (`[[departments]]`) и шардов в `department.toml` (`[[shards]]`) задаются в поле `config` в виде относительных путей (например, `SensoryCortex/SensoryCortex.toml` и `Retina/Retina.toml`).
- *Примечание*: Rust-движок считывает пути `config` как относительные строки от места запуска симуляции и не накладывает жестких синтаксических ограничений на структуру папок. `[Tier: Serde/Rust]`

---

## 3. Общие правила TOML-схемы

- **Unknown Fields**: Любые недокументированные или посторонние поля вызывают ошибку десериализации на уровне Rust/serde `[Tier: Serde/Rust]` (обеспечивается макросом `#[serde(deny_unknown_fields)]`).
- **Имя (Name) и Идентификатор (ID)**: 
  - `name` служит уникальным доменным идентификатором сущности в рамках родительского контейнера и является базовым компонентом доменного пути (например, `SensoryCortex.Retina`).
  - Во внутренней модели реактивного Store AxiCAD в качестве уникального ключа записи (primary identity) используется автоматически генерируемый UUID, а имя TOML хранится внутри `data.name`.
  - Поле `meta.id` представляет собой системные метаданные версии или архива проекта и не имеет отношения к доменному пути или уникальной идентичности биологического объекта.
- **Направление (Direction)**: Значения направления в сокетах и портах пишутся строго строчными буквами: `"in"` (входящий) или `"out"` (исходящий). `[Tier: Serde/Rust]`
- **Блок [meta] (SystemMeta)**: Необязательная секция (`Option<SystemMeta>`), которая может быть добавлена в любой корневой TOML-файл (`model.toml`, `department.toml`, `shard.toml`).

### Спецификация SystemMeta [meta] / meta:
- `id` (String, Required): Идентификатор версии или архива проекта.
- `version` (String, Required): Семантическая версия спецификации проекта (например, `1.0.0`).
- `created_at` (String, Required): Временная метка создания файла.

---

## 4. Схема model.toml (ModelConfig)

Файл `model.toml` описывает глобальные физические свойства мира, симуляции, состав департаментов и междепартаментную проводку.

### 4.1 Секция [world] (WorldConfig)
Описывает физические размеры пространства симуляции.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `width_um` | f64 | Required | Ширина мира в микрометрах. Должна быть > 0.0. `[Tier: Rust Validator]` |
| `depth_um` | f64 | Required | Глубина мира в микрометрах. Должна быть > 0.0. `[Tier: Rust Validator]` |
| `height_um` | f64 | Required | Высота мира в микрометрах. Должна быть > 0.0. `[Tier: Rust Validator]` |

### 4.2 Секция [simulation] (SimulationParams)
Содержит глобальные параметры вычислений и биологические ограничения.

| Поле | Тип данных | Обязательность | Значение по умолчанию | Описание |
|---|---|---|---|---|
| `tick_duration_us` | u32 | Required | — | Длительность симуляционного тика в микросекундах. Должна быть > 0. `[Tier: Rust Validator]` |
| `total_ticks` | u64 | Required | — | Лимит тиков симуляции. `0` означает непрерывную симуляцию. |
| `master_seed` | String | Required | — | Сид генератора случайных чисел симулятора. Не должен быть пустым. `[Tier: Rust Validator]` |
| `voxel_size_um` | f32 | Required | — | Физический размер одного вокселя в микрометрах. Должен быть > 0.0. `[Tier: Rust Validator]` |
| `segment_length_voxels` | u32 | Optional | `2` | Длина одного дискретного сегмента аксона в вокселях. Должна быть > 0. `[Tier: Rust Validator]` |
| `signal_speed_m_s` | f32 | Required | — | Физическая скорость сигнала в метрах в секунду. Должна быть > 0.0. `[Tier: Rust Validator]` |
| `sync_batch_ticks` | u32 | Required | — | Размер батча шагов для барьера синхронизации потоков. |
| `axon_growth_max_steps` | u32 | Optional | `255` | Максимальное количество шагов роста аксона при компиляции. Должно быть $\le 255$. `[Tier: Rust Validator]` |
| `max_dendrites` | u8 | Required | — | Максимальное количество дендритов на сому. Должно быть строго равно `128`. `[Tier: Rust Validator]` |

### 4.3 Массив [[departments]] (Vec\<DepartmentEntry\>)
Список логических департаментов сети.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `name` | String | Required | Имя департамента. Должно быть уникальным. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]` |
| `config` | String | Required | Относительный путь к файлу `department.toml`. Не должен быть пустым. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]` |

> Спецификация Rust Serde также поддерживает десериализацию опционального поля `meta` (`Option<SystemMeta>`) внутри элементов `[[departments]]` в `model.toml` (например):
> ```toml
> [[departments]]
> name = "SensoryCortex"
> config = "SensoryCortex/SensoryCortex.toml"
> meta = { id = "v1", version = "1.0.0", created_at = "2026-06-27" }
> ```
> Однако для MVP-редактора AxiCAD рекомендуется опускать это поле, если в нем нет необходимости.

### 4.4 Массив [[connections]] (Vec\<ModelConnectionConfig\>)
Глобальные междепартаментные соединения сокетов.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `id` | String | Recommended *(Target extension)* | Стабильный уникальный идентификатор соединения (`stable connection id/ref`). Используется для привязки геометрии трактов. *(Требует обновления Rust Serde схемы)* |
| `from` | String | Required | Точка отправления в формате `DepartmentName.ShardName.SocketName`. Человекочитаемый путь эндпоинта. `[Tier: Baker resolver / ABI checks]` |
| `to` | String | Required | Точка назначения в формате `DepartmentName.ShardName.SocketName`. Человекочитаемый путь эндпоинта. `[Tier: Baker resolver / ABI checks]` |

> [!NOTE]
> Строки `from` и `to` хранят человекочитаемые пути эндпоинтов (`human-readable endpoint paths`). Физическая геометрия трактов привязывается исключительно к стабильному идентификатору соединения (`id` / `ref`).

---

## 5. Схема department.toml (DepartmentConfig)

Департамент описывает локальную группу шардов и связи между ними.

### 5.1 Массив [[shards]] (Vec\<ShardEntry\>)
Список шардов, входящих в департамент.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `name` | String | Required | Имя шарда. Должно быть уникальным. `[Tier: Rust Validator]` (Rust проверяет уникальность имен, но не проверяет их формат `[Tier: AxiCAD editor validator]`). |
| `config` | String | Required | Относительный путь к файлу `shard.toml`. Не должен быть пустым. `[Tier: Rust Validator]` |

### 5.2 Массив [[connections]] (Vec\<DepartmentConnection\>)
Внутренние соединения сокетов в пределах департамента.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `id` | String | Recommended *(Target extension)* | Стабильный уникальный идентификатор соединения (`stable connection id/ref`). Используется для привязки геометрии трактов. *(Требует обновления Rust Serde схемы)* |
| `from` | String | Required | Исходящий сокет в формате `ShardName.SocketName`. Должен состоять строго из 2 частей. Человекочитаемый путь. `[Tier: Rust Validator]` |
| `to` | String | Required | Входящий сокет в формате `ShardName.SocketName`. Должен состоять строго из 2 частей. Человекочитаемый путь. `[Tier: Rust Validator]` |

---

## 6. Схема shard.toml (ShardConfig)

Шард описывает геометрию, состав слоев, типы клеток и локальные настройки GPU-юнита.

### 6.1 Секция [dimensions] (ShardDimensions)
Размеры шарда в вокселях.

| Поле | Тип данных | Обязательность | Ограничение (PackedPosition) |
|---|---|---|---|
| `w` | u32 | Required | Ширина (ось X). Диапазон: `1..1023`. `[Tier: Rust Validator]` |
| `d` | u32 | Required | Глубина (ось Y). Диапазон: `1..1023`. `[Tier: Rust Validator]` |
| `h` | u32 | Required | Высота (ось Z). Диапазон: `1..255`. `[Tier: Rust Validator]` |

### 6.2 Массив [[layers]] (Vec\<LayerConfig\>)
Анатомическое разделение шарда на слои по вертикальной оси Z.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `name` | String | Required | Имя слоя. |
| `height_pct` | f32 | Required | Относительная высота слоя (доля от высоты шарда `h`). Должна быть > 0.0. `[Tier: Rust Validator]` |
| `density` | f32 | Required | Доля заполненных нейронами вокселей в слое. Должна быть $\ge 0.0$ (`INV-CONFIG-002`). `[Tier: Rust Validator]` |
| `composition` | Array | Required | Массив распределения типов нейронов `NeuronTypeDistribution` в слое. |

#### Спецификация layers.composition:
- `type_name` (String, Required): имя типа нейрона. Должно быть объявлено в `[[neuron_types]]` шарда. `[Tier: Rust Validator]`
- `share` (f32, Required): относительная доля типа клеток в слое. Должна быть $\ge 0.0$. `[Tier: Rust Validator]`
- *Инвариант*: Сумма всех `composition.share` в слое должна быть равна `1.0` (±1e-4). `[Tier: Rust Validator]`
- *Инвариант*: Сумма всех `layers.height_pct` шарда должна быть равна `1.0` (±1e-4). `[Tier: Rust Validator]`

### 6.3 Массив [[neuron_types]] (Vec\<NeuronType\>)
Параметры типов нейронов шарда. Порядок в TOML-файле задает бинарный индекс типа (0..15). Длина массива не должна превышать 16 (`INV-CONFIG-001`). `[Tier: Rust Validator]`

| Имя секции | Поле | Тип данных | Описание и инварианты |
|---|---|---|---|
| — | `name` | String | Имя типа. Должно быть уникальным в шарде. `[Tier: Rust Validator]` |
| `membrane` | `threshold` | i32 | Порог срабатывания спайка (в микровольтах). |
| `membrane` | `rest_potential` | i32 | Потенциал покоя мембраны (в микровольтах). |
| `membrane` | `leak_shift` | u32 | Коэффициент экспоненциальной утечки (бит-сдвиг). |
| `membrane` | `ahp_amplitude` | u16 | Амплитуда послегиперполяризационного потенциала. |
| `timings` | `refractory_period` | u8 | Абсолютный рефрактерный период в тиках (> 0). `[Tier: Rust Validator]` |
| `timings` | `synapse_refractory_period` | u8 | Рефрактерный период синапса в тиках. |
| `signal` | `signal_propagation_length` | u8 | Длина импульса спайка. Должна быть $\ge$ `refractory_period` (`INV-CONFIG-004`). `[Tier: Rust Validator]` |
| `homeostasis`| `homeostasis_penalty` | i32 | Временный штраф (прибавка к порогу) за каждый спайк. |
| `homeostasis`| `homeostasis_decay` | u16 | Коэффициент затухания гомеостаза (доли от 1000). |
| `adaptive_leak`| `adaptive_leak_min_shift`| i32| Минимальный сдвиг адаптивной утечки. |
| `adaptive_leak`| `adaptive_leak_gain` | u16 | Множитель прироста адаптивной утечки. |
| `adaptive_leak`| `adaptive_mode` | u8 | Режим утечки (0 = выкл, 1 = активность, 2 = потенциал). |
| `dopamine` | `d1_affinity` | u8 | Сродство к D1 рецепторам (STDP пластичность). |
| `dopamine` | `d2_affinity` | u8 | Сродство к D2 рецепторам. |
| `gsop` | `gsop_potentiation` | u16 | Сила потенциации синапсов. |
| `gsop` | `gsop_depression` | u16 | Сила депрессии синапсов. |
| `gsop` | `is_inhibitory` | bool | Тормозный (true) или возбуждающий (false) тип нейрона. |
| `gsop` | `inertia_curve` | Vec\<u8\>| Кривая инерции роста. Должна содержать ровно **8 элементов**. `[Tier: Rust Validator]` |
| `growth` | `steering_fov_deg` | f32 | Угол обзора конуса роста аксона (в градусах). |
| `growth` | `steering_radius_um` | f32 | Радиус чувствительности конуса роста (в микрометрах). |
| `growth` | `steering_weight_inertia`| f32 | Вес сохранения текущего направления движения. |
| `growth` | `steering_weight_sensor` | f32 | Вес химических градиентов привлекающих веществ. |
| `growth` | `steering_weight_jitter` | f32 | Вес случайных флуктуаций (шума). |
| `growth` | `dendrite_radius_um` | f32 | Радиус дендритного дерева сомы нейрона. |
| `growth` | `growth_vertical_bias` | f32 | Вертикальный приоритет роста аксона (смещение по Z). |
| `growth` | `type_affinity` | f32 | Сродство аксона к дендритам своего типа нейрона. |
| `growth` | `dendrite_whitelist` | Vec\<String\>| Список имен разрешенных к подключению типов нейронов. |
| `growth` | `sprouting_weight_distance`| f32 | Вес расстояния при ветвлении аксона. |
| `growth` | `sprouting_weight_power`| f32 | Вес силы внешнего электрического поля. |
| `growth` | `sprouting_weight_explore`| f32 | Вес поисковой эксплорации аксона. |
| `growth` | `sprouting_weight_type` | f32 | Вес соответствия типам клеток. |
| `spontaneous`| `spontaneous_firing_period_ticks`| u32| Период спонтанной генерации спайка (0 = выключено). |

### 6.4 Массив [[sockets]] (Vec\<SocketConfig\>, Optional)
Определяет зоны межшардовых подключений на внешних гранях.

| Поле | Тип данных | Обязательность | Описание и ограничения |
|---|---|---|---|
| `name` | String | Required | Имя сокета в шарде. Должно быть уникальным. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]` (Rust проверяет только непустоту `!sock.name.is_empty()` `[Tier: Rust Validator]`). |
| `direction` | String | Required | `"in"` или `"out"`. `[Tier: Serde/Rust]` |
| `width` | u32 | Required | Ширина области подключения сокета на грани. |
| `height` | u32 | Required | Высота области подключения сокета на грани. |
| `entry_z` | String | Optional | Высотная привязка аксонов: `"Top"`, `"Mid"`, `"Bottom"`. `[Tier: Serde/Rust]` |
| `target_type` | String | Optional | Целевой тип нейрона для роста входящих аксонов. |
| `growth_steps` | u32 | Optional | Шаги роста аксона при компиляции ($\le 255$). `[Tier: Baker resolver / ABI checks]` |

### 6.5 Массив [[ports]] (Vec\<PortConfig\>, Optional)
Интерфейсы связи с внешними сенсорами и актуаторами.

| Поле | Тип данных | Обязательность | Описание и ограничения |
|---|---|---|---|
| `name` | String | Required | Имя порта. Должно быть уникальным. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]` (Rust проверяет только непустоту `!port.name.is_empty()` `[Tier: Rust Validator]`). |
| `direction` | String | Required | `"in"` или `"out"`. `[Tier: Serde/Rust]` |
| `entry_z` | String | Optional | Привязка по высоте: `"Top"`, `"Mid"`, `"Bottom"`. `[Tier: Serde/Rust]` |
| `pins` | Array | Required | Массив проекционных пинов `PinConfig`. |

#### Спецификация ports.pins:
- `name` (String, Required): Имя пина (не пустое). `[Tier: Rust Validator]`
- `width`, `height` (u32, Required): Разрешение сетки сигнала.
- `local_u`, `local_v` (f32, Required): Координаты начала проекции на грани (диапазон `0.0..1.0`). `[Tier: Rust Validator]`
- `u_width`, `v_height` (f32, Required): Коэффициенты размеров проекции на грани.
- *Инвариант*: Общая область не должна превышать размеры грани: `local_u + u_width <= 1.0` и `local_v + v_height <= 1.0`. `[Tier: Rust Validator]`
- `target_type` (String, Required): Тип клеток для проекции внешних синапсов.
- `stride` (u32, Required): Шаг разрежения проекции.
- `growth_steps` (u32, Optional): Шаги прорастания пина ($\le 255$). `[Tier: Baker resolver / ABI checks]`
- `empty_pixel` (String, Optional): Поведение при нулевом сигнале: `"skip"` или `"zero"`. `[Tier: AxiCAD editor validator]` / `[Tier: Baker resolver / ABI checks]`

### 6.6 Секция [settings] (ShardSettings)
Настройки и вместимости VRAM для шарда.

| Поле | Тип данных | Обязательность | Описание |
|---|---|---|---|
| `ghost_capacity` | u32 | Required | Слоты в VRAM для приходящих ghost-аксонов. Должна быть > 0, если у шарда есть входящие сокеты. `[Tier: Rust Validator]` |
| `prune_threshold` | i32 | Required | Порог обрезки неэффективных синаптических контактов. |
| `max_sprouts` | u32 | Required | Максимум новых ответвлений за одну ночную фазу синаптогенеза. |
| `night_interval_ticks` | u32 | Required | Интервал вызова ночной фазы симуляции (0 = отключено). |
| `save_checkpoints_interval_ticks` | u32 | Required | Интервал сохранения бэкапов состояния. |

---

## 7. Грамматика путей соединений (Connection Grammar)

Связи сокетов маршрутизируются в секциях `[[connections]]` в `department.toml` или `model.toml`. Внешние порты ввода-вывода (`[[ports]]`) взаимодействуют по сети напрямую и **не описываются** в `[[connections]]`. Строки `from` и `to` задают человекочитаемые пути эндпоинтов, в то время как привязка вынесенной геометрии трактов осуществляется через стабильный `id` (`stable connection id/ref`).

### 7.1 Адресация в department.toml (Внутренние связи)
Адресует сокеты внутри одного департамента.
- Шаблон: `<ShardName>.<SocketName>`
- Пример: `Retina.cross_modal`

### 7.2 Адресация в model.toml (Междепартаментные связи)
Адресует сокеты между шардами из разных департаментов.
- Шаблон: `<DepartmentName>.<ShardName>.<SocketName>`
- Пример: `SensoryCortex.Retina.motor_commands`

---

## 8. Канонические валидные примеры TOML

Ниже приведены минимальные валидные конфигурационные файлы, соответствующие структурам `ModelConfig`, `DepartmentConfig` и `ShardConfig`.

### 8.1 Корневой model.toml
```toml
connections = [] # Плоские массивы корневого уровня пишутся СТРОГО до первого раздела [world]

[world]
width_um = 1000.0
depth_um = 1000.0
height_um = 500.0

[simulation]
tick_duration_us = 1000
total_ticks = 0
master_seed = "FishBrainSeed"
voxel_size_um = 10.0
segment_length_voxels = 2
signal_speed_m_s = 2.0
sync_batch_ticks = 10
axon_growth_max_steps = 200
max_dendrites = 128

[[departments]]
name = "SensoryCortex"
config = "SensoryCortex/SensoryCortex.toml"
```

### 8.2 Департамент SensoryCortex/SensoryCortex.toml
```toml
connections = [] # Пустые локальные соединения департамента до списка [[shards]]

[[shards]]
name = "Retina"
config = "Retina/Retina.toml"
```

### 8.3 Шард SensoryCortex/Retina/Retina.toml
```toml
[dimensions]
w = 256
d = 256
h = 63

[settings]
ghost_capacity = 1024
prune_threshold = 15
max_sprouts = 4
night_interval_ticks = 10000
save_checkpoints_interval_ticks = 100000

[[layers]]
name = "L4_Sensory"
height_pct = 0.6
density = 0.8
composition = [
    { type_name = "Stellate_Exc", share = 1.0 }
]

[[layers]]
name = "L5_Output"
height_pct = 0.4
density = 0.5
composition = [
    { type_name = "Stellate_Exc", share = 1.0 }
]

[[neuron_types]]
name = "Stellate_Exc"

  [neuron_types.membrane]
  threshold = 20000
  rest_potential = -70000
  leak_shift = 4
  ahp_amplitude = 0

  [neuron_types.timings]
  refractory_period = 5
  synapse_refractory_period = 10

  [neuron_types.signal]
  signal_propagation_length = 8

  [neuron_types.homeostasis]
  homeostasis_penalty = 1500
  homeostasis_decay = 990

  [neuron_types.adaptive_leak]
  adaptive_leak_min_shift = -5
  adaptive_leak_gain = 2
  adaptive_mode = 1

  [neuron_types.dopamine]
  d1_affinity = 80
  d2_affinity = 20

  [neuron_types.gsop]
  gsop_potentiation = 15
  gsop_depression = 5
  is_inhibitory = false
  inertia_curve = [10, 20, 30, 40, 50, 60, 70, 80]

  [neuron_types.growth]
  steering_fov_deg = 60.0
  steering_radius_um = 100.0
  steering_weight_inertia = 0.6
  steering_weight_sensor = 0.3
  steering_weight_jitter = 0.1
  dendrite_radius_um = 150.0
  growth_vertical_bias = 0.7
  type_affinity = 0.5
  dendrite_whitelist = []
  sprouting_weight_distance = 0.4
  sprouting_weight_power = 0.4
  sprouting_weight_explore = 0.1
  sprouting_weight_type = 0.1

  [neuron_types.spontaneous]
  spontaneous_firing_period_ticks = 10000

[[sockets]]
name = "cross_modal"
direction = "out"
width = 8
height = 8

[[ports]]
name = "retina_feed"
direction = "in"
entry_z = "Top"

  [[ports.pins]]
  name = "retina_left"
  width = 28
  height = 16
  local_u = 0.0
  local_v = 0.0
  u_width = 0.5
  v_height = 1.0
  target_type = "Stellate_Exc"
  stride = 1
  growth_steps = 255
  empty_pixel = "skip"
```

---

## Changelog

| Дата | Изменение |
|------|-----------|
| 2026-06-28 | Синхронизация схемы TOML с архитектурными решениями: зафиксировано отдельное хранение детальной геометрии трактов (dedicated storage), стабильные connection id/refs, системный тип `white_matter` и `insta-connect` route mode. |
| 2026-06-27 | Создание спецификации канонической TOML-схемы Axicor (TOML Schema Spec). Описаны структуры и правила десериализации, Connection grammar, Validation Tiers, различие name/id, а также канонические TOML-примеры с учетом table scopes. |
