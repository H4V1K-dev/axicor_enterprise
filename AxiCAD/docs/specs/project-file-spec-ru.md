# Спецификация файла проекта AxiCAD (Project File Spec)

> Этот документ формально описывает структуру, JSON-схему и логику работы служебного конфигурационного файла проекта `axicad.project.json`. Файл хранит исключительно метаданные среды проектирования 3D-редактора AxiCAD, сохраняя чистые файлы TOML DSL Axicor от загрязнения небиологическими параметрами визуализации.

## Status: Draft

---

## 1. Назначение документа

Файл `axicad.project.json` является основным контейнером состояния рабочей области редактора AxiCAD. Он располагается в корневой директории проекта рядом с биологическим конфигурационным файлом `model.toml`.

### Вне зоны ответственности (Non-goals)
- Файл проекта не содержит биологических или симуляционных параметров (они определены в TOML-файлах).
- Файл проекта не экспортируется сборщику Baker и не влияет на рантайм симулятора.
- Файл не хранит полную историю Undo/Redo стека (стек команд является сессионным и хранится в оперативной памяти).

---

## 2. Базовые принципы (Core Principles)

- **Разделение ответственности (Data Separation)**: Биологические параметры сети описываются каноническим TOML DSL, в то время как 3D-позиционирование, состояние интерфейса и настройки вьюпорта записываются в JSON проекта.
- **Временные UUID (Session-Local Identity)**: Уникальные UUID записей Store генерируются динамически при запуске сессии и не сохраняются на диске. 
- **Типизированные пути (Typed Paths)**: Для восстановления связей между визуальными метаданными и биологическими сущностями в JSON-файле проекта используются вычисляемые типизированные пути (например, `shard:SensoryCortex.Retina`).
- **Устойчивость к устареванию (Stale Reference Tolerance)**: В процессе редактирования TOML-файлы могут изменяться внешними утилитами. Редактор AxiCAD должен корректно обрабатывать ситуации, когда в `axicad.project.json` содержатся метаданные для путей, которые были удалены или изменены в TOML, помещая их в карантин.
- **Переносимость проекта (Portability)**: Проект должен быть полностью переносимым между дисками и машинами. Использование абсолютных файловых путей в канонической схеме запрещено.
- **Гибридный формат хранения (Hybrid Storage Architecture & Heavy Payloads)**: Отклонена идея единого монолитного бинарного файла проекта. Файл `axicad.project.json` выступает исключительно легким человекочитаемым манифестом метаданных (manifest/project metadata). Тяжелые производные или сгенерированные данные (артефакты компилятора Baker, кэш сом `shard-soma-cache`, записи симуляций) сохраняются в виде отдельных бинарных файлов в хранилище производных данных (`.local-storage/artifacts/` или дисковый кэш проекта). Ссылки, версии схем и SHA256-хэши этих файлов фиксируются через Artifact Registry.
- **Сохранение пресетов камер (Camera Presets & Saved Views)**: Пользовательские пресеты камер и сохраненные ракурсы сцены сохраняются в метаданных проекта внутри секции `layout_state` (или связанных профилей вьюпорта в JSON). Они не сериализуются в биологический TOML и не влияют на физическую модель.

---

## 3. Схема верхнего уровня (Top-Level Schema)

В схеме `axicad.project.json` строго разделены обязательные системные поля и опциональные блоки настроек интерфейса.

### 3.1 Обязательные поля (Required)
- `project_id` (String): Уникальный UUID проекта (генерируется один раз при создании проекта).
- `schema_version` (u32): Версия JSON-схемы метаданных проекта (для контроля обратной совместимости).
- `active_model` (String): Имя корневого файла модели (обычно `"model.toml"`). Все относительные пути проекта вычисляются от расположения этого файла.

### 3.2 Опциональные поля с дефолтными значениями (Optional/Defaulted)

