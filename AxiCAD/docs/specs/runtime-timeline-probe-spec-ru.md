# Спецификация контроллера времени, зондов и метрик симуляции (Runtime Timeline & Probe Spec)

> Этот документ формально определяет архитектурный контракт управления временной шкалой (Timeline), воспроизведением сессий, сбором временных рядов зондов (Probes), стримингом метрик и записью фреймов симуляции на стороне 3D-редактора AxiCAD. Спецификация задает единую платформу вычисления времени для предметных режимов отладки симуляции (Growth Workspace) и выполнения нейросетевого рантайма (Inference Runtime Workspace).

## Status: Draft

---

## 1. Назначение документа (Scope & Non-goals)

Данная спецификация определяет правила учета времени, записи и визуализации динамических показателей симуляции сети в AxiCAD.

### Назначение (Scope)
- **Управление воспроизведением (Playback Controls)**: Операции запуска (`play`), паузы (`pause`), пошагового исполнения (`step`), масштабирования скорости (`speed`) и перехода по тикам (`scrub`).
- **Индексация симуляционного времени (Tick & Frame Indexing)**: Учет времени в дискретных симуляционных тиках (`simulation ticks`), а не системных часах астрономического времени.
- **Детерминированное воспроизведение (Deterministic Replay)**: Точный повтор результатов при фиксированных зернах генерации (`randomSeed`) и бинарниках движка.
- **Конфигурация измерительных зондов (Probe Definitions)**: Визуальная настройка виртуальных датчиков и осциллографов для съема локальных показателей сети.
- **Буферизация и хранение каскадов кадров (Recording Buffers)**: Ограничение размера и оптимизация хранения записанных сессий симуляции.
- **Передача в слои визуализации (Rendering Handoff)**: Трансляция временных рядов зондов в `hudLayer` и анимированных состояний в `runtimeLayer`.

### Вне зоны ответственности (Non-goals)
- Документ **не описывает** математику и алгоритмы биологического роста (морфогенеза) или алгоритмы расчёта инференса.
- Документ **не дублирует** спецификацию внешних портов ввода/вывода External Port IO Spec.
- Документ **не делает** записанные кадры или сэмплы зондов каноническими TOML-данными.

---

## 2. Главный принцип (Main Principle)

Управление временем и мониторингом процессов подчиняется следующей фундаментальной формуле:

> **Timeline drives simulation ticks, not wall-clock. Recorded frames and probes are derived session artifacts. Store and TOML remain immutable during playback.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                          AxiEngine Simulation Core                            │
│             (Advances discrete ticks, calculates state & metrics)             │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Stream TimelineFrames & ProbeSamples
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                    AxiCAD Timeline & Probe Session Manager                    │
│          (Manages PlaybackState, RecordingBuffer & ProbeSeries Cache)         │
└──────────────────┬────────────────────────────────────────┬───────────────────┘
                   │ Render Handoff (Read-Only)             │ UI Display Only
                   ▼                                        ▼
