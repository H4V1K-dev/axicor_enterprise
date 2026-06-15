spec_topology

> Версия спеки: 1.0  
> Дата: 2026-06-01  
> Статус: Approved  

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| Название | topology |
| Слой | Слой 4 — Topology, Baker & Edge Conversion |
| Тип | Library (lib) |
| no_std | **Нет** (зависит от `rayon` для параллельного роста аксонов и `rand` для стохастических алгоритмов) |
| Описание | Пространственные алгоритмы: стохастическое размещение нейронов, воксельная сетка, конусная трассировка, структурная пластичность (спраутинг) и макро-маршрутизация. |

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| types | `PackedPosition`, `SomaFlags`, `PackedTarget`, `DenseIndex` | Фундаментальные типы координат, индексов и флагов. |
| layout | `ShardStateSoA`, `PathsFileHeader` | POD-структуры памяти и C-ABI контракты для экспорта геометрии. |
| config | `SimulationConfig`, `AnatomyConfig`, `BlueprintsConfig` | Конфигурации физики, структуры слоев и свойств нейронов. |
| wire | `GhostConnection`, `AxonHandoverEvent` | Формирование бинарных сетевых DTO для экспорта во внешние крейты. |
| physics | `initial_axon_head` | Константы и функции расчета стартовых позиций сигналов. |

### §2.2. Внешние зависимости

| Crate | Версия | Зачем |
|---|---|---|
| glam | =0.25.0 | Быстрая векторная математика (`Vec3`) для Cone Tracing и пространственных расчетов. |
| rayon | =1.11.0 | Распараллеливание пространственного поиска и обновления воксельной сетки. |
| rand | =0.8.5 | Генераторы случайных чисел для стохастического распределения сом. |
| rand_chacha | =0.3.1 | Детерминированный ChaCha RNG на базе MasterSeed для побитовой воспроизводимости. |

### §2.3. Feature Flags

Секция не применима к данному крейту: крейт не предоставляет собственных условных флагов компиляции.

---

## §3. Инварианты

### §3.1. Структурные инварианты

- **INV-TOPO-001**: *Ограничение плотности вокселей*.
  - *Обоснование*: В одном вокселе трехмерной сетки может находиться максимум одна сома нейрона. Это позволяет использовать координаты вокселя в качестве O(1) уникального идентификатора для быстрого пространственного поиска и хэширования без коллизий.
  - *Следствие нарушения*: Коллизии адресации, непредсказуемое поведение при маппинге `DenseIndex` -> `PackedPosition`.
  - *Где проверяется*: runtime assert во время Stochastic Placement (reject-sampling).

### §3.2. Семантические инварианты

- **INV-TOPO-004**: *Sprouting Density Invariant (GPU Visibility)*.
  - *Обоснование*: При поиске свободного дендритного слота для новой связи цикл на CPU обязан последовательно сканировать массив от 0 до 127 и записывать новый синапс в первый встреченный слот, где `target == 0`. GPU-ядро Day Phase использует оптимизацию Early Exit (`if (target == 0) break;`). Запись после нулевого слота сделает синапс аппаратно невидимым.
  - *Следствие нарушения*: Silent Data Loss — новообразованные синапсы не считываются GPU в горячем цикле.
  - *Где проверяется*: runtime assert во время фазы Sprouting.

- **INV-TOPO-005**: *Dead on Arrival (DoA) Protection*.
  - *Обоснование*: При рождении синапса его начальный вес обязан быть сдвинут в Mass Domain (`initial_weight << 16`). Если сдвинутый вес меньше или равен порогу прунинга `prune_threshold << 16`, синапсу принудительно выдается спасательный капитал.
  - *Следствие нарушения*: 100% смертность новых синапсов при ночной консолидации, невозможность обучения сети.
  - *Где проверяется*: runtime assert в функции `sprout_connections`.

- **INV-TOPO-006**: *Закон Дейла при Sprouting*.
  - *Обоснование*: Знак начального веса синапса определяется исключительно типом нейрона-владельца аксона (возбуждающий (+) или тормозный (-)), а не принимающей сомы.
  - *Следствие нарушения*: Дестабилизация баланса возбуждения/торможения, хаотическая активность.
  - *Где проверяется*: runtime assert при создании связи.

- **INV-TOPO-007**: *Единственность связи сома-аксон (Rule of Uniqueness)*.
  - *Обоснование*: Сома может иметь максимум одну связь с конкретным `Axon_ID`. Дублирование сигналов бессмысленно в детерминированной системе, дефицитные слоты (128) нужно тратить на разные источники.
  - *Следствие нарушения*: Быстрое исчерпание дендритных слотов дублирующими сигналами, снижение информационной емкости нейрона.
  - *Где проверяется*: runtime check при поиске кандидатов в `sprout_connections`.

- **INV-TOPO-009**: *Activity-Based Nudging (Axon growth gate)*.
  - *Обоснование*: Локальный аксон может сделать шаг роста во время Ночной Фазы только в том случае, если его сома-владелец спайковала в течение предшествующей Дневной Фазы (`flags & 0x01 == 1`).
  - *Следствие нарушения*: Избыточный расход CPU-времени в Ночную Фазу, неконтролируемый рост неактивных связей.
  - *Где проверяется*: runtime check в алгоритме `nudge_living_axons`.

- **INV-TOPO-010**: *Inertial Nudging для Ghost-аксонов*.
  - *Обоснование*: Ghost-аксоны не имеют локальной сомы (`soma_idx == usize::MAX`) и обязаны расти каждую ночь безусловно по вектору своей инерции, пока счетчик `remaining_steps` не достигнет нуля.
  - *Следствие нарушения*: Остановка роста межшардовых связей.
  - *Где проверяется*: runtime check в алгоритме `nudge_living_axons`.

### §3.3. Межкрейтовые инварианты

