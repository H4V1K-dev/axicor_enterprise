# Спецификация реестра артефактов и кэша производных данных (Artifact Cache Registry Spec)

> Этот документ формально определяет архитектурный контракт учета, адресации, проверки целостности, отслеживания устаревания и безопасно регулируемой очистки всех производных артефактов (Artifact Cache Registry) на стороне 3D-редактора AxiCAD. Спецификация регламентирует работу подсистемы управления дисковым кэшем тяжелых бинарных данных, скомпилированных моделей, временных предпросмотров и файлов захвата симуляций.

## Status: Draft

---

## 1. Назначение документа (Scope & Non-scope)

Данная спецификация определяет стандарты работы с артефактами, генерируемыми подсистемами редактора и вычислительным ядром AxiEngine.

### Назначение (Scope)
- **Учет скомпилированных артефактов (Baker Compiled Artifacts)**: Регистрация и проверка целостности запеченных нейросетевых моделей.
- **Временные экспортные пакеты (Temporary Export Bundles)**: Управление жизненным циклом сгенерированных экспортных архивов.
- **Артефакты предпросмотра (Engine Preview Artifacts)**: Кэширование промежуточных бинарных геометрий и диагностических оверлеев.
- **Записи симуляций (Growth & Inference Recordings)**: Хранение каскадов кадров роста и временных тиковых записей выполнения рантайма.
- **Файловые захваты стимулов (Input Captures)**: Регистрация бинарных файлов входных данных для детерминированного воспроизведения.
- **Дамп-ряды зондов (Probe Series Dumps)**: Хранение сэмплированных массивов физических величин измерительных приборов.
- **Логи рантайма и дампы прогресса (Runtime Logs & Progress Dumps)**: Учет протоколов работы вычислительного ядра и компилятора.
- **Управление очисткой и квотами (Retention & Cleanup)**: Безопасное удаление устаревших артефактов с учетом связей со сценариями и пользовательских закреплений.

### Вне зоны ответственности (Non-scope)
- Документ **не является** спецификацией канонической биологической модели (источником истины структуры остается TOML).
- Документ **не хранит** секреты, API-токены авторизации, хэндлы устройств и локальные физические URI.
- Документ **не описывает** устройство внутренней оперативной памяти Rust- ядра AxiEngine в реальном времени.
- Документ **не выполняет** функции системы долговременного версионирования исходного кода (Git / Git LFS).
- Удаление любых файлов из дискового кэша артефактов **не должно разрушать проект**, а лишь требовать пересборки или повторной записи данных.

---

## 2. Разграничение и владение данными (Data Ownership & Storage)

Разграничение обязанностей подсистемы кэша артефактов строго соответствует следующей формуле:

> **Artifact Cache Registry manages derived binary artifacts and session files. AxiEngine/Baker generate. AxiCAD registers, verifies, visualizes, and cleans up. Canonical TOML remains immutable.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                          Canonical Biological Source                          │
│                   model.toml / department.toml / shard.toml                   │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Compiles / Generates Preview
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                      AxiCAD Artifact Cache Registry                           │
│       (Manages ArtifactManifest, ChecksumsSha256, Retention & Eviction)       │
└──────────────────┬────────────────────────────────────────┬───────────────────┘
                   │                   │ Storage Offload    │
                   ┌───────────────────┴────────────────────┐
                   ▼                                        ▼
