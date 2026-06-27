# Спецификация предметного режима выполнения симуляции и инференса Inference Runtime Workspace (Inference Runtime Workspace Spec)

> Этот документ формально определяет архитектурный контракт предметного режима динамического выполнения, мониторинга и визуальной отладки нейросетевого инференса (Inference Runtime Workspace) на стороне 3D-редактора AxiCAD. Спецификация регламентирует интерактивное наблюдение за электрической активностью сети, управление внешними потоками ввода/вывода (IO), подключение осциллографов и зондов, а также обработку специфичных рантайм-диагностик.

## Status: Draft

---

## 1. Назначение документа (Scope & Non-scope)

Данная спецификация определяет границы ответственности и механизмы взаимодействия редактора AxiCAD при отладке и запуске нейросетевого инференса в реальном времени.

### Назначение (Scope)
- **Управление сессией симуляции (Runtime Session Controls)**: Операции запуска (`run`), паузы (`pause`), пошагового исполнения тиков (`step`), полной остановки (`stop`), сброса состояний (`reset`) и воспроизведения захваченных потоков (`replay`).
- **Конфигурирование внешних интерфейсов (External IO Bindings)**: Настройка связей внешних портов модели с реальными датчиками, видеопотоками, аудио-каналами или синтетическими генераторами стимулов.
- **Мониторинг зондов и метрик (Probes & Metrics Inspection)**: Отображение динамических графиков осциллографов, вольтметров и спайковых частот в реальном времени.
- **Визуализация рантайм-слоев (Runtime Overlays)**: Отрисовка волн потенциалов сом, движущихся импульсов в трактах связей и теплокарт активности в 3D-вьюпорте.
- **Перехват диагностик симулятора (Runtime Diagnostics)**: Отслеживание перегрузок тикового бюджета, сбоев связи с мостом и рантайм-ошибок.

### Вне зоны ответственности (Non-scope)
- Документ **не описывает** математику нейронов, уравнение мембранного потенциала и низкоуровневые кернелы Rust в вычислительном ядре AxiEngine.
- Документ **не является** спецификацией алгоритмов нейронного обучения или пластичности (training / learning mechanisms).
- Документ **не разрабатывает** низкоуровневые драйверы физического оборудования (PCI-плат, камер, робототехнических контроллеров).
- Документ **запрещает** прямое ручное редактирование TOML-файлов или канонического графа редактора во время активной фазы симуляции.

---

## 2. Главный принцип (Main Principle)

Архитектура предметного режима динамического выполнения строго подчиняется следующей фундаментальной формуле:

> **Inference Runtime Workspace observes and debugs active network execution. AxiEngine executes. AxiCAD controls, connects, visualizes, and monitors. Canonical TOML remains immutable during runtime.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                            AxiEngine Runtime Core                             │
│       (Executes neural dynamics, processes IO streams & streams frames)       │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Stream InferenceFrames & Diagnostics
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│              AxiCAD Inference Runtime Workspace Manager                       │
│      (Manages RuntimeBindings, ProbeSeries, Timeline & Input Captures)        │
└──────────────────┬────────────────────────────────────────┬───────────────────┘
                   │ Render Handoff (Read-Only)             │ UI Display Only
                   ▼                                        ▼
