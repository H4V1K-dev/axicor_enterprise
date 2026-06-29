# spec_types

> Версия спеки: 2.2  
> Дата: 2026-06-29  
> Статус: Approved (Architecture Pass 2)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| Название | `types` |
| Слой | Слой 0 — Примитивы (Foundational Vocabulary) |
| Тип | Library (`lib`) |
| no_std | Строго обязателен (`true`) |
| Описание | Фундаментальный целочисленный словарь и единый источник истины (Single Source of Truth) для движка `AxiEngine`. Крейт определяет атомарные целочисленные примитивы и newtype-обертки, упакованные битовые ABI-контракты, гибридную систему координат, кванты времени, детерминированное хеширование, stateless-генерацию псевдослучайных чисел в целочисленных регистрах и базовые константы лимитов/сентинелей. Крейт является на 100% целочисленным и не содержит бизнес-логики, float-алиасов, I/O и сетевых операций. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| — | Нет внутренних зависимостей | Крейт является абсолютным фундаментом (Слой 0) для всего workspace `AxiEngine`. |

### §2.2. Зависимые компоненты (outbound consumers)

Все остальные крейты системы (`physics`, `layout`, `config`, `wire`, `protocol`, `compute`, `runtime`, `baker`) обязаны импортировать базовые типы и ABI-контракты исключительно из `types`. Прямое повторное объявление или переопределение битовых масок, структур координат и упакованных типов в других крейтах **строжайше запрещено**.

### §2.3. Внешние зависимости

| Crate | Версия | Сфера использования и ограничения |
|---|---|---|
| `bytemuck` | `=1.25.0`, features=`["derive"]` | Runtime `no_std` и `no_alloc` совместим. Исполняемый код берет маркерные трейты `Pod` и `Zeroable` для zero-cost приведения сырых байтовых массивов. Опция `derive` является процедурным макросом этапа компиляции и не влечет рантайм-зависимостей. |
| `static_assertions` | `=1.1.0` *(Dev Dependency)* | Используется **исключительно** как dev-зависимость в юнит-тестах для compile-time проверок размеров и выравнивания типов (`const_assert_eq!`). В продакшн-бинарники не попадает. |

> **Замечание о хешировании**: Крейт **не использует** внешнюю зависимость `wyhash`. Все алгоритмы лавинного перемешивания битов (avalanche mixers) и FNV-1a хеширование реализованы inline с 0 внешних зависимостей.

### §2.4. Feature Flags

| Feature | Default | Что включает |
|---|---|---|
| `default` | `[]` | По умолчанию крейт собирается в строго изолированном `no_std` и `no_alloc` окружении. |
| `std` | `[]` | Опциональный флаг исключительно для запуска расширенных юнит-тестов и интеграции с тестовыми harness в среде разработки. |

### §2.5. Запрещенные операции и зависимости

В крейте `types` физически и архитектурно запрещены:
- Зависимость от `std` и `alloc` в продакшн-сборке (0 динамических аллокаций памяти).
- Вычисления с плавающей точкой (`f32`/`f64`). Крейт является на 100% целочисленным.
- Внешние библиотеки хеширования, UUID и валидации текстовых идентификаторов (зоны `config`/editor/baker).
- Системные вызовы времени и энтропии (`std::time`, `SystemTime`, `thread_rng`, `/dev/urandom`).
- Операции с файловой системой (`std::fs`), сетевым стеком (`std::net`) и парсингом TOML/JSON (`serde`).
- FFI-вызовы GPU-драйверов (NVIDIA CUDA, AMD ROCm/HIP) или рантайма.
- Использование ветвящихся операций, макросов `panic!`, `unwrap()`, `expect()` или `assert!` в продакшн-пути (все функции упаковки/распаковки обязаны быть O(1) с `debug_assert!` для проверки границ в debug-сборках).

---

## §3. Ownership Boundaries (Границы Владения)

Для исключения расползания источников истины и предотвращения размытия архитектурных слоев, данная спецификация жестко закрепляет зоны ответственности между крейтами workspace `AxiEngine`. Ни один крейт не имеет права дублировать функционал из зоны владения другого крейта.

