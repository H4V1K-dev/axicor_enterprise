# AxiEngine — Единый Рабочий Реестр Замечаний, Вопросов и Архитектурного Долга (`review.md`)

> Версия: 2.0 | Дата: 2026-06-29  
> Данный документ является единым рабочим реестром вопросов и архитектурного долга, а не местом принятия финальных решений. Принятие решений происходит в процессе архитектурного ревью с последующим внесением изменений в целевые спецификации.

---

## §1. P0 Blockers (Критические Блокеры)
*Критические расхождения и противоречия, без разрешения которых нельзя начинать кодинг базовых крейтов.*

### REV-COMPUTE-API-001: Несоответствие имен методов аллокации/деаллокации VRAM в Layer 3
- **ID**: REV-COMPUTE-API-001
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P0
- **Owner candidate**: `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.2) vs [compute_spec.md](./spec_L3/compute_spec.md) (§9.1)
- **Question / Problem**: Трейт `ComputeBackend` в `compute-api` и фасад `ShardEngine` использовали разный набор имен.
- **Why it matters**: Разнобой в названиях мешает согласованной разработке API.
- **Affected specs**: [compute_api_spec.md](./spec_L3/compute_api_spec.md), [compute_spec.md](./spec_L3/compute_spec.md), [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: Имена методов стандартизированы по всему L3 слою: `alloc_shard`, `upload_shard`, `run_day_batch`, `free_shard`, `teardown`.

### REV-TYPES-001: Коллизия маркерного таргета `EMPTY_PIXEL` и сырого `0` при Early Exit
- **ID**: REV-TYPES-001
- **Status**: Resolved
- **Priority**: P0
- **Owner candidate**: `types`
- **Source**: [types_spec.md](./spec_L0/types_spec.md#L211) (§5.2) vs [physics_spec.md](./spec_L0/physics_spec.md) vs [compute_api_spec.md](./spec_L3/compute_api_spec.md)
- **Question / Problem**: `types` определяет `EMPTY_PIXEL = 0xFFFF_FFFF` в качестве tombstone-маркера для обрезанных синапсов, а сырой `0` — как неинициализированный target. Но ядра `physics` и бэкенды `compute-cpu/cuda/hip` проверяли только `target == 0` для аппаратного Early Exit.
- **Why it matters**: Ядра GPU/CPU будут выполнять бессмысленные математические операции над отброшенными синапсами со значением `EMPTY_PIXEL`, что приведет к фатальному падению производительности или расхождениям в симуляции.
- **Affected specs**: [types_spec.md](./spec_L0/types_spec.md), [physics_spec.md](./spec_L0/physics_spec.md), [compute_api_spec.md](./spec_L3/compute_api_spec.md), [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md)
- **Notes**: **[РЕШЕНО в types v2.2]**: Утверждены 3 состояния `PackedTarget` и метод проверки неактивности `is_inactive()` (`0` ИЛИ `EMPTY_PIXEL`). Функция `unpack()` возвращает `None` для любых неактивных слотов. Вычислительные ядра и топология обязаны перейти на использование `is_inactive()`. *(Спецификации physics и compute-бэкендов будут обновлены при их проходе).*


### REV-COMPUTE-CPU-001: Межспецификационный долг фабрики дескрипторов `VramHandle`
- **ID**: REV-COMPUTE-CPU-001
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P0
- **Owner candidate**: `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.3)
- **Question / Problem**: `VramHandle` имел приватные поля, блокирующие его создание бэкендами.
- **Why it matters**: Ни один вычислительный бэкенд не мог вернуть валидный дескриптор аллоцированного шарда.
- **Affected specs**: [compute_api_spec.md](./spec_L3/compute_api_spec.md), [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: Добавлен публичный фабричный метод `VramHandle::from_raw_parts(kind, id, generation)` и геттеры.

### REV-LAYOUT-001: Двусмысленность раскладки `.state` блоба (Header vs State Payload Align)
- **ID**: REV-LAYOUT-001
- **Status**: Resolved (layout v2.2)
- **Priority**: P0
- **Owner candidate**: `layout`
- **Source**: [layout_spec.md](./spec_L1/layout_spec.md#L208) (§6.2)
- **Question / Problem**: В `layout_spec.md` формулы смещений описывают файл с заголовком 16B и выравниванием плоскостей по 64B. В результате для `padded_n = 64` смещение `dendrite_targets` равно 960B. При этом в тексте параллельно упоминался 896B для плотного режима без падов.
- **Why it matters**: Вызывало расхождение в расчете смещений между юнит-тестами, валидатором `baker` и вычислительными бэкендами, приводя к чтению неверных областей памяти VRAM/RAM.
- **Affected specs**: [layout_spec.md](./spec_L1/layout_spec.md), [baker_spec.md](./spec_L4/baker_spec.md), [compute_api_spec.md](./spec_L3/compute_api_spec.md)
- **Notes**: Утвержден единый стандарт Per-Plane 64B Alignment (`PADDED_N_ALIGNMENT = 64`). Первая плоскость выравнивается по 64B (`off_voltage = 64`), смещение `off_targets` составляет строго 960B для `padded_n = 64`. Альтернативный плотный блок (896B) аннулирован.

### REV-WIRE-001: Рассогласование полей `AxonHandoverEvent` с legacy-структурой
- **ID**: REV-WIRE-001
- **Status**: Open
- **Priority**: P0
- **Owner candidate**: `wire`
- **Source**: [wire_spec.md](./spec_L1/wire_spec.md#L378) (§12.1)
- **Question / Problem**: В `wire_spec.md` структура `AxonHandoverEvent` описана 5 полями `u32`. В legacy-коде используется оригинальный набор полей (`entry_x/y/z`, `vector_x/y/z`, `type_mask`, `remaining_length`).
- **Why it matters**: Ломает бинарную совместимость межшардовых пакетов спайков и передачи аксонов по сети.
- **Affected specs**: [wire_spec.md](./spec_L1/wire_spec.md), [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md), [protocol_spec.md](./spec_L5/protocol_spec.md)
- **Notes**: Сохранить оригинальный legacy-набор полей с атрибутом `#[repr(C)]` и явным `_padding` до 20 байт.

### REV-WEAVER-001: Отсутствие экспорта типов Weaver-сообщений для Runtime
- **ID**: REV-WEAVER-001
- **Status**: Open
- **Priority**: P0
- **Owner candidate**: `ipc`
- **Source**: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L276) (§11.4) vs [runtime_spec.md](./spec_L6/runtime_spec.md#L479) (§8)
- **Question / Problem**: Крейт `weaver-daemon` собирается как исполняемый бинарный файл (`bin`) и не экспортирует библиотечную публичную API-часть. Однако `runtime` обязан формировать и отправлять структуры `WeaverJobRequest`, `WeaverReport` и `WeaverGrowthContext` по IPC.
- **Why it matters**: `runtime` не может импортировать типы сообщений Weaver, что делает невозможным сборку L6 процесса.
- **Affected specs**: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md), [ipc_spec.md](./spec_L2/ipc_spec.md)
- **Notes**: Перенести DTO-структуры сообщений в крейт `ipc` (Layer 2) или в новый интерфейсный крейт `weaver-api`.

---

## §2. P1 Architecture Decisions (Архитектурные Решения)
*Архитектурные узлы и границы владения, влияющие на несколько спецификаций и требующие согласования перед генерацией кода.*

### REV-NET-001: Монопольное владение идентификаторами нод и зон
- **ID**: REV-NET-001
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `net`
- **Source**: [net_spec.md](./spec_L5/net_spec.md#L348) (§9.1)
- **Question / Problem**: Идентификаторы `NodeId` и `ZoneId` используются в `topology`, `baker`, `net`, `protocol` и `boot`. Не зафиксирован единый крейт-владелец их десериализации и валидации.
- **Why it matters**: Риск дублирования типов и десинхронизации бинарного представления сеток нод.
- **Affected specs**: [net_spec.md](./spec_L5/net_spec.md), [topology_spec.md](./spec_L4/topology_spec.md), [types_spec.md](./spec_L0/types_spec.md)
- **Notes**: Закрепить владение типами идентификаторов за `types` или `net`.

### REV-NET-002: Первичный источник конфигурации MTU
- **ID**: REV-NET-002
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `net`
- **Source**: [net_spec.md](./spec_L5/net_spec.md#L352) (§9.2) vs [protocol_spec.md](./spec_L5/protocol_spec.md#L265) (§10.8) vs [transport_spec.md](./spec_L5/transport_spec.md#L277) (§10.2)
- **Question / Problem**: MTU используется как в `protocol` (для L7-фрагментации), так и в `transport` (для размера сокетных буферов). Не ясно, кто вычисляет итоговый рабочий MTU.
- **Why it matters**: Несогласованный MTU приводит к повторной фрагментации на уровне IP или отбрасыванию пакетов сокетами (`DatagramTruncated`).
- **Affected specs**: [net_spec.md](./spec_L5/net_spec.md), [protocol_spec.md](./spec_L5/protocol_spec.md), [transport_spec.md](./spec_L5/transport_spec.md)
- **Notes**: Зафиксировать, что `net` вычисляет эффективный MTU на базе профиля маршрута и передает его в `protocol` и `transport`.

### REV-COMPUTE-API-002: Владение закрепленной хозяйской памятью (Pinned Host Buffers Ownership)
- **ID**: REV-COMPUTE-API-002
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P1
- **Owner candidate**: `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.4)
- **Question / Problem**: Не было зафиксировано владение Pinned Host буферами для DMA.
- **Why it matters**: Влияет на скорость PCIe-трансфера.
- **Affected specs**: [compute_api_spec.md](./spec_L3/compute_api_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md), [test_harness_spec.md](./spec_L3/test_harness_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: Владение Pinned Host staging буферами закреплено внутри конкретных реализаций бэкендов (`compute-cuda`, `compute-hip`). `ShardUpload` принимает заимствованные срезы.

### REV-COMPUTE-API-003: Синхронный vs асинхронный пакетный режим выполнения (Sync vs Async Batch Execution)
- **ID**: REV-COMPUTE-API-003
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P1
- **Owner candidate**: `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.5)
- **Question / Problem**: Определение базовой модели выполнения батча в L3 API.
- **Why it matters**: Влияет на блокировку потоков и синхронизацию буферов.
- **Affected specs**: [compute_api_spec.md](./spec_L3/compute_api_spec.md), [compute_spec.md](./spec_L3/compute_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: Базовый контракт `run_day_batch` зафиксирован как строго синхронный (блокирующий). Асинхронный контракт оставлен как будущий extension-trait.

### REV-COMPUTE-004: Модель инициализации воркеров и Thread-Affinity GPU контекста
- **ID**: REV-COMPUTE-004
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `compute`
- **Source**: [compute_spec.md](./spec_L3/compute_spec.md#L217) (§9.2) vs [boot_spec.md](./spec_L6/boot_spec.md#L354) (§8.4) vs [runtime_spec.md](./spec_L6/runtime_spec.md#L483) (§8.3) vs [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L205) (§11.5)
- **Question / Problem**: Не зафиксирован выбор между Моделью А (Send: создание контекста в boot с передачей воркеру) и Моделью B (Thread-Affine: создание GPU-контекста строго внутри выделенного потока воркера).
- **Why it matters**: В CUDA/HIP контекст привязан к потоку OS (thread-affinity). Передача дескриптора между потоками без `cuCtxMigrateCurrent` вызывает аварийный сбой CUDA_ERROR_INVALID_CONTEXT.
- **Affected specs**: [compute_spec.md](./spec_L3/compute_spec.md), [boot_spec.md](./spec_L6/boot_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md)
- **Notes**: Рекомендована Модель B (Thread-Affine инициализация).

### REV-IPC-001: Разделение владения C-ABI структурами SHM (`ShmHeader`, `ShmState`, `EphysShm`)
- **ID**: REV-IPC-001
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `layout`
- **Source**: [ipc_spec.md](./spec_L2/ipc_spec.md#L257) (§12.1) vs [boot_spec.md](./spec_L6/boot_spec.md#L353) (§8.3) vs [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L264) (§11.1)
- **Question / Problem**: Спецификация `ipc` описывает C-ABI структуры разделяемой памяти (`ShmHeader`, `ShmState`, `EphysShm`), но по общей архитектурной конвенции все бинарные layouts должны владеться крейтом `layout`.
- **Why it matters**: Размытие границ владения приводит к дублированию DTO структур между `layout` и `ipc`.
- **Affected specs**: [ipc_spec.md](./spec_L2/ipc_spec.md), [layout_spec.md](./spec_L1/layout_spec.md), [boot_spec.md](./spec_L6/boot_spec.md), [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md)
- **Notes**: Перенести бинарные описания структур заголовков в `layout`, а в `ipc` оставить управление жизненным циклом и кольцевыми буферами.

### REV-TOPOLOGY-001: Владение и разметка артефактов Ghost-связей (`.gxi`, `.gxo`, `.ghosts`)
- **ID**: REV-TOPOLOGY-001
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `layout` / `topology`
- **Source**: [topology_spec.md](./spec_L4/topology_spec.md#L237) (§11.1) vs [boot_spec.md](./spec_L6/boot_spec.md#L352) (§8.2) vs [baker_spec.md](./spec_L4/baker_spec.md#L247) (§11.2)
- **Question / Problem**: Архитектурный слой и крейт-владелец для спек бинарных файлов `.gxi`, `.gxo` и `.ghosts` размыт между `layout` (структуры) и `topology` / `baker` (алгоритмы).
- **Why it matters**: Затрудняет реализацию загрузчика `boot` и генератора артефактов `baker`.
- **Affected specs**: [topology_spec.md](./spec_L4/topology_spec.md), [layout_spec.md](./spec_L1/layout_spec.md), [boot_spec.md](./spec_L6/boot_spec.md), [baker_spec.md](./spec_L4/baker_spec.md)
- **Notes**: Зафиксировать бинарную разметку файлов в `layout`, а алгоритмы построения графа — в `topology`.

### REV-COMPUTE-API-004: Точный состав полезной нагрузки `BatchResult`
- **ID**: REV-COMPUTE-API-004
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P1
- **Owner candidate**: `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.6)
- **Question / Problem**: Разделение выходящих данных спайков и результатов телеметрии.
- **Why it matters**: Извлечение спайков для передачи в рантайм и сеть.
- **Affected specs**: [compute_api_spec.md](./spec_L3/compute_api_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md), [net_spec.md](./spec_L5/net_spec.md), [test_harness_spec.md](./spec_L3/test_harness_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: Выходящие спайковые ID записываются в `cmd.output_spikes`, а `BatchResult` возвращает счетчики и время исполнения.

### REV-TEST-001: API снимков состояния для тестового комплекса (Debug Snapshot API)
- **ID**: REV-TEST-001
- **Status**: Resolved (compute-api v2.1)
- **Priority**: P1
- **Owner candidate**: `test-harness` / `compute-api`
- **Source**: [compute_api_spec.md](./spec_L3/compute_api_spec.md) (§10.7)
- **Question / Problem**: Чтение полного состояния VRAM для пошаговых тестов детерминизма.
- **Why it matters**: Верификация потиковой корректности вычислений.
- **Affected specs**: [test_harness_spec.md](./spec_L3/test_harness_spec.md), [compute_api_spec.md](./spec_L3/compute_api_spec.md)
- **Notes**: **[РЕШЕНО в compute-api v2.1]**: В `ComputeBackend` добавлен метод по умолчанию `debug_snapshot(&mut self, handle, snapshot: ShardSnapshotMut<'_>)`.

### REV-PHYS-009: Генерация и верификация C++ зеркал из Rust-источников
- **ID**: REV-PHYS-009
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `physics` / `layout`
- **Source**: [physics_spec.md](./spec_L0/physics_spec.md#L286) (§8.7) vs [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L189) (§11.1) vs [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L199) (§11.2) vs [test_harness_spec.md](./spec_L3/test_harness_spec.md#L204) (§10.3)
- **Question / Problem**: C++ заголовки CUDA/HIP ядер ручным образом дублируют Rust-структуры из `physics` и `layout`. Требуется автоматическая кодогенерация или статическая верификация размеров/смещений полей.
- **Why it matters**: Любое изменение типов в Rust без обновления C++ файлов приведет к невидимому повреждению памяти в GPU ядер.
- **Affected specs**: [physics_spec.md](./spec_L0/physics_spec.md), [layout_spec.md](./spec_L1/layout_spec.md), [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md), [compute_hip_spec.md](./spec_L3/compute_hip_spec.md), [test_harness_spec.md](./spec_L3/test_harness_spec.md)
- **Notes**: Вынести вопрос разработки AOT-кодогенератора C++ зеркал на следующий архитектурный проход.

### REV-CFG-004: Размещение параметра `initial_synapse_weight` в TOML
- **ID**: REV-CFG-004
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `config`
- **Source**: [config_spec.md](./spec_L1/config_spec.md#L325) (§15.4) vs [boot_spec.md](./spec_L6/boot_spec.md#L359) (§8.9) vs [baker_spec.md](./spec_L4/baker_spec.md#L255) (§11.4) vs [topology_spec.md](./spec_L4/topology_spec.md#L247) (§11.3)
- **Question / Problem**: Поле `initial_synapse_weight` внесено в тесты валидации, но в схеме `NeuronType` для него не определена конкретная секция (`gsop` или `membrane`).
- **Why it matters**: Парсер `config` отклоняет валидные файлы конфигурации нейросетей при отсутствии этого поля.
- **Affected specs**: [config_spec.md](./spec_L1/config_spec.md), [boot_spec.md](./spec_L6/boot_spec.md), [baker_spec.md](./spec_L4/baker_spec.md), [topology_spec.md](./spec_L4/topology_spec.md)
- **Notes**: Зафиксировать размещение `initial_synapse_weight` в секции `[neuron_types.gsop]`.

### REV-BOOT-005: Точки интеграции boot/runtime/node при материализации сетевого рантайма
- **ID**: REV-BOOT-005
- **Status**: Open
- **Priority**: P1
- **Owner candidate**: `boot` / `node`
- **Source**: [boot_spec.md](./spec_L6/boot_spec.md#L356) (§8.6) vs [node_spec.md](./spec_L6/node_spec.md#L368) (§8.2)
- **Question / Problem**: Не определено, должен ли `boot` возвращать материализованный живой `NetRuntime` или только декларативный план `NetInitPlan` для последующей сборки в `node`.
- **Why it matters**: Нарушает разделение ответственности между загрузчиком ресурсов (`boot`) и процессным оркестратором (`node`).
- **Affected specs**: [boot_spec.md](./spec_L6/boot_spec.md), [node_spec.md](./spec_L6/node_spec.md), [net_spec.md](./spec_L5/net_spec.md)
- **Notes**: Утвердить, что `boot` возвращает `NetInitPlan`, а материализацию выполняет `node`.

---

## §3. P2 Cleanup / Naming / Consistency (Чистка, Именования и Согласованность)
*Согласование терминологии, названий типов, фиксирование версий зависимостей и мелкая чистка API.*

### REV-WIRE-006: Единая фиксация версии `bytemuck` в Cargo.toml
- **ID**: REV-WIRE-006
- **Status**: Open
- **Priority**: P2
- **Owner candidate**: `wire` / Workspace
- **Source**: [wire_spec.md](./spec_L1/wire_spec.md#L390) (§12.6) vs [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L223) (§12.6) vs [baker_spec.md](./spec_L4/baker_spec.md#L259) (§11.5)
- **Question / Problem**: В спецификациях версия `bytemuck = 1.25.0` указана как зафиксированная, но в разделах ревью тот же вопрос отмечен как находящийся на подтверждении.
- **Why it matters**: Двойственность статуса фиксации внешней зависимости в документации workspace.
- **Affected specs**: [wire_spec.md](./spec_L1/wire_spec.md), [layout_spec.md](./spec_L1/layout_spec.md), [types_spec.md](./spec_L0/types_spec.md), [baker_cli_spec.md](./spec_L4/baker_cli_spec.md), [baker_spec.md](./spec_L4/baker_spec.md)
- **Notes**: Окончательно утвердить выбор версии `=1.25.0` в Workspace Cargo.toml и удалить данный пункт из открытых вопросов.

### REV-NODE-006: Синхронизация версии библиотеки `tracing`
- **ID**: REV-NODE-006
- **Status**: Open
- **Priority**: P2
- **Owner candidate**: `node` / Workspace
- **Source**: [node_spec.md](./spec_L6/node_spec.md#L372) (§8.6)
- **Question / Problem**: Необходимость согласования и фиксации единой версии `tracing` / `tracing-subscriber` (`=0.1.40` / `=0.3.22`) на уровне всего workspace.
- **Why it matters**: Предотвращает дублирование версий и конфликты с глобальным диспетчером логов Tracing.
- **Affected specs**: [node_spec.md](./spec_L6/node_spec.md), [boot_spec.md](./spec_L6/boot_spec.md), [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md)
- **Notes**: Зафиксировать единые версии в коренном `Cargo.toml`.

### REV-PHYS-008: Крайние случаи DDS Heartbeat при `period=1`
- **ID**: REV-PHYS-008
- **Status**: Resolved (physics v2.2)
- **Priority**: P2
- **Owner candidate**: `physics`
- **Source**: [physics_spec.md](./spec_L0/physics_spec.md#L116) (§5.1.2)
- **Question / Problem**: При `period=1` значение фазы $65535$ не вызовет спайк (`65535 < 65535` ложно), хотя legacy-документация определяет `period=1` как генерацию спайка каждый тик.
- **Why it matters**: Приводит к пропуску спайков генераторов при определенных настройках периода.
- **Affected specs**: [physics_spec.md](./spec_L0/physics_spec.md), [config_spec.md](./spec_L1/config_spec.md)
- **Notes**: Зафиксировано, что при `period_ticks == 1` шаг фазы равен `MAX_HEARTBEAT_M` (65535), а предикат `is_heartbeat` возвращает `true` на каждом тике.

### REV-LAYOUT-003: Магическая сигнатура `.paths` и эндианность
- **ID**: REV-LAYOUT-003
- **Status**: Resolved (layout v2.2)
- **Priority**: P2
- **Owner candidate**: `layout`
- **Source**: [layout_spec.md](./spec_L1/layout_spec.md#L268) (§7.3)
- **Question / Problem**: Сигнатура magic для `.paths` задана как число `u32 = 0x50415448` (`"PATH"`). При записи на LE платформах байты записаны как `"HTAP"`.
- **Why it matters**: Затрудняет сторонний бинарный анализ и вызывает ошибки парсинга заголовка.
- **Affected specs**: [layout_spec.md](./spec_L1/layout_spec.md)
- **Notes**: Привести все magic сигнатуры в заголовках к типу байтового массива `[u8; 4]` (`*b"AXPT"`).

### REV-NET-009: Замена устаревших префиксов `axi-` на имя workspace
- **ID**: REV-NET-009
- **Status**: Open
- **Priority**: P2
- **Owner candidate**: `net`
- **Source**: [net_spec.md](./spec_L5/net_spec.md#L23) (§2.1)
- **Question / Problem**: `net_spec.md` ссылается на смежные модули с префиксом (например, `axi-types`, `axi-wire`), тогда как в остальной системе утверждены чистые имена (`types`, `wire`).
- **Why it matters**: Несоответствие имен крейтов в документации и коде.
- **Affected specs**: [net_spec.md](./spec_L5/net_spec.md)
- **Notes**: Заменить все вхождения `axi-` в `net_spec.md` на стандартные имена workspace.

### REV-BOOT-007: Унификация названия плана загрузки шарда (`BootShardPlan` vs `ShardBootPlan`)
- **ID**: REV-BOOT-007
- **Status**: Open
- **Priority**: P2
- **Owner candidate**: `boot` / `runtime`
- **Source**: [boot_spec.md](./spec_L6/boot_spec.md#L355) (§8.5) vs [runtime_spec.md](./spec_L6/runtime_spec.md#L259) (§4.1)
- **Question / Problem**: `boot_spec.md` использует имя `BootShardPlan`, а `runtime` оперирует именем `ShardBootPlan`.
- **Why it matters**: Разнобой в названиях фундаментальных DTO структур.
- **Affected specs**: [boot_spec.md](./spec_L6/boot_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md)
- **Notes**: Унифицировать под именем `ShardBootPlan`.

---

## §4. Accepted Deferred Debt (Осознанно Отложенный Долг)
*Вопросы и инженерные альтернативы, решение по которым осознанно отложено на будущее без блокировки текущей разработки.*

### REV-BOOT-008: Эмуляция системного RAM-диска на Windows
- **ID**: REV-BOOT-008
- **Status**: Deferred
- **Priority**: Deferred
- **Owner candidate**: `boot` / `ipc`
- **Source**: [boot_spec.md](./spec_L6/boot_spec.md#L357) (§8.7) vs [ipc_spec.md](./spec_L2/ipc_spec.md#L255) (§12)
- **Question / Problem**: Выбор системного механизма памяти для Windows-платформ (virtual RAM-drive / ImDisk) для временных рабочих директорий.
- **Why it matters**: На боевых Linux-нодах используется стандартный tmpfs; на Windows требуется эмуляция.
- **Affected specs**: [boot_spec.md](./spec_L6/boot_spec.md), [ipc_spec.md](./spec_L2/ipc_spec.md)
- **Notes**: Оставлен в статусе Deferred. На этапе прототипирования на Windows допускается использование обычной файловой системы.

### REV-NODE-004: Протокол и контракт внешней службы чекпоинтов
- **ID**: REV-NODE-004
- **Status**: Deferred
- **Priority**: Deferred
- **Owner candidate**: `node` / `runtime`
- **Source**: [node_spec.md](./spec_L6/node_spec.md#L370) (§8.4) vs [runtime_spec.md](./spec_L6/runtime_spec.md#L482) (§8.2) vs [boot_spec.md](./spec_L6/boot_spec.md#L358) (§8.8)
- **Question / Problem**: Сформировать точный интерфейс взаимодействия ноды со службой записи чекпоинтов (gRPC-клиент, запись в локальный RAM-диск с последующим сбросом или отдельный тред-райтер).
- **Why it matters**: Сохранение чекпоинтов большого размера не должно блокировать симуляцию.
- **Affected specs**: [node_spec.md](./spec_L6/node_spec.md), [runtime_spec.md](./spec_L6/runtime_spec.md), [boot_spec.md](./spec_L6/boot_spec.md)
- **Notes**: Вынесено за скобки текущей фазы проектирования ядра.

### REV-WEAVER-003: Модель управления процессом Weaver-Daemon
- **ID**: REV-WEAVER-003
- **Status**: Deferred
- **Priority**: Deferred
- **Owner candidate**: `weaver-daemon` / `node`
- **Source**: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L288) (§11.7) vs [node_spec.md](./spec_L6/node_spec.md#L369) (§8.3)
- **Question / Problem**: Модель управления PID демона координации (запуск нодой напрямую как child-process vs управление внешним супервизором OS / Kubernetes).
- **Why it matters**: Влияет на стратегию обработки сбоев процессов.
- **Affected specs**: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md), [node_spec.md](./spec_L6/node_spec.md)
- **Notes**: Будет определена на этапе DevOps-интеграции и составления Helm/systemd манифестов.

---

## §5. Raw Per-Spec Open Questions (Полный Реестр по Спецификациям)
*Полный систематизированный список открытых вопросов по каждому файлу спецификаций L0–L6.*

### §5.0. Слой L0 (Core Domain & Mathematics)

#### [physics_spec.md](./spec_L0/physics_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-PHYS-001**: Формула Magnetic Sentinel в legacy CUDA vs Инвариант
  - *Status*: Resolved (physics v2.2) | *Priority*: P1 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L278)
  - *Decision*: Побитовая формула из `INV-PHYS-006` (`((h ^ AXON_SENTINEL) >= v_seg)`) утверждена как единый стандарт для всех бэкендов.

- **REV-PHYS-002**: Граница Active Tail Hit (`< prop` vs `<= prop`)
  - *Status*: Resolved (physics v2.2) | *Priority*: P1 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L281)
  - *Decision*: Утверждено строгое неравенство `d < propagation_length` (§5.2.3).

- **REV-PHYS-003**: Противоречие Spatial Cooling в GSOP
  - *Status*: Resolved (physics v2.2) | *Priority*: P1 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L284)
  - *Decision*: Spatial Cooling официально и окончательно удалён из математики GSOP в `AxiEngine`.

- **REV-PHYS-004**: Типы аргументов AOT-деривации (`f32` vs Integer-scaled)
  - *Status*: Resolved (physics v2.2) | *Priority*: P2 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L287)
  - *Decision*: Использование `f32` в `compute_v_seg` изолировано строго на границе AOT/config (`config`, `baker`). Горячий рантайм на 100% целочисленный.

- **REV-PHYS-005**: Поведение `compile_dds_heartbeat` при периодах $> 65536$
  - *Status*: Resolved (physics v2.2) | *Priority*: P2 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L290)
  - *Decision*: Для `period_ticks > 65536` устанавливается `heartbeat_m = 0` (спонтанный спайкинг явно отключен).

- **REV-PHYS-006**: Коллизия маркеров `EMPTY_PIXEL` и `0` в `PackedTarget`
  - *Status*: Resolved (physics v2.2) | *Priority*: P1 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L293)
  - *Decision*: Синхронизировано с `types v2.2`. Физика и ядра используют предикат `PackedTarget::is_inactive()` (`0` или `EMPTY_PIXEL`).

- **REV-PHYS-007**: Семантика нулевого веса в Законе Дейла
  - *Status*: Resolved (physics v2.2) | *Priority*: P1 | *Owner*: `physics` | *Duplicate Of*: - | *Source*: [physics_spec.md](./spec_L0/physics_spec.md#L300)
  - *Decision*: Утверждён контракт Mass Floor Guard (`MIN_WEIGHT_LIMIT = 1`). При депрессии вес живого синапса ограничен снизу значением 1, сохраняя знак. Настоящий 0 веса не образуется при GSOP и существует только при занулении/удалении слота топологией через `PackedTarget::is_inactive()`.

#### [types_spec.md](./spec_L0/types_spec.md)
*Source items: 8 / Registered items: 8 (All Resolved in v2.2)*

- **REV-TYPES-001**: Размещение float-типов `Microns` и `Fraction`
  - *Status*: Resolved | *Priority*: P0 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L108)
  - *Decision*: Float-типы `Microns` и `Fraction` полностью вынесены из крейта `types` в крейты `config`/`topology`. Крейт `types` зафиксирован как на 100% целочисленный ABI-фундамент.

- **REV-TYPES-002**: Унификация `EMPTY_PIXEL` (`0xFFFF_FFFF`) и сырого `0`
  - *Status*: Resolved | *Priority*: P2 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L211)
  - *Decision*: Утверждены 3 состояния `PackedTarget` (`0` = zero-init None, `EMPTY_PIXEL` = pruned tombstone, live target) и инспекторы `is_zero_none()`, `is_tombstone()`, `is_inactive()`. Функция `unpack()` возвращает `None` для любых неактивных слотов (`is_inactive()`). Все compute-ядра оперируют `is_inactive()`.

- **REV-TYPES-003**: Разрядность `SegmentIndex`
  - *Status*: Resolved | *Priority*: P2 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L98)
  - *Decision*: Зафиксировано, что внутри `PackedTarget` смещение сегмента ограничено 8 битами (`MAX_SEGMENT_OFFSET = 255`). Обобщенный тип `SegmentIndex = u32` в `types` служит внешним контейнером верхнего уровня.

- **REV-TYPES-004**: Внешняя зависимость `wyhash` vs Inline Реализация
  - *Status*: Resolved | *Priority*: P2 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L37)
  - *Decision*: Зафиксирован курс на 0 внешних хеш-зависимостей. Avalanche mixers реализованы inline. В prod используется только `bytemuck = "=1.25.0"`, `static_assertions` — dev-dependency.

- **REV-TYPES-005**: Распиновка `SomaFlags`
  - *Status*: Resolved | *Priority*: P2 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L349)
  - *Decision*: Утверждена распиновка `0:spiking, 1..3:burst_count, 4..7:type_id`. Поле `type_id` в `SomaFlags` зафиксировано как рантайм-кэш зеркало для `PackedPosition.type_id` (SSOT). `baker`/`layout` формируют плоскости, `boot` валидирует при загрузке, `compute-api` фиксирует контракт, а вычислительные ядра сохраняют маску `SOMA_TYPE_MASK` (`0xF0`).


- **REV-TYPES-006**: Резервирование `MAX_AXON_ID`
  - *Status*: Resolved | *Priority*: P0 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L203)
  - *Decision*: Лимит `MAX_AXON_ID` уменьшен с `16_777_214` до `16_777_213` (`0x00FF_FFFD`) во избежание коллизии `pack(16_777_214, 255) == EMPTY_PIXEL`.

- **REV-TYPES-007**: Dual API Pattern для упакованных типов
  - *Status*: Resolved | *Priority*: P1 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L151)
  - *Decision*: Утвержден Dual API Pattern: быстрые O(1) конструкторы `new`/`pack` (с `debug_assert!` в debug-сборке), проверяемые `try_new`/`try_pack` (возвращающие `Result`) и предикаты `is_valid_*`. Upper layers обязаны использовать `try_*`.

- **REV-TYPES-008**: Удаление `random_f32` и целочисленные RNG/Hash
  - *Status*: Resolved | *Priority*: P1 | *Owner*: `types` | *Duplicate Of*: - | *Source*: [types_spec.md](./spec_L0/types_spec.md#L437)
  - *Decision*: `random_f32` удален из `types`. Крейт предоставляет только целочисленные генераторы `random_u32`/`random_u64`. Владение UUID и текстовыми слагами закреплено за `config`/baker, не добавляя UUID в `types`.

    ---

### §5.1. Слой L1 (Layouts, Configuration & Binary Formats)

#### [config_spec.md](./spec_L1/config_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-CFG-001**: Тип Физических Размеров `WorldConfig` (`f64` vs `u32`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L321)
  - *Question / Problem*: - *Контекст*: В AxiCAD TOML-схеме размеры мира `width_um` заданы как `f64`, в то время как легаси-движок использовал целые числа `u32`.
    - *Вопрос*: Фиксируется ли `f64` как единый целевой тип для размеров мира?

- **REV-CFG-002**: Регистр Символов в `EntryZ` (`"Top"` vs `"top"`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L325)
  - *Question / Problem*: - *Контекст*: В примерах AxiCAD встречается написание с заглавной буквы (`"Top"`, `"Mid"`, `"Bottom"`), хотя `Direction` использует строчные буквы (`"in"`, `"out"`).
    - *Вопрос*: Приводится ли `EntryZ` к нижнему регистру (`"top"`, `"mid"`, `"bottom"`) для единообразия Serde?

- **REV-CFG-003**: Верхняя Граница Плотности `density` (`<= 1.0`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L329)
  - *Question / Problem*: - *Контекст*: Инвариант `INV-CONFIG-002` требует только `density >= 0.0`.
    - *Вопрос*: Требуется ли жестко ограничить плотность сверху значением `density <= 1.0`?

- **REV-CFG-004**: Формат Стабильного Идентификатора Связи `connections.id`
  - *Status*: Open | *Priority*: P1 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L333)
  - *Question / Problem*: - *Контекст*: Поле `id` добавлено как целевой связующий ключ для геометрии.
    - *Вопрос*: Фиксируется ли формат `id` как UUID v4 или разрешаются произвольные текстовые слаги?

- **REV-CFG-005**: Размещение и Тестирование `initial_synapse_weight` в TOML-схеме
  - *Status*: Open | *Priority*: P1 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L337)
  - *Question / Problem*: - *Контекст*: В C-ABI структуре `VariantParameters` из `layout` присутствует поле `initial_synapse_weight: u16`, однако в текущей TOML-схеме `NeuronType` оно отсутствует.
    - *Вопрос*: В какую секцию `NeuronType` в TOML следует добавить поле `initial_synapse_weight` (в `gsop` или `membrane`)? Тест этого поля перенесен в категорию Review Debt.

- **REV-CFG-006**: Крайний Случай DDS Heartbeat (`period = 1`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L341)
  - *Question / Problem*: - *Контекст*: В валидации `spontaneous_firing_period_ticks` значение `1` связано с открытым вопросом в `physics_spec.md` по поводу вычисления фазового аккумулятора.
    - *Вопрос*: Как семантически утверждается период `1` на уровне конфигурации?

- **REV-CFG-007**: Политика Точности Валидации `v_seg`
  - *Status*: Open | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: - | *Source*: [config_spec.md](./spec_L1/config_spec.md#L345)
  - *Question / Problem*: - *Контекст*: Проверка целочисленности `v_seg` в `physics` зависит от точности входных параметров.
    - *Вопрос*: Рассмотреть использование фиксированной точной арифметики при проверке `v_seg`?

- **REV-CFG-008**: Будущее Поля `max_dendrites` в TOML
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `config` | *Duplicate Of*: REV-PHYS-004 | *Source*: [config_spec.md](./spec_L1/config_spec.md#L349)
  - *Question / Problem*: - *Контекст*: Сейчас `max_dendrites` жестко проверяется на равенство `128` (согласно `layout::MAX_DENDRITES`).
    - *Вопрос*: Сохраняется ли это поле как явный assertion пользователя в TOML или удаляется из пользовательского DSL в следующих версиях?

#### [layout_spec.md](./spec_L1/layout_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-LAYOUT-001**: Единый квант выравнивания `PADDED_N_ALIGNMENT`
  - *Status*: Resolved (layout v2.2) | *Priority*: P0 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L208)
  - *Decision*: Утвержден единый стандарт Per-Plane 64B Alignment (`PADDED_N_ALIGNMENT = 64`). Первая плоскость выравнивается по 64B (off_voltage = 64), смещение off_targets составляет строго 960B для padded_n = 64. Альтернативный плотный блок (896B) аннулирован.

- **REV-LAYOUT-002**: Монопольное владение `ShardVramPtrs` (`layout` vs `compute-api`)
  - *Status*: Resolved (layout v2.2) | *Priority*: P2 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L158)
  - *Decision*: C-ABI DTO указателей `ShardVramPtrs` перенесено под монопольное владение `layout`.

- **REV-LAYOUT-003**: Магические константы бинарных файлов
  - *Status*: Resolved (layout v2.2) | *Priority*: P2 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L248)
  - *Decision*: Сигнатуры заголовков стандартизированы как байтовые массивы `[u8; 4]`: `*b"AXST"`, `*b"AXAX"`, `*b"AXPT"`.

- **REV-LAYOUT-004**: Коллизия `EMPTY_PIXEL = 0xFFFF_FFFF` в массиве таргетов
  - *Status*: Resolved (layout v2.2) | *Priority*: P2 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L192)
  - *Decision*: Синхронизировано с `types v2.2`. Вычислительные ядра проверяют неактивность слота через предикат `PackedTarget::is_inactive()`.

- **REV-LAYOUT-005**: Размещение отладочной структуры `EphysShm`
  - *Status*: Resolved (layout v2.2) | *Priority*: P2 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L65)
  - *Decision*: Отладочная структура `EphysShm` вынесена под монопольное владение `ipc` / `test-harness`.

- **REV-LAYOUT-006**: Состав `.state` дампа (Day Hot vs Night State)
  - *Status*: Resolved (layout v2.2) | *Priority*: P1 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L182)
  - *Decision*: Файл `.state` содержит строго 8 горячих SoA-плоскостей Дневной Фазы.

- **REV-LAYOUT-007**: Несоответствие сентинеля аксона в legacy-комментариях
  - *Status*: Resolved (layout v2.2) | *Priority*: P1 | *Owner*: `layout` | *Duplicate Of*: - | *Source*: [layout_spec.md](./spec_L1/layout_spec.md#L153)
  - *Decision*: Аннулирована устаревшая запись `0xFFFFFFFF`. Единственным стандартом зафиксирован `AXON_SENTINEL = 0x8000_0000` из `types v2.2`.

#### [wire_spec.md](./spec_L1/wire_spec.md)
*Source items: 6 / Registered items: 6*

- **REV-WIRE-001**: Отсутствие Magic-поля в `SpikeBatchHeaderV2`
  - *Status*: Open | *Priority*: P0 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L380)
  - *Question / Problem*: - *Контекст*: Первое слово `src_zone_hash` не является сигнатурой magic. При диспетчеризации пакетов по первому `u32` возможен конфликт с другими magic.
    - *Вопрос*: Требуется ли введение новой версии заголовка спайков с явным полем magic/version?

- **REV-WIRE-002**: Границы Владения Файловыми Заголовками `.gxi`, `.gxo`, `.ghosts`
  - *Status*: Open | *Priority*: P2 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L384)
  - *Question / Problem*: - *Контекст*: В легаси-коде эти заголовки находились в `ipc.rs`.
    - *Вопрос*: К какому крейту следует отнести владение этими заголовками — `layout`, `baker` или `ipc`?