┌──────────────────────────────────────┐  ┌─────────────────────────────────────┐
│          Rendering Pipeline          │  │     Oscilloscopes & Metric HUDs     │
│runtimeLayer: Active Spikes & Heatmaps│  |─────────────────────────────────────|
│hudLayer: Throughput & Latency HUDs   │  │ Read-Only charts, Store is unchanged│
└──────────────────────────────────────┘  └─────────────────────────────────────┘
```

1. **AxiEngine исполняет**: Вычислительное ядро производит канонический расчёт нейронной динамики и прокачку импульсов через сокеты и тракты.
2. **AxiCAD наблюдает, связывает и мониторит**: Редактор управляет командным потоком плеера, связывает внешние порты с физическими источниками данных и визуализирует осциллограммы.
3. **Иммутабельность канона**: Вход в режим симуляции не изменяет каноническую структуру TOML-документов. Все динамические показатели являются строго временными сессионными данными.

---

## 3. Режимы входных данных (Runtime Input Modes)

Подсистема инференса поддерживает 4 базовых режима подачи входных сигналов на внешние порты ввода:

- **`live external inputs`**: Подача физических сигналов в реальном времени с подключенных камер, микрофонов или сетевых сокетов. Режим является интерактивным и недетерминированным.
- **`captured input replay`**: Воспроизведение ранее записанного файла входных стимулов (`inputCaptureId`). Может обеспечивать детерминированный повтор только при совпадении полного контекста `ReplayDescriptor`: `snapshotId`, `randomSeed`, `engineBuildHash`, `schemaVersion`, `protocolVersion`, `compiledArtifactHash`, `runtimeInputBindingHash` и `inputCaptureId`/`recorded input stream`.
- **`synthetic/stimulus fixtures`**: Генерирование процедурных сигналов (синусоиды, белого шума, меандра) для изолированного тестирования отклика отдельных департаментов.
- **`detached/no-input mode`**: Автономная симуляция фоновой активности сети без подвода внешних стимулов.

---

## 4. Основные сущности и DTO (Core Entities DTOs)

Для управления сессиями инференса и визуализацией активности в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type RuntimeInputMode = 
  | 'live-external-inputs'
  | 'captured-input-replay'
  | 'synthetic-fixtures'
  | 'detached-no-input';

export type RuntimeExecutionState = 
  | 'uninitialized'
  | 'initializing'
  | 'ready'
  | 'running'
  | 'paused'
  | 'stepping'
  | 'stopping'
  | 'stopped'
  | 'error'
  | 'stale';

export interface RuntimePortBindingSummary {
  portId: string;
  direction: 'input' | 'output' | 'bidirectional';
  targetEndpointRef: string;
  boundSourceUri?: string; // Volatile/redacted/session-only URI (Never saved in canonical TOML)
  displayLabel?: string;
  bindingProfileId?: string;
  isRequiredForRun?: boolean;
  status: 'connected' | 'disconnected' | 'error';
}

export interface StimulusSource {
  sourceId: string;
  name: string;
  fixtureType: 'sine-wave' | 'pulse-train' | 'white-noise' | 'custom-generator';
  frequencyHz: number;
  amplitude: number;
}

export interface InferenceRunConfig {
  configId: string;
  inputMode: RuntimeInputMode;
  targetTickRateHz: number;
  enableInputCapture: boolean;
  activeStimuli: StimulusSource[];
  losslessRecording: boolean;
  randomSeed?: number;
  snapshotId?: string;
  inputCaptureId?: string;
  runtimeInputBindingHash?: string;
  runConfigHash?: string;
}

export interface SpikeEvent {
  tick: number;
  sourceSomaId: string;
  tractId?: string;
  amplitude: number;
}

export interface SomaStateSample {
  somaId: string;
  membranePotential: number;
  isSpiking: boolean;
}

export interface PortSample {
  portId: string;
  tick: number;
  dataVector: number[];
}

export interface InferenceFrame {
  frameIndex: number;
  tick: number;
  timestampMs: number;
  spikes: SpikeEvent[];
  somaSamples: SomaStateSample[];
  portSamples: PortSample[];
  executionDurationMs: number;
}

export interface RuntimeMetricSummary {
  currentFps: number;
  ticksPerSecond: number;
  activeSpikeCount: number;
  averageLatencyMs: number;
  memoryUsageBytes: number;
}

export interface InferenceSessionRef {
  sessionId: string;
  projectId: string;
  compiledArtifactHash: string;
  engineBuildHash: string;
  schemaVersion: string;
  protocolVersion: string;
  state: RuntimeExecutionState;
  config: InferenceRunConfig;
  activeBindings: RuntimePortBindingSummary[];
  metrics: RuntimeMetricSummary;
}

export interface InferenceWorkspaceState {
  activeSession?: InferenceSessionRef;
  selectedProbeIds: string[];
  focusedDepartmentId?: string;
  isOscilloscopeExpanded: boolean;
}
```

---

## 5. Интеграция с AxiEngine Bridge

Взаимодействие с подсистемой выполнения осуществляется через отправку канонических команд в `AxiEngine Bridge`:

- `create_inference_session(compiledArtifactHash, runConfig)`: Инициализация сессии инференса на основе запеченного артефакта.
- `bind_runtime_ports(sessionId, bindings)`: Привязка внешних портов к физическим источникам стимулов.
- `start_inference(sessionId)`: Перевод сессии в режим непрерывного расчета тиков `running`.
- `pause_inference(sessionId)`: Перевод сессии в состояние паузы `paused`.
- `step_ticks(sessionId, tickCount)`: Просчет фиксированного количества тиков в режиме отладки.
- `stop_inference(sessionId)`: Остановка расчета и сохранение итоговой статистики.
- `reset_inference(sessionId)`: Сброс мембранных потенциалов сом и внутренних буферов к начальному состоянию.
- `dispose_inference_session(sessionId)`: Освобождение ресурсов вычисления в движке.
- `capture_runtime_inputs(sessionId, captureOptions)`: Включение фиксации входного потока стимулов в бинарный файл кадра.