| Поле | Тип данных | Дефолтное значение | Описание |
|---|---|---|---|
| `axicad_version` | String | `"1.0.0"` | Версия редактора AxiCAD, записавшего файл. |
| `created_at` | String | ISO метка | ISO-дата создания проекта. |
| `updated_at` | String | ISO метка | ISO-дата последнего сохранения проекта. |
| `workspace_root` | String | `null` | Опциональное поле для отладки. В каноническом виде опускается ради переносимости проекта. |
| `layout_state` | Object | `{}` | Состояние интерфейса, размеры панелей и 3D-камера (заменяет устаревший объект `camera`). |
| `viewport` | Object | `{}` | Настройки визуализации (фильтры, сетки). |
| `composition_levels` | Array | `[]` | Ограничивающие плоскости и диапазоны уровней композиции на сцене. |
| `department_layout` | Array | `[]` | Координаты размещения департаментов, привязка к уровням композиции и флаги видимости. |
| `placements` | Array | `[]` | Массив локальных относительных координат шардов внутри департаментов. |
| `expanded_tree_nodes`| Array | `[]` | Список типизированных путей, раскрытых в UI дерева проекта. |
| `selection` | Array | `[]` | Список выделенных сущностей (типизированные пути). |
| `diagnostics_cache` | Array | `[]` | Кэш диагностических сообщений, привязанных к путям. |
| `dirty_state` | Object | `{ "project_file_dirty": false, "toml_documents_dirty": [] }` | Состояние несохраненных изменений на момент сохранения. |
| `migrations` | Object | `{ "history": [] }` | История переименований и миграций путей. |

---

## 4. Типизированные пути (Typed Paths)

Типизированный путь однозначно сопоставляет метаданные с биологической сущностью в обход временных UUID сессии.

- **Синтаксический формат**: `<entity_type>:<domain_path>`
- **Допустимые типы (entity_type)**: `model`, `department`, `shard`, `layer`, `neuron_type`, `socket`, `port`, `pin`, `connection`.
- **Примеры путей**:
  - `department:SensoryCortex` (департамент)
  - `shard:SensoryCortex.Retina` (шард Retina в SensoryCortex)
  - `socket:SensoryCortex.Retina.cross_modal` (сокет шарда)

### Поведение при переименованиях (Rename Migration)
Когда пользователь переименовывает доменный элемент (например, департамент `SensoryCortex` $\to$ `VisualCortex`), редактор автоматически обновляет все префиксы путей в Store и прописывает запись в секцию `migrations` файла проекта, чтобы при загрузке старых метаданных настройки корректно привязались к новому пути.

---

## 5. Макет и вьюпорт (Layout and Viewport State)

Параметры восстанавливают рабочую область пользователя, размеры панелей и состояние камеры в 3D-сцене:

### 5.1 Объект `layout_state`
- `activeWorkspaceMode` (String): Имя активного режима работы (например, `"Composition"`).
- `panelCollapsed` (Object): Состояние свертывания панелей `{ "left": boolean, "right": boolean, "bottom": boolean }`.
- `panelSizes` (Object): Размеры выдвинутых панелей `{ "left": number, "right": number, "bottom": number }`.
- `bottomPanelMode` (String): Активная вкладка нижней панели (например, `"diagnostics"`, `"timeline"`, `"logs"`).
- `lastViewportCamera` (Object): Состояние камеры вьюпорта:
  - `position` (Array of f32): 3D-вектор координат камеры `[X, Y, Z]`.
  - `target` (Array of f32): Точка направления обзора камеры `[X, Y, Z]`.
  - `fov` (f32): Угол обзора в градусах.
- `workspaceModeLayoutPresets` (Optional Object): Специфические для режимов пресеты размеров панелей.

### 5.2 Объект `viewport`
- `render_mode` (String): Режим отображения. Дефолт: `"solid"`.
- `visibility_filters` (Array of String): Список скрытых элементов. Дефолт: `[]`.
- `grid_visible` (Boolean): Флаг видимости воксельной сетки. Дефолт: `true`.

---

## 6. Размещения шардов (Placements)

Каждый шард визуализируется на 3D-сцене в соответствии со своим размещением. Локальные координаты шардов в JSON не пересекаются с биологическим TOML-контрактом.

