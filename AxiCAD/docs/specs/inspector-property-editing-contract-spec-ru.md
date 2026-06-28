# Спецификация контракта инспектора свойств и редактирования параметров (Inspector Property Editing Contract Spec)

> Этот документ формально определяет архитектурный контракт работы UI-панели инспектора свойств (Inspector Property Panel) на стороне 3D-редактора AxiCAD. Спецификация регламентирует правила отображения, разрешения схем параметров, валидации черновиков, обработки множественного выделения и применения изменений полей через транзакционный командный слой Command Mutation.

## Status: Draft

---

## 1. Назначение документа (Scope & Non-scope)

Данная спецификация определяет унифицированные стандарты взаимодействия пользовательского интерфейса с моделью данных редактора при инспектировании и редактировании свойств.

### Назначение (Scope)
- **Контракт панелей свойств (Property Panels Contract)**: Декларативное описание полей, их группировка и сопоставление с выделенными сущностями.
- **Разграничение прав редактирования (Readonly / Editable Fields)**: Классификация полей по режимам доступа и причинам блокировки записи.
- **Разрешение схем параметров (Field Schema Resolution)**: Динамическое сопоставление выделенного объекта (`InspectorSelection`) с типом схемы.
- **Валидация черновиков (Validation Before Commit)**: Проверка синтаксиса и семантики в буфере черновиков (`draft edit buffer`) до коммита.
- **Управление флагами загрязнения (Dirty Flags Management)**: Точные правила пометки `toml_documents_dirty`, `dirty_entities` и `project_file_dirty`.
- **Интеграция с Undo/Redo**: Гарантии атомарности и отменяемости любых пользовательских изменений через `Command Mutation`.
- **Множественное выделение (Mixed Selection)**: Отображение одинаковых, неоднородных (`MixedValueState`) и несовместимых параметров группы объектов.
- **Инспектирование рантайм-данных и спайков**: Показ вычисляемых показателей AxiEngine, зондов и динамических артефактов без права прямого коммита в TOML. Спайк (`runtime spike`) инспектируется через связанный сегмент тракта/зонд в режиме `readonly-runtime` / `derived-preview`.
- **Инспектирование пресетов инструментов (Tool Presets)**: Отображение текущих рабочих параметров инструментов с визуализацией каскада переопределений источника (`session` > `project-local` > `user-global` > `built-in default`).

### Вне зоны ответственности (Non-scope)
- Документ **не определяет** визуальный графический дизайн UI (цвета, шрифты, CSS-стили) и выбор конкретного фреймворка (React, Vue, Svelte).
- Документ **не описывает** внутренние вычисления и математические алгоритмы ядра AxiEngine.
- Документ **не является** спецификацией низкоуровневого парсера или формата файлов TOML.
- Документ **запрещает** прямое внесение изменений в хранилище Store в обход командного слоя Command Mutation.

---

## 2. Главный принцип (Main Principle)

Архитектура инспектора свойств строго подчиняется следующей фундаментальной формуле:

> **Inspector reads from Store, edits through Command Mutation, validates before commit, never writes canonical files directly.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                           Reactive Editor Store                               │
│        (Holds Entity State, Selection, Diagnostics & Active Job Locks)        │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Reads Selection & Property Schemas
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                  AxiCAD Inspector Property Panel Controller                   │
│         (Manages Draft Edit Buffers, Field Validation & Mixed States)         │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Submits PropertyCommitRequest
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                     Command Mutation / Transaction Layer                      │
│      (Validates, Executes Mutation, Updates Store Revision & Dirty Flags)     │
└───────────────────────────────────────────────────────────────────────────────┘
```

1. **Чтение из Store**: Инспектор является реактивным подписчиком на состояние выделенного объекта и текущие метаданные Store.
2. **Коммит через Command Mutation**: Любое изменение редактируемого поля формирует и отправляет команду мутации, обеспечивая поддержку истории отмены (Undo/Redo).
3. **Валидация перед коммитом**: Черновик значения проверяется на валидность до отправки команды. Ошибки с блокирующим уровнем запрещают выполнение транзакции.
4. **Запрет прямого проведения записи**: Панель инспектора никогда не производит прямую перезапись файлов диска или внутреннего дерева Store.

---

## 3. Основные сущности и DTO (Core Entities DTOs)

Для управления процессами инспектирования и редактирования параметров в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type PropertyValueSource = 
  | 'toml-biological'
  | 'project-metadata'
  | 'derived-preview'
  | 'runtime-session'
  | 'artifact-metadata'
  | 'diagnostic-state';

export type PropertyEditability = 
  | 'editable'
  | 'readonly'
  | 'readonly-derived'
  | 'readonly-runtime'
  | 'locked-by-active-job'
  | 'unsupported-mixed-selection'
  | 'requires-engine-patchset';

export interface ReadonlyReason {
  code: string;
  message: string;
  blockingJobId?: string;
  blockingJobType?: 'baker-compile' | 'preview-generation' | 'growth-simulation' | 'inference-run';
}

export interface MixedValueState {
  isMixed: boolean;
  distinctValuesCount: number;
  sampleValues: unknown[];
}

export interface PropertyFieldDescriptor {
  fieldId: string;
  displayName: string;
  description?: string;
  valueSource: PropertyValueSource;
  dataType: 'string' | 'number' | 'boolean' | 'enum' | 'vector' | 'color' | 'reference';
  editability: PropertyEditability;
  readonlyReason?: ReadonlyReason;
  enumOptions?: Array<{ label: string; value: unknown }>;
  isRequired: boolean;
  propertyPath: string;
  typedFieldPath?: string;
  targetDocumentPath?: string;
  owningEntityId?: string;
  commandKind?: 'update-toml-field' | 'update-project-metadata' | 'apply-patchset' | 'readonly-display';
}

export interface PropertyPanelDescriptor {
  panelId: string;
  title: string;
  targetEntityType: string;
  fields: PropertyFieldDescriptor[];
}

export interface InspectorSelection {
  selectedEntityIds: string[];
  primaryEntityId?: string;
  commonEntityType?: string;
  isMixedSelection: boolean;
}

export interface InspectorContext {
  selection: InspectorSelection;
  activePanels: PropertyPanelDescriptor[];
  draftValues: Map<string, unknown>; // fieldId -> draftValue
  storeRevision: number;
}

export interface PropertyValidationResult {
  isValid: boolean;
  fieldId: string;
  errorMessage?: string;
  warningMessage?: string;
  severity?: 'info' | 'warning' | 'error';
}

export interface PropertyCommitRequest {
  fieldId: string;
  propertyPath: string;
  typedFieldPath?: string;
  targetDocumentPath?: string;
  affectedEntityIds: string[]; // Required list of entity IDs for batch edits
  newValue: unknown;
  oldValue: unknown; // Used for single-entity edits
  previousValuesByEntityId?: Record<string, unknown>; // Required for batch/mixed selection edits
  valueSource: PropertyValueSource;
}

export interface PropertyCommitResult {
  success: boolean;
  commandId?: string;
  executedRevision?: number;
  validation: PropertyValidationResult;
}

/**
 * Правило отмены групповых изменений (Undo Contract for Batch Edits):
 * 1. Поле oldValue используется исключительно при одиночном редактировании (affectedEntityIds.length === 1).
 * 2. При групповом редактировании нескольких объектов (affectedEntityIds.length > 1) или неоднородных значениях (MixedValueState) обязательным является заполнение словаря previousValuesByEntityId.
 * 3. Командный слой (Command Mutation) обязан использовать previousValuesByEntityId для корректного восстановления исходных индивидуальных значений каждого объекта при выполнении Undo.
 */
```

---

## 4. Источники значений полей (PropertyValueSource Enum)