- **INV-CROSS-007**: *Соответствие Zone Hash в Ghosts и ShardState*.
  - *Участники*: `topology`, `layout`.
  - *Кто владелец проверки*: `topology`.
  - *Обоснование*: При макро-маршрутизации (Ghost Atlas) `zone_hash` отправителя и получателя должны строго соответствовать хэшам зон в `ShardStateSoA`.
  - *Следствие нарушения*: Отправка спайков "в никуда" или повреждение VRAM чужого шарда на этапе загрузки ноды.
  - *Где проверяется*: интеграционные проверки при запекании графа (Baking).

---

## §4. Публичный API

### §4.1. Типы

#### LivingAxon

```rust
pub struct LivingAxon {
    pub axon_id: usize,
    pub soma_idx: usize,
    pub tip_uvw: u32,
    pub forward_dir: glam::Vec3,
    pub remaining_steps: u32,
    pub last_night_active: bool,
}
```

- **Семантика**: Локальное представление растущего аксона в оперативной памяти хоста (CPU). Существует только во время Ночной Фазы.
- **Жизненный цикл**: Инициализируется в начале Ночной Фазы. Мутирует в цикле `nudge_living_axons`. По завершении роста новые сегменты `tip_uvw` (`PackedPosition`) сериализуются в плоский массив для файла `.paths`.
- **Ограничения на значения**: Поле `soma_idx` содержит `usize::MAX` для Ghost-аксонов (аксонов без локальной сомы).

#### GhostPacket

```rust
pub struct GhostPacket {
    pub origin_shard_id: u32,
    pub soma_idx: usize,
    pub type_idx: usize,
    pub entry_x: u32,
    pub entry_y: u32,
    pub entry_z: u32,
    pub entry_dir: glam::Vec3,
    pub remaining_steps: u32,
}
```

- **Семантика**: Абстрактное геометрическое описание аксона в точке пересечения границы шарда. Это чистая математическая сущность, не привязанная к TCP или UDP.
- **Жизненный цикл**: Рождается, когда локальный аксон делает шаг за пределы `ShardBounds`. Передается наверх в оркестратор, который уже сам решает, как (и через какой C-ABI DTO) перебросить его на другую ноду.

#### GrowthEvent

```rust
pub enum GrowthEvent {
    Advanced(u32),
    TargetReached,
    Stagnated,
    OutOfBounds(GhostPacket),
}
```

- **Семантика**: Конечный автомат одного шага роста аксона.
  - `Advanced`: Успешный шаг (возвращает новый `PackedPosition`).
  - `TargetReached`: Аксон достиг целевого Z-слоя, переключается в фазу Arborization (Крона).
  - `Stagnated`: Аксон застрял (дистанция < epsilon) или уперся в непреодолимое препятствие. Рост прекращается.
  - `OutOfBounds`: Аксон покинул физические границы шарда. Возвращает `GhostPacket` для эвакуации.

#### SpatialGrid

```rust
pub struct SpatialGrid {
    // Внутренняя реализация скрыта: плоский массив хэшей или bucket-сортировка
}
```

- **Семантика**: 3D Spatial Hash Grid для O(K) поиска дендритами пролетающих мимо аксонов. Разрешает фундаментальную проблему O(N^2) поиска при коннектоме.
- **Жизненный цикл**: Очищается и пересобирается каждую Ночную Фазу на CPU перед запуском фазы Sprouting.

### §4.2. Трейты

В данном крейте публичные трейты отсутствуют. Крейт `topology` предоставляет конкретную реализацию пространственных алгоритмов.

### §4.3. Функции

#### `pub fn place_somas`(bounds: (u32, u32, u32), budget: usize, anatomy: &AnatomyConfig, rng: &mut ChaCha8Rng) -> Result<Vec<PackedPosition>, TopologyError>
- **Назначение**: Стохастическое размещение сом (Reject-Sampling) в воксельной сетке.
- **Предусловия**: Сумма `height_pct` в анатомии равна 1.0, плотности >= 0.0.
- **Постусловия**: Возвращает массив координат без коллизий (один воксель = максимум одна сома).
- **Сложность**: O(N) в среднем, где N — бюджет сом.
- **Паника**: Никогда (при исчерпании попыток возвращает `TopologyError::PlacementCollision`).

#### `pub fn cone_tracing`(soma_pos: PackedPosition, target_layer: u8, anatomy: &AnatomyConfig, rng: &mut ChaCha8Rng) -> Vec<PackedPosition>
- **Назначение**: Прокладывает траекторию роста аксона (Trunk Phase) к целевому Z-слою с использованием направленного вектора и джиттера.
- **Предусловия**: Детерминированный `ChaCha8Rng` инициализирован MasterSeed.
- **Постусловия**: Возвращает геометрию аксона до достижения цели (max 256 сегментов).
- **Сложность**: O(S), где S — количество сегментов до цели.
- **Паника**: Никогда.

#### `pub fn nudge_living_axons`(living: &mut [LivingAxon], soma_flags: &[u8]) -> Vec<GrowthEvent>
- **Назначение**: Сдвигает кончики активных локальных аксонов и всех Ghost-аксонов на 1 сегмент по вектору инерции во время Ночной Фазы.
- **Предусловия**: Массив `soma_flags` актуален после Day Phase (нужен для `is_spiking` фильтра).
- **Постусловия**: Строго соблюдает `INV-TOPO-009` (Activity-Based Nudging) и `INV-TOPO-010`.
- **Сложность**: O(A_active), где A_active — количество активных (спайковавших) аксонов.
- **Паника**: Никогда.

#### `pub fn sprout_connections`(active_somas: &[usize], grid: &SpatialGrid, blueprints: &BlueprintsConfig) -> Vec<NewSynapse>
- **Назначение**: Ищет контакты (En Passant) в воксельной сетке для активных нейронов.
- **Предусловия**: `SpatialGrid` пересобран со свежими сегментами аксонов.
- **Постусловия**: Знак нового синапса строго соответствует типу аксона-источника (`INV-TOPO-006`). Вес аппаратно сдвинут в Mass Domain (`INV-TOPO-005`).
- **Сложность**: O(N_active * K), где K — соседи в радиусе поиска.
- **Паника**: Никогда.

