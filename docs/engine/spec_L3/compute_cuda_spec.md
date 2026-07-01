# spec_compute_cuda

> Версия спеки: v2.3  
> Дата: 2026-07-01  
> Статус: Approved v2.3 / Stage 1R Batch-Native Implemented

---

## §1. Идентификация

| Поле | Значение |
|---|---|
| **Имя крейта** | `compute-cuda` |
| **Слой** | Слой 3 — Вычислительные Бэкенды (Compute Hardware Backends) |
| **Тип** | Library (`lib`) |
| **no_std** | Нет (`false`) — зависит от CUDA Runtime API, FFI-связывания и системного аллокатора кучи |
| **Описание** | NVIDIA CUDA вычислительный бэкенд, реализующий трейт `ComputeBackend` из `compute-api`. Крейт управляет FFI-вызовами CUDA Runtime API, VRAM-ресурсами ускорителя, неблокирующими стримами (`cudaStream_t`), внутренними Pinned Host staging буферами, запуском CUDA-ядер и маппингом ошибок вендора в `ComputeApiError`. Крейт не владеет DTO, макетами памяти, физическими уравнениями или рантайм-планировщиком. |

---

## §2. Стек и Окружение

### §2.1. Внутренние зависимости (inbound)

| Крейт | Что используется | Зачем |
|---|---|---|
| `types` (Слой 0) | Атомарные типы и константы, `PackedTarget::is_inactive()` | Использование типов (`Tick`, `Voltage`, `SomaFlags`, `AXON_SENTINEL`) и проверка неактивных синапсов. |
| `layout` (Слой 1) | Расчет смещений SoA-плоскостей | Нарезка монолитных буферов VRAM с использованием `CACHE_LINE_BYTES` и `PADDED_N_ALIGNMENT`. |
| `physics` (Слой 0) | Математика GLIF, GSOP, Active Tail | Ожидание побитовой идентичности вычислений на GPU с эталоном `physics`. |
| `compute-api` (Слой 3) | `ComputeBackend`, `VramHandle`, DTO вызовов и `ComputeApiError` | Реализация абстрактного контракта вычислений HAL и типизированной иерархии ошибок. |

> [!IMPORTANT]
> Крейт `compute-cuda` содержит C++/CUDA ядра, являющиеся аппаратными зеркалами математических уравнений. Крейт **не является источником истины** по физике и макетам памяти: все C++/CUDA ядра и ABI-заголовки верифицируются из оригинальных крейтов `physics` и `layout`.

### §2.2. Зависимые Компоненты и Характеристики Бэкенда (`BackendCapabilities`)

| Крейт / Компонент | Роль в системе и взаимодействие |
|---|---|
| `compute` (Слой 3) | Фасад вычислений подсоединяет `CudaBackend` через feature flag `cuda` для высокопроизводительного исполнения на NVIDIA GPU. |
| `test-harness` (Слой 3) | Выполняет дифференциальное тестирование результатов `CudaBackend` против `CpuBackend`. |

При вызове метода `capabilities()` структура `CudaBackend` возвращает характеристики CUDA-вычислителя:
```rust
BackendCapabilities {
    lane_count: 32,                // Аппаратный размер варпа NVIDIA (Warp Size)
    supports_async: true,          // Внутренняя поддержка неблокирующих стримов CUDA
    supports_ephys: false,         // Ограничено до фиксации DTO отладчика
    max_batch_ticks: 1000,         // Лимит тиков за один вызов run_day_batch
    alignment_bytes: 64,           // Выравнивание SoA-плоскостей VRAM под L2 кэш
    pinned_host_required: true,    // Требование Page-Locked памяти для скоростного DMA
}
```

### §2.3. Внешние Зависимости

| Crate | Версия | Сфера использования |
|---|---|---|
| `cc` | `=1.2.56` *(build-dep)* | Компиляция CUDA-ядер (`.cu`) с помощью компилятора `nvcc` на этапе сборки. |

---

## §3. Ownership Boundaries (Границы Владения)