- **REV-WIRE-003**: Размер Структуры `AxonHandoverPrune` (12B vs 16B)
  - *Status*: Open | *Priority*: P2 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L388)
  - *Question / Problem*: - *Контекст*: Текущий размер структуры равен 12 байтам.
    - *Вопрос*: Требуется ли выровнять структуру явным падом до 16 байт для кратности 8/16 байтам?

- **REV-WIRE-004**: Размещение `ShardStateHeader` (`SNAP`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L392)
  - *Question / Problem*: - *Контекст*: Структура используется при сетевой репликации VRAM, но соприкасается с макетами дампов.
    - *Вопрос*: Оставляем ли мы `ShardStateHeader` в `wire` или относим к `layout`/`ipc`?

- **REV-WIRE-005**: Фиксация Версии `bytemuck` (`=1.25.0` vs `=1.20.0`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L396)
  - *Question / Problem*: - *Контекст*: В легаси документации встречались разные версии.
    - *Вопрос*: Подтверждается ли единая фиксация версии `=1.25.0` в Cargo.toml?

- **REV-WIRE-006**: Поддержка Архитектур Big-Endian
  - *Status*: Open | *Priority*: P2 | *Owner*: `wire` | *Duplicate Of*: - | *Source*: [wire_spec.md](./spec_L1/wire_spec.md#L400)
  - *Question / Problem*: - *Контекст*: Все пакеты сериализуются в Little-Endian.
    - *Вопрос*: Фиксируем ли мы `compile_error!` при сборке под Big-Endian платформы или вводим явные функции конвертации полей?

### §5.2. Слой L2 (VFS & IPC Interfaces)

#### [ipc_spec.md](./spec_L2/ipc_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-IPC-001**: Нерешенный Владелец Декларации `ShmHeader`, `ShmState` и `EphysShm`
  - *Status*: Open | *Priority*: P1 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L259)
  - *Question / Problem*: - *Контекст*: Сейчас макеты структур соприкасаются с `layout` и `ipc`. Принятие решения требует обновления `layout_spec.md`.
    - *Вопрос*: Утверждается ли полное монопольное владение C-ABI объявлениями этих структур за крейтом `layout` с обновлением его спецификации?