| Крейт / Модуль | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`types`** (Слой 0) | **Примитивы, newtypes и битовые ABI-контракты**: `PackedPosition`, `PackedTarget`, `SomaFlags`, `MasterSeed`, типы координат (`VoxelCoord`), битовые маски и сдвиги, базовые константы лимитов и сентинелей (`AXON_SENTINEL`, `EMPTY_PIXEL`), детерминированное хеширование и целочисленный RNG. | Запрещены float-типы, UUID, SoA-структуры дампов (`ShardStateSoA`), лимиты рантайма, GPU-политики, сетевое фреймирование пакетов, TOML-схемы, формулы физики, FFI и `std`. |
| **`physics`** (Слой 0) | **Чистая математика и физика**: уравнения GLIF (потенциал, рефрактерность, AHP), GSOP (пластичность), Active Tail, правила затухания, Headroom Guard (`MAX_WEIGHT_LIMIT`), а также математические формулы деривации параметров (например, вычисление `v_seg` / `compute_v_seg`). | Запрещены повторное объявление битовых масок `types`, макеты памяти SoA (`layout`), менеджмент VRAM, сетевые протоколы и TOML. |
| **`layout`** (Слой 1) | **Физическая раскладка памяти**: SoA-структуры (`ShardStateSoA`), выравнивание под GPU-варп (`GPU_WARP_SIZE` = 32/64), размеры кэш-линий (64B), заголовки файлов бинарных дампов (`.state`, `.axons`, `.paths`) и SHM. | Запрещена математика GLIF/GSOP, бизнес-логика симуляции, сетевое пакетирование и парсинг TOML. |
| **`config`** (Слой 1) | **Схема конфигурации DSL (TOML)**: Rust-структуры Serde (`ModelConfig`, `DepartmentConfig`, `ShardConfig`), валидация параметров при парсинге, float-единицы (`Microns`, `Fraction`), UUID и текстовые слаги. Вычисляет и валидирует физические параметры на этапе загрузки, используя формулы из `physics`. | Запрещены декларации бинарных C-ABI структур `#[repr(C)]`, GPU-ядра и вычисления физики в горячем цикле рантайма. |
| **`wire`** (Слой 1) | **Сетевые DTO и бинарная сериализация**: структура заголовков UDP-пакетов (`SpikeBatchHeaderV2`, `ExternalIoHeader`), сериализация кадра и L7-фрагментация. | Запрещена битовая упаковка внутренних координат нейрона `PackedPosition`, физика и прямые GPU DMA операции. |
| **`protocol`** (Слой 5) | **Протокол межнодовой синхронизации**: управление состояниями эпох (Ring Epochs), консенсус времени, логика Amnesia Drop / Fast Forward и макро-маршрутизация. | Запрещены определение низкоуровневых байтовых DTO пакетов (определяются в `wire`) и манипуляция VRAM. |
| **`baker`** (AOT Compiler) | **Топологическая линковка и сборка**: AOT-генератор графа связей, трассировка путей аксонов. Применяет и валидирует физические формулы деривации из `physics`. | Запрещены мутация битовых контрактов `PackedPosition`/`PackedTarget` из `types` и исполнение горячего цикла симуляции. |
| **`compute`** (Слой 3) | **Оркестрация вычислителей**: исполнение ядер CUDA/HIP/CPU, менеджмент VRAM, вызовы DMA. | Запрещены повторное определение структур данных нейронов (используются `layout` и `types`) и доменная логика TOML. |
| **`runtime`** (Слой 6) | **Рантайм-оркестратор**: управление фазами Day/Night, синхронизация IPC SHM, жизненный цикл ноды. | Запрещены самостоятельное выполнение математики симуляции (делегируется `compute`/`physics`) и парсинг бинарных макетов. |

---

## §4. Фундаментальные Примитивы и Newtypes

Все базовые типы данных в `types` объявляются с явными атрибутами представления в памяти (`#[repr(transparent)]` для newtypes и `#[repr(C)]` для POD-структур) для обеспечения 100% совместимости с C-ABI и GPU compute-ядрами. Крейт является строго целочисленным; float-алиасы (`Microns`, `Fraction`) удалены из `types` и принадлежат верхним слоям (`config`/`topology`).

### §4.1. Список атомарных типов

```rust
/// Дискретный счетчик тиков симуляции (монотонное время)
pub type Tick = u64;

/// Мембранный потенциал сомы нейрона (в микровольтах, мкВ)
pub type Voltage = i32;

/// Синаптический вес в домене массы (Mass Domain)
/// ИНВАРИАНТ: Строго i32 для обеспечения знаковой математики Закона Дейла
pub type Weight = i32;

/// Положение головы распространяющегося сигнала (индекс сегмента аксона).
/// При неактивности содержит значение AXON_SENTINEL (0x80000000)
pub type AxonHead = u32;

/// Индекс сегмента внутри аксона (обобщенный внешний контейнер верхнего уровня).
/// ВНИМАНИЕ: Внутри PackedTarget смещение сегмента строго ограничено 8 битами (0..255).
pub type SegmentIndex = u32;

/// Идентификатор профиля (варианта) нейрона в шарде (0..15)
pub type VariantId = u8;

/// Дискретная координата воксельной сетки (0..1023)
pub type VoxelCoord = u32;

/// Ошибки валидации упакованных типов на границах системы (Checked Constructors / try_* methods).
/// Легковесный no_std / no_alloc enum без динамических аллокаций (без String / Vec).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeError {
    PositionOutOfBounds { x: u32, y: u32, z: u32, type_id: u8 },
    TargetOutOfBounds { axon_id: u32, segment_offset: u32 },
    CorruptTarget { raw: u32 },
}
```

### §4.2. Endian Policy (Политика Эндианности)

Спецификация эндианности разделена на три четких архитектурных уровня:

1. **Numeric Bit Semantics (Битовая семантика)**: Для типов-оберток (`PackedPosition`, `PackedTarget`, `SomaFlags`, `MasterSeed`) битовые сдвиги (`<<`, `>>`) и побитовые операции (`&`, `|`) определены математически над базовыми регистрами Rust/C (`u32`, `u64`, `u8`) и не зависят от порядка байтов хоста.
2. **Serialization / IO (Сериализация на диск и в сеть)**: Сериализация байтов в бинарные файлы состояния (`.state`, `.axons`, `.paths`), Shared Memory (SHM) и сетевые буферы строго зафиксирована в формате **Little-Endian (LE)**. Крейт `types` определяет порядок байтов сырых примитивов, но не отбирает у крейтов `wire`/`protocol` право владения сетевыми заголовками и фреймингом пакетов.
3. **In-Memory Zero-Copy FFI**: Прямое приведение сырых указателей (`bytemuck::cast_slice`) между RAM хоста и VRAM видеокарты/микроконтроллера опирается на то, что целевые поддерживаемые архитектуры (x86_64, aarch64, nvptx, amdgcn, esp32) являются Little-Endian. На Big-Endian платформах перед DMA/FFI операциями обязателен явный вызов вспомогательных функций конвертации (`to_le_bytes`/`from_le_bytes`).

---

## §5. Packed ABI Контракты

Крейт `types` является владельцем 4 ключевых упакованных структур данных, оперирующих на уровне битовых полей.

### §5.1. `PackedPosition` (4 байта)