┌──────────────────────────────────────┐  ┌─────────────────────────────────────┐
│          Project Local Cache         │  │       Session Temp Directory        │
├──────────────────────────────────────┤  ├─────────────────────────────────────┤
│(.local-storage/artifacts/ - Captures)│  │ Session-only ephemeral previews/logs│
└──────────────────────────────────────┘  └─────────────────────────────────────┘
```

| Тип данных | Место хранения | Канонический статус | Описание |
|---|---|---|---|
| **Биологическая модель** | `model.toml` / `shard.toml` | **Canonical Source** | Биологическая структура модели. Подсистема артефактов **никогда не изменяет** эти файлы. |
| **Реестр и ссылки на артефакты** | `axicad.project.json` | Project-Scoped Config | Легковесные ссылки (`ArtifactRef`), метаданные сценариев и конфигурации удержания. |
| **Тяжелые файлы производных данных** | Project Cache Directory | Derived Binary Storage | Бинарные файлы артефактов Baker, записи кадров, файлы захвата входов и дамп-ряды. |
| **Сессионные временные файлы** | Session Temp Directory | Ephemeral Temp Storage | Временные предпросмотры, файлы прогресса компиляции и сессионные логи. |

*Инвариант адресации*: Абсолютные пути файловой системы пользователя (`C:\...`, `/tmp/...`) и сессионные дескрипторы файлов категорически запрещено сериализовать в переносимые ссылки `ArtifactRef` или проектный файл JSON.

---

## 3. Основные сущности и DTO (Core Entities DTOs)

Для управления реестром кэша артефактов в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type ArtifactKind = 
  | 'temporary-export-bundle'
  | 'baker-compiled-artifact'
  | 'shard-soma-cache'
  | 'engine-preview'
  | 'growth-recording'
  | 'inference-recording'
  | 'input-capture'
  | 'probe-series-dump'
  | 'runtime-log'
  | 'diagnostic-snapshot';

export type ArtifactStorageKind = 'temp-artifact' | 'project-cache' | 'external-artifact-ref';

export type ArtifactRetentionPolicy = 
  | 'session-only'
  | 'keep-until-project-close'
  | 'keep-for-replay'
  | 'keep-for-scenario-record'
  | 'user-pinned'
  | 'auto-evict';

export interface ArtifactRef {
  artifactId: string;
  artifactKind: ArtifactKind;
  storageKind: ArtifactStorageKind;
  artifactRef: string; // Relative or canonical reference identifier
  relativeCachePath?: string;
  checksumSha256?: string;
  byteLength: number;
}

export interface ArtifactManifest {
  artifactId: string;
  artifactKind: ArtifactKind;
  storageKind: ArtifactStorageKind;
  relativeCachePath?: string;
  externalArtifactRef?: string;
  checksumSha256?: string;
  byteLength: number;
  createdAtIso: string;
  producer: 'axicad' | 'axiengine' | 'baker';
  schemaVersion: string;
  protocolVersion: string;
  engineBuildHash?: string;
  compiledArtifactHash?: string;
  snapshotId?: string;
  storeRevision?: number;
  runConfigHash?: string;
  runtimeInputBindingHash?: string;
  inputCaptureHash?: string;
  retentionPolicy: ArtifactRetentionPolicy;
  isPinned: boolean;
  linkedScenarioIds: string[];
}

export interface ArtifactIntegrityStatus {
  artifactId: string;
  isValid: boolean;
  fileExists: boolean;
  sizeMatches: boolean;
  checksumMatches: boolean;
  isVersionCompatible: boolean;
  failureReason?: 'file-missing' | 'size-mismatch' | 'checksum-mismatch' | 'incompatible-version';
}

export interface ArtifactStaleState {
  artifactId: string;
  isStale: boolean;
  staleReason?: 
    | 'store-revision-changed'
    | 'snapshot-changed'
    | 'schema-version-changed'
    | 'protocol-version-changed'
    | 'engine-updated'
    | 'crate-graph-hash-changed'
    | 'compiled-artifact-updated'
    | 'run-config-changed'
    | 'binding-hash-mismatch'
    | 'input-capture-changed'
    | 'source-toml-missing'
    | 'referenced-entity-missing';
  affectedRef?: string;
}

export interface ArtifactCacheIndex {
  projectDirectory: string;
  totalCacheSizeBytes: number;
  maxCacheQuotaBytes: number;
  registeredArtifacts: Map<string, ArtifactManifest>;
}

export interface ArtifactCleanupPlan {
  planId: string;
  createdAtIso: string;
  artifactsToDelete: string[]; // List of artifactIds
  artifactsBlockedFromDelete: Array<{ artifactId: string; reason: 'user-pinned' | 'scenario-linked' }>;
  freedBytesEstimate: number;
}
```