- **REV-IPC-002**: Граница Расчета `shm_size` (Логический Макет vs Страница ОС)
  - *Status*: Open | *Priority*: P1 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L263)
  - *Question / Problem*: - *Контекст*: Логическая сумма полей рассчитывается по формулам SoA, но mmap требует выравнивания на 4096 байт.
    - *Вопрос*: К какому крейту относится функция расчета итогового выровненного размера — `layout` или `ipc`?

- **REV-IPC-003**: Канал Управления на платформе Windows (Named Pipe vs Localhost TCP)
  - *Status*: Open | *Priority*: P1 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L267)
  - *Question / Problem*: - *Контекст*: На Linux используется Unix Domain Sockets. На Windows легаси-код применял localhost TCP сокеты.
    - *Вопрос*: Утверждается ли Named Pipes в качестве основного стандарта управляющего канала для Windows взамен fallback TCP?

- **REV-IPC-004**: Доменная Принадлежность Сигналов `BakeRequest`
  - *Status*: Open | *Priority*: P2 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L271)
  - *Question / Problem*: - *Контекст*: Сигналы запуска AOT-сборки передаются через управляющий канал IPC.
    - *Вопрос*: Относятся ли структуры сигналов `BakeRequest` к крейту `wire` или объявляются в `ipc`?