| Модуль / Крейт | Монопольная Зона Владения (Single Source of Truth) | Строгие Запреты (Что категорически запрещено в крейте) |
|---|---|---|
| **`compute-cuda`** (Слой 3) | **Реализация CUDA Вычислений**: Структура `CudaBackend`, контекст CUDA, реестр живых физических указателей/аллокаций VRAM за дескрипторами `VramHandle`, внутренние Pinned Host staging буферы (`cudaHostAlloc`), CUDA-стримы (`cudaStream_t`), FFI-врапперы запуска ядер, передачи H2D/D2H и трансляция ошибок вендора в `ComputeApiError`. | Запрещено объявление публичных DTO/трейтов/ошибок (владелец `compute-api`), автовыбор бэкендов (владелец `compute`), объявление C-ABI структуры `ShardVramPtrs` как типа или порядка полей (владелец `layout`), прямое объявление смещений дисковых файлов (владелец `layout`), физические уравнения (владелец `physics`), управление потоками рантайма (`runtime`), а также IPC и сетевая маршрутизация. |
| **`compute-api`** (Слой 3) | **HAL Контракт**: Публичный трейт `ComputeBackend`, структуры DTO и иерархия ошибок `ComputeApiError`. | Запрещена привязка к вендорским типам `cudaStream_t` или CUDA Runtime. |
| **`physics`** (Слой 0) | **Физическая Математика**: Чистые формулы интеграции GLIF, AHP, гомеостаза и пластичности GSOP. | Запрещена привязка к CUDA-указателям и вызовам драйвера. |
| **`layout`** (Слой 1) | **Макеты Памяти и ABI**: Структуры данных (`ShardVramPtrs`, `BurstHeads8`, `VariantParameters`), правила выравнивания и смещения SoA-плоскостей. | Запрещен запуск CUDA-ядер. |

---

## §4. Модель Ресурсов VRAM и Переиспользуемый Scratch

### §4.1. Физическая Стратегия Аллокаций и Pinned Staging
При вызове `alloc_shard(spec: ShardAllocSpec)` бэкенд вычисляет необходимые объемы VRAM на основе формул `layout` и создает `VramHandle` через `VramHandle::from_raw_parts(BackendKind::Cuda, id, generation)`. Аллокация VRAM под шард выполняется строго **двумя монолитными блоками памяти** (Блок 1: Соматические плоскости и синапсы; Блок 2: Аксонные головки `BurstHeads8`), предотвращая фрагментацию памяти видеокарты.

При вызове `upload_shard(handle, upload)` бэкенд осуществляет загрузку состояния и аксонов во VRAM, а также выполняет синхронный H2D-перенос таблицы вариантов `upload.variant_table` в GPU Constant Memory (`__constant__`). 

### §4.2. Переиспользуемая Память (CudaScratch)
Для исключения динамических аллокаций памяти (`cudaMalloc`/`cudaFree`) в горячем пути выполнения батча используется лениво расширяемый буфер `CudaScratch`, ассоциированный с каждым шардом.
- `CudaScratch` владеет плоскими device-буферами для входных и выходных данных батча (`d_input_bitmask`, `d_incoming_spikes`, `d_mapped_soma_ids`, `d_output_spikes`, `d_output_spike_counts`).
- `CudaScratch` также владеет тиковыми скалярами (`d_tick_generated`, `d_tick_written`, `d_tick_dropped`) и итоговыми счетчиками батча, что полностью исключает аллокации внутри FFI-вызовов.
- Очистка всех буферов происходит централизованно при уничтожении шарда (`Drop` для `CudaScratch`).

---

## §5. Модель Выполнения и Порядок Этапов (Execution Model & Stage Order)

Метод `run_day_batch(handle, cmd)` выполняет запуск батча тиков на выделенном CUDA-стриме шарда через **единственный batch-level FFI-вызов** `axi_cuda_run_day_batch_production`. Это устраняет Rust-side per-tick loop и накладные расходы на частые context switches.

### §5.1. Порядок Передачи Данных (Batch I/O Strategy)
- **H2D (Один раз на батч)**: Копирование плоских входных данных `input_bitmask` и `incoming_spikes`, маппингов `mapped_soma_ids` и таблицы параметров `variant_table`.
- **D2H (Один раз после батча)**: Скачивание результирующего плоского буфера `output_spikes`, тиковых объемов спайков `output_spike_counts` и суммарных счетчиков батча (с использованием saturating_add на стороне GPU).

