# Спецификация внешних портов ввода/вывода и интерфейсов рантайма (External Port IO Spec)

> Этот документ формально определяет архитектурный контракт декларации, отображения, валидации и связывания внешних портов ввода/вывода (External Ports IO) на стороне 3D-редактора AxiCAD. Спецификация фиксирует статус Port как границы взаимодействия нейронной модели с внешним миром рантайма (External Runtime-Facing Boundary), определяет принципы работы кодировщиков (Encoders) и декодировщиков (Decoders), а также разграничивает канонические TOML-данные и динамические сессионные привязки.

## Status: Draft

---

## 1. Назначение документа (Scope & Non-goals)

Данная спецификация устанавливает правила описания внешних интерфейсов нейронной модели и их интеграции в визуальную среду AxiCAD.

### Назначение (Scope)
- **Декларация портов (Port Declarations)**: Формальное описание входных (`InputPort`) и выходных (`OutputPort`) точек взаимодействия модели.
- **Интерфейсы преобразования (Encoders & Decoders)**: Определение правил трансляции внешних сигналов (изображение, звук, векторные величины) в асинхронные спайковые импульсы и обратно.
- **Потоки стимулов и реакций (StimulusStreams & MotorOutputs)**: Связывание внешних каналов данных с элементами ввода/вывода.
- **Динамические привязки рантайма (RuntimeBindings)**: Управление сессионными профилями подключения реальных устройств или виртуальных файлов данных.
- **Связывание с моделью (Model Endpoint Binding)**: Разрешение ссылок от внешних портов к внутренним точкам подключения (`Endpoint`) департаментов и шардов.
- **Визуализация и валидация**: Отображение портов в 3D-вьюпорте AxiCAD и проверка целостности IO-инвариантов.

### Вне зоны ответственности (Non-goals)
- Документ **не описывает** внутреннее устройство рантайм-симулятора Inference Engine.
- Документ **не разрабатывает** низкоуровневые драйверы физического оборудования (камер, робототехнических суставов, PCI-плат) на Rust/C++.
- Документ **не определяет** проприетарные бинарные форматы физических сенсоров сторонних производителей.

---

## 2. Миграция и конфликты терминологии (Migration & Terminology Collision)

При работе с предыдущими спецификациями следует учитывать важное терминологическое разграничение:

> [!IMPORTANT]
> Существующие структуры `PortConfig` и секции `[[ports]]` в файлах `shard.toml`, упомянутые в `toml-schema-spec-ru.md` и `domain-model-spec-ru.md`, представляют собой **legacy-терминологию**. 
> Новый сущностный `Port` (`InputPort` / `OutputPort`) формально определяется исключительно как **external-runtime-facing boundary на уровне модели** (`model.toml`).

Старые локальные структуры на уровне шардов подлежат миграции и переименованию в `socket samples`, `socket pins` или `legacy shard IO`. До проведения планового обновления спецификаций TOML-схемы текущий документ задает целевую каноническую терминологию.

---

## 3. Главный принцип (Main Principle)

Границы ответственности элементов транспортной подсистемы строго подчиняются следующей фундаментальной формуле:

> **Socket is geometry-local. Endpoint is graph/model-local. Port is external-runtime-facing. Ports describe how the model talks to the outside world.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                                External World                                 │
│                 (Cameras, Sensors, Motors, Synthetic Files)                   │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Physical Signals / Data Streams
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                           External Port IO (Port)                             │
│                  (Encoders, Decoders, RuntimeBindings)                        │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Model-Local Logical Links
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                           Model Endpoint (Graph)                              │
│                (Internal IO Aggregators inside Departments)                   │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Geometry-Local Voxel Mapping
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                           Shard Socket (Geometry)                             │
│                  (Discrete 3D Contact Pins on Shard Boundary)                 │
└───────────────────────────────────────────────────────────────────────────────┘
```

1. **Socket (геометрически-локальный)**: Точка контакта на 3D-границе конкретного шарда. Владеет точной воксельной координатой.
2. **Endpoint (модельно-локальный)**: Логический агрегатор в графе связей модели. Группирует каналы внутри департамента или шарда.
3. **Port (внешний интерфейс рантайма)**: Глобальная граница модели. Определяет, как именно вся нейросеть взаимодействует с внешним миром через кодеки и драйверы рантайма.

---

## 4. Терминология и понятийный аппарат (Taxonomy & Glossary)

Для описания внешних интерфейсов в AxiCAD определены следующие TypeScript DTO и интерфейсы:

```typescript
export type ExternalPortDirection = 'input' | 'output' | 'bidirectional';