---

## 6. Интеграция с подсистемами времени, зондов и внешних портов

Режим Inference Runtime Workspace интегрирует фундаментальные сервисы платформы:

### Интеграция с Runtime Timeline & Probe:
- **Управление временем (`PlaybackState`)**: Кнопки плеера GUI напрямую связаны с вызовами `play/pause/step/scrub`.
- **Осциллографы и метрики (`ProbeDefinition` / `ProbeSeries`)**: Зонды снимают показания мембранных потенциалов (`membrane-potential`) и спайков (`spike-event`), транслируя их в графики на HUD.
- **Детерминированный повтор (`ReplayDescriptor`)**: Режим детерминированного повтора задействуется и гарантируется **только** при полном совпадении всего комплекса контекстных параметров: `snapshotId`, `randomSeed`, `engineBuildHash`, `schemaVersion`, `protocolVersion`, `compiledArtifactHash`, `runtimeInputBindingHash` и при наличии сохраненного файла входных стимулов (`inputCaptureId` / recorded input stream). При работе с живыми физическими сигналами в реальном времени (`live-external-inputs`) без захвата потока детерминизм воспроизведения не гарантируется.

### Интеграция с External Port IO:
- Порты модели (`InputPort` / `OutputPort`) привязываются к сессионным оберткам `RuntimeBinding`.
- Сессионные привязки оборудования (IP-адреса, COM-порты, токены) **не сериализуются** в канонический `model.toml`.
- **Правила валидации привязок**: Отсутствие привязки `AXI-INF-001` блокирует запуск симуляции `run-simulation` только для обязательных портов (`isRequiredForRun === true`) с направлением `input` или `bidirectional` в режимах, требовательных к наличии сигналов (`live-external-inputs` / `captured-input-replay`). Режимы `detached-no-input`, процедурные генераторы `synthetic-fixtures`, необязательные порты и `output-only` привязки не блокируют запуск симуляции.

---

## 7. Визуализация и слои рендеринга (Rendering Layers)

Визуальное сопровождение инференса распределяется по слоям подсистемы `Rendering Pipeline`:

| Слой рендеринга | Отображаемый контент и поведение | Режим доступа |
|---|---|---|
| **`runtimeLayer`** | Динамическая анимация спайковых вспышек, волн мембранных потенциалов сом, импульсов в трактах связей и оверлеев теплокарт (Heatmaps/Flow). | Read-Only (3D Animated) |
| **`hudLayer`** | Вывод 2D-вида осциллографов, показателей задержек (Latency), пропускной способности (Throughput) и панели плеера. | Read-Only (2D Overlays) |
| **`diagnosticOverlay`** | Цветовое подсвечивание перегруженных каналов, сброшенных кадров и рантайм-предупреждений. | Read-Only (Warnings) |

---

## 8. Разграничение и владение данными (Data Ownership & Storage)

Изоляция канонического описания сети от динамических сессионных данных строго соблюдает следующий регламент:

| Тип данных | Место хранения | Канонический статус | Описание |
|---|---|---|---|
| **Биологическая модель** | `model.toml` / `shard.toml` | **Canonical Source** | Каноническая структура сети. Симуляция **никогда не модифицирует** эти файлы. |
| **Настройки воркспейса** | `axicad.project.json` | Project-Scoped Config | Раскладка панелей инференса, списки выбранных зондов и метаданные рантайм-привязок. |
| **Рантайм-кэш и записи** | Temp Artifact Cache / Memory | Transient / Session Cache | Буферы кадров (`InferenceFrame`), файлы захвата стимулов (`inputCaptureId`) и временные ряды зондов. |

> [!NOTE]
> Записанные рантайм-кадры (`InferenceFrame`), захваты входов (`inputCaptureId`), выборки зондов (`ProbeSeries`) и сборы метрик являются строго производными сессионными артефактами (`derived session artifacts`) и ни при каких условиях не становятся частью канонических документов TOML. Любые предложенные движком или принятые пользователем структурные изменения биологической модели применяются исключительно через командный слой `Command Mutation` / `PatchSet` и помечают `toml_documents_dirty` и `dirty_entities` в Store.