### §5.2. Порядок Выполнения Этапов Внутри Тика (Tick Stage Order на GPU)
Цикл по тикам выполняется полностью на стороне C++/CUDA внутри FFI:
1. **Инъекция Виртуальных Входов**: Сенсорные сигналы инжектируются по смещениям тика из `d_input_bitmask`. При `None` или длине 0 в FFI передается `null`, чтобы исключить использование stale scratch указателя.
2. **Инъекция Входных Спайков**: Спайки распаковываются из `d_incoming_spikes`. Соответствующий `incoming_spike_counts` читается host-side внутри FFI вызова по индексу текущего тика для ограничения лимита инжектируемых спайков. При отсутствии данных передается `null`.
3. **Продвижение Аксонных Головок (Active Tail)**: Продвижение спайков с учетом `v_seg` и маркера `AXON_SENTINEL`.
4. **Обновление Состояния Нейронов (GLIF)**: Интеграция потенциала GLIF, гомеостаза и генерация спайков с записью в плоский `d_output_spikes` со смещением тика.
5. **Применение Синаптической Пластичности (GSOP)**: Обновление весов с использованием GSOP.
6. **Накопление счетчиков**: GPU-ядро `accumulate_tick_counters_kernel` суммирует сгенерированные/записанные/сброшенные спайки с насыщением (`0xFFFFFFFF`).

### §5.3. Защитные Механизмы
- **Checked Output Limits**: Размер выходных спайков вычисляется с помощью `checked_mul` в Rust, предохраняя от переполнения типов.
- **Pre-check Size Arithmetic**: На C++ стороне размеры буферов проверяются на переполнение через деление (`SIZE_MAX / limit`) перед непосредственным вычислением байтового размера.
- **Optional Buffer Safeguards**: Все опциональные и условные буферы при их отсутствии строго заменяются на `null`/`null_mut` при передаче в FFI, предотвращая обращение к данным прошлых запусков.

---

## §6. Параллелизм, Защита от Гонки и Детерминизм

1. **Строгий Побитовый Детерминизм**: Результат вычислений на GPU побитово идентичен `compute-cpu`.
2. **Защита от Гонок Данных во VRAM**: Запись спайков в `axon_heads` ядрами GPU исключает гонки данных во VRAM за счет использования дисюнктной карты `soma_to_axon` или буфера накопления во VRAM.

---

## §7. Трансляция Ошибок Вендора и Политика Безопасности

Все вызовы CUDA API обернуты в проверки семейств ошибок с трансляцией в `ComputeApiError`, прямой вызов паник категорически запрещен:

| Семейство Ошибок CUDA Runtime API | Итоговый Код `ComputeApiError` |
|---|---|
| **Allocation Errors** (`cudaMalloc`) | `ComputeApiError::OutOfMemory` |
| **Copy / DMA Errors** (`cudaMemcpyAsync`) | `ComputeApiError::DmaFailed` |
| **Kernel Launch Errors** | `ComputeApiError::KernelLaunchFailed` |
| **Stream Synchronization Errors** | `ComputeApiError::SynchronizeFailed` |
| **Device Lost / Reset Errors** | `ComputeApiError::DeviceLost` |

---

## §8. Требуемые Инварианты

- **INV-COMPUTE-CUDA-001**: Структура `CudaBackend` имплементирует трейт `ComputeBackend` из `compute-api`.
- **INV-COMPUTE-CUDA-002**: Вызов `kind()` возвращает строго `BackendKind::Cuda`.
- **INV-COMPUTE-CUDA-003**: Выделение VRAM под шард выполняется строго двумя монолитными блоками памяти.
- **INV-COMPUTE-CUDA-004**: Побитовая идентичность вычислений сохраняется с `compute-cpu`.
- **INV-COMPUTE-CUDA-005**: Параллельная запись спайков ядрами GPU исключает гонки данных во VRAM за счет дисюнктной карты `soma_to_axon` или буфера накопления.
- **INV-COMPUTE-CUDA-006**: Публичный API бэкенда на Rust не раскрывает сырые указатели (`*mut u8`), `cudaStream_t` и C-ABI структуры указателей.
- **INV-COMPUTE-CUDA-007**: Все вызовы CUDA API обернуты в проверки семейств ошибок с маппингом в `ComputeApiError`, паники запрещены.
- **INV-COMPUTE-CUDA-008**: CUDA-ядра проверяют неактивные синаптические таргеты строго через `PackedTarget::is_inactive()`.
- **INV-COMPUTE-CUDA-009**: `CudaBackend` не реализует маркерные авто-трейты `Send` и `Sync` (Thread-Affine привязка контекста).

---

## §9. Golden Tests / Обязательная Матрица Тестирования