// Built-in presets for codecs (extensible string types)
export type BuiltInEncoderPreset = 'rate-coder' | 'latency-coder' | 'population-coder' | 'direct-spike-passthrough';
export type BuiltInDecoderPreset = 'population-vector-decoder' | 'mean-rate-decoder' | 'first-spike-decoder';

export interface CodecSpec {
  codecId: string;
  codecKind: 'encoder' | 'decoder';
  provider: string; // e.g. 'builtin', 'axicor-standard', 'vendor-plugin'
  pluginRef?: string;
  capabilities: string[];
  schemaVersion: string;
  samplingRateHz?: number;
  integrationWindowMs?: number;
  resolutionChannels?: number;
  parameters: Record<string, unknown>;
}

export interface InputPort {
  portId: string;
  name: string;
  targetEndpointRef: string; // Proposed logical endpoint ref (e.g., @model/departments/Visual/endpoints/retina_in)
  encoder: CodecSpec;
  expectedStreamShape: number[];
}

export interface OutputPort {
  portId: string;
  name: string;
  sourceEndpointRef: string; // Proposed logical endpoint ref
  decoder: CodecSpec;
  expectedOutputShape: number[];
}

// Runtime/Session Entities (stored ONLY in axicad.project.json or session cache)
export interface StimulusStream {
  streamId: string;
  sourceType: 'camera-feed' | 'audio-stream' | 'file-dataset' | 'synthetic-generator';
  resourceUri?: string; // Private/local URI (Never saved in canonical TOML)
  sampleRate: number;
}

export interface MotorOutput {
  channelId: string;
  targetDeviceType: 'robotic-joint' | 'virtual-actuator' | 'file-logger';
  commandMapping: string;
}

export interface RuntimeBinding {
  bindingId: string;
  portId: string;
  activeStreamId?: string;
  activeMotorChannelId?: string;
  status: 'connected' | 'disconnected' | 'stale' | 'error';
  lastActivityTimestamp?: number;
}
```

---

## 5. Декларация Port в структуре TOML (TOML Schema Integration)

> [!CAUTION]
> Массив секций `[[model.ports]]` является **proposed target extension** (предлагаемым расширением целевой схемы TOML) и потребует последующего обновления `toml-schema-spec-ru.md` и Rust-сборщиков. До момента официальной поддержки в Rust serde парсерах это поле является несовместимым с флагом `deny_unknown_fields` ядра.

Внешние порты модели декларируются в файле `model.toml` без указывания локальных физических путей и секретов:

```toml
# Предлагаемая расширенная декларация внешнего порта ввода в model.toml
[[model.ports]]
id = "port_visual_retina"
name = "Retinal Camera Input"
direction = "input"
target_endpoint_ref = "@model/departments/Visual/endpoints/retina_in"
expected_shape = [128, 128, 1]

[model.ports.encoder]
codec_id = "enc_pop_retina"
codec_kind = "encoder"
provider = "builtin"
preset = "population-coder"
schema_version = "1.0.0"
capabilities = ["spatial-2d", "rate-coding"]

[model.ports.encoder.parameters]
sampling_rate_hz = 60
resolution_channels = 16384
tuning_curve = "gaussian"

# Предлагаемая расширенная декларация внешнего порта вывода в model.toml
[[model.ports]]
id = "port_motor_arm"
name = "Primary Arm Motor Output"
direction = "output"
source_endpoint_ref = "@model/departments/Motor/endpoints/arm_cortex_out"
expected_shape = [6]

[model.ports.decoder]
codec_id = "dec_vec_arm"
codec_kind = "decoder"
provider = "builtin"
preset = "population-vector-decoder"
schema_version = "1.0.0"
capabilities = ["vector-6dof"]