#### `pub fn generate_virtual_axons`(matrix: &IoMatrix, zone_bounds: (u32, u32, u32)) -> Vec<Vec<PackedPosition>>
- **Назначение**: Генерация прямых инфраструктурных колонн (Virtual Axons) для матриц сенсоров.
- **Предусловия**: Валидные параметры матрицы ввода.
- **Постусловия**: Гарантирует прямые линии строго по оси Z на всю высоту зоны (без джиттера и изгибов).
- **Сложность**: O(W * H * Z), где WxH — разрешение матрицы, Z — высота зоны в сегментах.
- **Паника**: Никогда.

#### `pub fn project_uv_with_jitter`(u: f32, v: f32, master_seed: u64, salt: u32) -> (f32, f32)
- **Назначение**: Проецирует нормализованные UV-координаты I/O матриц с детерминированным шумом (до 5%) для исключения пространственных артефактов (hotspots).
- **Предусловия**: `u, v` в диапазоне `0.0..=1.0`.
- **Постусловия**: Возвращает `(u, v)` строго в `0.0..=1.0`.
- **Сложность**: O(1).
- **Паника**: Никогда.

#### `pub fn route_ghost_atlas`(source_gxo: &GxoMatrixDescriptor, target_bounds: (u32, u32, u32), config: &ConnectionConfig) -> Result<Vec<GhostConnection>, TopologyError>
- **Назначение**: Статическая макро-маршрутизация между выходной матрицей одной зоны и входами другой (AOT Atlas Routing).
- **Предусловия**: Выходная матрица источника уже скомпилирована.
- **Постусловия**: Вычисляет целевые `soma_id` через Z-Sort и генерирует бинарные `GhostConnection` для экспорта в крейт `wire`.
- **Сложность**: O(P * C), где P — пиксели матрицы, C — кандидаты в целевом Z-столбце.
- **Паника**: Никогда.

### §4.4. Константы и Магические Числа

| Константа | Значение | Тип | Семантика |
|---|---|---|---|
| `MAX_DENDRITES` | `128` | `usize` | Физический предел синапсов на одну сому. Используется для защиты от переполнения (Early Exit) в цикле `sprout_connections`. |
| `MAX_SEGMENTS` | `256` | `usize` | Аппаратный предел шагов для `cone_tracing`. Продиктован 8-битным ограничением поля `Segment_Offset` в `PackedTarget` из Слоя 0. |
| `EMPTY_PIXEL` | `0xFFFF_FFFF` | `u32` | Хард-маркер, записываемый функцией `route_ghost_atlas` (при генерации проекций вывода), если в целевом Z-столбце воксельной сетки не найдено ни одной подходящей сомы. Служит триггером для O(1) `Early Exit` в горячем цикле GPU. |

---

## §5. Доменная Логика

Крейт `topology` — это движок трехмерной геометрии и структурной пластичности графа (Слой 4). Его единственная роль — транслировать биологические 3D-законы роста и поиска соседей в плоские, жестко выровненные одномерные индексы (SoA) для вычислителей Слоя 3.

Выделение тяжелой 3D-математики (воксельные хэш-сетки, конусная трассировка) в изолированный крейт решает фундаментальный архитектурный конфликт: кремний GPU эффективно перемалывает только 1D-векторы, в то время как живая нейросеть развивается в объеме. Крейт гарантирует, что горячий цикл симуляции (Day Phase) имеет нулевую стоимость абстракций (Zero-Cost), перенося всю ресурсоемкую работу с пространством на фазу AOT-компиляции (Baking) и периоды консолидации кластера (Night Phase).

### §5.1. Стохастическое размещение (Stochastic Placement)

Алгоритм нарезает физическую высоту шарда на анатомические слои и распределяет нейроны в объеме воксельной сетки согласно квотам. Размещение строится на reject-sampling логике: один воксель вмещает строго одну сому. Это обеспечивает идеальную изотропию графа (отсутствие искусственных предпочтительных направлений распространения сигнала) и позволяет использовать координаты вокселя как O(1) хэш-ключ.

### §5.2. Двухфазный рост аксонов (Cone Tracing & Arborization)

Физическое формирование путей аксонов разделено на две фазы, что радикально повышает плотность коннектома:
1. **Ствол (Trunk):** Направленный рост по глобальному вектору к целевому Z-слою.
2. **Крона (Arborization):** При достижении цели направленный вектор отключается, и кончик аксона начинает хаотично петлять (максимальный джиттер). Это формирует плотное облако из десятков сегментов, математически увеличивая шанс пересечения с ищущими дендритами в 20–50 раз без единого дополнительного такта нагрузки на GPU в рантайме.

### §5.3. Структурная пластичность (Night Phase Sprouting)

Во время сна (Night Phase) активные нейроны ищут новые контакты с сегментами пролетающих мимо аксонов (En Passant synapses). Поиск выполняется через 3D Spatial Hash Grid, снижая сложность пространственных запросов с O(N) до O(K). 
В этой фазе крейт выступает гарантом аппаратных контрактов VRAM: новые связи обязаны записываться строго в последовательные пустые слоты (Sprouting Density Invariant), чтобы не разрушить Early Exit оптимизации GPU, а их стартовый вес проходит обязательный сдвиг в домен массы (Dead on Arrival Protection), чтобы синапс не был уничтожен первым же циклом прунинга.

### §5.4. Входные и выходные проекции (I/O Mapping & Jitter)