### Свойства элементов `placements`:
- `entityPath` (String, Required): Стабильный типизированный путь к шарду (например, `shard:SensoryCortex.Retina`). Сохранение временных runtime-UUID запрещено.
- `position` (Array of f32, Required): Локальное относительное смещение шарда в микрометрах (department-local offset) внутри родительского департамента `[X, Y, Z]`.
  - *Формула глобального положения*: Глобальное положение шарда в 3D-пространстве сцены вычисляется как сумма глобальных координат департамента и относительных координат шарда:
    $$\text{Global\_Position}_{\text{shard}} = \text{department\_layout.position} + \text{placements.position}_{\text{shard}}$$
- `rotation` (Array of f32, Required): Кватернион вращения шарда `[X, Y, Z, W]`.
- `visible` (Boolean, Required): Скрыт ли шард во вьюпорте (`visible = false`).
- `locked` (Boolean, Required): Заблокирован ли шард от перемещения мышью (`locked = true`).
- `colorOverride` (String, Optional): Пользовательский цвет для HUD-подсветки шарда.

---

## 7. Состояние интерфейса (UI State)

Хранит состояние UI для бесшовного продолжения сессии (никогда не экспортируется в TOML):

- **`expanded_tree_nodes`** (Array of String): Массив типизированных путей, ветки которых развернуты в UI дерева проекта (дефолт `[]`).
- **`selection`** (Array of String): Список типизированных путей сущностей, выделенных в момент закрытия проекта (дефолт `[]`).


---

## 7.1 Уровни композиции (Composition Levels)

Плоскости разметки и группировки (vertical bands) в трехмерном пространстве (editor-only metadata):

- **`id`** (String, Required): Уникальный стабильный идентификатор уровня (например, `"level_sensory"`).
- **`name`** (String, Required): Пользовательское имя плоскости (например, `"Sensory Level"`).
- **`order`** (u32, Required): Порядковый номер расположения уровня по высоте (z-index / vertical ordering).
- **`visible`** (Boolean, Required): Скрывает/показывает все дочерние департаменты.
- **`locked`** (Boolean, Required): Запрещает перемещение любых элементов на данном уровне.
- **`height`** (f32, Required): Высота расположения уровня на сцене.
- **`colorTag`** (String, Optional): Пользовательский цвет плоскости для визуального разделения.

---

## 7.2 Раскладка департаментов (Department Layout)

Пространственное положение контейнеров департаментов в глобальных world/model координатах (editor-only metadata):

- **`entityPath`** (String, Required): Стабильный типизированный путь к департаменту (например, `department:SensoryCortex`).
- **`assignedCompositionLevel`** (String, Required): Ссылка на идентификатор `id` из `composition_levels`, к которому привязан департамент.
- **`position`** (Array of f32, Required): Координаты локального центра департамента в глобальной системе world/model `[X, Y, Z]`.
- **`visible`** (Boolean, Required): Скрывает/показывает департамент и все его шарды.
- **`locked`** (Boolean, Required): Запрещает перемещение департамента и его шардов.
- **`collapsed`** (Boolean, Required): Сворачивает визуальное отображение департамента до одной компактной точки-контейнера в 3D.
- **`boundsMode`** (String, Optional): Режим расчета ограничивающего бокса AABB: `"computed"` (динамически вычисляется по дочерним шардам, значение по умолчанию) или `"fixed"` (жестко заданный размер).
- **`fixedBounds`** (Object, Optional): Размеры контейнера при `"fixed"` режиме: `{ "size": [f32, f32, f32] }`.

> [!NOTE]
> Динамически рассчитываемый ограничивающий бокс `computedAABB` департамента никогда не сериализуется как источник истины, а пересчитывается редактором "на лету" на основе локальных координат шардов.

---

## 8. Кэш диагностик (Diagnostics Cache)

Сохраняет список ошибок и предупреждений сессии. Поскольку валидация может занимать время, загрузка кэша диагностик позволяет мгновенно подсветить проблемные зоны до завершения фонового прогона проверок.

### Свойства элементов `diagnostics_cache`:
- `typedPath` (String, Required): Типизированный путь к невалидной сущности.
- `tomlFieldPath` (String, Required): Относительный путь свойства внутри TOML (например, `neuron_types.Stellate_Exc.membrane.threshold`).
- `code` (String, Required): Унифицированный код ошибки.
- `severity` (String, Required): Серьезность (`"error"` / `"warning"` / `"info"`).
- `message` (String, Required): Текстовое описание проблемы на русском языке.
- `suggested_fix` (String, Optional): Рекомендация по исправлению.
- `generated_at` (String, Required): Временная метка генерации ошибки.