Упакованная 3D-координата сомы нейрона и бинарный индекс его типа в рамках шарда. Упаковывается ровно в один 32-битный регистр `u32`.

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct PackedPosition(pub u32);
```

#### Битовый макет (`PackedPosition` Bit Layout)

```text
 31        28 27        20 19        10 9          0
+------------+------------+------------+------------+
| Type_ID(4b)|   Z (8b)   |   Y (10b)  |   X (10b)  |
+------------+------------+------------+------------+
```

| Поле | Диапазон Бит | Сдвиг (Shift) | Маска (Mask) | Макс. Значение | Описание |
|---|---|---|---|---|---|
| **X** | `[0..9]` | `0` | `0x3FF` (10b) | `1023` | Координата ширины воксельной сетки |
| **Y** | `[10..19]` | `10` | `0x3FF` (10b) | `1023` | Координата глубины воксельной сетки |
| **Z** | `[20..27]` | `20` | `0xFF` (8b) | `255` | Координата высоты воксельной сетки |
| **Type_ID** | `[28..31]` | `28` | `0xF` (4b) | `15` | Локальный индекс типа нейрона |

#### Dual API Pattern и Формулы Упаковки / Распаковки

Крейты workspace оперируют по шаблону **Dual API Pattern**:
- **Быстрые тотальные конструкторы (`new`, `pack`)**: Оптимизированы для горячего цикла и внутренних геометрических процедур. Являются O(1) `const fn` функциями без ветвлений в release-сборке, но содержат `debug_assert!` для мгновенного отлова выходов за границы в debug-сборке.
- **Проверяемые конструкторы (`try_new`, `try_pack`)**: Предназначены для внешних границ (`config`, `baker`, `topology`, I/O). Выполняют строгую проверку доменных лимитов и возвращают `Result`. Верхние слои обязаны валидировать входные данные до вызова быстрых конструкторов.
- **Предикаты валидации (`is_valid_coords`, `is_valid_target`)**: Чистые константные функции проверки диапазонов.

```rust
impl PackedPosition {
    #[inline(always)]
    pub const fn is_valid_coords(x: u32, y: u32, z: u32, type_id: u8) -> bool {
        x <= MAX_VOXEL_X && y <= MAX_VOXEL_Y && z <= MAX_VOXEL_Z && (type_id as u32) <= MAX_TYPE_ID
    }

    pub fn try_new(x: u32, y: u32, z: u32, type_id: u8) -> Result<Self, TypeError> {
        if Self::is_valid_coords(x, y, z, type_id) {
            Ok(Self::new(x, y, z, type_id))
        } else {
            Err(TypeError::PositionOutOfBounds { x, y, z, type_id })
        }
    }

    #[inline(always)]
    pub const fn new(x: u32, y: u32, z: u32, type_id: u8) -> Self {
        debug_assert!(x <= MAX_VOXEL_X, "X coordinate exceeds MAX_VOXEL_X");
        debug_assert!(y <= MAX_VOXEL_Y, "Y coordinate exceeds MAX_VOXEL_Y");
        debug_assert!(z <= MAX_VOXEL_Z, "Z coordinate exceeds MAX_VOXEL_Z");
        debug_assert!((type_id as u32) <= MAX_TYPE_ID, "Type_ID exceeds MAX_TYPE_ID");

        let x_q = x & 0x3FF;
        let y_q = y & 0x3FF;
        let z_q = z & 0xFF;
        let t_q = (type_id as u32) & 0xF;
        Self(x_q | (y_q << 10) | (z_q << 20) | (t_q << 28))
    }

    #[inline(always)]
    pub const fn x(&self) -> u16 { (self.0 & 0x3FF) as u16 }

    #[inline(always)]
    pub const fn y(&self) -> u16 { ((self.0 >> 10) & 0x3FF) as u16 }

    #[inline(always)]
    pub const fn z(&self) -> u8 { ((self.0 >> 20) & 0xFF) as u8 }

    #[inline(always)]
    pub const fn type_id(&self) -> u8 { ((self.0 >> 28) & 0xF) as u8 }
}
```

---

### §5.2. `PackedTarget` (4 байта)

Упакованный целевой адрес синаптического контакта дендрита. Связывает дендрит сомы с конкретным аксоном и смещением сегмента на нем. В отличие от legacy MVP (использовавшего простой псевдоним `pub type PackedTarget = u32`), в AxiEngine тип реализован как строгая `#[repr(transparent)]` newtype-структура.

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct PackedTarget(pub u32);
```

#### Битовый макет (`PackedTarget` Bit Layout)

```text
 31        24 23                                   0