---

## 4. Классификация видов артефактов (ArtifactKind Enum)

Подсистема реестра классифицирует артефакты по 9 функциональным категориям:

1. **`temporary-export-bundle`**: Временный архив экспортированного пакета модели или проекта.
2. **`baker-compiled-artifact`**: Бинарный артефакт, скомпилированный подсистемой Baker для отправки в AxiEngine.
3. **`engine-preview`**: Промежуточный результат расчета генерации геометрии или диагностического предпросмотра.
4. **`growth-recording`**: Бинарная запись каскада кадров роста сети в Growth Workspace.
5. **`inference-recording`**: Запись симуляционных тиков выполнения рантайма в Inference Runtime Workspace.
6. **`input-capture`**: Бинарный файл захвата внешних стимулов для обеспечения детерминированного повтора.
7. **`probe-series-dump`**: Файловый дамп массивов временных рядов физических величин зондов.
8. **`runtime-log`**: Протоколы работы вычислительного ядра, выводы консоли движка и прогресс компиляции.
9. **`diagnostic-snapshot`**: Точечный снимок диагностического состояния системы при возникновении сбоя.

---

## 5. Обязательные метаданные артефактов (Required Artifact Metadata)

Каждый регистрируемый артефакт в реестре сопровождается обязательным паспортом метаданных `ArtifactManifest`. Подсистема соблюдает следующие условные инварианты (Conditional Invariants):

- **Хранение в кэше проекта (`storageKind === 'project-cache'`)**: Поле `relativeCachePath` является **строго обязательным**.
- **Внешняя ссылка (`storageKind === 'external-artifact-ref'`)**: Поле `externalArtifactRef` является **строго обязательным**.
- **Временные артефакты (`storageKind === 'temp-artifact'`)**: Физический временный путь является сессионным (`session-only`) и **категорически запрещен** к сериализации в качестве переносимой ссылки в `ArtifactRef` или проектном файле JSON.
- **Контрольные суммы (`checksumSha256`)**: Наличие контрольной суммы является обязательным строго по регламенту политики целостности (`Checksum Policy`), а не для абсолютно всех типов артефактов подряд.

---

## 6. Правила хранения и адресации (Storage & Path Rules)

Организация хранилища кэша на диске подчиняется строгим правилам изоляции:

- **Относительные пути кэша проекта**: Файлы кэша проекта хранятся в канонической поддиректории проекта `.local-storage/artifacts/` и адресуются строго относительно корня проекта (`relativeCachePath`).
- **Сессионные временные пути (`temp-artifact`)**: Временные предпросмотры и текущие логи хранятся в системной временной директории сессии. Такие пути помечаются как `session-only` и **никогда не сериализуются** в качестве переносимых ссылок в проектные файлы.
- **Внешние ссылки (`external-artifact-ref`)**: Допускается адресация внешних наборов данных, но в такие ссылки **категорически запрещено** включать секреты, API-токены авторизации и приватные сетевые URI.
- **Адресация вместо внедрения**: Тяжелые бинарные файлы никогда не сериализуются в проектный JSON. Файл `axicad.project.json` хранит исключительно компактные ссылки `ArtifactRef`.

---

## 7. Целостность и валидация кэша (Integrity & Validation)

Подсистема реестра гарантирует надежность данных перед их подачей в вычислительное ядро:

- **Политика контрольных сумм (Checksum Policy)**: Наличие валидного SHA-256 хэша (`checksumSha256`) является **строго обязательным** для артефактов категорий `baker-compiled-artifact`, `input-capture`, `growth-recording`, `inference-recording` и `probe-series-dump`, если они имеют тип хранения `project-cache` (сохранены в проекте) или привязаны к карточкам сценариев (`linkedScenarioIds`).
- **Исключения для временных файлов**: Отсутствие контрольной суммы разрешено исключительно для некритичных сессионных временных файлов с политикой `session-only` (`temp-artifact`).
- **Процедура валидации (`validateArtifactIntegrity`)**: Выявляет физическое отсутствие файла на диске (`file-missing`), расхождение размера в байтах (`size-mismatch`), несоответствие контрольной суммы (`checksum-mismatch`) и версию бинарного протокола (`incompatible-version`).