- **REV-IPC-005**: Владелец Сетевой Теневой Репликации (Shadow Replication)
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `ipc` | *Duplicate Of*: REV-WEAVER-001 | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L275)
  - *Question / Problem*: - *Контекст*: Использование `sendfile`/`splice` для передач VRAM-дампов соприкасается с IPC и сетевым стеком.
    - *Вопрос*: Является ли теневая репликация исключительно инфраструктурным примитивом `ipc` или переносится в ведение `transport`/`runtime`?

- **REV-IPC-006**: Строгость Порядка Атомарных Операций (AcqRel vs SeqCst)
  - *Status*: Open | *Priority*: P2 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L279)
  - *Question / Problem*: - *Контекст*: Для автоматов переходов предложен `AcqRel`, однако для упрощения модели анализа возможен переход на `SeqCst`.
    - *Вопрос*: Требуется ли зафиксировать `SeqCst` для всех переходов автомата состояний SHM?

- **REV-IPC-007**: Политика Обработки Переполнения Внешнего Входного Буфера
  - *Status*: Open | *Priority*: P2 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L283)
  - *Question / Problem*: - *Контекст*: При высокой плотности данных внешний поток ввода может переполнить емкость буфера.
    - *Вопрос*: Выбирается ли политика отбрасывания пакетов (Drop), обратного давления (Backpressure) или возврат ошибки?

- **REV-IPC-008**: Целевая Версия `SHM_VERSION`
  - *Status*: Duplicate Of | *Priority*: Deferred | *Owner*: `ipc` | *Duplicate Of*: REV-BOOT-008 | *Source*: [ipc_spec.md](./spec_L2/ipc_spec.md#L287)
  - *Question / Problem*: - *Контекст*: В legacy-коде используется версия `SHM_VERSION = 3`.
    - *Вопрос*: Сохраняется ли значение 3 или инкрементируется до 4 при утверждении обновленного макета `layout v2.0`?

#### [vfs_spec.md](./spec_L2/vfs_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-VFS-001**: Версионирование Бинарного Формата `.axic`
  - *Status*: Open | *Priority*: P1 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L231)
  - *Question / Problem*: - *Контекст*: В текущей спецификации зафиксирована версия `1`.
    - *Вопрос*: Сохраняется ли версия `1` для первого Rust-переписывания, или требуется инкремент версии?

- **REV-VFS-002**: Объем Отображения mmap (Whole Archive vs Sub-File Views)
  - *Status*: Open | *Priority*: P2 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L235)
  - *Question / Problem*: - *Контекст*: Сейчас `vfs` отображает весь архив целиком и возвращает срезы `&[u8]`.
    - *Вопрос*: Требуется ли поддержка независимых подфайловых отображений (Sub-File Mmap Views) для каждого файла отдельно?

- **REV-VFS-003**: Адекватность Выравнивания для Платформы Windows
  - *Status*: Open | *Priority*: P1 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L239)
  - *Question / Problem*: - *Контекст*: Размер `ARCHIVE_PAYLOAD_ALIGNMENT` равен 4096 байтам, но граница выделения памяти (Allocation Granularity) на Windows равна 64 КБ (65536 байт).
    - *Вопрос*: Требуется ли увеличить выравнивание полезной нагрузки до 64 КБ для поддержки независимого sub-file mmap на Windows?

- **REV-VFS-004**: Контрольные Суммы Элементов TOC (SHA256 / CRC32)
  - *Status*: Open | *Priority*: P2 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L243)
  - *Question / Problem*: - *Контекст*: Сейчас TOC хранит только смещение и размер.
    - *Вопрос*: Требуется ли добавить контрольные суммы (CRC32 или SHA256) для каждого файла в TOC для верификации целостности реестра артефактов?

- **REV-VFS-005**: Политика Сжатия Файлов в Архиве
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `vfs` | *Duplicate Of*: REV-LAYOUT-001 | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L247)
  - *Question / Problem*: - *Контекст*: Сжатие файлов делает невозможным прямой Zero-Copy access через mmap без декомпрессии в кучу.
    - *Вопрос*: Запрещается ли сжатие окончательно, или допускается опциональный алгоритм (например, zstd) для некритичных артефактов?

- **REV-VFS-006**: Размещение Низкоуровневого Упаковщика (`AxicPacker`)
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `vfs` | *Duplicate Of*: REV-COMPUTE-API-002 | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L251)
  - *Question / Problem*: - *Контекст*: Сейчас примитивы упаковки находятся в `vfs`.
    - *Вопрос*: Сохраняется ли `AxicPacker` в `vfs`, или он полностью переносится в `baker`, оставляя за `vfs` только структуры формата?

- **REV-VFS-007**: Минимальный Обязательный Набор Файлов Загрузочного Архива
  - *Status*: Open | *Priority*: P2 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L255)
  - *Question / Problem*: - *Контекст*: Модуль `boot` проверяет состав файлов перед запуском.
    - *Вопрос*: Какой точный минимальный список файлов (`.state`, `.axons`, `model.toml` и т.д.) является обязательным для признания `.axic` архива валидным к загрузке?

- **REV-VFS-008**: Владелец Экспорта Манифеста в SHM / Temp
  - *Status*: Open | *Priority*: P2 | *Owner*: `vfs` | *Duplicate Of*: - | *Source*: [vfs_spec.md](./spec_L2/vfs_spec.md#L259)
  - *Question / Problem*: - *Контекст*: Файл манифеста экспортируется в временную директорию ОС или SHM при старте.
    - *Вопрос*: К какому крейту относится логика экспорта манифеста из сырых байтов `vfs` — `boot` или `ipc`?

### §5.3. Слой L3 (Compute Abstractions & Backends)

#### [compute_api_spec.md](./spec_L3/compute_api_spec.md)
*Source items: 6 / Registered items: 6*

- **REV-COMPUTE-API-007**: Поддержка Окружений `no_std + alloc`
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P0 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Крейт `compute-api` содержит только абстрактные контракты и DTO.
    - *Вопрос*: Требуется ли перевести `compute-api` в режим `no_std + alloc` для поддержки встраиваемых систем (Edge devices)?
  - *Resolution*: Утвержден строго легкий `no_std` / `no_alloc` контракт без динамических аллокаций.

- **REV-COMPUTE-API-002**: Модель Владения Pinned Host Буферами
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P1 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Для скоростного DMA переноса требуются закрепощенные страницы памяти хоста (Pinned Memory).
    - *Вопрос*: Кто должен монопольно владеть Pinned-буферами — DTO дескриптор API, сам бэкенд или вышележащий фасад `compute`?
  - *Resolution*: Владение Pinned Host буферами закреплено внутри конкретных реализаций бэкендов (`compute-cuda`, `compute-hip`).

- **REV-COMPUTE-API-003**: Модель Выполнения Батча (Синхронная vs Асинхронная)
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P1 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Метод `run_day_batch` может быть блокирующим синхронным вызовом или асинхронной моделью сабмита со сплитом `submit_batch` / `sync_batch`.
    - *Вопрос*: Зафиксировать ли строго синхронную модель выполнения батча на уровне API?
  - *Resolution*: Базовый метод `run_day_batch` зафиксирован как строго синхронный (блокирующий).

- **REV-COMPUTE-API-004**: Точная Форма DTO Результатов Телеметрии (`BatchResult`)
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P1 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Структура результатов пока содержит минимальный набор полей.
    - *Вопрос*: Какая точная структура массива сгенерированных спайков должна возвращаться в `BatchResult`?
  - *Resolution*: Выходящие спайковые ID записываются в `cmd.output_spikes`, а `BatchResult` возвращает счетчики и телеметрию.

- **REV-COMPUTE-API-005**: Допустимость Частичной Загрузки Таблицы Аксонов (`.axons`)
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P1 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Таблица аксонов может быть огромной.
    - *Вопрос*: Допускается ли частичная загрузка (partial upload) буфера аксонов, или требовать только полный единовременный блоб?
  - *Resolution*: В v2.1 допускается только полная загрузка `ShardUpload`. Частичная загрузка оставлена как будущая фича.

- **REV-COMPUTE-API-006**: Зона Владения Вспомогательных Команд Сортировки и Синхронизации
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `compute-api` | *Duplicate Of*: REV-COMPUTE-006 | *Source*: [compute_api_spec.md](./spec_L3/compute_api_spec.md)
  - *Question / Problem*: - *Контекст*: Команды `sort_and_prune`, синхронизация Ghost-аксонов и отладочные вызовы Ephys пересекаются с различными слоями.
    - *Вопрос*: Относятся ли данные методы к `compute-api` или выносятся на уровень фасада `compute` / `runtime`?
  - *Notes*: Вопрос распределения владения операциями уплотнения и синхронизации Ghost-аксонов вынесен за рамки базового контракта compute-api v2.1 и остается открытым в спецификациях compute, runtime и бэкендов.

#### [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-COMPUTE-CPU-001**: Модель Фабричного Конструктора `VramHandle` в `compute-api`
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P0 | *Owner*: `compute-api` | *Duplicate Of*: - | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md)
  - *Question / Problem*: - *Контекст*: Приватное поле `VramHandle` блокирует его создание внутри `compute-cpu`.
    - *Вопрос*: Как именно должен выглядеть контролируемый фабричный метод в `compute-api` для создания дескрипторов бэкендами?
  - *Resolution*: В `VramHandle` добавлен фабричный метод `from_raw_parts(kind, id, generation)` и геттеры.