+------------+---------------------------------------+
|SegOffset(8b|           Axon_ID + 1 (24b)           |
+------------+---------------------------------------+
```

| Поле | Диапазон Бит | Сдвиг (Shift) | Маска (Mask) | Макс. Значение | Описание |
|---|---|---|---|---|---|
| **Axon_ID + 1** | `[0..23]` | `0` | `0x00FFFFFF` (24b) | `16_777_213` (закодировано `16_777_214`) | Идентификатор целевого аксона со смещением +1 |
| **Segment_Offset** | `[24..31]` | `24` | `0xFF` (8b) | `255` | Индекс сегмента аксона в точке контакта |

#### Резервирование `MAX_AXON_ID`, Коллизии и Зарезервированные Коды

В архитектуре зафиксирован `MAX_AXON_ID = 16_777_213` (`0x00FF_FFFD`).
* **Причина резервирования верхнего значения**: Упаковка гипотетического аксона `16_777_214` со смещением `255` по формуле `pack(16_777_214, 255)` дала бы сырое значение `0xFFFF_FFFF`.
* Значение `0xFFFF_FFFF` зарезервировано в системе под константу **`EMPTY_PIXEL`** (Pruned Tombstone).
* **Зарезервированные и поврежденные битовые коды (Reserved / Corrupt Encodings)**: После уменьшения `MAX_AXON_ID` диапазон закодированного аксона (`axon_q = raw & 0x00FFFFFF`) для валидной живой связи составляет строго `1..=MAX_AXON_ID + 1` (`1..=16_777_214` / `0x00FF_FFFE`).
  * Значения вида `0x00FF_FFFF`, `0x01FF_FFFF`, `0xFEFF_FFFF` (где `axon_q == 0x00FF_FFFF`) не являются `EMPTY_PIXEL`, но и не являются валидной живой связью. Это зарезервированные/поврежденные битовые комбинации.
  * Значения вида `0x0100_0000`, `0x0200_0000` (где `axon_q == 0`, но смещение сегмента не ноль) также невалидны.

#### Изоляция Домена `EMPTY_PIXEL`
Константа `EMPTY_PIXEL` (`0xFFFF_FFFF`) имеет семантический смысл надгробия **исключительно внутри `PackedTarget`** и синаптических плоскостей.
Для других типов это значение абсолютно легально. Например, `PackedPosition::new(1023, 1023, 255, 15).0 == 0xFFFF_FFFF` является полностью допустимой координатой нейрона и не вызывает никакого конфликта, так как оперирует в отдельном типе и в другой плоскости памяти.

#### Предусловия Быстрых Конструкторов (`new` / `pack`)
Быстрые тотальные конструкторы (`new`, `pack`) имеют строгое предусловие (precondition): входные аргументы уже валидированы вышестоящим слоем (например, при парсинге TOML в `config` или при генерации в `baker` через `try_*` методы). В релизной сборке быстрые конструкторы гарантируют отсутствие Undefined Behavior (UB), но не обязаны «красиво» обрабатывать заведомо невалидные входы. В debug-сборке выходы за границы мгновенно отлавливаются через `debug_assert!`.

#### Семантика Таргета, Безопасная Распаковка и Защита от Underflow

Для `PackedTarget` определены следующие правила и методы:
- **`is_inactive(&self)`**: Оказывается строго тождественным `self.0 == 0 || self.0 == EMPTY_PIXEL`. Используется вычислительными ядрами (`compute-*`, `physics`) для сверхбыстрого O(1) аппаратного Early Exit.
- **`is_valid_raw(&self)`**: Проверяет, является ли сырое число `0` (zero-init None), `EMPTY_PIXEL` (tombstone) или валидным живым таргетом с `axon_q` в диапазоне `1..=MAX_AXON_ID + 1`.
- **`is_reserved_encoding(&self)`**: Возвращает `!self.is_valid_raw()`.
- **`unpack(&self)`**: Является тотальным и абсолютно безопасным методом. Возвращает `None` для любых неактивных, зарезервированных или битово поврежденных слотов. Гарантирует **полное отсутствие паник и underflow (`0 - 1`)** при любых сырых значениях `u32`.
- **`try_unpack(&self)`**: Предназначен для валидаторов и AOT-инструментов. Возвращает `Err(TypeError::CorruptTarget { raw: self.0 })` при обнаружении поврежденных битовых кодов.

```rust
impl PackedTarget {
    pub const NONE: Self = Self(0);
    pub const TOMBSTONE: Self = Self(EMPTY_PIXEL);

    #[inline(always)]
    pub const fn is_valid_target(axon_id: u32, segment_offset: u32) -> bool {
        axon_id <= MAX_AXON_ID && segment_offset <= MAX_SEGMENT_OFFSET
    }

    pub fn try_pack(axon_id: u32, segment_offset: u32) -> Result<Self, TypeError> {
        if Self::is_valid_target(axon_id, segment_offset) {
            Ok(Self::pack(axon_id, segment_offset))
        } else {
            Err(TypeError::TargetOutOfBounds { axon_id, segment_offset })
        }
    }

    #[inline(always)]
    pub const fn pack(axon_id: u32, segment_offset: u32) -> Self {
        debug_assert!(axon_id <= MAX_AXON_ID, "axon_id exceeds MAX_AXON_ID");
        debug_assert!(segment_offset <= MAX_SEGMENT_OFFSET, "segment_offset exceeds MAX_SEGMENT_OFFSET");

        let axon_q = axon_id.wrapping_add(1) & 0x00FFFFFF;
        let seg_q = (segment_offset & 0xFF) << 24;
        Self(axon_q | seg_q)
    }

    #[inline(always)]
    pub const fn is_zero_none(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn is_tombstone(&self) -> bool {
        self.0 == EMPTY_PIXEL
    }

    #[inline(always)]
    pub const fn is_inactive(&self) -> bool {
        self.0 == 0 || self.0 == EMPTY_PIXEL
    }

    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        !self.is_inactive()
    }

    #[inline(always)]
    pub const fn is_valid_raw(&self) -> bool {
        if self.is_inactive() {
            true
        } else {
            let axon_q = self.0 & 0x00FFFFFF;
            axon_q >= 1 && axon_q <= MAX_AXON_ID + 1
        }
    }

    #[inline(always)]
    pub const fn is_reserved_encoding(&self) -> bool {
        !self.is_valid_raw()
    }

    #[inline(always)]
    pub const fn unpack(&self) -> Option<(u32, u32)> {
        if self.is_inactive() {
            None
        } else {
            let axon_q = self.0 & 0x00FFFFFF;
            if axon_q == 0 || axon_q > MAX_AXON_ID + 1 {
                None
            } else {
                let axon_id = axon_q - 1;
                let segment_offset = (self.0 >> 24) & 0xFF;
                Some((axon_id, segment_offset))
            }
        }
    }

    pub fn try_unpack(&self) -> Result<Option<(u32, u32)>, TypeError> {
        if self.is_reserved_encoding() {
            Err(TypeError::CorruptTarget { raw: self.0 })
        } else {
            Ok(self.unpack())
        }
    }
}
```


---

### §5.3. `SomaFlags` (1 байт)

Компактный 8-битный флаг состояния сомы нейрона для горячего цикла симуляции и выравнивания в SoA-массивах.

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct SomaFlags(pub u8);
```

#### Битовый макет (`SomaFlags` Bit Layout)

```text
 7          4 3        1  0
+------------+----------+---+
| Type_ID(4b)|BurstCnt3b|Spk|
+------------+----------+---+
```

#### Битовые Константы, Маски и Сдвиги

```rust
pub const SOMA_SPIKING_MASK: u8 = 0x01;
pub const SOMA_SPIKING_SHIFT: u8 = 0;

pub const SOMA_BURST_MASK: u8   = 0x0E;
pub const SOMA_BURST_SHIFT: u8  = 1;

pub const SOMA_TYPE_MASK: u8    = 0xF0;
pub const SOMA_TYPE_SHIFT: u8   = 4;
```

#### Кэш-зеркало `type_id` и Правила Синхронизации

Поле `type_id` (биты `4..7`) в `SomaFlags` является **рантайм-зеркалом (Runtime Cache Mirror)** первичного `type_id` из статической геометрии `PackedPosition` (которая выступает как Single Source of Truth).
- **Зачем дублируется**: При обработке спайков ядро читает 1 байт `soma_flags` и мгновенно получает доступ к таблице параметров `VARIANT_LUT[type_id]` без дополнительного чтения 4 байт геометрии из `soma_positions`.
- **Разделение ответственности по синхронизации**:
  - `baker` и `layout` формируют согласованные исходные плоскости при сборке и дампе.
  - `boot` валидирует совпадение `SomaFlags.type_id` с `PackedPosition.type_id` при загрузке в оперативную память / VRAM.
  - `compute-api` фиксирует данный ABI-контракт в интерфейсах вычислителя.
  - Вычислительные ядра (`compute-cpu`, `compute-cuda`, `compute-hip`) при любых мутациях рантайма (обновление спайков/burst) **обязаны строго сохранять** маску `0xF0` (`SOMA_TYPE_MASK`).


#### Методы Доступа и Мутации (Accessors & Mutators)

```rust
impl SomaFlags {
    #[inline(always)]
    pub const fn new(spiking: bool, burst_count: u8, type_id: u8) -> Self {
        let spk = (spiking as u8) & SOMA_SPIKING_MASK;
        let burst = (burst_count.min(7) << SOMA_BURST_SHIFT) & SOMA_BURST_MASK;
        let typ = (type_id << SOMA_TYPE_SHIFT) & SOMA_TYPE_MASK;
        Self(spk | burst | typ)
    }

    #[inline(always)]
    pub const fn spiking(&self) -> bool {
        (self.0 & SOMA_SPIKING_MASK) != 0
    }

    #[inline(always)]
    pub const fn burst_count(&self) -> u8 {
        (self.0 & SOMA_BURST_MASK) >> SOMA_BURST_SHIFT
    }

    #[inline(always)]
    pub const fn type_id(&self) -> u8 {
        (self.0 & SOMA_TYPE_MASK) >> SOMA_TYPE_SHIFT
    }

    #[inline(always)]
    pub fn set_spiking(&mut self, spiking: bool) {
        if spiking {
            self.0 |= SOMA_SPIKING_MASK;
        } else {
            self.0 &= !SOMA_SPIKING_MASK;
        }
    }

    #[inline(always)]
    pub fn set_burst_count(&mut self, count: u8) {
        let clamped = count.min(7);
        self.0 = (self.0 & !SOMA_BURST_MASK) | ((clamped << SOMA_BURST_SHIFT) & SOMA_BURST_MASK);
    }
}
```

---

### §5.4. `MasterSeed` (8 байт)

Корневой 64-битный сид генератора псевдослучайных чисел сети.

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct MasterSeed(pub u64);
```

---

## §6. Детерминированное Хеширование и Семена

Крейт `types` гарантирует побитовую воспроизводимость симуляции на любом оборудовании за счет детерминированного хеширования без использования системной энтропии.

### §6.1. Преобразование Строки в Корневой Сид (`seed_from_str`)

Преобразует произвольную текстовую строку конфигурации (ASCII, UTF-8) в 64-битное число с использованием алгоритма FNV-1a 64-bit:

$$\text{hash}_{0} = \text{0xcbf29ce484222325}$$
$$\text{hash}_{i} = (\text{hash}_{i-1} \oplus \text{byte}_i) \times \text{0x00000100000001B3} \pmod{2^{64}}$$

*Тестовый контракт (Golden Vector)*:
- Строка `"AXICOR"` хэшируется строго в `0x0d7388e891ead1f9`.

### §6.2. Stateless Генерация Локальных Семян (`entity_seed`)

Вычисление уникального сида конкретной сущности (нейрона, аксона, синапса) за O(1) без сохранения состояния RNG-генератора:

$$\text{Local\_Seed} = \text{Mix}(\text{Master\_Seed} + \text{Entity\_ID} + \text{0x60bee2bee120fc15})$$

Где $\text{Mix}(s)$ — битовая лавинная перемешивающая функция (WyHash avalanche mixer, реализована inline):
1. $t = (\text{u128})s \times \text{0xa3b195354a39b70d}$
2. $m_1 = (t \gg 64) \oplus t$
3. $t_2 = (\text{u128})m_1 \times \text{0x1b03738712fad5c9}$
4. $\text{result} = (t_2 \gg 64) \oplus t_2$

*Тестовый контракт (Golden Vector)*:
- `entity_seed(MasterSeed(0), 1)` фиксируется строго как `0x0d603133dc4196d3`.


### §6.3. Хеширование Имен Протокола (`hash_name_fnv1a` / `fnv1a_32`)

Детерминированный 32-битный FNV-1a хэш для идентификации зон и матриц I/O в UDP-пакетах:

$$\text{hash}_{0} = \text{0x811c9dc5}$$
$$\text{hash}_{i} = (\text{hash}_{i-1} \oplus \text{byte}_i) \times \text{0x01000193} \pmod{2^{32}}$$

*Тестовый контракт*: Строка `"SensoryCortex"` хэшируется строго в `0x273fd103`.

### §6.4. Stateless Целочисленные RNG-примитивы (`random_u32` / `random_u64`)

Крейт `types` предоставляет исключительно целочисленные генераторы псевдослучайных бит без операций с плавающей точкой.

#### Математические Формулы
Вычисление 64-битного и 32-битного случайных чисел на базе корневого сида `MasterSeed(seed)` и аргумента `salt`:

$$\text{random\_u64}(\text{seed}, \text{salt}) = \text{Mix}(\text{seed} \text{.wrapping\_add}(\text{salt}) \text{.wrapping\_add}(\text{0x9e3779b97f4a7c15}))$$

$$\text{random\_u32}(\text{seed}, \text{salt}) = (\text{random\_u64}(\text{seed}, \text{salt}) \gg 32) \text{ as u32}$$

Где $\text{Mix}(s)$ — битовая лавинная перемешивающая функция WyHash avalanche mixer (§6.2), а константа `0x9e3779b97f4a7c15` служит постоянной золотого сечения (Golden Ratio Constant) для сжатия корреляций нулевых сидов.

#### Голден-Векторы (Golden Vectors Test Contract)
- `MasterSeed(0).random_u64(0)` фиксируется как `0xdfdf403e8fd5912b`.
- `MasterSeed(0).random_u32(0)` фиксируется как `0xdfdf403e`.
- `MasterSeed(0x123456789ABCDEF0).random_u32(42)` фиксируется в юнит-тестах `E-005`.

*Правило архитектуры*: Если в будущем вышестоящим слоям (`config`, `topology`, `baker`) потребуется представление в виде float $[0.0, 1.0)$, они реализуют функции преобразования (`u32_to_float_01`) на своем уровне. Крейт `types` остается на 100% целочисленным.

### §6.5. Владение Идентификаторами и Хешами (Hash/ID Ownership)

Крейт `types` владеет строго числовыми формами идентификаторов (seeds, integer hash helpers, packed numeric IDs).
- Человекочитаемые текстовые ID, UUID v4, валидация слагов и схемы парсинга имён категорически не входят в `types` и принадлежат крейту `config` или инструментарию `baker`/editor.
- Внешние зависимости от UUID-библиотек в `types` строго запрещены.

---

## §7. Константы и Аппаратные Лимиты

Таблица базовых констант, принадлежащих **исключительно** крейту `types`:

| Константа | Тип | Значение | Описание и Зона Владения |
|---|---|---|---|
| `AXON_SENTINEL` | `u32` | `0x80000000` | Маркер неактивности головы аксона. Владелец: `types`. |
| `EMPTY_PIXEL` | `u32` | `0xFFFF_FFFF` | Хард-маркер неактивного/обрезанного синаптического слота дендрита (Pruned Tombstone). Используется `baker`, `weaver-daemon` и GPU/MCU ядрами для Early Exit. Владелец: `types`. |
| `TARGET_AXON_MASK` | `u32` | `0x00FF_FFFF` | Битовая маска для извлечения Axon_ID (24 бита). Владелец: `types`. |
| `TARGET_SEG_SHIFT` | `u32` | `24` | Битовый сдвиг для извлечения Segment_Offset. Владелец: `types`. |
| `DEFAULT_MASTER_SEED` | `&str` | `"AXICOR"` | Дефолтная строка сида симуляции. Владелец: `types`. |
| `MAX_TYPE_ID` | `u8` | `15` | Аппаратный предел индексов типов нейронов (4 бита). Владелец: `types`. |
| `MAX_VOXEL_X` | `u32` | `1023` | Лимит координаты X в `PackedPosition` (10 бит). Владелец: `types`. |
| `MAX_VOXEL_Y` | `u32` | `1023` | Лимит координаты Y в `PackedPosition` (10 бит). Владелец: `types`. |
| `MAX_VOXEL_Z` | `u32` | `255` | Лимит координаты Z в `PackedPosition` (8 бит). Владелец: `types`. |
| `MAX_AXON_ID` | `u32` | `16_777_213` (`0x00FF_FFFD`) | Максимальный ID аксона с учетом сдвига +1 и резервирования EMPTY_PIXEL. Владелец: `types`. |
| `MAX_SEGMENT_OFFSET`| `u32` | `255` | Максимальное смещение сегмента в `PackedTarget` (8 бит). Владелец: `types`. |

#### Разграничение `SegmentIndex` и Внутренней Упаковки
- Внутри `PackedTarget` под смещение сегмента отводится строго 8 бит (`MAX_SEGMENT_OFFSET = 255`).
- Обобщенный псевдоним `SegmentIndex = u32` служит внешним контейнером верхнего уровня.
- Аксоны повышенной длины, превышающие 255 сегментов, требуют альтернативных архитектурных механизмов адресации в вышестоящих слоях, а не расширения текущей 32-битной упакованной структуры `PackedTarget`.

---

## §8. Инварианты

Крейт `types` гарантирует выполнение следующих 12 фундаментальных инвариантов:

- **INV-TYPES-001**: `size_of::<PackedPosition>() == 4` и `align_of::<PackedPosition>() == 4`.
- **INV-TYPES-002**: `size_of::<PackedTarget>() == 4` и `align_of::<PackedTarget>() == 4`.
- **INV-TYPES-003**: `size_of::<SomaFlags>() == 1` и `align_of::<SomaFlags>() == 1`.
- **INV-TYPES-004**: `size_of::<MasterSeed>() == 8` и `align_of::<MasterSeed>() == 8`.
- **INV-TYPES-005**: Все обертки newtypes обязаны иметь атрибут `#[repr(transparent)]` и реализовывать `bytemuck::Pod` и `Zeroable`.
- **INV-TYPES-006**: Бинарный `0` в `PackedTarget` обозначает `None` (Zero-Index Trap Protection).
- **INV-TYPES-007**: Превышение `axon_id > 16_777_213` при упаковке `PackedTarget` запрещено. Быстрые конструкторы (`new`, `pack`) имеют предусловие о предварительной валидации входов вышестоящими слоями и в debug-сборке содержат `debug_assert!`.
- **INV-TYPES-008**: `PackedTarget::unpack()` безопасен и тотален для любого raw `u32`. Гарантируется полное отсутствие паник и underflow (включая `0 - 1`) даже на поврежденных битовых кодах (`is_reserved_encoding()`). Функция `try_unpack()` возвращает `Err(TypeError::CorruptTarget { raw })` для невалидных кодов.
- **INV-TYPES-009**: В крейте запрещено использование `Result`, `panic!`, `unwrap` или `assert` в hot-path функциях `new`/`pack`. Проверка границ в них выполняется через `debug_assert!`.
- **INV-TYPES-010**: Запрещено использование системных источников энтропии (`std::time`, `thread_rng`) и внешних хеш-библиотек (`wyhash`).
- **INV-TYPES-011**: Константа `EMPTY_PIXEL` (`0xFFFF_FFFF`) имеет значение надгробия строго внутри `PackedTarget`. В других типах (например, `PackedPosition`) сырое значение `0xFFFF_FFFF` является полностью валидным и не вызывает конфликтов.
- **INV-TYPES-012**: Рантайм-сеттеры `SomaFlags` (`set_spiking`, `set_burst_count`) строго сохраняют маску типов `SOMA_TYPE_MASK` (`0xF0`).

---

## §9. Golden Tests / Required Test Matrix

Крейт `types` обязан быть покрыт стопроцентной матрицей автоматических тестов, разделенной на 11 обязательных тестовых сюит:

1. **Compile-Time Size, Alignment, Trait & Const Fn Asserts**:
   - Статические проверки `static_assertions::const_assert_eq!` для `size_of` и `align_of` всех 4 упакованных типов (`PackedPosition`, `PackedTarget`, `SomaFlags`, `MasterSeed`).
   - Проверки `static_assertions::assert_impl_all!` для реализации `Pod` и `Zeroable` всеми newtypes.
   - `const fn` smoke tests: подтверждение создания упакованных значений в `const` контексте (`const POS: PackedPosition = PackedPosition::new(1, 2, 3, 4);`, `const TGT: PackedTarget = PackedTarget::pack(10, 5);`).
2. **Pack / Unpack Roundtrip Tests**:
   - Полный цикл упаковки и распаковки `PackedPosition` и `PackedTarget` на случайно сгенерированных и синтетических валидных наборах данных.
3. **Boundary Values Tests**:
   - Проверка крайних значений: X=1023, Y=1023, Z=255, Type_ID=15 для `PackedPosition`.
   - Проверка Axon_ID=0 и Axon_ID=16_777_213, а также Segment_Offset=255 для `PackedTarget`.
   - Точные значения упаковки: `PackedTarget::pack(0, 0).0 == 1`, `PackedTarget::pack(MAX_AXON_ID, 255).0 == 0xFFFF_FFFE`.
   - Проверка `PackedPosition::new(1023, 1023, 255, 15).0 == 0xFFFF_FFFF` (подтверждение отсутствия конфликта с `PackedTarget`).
4. **`PackedTarget` Collision & Safety Tests**:
   - Подтверждение того, что `PackedTarget::pack(MAX_AXON_ID, MAX_SEGMENT_OFFSET).0 != EMPTY_PIXEL`.
   - Проверка работы `debug_assert!` в debug-сборках при передаче недопустимых значений в быстрые конструкторы `pack()` и `new()`.
5. **Bit Bleed Tests (`E-001`)**:
   - Валидация маскирования и проверок `try_new` / `try_pack` при выходе за пределы диапазонов.
   - Проверка `PackedTarget::try_pack(MAX_AXON_ID + 1, 0)`, возвращающего `Err(TypeError::TargetOutOfBounds { .. })`.
6. **Target States & Unpack Safety Tests (`E-002`)**:
   - Подтверждение того, что `PackedTarget(0).unpack()` и `PackedTarget(EMPTY_PIXEL).unpack()` возвращают `None`.
   - Подтверждение корректной работы инспекторов `is_zero_none()`, `is_tombstone()`, `is_inactive()`, `is_active()`, `is_valid_raw()`, `is_reserved_encoding()`.
   - Подтверждение `PackedTarget(EMPTY_PIXEL).is_tombstone() == true`.
   - Проверка зарезервированных кодов `PackedTarget(0x00FF_FFFF)` и `PackedTarget(0xFEFF_FFFF)`: `is_valid_raw() == false`, `unpack() == None`, `try_unpack() == Err(TypeError::CorruptTarget { raw })`.
   - Проверка защиты от underflow: `PackedTarget(0x0100_0000).unpack() == None` без паник и переполнений.
7. **SomaFlags Accessors & Saturating Clamp Tests (`E-003`)**:
   - Проверка работы методов `spiking()`, `burst_count()`, `type_id()`. Инкремент счетчика серий спайков сверх 7 и подтверждение фиксации на значении 7.
   - Подтверждение того, что рантайм-сеттеры `set_spiking()` и `set_burst_count()` строго сохраняют маску `SOMA_TYPE_MASK` (`0xF0`).
8. **Deterministic Seed & Hash Tests (`E-004`)**:
   - Проверка `hash_name_fnv1a(b"SensoryCortex") == 0x273fd103`.
   - Проверка детерминизма `seed_from_str` на пустой строке `""`, стандартной ASCII `"AXICOR"` (`0x0d7388e891ead1f9`) и UTF-8 мультибайтовой строке `"НейроСеть_42 🚀"`.
   - Проверка `entity_seed(MasterSeed(0), 1) == 0x0d603133dc4196d3`.
9. **Integer RNG Boundary Tests (`E-005`)**:
   - Проверка работы `random_u32` и `random_u64` на краевых семенах (`0`, `u64::MAX`, известные векторы) на полноту заполнения разрядной сетки.
   - Golden vectors: `MasterSeed(0).random_u64(0) == 0xdfdf403e8fd5912b`, `MasterSeed(0).random_u32(0) == 0xdfdf403e`, `MasterSeed(0x123456789ABCDEF0).random_u32(42)`.

10. **`no_std` Build & Forbidden Dependency Check**:
    - Автоматическая сборка крейта командой `cargo build --target thumbv7em-none-eabi --no-default-features` для проверки чистой `no_std` совместимости.
11. **Fixture Compatibility with Legacy Examples**:
    - Проверка бинарной совместимости упакованных координат с тестовыми фикстурами из legacy MVP `axicor-master`.
    
---

## §10. Resolved Architectural Decisions (Принятые Решения Pass 2)

Все открытые вопросы по крейту `types` закрыты на втором системном архитектурном проходе.

1. **[RESOLVED] Размещение float-типов `Microns` и `Fraction`**:
   - *Решение*: Float-типы `Microns` и `Fraction` полностью вынесены из `types` в крейт `config`/`topology`. Крейт `types` зафиксирован как 100% целочисленный ABI-фундамент.
2. **[RESOLVED] Семантика неактивных слотов и `EMPTY_PIXEL`**:
   - *Решение*: Зафиксированы 3 состояния: `0` (zero-init None), `EMPTY_PIXEL` (pruned tombstone) и `live target`. Введены инспекторы `is_zero_none()`, `is_tombstone()`, `is_inactive()`. Функция `unpack()` возвращает `None` для любых неактивных слотов (`is_inactive()`). Все compute-ядра оперируют проверкой `is_inactive()`.
3. **[RESOLVED] Резервирование `MAX_AXON_ID`**:
   - *Решение*: Лимит `MAX_AXON_ID` уменьшен до `16_777_213` (`0x00FF_FFFD`) во избежание коллизии `pack(16_777_214, 255) == EMPTY_PIXEL`.
4. **[RESOLVED] Dual API Pattern**:
   - *Решение*: Утвержден паттерн из тотальных O(1) конструкторов `new`/`pack` (с `debug_assert!`), проверяемых `try_new`/`try_pack` (с возвратом `Result`) и предикатов `is_valid_*`.
5. **[RESOLVED] Внешние зависимости и WyHash**:
   - *Решение*: Зафиксирован курс на 0 внешних хеш-зависимостей. Алгоритмы лавинного перемешивания реализованы inline. Зависимость `bytemuck = "=1.25.0"` зафиксирована в prod, `static_assertions` — только dev.
6. **[RESOLVED] Распиновка и синхронизация `SomaFlags`**:
   - *Решение*: Утверждена распиновка `0:spiking, 1..3:burst_count, 4..7:type_id`. Поле `type_id` в `SomaFlags` зафиксировано как рантайм-кэш зеркало для `PackedPosition.type_id` (SSOT). `baker`/`layout` формируют плоскости, `boot` валидирует при загрузке, `compute-api` фиксирует контракт, а вычислительные ядра сохраняют маску `SOMA_TYPE_MASK` (`0xF0`).

7. **[RESOLVED] Хеширование и float RNG**:
   - *Решение*: `random_f32` удален из `types`. Крейт предоставляет только целочисленные генераторы `random_u32`/`random_u64`. Текстовые ID и UUID вынесены в `config`/baker.

---

## Changelog

| Дата | Версия | Изменение |
|---|---|---|
| 2026-06-29 | 2.2 | **Утверждение архитектуры types**: Крейт переведен на 100% целочисленный фундамент. Удалены `Microns`, `Fraction`, `random_f32`. Уменьшен `MAX_AXON_ID` до `16_777_213` для исключения коллизий с `EMPTY_PIXEL`. Зафиксирован Dual API Pattern (`try_pack`/`try_new`), три состояния `PackedTarget` и инспекторы `is_inactive()`. Зафиксирована роль `SomaFlags.type_id` как кэш-зеркала. Разграничены зоны владения хешами и UUID. Закрыты все Open Questions. Статус обновлен до Approved. |
| 2026-06-29 | 2.1 | **Второй системный проход**: Уточнены границы владения. Добавлена константа `EMPTY_PIXEL` и разграничена её семантика с сырым `0`. Уточнен `PackedTarget`. Добавлены битовые маски и акцессоры в `SomaFlags`. Документ сохранен в статусе Draft. |
| 2026-06-29 | 2.0 | Полное переосмысление спецификации L0-крейта `types` для workspace `AxiEngine`. |
| 2026-05-27 | 1.0 | Первоначальная версия спецификации `types`. |