Инспектор классифицирует каждое отображаемое поле по источнику происхождения его данных (`PropertyValueSource`):

1. **`toml-biological`**: Канонические свойства нейронной сети (имена сом, пороги спайков, тип медиаторов). Сохраняются в `model.toml` / `shard.toml`.
2. **`project-metadata`**: Проектные настройки UI и воркспейсов (цвет подсветки, видимость оверлеев, раскладка). Сохраняются в `axicad.project.json`.
3. **`derived-preview`**: Вычисленные промежуточные данные предпросмотра от AxiEngine. Являются строго Read-Only.
4. **`runtime-session`**: Динамические рантайм-показатели активной симуляции (мембранные потенциалы сом, текущие спайки). Являются Read-Only.
5. **`artifact-metadata`**: Метаданные бинарных файлов (размеры, SHA-256 хэши, авторы). Являются Read-Only.
6. **`diagnostic-state`**: Статусы диагностических ошибок и предупреждений подсистемы валидации. Являются Read-Only.

---

## 5. Режимы редактируемости полей (PropertyEditability Enum)

Каждое поле инспектора вычисляется с определенным уровнем доступности для ввода (`PropertyEditability`):

- **`editable`**: Поле доступно для изменения и коммита через команду.
- **`readonly`**: Поле информационное и недоступно для редактирования по дизайну.
- **`readonly-derived`**: Поле является производным результатом расчётов и не может быть изменено вручную.
- **`readonly-runtime`**: Поле отражает текущее состояние симуляции рантайма и изменяется только сервером AxiEngine.
- **`locked-by-active-job`**: Поле временно заблокировано из-за выполнения активной фоновой задачи (компиляция, рост, симуляция).
- **`unsupported-mixed-selection`**: Поле недоступно для редактирования при групповом выделении сущностей разных типов.
- **`requires-engine-patchset`**: Прямой коммит изменения из инспектора запрещен (`commit-property-change` блокируется). Поле может отображать предлагаемое или предпросматриваемое значение (`proposed / preview value`). Принятие изменений в канонический Store возможно исключительно через подтверждение `PatchSet` от вычислительного ядра AxiEngine с последующим формированием `Command Mutation`. Сгенерированные данные предпросмотра сами по себе не помечают модель как dirty.

---

## 6. Правила флагов загрязнения и временных состояний UI (Dirty & UI State Rules)

Коммит изменений полей через `Command Mutation` приводит к точному обновлению флагов состояния редактора (`Dirty Flags`):

| Источник поля (`PropertyValueSource`) | Генерируемая команда и флуш флагов |
|---|---|
| **`toml-biological`** | Создает команду изменения биологии ➔ Добавляет относительный путь/пути затронутых TOML-документов в список `toml_documents_dirty` и идентификаторы сущностей в `dirty_entities`. Флаг `toml_documents_dirty` является списком путей, а не boolean. |
| **`project-metadata`** | Создает команду изменения настроек проекта ➔ Устанавливает флаг `project_file_dirty = true`. |
| **`derived-preview` / `runtime-session`** | Не создает команд мутации канона. Данные являются производными и не помечают проект как dirty. |
| **Сохраняемый расклад UI инспектора** | Пользовательские настройки ширины панелей, видимости или сохраненных вкладок инспектора помечают проект как `project_file_dirty = true`. |

### Состояния UI инспектора (Transient vs Persisted UI State)
- **Временные состояния (Ephemeral UI State)**: Наведение курсора (hover), ввод в нескоммиченный буфер черновика (draft input), открытый выпадающий список (open dropdown) и транзиентные жесты мыши являются сессионными и **никогда не устанавливают** флаги загрязнения (`dirty = false`).
- **Изменения выделения (Selection Changes)**: Переключение выделенных объектов регулируется правилами `Command Mutation` и спецификацией `selection-engine-spec-ru` / project metadata.

---

