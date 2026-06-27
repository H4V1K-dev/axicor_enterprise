# Спецификация пользовательских настроек и профилей рабочих пространств (User Preferences & Workspace Profiles Spec)

> Этот документ формально определяет архитектурный контракт управления пользовательскими настройками (User Preferences), профилями рабочих пространств (Workspace Profiles) и локальными конфигурационными пресетами на стороне 3D-редактора AxiCAD. Спецификация регламентирует правила хранения, иерархию приоритетов, изолированность от канонического TOML-кода модели, управление горячими клавишами и безопасность импорта/экспорта.

## Status: Draft

---

## 1. Назначение документа (Scope & Non-scope)

Данная спецификация определяет стандарты работы с индивидуальными пользовательскими конфигурациями рабочей среды редактора.

### Назначение (Scope)
- **Глобальные пользовательские настройки (User Preferences)**: Хранение индивидуальных параметров интерфейса, тем и конфигураций графики.
- **Профили рабочих пространств (Workspace Profiles)**: Управление пресетами панелей, активными воркспейсами и раскладками осциллографов.
- **Визуальное оформление и плотность (UI Theme & Density)**: Конфигурирование цветовых тем, масштабирования элементов и компактности панелей.
- **Дефолтные настройки инструментов (Tool Defaults)**: Фиксация исходных параметров сетки, прилипания (`snap`), выделения и кистей.
- **Привязка горячих клавиш и палитра команд (Shortcuts & Command Palette)**: Управление комбинациями клавиш и символьными алиасами команд.
- **Предпочтения графики и кэша (Render & Cache Preferences)**: Конфигурирование лимитов дискового пространства и пресетов качества рендеринга.
- **Локальные профили связывания рантайма (Runtime Binding Preferences)**: Хранение безопасных пользовательских конфигураций входов/выходов.
- **Безопасный импорт и экспорт (Safe Import/Export)**: Упаковка и перенос профилей предпочтений без утечки приватных данных и секретов.

### Вне зоны ответственности (Non-scope)
- Документ **не является** спецификацией канонической биологической модели (источником истины структуры остается TOML).
- Документ **не владеет** полной схемой project-scoped metadata; описывает только preference/profile-срез, который может сохраняться в axicad.project.json.
- Документ **не хранит** секреты, API-токены авторизации, хэндлы системных устройств и физические абсолютные пути.
- Документ **не описывает** низкоуровневую реализацию защищенных хранилищ операционной системы (OS Credential Vault).

---

## 2. Разграничение и владение хранилищем (Storage Ownership)

Управление пользовательскими конфигурациями и метаданными подчиняется строгому разделению уровней ответственности:

> **User Preferences manage machine-local UI defaults. Workspace Profiles adapt panel layouts. Project File stores project-scoped configs. OS Vault secures credentials. Canonical TOML remains untouched.**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                          Canonical Biological Source                          │
│                   model.toml / department.toml / shard.toml                   │
└──────────────────────────────────────┬────────────────────────────────────────┘
                                       │ Read-Only for Preferences
                                       ▼
┌───────────────────────────────────────────────────────────────────────────────┐
│                     AxiCAD Preference & Profile Manager                       │
│        (Evaluates Precedence: Session -> Project -> User-Global -> Default)   │
└──────┬───────────────────────────────┬───────────────────────────────┬────────┘
       │                               │                               │
       ▼                               ▼                               ▼
