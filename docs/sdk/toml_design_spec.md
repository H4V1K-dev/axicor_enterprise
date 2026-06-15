# TOML Design Spec — 3-Level Model Hierarchy (Sockets & Ports Edition)

> Дизайн конфигурационных файлов.
> Всё что здесь — это **декларация пользователя** для Python SDK, Baker и Визуализатора.
> Всё что здесь НЕ упомянуто — генерируется Baker'ом в manifest.

---

## Обзор: Что Где Живёт

```
model.toml                          ← Физика + список департаментов + МЕЖД. провода (сокетов)
├── SensoryCortex/
│   ├── SensoryCortex.toml          ← Список шардов + ВНУТР. провода (сокетов)
│   ├── Retina/
│   │   └── Retina.toml             ← Геометрия + слои + нейроны + сокеты + порты
│   └── Auditory/
│       └── Auditory.toml
└── MotorGanglion/
    ├── MotorGanglion.toml
    └── SpinalRelay/
        └── SpinalRelay.toml
```

### Правила разделения: Сокеты и Порты

```
1. Внутренние связи (между шардами) = [[sockets]]
   - Объявляются в shard.toml с указанием direction ("in" / "out") и геометрии.
   - Маршрутизируются через [[connections]] в department.toml или model.toml.

2. Связи с внешним миром (сенсоры, моторы, Python-агент) = [[ports]]
   - Объявляются в shard.toml с указанием direction ("in" / "out").
   - Не требуют записей в [[connections]]. Baker автоматически открывает UDP сокет для каждого.
```

---

## Level 1: model.toml

Один файл на всю модель. Содержит **физические законы** и **топологию департаментов** + меж-департаментную проводку.

```toml
# ═══════════════════════════════════════════════════════
# FishBrain.toml — Model Configuration
# ═══════════════════════════════════════════════════════

# ─── Пространство ──────────────────────────────────────────────
# Физические размеры пространства симуляции.
# Baker использует для вычисления bounds при placement.
[world]
width_um  = 25000.0       # Ширина мира (микрометры)
depth_um  = 25000.0       # Глубина мира
height_um = 6375.0        # Высота мира

# ─── Физика ───────────────────────────────────────────
# Глобальные параметры симуляции. Одинаковы для ВСЕХ шардов.
[simulation]
tick_duration_us       = 100       # Длительность одного тика (мкс)
total_ticks            = 0         # 0 = бесконечно (live mode)
master_seed            = "AXICOR"  # Детерминистичный сид для RNG
voxel_size_um          = 25.0      # Размер вокселя (мкм)
signal_speed_m_s       = 0.5       # Скорость сигнала (м/с)
sync_batch_ticks       = 100       # Batch для sync (BSP барьер)
segment_length_voxels  = 2         # Длина сегмента аксона (воксели)
axon_growth_max_steps  = 250       # Макс шагов роста аксона (≤255)
max_dendrites          = 128       # Макс дендритов на нейрон (хардкод)

# ─── Департаменты ─────────────────────────────────────
# Каждый департамент = независимый "мозг" со своей топологией.
# `config` — относительный путь к department.toml
[[departments]]
name   = "SensoryCortex"
config = "SensoryCortex/SensoryCortex.toml"

[[departments]]
name   = "MotorGanglion"
config = "MotorGanglion/MotorGanglion.toml"

# ─── Межд. проводка сокетов ───────────────────────────
# Связи МЕЖДУ шардами РАЗНЫХ департаментов.
# `from` / `to` — точечная нотация: "Департамент.Шард.Сокет"
# Логика и геометрия подключения лежат внутри самих сокетов в shard.toml.
[[connections]]
from = "SensoryCortex.Retina.motor_commands"
to   = "MotorGanglion.SpinalRelay.motor_feed"
```

---

## Level 2: department.toml (Департамент = Отдел мозга/Еденица трансформера)

Один файл на департамент. Содержит список входящих шардов и проводку между ними.

```toml
# ═══════════════════════════════════════════════════════
# SensoryCortex/SensoryCortex.toml — Department Config
# ═══════════════════════════════════════════════════════

# ─── Шарды ────────────────────────────────────────────
# Каждый шард = один GPU-процесс со своей VRAM.
# `config` — относительный путь к shard.toml
[[shards]]
name   = "Retina"
config = "Retina/Retina.toml"

[[shards]]
name   = "Auditory"
config = "Auditory/Auditory.toml"

# ─── Внутр. проводка сокетов ──────────────────────────
# Связи МЕЖДУ шардами ВНУТРИ этого департамента.
# `from` / `to` — точечная нотация: "Шард.Сокет" (без префикса департамента)
[[connections]]
from = "Retina.cross_modal"
to   = "Auditory.cross_feed"
```

---

## Level 3: shard.toml (Шард = GPU Unit)

