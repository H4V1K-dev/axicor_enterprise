# spec_protocol

> Версия спеки: 2.0  
> Дата: 2026-06-29  
> Статус: Draft (Architecture Pass 1)

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `protocol` |
| **Слой** | Слой 5 — Network Stack (`L5`) |
| **Тип** | Library (`lib`) |
| **no_std** | Да (`true`) — строго обязательно для работы в Zero-Allocation режиме на гетерогенных и встраиваемых платформах |
| **Описание** | Stateless-парсер, движок L7-фрагментации и сборки спайковых батчей, а также математический фильтр валидации биологического времени (эпох). Крейт работает поверх `wire`, определяя правила уровня L7 над байтовыми слайсами. Крейт не открывает сокеты, не отправляет пакеты, не управляет таблицами маршрутизации, не ведет RCU-таблицы, не использует асинхронные рантаймы, не работает с SHM и не вызывает функции рантайма, вычислений или сетевых транспортов. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | `Tick` и базовые временные типы | Валидация временных отсечек и математика эпох. |
| `wire` (Слой 1) | `SpikeBatchHeaderV2`, `SpikeEventV2`, `ExternalIoHeader`, `RouteUpdate`, `ControlPacket`, `TelemetryFrameHeader`, `WireError`, безопасные хелперы `try_read_header`, `payload_slice` | Использование C-ABI макетов сетевых пакетов, безопасная проверка длин и разбор заголовков. |

### §2.2. Зависимые Компоненты (outbound consumers)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `transport` (Слой 5) | Передает сырые принятые байты в `protocol` для классификации и нарезает исходящие батчи через итератор фрагментации. |
| `net` (Слой 5) | Получает собранные батчи и типизированные вердикты эпох `EpochAction` для управления кластерным барьером и маршрутизацией. |

### §2.3. Внешние Зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `bytemuck` | `=1.25.0` | Безопасный нуль-копийный каст выровненных слайсов хоста в C-ABI структуры (на входящих сырых сетевых байтах прямой каст без проверки выравнивания запрещен). |

> [!IMPORTANT]
> Настоящая спецификация категорически запрещает использование стандартной библиотеки (`std`), аллокатора кучи (`alloc`, `Vec`, `Box`, `String`), динамических коллекций (`HashMap`), а также асинхронных и сетевых библиотек (`tokio`, `crossbeam`, `socket2`, `mio`). Крейт собирается строго под тройку `no_std`.

### §2.4. Feature Flags

Секция публичных feature flags не используется. Крейт собирается как единая `no_std` библиотека.

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`protocol`** (Слой 5) | **Семантика уровня L7 и Математика Эпох**: Валидация и классификация пакетов, математика L7-фрагментации спайков, состояние и буфер сборки чанков (`ReassemblyBuffer`), вердикт валидации эпохи (`EpochAction`) и типизированные L7-ошибки (`ProtocolError`). | Запрещены открытие сокетов и I/O вызовы ОС (владелец `transport`), управление таблицами маршрутизации и кластерными барьерами (владелец `net`), определение бинарных DTO, magic-константы и C-структур (владелец `wire`), а также перевод состояний исполнения ноды (владелец `runtime`). |
| **`wire`** (Слой 1) | **Бинарные Контракты Пакетов**: C-ABI макеты полей, magic-константы, Little-Endian политика и проверки физических размеров структур. | Запрещено хранение состояния сборки фрагментов и математика валидации эпох. |
| **`transport`** (Слой 5) | **Сетевой Транспорт и Сокеты**: Системные вызовы ОС, отправка/прием UDP/TCP датограмм, управление сокетами. | Запрещен анализ биологической семантики содержимого пакетов. |
| **`net`** (Слой 5) | **Оркестрация Кластера**: Маршрутизация пакетов между нодами, BSP-барьеры синхронизации и реакция на вердикт `EpochAction`. | Запрещена самостоятельная нарезка байтовых слайсов без участия `protocol`. |

---

## §4. Публичная API-Модель (Public API Model)