- **REV-COMPUTE-CPU-002**: Окончательная Семантика Двойной Проверки Tombstone Target (`0` vs `EMPTY_PIXEL`)
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `compute-cpu` | *Duplicate Of*: REV-TYPES-001 | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md)
  - *Question / Problem*: - *Контекст*: В спецификациях слоя L0/L1 сохраняется долг по стандартизации проверки неактивных синапсов.
    - *Вопрос*: Какое единое побитовое правило проверки целевого пикселя должно использоваться бэкендами вычислений?

- **REV-COMPUTE-CPU-003**: Окончательная Форма DTO Результатов Телеметрии в `compute-api`
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `compute-cpu` | *Duplicate Of*: REV-COMPUTE-API-004 | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md)
  - *Question / Problem*: - *Контекст*: Структура `BatchResult` находится на этапе согласования.
    - *Вопрос*: Каким образом `compute-cpu` должен передавать массив сгенерированных спайков и осциллограммы телеметрии?
  - *Resolution*: Выходящие спайковые ID записываются в `cmd.output_spikes`, потиковые счетчики спайков в `cmd.output_spike_counts`, а `BatchResult` содержит сводную телеметрию и счетчики.

- **REV-COMPUTE-CPU-004**: Монопольный Владелец Маршрутизации Данных Отладчика Ephys
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cpu` | *Duplicate Of*: - | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md#L211)
  - *Question / Problem*: - *Контекст*: Отладочный съем осциллограмм требует доступа к SoA-массивам мембранных потенциалов.
    - *Вопрос*: Являются ли отладочные методы частью `ComputeBackend`, или они выносятся в отдельный сервис?

- **REV-COMPUTE-CPU-005**: Владелец Операций Сортировки и Уплотнения Связей (Sort & Prune)
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cpu` | *Duplicate Of*: - | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md#L215)
  - *Question / Problem*: - *Контекст*: Операция `sort_and_prune` удаляет деградировавшие синапсы во время Ночной Фазы.
    - *Вопрос*: Относится ли `sort_and_prune` к методам `ComputeBackend`, или выполняется на уровне `compute`/`runtime`?

- **REV-COMPUTE-CPU-006**: Необходимость Внутренних Зависимостей `slotmap` и `bytemuck`
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cpu` | *Duplicate Of*: - | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md#L219)
  - *Question / Problem*: - *Контекст*: Текущая спецификация упоминает эти библиотеки как внутренние детали реализации.
    - *Вопрос*: Фиксируются ли эти крейты в качестве обязательных внутренних зависимостей `compute-cpu`?

- **REV-COMPUTE-CPU-007**: Управление Пулом Потоков Rayon (Global vs Custom Threadpool)
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cpu` | *Duplicate Of*: - | *Source*: [compute_cpu_spec.md](./spec_L3/compute_cpu_spec.md#L223)
  - *Question / Problem*: - *Контекст*: По умолчанию Rayon использует глобальный пул потоков процесса.
    - *Вопрос*: Должен ли `CpuBackend` создавать и хранить собственный изолированный `rayon::ThreadPool` для предотвращения конфликтов с другими модулями движка?

#### [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md)
*Source items: 6 / Registered items: 6*

- **REV-COMPUTE-CUDA-001**: Механизм Кодогенерации и Верификации C++ Зеркал из Rust (`physics`/`layout`)
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-cuda` | *Duplicate Of*: REV-PHYS-009 | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L189)
  - *Question / Problem*: - *Контекст*: Зафиксирован запрет на ручной дублирующий C++ код.
    - *Вопрос*: Какая утилита или генератор (например, `cbindgen` или пользовательский AOT-скрипт) будет координировать автоматическую сборку C++ зеркал из источников истины?

- **REV-COMPUTE-CUDA-002**: API и DTO Загрузки Таблицы Вариантов в Constant Memory
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cuda` | *Duplicate Of*: - | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L193)
  - *Question / Problem*: - *Контекст*: `ShardUpload` содержит только байтовые блобы состояния и аксонов.
    - *Вопрос*: Через какой интерфейс (отдельный метод HAL, расширение `ShardUpload` или операция фасада `compute`) таблица вариантов нейронов должна передаваться бэкенду для загрузки в Constant Memory?

- **REV-COMPUTE-CUDA-003**: Модель Фабричного Конструктора `VramHandle` в `compute-api`
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `compute-cuda` | *Duplicate Of*: REV-COMPUTE-CPU-001 | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L197)
  - *Question / Problem*: - *Контекст*: Приватность `VramHandle` блокирует создание дескрипторов бэкендами.
    - *Вопрос*: Каким образом бэкенд вычислений будет получать экземпляры `VramHandle` из `compute-api`?

- **REV-COMPUTE-CUDA-004**: Формат и Владелец Pinned-Буферов Результатов
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-cuda` | *Duplicate Of*: REV-COMPUTE-API-002 | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L201)
  - *Question / Problem*: - *Контекст*: Требование `pinned_host_required = true` необходимо для скорейшего DMA D2H.
    - *Вопрос*: Кто создает и держит Pinned-буферы для результатов — `compute-cuda` или IPC/runtime swapchain?

- **REV-COMPUTE-CUDA-005**: Аффинность Потоков ОС и Маркер `Send` для CUDA Контекста
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-cuda` | *Duplicate Of*: REV-COMPUTE-004 | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L205)
  - *Question / Problem*: - *Контекст*: CUDA контексты и стримы привязаны к создавшему их OS-потоку.
    - *Вопрос*: Является ли `CudaBackend` маркерным `Send`, или инициализация контекста должна происходить строго внутри целевого OS-потока шарда?

- **REV-COMPUTE-CUDA-006**: Владение Операциями Синхронизации Ghost-Аксонов и Сортировки
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-cuda` | *Duplicate Of*: - | *Source*: [compute_cuda_spec.md](./spec_L3/compute_cuda_spec.md#L209)
  - *Question / Problem*: - *Контекст*: Операции `sort_and_prune` и межшардовые патчи затрагивают памяти ускорителя.
    - *Вопрос*: Относятся ли методы уплотнения синапсов к `ComputeBackend`, или они выносятся в отдельный сервисный слой?

#### [compute_hip_spec.md](./spec_L3/compute_hip_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-COMPUTE-HIP-001**: Поддержка Режима Wave32 на Архитектурах AMD RDNA (Future Work)
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-hip` | *Duplicate Of*: - | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L195)
  - *Question / Problem*: - *Контекст*: Спецификация v2 жестко фиксирует использование Wave64.
    - *Вопрос*: Каким образом в будущих версиях спецификации будет организована адаптация масок вейвфронта для архитектур RDNA с режимом Wave32?

- **REV-COMPUTE-HIP-002**: Единая Стратегия Автоматической Кодогенерации C++ Зеркал CUDA/HIP из Rust
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-hip` | *Duplicate Of*: REV-PHYS-009 | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L199)
  - *Question / Problem*: - *Контекст*: Во избежание рассинхронизации математики ядра CUDA и HIP не должны содержать дублирующих ручных формул.
    - *Вопрос*: Каким образом будет организован общий пайплайн кодогенерации C++ ядер из источников истины в `physics` и `layout`?

- **REV-COMPUTE-HIP-003**: API и DTO Загрузки Таблицы Вариантов в Constant Memory
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `compute-hip` | *Duplicate Of*: REV-COMPUTE-CUDA-002 | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L203)
  - *Question / Problem*: - *Контекст*: `ShardUpload` не содержит вариантов нейронов.
    - *Вопрос*: Через какой интерфейс (отдельный метод HAL, расширение `ShardUpload` или операция фасада `compute`) таблица вариантов должна передаваться бэкенду?

- **REV-COMPUTE-HIP-004**: Модель Фабричного Конструктора `VramHandle` в `compute-api`
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `compute-hip` | *Duplicate Of*: REV-COMPUTE-CPU-001 | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L207)
  - *Question / Problem*: - *Контекст*: Приватность `VramHandle` блокирует создание дескрипторов бэкендами.
    - *Вопрос*: Каким образом бэкенд вычислений будет получать экземпляры `VramHandle` из `compute-api`?

- **REV-COMPUTE-HIP-005**: Формат и Владелец Pinned-Буферов Результатов
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-hip` | *Duplicate Of*: REV-COMPUTE-API-002 | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L211)
  - *Question / Problem*: - *Контекст*: Требование `pinned_host_required = true` необходимо для скорейшего DMA D2H.
    - *Вопрос*: Кто создает и держит Pinned-буферы для результатов — `compute-hip` или IPC/runtime swapchain?

- **REV-COMPUTE-HIP-006**: Аффинность Потоков ОС и Маркер `Send` для HIP Контекста
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute-hip` | *Duplicate Of*: REV-COMPUTE-004 | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L215)
  - *Question / Problem*: - *Контекст*: HIP контексты и стримы привязаны к создавшему их OS-потоку.
    - *Вопрос*: Является ли `HipBackend` маркерным `Send`, или инициализация контекста должна происходить строго внутри целевого OS-потока шарда?

- **REV-COMPUTE-HIP-007**: Владение Операциями Синхронизации Ghost-Аксонов и Сортировки
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute-hip` | *Duplicate Of*: - | *Source*: [compute_hip_spec.md](./spec_L3/compute_hip_spec.md#L219)
  - *Question / Problem*: - *Контекст*: Операции `sort_and_prune` и межшардовые патчи затрагивают память ускорителя.
    - *Вопрос*: Относятся ли методы уплотнения синапсов к `ComputeBackend`, или они выносятся в отдельный сервисный слой?

#### [compute_spec.md](./spec_L3/compute_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-COMPUTE-001**: Проверка Совместимости Сборки при Миграции Имен Фичей
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute` | *Duplicate Of*: - | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L213)
  - *Question / Problem*: - *Контекст*: Зафиксирована целевая политика v2.0 на замену legacy-имен `amd` и `mock-gpu` на `hip` и `mock`.
    - *Вопрос*: Требуется ли временное сохранение псевдонимов (aliases) фичей в `Cargo.toml` на период миграции?

- **REV-COMPUTE-002**: Аффинность Потоков ОС и Маркер `Send` для `ShardEngine`
  - *Status*: Open | *Priority*: P1 | *Owner*: `compute` | *Duplicate Of*: - | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L217)
  - *Question / Problem*: - *Контекст*: Модуль `runtime` выделяет отдельный OS-thread на каждый шард. Контексты некоторых GPU бэкендов привязаны к создавшему их потоку (Thread-Affine).
    - *Вопрос*: Должен ли `ShardEngine` быть `Send` для передачи из потока `boot` в поток шарда, или `ShardEngine` должен создаваться строго внутри целевого OS-потока шарда по загрузочному плану?

- **REV-COMPUTE-003**: Точный Приоритет Автовыбора для Кроссплатформенных Сборщиков
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute` | *Duplicate Of*: - | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L221)
  - *Question / Problem*: - *Контекст*: На некоторых системах могут быть одновременно установлены драйверы разных вендоров.
    - *Вопрос*: Является ли порядок CUDA -> HIP -> CPU универсальным для всех ОС, или требуется гибкая настройка приоритетов?