Содержит всё, что необходимо для описания одного вычислительного юнита: геометрия, анатомия, биологические типы нейронов, интерфейсы (сокеты и порты) и локальные настройки.

```toml
# ═══════════════════════════════════════════════════════
# SensoryCortex/Retina/Retina.toml — Shard Config
# ═══════════════════════════════════════════════════════

# ═══════ БЛОК 1: ГЕОМЕТРИЯ ════════════════════════════
# Размеры шарда в вокселях.
# w,d: 0..1023 (10 бит), h: 0..255 (8 бит) — PackedPosition лимит.
[dimensions]
w = 256          # Ширина (X)
d = 256          # Глубина (Y)
h = 63           # Высота (Z)

# ═══════ БЛОК 2: АНАТОМИЯ ═════════════════════════════
# Кортикальные слои. Укладываются снизу вверх.
# Инварианты:
#   - sum(height_pct) == 1.0 (±1e-4)
#   - sum(share) в каждом слое == 1.0 (±1e-4)
#   - type_name ссылается на существующий [[neuron_types]]
[[layers]]
name       = "L4_Sensory"
height_pct = 0.6           # 60% высоты шарда
density    = 0.8            # Доля заполненных вокселей
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

# ═══════ БЛОК 3: БЛУПРИНТЫ (НЕЙРОННЫЕ ТИПЫ) ══════════
# Макс 16 типов на шард (4-бит LUT в GPU Constant Memory).
# Все поля жестко соответствуют ABI структуре в blueprints.rs.
[[neuron_types]]
name = "Stellate_Exc"

  [neuron_types.membrane]
  threshold      = 20000       # Порог спайка (i32, микровольты)
  rest_potential  = -70000      # Потенциал покоя (i32)
  leak_shift      = 4           # Экспоненциальная утечка (сдвиг)
  ahp_amplitude   = 0           # Амплитуда AHP (после-гиперполяризация, u16)

  [neuron_types.timings]
  refractory_period          = 5    # Абсолютный рефрактерный (тики)
  synapse_refractory_period  = 10   # Синаптический рефрактерный

  [neuron_types.signal]
  signal_propagation_length  = 8    # Длина хвоста спайка (≥refractory)

  [neuron_types.homeostasis]
  homeostasis_penalty  = 1500       # Штраф за спайк
  homeostasis_decay    = 990        # Затухание (из 1000)

  [neuron_types.adaptive_leak]
  adaptive_leak_min_shift = -5
  adaptive_leak_gain      = 2
  adaptive_mode           = 1       # 0=off, 1=activity, 2=voltage

  [neuron_types.dopamine]
  d1_affinity = 80
  d2_affinity = 20

  [neuron_types.gsop]
  gsop_potentiation = 15
  gsop_depression   = 5
  is_inhibitory     = false
  inertia_curve     = [10, 20, 30, 40, 50, 60, 70, 80]   # ровно 8 элементов

  [neuron_types.growth]
  steering_fov_deg          = 60.0       # FOV роста аксона (градусы)
  steering_radius_um        = 100.0      # Радиус поиска при росте (мкм)
  steering_weight_inertia   = 0.6        # Вес инерции направления
  steering_weight_sensor    = 0.3        # Вес химического сенсора
  steering_weight_jitter    = 0.1        # Вес шума (jitter) при росте
  dendrite_radius_um        = 150.0      # Радиус дендритного дерева (мкм)
  growth_vertical_bias      = 0.7        # Вертикальное смещение роста аксона
  type_affinity             = 0.5        # Родство по типу нейрона
  dendrite_whitelist        = []         # Список разрешенных типов (пусто = все)
  sprouting_weight_distance = 0.4        # Вес расстояния при ветвлении (sprouting)
  sprouting_weight_power    = 0.4        # Вес силы сигнала
  sprouting_weight_explore  = 0.1        # Вес исследования
  sprouting_weight_type     = 0.1        # Вес соответствия типу

  [neuron_types.spontaneous]
  spontaneous_firing_period_ticks = 10000    # 0 = нет спонтанной активности

[[neuron_types]]
name = "Basket_Inh"
  # ... аналогичная структура, is_inhibitory = true

[[neuron_types]]
name = "Pyramidal_Exc"
  # ... аналогичная структура

# ═══════ БЛОК 4: ВНУТРЕННИЕ СОКЕТЫ (INTERNAL SOCKETS) ════
# Локальные интерфейсы связей с другими шардами.
# Физические параметры подключения инкапсулированы здесь.

[[sockets]]
name      = "motor_commands"
direction = "in"
width     = 16
height    = 16

[[sockets]]
name         = "cross_modal"
direction    = "in"
width        = 8
height       = 8

[[sockets]]
name         = "cross_feed"
direction    = "out"
width        = 8
height       = 8
entry_z      = "Mid"               # Точка входа: Top / Mid / Bottom или 0.0..1.0
target_type  = "L4_Stellate"       # Целевой тип нейронов для роста
growth_steps = 800                 # Шаги роста аксона при компиляции

# ═══════ БЛОК 5: ВНЕШНИЕ ПОРТЫ (EXTERNAL IO PORTS) ═══════
# Интерфейсы взаимодействия с внешним миром (UDP / Python).
# Baker генерирует UDP-сокет и пин-хэш на основе этих данных.

[[ports]]
name      = "retina_feed"
direction = "in"
entry_z   = "Top"

  [[ports.pins]]
  name         = "retina_left"
  width        = 28
  height       = 16
  local_u      = 0.0             # Нормализованная U-координата (0.0..1.0)
  local_v      = 0.0             # Нормализованная V-координата (0.0..1.0)
  u_width      = 0.5             # Нормализованная ширина проекции
  v_height     = 1.0             # Нормализованная высота проекции
  target_type  = "Stellate_Exc"
  stride       = 1
  growth_steps = 255
  empty_pixel  = "skip"          # "skip" / "zero"

  [[ports.pins]]
  name         = "retina_right"
  width        = 28
  height       = 16
  local_u      = 0.5
  local_v      = 0.0
  u_width      = 0.5
  v_height     = 1.0
  target_type  = "Stellate_Exc"
  stride       = 1
  growth_steps = 75
  empty_pixel  = "skip"

[[ports]]
name      = "motor_commands_out"
direction = "out"

  [[ports.pins]]
  name        = "motor_full"
  width       = 16
  height      = 16
  local_u     = 0.0
  local_v     = 0.0
  u_width     = 1.0
  v_height    = 1.0
  target_type = "All"
  stride      = 1

# ═══════ БЛОК 6: НАСТРОЙКИ ШАРДА ═════════════════════
[settings]
ghost_capacity                   = 1024    # VRAM слоты под входящие ghost-аксоны
prune_threshold                  = 15      # Порог обрезки слабых синапсов
max_sprouts                      = 4       # Макс побегов при night-phase
night_interval_ticks             = 10000   # Интервал ночной фазы (0 = выкл)
save_checkpoints_interval_ticks  = 100000  # Интервал чекпоинтов
```

