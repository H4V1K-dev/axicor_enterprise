# Спецификация сценариев симуляции и пресетов запусков (Simulation Scenario & Run Preset Spec)

> Этот документ формально определяет архитектурный контракт сохранения, оркестрации, воспроизведения и сравнения сценариев симуляции (Simulation Scenarios) и пресетов запусков (Run Presets) на стороне 3D-редактора AxiCAD. Спецификация регламентирует проектный уровень управления конфигурациями запусков для предметных режимов отладки роста (Growth Workspace) и выполнения рантайма (Inference Runtime Workspace).

## Status: Draft

---

## 1. Назначение документа (Scope & Non-scope)

Данная спецификация определяет стандарты оркестрации и длительного хранения настроек запусков симуляционных процессов в AxiCAD.

### Назначение (Scope)
- **Именованные сценарии запусков (Named Simulation Scenarios)**: Сохранение целостных проектных конфигураций отладки и тестирования нейронных моделей.
- **Пресеты конфигураций (Run Presets)**: Готовые повторно используемые наборы параметров квантования времени, зерен случайных чисел и политик записи.
- **Управление профилями входов (Input Profiles)**: Фиксация конфигураций стимулов, привязанных генераторов и режимов подачи данных.
- **Оркестрация зондов и метрик (Probe & Metric Panel Layouts)**: Сохранение визуальных раскладок осциллографов и выбранных измерительных датчиков.
- **Политика захвата и записи (Capture & Recording Policy)**: Управление автоматическим сохранением входных стимулов и буферизацией выходных кадров.
- **Журнал и сравнение запусков (Run Records & Benchmarking)**: Фиксация исторического реестра проведенных симуляций для сравнения показателей продуктивности и динамики роста.

### Вне зоны ответственности (Non-scope)
- Документ **не описывает** математику нейронной динамики, формулы морфогенеза или алгоритмы инференса.
- Документ **не содержит** канонические TOML-данные биологической структуры нейросетевой модели.
- Документ **не производит** прямую мутацию канонического графа модели в Store во время воспроизведения сценария.
- Документ **не разрабатывает** низкоуровневые драйверы физического оборудования и устройства ввода/вывода.

---

## 2. Разграничение и владение данными (Data Ownership & Storage)

Изоляция проектных сценариев от канонического описания нейросети подчиняется строгому разделению обязанностей:

> **Scenarios and Run Presets orchestrate simulation execution profiles. AxiEngine executes. AxiCAD orchestrates, records, and benchmarks. Canonical TOML remains immutable during scenario playback.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                            axicad.project.json                                │
│          (Stores SimulationScenarios, RunPresets & Probe Layouts)             │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Instantiates Execution Config
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                    AxiCAD Scenario Orchestrator & Session                     │
│          (Manages RunRecords, Artifact References & Stale State)              │
└──────────────────┬────────────────────────────────────────┬───────────────────┘
                   │ Execute via Engine Session             │ Storage Offload
                   ▼                                        ▼