┌──────────────────────────────────────┐  ┌─────────────────────────────────────┐
│          Rendering Pipeline          │  │     Oscilloscopes & Metric HUDs     │
│ runtimeLayer: Animated State         │  ├─────────────────────────────────────┤
│ hudLayer: Probe Charts & Oscilloscope│  │ Read-Only charts, Store is unchanged│
└──────────────────────────────────────┘  └─────────────────────────────────────┘
```

1. **Дискретные тики вместо системных часов**: Источником истины для плеера редактора является номер симуляционного тика (`simulation tick`). Системное астрономическое время (`wall-clock`) используется только для интерполяции в рендере.
2. **Результаты зондов являются производными**: Записанные кадры и выборки метрик являются сессионными данными (`derived artifacts`) и никогда не сериализуются в канонический TOML.
3. **Иммутабельность Store при воспроизведении**: Любые действия плеера (воспроизведение, пауза, перемотка) являются strictement просматривательными и ни при каких условиях не мутируют состояние редактора Store.

---

## 3. Основные сущности и DTO (Core Entities DTOs)

Для управления временной шкалой, зондами и записью кадров в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type PlaybackMode = 'realtime-stream' | 'step-by-step' | 'deterministic-replay';

export interface PlaybackState {
  isPlaying: boolean;
  isPaused: boolean;
  currentTick: number;
  currentFrameIndex: number;
  speedMultiplier: number; // e.g. 0.25, 1.0, 2.0, 8.0
  mode: PlaybackMode;
}

export interface ReplayDescriptor {
  replayId: string;
  randomSeed: number;
  engineBuildHash: string;
  snapshotId: string;
  schemaVersion: string;
  protocolVersion: string;
  compiledArtifactHash: string;
  runtimeInputBindingHash?: string;
  inputCaptureId?: string;
  initialParameters: Record<string, unknown>;
}

export interface ProbeDefinition {
  probeId: string;
  name: string;
  targetEntityType: 'soma' | 'synapse' | 'tract' | 'socket' | 'port';
  targetTypedPath: string; // Stable logical path (e.g. @model/departments/Visual/shards/S1)
  targetEntityId?: string; // Derived runtime UUID (Not saved as primary binding in project config)
  metricType: 'membrane-potential' | 'spike-event' | 'growth-velocity' | 'current-flow' | 'custom-metric';
  samplingIntervalTicks: number;
  colorHex: string;
}

export interface ProbeSample {
  tick: number;
  timestampMs: number;
  value: number;
  vectorValue?: number[];
}

export interface ProbeSeries {
  probeId: string;
  samples: ProbeSample[];
  minValue: number;
  maxValue: number;
  averageValue: number;
}

export interface MetricStream {
  streamId: string;
  name: string;
  activeProbes: string[];
  bufferCapacitySamples: number;
}

export interface TimelineFrameRef {
  frameIndex: number;
  tick: number;
  timestampMs: number;
  hasDiagnostics: boolean;
  payloadByteLength: number;
}

export interface RecordingBuffer {
  bufferId: string;
  sessionId: string;
  totalFrames: number;
  startTick: number;
  endTick: number;
  memoryUsageBytes: number;
  storageKind: 'memory' | 'temp-artifact' | 'project-cache';
  artifactRef?: string;
  tempPath?: string;
  checksumSha256?: string;
  retentionPolicy: 'keep-forever' | 'session-only' | 'auto-evict';
  isOptInSaved: boolean;
}

export interface TimelineSession {
  sessionId: string;
  projectId: string;
  attachedWorkspace: 'growth-workspace' | 'inference-runtime-workspace';
  playback: PlaybackState;
  replayDescriptor?: ReplayDescriptor;
  activeProbes: ProbeDefinition[];
  probeSeriesCache: Map<string, ProbeSeries>;
  recordingBuffer?: RecordingBuffer;
  status: 'active' | 'stale' | 'disposed';
}

export interface TimelineController {
  play(): void;
  pause(): void;
  stepForward(): void;
  stepBack(): void;
  scrubToTick(targetTick: number): void;
  setSpeedMultiplier(speed: number): void;
  attachProbe(probe: ProbeDefinition): void;
  detachProbe(probeId: string): void;
}
```

---

## 4. Управление временной шкалой и воспроизведением (Timeline & Playback Controls)

Интерфейс контроллера `TimelineController` предоставляет развитые механизмы управления навигацией по времени:

- **Воспроизведение и пауза (`play() / pause()`)**: Запуск непрерывной генерации/прокачки тиков от AxiEngine и ее безопасная остановка.
- **Пошаговое исполнение (`stepForward() / stepBack()`)**: Продвижение симуляции строго на 1 дискретный тик вперед или возврат к предыдущему сохраненному тику из `RecordingBuffer`.
- **Скраббинг шкалы времени (`scrubToTick()`)**: Мгновенный переход на произвольный номер тика в рамках записанного буфера кадров.
- **Множитель скорости (`setSpeedMultiplier()`)**: Регулировка темпа подачи кадров в интерфейсе (поддерживаемые пресеты: `0.25x`, `0.5x`, `1.0x`, `2.0x`, `4.0x`, `8.0x`).