---

## 9. Dirty State

Содержит метаданные о незафиксированных изменениях на момент сохранения проекта.

- `project_file_dirty` (Boolean): Флаг наличия несохраненных изменений самого служебного JSON-файла проекта. После успешной записи `axicad.project.json` этот флаг сбрасывается в `false`.
  - *Правила изменения*: Изменение полей `layout_state`, `composition_levels`, `department_layout` и относительных позиций в `placements` взводит **исключительно** флаг `project_file_dirty = true` (не влияет на TOML).
- `toml_documents_dirty` (Array of String): Список путей к TOML-файлам биологического описания, которые были изменены в Store, но ещё не экспортированы на диск. Очищается только при запуске операции экспорта TOML.
  - *Смешанные мутации*: Команды создания, удаления или изменения путей биологических сущностей (например, `CreateShardCommand`) порождают как изменение биологических TOML в Store (`toml_documents_dirty`), так и генерацию/удаление дефолтных пространственных метаданных (`project_file_dirty = true`). В этих сценариях выставляются **оба** dirty-флага.
- `dirty_entities` (Array of String): Список типизированных путей измененных сущностей, требующих фоновой перевалидации.
- `last_saved_at` (String): ISO-дата последней операции сохранения.
- *Примечание*: Сохранение файла проекта разрешено при наличии любых фатальных ошибок валидации биологической модели.


---

## 10. Миграции (Migrations)

При изменении структуры биологических папок или переименовании сущностей, файл проекта хранит карту перенаправлений для старых типизированных путей.

```json
"migrations": {
  "history": [
    {
      "type": "rename",
      "old_path": "shard:SensoryCortex.OldRetina",
      "new_path": "shard:SensoryCortex.Retina",
      "timestamp": "2026-06-27T08:30:00Z"
    }
  ]
}
```

### 10.1 Политика очистки истории миграций (Compaction Policy)
История миграций может автоматически сжиматься (pruning/compaction) после успешного применения путей к Store при загрузке проекта. Редактор может очищать примененные записи, оставляя только неразрешенные коллизии или самые свежие записи миграций.

### 10.2 Обработка устаревших путей (Карантин Stale Paths)
Если при загрузке `axicad.project.json` обнаружена запись размещения для пути, который отсутствует в загруженных файлах TOML и для которого нет подходящих записей в истории миграций:
- Данные метаданные **не конвертируются** в записи `DomainRecord` Store.
- Они помещаются в изолированную зону карантина Store редактора — `orphaned_metadata` / `stale_entries`.
- Пользователю выводится неблокирующее предупреждение (Warning) о наличии висящих метаданных с возможностью вручную перепривязать их к новому пути или безвозвратно удалить.

---

## 11. Поведение при загрузке и сохранении (Load/Save Behavior)

Импорт проекта в редактор выполняется в строгой последовательности:

```
1. Чтение и разбор model.toml (ошибки десериализации блокируют запуск)
       │
       ▼
2. Загрузка дочерних department.toml и shard.toml
       │
       ▼
3. Сборка реактивного Store
   - Генерация UUID сущностей в памяти
   - Расчет byTypedPath для всех сущностей
       │
       ▼
4. Чтение axicad.project.json (при отсутствии файла применяются дефолты)
       │
       ▼
5. Связывание (Latching)
   - placements и department_layout мапятся на UUID по typedPath
   - diagnostics_cache привязывается к UUID по typedPath
       │
       ▼
6. Анализ и фильтрация Stale Paths
   - Вынос висящих placements, department_layout и битых ссылок уровней в orphaned_metadata карантин
       │
       ▼
7. Инициализация макета
   - При отсутствии или невалидности layout_state применяются значения по умолчанию
```

- Сохранение файла проекта (`axicad.project.json`) и экспорт биологических файлов TOML являются **ортогональными операциями** и могут запускаться независимо.


---

## 12. Минимальный валидный JSON-пример

Пример файла `axicad.project.json`, описывающего проект с одним шардом Retina:

```json
{
  "project_id": "8f2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d",
  "schema_version": 1,
  "active_model": "model.toml",
  "axicad_version": "1.0.0",
  "created_at": "2026-06-27T08:00:00Z",
  "updated_at": "2026-06-27T09:12:00Z",
  "layout_state": {
    "activeWorkspaceMode": "Composition",
    "panelCollapsed": {
      "left": false,
      "right": false,
      "bottom": true
    },
    "panelSizes": {
      "left": 20,
      "right": 25,
      "bottom": 30
    },
    "bottomPanelMode": "diagnostics",
    "lastViewportCamera": {
      "position": [150.0, 300.0, 500.0],
      "target": [0.0, 0.0, 0.0],
      "fov": 60.0
    }
  },
  "viewport": {
    "render_mode": "solid",
    "visibility_filters": [],
    "grid_visible": true
  },
  "composition_levels": [
    {
      "id": "level_1",
      "name": "Sensory Level",
      "order": 1,
      "visible": true,
      "locked": false,
      "height": 120.0
    }
  ],
  "department_layout": [
    {
      "entityPath": "department:SensoryCortex",
      "assignedCompositionLevel": "level_1",
      "position": [0.0, 0.0, 10.0],
      "visible": true,
      "locked": false,
      "collapsed": false,
      "boundsMode": "computed"
    }
  ],
  "placements": [
    {
      "entityPath": "shard:SensoryCortex.Retina",
      "position": [10.0, 0.0, 50.0],
      "rotation": [0.0, 0.0, 0.0, 1.0],
      "visible": true,
      "locked": false,
      "colorOverride": "#ff5555"
    }
  ],
  "expanded_tree_nodes": [
    "department:SensoryCortex",
    "shard:SensoryCortex.Retina"
  ],
  "selection": [],
  "diagnostics_cache": [
    {
      "typedPath": "shard:SensoryCortex.Retina",
      "tomlFieldPath": "settings.ghost_capacity",
      "code": "AXI-SCHEMA-001",
      "severity": "warning",
      "message": "Превышение рекомендуемой емкости шарда (warning).",
      "generated_at": "2026-06-27T09:10:00Z"
    }
  ],
  "dirty_state": {
    "project_file_dirty": false,
    "toml_documents_dirty": [],
    "dirty_entities": [],
    "last_saved_at": "2026-06-27T09:12:00Z"
  },
  "migrations": {
    "history": []
  }
}
```

---

## 12.1 Ссылки на спецификации (References)

- [Оболочка и раскладка (workspace-shell-layout-spec-ru)](workspace-shell-layout-spec-ru.md)
- [Режим сборки Composition (composition-workspace-spec-ru)](composition-workspace-spec-ru.md)
- [Реактивное хранилище Store (editor-store-spec-ru)](editor-store-spec-ru.md)
- [Командная модель мутаций (command-mutation-spec-ru)](command-mutation-spec-ru.md)
- [Каталог диагностик (diagnostics-error-catalog-spec-ru)](diagnostics-error-catalog-spec-ru.md)

---

## 13. Исключения из целей спецификации (Non-goals)

- Параметры биологии, структуры VRAM и кривые роста (хранятся исключительно в TOML).
- Проводка внешних кабелей, схемы адресов и хэши пинов (вычисляются Baker-ом).
- Полная история действий Undo/Redo.

---

## 14. Changelog

| Дата | Изменение |
|------|-----------|
| 2026-06-28 | Зафиксирован гибридный формат проекта (`axicad.project.json` как легкий манифест + тяжелые бинарные артефакты под `.local-storage/artifacts/`), зафиксировано сохранение camera presets / saved views в метаданных проекта (`layout_state`). |
| 2026-06-27 | Создание спецификации файла проекта AxiCAD (Project File Spec). Описаны JSON-схема `axicad.project.json`, латчинг и карантин. Добавлены секции `layout_state` (активный режим, вьюпорт-камера, размеры панелей) и макет композиции (`composition_levels`, `department_layout` с флагами состояния и режимами AABB). Уточнено, что placements шардов содержат локальные координаты относительно департаментов, и определены правила dirty-флагов для смешанных биологических команд. |