Крейт `compute-cuda` обязан быть покрыт набором автоматических интеграционных тестов и статических проверок:

1. **Имплементация HAL Трейта (`test_cuda_implements_compute_backend`)**: Проверка реализации `ComputeBackend`.
2. **Идентификация Бэкенда (`test_cuda_backend_kind`)**: Проверка возврата `BackendKind::Cuda`.
3. **Проверка Характеристик (`test_cuda_backend_capabilities`)**: Проверка точности структуры `BackendCapabilities` (§2.2).
4. **Разграничение Инициализации и Потери Устройства (`test_cuda_device_lost_vs_unavailable`)**: Проверка того, что отсутствие GPU на фазе пробинга обрабатывается фасадом как `BackendUnavailable`, а аппаратный сбой во время работы рантайма возвращает `DeviceLost`.
5. **Браковка Неверных Размеров до FFI (`test_cuda_upload_rejects_bad_sizes`)**: Проверка возврата `SizeMismatch` до обращения к CUDA API.
6. **Защита от Невалидных Дескрипторов (`test_cuda_rejects_invalid_handles`)**: Проверка обработки битых `VramHandle`.
7. **Изоляция Публичного API (`test_cuda_no_raw_pointers_in_api`)**: Компиляционная проверка сигнатур на отсутствие сырых указателей и вендорских типов.
8. **Падение Сборки при Рассогласовании ABI Зеркал (`test_cuda_abi_mirror_drift_prevention`)**: Тест компиляции/верификации сгенерированного CUDA-заголовка, падающий при любом расхождении размеров, выравнивания или полей с Rust-крейтами.
9. **Загрузка Таблицы Вариантов в Constant Memory (`test_cuda_constant_upload_api`)**: Верификация H2D загрузки таблицы вариантов `upload.variant_table` в GPU Constant Memory (`__constant__`) при вызове `upload_shard`.
10. **Ограниченность Физических Аллокаций (`test_cuda_bounded_allocations`)**: Верификация вызова ровно 2 физических аллокаций VRAM на шард через Mock/Stub FFI.
11. **Совпадение Порядка Этапов с CPU (`test_cuda_stage_order_matches_cpu`)**: Дифференциальный тест последовательности вычислений на эталонном фикстурном шарде.
12. **Идемпотентность Teardown и Уничтожение Стримов (`test_cuda_idempotent_teardown`)**: Проверка уничтожения `cudaStream_t` и безопасного повторного вызова `teardown()`.
13. **Контроль Угрозы Смещения ABI констант (`test_cuda_constants_generated_match`)**: Статическая верификация того, что все константы (`AXON_SENTINEL`, `EMPTY_PIXEL`, лимиты весов) соответствуют значениям из Rust.
14. **Математика Скалярных Ядер (`test_cuda_scalar_physics_golden_vectors`)**: Автономные unit-тесты CUDA-ядер против оригинальных Rust-вычислений (`physics`): `propagate_head`, `active_tail_hit`, `update_glif_voltage`, `is_glif_spike`, `heartbeat_spike`, `homeostasis_decay`, `weight_to_charge`, `inertia_rank`, `apply_gsop_plasticity`.
15. **Интеграционный Сквозной Дифференциальный Тест (`test_cuda_differential_runner`)**: Запуск прогона ConformanceFixture на CPU бэкенде и CUDA бэкенде через `test-harness`.
16. **Проверка Поведения Фасада Вычислений (`test_cuda_facade_behavior_policy`)**: Проверка того, что при отключенной фиче `cuda` фасад возвращает `FeatureNotCompiled`, а при включенной фиче, но отсутствии GPU — `BackendUnavailable` без тихого фолбэка, в то время как Auto-режим корректно переключается на CPU.
17. **Пакетное Дифференциальное Тестирование (`test_cuda_native_run_day_batch_multi_tick_cpu_differential`)**: Верификация побитового совпадения результатов CPU и GPU при многотиковом пакетном выполнении батча.
18. **Переиспользование Scratch Памяти (`test_cuda_native_run_day_batch_scratch_reuse`)**: Подтверждение переиспользования выделенной памяти `CudaScratch` и отсутствия `cudaMalloc` в горячем пути.
19. **Тест Stale Bitmask и Axon Snapshot (`test_cuda_native_run_day_batch_stale_bitmask`)**: Верификация очистки опциональных указателей во FFI, исключающая повторные инъекции в головы аксонов при передаче `None`.
20. **Защитные Cast Guards (`test_cuda_native_run_day_batch_validation_errors`)**: Проверка возврата ошибок `CapacityExceeded` при переполнениях размеров буферов на Rust-стороне и делений на стороне C++.