┌──────────────┐              ┌───────────────────┐          ┌──────────────────┐
│ Session Vault│              │  Project File     │          │Global Preferences│
│ (Temp/Secrets│              │axicad.project.json│          │(~/.axicad/config │
│ In-Memory)   │              │ (Project Profile  │          │ User Preferences │
└──────────────┘              └───────────────────┘          └──────────────────┘
```

| Уровень хранения | Место хранения | Область видимости | Содержимое |
|---|---|---|---|
| **Biological Truth** | `model.toml` / `shard.toml` | **Canonical Model** | Каноническое описание структуры нейронной сети. Настройки **никогда не записываются** сюда. |
| **Project-Scoped Config** | `axicad.project.json` | Project Local | Специфичные для проекта раскладки панелей, активные сценарии и локальные профили воркспейсов. |
| **User-Global Preferences** | Пользовательский каталог (`~/.axicad/`) | Machine / User Local | Глобальные темы, настройки графики, дефолтные горячие клавиши и параметры квоты кэша. |
| **Secure Credential Vault** | OS Secure Vault / Session Memory | Session / Encrypted | Секреты, API-токены, хэндлы аппаратных устройств и приватные URI. |
| **Transient Session State** | Session Memory | Ephemeral | Временные оперативные переопределения параметров в текущей сессии редактора. |

---

## 3. Основные сущности и DTO (Core Entities DTOs)

Для работы с подсистемой предпочтений и профилей в AxiCAD определены следующие TypeScript-интерфейсы:

```typescript
export type WorkspaceProfileScope = 'user-global' | 'project-local' | 'session-temporary';
export type ThemePreference = 'dark-slate' | 'dark-cyber' | 'light-clean' | 'system-auto';
export type UIDensityPreference = 'comfortable' | 'compact' | 'touch-friendly';

export interface ShortcutBinding {
  bindingId: string;
  commandId: string;
  keyCombo: string; // e.g. "Ctrl+Shift+B" or "Cmd+B"
  platform: 'all' | 'windows' | 'mac' | 'linux';
  contextScope?: 'global' | 'viewport' | 'inspector' | 'timeline';
  enabled: boolean;
  sourceScope?: WorkspaceProfileScope | 'built-in-default';
  priority?: number;
}

export interface CommandPaletteAlias {
  aliasId: string;
  commandId: string;
  displayText: string;
  customKeywords: string[];
}

export interface RenderQualityPreset {
  presetId: string;
  targetFps: number;
  enableAntialiasing: boolean;
  enableShadows: boolean;
  useInstancedMesh: boolean;
  maxRenderDistance: number;
}

export interface CachePreference {
  maxCacheQuotaBytes: number;
  autoEvictUnusedArtifacts: boolean;
  defaultRetentionPolicy: 'session-only' | 'keep-until-project-close' | 'auto-evict';
}

export interface RuntimeBindingPreference {
  bindingProfileId: string;
  displayLabel: string;
  preferredCodecPreset?: string;
  credentialRef?: string; // Opaque safe ID for OS Vault/Session Vault reference
  isExportable: boolean; // Must be false for credentials
}

export interface ToolDefaultProfile {
  defaultSnapToGrid: boolean;
  gridSpacingVoxels: number;
  selectionMode: 'single' | 'box' | 'lasso' | 'connected';
  routePreviewQuality: 'fast' | 'exact';
  neuronEditorBrushRadius: number;
}

export interface WorkspaceProfile {
  profileId: string;
  name: string;
  scope: WorkspaceProfileScope;
  defaultActiveWorkspaceId: string;
  defaultToolId: string;
  visiblePanelIds: string[];
  panelWidths: Record<string, number>;
  oscilloscopeLayoutId?: string;
  inspectorLayoutId?: string;
}

export interface UserPreferences {
  version: string;
  theme: ThemePreference;
  uiDensity: UIDensityPreference;
  renderQuality: RenderQualityPreset;
  cache: CachePreference;
  toolDefaults: ToolDefaultProfile;
  shortcuts: ShortcutBinding[];
  commandAliases: CommandPaletteAlias[];
  runtimeBindings: RuntimeBindingPreference[];
  activeProfileId?: string;
}

export interface SafeUserPreferencesExport {
  theme?: ThemePreference;
  uiDensity?: UIDensityPreference;
  renderQuality?: RenderQualityPreset;
  cacheQuotaBytes?: number;
  toolDefaults?: Partial<ToolDefaultProfile>;
  shortcuts?: ShortcutBinding[];
  commandAliases?: CommandPaletteAlias[];
  workspaceProfiles?: WorkspaceProfile[];
}

export interface PreferenceExportManifest {
  manifestId: string;
  exportedAtIso: string;
  appVersion: string;
  preferences: SafeUserPreferencesExport;
  checksumSha256: string;
}
```

---

## 4. Иерархия и приоритет профилей (Profile Precedence & Merge Rules)

Вычисление финального значения любого параметра интерфейса подчиняется строгой 4-уровневой иерархии приоритетов (Profile Precedence):

1. **`session temporary override` (Высший приоритет)**: Временные переопределения в оперативной памяти текущей сессии (например, временное переключение сетки).
2. **`project-local workspace profile`**: Локальные настройки текущего проекта, сохраненные в `axicad.project.json`.
3. **`user-global preference`**: Глобальные пользовательские настройки из файла `~/.axicad/preferences.json`.
4. **`built-in default` (Низший приоритет)**: Встроенные жестко закодированные значения редактора по умолчанию.

### Правила слияния и разрешения конфликтов (Merge Rules & Precedence)
- **Вычисление эффективных настроек (Effective Preferences)**: Иерархия приоритетов используется для динамического вычисления текущего рабочего состояния интерфейса в оперативной памяти. Более высокий слой **физически не перезаписывает** значения нижних слоев на диске.
- **Сохранение проектных переопределений**: Переопределение параметров на уровне проекта (`project-local`) сохраняется исключительно в проектном файле `axicad.project.json` и **не модифицирует** глобальный файл конфигурации пользователя (`~/.axicad/preferences.json`).
- **Сессионные оперативные переопределения**: Оперативные переопределения сессии (`session-temporary`) хранятся строго в RAM и **не производят запись** ни в проектный JSON, ни в файл конфигурации пользователя.
- **Массивы горячих клавиш и алиасов**: Массивы горячих клавиш (`shortcuts`) и алиасов команд (`commandAliases`) объединяются по ключам `commandId` и `aliasId` с учетом приоритета слоя (`priority`) и области источника (`sourceScope`). При конфликтах платформа-зависимых комбинаций приоритет отдается комбинации, явно указанной для текущей ОС.

---

## 5. Профили рабочих пространств (Workspace Profiles)

Профили рабочих пространств управляют визуальной компоновкой графического интерфейса:

- **Параметризация профиля**: Профиль определяет размеры и видимость панелей, активное рабочее пространство по умолчанию (`Composition`, `Connectome`, `Shard Neuron Editor`, `Growth`, `Inference`), дефолтный инструмент и раскладку осциллографов.
- **Локальные проектные профили (`project-local`)**: Сохраняются в подструктуре проектного файла `axicad.project.json` и автоматически активируются при открытии данного проекта.
- **Глобальные профили (`user-global`)**: Доступны пользователю во всех проектах и сохраняются в глобальной конфигурации пользователя.
- **Безопасный экспорт**: При экспорте профилей рабочих пространств в файлы переносимых пакетов в манифест включаются только безопасные поля геометрии окон и идентификаторы панелей.

---

## 6. Настройки инструментов по умолчанию (Tool Defaults)

Подсистема фиксирует исходные рабочие параметры интерактивных инструментов:

- **Конфигурируемые параметры**: Привязка к воксельной сетке (`defaultSnapToGrid`), шаг сетки, режимы выделения объектов, качество предварительной трассировки связей и радиус кистей биологического редактора.
- **Фундаментальный политический инвариант**: Изменение любых настроек инструментов по умолчанию в инспекторе или окне предпочтений **никогда не изменяет модель** и не генерирует команды мутации до тех пор, пока инженер явно не выполнит интерактивное действие инструментом в Viewport.

---

## 7. Горячие клавиши и палитра команд (Shortcuts & Command Palette)

Организация горячих клавиш и текстовых команд подчиняется следующим правилам:

- **Абстракция через `commandId`**: Горячие клавиши привязываются строго к символьным идентификаторам команд (например, `editor.undo`, `viewport.focus-selection`), а не к вызовам конкретных функций или обработчикам событий.
- **Конфликтная детекция (`shortcut conflict`)**: При попытке назначить одну и ту же комбинацию клавиш на две разные команды в одном контекстном скоупе система выставляет предупреждение и генерирует диагностику `AXI-PREF-002`.
- **Кроссплатформенная абстракция**: Комбинации клавиш описываются абстрактно (например, использование модификатора `CmdOrCtrl`). Платформа-зависимые комбинации фильтруются по полю `platform`.

---

## 8. Предпочтения рантайм-связей и безопасность (Runtime Binding Preferences & Security)

Работа с конфигурациями внешнего оборудования и портов симуляции требует соблюдения политики безопасности:

> [!CAUTION]
> В файлы пользовательских предпочтений (`UserPreferences`) и проектные файлы `axicad.project.json` **категорически запрещено** записывать сырые секреты авторизации, API-токены, физические дескрипторы системных драйверов и приватные IP-адреса оборудования. 
> Предпочтения хранят только безопасные логические идентификаторы профилей (`bindingProfileId`), отображаемые метки (`displayLabel`), наименования кодеков и ссылку на учетные данные `credentialRef?: string`. Поле `credentialRef` представляет собой непрозрачный безопасный идентификатор (`opaque safe id`) для обращения к защищенному хранилищу OS Vault или сессионному сейфу. Оно не содержит внутри никаких секретов, но по соображениям приватности имеет флаг `isExportable: false` и **категорически не попадает** в переносимый манифест экспорта.

---

## 9. Предпочтения кэша и рендеринга (Cache & Render Preferences)

Конфигурирование производительности графики и распределения дисковых ресурсов подчиняется следующим правилам:

- **Лимиты дискового кэша**: Глобальный лимит объема кэша (`maxCacheQuotaBytes`) задается в пользовательских настройках. Проект может переопределять отдельные аспекты политики очистки, не нарушая глобальные дисковые лимиты.
- **Пресеты качества рендеринга**: Пользовательские настройки задают целевую частоту кадров (`targetFps`), сглаживание (`enableAntialiasing`), тени и использование инстансинга (`useInstancedMesh`). При создании снимков или видеозаписей проектов проект может временно использовать явную визуальную конфигурацию рендеринга.
- **Изоляция от канона**: Настройки графики и кэша являются строго опциями редактора и **никогда не влияют** на результаты бинарной компиляции Baker или каноническую биологическую структуру TOML-файлов.

---

## 10. Правила флагов загрязнения (Dirty Rules)

Изменение любых параметров в подсистеме предпочтений подчиняется четкой дифференциации флагов загрязнения (`Dirty Flags`):

| Категория изменений | Генерируемые флаги загрязнения |
|---|---|
| **Глобальные пользовательские настройки (`user-global`)** | Сохраняются в глобальный файл пользователя `~/.axicad/preferences.json` ➔ **НЕ взводят** `project_file_dirty`. |
| **Локальный профиль проекта (`project-local`)** | Модифицирует структуру проектного файла ➔ Устанавливает флаг `project_file_dirty = true`. |
| **Сессионные переопределения (`session-temporary`)** | Сохраняются в оперативной памяти сессии ➔ **НЕ взводят** никаких флагов загрязнения. |
| **Биологическая модель (`toml-biological`)** | Подсистема предпочтений **НИКОГДА не взводит** флаг `toml_documents_dirty` и не модифицирует `dirty_entities`. |

---

## 11. Безопасный импорт и экспорт (Safe Import / Export Policies)

Перенос пользовательских конфигураций между рабочими станциями регламентируется правилами защиты приватности:

- **Использование безопасного DTO экспорта**: В качестве структуры экспорта полей используется исключительно специальный DTO `SafeUserPreferencesExport`, разрешающий доступ только к безопасным визуальным настройкам (`theme`, `uiDensity`, `renderQuality`, safe cache limits, `toolDefaults`, `shortcuts`, `commandAliases`, `workspaceProfiles`).
- **Автоматическое исключение приватных данных**: Из пакетов экспорта принудительно исключаются `runtimeBindings`, ссылки на учетные данные (`credentialRef`), сессионные переопределения, локальные физические пути, дескрипторы системных устройств и приватные сетевые URI.
- **Валидация при импорте**: Сторонний манифест профилей проходит строгую синтаксическую проверку схемы и проверку версии приложения (`appVersion`). Небезопасные или нераспознанные поля отбрасываются с выведением предупреждения `AXI-PREF-003`.
- **Разрешение конфликтов**: При импорте пользователь может выбрать стратегию объединения (замена имеющихся комбинаций или сохранение текущих локальных настроек).

---

## 12. Каталог диагностик предпочтений (Preferences Diagnostics AXI-PREF-*)

Сбои и конфликты подсистемы настроек транслируются через объекты `DiagnosticItem`:

### Каталог диагностик пользовательских предпочтений:

| Код ошибки | Символьное имя | Severity | Блокируемые операции | Описание |
|---|---|---|---|---|
| `AXI-PREF-001` | `preference schema mismatch` | `'error'` | `'import-preferences'` | Файл предпочтений имеет несовместимую структуру или версию схемы. |
| `AXI-PREF-002` | `shortcut conflict` | `'warning'` | `'save-preferences'`, `'activate-shortcut-profile'` | Обнаружен конфликт горячих клавиш в одном активном контексте (блокирует сохранение/активацию). Для потенциальных конфликтов или disabled привязок (`enabled: false`) выставляется предупреждение без блокировки (`None`). |
| `AXI-PREF-003` | `unsafe preference export field` | `'warning'` | `'export-preferences'` | Попытка включения приватных системных данных в переносимый манифест экспорта (очищено). |
| `AXI-PREF-004` | `missing workspace profile` | `'warning'` | `None` | Запрошенный профиль рабочего пространства не найден в локальном или проектном индексе. |
| `AXI-PREF-005` | `invalid render quality preset` | `'warning'` | `None` | Указанный пресет качества рендеринга содержит недопустимые параметры производительности. |
| `AXI-PREF-006` | `cache quota invalid` | `'error'` | `'save-preferences'` | Задано недопустимое или слишком малое значение лимита дискового кэша. |
| `AXI-PREF-007` | `secure credential missing` | `'error'` | `'connect-runtime'` | Запрошенный профиль рантайма ссылается на отсутствующий токен авторизации в OS Vault. |
| `AXI-PREF-008` | `project/user profile conflict` | `'info'` | `None` | Параметры локального проектного профиля переопределили глобальные настройки пользователя. |
| `AXI-PREF-009` | `unsupported platform shortcut` | `'info'` | `None` | Комбинация горячих клавиш не поддерживается в текущей операционной системе. |

---

## 13. Ссылки на контекстные документы (References)

Данная спецификация опирается на следующие канонические документы экосистемы AxiCAD:

- [project-file-spec-ru](project-file-spec-ru.md) — Спецификация файла проекта `axicad.project.json`.
- [workspace-shell-layout-spec-ru](workspace-shell-layout-spec-ru.md) — Спецификация архитектуры интерфейса и зон раскладки.
- [tool-system-spec-ru](tool-system-spec-ru.md) — Спецификация интерактивных инструментов (Tool System).
- [inspector-property-editing-contract-spec-ru](inspector-property-editing-contract-spec-ru.md) — Спецификация контракта инспектора свойств и редактирования параметров.
- [artifact-cache-registry-spec-ru](artifact-cache-registry-spec-ru.md) — Спецификация реестра артефактов и кэша производных данных.
- [external-port-io-spec-ru](external-port-io-spec-ru.md) — Спецификация внешних портов ввода/вывода.
- [runtime-timeline-probe-spec-ru](runtime-timeline-probe-spec-ru.md) — Спецификация контроллера времени, зондов и метрик симуляции.
- [diagnostics-error-catalog-spec-ru](diagnostics-error-catalog-spec-ru.md) — Каталог диагностик и спецификация ошибок.

---

## 14. История изменений (Changelog)

| Дата | Версия | Описание изменений |
|---|---|---|
| 2026-06-27 | 0.1.0 | Первоначальное создание спецификации пользовательских настроек и профилей рабочих пространств User Preferences & Workspace Profiles Spec. Определены DTO сущности, 4 уровня приоритета профилей, правила защиты секретов, дифференциация флагов загрязнения и каталог диагностик AXI-PREF. |
| 2026-06-27 | 0.1.1 | Точечные доработки: введен `SafeUserPreferencesExport` DTO, расширены `ShortcutBinding` и `RuntimeBindingPreference`, уточнена семантика effective preferences и блокировок `AXI-PREF-002`, скорректирован Non-scope и обновлен Changelog. |