┌──────────────────────────────────────┐  ┌─────────────────────────────────────┐
│       AxiEngine Bridge Session       │  │       Project Temp / Cache          │
│ Advances Ticks (Growth / Inference)  │  ├─────────────────────────────────────┤
│                                      │  │ Heavy Binary Captures & Frame Buffers│
└──────────────────────────────────────┘  └─────────────────────────────────────┘
```

| Тип данных | Место хранения | Канонический статус | Описание |
|---|---|---|---|
| **Биологическая модель** | `model.toml` / `shard.toml` | **Canonical Source** | Биологическая структура модели. Сценарии симуляции **никогда не изменяют** эти файлы напрямую. |
| **Сценарии и пресеты** | `axicad.project.json` | Project-Scoped Config | Именованные сценарии (`SimulationScenario`), пресеты запусков (`RunPreset`), ссылки на используемые профили зондов. |
| **Бинарные артефакты запусков** | Project Temp / Cache / Artifacts | Transient / Session Cache | Тяжелые бинарные файлы захвата стимулов (`inputCaptureId`), буферы кадров (`RecordingBuffer`) и временные ряды. |

*Инвариант*: Любые предлагаемые или полученные в результате симуляции структурные изменения (например, выросшие синапсы) применяются к канонической модели исключительно через операцию `Command Mutation` / `PatchSet`.

---

## 3. Основные сущности и DTO (Core Entities DTOs)

Для управления сценариями и пресетами запусков в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type ScenarioTargetMode = 'growth' | 'inference';

export interface ScenarioInputConfig {
  inputMode: 'live-external-inputs' | 'captured-input-replay' | 'synthetic-fixtures' | 'detached-no-input';
  bindingProfileId?: string;
  displayLabel?: string;
  inputCaptureId?: string;
  activeFixtureIds?: string[];
}

export interface ScenarioGrowthConfig {
  growthSteps: number;
  voxelGridBounds?: number[];
  growthConstraintProfileId?: string;
}

export interface ScenarioInferenceConfig {
  inputConfig: ScenarioInputConfig;
}

export interface ScenarioProbeConfig {
  activeProbePaths: string[]; // List of targetTypedPath references
  expandedOscilloscopeIds: string[];
  hudLayoutPresetId?: string;
}

export interface ScenarioCapturePolicy {
  enableInputCapture: boolean;
  enableLosslessRecording: boolean;
  maxMemoryBufferBytes: number;
  autoEvictOldFrames: boolean;
}

export interface ScenarioStopCondition {
  maxTicks?: number;
  maxTimestampMs?: number;
  stopOnSpikeBurst?: boolean;
  stopOnGrowthConstraintViolation?: boolean;
}

export interface RunPreset {
  presetId: string;
  name: string;
  targetMode: ScenarioTargetMode;
  randomSeed: number;
  targetTickRateHz: number;
  stopCondition: ScenarioStopCondition;
  capturePolicy: ScenarioCapturePolicy;
}

export interface SimulationScenario {
  scenarioId: string;
  name: string;
  description?: string;
  targetMode: ScenarioTargetMode;
  preset: RunPreset;
  growthConfig?: ScenarioGrowthConfig;
  inferenceConfig?: ScenarioInferenceConfig;
  probeConfig: ScenarioProbeConfig;
  createdAtIso: string;
  updatedAtIso: string;
}

export interface ScenarioArtifactRef {
  artifactId: string;
  artifactKind: 'input-capture' | 'recording-buffer' | 'probe-series-dump';
  storageKind: 'temp-artifact' | 'project-cache' | 'external-artifact-ref';
  artifactRef: string; // Relative or canonical reference identifier
  relativeCachePath?: string;
  checksumSha256: string;
  byteLength: number;
}

export interface ScenarioRunRecord {
  recordId: string;
  scenarioId: string;
  executedAtIso: string;
  snapshotId: string;
  storeRevision: number;
  schemaVersion: string;
  protocolVersion: string;
  engineBuildHash: string;
  compiledArtifactHash: string;
  runtimeInputBindingHash?: string;
  inputCaptureId?: string;
  inputCaptureHash?: string;
  randomSeed: number;
  runConfigHash: string;
  totalTicksExecuted: number;
  executionDurationMs: number;
  isDeterministic: boolean;
  artifactRefs: ScenarioArtifactRef[];
}

export interface ScenarioStaleState {
  isStale: boolean;
  staleReason?: 
    | 'store-revision-changed' 
    | 'artifact-recompiled' 
    | 'engine-updated' 
    | 'probe-target-missing' 
    | 'binding-hash-mismatch'
    | 'schema-version-changed'
    | 'protocol-version-changed'
    | 'input-capture-changed'
    | 'run-config-changed'
    | 'random-seed-changed';
  affectedRef?: string;
}
```

### Инварианты соответствия режимов (Target Mode Invariants)

Структурная валидация сценариев подчиняется следующим обязательным правилам:
1. **Режим Growth**: Если `SimulationScenario.targetMode === 'growth'`, то блок `growthConfig` является обязательным, а `inferenceConfig` отсутствует или игнорируется при запуске.
2. **Режим Inference**: Если `SimulationScenario.targetMode === 'inference'`, то блок `inferenceConfig` является обязательным, а `growthConfig` отсутствует или игнорируется при запуске.
3. **Совпадение пресета**: Значение `RunPreset.targetMode` должно строго совпадать с целевым режимом сценария `SimulationScenario.targetMode`.

---

## 4. Интеграция с предметным режимом роста (Growth Workspace Integration)

Слой сценариев предоставляет развитые механизмы управления процессами морфогенеза сети:

- **Конфигурирование сессий роста**: Сценарий со сфокусированным режимом `growth` хранит параметры пошагового роста, ограничения объема воксельной сетки и зерна случайных чисел для ветвления аксонов.
- **Статус результатов**: Сгенерированные кадры роста (`GrowthFrame`) и векторные фронты являются строго производными артефактами (`derived artifacts`). Принятие результатов морфогенеза выполняется инженером вручную через команду `Apply PatchSet`.