---

## §10. Open Questions / Review Debt (Открытые Вопросы и Противоречия)

Все архитектурные и интеграционные вопросы по крейту `compute-cuda` были полностью разрешены в рамках прохода ревью Pass 2.3. Блокирующие вопросы по текущей реализации отсутствуют.

---

## §11. Resolved Architectural Decisions (Принятые Решения Pass 2.3)

1. **[RESOLVED] API и DTO Загрузки Таблицы Вариантов в Constant Memory (REV-COMPUTE-CUDA-002 / Pass 2.2)**:
   - *Решение*: В DTO `ShardUpload` добавлено фиксированное заимствованное поле `variant_table: &'a [VariantParameters; VARIANT_LUT_LEN]`. Таблица вариантов синхронно передается на GPU во время `upload_shard` и размещается в CUDA Constant Memory (`__constant__`).

2. **[RESOLVED] Механизм Кодогенерации и Верификации ABI Зеркал (REV-COMPUTE-CUDA-001 / Pass 2.3)**:
   - *Решение*: В Stage 1 полноценные вычислительные алгоритмы CUDA пишутся вручную. Для исключения дублирования констант и ABI-структур, файл `build.rs` крейта `compute-cuda` генерирует во время сборки в `OUT_DIR` C-совместимый заголовочный файл `generated/axi_cuda_abi.h`, используя Rust-зависимости `types`, `layout` и `physics` в качестве единого источника истины. Заголовок содержит:
     - Размеры и выравнивания (`align`/`size_of`): `VariantParameters`, `BurstHeads8`, `StateFileHeader`, `AxonsFileHeader`, `PathsFileHeader`, `ShardVramPtrs`.
     - Константы раскладки: `CACHE_LINE_BYTES`, `PADDED_N_ALIGNMENT`.
     - Константы физики/типов: `AXON_SENTINEL`, `EMPTY_PIXEL`, `MIN_WEIGHT_LIMIT`, `MAX_WEIGHT_LIMIT` и коэффициенты DDS.
     CUDA/C++ код подключает данный заголовок и использует эти значения, ручное дублирование запрещено. Полнота математики тестируется через скалярные golden-тесты ядер.

3. **[RESOLVED] Аффинность Потоков ОС и Потокобезопасность (REV-COMPUTE-CUDA-005 / Pass 2.3)**:
   - *Решение*: Крейт `CudaBackend` и его внутренние ресурсы (контекст CUDA, стримы) являются строго `!Send` и `!Sync`. Все операции инициализации, запуска ядер, переноса памяти и teardown выполняются строго в рамках одного системного OS-потока шарда-владельца. Это полностью согласуется с `compute` v2.2.

4. **[RESOLVED] Владение Операциями sort_and_prune и Ghost-синхронизацией (REV-COMPUTE-CUDA-006 / Pass 2.3)**:
   - *Решение*: Уплотнение синапсов (`sort_and_prune`), межшардовые патчи спайков и прочие фоновые/ночные операции перенесены на уровень рантайма/сети и исключены из рамок ответственности `ComputeBackend` в Stage 1. Бэкенд реализует исключительно базовые методы HAL API.

---

## §12. Future Implementation Work / Next Stage (Направления Оптимизации)

Ниже перечислены неблокирующие направления развития CUDA-бэкенда для последующих этапов:
1. **Параллелизация скалярных GPU-ядер**: Перевод последовательных вычислений (таких как `apply_glif_final_spike_probe_kernel` и `apply_gsop_plasticity_probe_kernel`) на параллельное выполнение силами GPU-нитей.
2. **Асинхронные CUDA Streams**: Введение неблокирующего перекрытия операций копирования памяти (H2D/D2H) и вычислений с помощью CUDA-стримов (сейчас вызовы синхронизируются на уровне хоста в конце батча).
3. **Device-side Handling для incoming_spike_counts**: Перенос обработки объема приходящих спайков на GPU-сторону для полного отказа от host-side цикла тиков внутри FFI-метода.
4. **Границы HAL**: Сохранение принципа невмешательства вычислительного бэкенда в задачи прунинга синапсов, ghost-синхронизации и распределенной сети.