---

## 9. Семантика устаревания (Stale Semantics)

Активная рантайм-сессия переходит в состояние `stale` (устаревшая) при наступлении любого из следующих событий:

1. **Мутация реактивного хранилища (`storeRevision`)**: Любое изменение структуры модели в Store.
2. **Перезапекание сети (`compiledArtifactHash`)**: Смена скомпилированного бинарного артефакта Baker.
3. **Обновление бинарника движка (`engineBuildHash`)**: Изменение версии ядра AxiEngine.
4. **Смена версий схемы или протокола**: Изменение `schemaVersion` или `protocolVersion`.
5. **Изменение привязок портов (`runtimeInputBindingHash`)**: Перекоммутация внешних физических устройств.
6. **Изменение параметров запуска**: Смена зерна `randomSeed`, целевой частоты тиков или состава активных стимулов.

---

## 10. Каталог диагностик рантайма (Runtime Diagnostics AXI-INF-*)

Отклонения и сбои в процессе выполнения симуляции транслируются через канонические объекты `DiagnosticItem`:

### Каталог диагностик инференса:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-INF-001` | `unresolved runtime port binding` | `'error'` | `'run-simulation'` | Обязательный порт ввода (`isRequiredForRun`) не привязан к физическому источнику в режиме live/captured inputs. |
| `AXI-INF-002` | `stale compiled artifact` | `'error'` | `'run-simulation'` | Скомпилированный артефакт Baker устарел по отношению к текущей модели. |
| `AXI-INF-003` | `bridge session unavailable` | `'error'` | `'run-simulation'` | Потеряна связь с сессией моста вычислительного ядра AxiEngine. |
| `AXI-INF-004` | `unsupported runtime metric` | `'warning'` | `None` / `'run-simulation'` | Выбранная метрика не поддерживается сборкой движка; отключает конкретную метрику (блокирует run-simulation только если метрика объявлена как обязательная `isRequired`). |
| `AXI-INF-005` | `live input is non-deterministic` | `'info'` | `None` | Уведомление о недетерминированном характере симуляции при `live inputs`. |
| `AXI-INF-006` | `input capture mismatch` | `'error'` | `'run-simulation'` | Хэш структуры модели не совпадает с заголовком файла захвата стимулов. |
| `AXI-INF-007` | `frame stream dropped` | `'warning'` | `None` | Пропуск кадров в рендере из-за превышения пропускной способности канала. |
| `AXI-INF-008` | `runtime overload / tick budget exceeded` | `'warning'` | `None` | Время просчета тика превышает целевой интервал дискретизации. |
| `AXI-INF-009` | `inference session crashed` | `'error'` | `'run-simulation'` | Критический сбой симулятора внутри вычислительного ядра AxiEngine. |

---

## 11. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [axiengine-bridge-session-spec-ru](axiengine-bridge-session-spec-ru.md) — Спецификация моста интеграции и менеджера сессий.
- [baker-compile-pipeline-spec-ru](baker-compile-pipeline-spec-ru.md) — Спецификация пайплайна подготовки и компиляции Baker.
- [external-port-io-spec-ru](external-port-io-spec-ru.md) — Спецификация внешних портов ввода/вывода.
- [runtime-timeline-probe-spec-ru](runtime-timeline-probe-spec-ru.md) — Спецификация контроллера времени, зондов и метрик симуляции.
- [rendering-pipeline-spec-ru](rendering-pipeline-spec-ru.md) — Спецификация визуального слоя рендеринга.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.
- [editor-store-spec-ru](editor-store-spec-ru.md) — Спецификация реактивного хранилища Store.
- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.

---

## 12. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации предметного режима выполнения симуляции и инференса Inference Runtime Workspace Spec. Определены DTO сущности, 4 режима ввода, команды управления сессией, слои рендеринга и каталог диагностик AXI-INF. |
| 2026-06-27 | 0.1.1 | Точечные доработки: исправлена опечатка ("запуске"), синхронизированы правила детерминированного повтора с таймлайном, расширены DTO `InferenceRunConfig`, `InferenceSessionRef` и `RuntimePortBindingSummary` (`isRequiredForRun`), добавлены явные правила Data Ownership (derived session artifacts), уточнена грамматика AXI-INF-006 и правила блокировок AXI-INF-001/004. |