---

## 5. Детерминированный повтор и запекание сессий (Deterministic Replay Mode)

Режим детерминированного повтора задействуется при необходимости точной отладки динамических процессов:

- **Условный контракт детерминизма**: Воспроизведение является абсолютно детерминированным **только** при абсолютном совпадении всего комплекса контекстных параметров: `snapshotId`, `randomSeed`, `engineBuildHash`, `schemaVersion`, `protocolVersion`, `compiledArtifactHash`, `runtimeInputBindingHash` и при наличии сохраненного входного потока (`inputCaptureId` / recorded input stream). Для интерактивных внешних физических сигналов в реальном времени (`live external inputs`) детерминированный повтор без предварительного захвата потока (`input capture`) не гарантируется.
- **Валидация ReplayDescriptor**: Перед запуском повтора контроллер проверяет соответствие метаданных сессии исходному дескриптору `ReplayDescriptor`. При расхождении генерируется ошибка `AXI-TIME-003`.

---

## 6. Измерительные зонды и осциллографы (Probes & Metric Streams)

Для анализа функционального состояния моделей инженеру предоставляется подсистема зондов (Probes):

- **Настройка зонда (`ProbeDefinition`)**: Визуальный датчик связывается с конкретной сущностью графа (сомой, синапсом, сокетом) и определяет отслеживаемую физическую величину.
- **Накопление временных рядов (`ProbeSeries`)**: Данные зондов собираются в сэмплированные массивы и отображаются в 2D-интерфейсах осциллографов и виртуальных вольтметров.
- **Потоки метрик (`MetricStream`)**: Объединяют группы зондов для комплексного мониторинга производительности и биологической активности отдельных департаментов.

---

## 7. Интеграция со слоями рендеринга (Rendering Handoff)

Интеграция подсистемы времени с подсистемой визуализации `Rendering Pipeline` подчиняется правилам режима Read-Only:

| Слой рендеринга | Отображаемый контент и поведение | Режим доступа |
|---|---|---|
| **`hudLayer`** | 2D-графики осциллографов, значения зондов в реальном времени, текущий номер тика и панель временной шкалы. | Read-Only (2D Overlays) |
| **`runtimeLayer`** | Динамическая 3D-анимация состояний сом, движение спайковых импульсов по трактам и векторных фронтов роста. | Read-Only (3D Animated) |
| **`diagnosticOverlay`** | Отображение временных сбоев, заторов передачи данных и временных ошибок симуляции. | Read-Only (Warnings) |

*Инвариант*: Слой рендеринга лишь отрисовывает состояние текущего кадра и не имеет прав на мутацию структуры редактора.

---

## 8. Разграничение данных и политика хранения (Data Ownership & Storage Policy)

Изоляция канонической модели от временных данных симуляции выполняется по строгому регламенту:

| Тип данных | Место хранения | Канонический статус | Описание |
|---|---|---|---|
| **Канонические TOML-файлы** | `model.toml` / `shard.toml` | **Canonical Source** | Биологическая структура модели. Вход в режим воспроизведения **никогда не изменяет** эти файлы. |
| **Конфигурация зондов и таймлайна** | `axicad.project.json` | Project-Scoped Config | Настройки списка зондов (`ProbeDefinition`), привязанные цвета, выбранная раскладка осциллографов. |
| **Записанные каскады кадров** | Temp Artifact Cache / Memory | Transient / Session Cache | Тяжелые массивы `RecordingBuffer` и `ProbeSeries`. Сохраняются на диск только по явной opt-in команде пользователя в виде временных артефактов. |

---

## 9. Семантика устаревания (Stale Semantics)

Буферы записей таймлайна и кэш зондов переходят в состояние `stale` (устаревшие) при наступлении любого из следующих триггеров:

1. **Мутация реактивного хранилища (`storeRevision`)**: Любое редактирование биологической структуры модели в Store.
2. **Обновление бинарника движка (`engineBuildHash`)**: Пересборка вычислительного ядра AxiEngine.
3. **Изменение версий схемы или протокола**: Смена `schemaVersion` или `protocolVersion`.
4. **Смена параметров генерации**: Изменение зерна `randomSeed` или параметров вычисления.
5. **Изменение скомпилированного артефакта Baker**: Перезапекание биологического графа.
6. **Мутация привязок портов рантайма**: Изменение `RuntimeBinding` во внешних интерфейсах ввода/вывода.

При переходе в состояние `stale` плеер сбрасывает текущий скраббинг, а записанные кадры помечаются как требующие перерасчета.

---

## 10. Каталог диагностик подсистемы времени (Timeline Diagnostics)

Отклонения в работе таймлайна и подсистемы зондов транслируются через канонические объекты `DiagnosticItem`:

### Каталог диагностик подсистемы времени:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-TIME-001` | `unresolved probe target` | `'error'` | `'run-simulation'` | Указанная целевая ссылка (`targetTypedPath`) для зонда не найдена в модели. |
| `AXI-TIME-002` | `recording buffer overflow` | `'warning'` | `None` / `'run-simulation'` | Буфер записи достиг лимита памяти; переводит запись в degraded/dropped-frames mode (блокирует только в режиме lossless recording). |
| `AXI-TIME-003` | `deterministic replay mismatch` | `'error'` | `'run-simulation'` | Метаданные текущей сессии не совпадают с дескриптором `ReplayDescriptor`. |
| `AXI-TIME-004` | `stale timeline session` | `'info'` | `'run-simulation'` | Сессия воспроизведения устарела из-за мутации графа редактора. |
| `AXI-TIME-005` | `unsupported probe metric` | `'warning'` | `'run-simulation'` | Выбранная метрика не поддерживается целевым типом сущности. |

---

## 11. Межмодульное взаимодействие (Cross-Workspace Integration)

Спецификация таймлайна является общей платформой для предметных режимов отладки:

- **Growth Workspace**: Использует контракт таймлайна для пошагового контроля роста аксонов, синаптогенеза и визуализации каскадов `GrowthFrame`.
- **Future Inference Runtime Workspace**: Использует данный контракт как базовый менеджер воспроизведения сигналов сети, прокачки данных через порты ввода/вывода и отрисовки осциллографов.

---

## 12. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [growth-workspace-spec-ru](growth-workspace-spec-ru.md) — Спецификация предметного режима симуляции и отладки роста сети.
- [external-port-io-spec-ru](external-port-io-spec-ru.md) — Спецификация внешних портов ввода/вывода.
- [axiengine-bridge-session-spec-ru](axiengine-bridge-session-spec-ru.md) — Спецификация моста интеграции и менеджера сессий.
- [engine-preview-pipeline-spec-ru](engine-preview-pipeline-spec-ru.md) — Спецификация пайплайна предпросмотра.
- [rendering-pipeline-spec-ru](rendering-pipeline-spec-ru.md) — Спецификация визуального слоя рендеринга.
- [editor-store-spec-ru](editor-store-spec-ru.md) — Спецификация реактивного хранилища Store.
- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.
- [rust-core-axiengine-source-of-truth-spec-ru](rust-core-axiengine-source-of-truth-spec-ru.md) — Спецификация вычислительного ядра AxiEngine.

---

## 13. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации контроллера времени, зондов и метрик симуляции Runtime Timeline & Probe Spec. Определены DTO сущности, тиковая навигация, детерминированный повтор, спецификация зондов, слои рендеринга и правила хранения. |
| 2026-06-27 | 0.1.1 | Точечные доработки: исправлена опечатка, сформулирован условный контракт детерминированного повтора (с учетом input capture), расширены DTO `ReplayDescriptor`, `ProbeDefinition` (`targetTypedPath`) и `RecordingBuffer` (`storageKind`, `retentionPolicy`), уточнено поведение при overflow записи (degraded/dropped-frames mode) и обновлен каталог диагностик. |