---

## 5. Интеграция с предметным режимом инференса (Inference Runtime Workspace Integration)

Для симуляции исполнения сети сценарии задают четкий регламент рантайм-среды:

- **Параметризация инференса**: Сценарии инференса хранят свою конфигурацию в блоке `inferenceConfig` (включая режим подачи стимулов `RuntimeInputMode`, ссылки на безопасные профили связывания портов `bindingProfileId`, задействованные синтетические генераторы и целевую частоту тиков).
- **Гарантии детерминизма**: Для сценариев инференса детерминированный повтор гарантируется **только** при полном совпадении всего контекста: `snapshotId`, `randomSeed`, `engineBuildHash`, `schemaVersion`, `protocolVersion`, `compiledArtifactHash`, `runtimeInputBindingHash` и наличия сохраненного файла стимулов (`inputCaptureId` / `inputCaptureHash`). Сессии с живыми входами (`live external inputs`) без предварительного захвата потока всегда являются недетерминированными (`isDeterministic: false`).

---

## 6. Интеграция с подсистемой времени, зондов и визуализации

Связь сценариев с фундаментальными измерительными сервисами подчиняется следующим правилам:

- **Стабильная адресация зондов**: Сценарий сохраняет списки зондов через их стабильные логические пути (`targetTypedPath`), гарантируя восстановление измерительных приборов при перезагрузке проекта.
- **Внешняя адресация тяжелых данных**: Тяжелые массивы записей рантайм-сессий (`ProbeSeries`, `InferenceFrame`) сохраняются во временном кэше и привязываются к сценарию через дескрипторы артефактов `ScenarioArtifactRef` со сверкой целостности по `checksumSha256`.

---

## 7. Интеграция с внешними портами и безопасность (External Port IO & Security)

При формировании переносимых пресетов и сценариев строго соблюдаются требования приватности:

> [!CAUTION]
> Сценарии симуляции (`SimulationScenario`) сериализуются в проектный файл `axicad.project.json` и могут передаваться между командами разработчиков. 
> В структуры сценариев **категорически запрещено** записывать физические секреты, API-токены авторизации, приватные IP-адреса оборудования, хэндлы устройств системных драйверов и локальные абсолютные пути на диске пользователя. Сценарии должны ссылаться исключительно на безопасные логические идентификаторы профилей (`bindingProfileId`) и отображаемые метки (`displayLabel`). 
> При обнаружении попытки сериализации секретов генерируется критическая ошибка `AXI-SCEN-004` (блокирует сохранение проекта и экспорт сценария). Подсистема сериализации поддерживает автоматическую очистку секретов перед сохранением (`auto-redaction before save`).

---

## 8. Семантика устаревания (Stale Semantics)

Сценарии симуляции и исторические записи запусков (`ScenarioRunRecord`) переходят в состояние `stale` (устаревшие) при наступлении любого из следующих событий:

1. **Мутация реактивного хранилища (`storeRevision`)**: Изменение канонической структуры биологической модели в Store (`store-revision-changed`).
2. **Изменение бинарного артефакта (`compiledArtifactHash`)**: Перезапекание модели с помощью компилятора Baker (`artifact-recompiled`).
3. **Обновление движка (`engineBuildHash`)**: Изменение версии вычислительного ядра AxiEngine (`engine-updated`).
4. **Смена версий схемы или протокола**: Изменение `schemaVersion` (`schema-version-changed`) или `protocolVersion` (`protocol-version-changed`).
5. **Мутация хэша привязок портов (`runtimeInputBindingHash`)**: Перекоммутация внешних физических портов (`binding-hash-mismatch`).
6. **Изменение хэша захвата стимулов (`inputCaptureHash`)**: Модификация файла записи входных сигналов (`input-capture-changed`).
7. **Изменение конфигурации запуска (`runConfigHash`)**: Модификация параметров пресета или конфигурации сессии (`run-config-changed`).
8. **Изменение зерна случайных чисел (`randomSeed`)**: Модификация псевдослучайного зерна генерации (`random-seed-changed`).
9. **Отсутствие целевого объекта зонда (`probe target missing`)**: Удаление элемента модели, на который ссылался `targetTypedPath` (`probe-target-missing`).

---

## 9. Операции подсистемы сценариев (Scenario Operations)

Оркестратор сценариев AxiCAD предоставляет следующий канонический набор функций управления:

- `createScenario(targetMode, name, preset)`: Создание нового сценария симуляции.
- `duplicateScenario(scenarioId)`: Клонирование существующего сценария для проведения сравнительных тестов.
- `updateScenarioPreset(scenarioId, updatedPreset)`: Модификация параметров пресета запуска.
- `runScenario(scenarioId)`: Запуск сессии симуляции на основе параметров сценария.
- `stopScenario(scenarioId)`: Безопасное завершение активного сценария с сохранением метрик.
- `captureScenarioInputs(scenarioId, options)`: Запуск фиксации входных стимулов в бинарный файл артефакта.
- `saveRunRecord(scenarioId, executionMetrics)`: Сохранение исторической карточки выполненного запуска.
- `compareRunRecords(recordIdA, recordIdB)`: Сравнительный анализ производительности и динамики двух запусков.
- `deleteScenarioArtifacts(scenarioId)`: Очистка тяжелых временных бинарных файлов симуляции с диска.

---

## 10. Каталог диагностик сценариев (Scenario Diagnostics AXI-SCEN-*)

Отклонения в конфигурациях сценариев и пресетов транслируются через объекты `DiagnosticItem`:

### Каталог диагностик сценариев:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-SCEN-001` | `unresolved scenario target` | `'error'` | `'run-simulation'` | Указанный целевой воркспейс или режим сценария недоступен. |
| `AXI-SCEN-002` | `stale scenario preset` | `'warning'` | `None` / `'run-simulation'` | Параметры пресета устарели по отношению к текущему состоянию модели. |
| `AXI-SCEN-003` | `missing input capture` | `'error'` | `'run-simulation'` | Файл захвата стимулов (`inputCaptureId`) не найден на диске (блокирует `run-simulation` только для режима `captured-input-replay`). |
| `AXI-SCEN-004` | `unsafe runtime binding reference` | `'error'` | `'save-project'`, `'export-scenario'` | Обнаружена попытка сериализации приватных физических данных или токенов в сценарий (требует редакции или auto-redaction). |
| `AXI-SCEN-005` | `probe target missing` | `'warning'` | `None` | Целевой объект (`targetTypedPath`), на который ссылается зонд сценария, не найден в модели. |
| `AXI-SCEN-006` | `incompatible engine build` | `'error'` | `'run-simulation'` | Версия вычислительного ядра не совпадает с требованиями пресета сценария. |
| `AXI-SCEN-007` | `artifact missing` | `'error'` | `'run-simulation'` | Файл бинарного артефакта сессии не найден или поврежден (сбой SHA-256). |
| `AXI-SCEN-008` | `non-deterministic live input warning` | `'info'` | `None` | Информирование о том, что сценарий запускается в недетерминированном режиме live inputs. |

---

## 11. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [growth-workspace-spec-ru](growth-workspace-spec-ru.md) — Спецификация предметного режима симуляции и отладки роста сети.
- [inference-runtime-workspace-spec-ru](inference-runtime-workspace-spec-ru.md) — Спецификация предметного режима выполнения симуляции и инференса.
- [runtime-timeline-probe-spec-ru](runtime-timeline-probe-spec-ru.md) — Спецификация контроллера времени, зондов и метрик симуляции.
- [external-port-io-spec-ru](external-port-io-spec-ru.md) — Спецификация внешних портов ввода/вывода.
- [axiengine-bridge-session-spec-ru](axiengine-bridge-session-spec-ru.md) — Спецификация моста интеграции и менеджера сессий.
- [engine-preview-pipeline-spec-ru](engine-preview-pipeline-spec-ru.md) — Спецификация пайплайна предпросмотра.
- [command-mutation-spec-ru](command-mutation-spec-ru.md) — Спецификация командных мутаций и истории действий.
- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.

---

## 12. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации сценариев симуляции и пресетов запусков Simulation Scenario & Run Preset Spec. Определены DTO сущности, правила хранения в `axicad.project.json`, интеграция с воркспейсами, безопасная изоляция секретов и каталог диагностик AXI-SCEN. |
| 2026-06-27 | 0.1.1 | Точечные доработки: исправлен `ScenarioArtifactRef` (убран `storagePath`, добавлена относительная адресация), расширены `ScenarioRunRecord` и `ScenarioStaleState`, разделены mode-specific конфиги `growthConfig`/`inferenceConfig`, уточнено условие детерминизма, Security diagnostics `AXI-SCEN-004` переведен в `'error'`, ограничены блокировки `AXI-SCEN-003` и исправлены опечатки. |