---

## 8. Политики удержания и очистки (Retention & Cleanup Policies)

Управление дисковым пространством и квотами кэша подчиняется регламентированным стратегиям:

| Стратегия удержания | Описание жизненного цикла |
|---|---|
| **`session-only`** | Автоматически удаляется при завершении текущей сессии редактора или сбросе кэша. |
| **`keep-until-project-close`** | Сохраняется на время работы с проектом и очищается при закрытии проекта. |
| **`keep-for-replay`** | Сохраняется для обеспечения возможностей повтора симуляции в текущей сессии. |
| **`keep-for-scenario-record`** | Сохраняется до тех пор, пока на артефакт ссылается историческая карточка запуска сценария (`ScenarioRunRecord`). |
| **`user-pinned`** | Явно закреплен пользователем от автоматического удаления (`isPinned: true`). |
| **`auto-evict`** | Подлежит автоматическому ротационному вытеснению при превышении дисковой квоты (`maxCacheQuotaBytes`). |

### Предварительное планирование очистки (Cleanup Plan)
Перед проведением процедуры очистки реестр обязан сформировать план `ArtifactCleanupPlan`. Если в списке на удаление оказываются артефакты с флагом `isPinned: true` или с зафиксированными связями в сценариях (`linkedScenarioIds`), очистка блокируется с формированием соответствующих диагностик.

---

## 9. Семантика устаревания (Stale Semantics)

Зарегистрированные артефакты автоматически переводятся в состояние `stale` (устаревшие) при наступлении любого из следующих факторов:

1. **Мутация реактивного хранилища (`storeRevision`)**: Любое изменение канонической структуры биологической модели в Store.
2. **Изменение снимка модели (`snapshotId`)**: Модификация геометрии или параметров компонентов.
3. **Обновление версий схемы или протокола**: Изменение `schemaVersion` или `protocolVersion`.
4. **Обновление движка (`engineBuildHash`)**: Изменение версии ядра AxiEngine.
5. **Перезапекание скомпилированного артефакта (`compiledArtifactHash`)**: Смена главного бинарника модели.
6. **Изменение конфигурации запуска (`runConfigHash`)**: Смена параметров пресета симуляции.
7. **Мутация привязок портов (`runtimeInputBindingHash`)**: Перекоммутация внешних физических устройств.
8. **Изменение хэша захвата стимулов (`inputCaptureHash`)**: Модификация файла записи входных сигналов.
9. **Отсутствие исходных данных**: Удаление исходного TOML-файла или целевой сущности модели.

---

## 10. Операции подсистемы реестра (Registry Operations)

Оркестратор реестра артефактов AxiCAD предоставляет канонический набор функций управления:

- `registerArtifact(manifest)`: Регистрация нового артефакта в индексе кэша.
- `resolveArtifactRef(artifactRef)`: Разрешение относительной ссылки артефакта в актуальный дисковый путь.
- `validateArtifactIntegrity(artifactId)`: Полная проверка целостности и версии артефакта на диске.
- `markArtifactStale(artifactId, reason)`: Перевод артефакта в состояние устаревшего.
- `pinArtifact(artifactId)`: Закрепление артефакта пользователем для защиты от авто-очистки.
- `unpinArtifact(artifactId)`: Снятие пользовательского закрепления.
- `buildCleanupPlan(targetFreedBytes)`: Расчет предварительного плана очистки дискового кэша.
- `executeCleanupPlan(planId)`: Безопасное исполнение плана удаления устаревших и незакрепленных артефактов.
- `exportArtifactBundle(artifactIds, targetPath)`: Упаковка выбранных артефактов во внешний экспортный архив.
- `importArtifactBundle(bundlePath)`: Импорт и проверка целостности стороннего пакета артефактов.