- **REV-COMPUTE-004**: Синхронная vs Асинхронная Модель API Выполнения Батча
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute` | *Duplicate Of*: REV-COMPUTE-API-003 | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L225)
  - *Question / Problem*: - *Контекст*: Метод `run_day_batch` в текущей версии является блокирующим.
    - *Вопрос*: Требуется ли введение асинхронной модели `submit_batch` / `poll_batch` на уровне фасада `ShardEngine`?

- **REV-COMPUTE-005**: Монопольный Владелец Pinned Host Буферов Ввода-Вывода
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `compute` | *Duplicate Of*: REV-COMPUTE-API-002 | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L229)
  - *Question / Problem*: - *Контекст*: Закрепощенные страницы памяти хоста (Pinned Memory) необходимы для скоростного DMA.
    - *Вопрос*: Кто владеет Pinned-буферами — фасад `compute` или IPC/runtime swapchain?

- **REV-COMPUTE-006**: Зона Владения Операциями Синхронизации Ghost-Аксонов и Сортировки
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute` | *Duplicate Of*: - | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L233)
  - *Question / Problem*: - *Контекст*: Межзоновые патчи и примитивы сортировки спайков затрагивают сетевой стек и вычисления.
    - *Вопрос*: Относятся ли методы синхронизации Ghost-слотов к фасаду `compute` или выносятся в `runtime`/`network`?

- **REV-COMPUTE-007**: Маршрутизация Данных Отладчика Ephys
  - *Status*: Open | *Priority*: P2 | *Owner*: `compute` | *Duplicate Of*: - | *Source*: [compute_spec.md](./spec_L3/compute_spec.md#L237)
  - *Question / Problem*: - *Контекст*: Снимок осциллограмм Ephys передается в Python SDK.
    - *Вопрос*: Проходит ли поток осциллограмм через `ShardEngine`, или отправляется напрямую через IPC сокет в формате `EphysShm`?

#### [test_harness_spec.md](./spec_L3/test_harness_spec.md)
*Source items: 6 / Registered items: 6*

- **REV-TEST-001**: Механизм Отладочного Снятия Состояния (Debug Full-State Snapshot API)
  - *Status*: Resolved (compute-api v2.1) | *Priority*: P1 | *Owner*: `test-harness` | *Duplicate Of*: - | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md)
  - *Question / Problem*: - *Контекст*: Публичный API `compute-api` не предоставлял выгрузку полных SoA-массивов VRAM.
    - *Вопрос*: Требуется ли введение отладочного extension-trait (например, `DebugSnapshotExt`), доступного только под фичей `test-harness`, или формат выгрузки `BatchResult` будет расширен?
  - *Resolution*: В `ComputeBackend` добавлен метод по умолчанию `debug_snapshot(&mut self, handle, snapshot: ShardSnapshotMut<'_>) -> Result<(), ComputeApiError>`, возвращающий `UnsupportedFeature` по умолчанию.

- **REV-TEST-002**: Окончательная Форма Полезной Нагрузки `BatchResult`
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `test-harness` | *Duplicate Of*: REV-COMPUTE-API-004 | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md#L200)
  - *Question / Problem*: - *Контекст*: Структура результатов батча находится на этапе согласования.
    - *Вопрос*: Какие именно поля (число спайков, маски выходов, чексуммы) включаются в `BatchResult` для базового потикового сравнения?

- **REV-TEST-003**: Локализация Пайплайна Генерации и Верификации ABI-Зеркал
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `test-harness` | *Duplicate Of*: REV-PHYS-009 | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md#L204)
  - *Question / Problem*: - *Контекст*: Тесты дрифта ABI-зеркал проверяют совпадение типов между Rust и C++.
    - *Вопрос*: Где именно должен жить генератор C++ заголовков — в крейте `layout`, внутри бэкендов или в виде автономной build-helper утилиты?

- **REV-TEST-004**: Организация Автоматического Запуска Аппаратных Тестов в CI/CD
  - *Status*: Open | *Priority*: P2 | *Owner*: `test-harness` | *Duplicate Of*: - | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md#L208)
  - *Question / Problem*: - *Контекст*: Тестирование CUDA и HIP требует наличия физических GPU и установленных SDK на раннерах.
    - *Вопрос*: Каким образом маркируются тесты для запуска на специализированных CI-раннерах с GPU ускорителями?

- **REV-TEST-005**: Разграничение Дифференциальных и Свойственных/Фаззинг Тестов (Property/Fuzzing Tests)
  - *Status*: Open | *Priority*: P2 | *Owner*: `test-harness` | *Duplicate Of*: - | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md#L212)
  - *Question / Problem*: - *Контекст*: Помимо детерминированных фикстур, эффективен случайный фаззинг входных буферов.
    - *Вопрос*: Должны ли property-based тесты (на базе `proptest` / `quickcheck`) жить внутри `test-harness` или выноситься в отдельный крейт `compute-fuzz`?

- **REV-TEST-006**: Управление Pinned-Буферами Хоста для Снэпшотов
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `test-harness` | *Duplicate Of*: REV-COMPUTE-API-002 | *Source*: [test_harness_spec.md](./spec_L3/test_harness_spec.md#L216)
  - *Question / Problem*: - *Контекст*: Для скоростного снятия дампов VRAM требуется Page-Locked память.
    - *Вопрос*: Кто выделяет и утилизирует Pinned-буферы при отладочном снятии дампов состояний в `test-harness`?

### §5.4. Слой L4 (Baker & Topology Tools)

#### [baker_cli_spec.md](./spec_L4/baker_cli_spec.md)
*Source items: 6 / Registered items: 6*

- **REV-BAKER-CLI-001**: Окончательное Имя Бинарного Файла Утилиты
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: - | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L203)
  - *Question / Problem*: - *Контекст*: В спецификации используется имя `baker-cli`.
    - *Вопрос*: Какое конечное имя бинарника будет утверждено для поставки — `baker-cli`, `axiengine-baker` или единый фасадный исполняемый файл `axiengine`?

- **REV-BAKER-CLI-002**: Единая Схема Протокола JSON-lines для AxiCAD Bridge
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: - | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L207)
  - *Question / Problem*: - *Контекст*: Sidecar-режим генерирует однострочные события.
    - *Вопрос*: Требуется ли вынести общие DTO-структуры событий sidecar в отдельный крейт контрактов сопряжения с AxiCAD?

- **REV-BAKER-CLI-003**: Зависимости Подкоманды Инспектирования (`inspect`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: - | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L211)
  - *Question / Problem*: - *Контекст*: Команда `inspect` читает структуру `.axic` архива.
    - *Вопрос*: Должна ли команда `inspect` обращаться исключительно к хелперам `baker` или может напрямую подключать легкие инспекционные методы `vfs`?

- **REV-BAKER-CLI-004**: Выделение Edge-Конвертора в Отдельный Бинарник
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: - | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L215)
  - *Question / Problem*: - *Контекст*: Подкоманда `edge` выполняет конвертацию моделей.
    - *Вопрос*: Целесообразно ли сохранять подкоманду `edge` в `baker-cli` или вынести ее в отдельный исполняемый файл `edge-cli`?

- **REV-BAKER-CLI-005**: Точный API Отмены (Cancellation Token API) из Библиотеки `baker`
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: - | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L219)
  - *Question / Problem*: - *Контекст*: CLI обрабатывает сигнал SIGINT как процессный останов до появления токена отмены.
    - *Вопрос*: Каким образом компилятор `baker` предоставит атомарные токены отмены для трансляции из `baker-cli`?

- **REV-BAKER-CLI-006**: Централизованное Фиксирование Версий Зависимостей (Workspace-Wide Pinning)
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `baker-cli` | *Duplicate Of*: REV-WIRE-006 | *Source*: [baker_cli_spec.md](./spec_L4/baker_cli_spec.md#L223)
  - *Question / Problem*: - *Контекст*: Версии `clap`, `tracing`, `serde_json` зафиксированы в спецификации.
    - *Вопрос*: Требуется ли централизованный манифест версий зависимостей на уровне всего workspace для исключения дрифта сторонних CLI-крейтов?

#### [baker_spec.md](./spec_L4/baker_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-BAKER-001**: Минимальный Набор Файлов Архива для Загрузки Рантаймом (`boot`)
  - *Status*: Open | *Priority*: P1 | *Owner*: `baker` | *Duplicate Of*: - | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L243)
  - *Question / Problem*: - *Контекст*: Компилятор генерирует дампы состояния, аксонов и путей.
    - *Вопрос*: Каков обязательный минимальный перечень файлов внутри `.axic` архива, необходимый для работы компонента `boot`?

- **REV-BAKER-002**: Статус и Владение Заголовками Файлов I/O (`.gxi`, `.gxo`, `.ghosts`)
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `baker` | *Duplicate Of*: REV-TOPOLOGY-001 | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L247)
  - *Question / Problem*: - *Контекст*: Опциональные файлы входов/выходов и ghost-связей находятся в статусе ожидания (pending debt).
    - *Вопрос*: В каком крейте (`layout` или `wire`) должны объявляться C-ABI заголовки и структуры этих файлов?

- **REV-BAKER-003**: Формат Хранения и Передачи Геометрии Трактов
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker` | *Duplicate Of*: - | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L251)
  - *Question / Problem*: - *Контекст*: `baker` передает в `topology` подготовленную структуру `ResolvedTractGeometry`.
    - *Вопрос*: В каком формате хранится геометрия трактов редактора и как именно выполняется ее первичный резолвинг?

- **REV-BAKER-004**: Инжекция Поля Начального Веса `initial_synapse_weight`
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `baker` | *Duplicate Of*: REV-CFG-004 | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L255)
  - *Question / Problem*: - *Контекст*: Поле начального веса учитывается в `VariantParameters`, но временно отсутствует в TOML-схемах `config`.
    - *Вопрос*: Каким образом значение базового синаптического веса передается из конфигурации проекта в макеты `layout`?

- **REV-BAKER-005**: Централизованное Фиксирование Версий Внешних Зависимостей (Workspace-Wide Pinning)
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `baker` | *Duplicate Of*: REV-WIRE-006 | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L259)
  - *Question / Problem*: - *Контекст*: Версии `tracing` и `tempfile` зафиксированы в спецификации.
    - *Вопрос*: Требуется ли централизованный манифест версий зависимостей на уровне всего workspace для исключения дрифта сторонних крейтов?

- **REV-BAKER-006**: Схема Событий Прогресса Компиляции (Progress Event Schema)
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker` | *Duplicate Of*: - | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L263)
  - *Question / Problem*: - *Контекст*: Внешние GUI-инструменты требуют пошагового отслеживания прогресса сборки.
    - *Вопрос*: Должна ли общая схема событий прогресса объявляться в `baker` или относиться к `baker-cli` / AxiCAD SDK?

- **REV-BAKER-007**: Управление Промежуточными Кекпоинтами Сборки
  - *Status*: Open | *Priority*: P2 | *Owner*: `baker` | *Duplicate Of*: - | *Source*: [baker_spec.md](./spec_L4/baker_spec.md#L267)
  - *Question / Problem*: - *Контекст*: При сборке больших проектов кеширование промежуточных шардов ускоряет повторную компиляцию.
    - *Вопрос*: Выполняется ли кеширование промежуточных результатов внутри `baker` или полностью управляется кешем артефактов AxiCAD?

#### [edge_model_spec.md](./spec_L4/edge_model_spec.md)
*Source items: 10 / Registered items: 10*

- **REV-EDGE-001**: Размещение Конфигурации Edge-Профилей
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L338)
  - *Question / Problem*: - *Контекст*: Параметры конвертации передаются через `EdgeConversionOptions`.
    - *Вопрос*: Должны ли профили edge-конвертации в будущем перенесены в TOML-конфигурации крейта `config`?

- **REV-EDGE-002**: Обязательный Перечень Файлов `.axic` для Edge-Конвертации
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L342)
  - *Question / Problem*: - *Контекст*: Для инференса требуются `.state`, `.axons` и таблица вариантов.
    - *Вопрос*: Требуется ли обязательное наличие файла `.paths` для генерации edge-модели или он опционален?

- **REV-EDGE-003**: Общий Крейт C-ABI Заголовков Встраиваемых Устройств (Firmware-Facing Edge ABI)
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L346)
  - *Question / Problem*: - *Контекст*: `layout` владеет десктопными `.state/.axons/.paths`, а `edge-model` владеет производными `shard.sram` и `shard.flash`.
    - *Вопрос*: Потребуется ли отдельный легкий крейт геометрических контрактов и C-структур для прошивок микроконтроллеров в будущем?

- **REV-EDGE-004**: Выделение Edge-Конвертора в Отдельный CLI-Исполняемый Файл
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: REV-BAKER-CLI-004 | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L350)
  - *Question / Problem*: - *Контекст*: Подкоманда `edge` активируется через Cargo feature в `baker-cli`.
    - *Вопрос*: Целесообразно ли выделение утилиты в отдельный бинарник `edge-cli`?