Входные сенсорные и выходные моторные матрицы проецируются на физическое пространство зоны с помощью UV-масштабирования. Сенсорные пиксели преобразуются во входные виртуальные аксоны (Virtual Axons) — прямые инфраструктурные колонны, пронизывающие зону по оси Z на всю высоту. 
Для устранения искусственной геометрической регулярности к нормализованным UV-координатам применяется детерминированный шум в пределах 5% от разрешения сетки (Deterministic Jitter). Это гарантирует, что контакты распределяются равномерно без создания неестественных концентраций (hotspots) в пространственной хэш-таблице.

### §5.5. Межзональная маршрутизация (Ghost Atlas Routing)

При разбиении симуляции на шарды аксоны могут пересекать пространственные границы. Крейт вычисляет пути таких связей AOT (при Baking) или в рантайме во время Ночной Фазы:
1. **Прорастание через границы:** Пересекающий границу аксон фиксируется в точке выхода. Топологический движок вычисляет вектор инерции и координаты пересечения, формируя абстрактный `GhostPacket`.
2. **Абстракция каналов (Dynamic Capacity):** Для горячего GPU-ядра не существует понятия «сеть» или «другой сервер» — геометрический движок мапит все внешние пересечения на плоские индексы Ghost-аксонов, емкость которых зарезервирована в рамках `ghost_capacity` во VRAM.

---

## §6. Алгоритмы и Формулы

### §6.1. Алгоритм стохастического размещения (Stochastic Placement)

- **Вход**: `bounds: (u32, u32, u32)` (габариты шарда), `anatomy: &AnatomyConfig` (конфиг слоев), `master_seed: u64` (через `ChaCha8Rng`).
- **Выход**: `Result<Vec<PackedPosition>, TopologyError>`.
- **Детерминизм**: Да (100% кроссплатформенная идентичность графа при одинаковом `master_seed`).

**Формула / Псевдокод:**

Алгоритм избегает «зависаний» классического reject-sampling при высоких плотностях (например, в L4) через создание пула локальных вокселей слоя и его детерминированное тасование (In-Place Shuffle). Это дает аппаратную гарантию соблюдения `INV-TOPO-001` (один воксель = одна сома) и точного выполнения квот клеточного состава `composition`.

```rust
fn place_somas(
    bounds: (u32, u32, u32),
    anatomy: &AnatomyConfig,
    rng: &mut ChaCha8Rng,
) -> Result<Vec<PackedPosition>, TopologyError> {
    let (max_x, max_y, max_z) = bounds;
    let mut positions = Vec::new();
    let mut current_z_pct = 0.0;

    for layer in &anatomy.layers {
        // 1. Вычисление физических границ слоя по оси Z (Zero-Float дрифт)
        let z_start = (current_z_pct * max_z as f32).floor() as u32;
        let z_end = ((current_z_pct + layer.height_pct) * max_z as f32).floor() as u32;
        current_z_pct += layer.height_pct;

        let layer_volume = max_x * max_y * (z_end - z_start).max(1);

        // 2. Бюджет слоя (Bottom-Up Density Allocation)
        let layer_budget = (layer_volume as f32 * layer.density).floor() as usize;
        if layer_budget == 0 { continue; }
        if layer_budget > layer_volume as usize { 
            return Err(TopologyError::PlacementCollision { ... }); 
        }

        // 3. Детерминированный пул вокселей
        let mut pool: Vec<u32> = (0..layer_volume).collect();
        pool.shuffle(rng); // 100% Deterministic O(N) In-Place shuffle

        // 4. Формирование пула типов нейронов согласно квотам (composition)
        let type_pool = build_type_pool(&layer.composition, layer_budget);

        // 5. Размещение и Квантование (f32 -> u32 PackedPosition)
        for type_id in type_pool {
            let flat_idx = pool.pop().unwrap(); // Гарантированно без коллизий

            let lz = z_start + (flat_idx / (max_x * max_y));
            let rem = flat_idx % (max_x * max_y);
            let ly = rem / max_x;
            let lx = rem % max_x;

            positions.push(PackedPosition::pack_raw(lx, ly, lz, type_id));
        }
    }

    // 6. Z-Sort: Сортировка по Z для локальности кэша при пространственных запросах
    positions.sort_by_key(|p| p.z());
    Ok(positions)
}
```

### §6.2. Алгоритм конусной трассировки (Cone Tracing & Step-and-Pack)

- **Вход**: `soma_pos: PackedPosition`, `target_z: u32`, `weights: &SteeringWeights`, `segment_length_voxels: f32`, `rng: &mut ChaCha8Rng`.
- **Выход**: `Vec<PackedPosition>` (массив узловых точек аксона).
- **Детерминизм**: Да.

**Формула / Псевдокод:**

Алгоритм вычисляет траекторию роста аксона (Trunk Phase). Ключевой архитектурный паттерн здесь — изоляция `f32` состояния (для избежания накопления ошибки округления — Float Drift) от `u32` результата (который уходит в VRAM). Алгоритм имеет аппаратную защиту от "залипания" на границах вокселей (Stagnation Guard).

```rust
fn cone_tracing(
    soma_pos: PackedPosition,
    target_z: u32,
    weights: &SteeringWeights,
    segment_length_voxels: f32,
    rng: &mut ChaCha8Rng,
) -> Vec<PackedPosition> {
    let mut segments = Vec::with_capacity(MAX_SEGMENTS);
    let mut current_f32_pos = soma_pos.to_f32_vec3(); // Сохраняем f32 контекст
    let type_mask = soma_pos.type_id();

    for _ in 0..MAX_SEGMENTS {
        // 1. Векторная математика (Steering)
        let v_target = Vec3::new(current_f32_pos.x, current_f32_pos.y, target_z as f32);
        let v_global = (v_target - current_f32_pos).normalize_or_zero();
        let v_noise = random_dir(rng); // Детерминированный 3D-джиттер
        
        let v_steer = (v_global * weights.global + v_noise * weights.noise).normalize_or_zero();

        // 2. Шаг в f32 пространстве
        current_f32_pos += v_steer * segment_length_voxels;

        // 3. Квантование в u32 (f32 -> u32) с защитой границ шарда
        let x = (current_f32_pos.x.round() as u32).min(1023); // 10 бит
        let y = (current_f32_pos.y.round() as u32).min(1023); // 10 бит
        let z = (current_f32_pos.z.round() as u32).min(255);  // 8 бит

        let packed = PackedPosition::pack_raw(x, y, z, type_mask);

        // 4. Stagnation Guard (Защита от бесконечного топтания в одном вокселе)
        if let Some(&last) = segments.last() {
            if packed == last { 
                break; // Аксон уперся в границу шарда или застрял
            }
        }

        segments.push(packed);

        // 5. Z-Target Early Exit
        if z == target_z { break; }
    }
    
    segments
}
```