---

## 11. Каталог диагностик кэша (Artifact Diagnostics AXI-ART-*)

Отклонения в работе реестра артефактов и кэша транслируются через канонические объекты `DiagnosticItem`:

### Каталог диагностик кэша артефактов:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-ART-001` | `artifact missing` | `'error'` | `'run-simulation'`, `'replay'`, `'load-artifact'` | Зарегистрированный файл артефакта не найден (блокирует baker-compile только если артефакт является обязательным входным файлом для компиляции). |
| `AXI-ART-002` | `checksum mismatch` | `'error'` | `'run-simulation'`, `'baker-compile'` | Контрольная сумма SHA-256 файла на диске не совпадает с паспортом манифеста. |
| `AXI-ART-003` | `artifact stale` | `'warning'` | `None` / `'run-simulation'` | Артефакт устарел по отношению к текущей редакции модели в Store. |
| `AXI-ART-004` | `incompatible artifact schema` | `'error'` | `'run-simulation'`, `'baker-compile'` | Версия схемы или бинарного протокола артефакта не поддерживается движком. |
| `AXI-ART-005` | `unsafe external artifact ref` | `'error'` | `'save-project'`, `'export-scenario'` | Обнаружена внешняя ссылка на артефакт, содержащая приватные секреты или absolute paths. |
| `AXI-ART-006` | `cache quota exceeded` | `'warning'` | `None` | Занимаемый кэшем объем диска превысил установленный лимит `maxCacheQuotaBytes`. |
| `AXI-ART-007` | `pinned artifact delete blocked` | `'warning'` | `None` | Удаление артефакта заблокировано из-за наличия пользовательского закрепления (`isPinned`). |
| `AXI-ART-008` | `scenario-linked artifact delete blocked` | `'warning'` | `None` | Удаление заблокировано из-за наличия активной связи с карточкой запуска сценария. |
| `AXI-ART-009` | `temp artifact expired` | `'info'` | `None` | Сессионный временный артефакт истек и был автоматически удален при очистке. |

---

## 12. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.
- [baker-compile-pipeline-spec-ru](baker-compile-pipeline-spec-ru.md) — Спецификация пайплайна подготовки и компиляции Baker.
- [engine-preview-pipeline-spec-ru](engine-preview-pipeline-spec-ru.md) — Спецификация пайплайна предпросмотра.
- [growth-workspace-spec-ru](growth-workspace-spec-ru.md) — Спецификация предметного режима симуляции и отладки роста сети.
- [inference-runtime-workspace-spec-ru](inference-runtime-workspace-spec-ru.md) — Спецификация предметного режима выполнения симуляции и инференса.
- [runtime-timeline-probe-spec-ru](runtime-timeline-probe-spec-ru.md) — Спецификация контроллера времени, зондов и метрик симуляции.
- [simulation-scenario-run-preset-spec-ru](simulation-scenario-run-preset-spec-ru.md) — Спецификация сценариев симуляции и пресетов запусков.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.

---

## 13. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-28 | 0.2.0 | Стандартизирован канонический project-local путь кэша `.local-storage/artifacts/` (устранены конкурирующие формулировки), добавлены маркеры инвалидации `crateGraphHash`, формализованы требования ротации/очистки старого кэша и подтвержден инвариант безотказности проекта при очистке дискового кэша. |
| 2026-06-27 | 0.1.1 | Точечные доработки: добавлены поля `runtimeInputBindingHash` и `inputCaptureHash` в `ArtifactManifest`, формализованы условные инварианты адресации и Checksum Policy, уточнены блокировки `AXI-ART-001`, исправлена опечатка и обновлен Changelog. |
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации реестра артефактов и кэша производных данных Artifact Cache Registry Spec. Определены DTO сущности, 9 категорий артефактов, правила валидации контрольных сумм SHA-256, политики удержания и каталог диагностик AXI-ART. |