Публичный интерфейс разделяет представления исходящих фрагментов хоста и входящих сырых сетевых датограмм:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketKind {
    SpikeBatch,
    ExternalIo,
    RouteUpdate,
    Control,
    Telemetry,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochAction {
    Accept,
    DropPast,
    HoldFuture,
    FastForwardRequired { target_epoch: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Wire(WireError),
    InvalidMtu,
    PacketTooSmall,
    InvalidPacketKind,
    InvalidSpecialMode,
    PayloadNotMultipleOfSpikeEvent,
    FragmentIndexOutOfBounds,
    TooManyFragments,
    DuplicateFragment,
    MissingFragment,
    ReassemblyCapacityExceeded,
    IntegerOverflow,
}

pub struct FragmentSpec {
    pub max_packet_bytes: usize, // Полный размер L7-датограммы (включая SpikeBatchHeaderV2)
}

/// Исходящий фрагмент хоста с типизированным выровненным слайсом спайков
pub struct OutgoingSpikeFragment<'a> {
    pub header: SpikeBatchHeaderV2,
    pub events: &'a [SpikeEventV2],
}

/// Входящий фрагмент из сети с сырыми байтами невыровненной полезной нагрузки
pub struct ParsedSpikeFragment<'a> {
    pub header: SpikeBatchHeaderV2,
    pub payload_bytes: &'a [u8],
}

pub struct SpikeFragmentIterator<'a> {
    // zero-allocation итератор нарезки исходящих спайков
}

pub struct ReassemblyBuffer<'a> {
    // буфер сборки на базе выровненного хранилища полезной нагрузки и битовой маски полученных чанков
}
```

---

## §5. Правила Фрагментации и Сборки (Fragmentation & Reassembly Rules)

### §5.1. Правила Нарезки Спайковых Батчей (Fragmentation)
1. **Расчет Границ Слайса**: Поле `max_packet_bytes` задает предельный размер физической L7-датограммы, **включая** заголовок `SpikeBatchHeaderV2`.
   $$\text{header\_size} = \text{size\_of}::<SpikeBatchHeaderV2>(), \qquad \text{event\_size} = \text{size\_of}::<SpikeEventV2>() = 8$$
   $$\text{max\_events\_per\_chunk} = \frac{\text{max\_packet\_bytes} - \text{header\_size}}{\text{event\_size}}$$
2. **Проверка Валидности MTU**: Если `max_packet_bytes <= header_size` или `max_events_per_chunk == 0`, возвращается ошибка `ProtocolError::InvalidMtu`.
3. **Расчет Количества Чанков**: Если `event_count == 0`, исходящая фрагментация генерирует служебный Heartbeat-пакет (`chunk_idx = 0`, `total_chunks = 0`, полезная нагрузка отсутствует). Для `event_count > 0` расчет выполняется безопасно:
   $$\text{total\_chunks\_usize} = \left\lceil \frac{\text{event\_count}}{\text{max\_events\_per\_chunk}} \right\rceil$$
   Если `total_chunks_usize > u16::MAX as usize`, возвращается ошибка `ProtocolError::TooManyFragments`.
4. **Нормальные и Служебные Чанки**:
   - Нормальный чанк требует `total_chunks > 0` и `chunk_idx < total_chunks`. Любой обычный чанк с `chunk_idx >= total_chunks` возвращает `ProtocolError::FragmentIndexOutOfBounds`.
   - **Heartbeat-пакет**: `chunk_idx == 0` и `total_chunks == 0` при `payload_bytes.is_empty()`.
   - **ACK-пакет**: `chunk_idx == 0xFFFF` и `total_chunks == 0` при `payload_bytes.is_empty()`.
   - Любая другая комбинация с `total_chunks == 0` возвращает `InvalidSpecialMode`.

### §5.2. Правила Сборки (Reassembly)
1. **Zero Allocation Storage**: Хранилище `ReassemblyBuffer` предоставляется вызывающим кодом и состоит из двух фиксированных плоских частей: буфера полезной нагрузки `payload_buffer: &mut [u8]` и битового трекера полученных слотов `seen_chunks: &mut [u8]`.
2. **Идентификация Батча в v2**: Ключ сборки определяется триплетом `(src_zone_hash, dst_zone_hash, epoch)`. На один маршрут за одну эпоху передается строго один спайковый батч (отсутствие `batch_id` вынесено в review debt, §11).
3. **Строгая Политика Дубликатов (Strict Duplicate Policy)**: Повторный приём чанка с идентичным `chunk_idx` для одного и того же ключа сборки строго возвращает ошибку `ProtocolError::DuplicateFragment`.
4. **Сборка по Порядку Индексов (Index Ordering)**: Чанки принимаются вне порядка (Out-of-order) и копируются строго по смещению `chunk_idx * max_events_per_chunk * 8`. Состояние готовности (`Ready`) наступает только после получения всех чанков от `0` до `total_chunks - 1`. Собраная полезная нагрузка отдается строго в порядке возрастания `chunk_idx`.

---

## §6. Классификация Пакетов и Математика Эпох (Classification & Epoch Math)

### §6.1. Классификация Пакетов (Packet Classification)
1. **Диспетчеризация по Magic**: Семейства пакетов (`ExternalIo`, `RouteUpdate`, `Control`, `Telemetry`) определяются по magic-константам крейта `wire`.
2. **Ограничение SpikeBatchHeaderV2**: Заголовок `SpikeBatchHeaderV2` не содержит собственного magic-числа. Классификация спайковых батчей выполняется строго на базе контекста маршрута/канала, передаваемого из `net`/`transport`.

### §6.2. Валидация Биологического Времени Без Переполнений (Epoch Math)
Валидация сравнивает эпоху пакета `packet_epoch` с текущей эпохой ноды `local_epoch` (тип `u32`) и возвращает вердикт `EpochAction`. Для предотвращения паник при переполнении используется wrapping-арифметика `local_epoch.wrapping_add(1)`:
- **`EpochAction::Accept`**: `packet_epoch == local_epoch`.
- **`EpochAction::DropPast`**: `packet_epoch < local_epoch` (с учетом wrapping-потенциала).
- **`EpochAction::HoldFuture`**: `packet_epoch == local_epoch.wrapping_add(1)`.
- **`EpochAction::FastForwardRequired { target_epoch }`**: пакет пришел из далекого будущего (`packet_epoch > local_epoch.wrapping_add(1)`).

> [!IMPORTANT]
> Крейт `protocol` не мутирует состояние ноды и не сбрасывает рантайм. Использование `SystemTime` категорически запрещено.

---

## §7. Иерархия Ошибок Протокола (`ProtocolError`)

Ошибки семантического уровня L7 выражаются строго через тип `ProtocolError`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Wire(WireError),
    InvalidMtu,
    PacketTooSmall,
    InvalidPacketKind,
    InvalidSpecialMode,
    PayloadNotMultipleOfSpikeEvent,
    FragmentIndexOutOfBounds,
    TooManyFragments,
    DuplicateFragment,
    MissingFragment,
    ReassemblyCapacityExceeded,
    IntegerOverflow,
}
```