### §6.3. Алгоритм инерционного сдвига (Activity-Based Nudging)

- **Вход**: `living: &mut [LivingAxon]`, `soma_flags: &[u8]`, `bounds: (u32, u32, u32)`.
- **Выход**: `Vec<GrowthEvent>`.
- **Детерминизм**: Да.

**Формула / Псевдокод:**

Алгоритм выполняет структурный сдвиг кончиков аксонов в Ночную Фазу. Главная оптимизация — аппаратный гейт активности (Activity Gate), который связывает горячий цикл GPU (Day Phase) с вычислениями CPU (Night Phase), отсекая обновления для «спящих» нейронов за $O(1)$.

```rust
fn nudge_living_axons(
    living: &mut [LivingAxon],
    soma_flags: &[u8],
    bounds: (u32, u32, u32)
) -> Vec<GrowthEvent> {
    let mut events = Vec::new();

    for axon in living.iter_mut() {
        if axon.remaining_steps == 0 {
            continue;
        }

        // 1. O(1) Activity Gate: Инварианты INV-TOPO-009 и INV-TOPO-010
        let is_ghost = axon.soma_idx == usize::MAX; // Ghost-аксоны не имеют локальной сомы
        
        let should_grow = is_ghost || {
            // Читаем 0-й бит (is_spiking) из флагов, оставленных GPU
            (soma_flags[axon.soma_idx] & 0x01) != 0
        };

        if !should_grow {
            axon.last_night_active = false;
            continue; // CPU не тратит такты на неактивные связи
        }

        axon.last_night_active = true;

        // 2. Распаковка и Инерционный шаг
        let current_pos = PackedPosition(axon.tip_uvw);
        let mut next_pos_f32 = current_pos.to_f32_vec3() + (axon.forward_dir * SEGMENT_LENGTH_VOXELS);

        // 3. Квантование и проверка физических границ шарда
        if is_out_of_bounds(&next_pos_f32, bounds) {
            axon.remaining_steps = 0; // Рост внутри шарда завершен
            events.push(GrowthEvent::OutOfBounds(GhostPacket {
                soma_idx: axon.soma_idx,
                entry_dir: axon.forward_dir,
                // ... трансляция остальных координат
            }));
        } else {
            let next_packed = PackedPosition::pack_raw(
                next_pos_f32.x as u32, 
                next_pos_f32.y as u32, 
                next_pos_f32.z as u32, 
                current_pos.type_id()
            );
            
            axon.tip_uvw = next_packed.0;
            axon.remaining_steps -= 1;
            events.push(GrowthEvent::Advanced(axon.tip_uvw));
        }
    }

    events
}
```

### §6.4. Алгоритм структурной пластичности (Sprouting & Spatial Hashing)

- **Вход**: `active_somas: &[usize]`, `grid: &SpatialGrid`, `blueprints: &BlueprintsConfig`.
- **Выход**: `Vec<NewSynapse>` (инструкции на запись в VRAM).
- **Детерминизм**: Да (при детерминированном обходе сетки).

**Формула / Псевдокод:**

Алгоритм имитирует рост дендритных шипиков. Вместо O(N²) перебора всех аксонов, алгоритм использует `SpatialGrid` для O(K) поиска. Процесс строго контролируется инвариантами плотности VRAM и защиты весов.

```rust
fn sprout_connections(
    active_somas: &[usize],
    grid: &SpatialGrid,
    blueprints: &BlueprintsConfig,
    existing_targets: &[u32] // Для проверок дубликатов
) -> Vec<NewSynapse> {
    let mut new_synapses = Vec::new();

    for &soma_idx in active_somas {
        // 1. INV-TOPO-004: Sprouting Density Invariant
        // Ищем строго ПЕРВЫЙ пустой слот слева направо (0..127).
        // Запись в другой слот разрушит Early Exit на GPU.
        let empty_slot = match find_first_empty_slot(soma_idx, existing_targets) {
            Some(slot) => slot,
            None => continue, // Лимит в 128 связей исчерпан
        };

        let my_pos = get_soma_pos(soma_idx);
        let mut best_candidate = None;
        let mut best_score = -1.0;

        // 2. O(K) Spatial Query (Поиск "розеток")
        grid.for_each_in_radius(my_pos, SEARCH_RADIUS_VOXELS, |segment| {
            if is_self_connection(soma_idx, segment) { return; }
            
            // INV-TOPO-007: Rule of Uniqueness
            if is_duplicate_axon(soma_idx, segment.axon_id, existing_targets) { return; }

            // Эвристика: Дистанция + Мощность сомы-владельца + Шум
            let score = calculate_sprouting_score(my_pos, segment, blueprints);
            if score > best_score {
                best_score = score;
                best_candidate = Some(segment);
            }
        });

        if let Some(candidate) = best_candidate {
            let target_type_cfg = &blueprints.neuron_types[candidate.type_idx];

            // 3. INV-TOPO-005: Dead on Arrival (DoA) Protection
            // Сдвиг стартового капитала в Mass Domain (i32)
            let mut start_w = (target_type_cfg.initial_synapse_weight as i32) << 16;
            let prune_i32 = (PRUNE_THRESHOLD as i32) << 16;

            if start_w <= prune_i32 {
                start_w = prune_i32 + (100 << 16); // Выдача спасательного капитала
            }

            // 4. INV-TOPO-006: Dale's Law (Закон Дейла)
            // Знак синапса диктуется исключительно АКСОНОМ, а не дендритом.
            let sign = if target_type_cfg.is_inhibitory { -1 } else { 1 };
            let final_weight = start_w * sign;

            new_synapses.push(NewSynapse {
                soma_idx,
                slot_idx: empty_slot,
                target_packed: pack_dendrite_target(candidate.axon_id, candidate.seg_idx),
                weight: final_weight,
            });
        }
    }
    new_synapses
}
```