---

## Внешний IO — Как Работает

Связь с камерами или RL-агентами идёт через внешние порты (`[[ports]]`).

1. Шард объявляет порт `[[ports]]` с `direction = "in"` и именем `"retina_feed"`.
2. Baker компилирует топологию и выделяет сетевой UDP-порт в `manifest.toml` (например, `external_udp_in = 8081`).
3. При компиляции Baker рассчитывает `pin_hash = fnv1a32("retina_left")`.
4. В рантайме внешний Python-агент подключается к UDP-порту `8081` и передает пакеты с заголовком `pin_hash`.
5. Пользователю не нужно вручную прописывать порты или IP в TOML — Baker и рантайм связывают внешнее устройство и физический порт автоматически.

---

## SDK Линтер — Что Проверяет

### На уровне shard.toml:
1. `sum(layer.height_pct) == 1.0` (±1e-4).
2. `sum(composition.share) == 1.0` для каждого слоя.
3. Имена типов клеток в `composition.type_name` должны присутствовать в списке `[[neuron_types]]`.
4. Длина списка `[[neuron_types]]` не превышает 16 (LUT-лимит GPU).
5. Размеры шарда `dimensions` находятся в пределах: $w \le 1023, d \le 1023, h \le 255$.
6. Массив `inertia_curve` имеет ровно 8 элементов.
7. `signal_propagation_length >= refractory_period` для всех типов.
8. Если у шарда есть входящие сокеты (`[[sockets]]` с `direction = "in"`), его `ghost_capacity` должен быть строго больше нуля.
9. Координаты проекционных пинов внешних портов лежат строго в границах $0.0..1.0$: `local_u + u_width <= 1.0` и `local_v + v_height <= 1.0`.

### На уровне department.toml:
1. Каждая связь в `[[connections]]` должна указывать на существующие шарды и сокеты, объявленные внутри департамента.
2. Сокет источника должен иметь `direction = "out"`, сокет приёмника — `direction = "in"`.
3. Совпадение размерностей сокетов: `from_socket.width == to_socket.width` and `from_socket.height == to_socket.height`.
4. В списке `[[shards]]` нет дубликатов имён.

### На уровне model.toml:
1. Физический шаг сигнала `segment_length_voxels` должен быть строго целым числом.
2. Каждая связь в `[[connections]]` должна успешно разрешаться в формате `Department.Shard.Socket`.
3. Сокет источника должен иметь `direction = "out"`, сокет приёмника — `direction = "in"`.
4. Совпадение размерностей сокетов: `from_socket.width == to_socket.width` and `from_socket.height == to_socket.height`.
5. Нет дубликатов департаментов.
6. Для каждого шарда-приёмника его настроенный `ghost_capacity` должен быть больше или равен сумме площадей (`width * height`) всех входящих в его сокеты соединений.