[model.ports.decoder.parameters]
integration_window_ms = 20
output_channels = 6
```

---

## 6. Визуализация и редактирование Ports в AxiCAD (UI/UX Operations)

AxiCAD предоставляет специализированный инструментарий для работы с внешними портами в 3D-пространстве:

- **Визуальная оболочка модели (Bounding Envelope)**: Порты отображаются на внешней габаритной границе всей модели в виде контрастных интерактивных коннекторов (Glyphs).
- **Цветовая индикация направленности**: Входные порты (`InputPort`) маркируются синим цветом, выходные (`OutputPort`) — зеленым, некорректно связанные — красным индикатором ошибки.
- **Инспектор кодеков (Codec Inspector)**: Панель параметров позволяет тонко настраивать частоты дискретизации (`samplingRateHz`), окна интеграции кодеков и размерности сигналов.
- **Интерактивная прокладка связей**: Редактор позволяет визуально перетаскивать связи (Drag-and-Drop) от внешнего порта к внутренним `Endpoint` департаментов.

---

## 7. Связывание с внутренними сущностями модели (Model Endpoint Binding)

Внешний порт не подключается напрямую к вокселям сом. Связывание осуществляется через каноническую адресацию `target_endpoint_ref` / `source_endpoint_ref`:

```
┌─────────────────────────────────┐      Endpoint Ref       ┌──────────────────────────┐
│ InputPort (model.toml)          ├────────────────────────►│  Endpoint (dept.toml)    │
│ target_endpoint_ref =           │  (@model/Depts/...)     └────────────┬─────────────┘
│ "@model/departments/Visual/..." │                                      │ Internal Routing
└─────────────────────────────────┘                                      ▼
                                                            ┌──────────────────────────┐
                                                            │   Socket (shard.toml)    │
                                                            └──────────────────────────┘
```

1. **Proposed Endpoint Ref Grammar**: Формат адресации вида `@model/departments/Visual/endpoints/retina_in` является предлагаемым расширением грамматики ссылок и потребует обновления `path-resolver-spec-ru.md`.
2. **Разрешение ссылок (Path Resolving)**: Модуль Path Resolver проверяет доступность ссылки `target_endpoint_ref` в текущем графе моделей.
3. **Runtime UUID Binding**: Динамический UUID-маппинг вычислительного ядра создается в сессионном кэше резолвера и **никогда не сериализуется** в файлы TOML.
4. **Проверка типов данных**: Слой валидации сверяет число выходов кодека порта с емкостью целевого `Endpoint`.

---

## 8. Разграничение канонических TOML и рантайм-кэша (Data Ownership & Security)

Изоляция декларации модели от конкретного физического оборудования на ПК пользователя строго соблюдает разграничение данных:

| Тип данных | Место хранения | Канонический статус | Описание |
|---|---|---|---|
| **Декларация портов и кодеков** | `model.toml` | **Canonical Source** | Имена портов, спецификация кодеков (`CodecSpec`), публичные детерминированные параметры (`parameters`), логические ссылки `target_endpoint_ref` и форма сигналов (`expected_shape`). |
| **Сессионные привязки оборудования** | `axicad.project.json` / session cache | Transient / Session Cache | Динамические IP-адреса, Camera Device ID, имена COM-портов, локальные пути к стимулам, API-токены и приватные URI (`RuntimeBinding` / `StimulusStream`). |
| **Буферы потоков данных** | Memory / Session Temp | Transient Data | Сырые байтовые потоки сенсорных данных, используемые для отладки в UI. |

### Безопасность и приватность данных (Security & Privacy Note)
> [!CAUTION]
> Поле `CodecSpec.parameters` в каноническом `model.toml` хранит исключительно публичные детерминированные параметры кодека (например, гамма-коррекцию, размер ядра, фильтры). 
> Локальные абсолютные пути к файлам устройств, приватные IP-адреса, маркеры авторизации (API-токены) и персональные настройки окружения **категорически запрещено** сериализовать в файлы `model.toml` или экспортные пакеты модели. Все непубличные физические данные хранятся строго в несинхронизируемом файле сессионного проекта `axicad.project.json` или передаются через переменные окружения рантайма.

---

## 9. Валидация и каталог диагностик (Validation & Diagnostics)

Нарушения правил связывания и параметров портов генерируют канонические объекты `DiagnosticItem` со строго настроенными флагами блокировок `blockingOperations`:

### Каталог диагностик IO-портов:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-PORT-001` | `unresolved port binding` | `'error'` | `'export-toml'`, `'baker-compile'`, `'run-simulation'`, `'apply-patchset'` | Указанная ссылка `target_endpoint_ref` или `source_endpoint_ref` не существует в графе модели. |
| `AXI-PORT-002` | `incompatible encoder/decoder` | `'error'` | `'export-toml'`, `'baker-compile'`, `'run-simulation'`, `'apply-patchset'` | Число каналов кодека не совпадает с емкостью привязанного Endpoint. |
| `AXI-PORT-003` | `missing external source` | `'warning'` | `'run-simulation'` | К порту не привязано физическое устройство или файл стимулов в сессии (не блокирует Baker compile). |
| `AXI-PORT-004` | `invalid stream shape/rate` | `'warning'` | `'run-simulation'` | Частота или размерность входного потока данных превышает допустимые диапазоны кодека. |
| `AXI-PORT-005` | `stale runtime binding` | `'info'` | `'run-simulation'` | Сессионная привязка оборудования устарела после командной мутации графа модели. |