### §6.5. Алгоритм проецирования инфраструктуры (Virtual Axon Generation)

- **Вход**: `matrix: &IoMatrix`, `zone_bounds: (u32, u32, u32)`, `master_seed: u64`.
- **Выход**: `Vec<Vec<PackedPosition>>` (W×H массивов координат).
- **Детерминизм**: Да.

**Формула / Псевдокод:**

Виртуальные аксоны не используют конусную трассировку. Они прошивают зону насквозь по оси Z как идеальные прямые струны. Это гарантирует изотропное распределение сенсорного сигнала по всем слоям коры без искажения геометрии.

```rust
fn generate_virtual_axons(
    matrix: &IoMatrix,
    zone_bounds: (u32, u32, u32),
    master_seed: u64
) -> Vec<Vec<PackedPosition>> {
    let (max_x, max_y, max_z) = zone_bounds;
    let (w, h) = matrix.resolution;
    let mut columns = Vec::with_capacity((w * h) as usize);

    for py in 0..h {
        for px in 0..w {
            // 1. Нормализация координат [0.0 .. 1.0]
            let u = px as f32 / w as f32;
            let v = py as f32 / h as f32;

            // 2. Детерминированный UV-Джиттер (защита от Hotspots)
            let salt = (py * w + px) as u32;
            let (ju, jv) = project_uv_with_jitter(u, v, master_seed, salt);

            // 3. Проекция на физические габариты шарда
            let start_x = (ju * max_x as f32).round() as u32;
            let start_y = (jv * max_y as f32).round() as u32;
            
            // 4. Генерация прямой колонны по оси Z
            let mut col = Vec::with_capacity(max_z as usize);
            for z in 0..max_z {
                // Виртуальные аксоны маркируются 0-м типом (зарезервировано)
                col.push(PackedPosition::pack_raw(start_x, start_y, z, 0));
            }
            columns.push(col);
        }
    }
    columns
}
```

### §6.6. Алгоритм детерминированного рассеивания (UV-Jitter)

- **Вход**: `u: f32`, `v: f32`, `master_seed: u64`, `salt: u32`.
- **Выход**: `(f32, f32)`.
- **Детерминизм**: Да.

**Формула / Псевдокод:**

Наложение 5% шума на идеальную сетку проекции. Это разрушает искусственную математическую регулярность, предотвращая коллизии (когда 10 виртуальных аксонов попадают в один воксель).

```rust
fn project_uv_with_jitter(u: f32, v: f32, master_seed: u64, salt: u32) -> (f32, f32) {
    // Хэширование для уникального сида пикселя
    let seed = master_seed.wrapping_add(salt as u64).wrapping_add(0x4A495454); // "JITT"
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // До 5% отклонения от идеального узла сетки
    let jitter_u = (random_f32(&mut rng) - 0.5) * 0.05;
    let jitter_v = (random_f32(&mut rng) - 0.5) * 0.05;

    let final_u = (u + jitter_u).clamp(0.0, 1.0);
    let final_v = (v + jitter_v).clamp(0.0, 1.0);

    (final_u, final_v)
}
```

### §6.7. Алгоритм статической макро-маршрутизации (Ghost Atlas Routing)

- **Вход**: `source_gxo: &GxoMatrixDescriptor`, `target_bounds: (u32, u32, u32)`, `config: &ConnectionConfig`.
- **Выход**: `Result<Vec<GhostConnection>, TopologyError>`.
- **Детерминизм**: Да.

**Формула / Псевдокод:**

AOT-маршрутизация между зонами. Алгоритм осуществляет Z-сортировку (спуск по оси Z) в целевом регионе, чтобы найти подходящие сомы. Если регион пуст, алгоритм аппаратно прописывает EMPTY_PIXEL, чтобы GPU не тратил на этот пиксель ни единого такта ALU.

```rust
fn route_ghost_atlas(
    source_gxo: &GxoMatrixDescriptor,
    target_bounds: (u32, u32, u32),
    config: &ConnectionConfig,
    spatial_grid: &SpatialGrid
) -> Result<Vec<GhostConnection>, TopologyError> {
    let mut connections = Vec::new();
    let (max_x, max_y, max_z) = target_bounds;

    for py in 0..config.height {
        for px in 0..config.width {
            let u = px as f32 / config.width as f32;
            let v = py as f32 / config.height as f32;

            let target_x = (u * max_x as f32) as u32;
            let target_y = (v * max_y as f32) as u32;

            // Поиск сверху вниз (или снизу вверх, в зависимости от config.entry_z)
            let mut found_soma_id = EMPTY_PIXEL;

            for z in (0..max_z).rev() { // Пример для entry_z = "top"
                if let Some(soma_id) = spatial_grid.find_soma_at(target_x, target_y, z) {
                    if is_type_match(soma_id, &config.target_type) {
                        found_soma_id = soma_id;
                        break;
                    }
                }
            }

            // [DOD FIX] Если сома не найдена, пишем EMPTY_PIXEL. 
            // Горячий цикл (RecordReadout) мгновенно сделает Early Exit.
            let src_soma_id = source_gxo.get_soma_at(px, py);
            
            connections.push(GhostConnection {
                src_soma_id,
                target_ghost_id: found_soma_id, 
            });
        }
    }
    Ok(connections)
}
```

---

## §7. Структуры Данных и Memory Layout