- **REV-EDGE-005**: Политика Таймеров Дендритов (`dendrite_timers`) при Запуске на Устройстве
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L354)
  - *Question / Problem*: - *Контекст*: В режиме только инференса таймеры рефрактерности могут быть не нужны.
    - *Вопрос*: Следует ли обнулять или копировать исходные `dendrite_timers` при генерации образа SRAM?

- **REV-EDGE-006**: Перспективы Поддержки Пластичности на Устройстве (On-Device Plasticity)
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L358)
  - *Question / Problem*: - *Контекст*: Текущая версия проектируется строго под Pure Inference.
    - *Вопрос*: Каковы архитектурные границы при будущей поддержке ночной фазы на микроконтроллерах?

- **REV-EDGE-007**: Квантование Синаптических Весов (Weight Quantization)
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L362)
  - *Question / Problem*: - *Контекст*: Веса хранятся в Mass Domain (`i32`).
    - *Вопрос*: Требуется ли поддержка квантования весов до типов `i16`, `i8` или `i4` для сверхкомпактных устройств?

- **REV-EDGE-008**: Поддержка Внешней Памяти PSRAM и Нескольких Бюджетов SRAM
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L366)
  - *Question / Problem*: - *Контекст*: Некоторые чипы (ESP32-WROVER) обладают внешней оперативной памятью PSRAM.
    - *Вопрос*: Должен ли `edge-model` поддерживать трехуровневое разделение (SRAM / PSRAM / Flash)?

- **REV-EDGE-009**: Разбиение Огромных Моделей на Каскад Микроконтроллеров
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L370)
  - *Question / Problem*: - *Контекст*: Крупный шардированный граф может не влезать в один чип.
    - *Вопрос*: Каким образом будет осуществляться нарезка одного шарда на несколько MCU?

- **REV-EDGE-010**: Владелец Спецификации Bare-Metal Runtime (`axicor-lite`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `edge-model` | *Duplicate Of*: - | *Source*: [edge_model_spec.md](./spec_L4/edge_model_spec.md#L374)
  - *Question / Problem*: - *Контекст*: `edge-model` является оффлайн-конвертором.
    - *Вопрос*: В каком документе и крейте будет описан рантайм-цикл исполнения устройств (device loop) и прошивки?

#### [topology_spec.md](./spec_L4/topology_spec.md)
*Source items: 5 / Registered items: 5*

- **REV-TOPOLOGY-001**: Унификация Маркеров Пустого Слота (`EMPTY_PIXEL` vs `PackedTarget::None`)
  - *Status*: Open | *Priority*: P1 | *Owner*: `topology` | *Duplicate Of*: - | *Source*: [topology_spec.md](./spec_L4/topology_spec.md#L239)
  - *Question / Problem*: - *Контекст*: Дендритные слоты могут содержать как `None` (сырой нуль), так и `EMPTY_PIXEL` после прунинга.
    - *Вопрос*: Требуется ли принудительная унифицированная миграция всех `None` слотов в `EMPTY_PIXEL` на этапе загрузки шарда?

- **REV-TOPOLOGY-002**: Локализация Деклараций DTO Трактов Редактора
  - *Status*: Open | *Priority*: P2 | *Owner*: `topology` | *Duplicate Of*: - | *Source*: [topology_spec.md](./spec_L4/topology_spec.md#L243)
  - *Question / Problem*: - *Контекст*: Крейт `topology` принимает только подготовленные структурированные геометрии `ResolvedTractGeometry`.
    - *Вопрос*: Где именно должны жить оригинальные декларативные DTO документов трактов — в `config`, в `baker` или в отдельном крейте геометрических контрактов?

- **REV-TOPOLOGY-003**: Отсутствие Поля Начального Веса `initial_synapse_weight` в Конфигурации
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `topology` | *Duplicate Of*: REV-CFG-004 | *Source*: [topology_spec.md](./spec_L4/topology_spec.md#L247)
  - *Question / Problem*: - *Контекст*: При заведении новых синапсов начальный вес рассчитывается с защитой DoA.
    - *Вопрос*: Каким образом параметры базового веса синапсов должны передаваться из TOML конфигурации в `topology`?

- **REV-TOPOLOGY-004**: Разграничение Исполнения Уплотнения (Compaction Execution Ownership)
  - *Status*: Open | *Priority*: P2 | *Owner*: `topology` | *Duplicate Of*: - | *Source*: [topology_spec.md](./spec_L4/topology_spec.md#L251)
  - *Question / Problem*: - *Контекст*: `topology` формирует план уплотнения `CompactionPlan`.
    - *Вопрос*: Должна ли физическая переписка SoA-массивов памяти выполняться внутри `topology` или относиться к рантайм-компоненту `weaver-daemon`?

- **REV-TOPOLOGY-005**: Структура Нейтрального Перехода GhostHandoverDraft
  - *Status*: Open | *Priority*: P2 | *Owner*: `topology` | *Duplicate Of*: - | *Source*: [topology_spec.md](./spec_L4/topology_spec.md#L255)
  - *Question / Problem*: - *Контекст*: При выходе Ghost-аксона за границы шарда формируется промежуточный результат.
    - *Вопрос*: Какую именно форму имеет чистая структура `GhostHandoverDraft` до ее упаковки компонентом `weaver-daemon` в сетевой пакет `wire::AxonHandoverEvent`?

#### [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md)
*Source items: 7 / Registered items: 7*

- **REV-WEAVER-001**: Разграничение Владения `ShmHeader` и `ShmState`
  - *Status*: Open | *Priority*: P0 | *Owner*: `ipc` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L264)
  - *Question / Problem*: - *Контекст*: Структуры заголовков и автомата состояний используются при подключении.
    - *Вопрос*: В каком крейте (`layout` или `ipc`) должны монопольно зафиксироваться структуры `ShmHeader` и представление живого состояния?

- **REV-WEAVER-002**: Локализация Изменяемой Рабочей Копии Геометрии Аксонов (.paths)
  - *Status*: Open | *Priority*: P2 | *Owner*: `weaver-daemon` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L268)
  - *Question / Problem*: - *Контекст*: `.axic` архив является Read-Only, но для динамического роста требуется изменение путей.
    - *Вопрос*: Где именно должна храниться мутабельная рабочая копия путей аксонов во время Ночной Фазы — в дополнительном сегменте SHM или во временном кеше рантайма?

- **REV-WEAVER-003**: Локализация Исполнения Прунинга и Уплотнения
  - *Status*: Open | *Priority*: P2 | *Owner*: `weaver-daemon` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L272)
  - *Question / Problem*: - *Контекст*: В настоящей спецификации уплотнение выполняется на хосте в `weaver-daemon`.
    - *Вопрос*: Должна ли операция уплотнения всегда оставаться хостовой или часть сервисных операций в будущем уходит в сервисный слой вычислений?

- **REV-WEAVER-004**: Локализация DTO-Контрактов Межшардовых Связей (`AxonHandoverEvent` и др.)
  - *Status*: Duplicate Of | *Priority*: P0 | *Owner*: `weaver-daemon` | *Duplicate Of*: REV-WIRE-001 | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L276)
  - *Question / Problem*: - *Контекст*: Структуры переходов упоминаются в `wire` и `topology`.
    - *Вопрос*: В каком крейте (`wire`, `ipc` или отдельном крейте контрактов сопряжения `weaver-core`) должны окончательно разместиться DTO межшардовых переходов?

- **REV-WEAVER-005**: Выбор Рантайма Канала Управления (Async vs Blocking)
  - *Status*: Open | *Priority*: P2 | *Owner*: `weaver-daemon` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L280)
  - *Question / Problem*: - *Контекст*: В спецификации используется `clap` и типизированные каналы.
    - *Вопрос*: Требуется ли подключение асинхронного рантайма (`tokio`) для канала управления или достаточно блокирующих примитивов из `ipc`?

- **REV-WEAVER-006**: Выделение Библиотеки `weaver-core` для Контрактов и Юнит-Тестирования
  - *Status*: Open | *Priority*: P2 | *Owner*: `weaver-daemon` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L284)
  - *Question / Problem*: - *Контекст*: `weaver-daemon` является исполняемым бинарником (`bin`).
    - *Вопрос*: Требуется ли вынести типы DTO-сообщений управления и доменные функции в отдельную библиотеку контрактов `weaver-core` для проведения модульного тестирования без запуска процессов ОС?

- **REV-WEAVER-007**: Дисциплина Самозавершения при Сбоях Родительского Процесса
  - *Status*: Open | *Priority*: P2 | *Owner*: `weaver-daemon` | *Duplicate Of*: - | *Source*: [weaver_daemon_spec.md](./spec_L4/weaver_daemon_spec.md#L288)
  - *Question / Problem*: - *Контекст*: При аварии рантайма демон должен гарантированно завершаться.
    - *Вопрос*: Каковы специфичные механизмы отслеживания падения родительского процесса на платформах Windows и Linux?

### §5.5. Слой L5 (Networking Stack & Protocols)

#### [net_spec.md](./spec_L5/net_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-NET-001**: Монопольное Владение Идентификаторами Нод и Зон
  - *Status*: Open | *Priority*: P1 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L348)
  - *Question / Problem*: - *Контекст*: Идентификаторы нод и зон используются во всех слоях сети.
    - *Вопрос*: В каком именно крейте (`axi-types` или отдельном легком крейте контрактов) должны окончательно зафиксироваться типы `NodeId` и `ZoneId`?

- **REV-NET-002**: Первичный Источник Конфигурации MTU
  - *Status*: Open | *Priority*: P1 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L352)
  - *Question / Problem*: - *Контекст*: Размер MTU фигурирует в профилях маршрутов и настройках адаптера.
    - *Вопрос*: Кто является главным источником эффективного размера пакета при разногласиях между настройкой сокета и маршрутом?

- **REV-NET-003**: Локализация Модуля Телеметрии (Axum/WebSocket)
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L356)
  - *Question / Problem*: - *Контекст*: В v2 сервер телеметрии включен в `axi-net`.
    - *Вопрос*: Следует ли в будущем вынести HTTP/WebSocket сервер телеметрии в отдельный крайний крейт `axi-telemetry-server`?

- **REV-NET-004**: Детализация Семантики ACK для Протокола v3
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L360)
  - *Question / Problem*: - *Контекст*: В v2 используется упрощенный ACK-пакет.
    - *Вопрос*: Какова будет точная семантика подтверждений при появлении `batch_id` и счетчиков в `wire` v3?

- **REV-NET-005**: Шифрование и Аутентификация Межнодового Трафика
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L364)
  - *Question / Problem*: - *Контекст*: Пакеты передаются в открытом виде.
    - *Вопрос*: В каком слое (модуль `axi-net` или надстройка над `axi-transport`) должны выполняться шифрование (TLS/Noise) и аутентификация нод?

- **REV-NET-006**: Протокол Автоматического Обнаружения Нод (Discovery / Bootstrap)
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L368)
  - *Question / Problem*: - *Контекст*: Маршруты задаются через `apply_route_update`.
    - *Вопрос*: Каким образом будет реализован протокол автообнаружения соседних нод при старте кластера?

- **REV-NET-007**: Политика Порядка Байт (Big-Endian vs Little-Endian)
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L372)
  - *Question / Problem*: - *Контекст*: Все заголовки зафиксированы в Little-Endian.
    - *Вопрос*: Требуется ли поддержка авто-конверсии байт при межсетевом обмене с Big-Endian устройствами?

- **REV-NET-008**: Целесообразность Размещения L1-Транспонирования в Net
  - *Status*: Open | *Priority*: P2 | *Owner*: `net` | *Duplicate Of*: - | *Source*: [net_spec.md](./spec_L5/net_spec.md#L376)
  - *Question / Problem*: - *Контекст*: Транспонирование матриц для Python SDK сейчас описано в `axi-net`.
    - *Вопрос*: Должна ли эта операция оставаться в `axi-net` или ее место в адаптере ввода-вывода внешней платформы?

#### [protocol_spec.md](./spec_L5/protocol_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-PROTOCOL-001**: Проектирование Заголовка `SpikeBatchHeaderV3`
  - *Status*: Open | *Priority*: P1 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L237)
  - *Question / Problem*: - *Контекст*: Заголовок v2 не имеет magic-числа, `batch_id` и счетчика событий.
    - *Вопрос*: Какие поля (magic, version, batch_id, total_event_count, checksum) должны войти в следующую версию заголовка спайков в `wire`?

- **REV-PROTOCOL-002**: Согласование Разрядности Эпох (`u32` vs `u64`)
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L241)
  - *Question / Problem*: - *Контекст*: Сетевые заголовки передают эпоху как `u32`, а внутренний `Tick` в `types` имеет разрядность `u64`.
    - *Вопрос*: Какова официальная политика переполнения и экранирования 32-битного сетевого счетчика эпох?

- **REV-PROTOCOL-003**: Обобщение L7-Фрагментации на Другие Типы Пакетов
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L245)
  - *Question / Problem*: - *Контекст*: В текущей версии фрагментация реализована строго для спайковых батчей.
    - *Вопрос*: Требуется ли поддержка L7-фрагментации для внешнего ввода-вывода (`ExternalIo`) и телеметрии?