## 7. Валидация и жизненный цикл черновиков (Validation Lifecycle)

Редактирование полей в инспекторе происходит через безопасный буфер черновиков (`draft edit buffer`):

1. **Ввод значения (`beginEdit` / `updateDraftValue`)**: При вводе символов пользователем значение обновляется во временном буфере черновика `draftValues` без немедленного вызова команд мутации.
2. **Синтаксическая проверка полей**: На каждом вводе выполняется быстрая проверка синтаксиса (диапазоны чисел, формат регулярок).
3. **Семантическая валидация перед коммитом (`validateDraft`)**: При потере фокуса (Blur) или нажатии Enter подсистема вызывает семантическую валидацию через `Validation Spec` и `Constraint Engine`.
4. **Блокировка при критических ошибках**: Если результат валидации содержит критическую ошибку (`severity === 'error'`), коммит блокируется, а буфер черновика подсвечивается красным цветом.
5. **Коммит при предупреждениях**: Наличие предупреждений (`warning`) не блокирует коммит команды, но отображается во всплывающей подсказке поля.

---

## 8. Жизненный цикл коммита (Commit Lifecycle)

Успешное применение изменений полей проходит через 9 последовательных фаз:

```
[1. beginEdit] ➔ [2. updateDraftValue] ➔ [3. validateDraft] ➔ [4. commitPropertyChange]
                                                                        │
┌───────────────────────────────────────────────────────────────────────┘
▼
[5. executeCommand] ➔ [6. Store Revision++] ➔ [7. Enqueue Validation] ➔ [8. Update Dirty Flags] ➔ [9. Refresh Inspector]
```

---

## 9. Множественное выделение (Mixed Selection)

Инспектирование группы выделенных объектов подчиняется правилам обработки неоднородных данных:

- **Однородные значения**: Если все выделенные объекты одного типа имеют одинаковое значение поля, инспектор отображает это значение.
- **Неоднородные значения (`MixedValueState`)**: Если значения поля у выделенных объектов различаются, поле подсвечивается как `MixedValueState` (например, выводится плейсхолдер `*Multiple Values*`).
- **Групповой коммит (`batch command`)**: Ввод нового значения в неоднородное или групповое поле создает единую атомарную команду `PropertyCommitRequest`, содержащую полный массив `affectedEntityIds` и словарь предыдущих индивидуальных значений `previousValuesByEntityId`. Если `affectedEntityIds.length > 1`, командный слой использует этот словарь для безупречно точного исполнения отмены операции Undo.
- **Несовместимые сущности**: Поля, не являющиеся общими для всех выделенных типов сущностей, скрываются или переводятся в статус `unsupported-mixed-selection`.

---

## 10. Блокировки активных задач (Active Job Locks)

Во время выполнения фоновых вычислительных процессов инспектор защищает целостность симуляций:

- **Блокировка полей**: При активных процессах `baker-compile`, `preview-generation`, `growth-simulation` или `inference-run` поля моделей, модификация которых нарушит валидность текущего запуска, переводятся в режим `locked-by-active-job`.
- **Причина блокировки (`ReadonlyReason`)**: Поле снабжается детальным описанием с указанием ID и типа блокирующей задачи (например, *"Поле заблокировано: выполняется компиляция Baker job #1042"*).
- **Доступность метаданных**: Безопасные визуальные метаданные проекта (выбор цвета, комментарии UI) остаются доступными для редактирования даже во время активных фоновых задач.

---

## 11. Каталог диагностик инспектора (Inspector Diagnostics AXI-INSP-*)

Сбои и ошибки подсистемы инспектирования свойств транслируются через объекты `DiagnosticItem`:

### Каталог диагностик инспектора свойств:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-INSP-001` | `property schema missing` | `'error'` | `None` | Не найдена каноническая схема полей для выделенного типа сущности. |
| `AXI-INSP-002` | `readonly field edit attempted` | `'warning'` | `'commit-property-change'` | Попытка изменить поле, находящееся в режиме Read-Only. |
| `AXI-INSP-003` | `invalid draft value` | `'error'` | `'commit-property-change'` | Введенное в черновик значение не прошло синтаксическую или семантическую валидацию. |
| `AXI-INSP-004` | `mixed selection unsupported` | `'warning'` | `None` | Данное поле не поддерживает групповое редактирование для выбранного набора сущностей. |
| `AXI-INSP-005` | `active job lock` | `'warning'` | `'commit-property-change'` | Редактирование поля заблокировано активным вычислительным процессом ядра. |
| `AXI-INSP-006` | `command mutation failed` | `'error'` | `None` | Сбой исполнения команды изменения свойства на стороне слоя Command Mutation. |
| `AXI-INSP-007` | `stale selection target` | `'info'` | `None` | Выделенный объект был удален или деактивирован во время редактирования поля. |
| `AXI-INSP-008` | `runtime field commit forbidden` | `'error'` | `'commit-property-change'` | Попытка прямой записи вычисляемого рантайм-показателя в канонический Store. |
| `AXI-INSP-009` | `project metadata write failed` | `'error'` | `None` | Ошибка записи изменений метаданных проекта в структуру `axicad.project.json`. |

---

## 12. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [editor-store-spec-ru](editor-store-spec-ru.md) — Спецификация реактивного хранилища и модели состояния редактора.
- [command-mutation-spec-ru](command-mutation-spec-ru.md) — Спецификация командной модели изменения состояния и Undo/Redo.
- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.
- [validation-spec-ru](validation-spec-ru.md) — Спецификация системы валидации и уровней проверок.
- [constraint-engine-spec-ru](constraint-engine-spec-ru.md) — Спецификация ядра проверки ограничений (Constraint Engine).
- [selection-engine-spec-ru](selection-engine-spec-ru.md) — Спецификация ядра выделения объектов (Selection Engine).
- [artifact-cache-registry-spec-ru](artifact-cache-registry-spec-ru.md) — Спецификация реестра артефактов и кэша производных данных.
- [composition-workspace-spec-ru](composition-workspace-spec-ru.md) — Спецификация предметного режима сборки Composition Workspace.
- [connectome-workspace-spec-ru](connectome-workspace-spec-ru.md) — Спецификация предметного режима проектирования связей Connectome Workspace.
- [shard-neuron-editor-workspace-spec-ru](shard-neuron-editor-workspace-spec-ru.md) — Спецификация предметного режима редактора внутренней биологии шарда Shard Neuron Editor.
- [growth-workspace-spec-ru](growth-workspace-spec-ru.md) — Спецификация предметного режима симуляции и отладки роста сети Growth Workspace.
- [inference-runtime-workspace-spec-ru](inference-runtime-workspace-spec-ru.md) — Спецификация предметного режима выполнения симуляции и инференса.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.

---

## 13. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации контракта инспектора свойств и редактирования параметров Inspector Property Editing Contract Spec. Определены DTO сущности, 6 источников полей, 7 режимов доступности, правила флагов загрязнения, 9 этапов коммита и каталог диагностик AXI-INSP. |
| 2026-06-28 | 0.2.0 | Специфицировано инспектирование спайков через связанные сегменты/зонды (read-only derived), добавлена поддержка визуализации пресетов инструментов и источников их переопределений по каскаду приоритетов. (Закрыты Open Decisions #31, 37). |
| 2026-06-27 | 0.1.1 | Точечные доработки: расширены `PropertyFieldDescriptor` и `PropertyCommitRequest`, исправлена семантика `toml_documents_dirty` как списка путей, а не boolean, разграничены Ephemeral и Persisted UI states, подробно описан режим `requires-engine-patchset`, добавлены блокировки `'commit-property-change'` в каталог диагностик, добавлены ссылки на воркспейсы и обновлен Changelog. |