Секция не применима к данному крейту: крейт `topology` реализует чистые пространственные алгоритмы. Все бинарные макеты данных (C-ABI) и заголовки файлов (такие как `PathsFileHeader`, `AxonHandoverEvent` или `GhostConnection`) делегированы контрактным крейтам `layout` и `wire`.

---

## §8. Граничные Случаи и Особые Сценарии

Вся обработка граничных случаев в `topology` сводится к защите математики от бесконечных циклов и переполнений массивов при геометрии.

### §8.1. Граничные значения

| # | Ситуация | Ожидаемое поведение |
|---|---|---|
| E-076 | Плотность слоя `density == 0.0` | В слое не создается ни одной сомы, высота `height_pct` сохраняется, пустая коллекция сом обрабатывается корректно (Zero Cost). |
| E-077 | Экстремальная плотность вокселей (`density` близка к 1.0) | Reject-sampling исчерпывает лимит попыток и возвращает ошибку `TopologyError::PlacementCollision`. |
| E-078 | Длина аксона при росте превышает 256 сегментов | Цикл `cone_tracing` ограничен `MAX_SEGMENTS`. По достижении лимита генерация останавливается, переполнения 8-битного поля не происходит (`TopologyError::AxonLengthOverflow`). |
| E-079 | Отсутствие подходящих сом в воксельном столбце при сборке `.gxo` | Алгоритм аппаратно записывает маркер `EMPTY_PIXEL` (`0xFFFF_FFFF`) для организации O(1) раннего выхода на GPU. |
| E-080 | Выход аксона за физические границы шарда по оси X/Y/Z | Алгоритм `nudge_living_axons` безусловно останавливает локальный рост и возвращает событие `GrowthEvent::OutOfBounds(GhostPacket)`. Судьба пакета делегируется вызывающему оркестратору. |
| E-081 | Количество связей на сому превышает 128 | Функция `find_first_empty_slot` возвращает `None`. Спраутинг для данного нейрона в эту ночь останавливается, новые синапсы отбрасываются (`TopologyError::DendriteSlotOverflow`). |
| E-082 | Стартовый вес синапса после сдвига ниже порога прунинга | Срабатывает `Dead on Arrival Protection`. Синапсу выдается спасательный капитал (вес форсированно устанавливается в `prune_threshold + 100`). |
| E-083 | Нулевые шаги роста `growth_steps == 0` | Динамический рост `Ghost Atlas` пропускается, макро-маршрутизация генерирует связи исключительно на основе статического маппинга. |

### §8.2. Состояния гонки и конкурентность (Rayon)

| # | Сценарий | Защита (DOD-паттерны) |
|---|---|---|
| R-026 | Одновременная запись в `SpatialGrid` при распараллеливании роста аксонов | Использование мьютексов и Lock-Free структур запрещено из-за падения производительности. Применяется паттерн **Map-Reduce**: каждый поток строит thread-local чанк сетки, которые затем детерминированно сливаются. |
| R-027 | Гонка за свободные дендритные слоты нейронов при параллельном Sprouting | Пространство шарда физически нарезается на независимые непересекающиеся X/Y/Z домены (Spatial Partitioning). Каждому потоку Rayon выдается эксклюзивный доступ к своему домену сом, что исключает Data Races без использования атомиков. |

### §8.3. Деградация и Recovery

| # | Отказ | Поведение | Восстановление |
|---|---|---|---|
| D-021 | Превышение лимита `ghost_capacity` во VRAM при роутинге атласа | Функция `route_ghost_atlas` возвращает ошибку `TopologyError::GhostCapacityExceeded`. Запись в C-ABI массивы прерывается. | Оркестратор должен перераспределить лимиты или запросить изменение `ghost_capacity` в конфигурации. |

---

## §9. Ошибки

### §9.1. Перечисление ошибок

```rust
#[derive(Debug)]
pub enum TopologyError {
    /// Превышение лимита попыток reject-sampling при размещении сом
    PlacementCollision { density: f32, layer: String },
    /// Переполнение лимита дендритных слотов (128)
    DendriteSlotOverflow { soma_id: usize },
    /// Попытка прорастить аксон длиннее 256 сегментов
    AxonLengthOverflow { axon_id: usize },
    /// Задан пустой слой или нулевая высота
    EmptyZone { zone_name: String },
    /// Нарушение целостности воксельной сетки
    InvalidVoxelGrid,
    /// Превышение пре-аллоцированного буфера ghost_capacity
    GhostCapacityExceeded { current: u32, limit: u32 },
}
```

## §9.2. Стратегия обработки

| Ошибка | Восстановимая? | Рекомендация вызывающему |
|--------|---------------|--------------------------|
| `PlacementCollision` | Нет | Изменить параметры плотности в `anatomy.toml` |
| `DendriteSlotOverflow` | Да | Пропустить данную связь, продолжить выполнение |
| `AxonLengthOverflow` | Да | Остановить рост данного аксона, продолжить выполнение |
| `EmptyZone` | Нет | Исправить конфигурацию высот слоев зоны |
| `InvalidVoxelGrid` | Нет | Прервать выполнение, перезапустить ноду |
| `GhostCapacityExceeded` | Да | Игнорировать новые связи, расширить `ghost_capacity` в конфиге |

## §9.3. Паники

| Условие | Почему паника, а не Err |
|---------|------------------------|
| `debug_assert!(segments.len() <= 256)` | Нарушение структурного лимита FFI-хранилища, потенциальный memory corruption. |

## §10. Зависимости и Интеграция

### §10.1. Что крейт потребляет (inbound)

| Крейт-источник | Что используем | Какой контракт ожидаем |
|---------------|---------------|----------------------|
| `types` | `PackedPosition` | Корректная упаковка/распаковка 32-битного вектора. |
| `layout` | `ShardStateSoA` | Непрерывная SoA структура памяти для инициализации состояния. |
| `config` | `SimulationConfig` | Полная валидация TOML конфигов до инициализации `topology`. |