---

## §8. Требуемые Инварианты

- **INV-PROTO-001**: Нулевая аллокация кучи (`no_std`, 0 аллокаций памяти во всех режимах работы).
- **INV-PROTO-002**: Полное отсутствие зависимостей от системных сокетов, сетевых транспортов и ОС I/O.
- **INV-PROTO-003**: Крейт не содержит таблиц маршрутизации и не хранит глобальное состояние кластера.
- **INV-PROTO-004**: Полное отсутствие дублирования C-ABI структур пакетов из крейта `wire`.
- **INV-PROTO-005**: Обязательная проверка физической длины байтового слайса перед любым доступом к полям пакета.
- **INV-PROTO-006**: Абсолютная гарантия отсутствия паник (`panic!`) при обработке поврежденных сетевых байтов или переполнении счетчиков.
- **INV-PROTO-007**: Все неизвестные или невалидные пакеты переводятся в типизированную ошибку `ProtocolError` или вердикт отбрасывания.
- **INV-PROTO-008**: Математика фрагментации строго проверяет границы overflow через `checked_*` операторы.
- **INV-PROTO-009**: Крейт `protocol` никогда не выполняет `bytemuck` каст сырых входящих сетевых байтов полезной нагрузки в `&[SpikeEventV2]` без предварительной проверки выравнивания по адресу (чтение входящих событий выполняется через безопасный невыровненный итератор).

---

## §9. Golden Tests / Обязательная Матрица Тестирования

Крейт `protocol` обязан быть покрыт набором юнитов и интеграционных тестов:

1. **Компиляция в no_std (`test_protocol_no_std_build`)**: Проверка успешной сборки без стандартной библиотеки.
2. **Изоляция Зависимостей (`test_protocol_no_forbidden_dependencies`)**: Проверка отсутствия сетевых и системных крейтов в графе компиляции.
3. **Соблюдение Лимитов MTU Итератором (`test_fragment_iterator_respects_mtu`)**: Верификация того, что размер сгенерированных чанков не превышает `max_packet_bytes`.
4. **Браковка Малого MTU (`test_fragment_iterator_rejects_too_small_mtu`)**: Возврат `InvalidMtu`, если MTU меньше размера заголовка.
5. **Защита от Переполнения при Расчете Чанков (`test_fragment_total_chunks_checked_overflow`)**: Проверка безопасной арифметики количества фрагментов.
6. **Браковка Количество Чанков Свыше u16::MAX (`test_total_chunks_over_u16_rejected`)**: Возврат `TooManyFragments` при превышении лимита `u16`.
7. **Проверка Кратности Полезной Нагрузки Спайкам (`test_spike_payload_len_multiple_of_event_size`)**: Валидация размера полезной нагрузки на кратность `SpikeEventV2`.
8. **Генерация Heartbeat при Пустом Батче (`test_empty_batch_generates_heartbeat`)**: Верификация формирования пакета Heartbeat при `event_count == 0`.
9. **Отсутствие Полезной Нагрузки у ACK (`test_ack_packet_has_no_payload`)**: Проверка структуры пакета ACK.
10. **Отбраковка Невалидных Специальных Режимов (`test_invalid_special_modes_rejected`)**: Проверка возврата `InvalidSpecialMode` при некорректных индексах.
11. **Прием Чанков Вне Порядка при Сборке (`test_reassembly_accepts_out_of_order_chunks`)**: Верификация успешной сборки батча при произвольном порядке прибытия чанков.
12. **Сборка Полезной Нагрузки Строго по Индексам Чанков (`test_reassembly_orders_chunks_by_index`)**: Проверка детерминированного порядка конкатенации событий независимо от порядка прибытия.
13. **Строгая Браковка Дубликатов Чанков (`test_reassembly_detects_duplicate_chunk`)**: Возврат ошибки `DuplicateFragment` при повторной передаче одного чанка.
14. **Безопасное Чтение Невыровненной Полезной Нагрузки (`test_inbound_payload_unaligned_safe`)**: Чтение входящих байтов без паник и unaligned кастов.
15. **Контроль Емкости Буфера Сборки (`test_reassembly_capacity_limit`)**: Возврат `ReassemblyCapacityExceeded` при превышении размера буфера.
16. **Вердикт Accept для Текущей Эпохи (`test_epoch_action_accept_current`)**: Верификация совпадения эпох.
17. **Вердикт DropPast для Устаревшей Эпохи (`test_epoch_action_drop_past`)**: Верификация отбрасывания пакетов из прошлого.
18. **Вердикты HoldFuture и FastForward для Будущих Эпох (`test_epoch_action_hold_or_fast_forward_future`)**: Верификация удержания и сдвига времени.
19. **Гарантия Отсутствия Паник на Мусорных Байтах (`test_malformed_packet_never_panics`)**: Проверка устойчивости к произвольным битым байтовым массивам.

---

## §10. Open Questions / Review Debt (Открытые Вопросы и Противоречия)

1. **Проектирование Заголовка `SpikeBatchHeaderV3`**:
   - *Контекст*: Заголовок v2 не имеет magic-числа, `batch_id` и счетчика событий.
   - *Вопрос*: Какие поля (magic, version, batch_id, total_event_count, checksum) должны войти в следующую версию заголовка спайков в `wire`?

2. **Согласование Разрядности Эпох (`u32` vs `u64`)**:
   - *Контекст*: Сетевые заголовки передают эпоху как `u32`, а внутренний `Tick` в `types` имеет разрядность `u64`.
   - *Вопрос*: Какова официальная политика переполнения и экранирования 32-битного сетевого счетчика эпох?

3. **Обобщение L7-Фрагментации на Другие Типы Пакетов**:
   - *Контекст*: В текущей версии фрагментация реализована строго для спайковых батчей.
   - *Вопрос*: Требуется ли поддержка L7-фрагментации для внешнего ввода-вывода (`ExternalIo`) и телеметрии?

4. **Владение Емкостью Буферов Сборки (Reassembly Capacity)**:
   - *Контекст*: `protocol` принимает внешние буферы сборки.
   - *Вопрос*: Должны ли лимиты емкости задаваться профилем маршрута в `net` или параметрами сокета в `transport`?

5. **Семантика Повторов и Подтверждений (ACK Semantics)**:
   - *Контекст*: `protocol` определяет бинарную структуру ACK-пакета.
   - *Вопрос*: Как именно распределяются обязанности по обработке таймаутов и повторов между `protocol` и `transport`?

6. **Поддержка Архитектур с Big-Endian Порядком Байт**:
   - *Контекст*: Все контракты зафиксированы в Little-Endian.
   - *Вопрос*: Требуется ли явный конвертер байт для редких Big-Endian устройств или Little-Endian зафиксирован как аппаратный стандарт?

7. **Локализация Проверки Целостности и Аутентификации**:
   - *Контекст*: Пакеты валидируются на структурный размер.
   - *Вопрос*: В каком крейте (`wire`, `protocol` или будущем модуле безопасности) должны проверяться криптографические подписи или CRC32/CRC64?

8. **Источник Точного Значения MTU**:
   - *Контекст*: Итератор фрагментации принимает структуру `FragmentSpec`.
   - *Вопрос*: Кто выступает источником MTU — профиль маршрута в `net` или параметры адаптера в `transport`?