- **REV-PROTOCOL-004**: Владение Емкостью Буферов Сборки (Reassembly Capacity)
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L249)
  - *Question / Problem*: - *Контекст*: `protocol` принимает внешние буферы сборки.
    - *Вопрос*: Должны ли лимиты емкости задаваться профилем маршрута в `net` или параметрами сокета в `transport`?

- **REV-PROTOCOL-005**: Семантика Повторов и Подтверждений (ACK Semantics)
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L253)
  - *Question / Problem*: - *Контекст*: `protocol` определяет бинарную структуру ACK-пакета.
    - *Вопрос*: Как именно распределяются обязанности по обработке таймаутов и повторов между `protocol` и `transport`?

- **REV-PROTOCOL-006**: Поддержка Архитектур с Big-Endian Порядком Байт
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L257)
  - *Question / Problem*: - *Контекст*: Все контракты зафиксированы в Little-Endian.
    - *Вопрос*: Требуется ли явный конвертер байт для редких Big-Endian устройств или Little-Endian зафиксирован как аппаратный стандарт?

- **REV-PROTOCOL-007**: Локализация Проверки Целостности и Аутентификации
  - *Status*: Open | *Priority*: P2 | *Owner*: `protocol` | *Duplicate Of*: - | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L261)
  - *Question / Problem*: - *Контекст*: Пакеты валидируются на структурный размер.
    - *Вопрос*: В каком крейте (`wire`, `protocol` или будущем модуле безопасности) должны проверяться криптографические подписи или CRC32/CRC64?

- **REV-PROTOCOL-008**: Источник Точного Значения MTU
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `protocol` | *Duplicate Of*: REV-NET-002 | *Source*: [protocol_spec.md](./spec_L5/protocol_spec.md#L265)
  - *Question / Problem*: - *Контекст*: Итератор фрагментации принимает структуру `FragmentSpec`.
    - *Вопрос*: Кто выступает источником MTU — профиль маршрута в `net` или параметры адаптера в `transport`?

#### [transport_spec.md](./spec_L5/transport_spec.md)
*Source items: 8 / Registered items: 8*

- **REV-TRANSPORT-001**: Целесообразность Подключения Крейта `socket2` и Ошибка `DatagramTruncated`
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L273)
  - *Question / Problem*: - *Контекст*: Стандартный `std::net` не предоставляет низкоуровневых флагов детектирования усечения датограмм.
    - *Вопрос*: Требуется ли подключение `socket2` для надёжного детектирования усечения датограмм (`DatagramTruncated`) и низкоуровневых опций сокетов (`SO_BUSY_POLL`, `SO_REUSEPORT`)?

- **REV-TRANSPORT-002**: Первичный Источник Настроек MTU
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `transport` | *Duplicate Of*: REV-NET-002 | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L277)
  - *Question / Problem*: - *Контекст*: MTU используется как в `protocol`, так и в `transport`.
    - *Вопрос*: Кто является первичным источником конфигурации MTU — профиль маршрута в `net` или конфигурация сетевого адаптера в `transport`?

- **REV-TRANSPORT-003**: Локализация Политики Бэкпрешера
  - *Status*: Open | *Priority*: P1 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L281)
  - *Question / Problem*: - *Контекст*: При переполнении очередей транспорт возвращает `QueueFull`.
    - *Вопрос*: Каким образом `net` реагирует на бэкпрешер и где проходит грань между ошибкой сокета и давлением очереди?

- **REV-TRANSPORT-004**: Политика Переподключения TCP-Стримов
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L285)
  - *Question / Problem*: - *Контекст*: При разрыве TCP-соединения сокет переходит в состояние `Stopped` или `SocketClosed`.
    - *Вопрос*: Должна ли повторная установка соединения выполняться сервисами `net` или транспорту требуется автоматический реконнект?

- **REV-TRANSPORT-005**: Перспективы Перехода на Event-Loop (mio / epoll)
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L289)
  - *Question / Problem*: - *Контекст*: В v2 используется модель воркер-тредов ОС.
    - *Вопрос*: Требуется ли в будущем перевод транспортов на event-loop модель (`mio`) для поддержки тысяч соединений?

- **REV-TRANSPORT-006**: Платформозависимые Опции Сокетов (Windows vs Linux)
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L293)
  - *Question / Problem*: - *Контекст*: Поведение сокетов разнится между ОС.
    - *Вопрос*: Каковы специфичные платформенные флаги для оптимизации задержек на Linux и Windows?

- **REV-TRANSPORT-007**: Безопасность Zero-Copy Буферов Между Потоками
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L297)
  - *Question / Problem*: - *Контекст*: Буферы передаются между воркерами через `BufferId`.
    - *Вопрос*: Каким образом обеспечить строгую проверку единственности владения буфером без накладных расходов RCU/Arc?

- **REV-TRANSPORT-008**: Владение Слотом Буфера в `EgressDatagram` / `EgressStreamChunk`
  - *Status*: Open | *Priority*: P2 | *Owner*: `transport` | *Duplicate Of*: - | *Source*: [transport_spec.md](./spec_L5/transport_spec.md#L301)
  - *Question / Problem*: - *Контекст*: При постановке в очередь передается `BufferId`.
    - *Вопрос*: Владеет ли сообщение слотом буфера напрямую или копирует байты в отдельный пул при постановке в очередь?

### §5.6. Слой L6 (Node Runtime & Process Host)

#### [boot_spec.md](./spec_L6/boot_spec.md)
*Source items: 10 / Registered items: 10*

- **REV-BOOT-001**: **Точный список обязательных файлов в `.axic`**: Точный список обязательных файлов в фазе `RequiredFilesResolved` не зафиксирован в спецификациях baker/vfs/config и может определяться динамически на основе манифеста.
  - *Status*: Open | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: - | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L351)

- **REV-BOOT-002**: **Окончательное владение файлами Ghost-связей**: Архитектурный слой для `.gxi`, `.gxo` и `.ghosts` не определен (layout vs topology).
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: REV-TOPOLOGY-001 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L352)

- **REV-BOOT-003**: **Разделение заголовка SHM**: Окончательное владение `ShmHeader`, `ShmState` и `EphysShm` находится на согласовании (layout vs ipc).
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: REV-IPC-001 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L353)

- **REV-BOOT-004**: **Модель инициализации воркеров**: Решение о выборе между Моделью А (Send) и Моделью B (Thread-Affine) зависит от технических возможностей compute бэкендов.
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: REV-COMPUTE-004 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L354)

- **REV-BOOT-005**: **Физическое размещение и контракт BootShardPlan / ShardBootPlan**: Окончательное определение места владения и контракта обмена планами между `boot` и `runtime`.
  - *Status*: Open | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: - | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L355)

- **REV-BOOT-006**: **Материализация сетевого рантайма**: Определить, должен ли `boot` возвращать живой `NetRuntime` или только спецификацию `NetInitPlan` (предпочтительно второе).
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: REV-BOOT-005 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L356)

- **REV-BOOT-007**: **RAM-диск на Windows**: Определение системного механизма памяти для Windows-платформ (virtual RAM-drive).
  - *Status*: Deferred | *Priority*: Deferred | *Owner*: `boot` | *Duplicate Of*: - | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L357)

- **REV-BOOT-008**: **Точки интеграции службы чекпоинтов**: Правила восстановления из чекпоинта VRAM при холодном старте.
  - *Status*: Duplicate Of | *Priority*: Deferred | *Owner*: `boot` | *Duplicate Of*: REV-NODE-004 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L358)

- **REV-BOOT-009**: **Недостающие параметры TOML**: Определение полей `initial_synapse_weight`, а также физических координат сетевых сокетов.
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `boot` | *Duplicate Of*: REV-CFG-004 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L359)

- **REV-BOOT-010**: **Протоколы сетевого автообнаружения**: Начальная геометрия распределения соседей по зонам.
  - *Status*: Duplicate Of | *Priority*: P2 | *Owner*: `boot` | *Duplicate Of*: REV-NET-006 | *Source*: [boot_spec.md](./spec_L6/boot_spec.md#L360)

#### [node_spec.md](./spec_L6/node_spec.md)
*Source items: 10 / Registered items: 10*

- **REV-NODE-001**: **Спецификация команд CLI:** Требуется детализировать синтаксис дополнительных команд командной строки (например, `print-plan`, `validate`, `run` как подкоманды `clap` vs флаги).
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L367)

- **REV-NODE-002**: **Материализация NetRuntime:** Определить, должен ли крейт `node` напрямую вызывать фабрику инициализации сети или же логику материализации следует вынести в отдельный промежуточный крейт композиции.
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `node` | *Duplicate Of*: REV-BOOT-005 | *Source*: [node_spec.md](./spec_L6/node_spec.md#L368)

- **REV-NODE-003**: **Модель владения weaver-daemon:** Определить финальную схему жизненного цикла демона координации: запуск в качестве дочернего процесса самой нодой с отслеживанием PID vs управление внешним супервизором OS (systemd/kubernetes) с проксированием команд.
  - *Status*: Deferred | *Priority*: Deferred | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L369)

- **REV-NODE-004**: **Контракт Checkpoint Service:** Сформировать точный интерфейс взаимодействия ноды со службой записи чекпоинтов (gRPC-клиент, запись в локальный RAM-диск с последующим сбросом или выделенный thread-writer).
  - *Status*: Deferred | *Priority*: Deferred | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L370)

- **REV-NODE-005**: **CPU Affinity Platform Crate:** Выбрать стабильную мультиплатформенную библиотеку для управления привязкой потоков к ядрам процессора и настройки приоритетов процессов на Linux и Windows (например, `raw-cpuid`, `affinity` или платформозависимые OS-API).
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L371)

- **REV-NODE-006**: **Версия библиотеки Tracing:** Согласовать и зафиксировать единую версию `tracing`/`tracing-subscriber` на уровне всего workspace для предотвращения конфликтов дублирования глобального диспетчера логов.
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L372)

- **REV-NODE-007**: **Реэкспорт BackendPreference:** Подтвердить, что `BackendPreference` импортируется нодой через реэкспорт из `boot`, полностью исключая прямую зависимость от `compute`.
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L373)

- **REV-NODE-008**: **Системные режимы OS:** Необходимость поддержки специфических режимов запуска процесса, таких как служба Windows (Windows Service) или демон systemd (уведомления через `sd_notify`).
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L374)

- **REV-NODE-009**: **Контрольный веб-интерфейс:** В чьей зоне ответственности находится запуск HTTP/RPC сервера управления/здоровья (healthcheck): запускается ли он внутри `node` через Tokio или полностью делегирован рантайму `net`.
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L375)

- **REV-NODE-010**: **Типизация сетевых ошибок при материализации:** Согласовать конкретный тип ошибки материализации сетевого рантайма (например, `net::NetError` или специализированный `net::NetInitError`) и интегрировать его в `NodeError` вместо промежуточных текстовых представлений.
  - *Status*: Open | *Priority*: P2 | *Owner*: `node` | *Duplicate Of*: - | *Source*: [node_spec.md](./spec_L6/node_spec.md#L376)

#### [runtime_spec.md](./spec_L6/runtime_spec.md)
*Source items: 4 / Registered items: 4*

- **REV-RUNTIME-001**: **Точная структура полезной нагрузки спайков**: Формат исходящего сетевого контекста доставки спайков не зафиксирован в рантайме. Временное решение: передавать данные пакета через интерфейс `NetRuntime` в виде обобщенных слайсов байт или абстрактного контракта обмена.
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `runtime` | *Duplicate Of*: REV-COMPUTE-API-004 | *Source*: [runtime_spec.md](./spec_L6/runtime_spec.md#L481)

- **REV-RUNTIME-002**: **Механизм записи теневых чекпоинтов**: Архитектурные границы записи VRAM чекпоинтов на диск не утверждены. Не определено, должен ли рантайм генерировать событие внешней асинхронной записи, либо использовать специализированный системный сервис `CheckpointWriter`.
  - *Status*: Duplicate Of | *Priority*: Deferred | *Owner*: `runtime` | *Duplicate Of*: REV-NODE-004 | *Source*: [runtime_spec.md](./spec_L6/runtime_spec.md#L482)

- **REV-RUNTIME-003**: **Окончательный выбор модели инициализации воркеров**: Решение о переходе на эксклюзивную Thread-Affine инициализацию (Модель B) будет принято после закрытия технических вопросов в спецификации `compute_spec.md`.
  - *Status*: Duplicate Of | *Priority*: P1 | *Owner*: `runtime` | *Duplicate Of*: REV-COMPUTE-004 | *Source*: [runtime_spec.md](./spec_L6/runtime_spec.md#L483)

- **REV-RUNTIME-004**: **Синтаксис безопасного API применения ночных изменений**: Конкретные параметры метода `apply_night_delta` в `ShardEngine` остаются на стадии согласования с вычислительным слоем.
  - *Status*: Open | *Priority*: P1 | *Owner*: `runtime` | *Duplicate Of*: - | *Source*: [runtime_spec.md](./spec_L6/runtime_spec.md#L484)