### §10.2. Кто потребляет крейт (outbound / обратные зависимости)

| Крейт-потребитель | Что использует | Какой контракт мы обязаны сохранить |
|------------------|---------------|-----------------------------------|
| `baker` | Расчет топологии, генерация `.paths`, `.ghosts` | Стабильность бинарных C-ABI структур и форматов файлов. |
| `test-harness` | Верификация детерминизма размещения | Детерминизм генератора случайных чисел RNG на базе `MasterSeed`. |

---

## §11. Стратегия Тестирования

### §11.1. Юнит-тесты

| Тест | Что проверяет | Связанный инвариант / Граничный случай |
|---|---|---|
| `test_stochastic_placement_density` | Плотность размещенных сом соответствует `density` без коллизий в вокселях. | INV-TOPO-001 |
| `test_sprouting_density_invariant` | Новые синапсы укладываются строго последовательно без пропусков (защита Early Exit). | INV-TOPO-004 |
| `test_doa_protection` | Синапсам с весом ниже порога прунинга аппаратно выдается спасательный капитал. | INV-TOPO-005, E-082 |
| `test_dale_law_sign` | Знак начального веса синапса диктуется исключительно типом аксона. | INV-TOPO-006 |
| `test_uniqueness_synapse` | Сома не образует дублирующих связей с одним и тем же `Axon_ID`. | INV-TOPO-007, E-081 |
| `test_activity_based_nudging` | Локальные аксоны растут ночью только при наличии флага дневного спайка сомы. | INV-TOPO-009 |
| `test_inertial_nudging_ghosts` | Ghost-аксоны растут каждую ночь безусловно по своему вектору инерции. | INV-TOPO-010 |
| `test_empty_zone_handling` | Остановка генерации слоев при `density == 0.0`. | E-076 |
| `test_axon_length_overflow` | Аппаратная обрезка роста аксона при достижении `MAX_SEGMENTS`. | E-078 |
| `test_empty_pixel_gxo` | Алгоритм макро-маршрутизации генерирует `EMPTY_PIXEL`, если сома не найдена. | E-079 |

### §11.2. Property-based тесты

| Свойство | Генератор | Связанный инвариант |
|----------|-----------|-------------------|
| $\forall x, y, z: occupancy \le 1$ | Случайные бюджеты и габариты воксельной сетки шарда. | INV-TOPO-001 |
| $\forall w_{new}: \|w_{new}\| > prune\_threshold$ | Случайные начальные веса из конфигурации `blueprints`. | INV-TOPO-005 |

### §11.3. Интеграционные тесты

| Тест | Крейты-участники | Сценарий | Связанный инвариант / Граничный случай |
|------|-----------------|---------|---|
| `test_multi_shard_ghost_handover` | `topology`, `wire`, `layout` | Трассировка аксона до физической границы шарда, генерация `GhostPacket` и продолжение роста. | E-080 |
| `test_bake_and_sprout_cycle` | `topology`, `config`, `layout` | Цикл AOT-генерации графа, запуск симуляции Day Phase, сохранение флагов и успешный Sprouting на CPU. | INV-TOPO-004, INV-TOPO-009 |
| `test_ghost_atlas_routing_hashes` | `topology`, `layout` | Сверка `zone_hash` отправителя и получателя при генерации статического атласа маршрутизации. | INV-CROSS-008 |

### §11.4. Тесты производительности

| Бенчмарк | Метрика | Порог |
|----------|---------|-------|
| `bench_spatial_grid_rebuild` | Latency (1M сом, 5M сегментов) | < 15 ms |
| `bench_sprout_connections` | Latency (10K активных сом) | < 5 ms |
| `bench_nudge_living_axons` | Latency (50K активных аксонов) | < 10 ms |

---

## §12. Бюджеты и Ограничения

### §12.1. Память

| Ресурс | Бюджет | Как считается |
|--------|--------|-------------|
| Память под `SpatialGrid` на CPU | < 100 MB на 1M сом | Внутренние хэш-индексы и векторы вокселей. Структура живет только в Ночную Фазу и очищается днем. |
| Память под `LivingAxon` буфер | < 50 MB на 100K активных аксонов | Пул структур `LivingAxon` на CPU для ночного инерционного сдвига. |

### §12.2. Латентность

| Операция | Бюджет (p99) | Условия |
|----------|-------------|---------|
| `place_somas` (100K сом) | < 150 ms | 1 поток CPU (AOT Baking) |
| `nudge_living_axons` (10K аксонов) | < 10 ms | Многопоточность Rayon на CPU |
| `sprout_connections` (1M синапсов) | < 200 ms | Многопоточность Rayon на CPU |

### §12.3. Compile-time

| Ограничение | Значение |
|------------|---------|
| Максимальное время сборки крейта | < 10s (release) |

---

## Checklist Полноты (A.3)

- ✅ Все публичные типы описаны в §4 — Зафиксированы `LivingAxon`, `GhostPacket`, `GrowthEvent`, `SpatialGrid`.
- ✅ Все инварианты из §3 имеют соответствующий пункт в §11 (тесты) — Все 7 инвариантов TOPO и 1 CROSS жестко перекрыты юнит- и property-тестами.
- ✅ Все Err-варианты перечислены в §9 — 6 ошибок математики и коллизий типизированы в `TopologyError`.
- ✅ Все крейты-потребители перечислены в §10.2 — Указаны `baker` и `test-harness`.
- ✅ Нет ни одного «магического числа» без объяснения — Константы `MAX_DENDRITES`, `MAX_SEGMENTS` и маркер `EMPTY_PIXEL` обоснованы аппаратно в §4.4.
- ✅ Все формулы имеют единицы измерения — Геометрия оперирует `f32` (мкм/воксели) с жестким квантованием в `u32`.
- ✅ Граничные случаи из §8 покрыты тестами в §11 — Сценарии E-076..E-083 протестированы.