---

## 10. Межмодульное взаимодействие (Cross-Module Interaction)

Подсистема внешних портов интегрирована со смежными модулями AxiCAD:

- **Connectome Workspace**: Отображает проспективные внешние связи (prospective external-port links), позволяя инженеру видеть, как внешние сигналы распределяются по внутренним трактам связей.
- **Inference Runtime Workspace (Future)**: Является прямым владельцем исполнения IO во время симуляции сети в реальном времени, осуществляя прокачку байтовых потоков через кодеки.
- **Import/Export Serialization**: Гарантирует точное сохранение и десериализацию секций `[[model.ports]]` без потерь специфичных параметров кодеков.

---

## 11. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [domain-model-spec-ru](domain-model-spec-ru.md) — Доменная модель AxiCAD и соответствие TOML/Rust контракту.
- [toml-schema-spec-ru](toml-schema-spec-ru.md) — Каноническая TOML-схема Axicor и описание полей.
- [path-resolver-spec-ru](path-resolver-spec-ru.md) — Спецификация путей, адресации и резолвера ссылок.
- [connectome-workspace-spec-ru](connectome-workspace-spec-ru.md) — Спецификация предметного режима проектирования связей Connectome Workspace.
- [import-export-serialization-spec-ru](import-export-serialization-spec-ru.md) — Спецификация импорта, экспорта и сериализации.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.
- [rust-core-axiengine-source-of-truth-spec-ru](rust-core-axiengine-source-of-truth-spec-ru.md) — Спецификация вычислительного ядра AxiEngine.
- [axiengine-bridge-session-spec-ru](axiengine-bridge-session-spec-ru.md) — Спецификация моста интеграции и менеджера сессий AxiEngine.
- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.

---

## 12. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации внешних портов ввода/вывода External Port IO Spec. Определены разграничения Socket/Endpoint/Port, DTO кодеков и стимулов, TOML-схема `[[model.ports]]`, правила адресации и каталог диагностик IO. |
| 2026-06-27 | 0.1.1 | Точечная доработка спецификации: добавлен раздел о миграции legacy-терминологии `PortConfig`, уточнено расширение TOML-схемы `[[model.ports]]`, расширена модель `CodecSpec` с поддержкой плагинов, разделены канонические данные и локальные сессионные секреты/URI, добавлены правила Security/Privacy и уточнены блокировки диагностик в `blockingOperations`. |
| 2026-06-27 | 0.1.2 | Финальная доработка: выравнена нумерация разделов (1-12), зафиксирована несовместимость `[[model.ports]]` с `deny_unknown_fields`, TOML-примеры приведены к структуре `CodecSpec` с полем `preset`, унифицирована грамматика `target_endpoint_ref`, и разделены публичные детерминированные параметры кодека и рантайм-секреты. |
