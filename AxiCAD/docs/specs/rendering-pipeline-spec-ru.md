# Спецификация визуального слоя рендеринга AxiCAD (Rendering Pipeline Spec)

> Этот документ формально описывает архитектуру, интерфейсы и политики визуализации (Rendering Pipeline) в 3D-редакторе AxiCAD. Rendering Pipeline преобразует снимки Store, вычисленную геометрию, состояние выделения, превью инструментов и диагностические маркеры в кадр 3D-вьюпорта сцены. 

## Status: Draft

---

## 1. Назначение и границы (Scope & Non-goals)

`Rendering Pipeline` отвечает за преобразование абстрактного состояния приложения в графический вывод в окне просмотра.

### Вне зоны ответственности (Non-goals)
- **Не мутирует Store**: Визуальный слой только читает данные. Рендерер не может напрямую вызывать действия изменения Store или команды.
- **Не пишет TOML/JSON файлы**: Рендерер не занимается сохранением или экспортом проектов.
- **Не является Selection Engine**: Рендерер не определяет, какой объект находится под курсором мыши, и не вычисляет Hit Stack. Он лишь отрисовывает контуры наведения (`hover`) и выделения (`selection`), полученные от [Selection Engine](selection-engine-spec-ru.md).
- **Не выполняет политику выбора (Ray picking policy)**: Рендерер не решает, какие объекты отсекать по нормалям или приоритетам при клике.
- **Не является Tool System**: Рендерер не управляет поведением гизмо, не обрабатывает drag-and-drop жесты и не строит трассы. Он лишь визуализирует дескрипторы гизмо и превью, предоставленные [Tool System](tool-system-spec-ru.md).
- **Не принимает решений по ограничениям**: Валидацией геометрии и пересечений занимается [Constraint Engine](constraint-engine-spec-ru.md). Рендерер лишь визуализирует переданные маркеры ошибок.
- **Не является Geometry/Spatial Service**: Рендерер не рассчитывает воксельные сетки, трассы и OBB. Он использует результаты вычислений от [Geometry & Spatial Service](geometry-spatial-service-spec-ru.md).
- **Не владеет канонической геометрией модели**: Канонические данные хранятся в Store. Рендерер владеет только локальным кэшем ресурсов GPU (буферы вершин, текстуры, материалы) и транзитным кэшем отображения.

---

## 2. Контракт независимости от бэкенда (Backend-Agnostic Contract)

Архитектура Rendering Pipeline спроектирована как **независимая от конкретного графического API** (backend-agnostic). 

- **Описание сцены**: Все сущности сцены, слои, материалы и состояния камеры описываются декларативными структурами данных (дескрипторами рендеринга).
- **Кандидат MVP**: Для веб-версии MVP в качестве одного из возможных бэкендов реализации выступает библиотека **Three.js** поверх WebGL.
- **Перспектива**: Декларативный контракт позволяет заменить Three.js на нативный графический бэкенд на базе **WGPU / WebAssembly / Rust** без изменения логики Store, инструментов и математических сервисов.
- **Изоляция API**: Код AxiCAD не содержит прямых вызовов привязанных к бэкенду классов вне специализированного адаптера бэкенда (`RenderBackendAdapter`).

---

## 3. Место в архитектуре (Architecture Placement)

Rendering Pipeline принимает данные из нескольких источников и передает их в адаптер отрисовки:

```
┌──────────────────────────────────────┐
│             Editor Store             │
└───────────────────┬──────────────────┘
                    │ (Store Snapshot / Derived Selectors)
                    ▼
┌──────────────────────────────────────┐
│          Render Scene Model          │◄── Selection Engine
│                                      │    (SelectionState)
│                                      │◄── Tool System
│                                      │    (ToolPreview / GizmoHandles)
│                                      │◄── Diagnostics Layer
│                                      │    (Diagnostic descriptors)
│                                      │◄── Project File
│                                      │    (Camera layout state)
│                                      │◄── Geometry/Spatial Service
│                                      │    (Transforms / Bounds / OBB / AABB)
└───────────────────┬──────────────────┘
                    │ (Invalidation / Layer Update)
                    ▼
┌──────────────────────────────────────┐
│         RenderBackendAdapter         │
└───────────────────┬──────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│               GPU Frame              │
└──────────────────────────────────────┘
```

### Потоки данных:
1. **Store -> Render Scene Model**: Построение декларативного дерева объектов сцены на основе биологического состояния.
2. **Selection Engine -> Render Scene Model**: Передача информации об активном выделении, наведении и фокусе для отрисовки эффектов постобработки (outlines, glow).
3. **Tool System -> Render Scene Model**: Передача временных превью (`ToolPreview[]`) и дескрипторов гизмо (`GizmoHandle[]`).
4. **Diagnostics Layer -> Render Scene Model**: Передача маркеров предупреждений и ошибок сцены.
5. **Project File -> Render Scene Model**: Передача сохраненных настроек камеры (`ViewportCameraState`) из секции `layout_state`.
6. **Geometry & Spatial Service -> Render Scene Model**: Предоставление геометрических дескрипторов, матриц трансформации, границ (AABB/OBB) и вычисленных путей трактов для сборки модели сцены.
7. **RenderBackendAdapter -> GPU Frame**: Трансляция абстрактных дескрипторов `RenderSceneModel` в графические ресурсы видеокарты (с учетом минимизации обновлений) и отрисовка кадра.

---

## 4. Основные сущности и контракты (Core Entities)

```typescript
// Концептуальный интерфейс (Conceptual interface, не является прямым API реализации)

type UUID = string;
type TypedEntityPath = string;

export type RenderObjectId = string; // Стабильный ID объекта сцены, совпадающий с entityPath или ID гизмо

export type RenderObjectKind =
  | 'level'
  | 'department'
  | 'shard'
  | 'shard-face'
  | 'socket'
  | 'socket-sample'
  | 'tract'
  | 'gizmo-handle'
  | 'diagnostic-marker'
  | 'runtime-spike'
  | 'runtime-path'
  | 'background-grid';

export type RenderMaterialPolicy =
  | 'canonical'             // Стандартный биологический объект
  | 'transparentContainer'  // Полупрозрачная внешняя оболочка
  | 'activeSelection'       // Выделенный элемент
  | 'hoverOutline'          // Контур под курсором
  | 'focusAccent'           // Акцент инспектора
  | 'disabledTint'          // Заблокированное состояние
  | 'validPreview'          // Валидный предпросмотр
  | 'invalidPreview'        // Невалидный предпросмотр коллизий
  | 'warningPreview'        // Предупреждающий предпросмотр
  | 'errorOverlay'          // Маркер ошибки
  | 'warningOverlay'        // Маркер предупреждения
  | 'runtimeOverlay';       // Симуляционные частицы/спайки

export interface RenderTransform {
  readonly position: [number, number, number]; // Физические координаты µm
  readonly rotationQuaternion: [number, number, number, number];
  readonly scale: [number, number, number];
}

export interface RenderVisibilityState {
  readonly visible: boolean;
  readonly opacity: number; // 0.0 .. 1.0
  readonly castShadow: boolean;
  readonly receiveShadow: boolean;
}

/** Декларативное описание 3D-объекта на сцене */
export interface RenderObject {
  readonly id: RenderObjectId;
  readonly kind: RenderObjectKind;
  readonly entityPath?: TypedEntityPath; // Обратная ссылка на исходную сущность для подсветки и метаданных; выборка (picking) остается ответственностью Selection/Geometry сервисов
  readonly transform: RenderTransform;
  readonly visibility: RenderVisibilityState;
  readonly materialPolicy: RenderMaterialPolicy;
  readonly geometryDescriptor: {
    readonly type: 'box' | 'cylinder' | 'plane' | 'mesh' | 'line-strip';
    readonly parameters: any; // Специфичные размеры в µm
  };
}

/** Состояние подсветки сущности */
export interface HighlightState {
  readonly hovered: boolean;
  readonly selected: boolean;
  readonly active: boolean;
  readonly focused: boolean;
  readonly disabled: boolean;
}

/** Декларативное описание слоя рендеринга */
export interface RenderLayer {
  readonly layerId: string;
  readonly visible: boolean;
  readonly renderOrder: number;
  readonly objects: RenderObject[];
}

/** Обобщенный дескриптор оверлея */
export interface RenderOverlay {
  readonly overlayId: string;
  readonly kind: 'preview' | 'gizmo' | 'diagnostic' | 'runtime' | 'hud';
  readonly visible: boolean;
}

/** Декларативная модель графической сцены */
export interface RenderSceneModel {
  readonly layers: Map<string, RenderLayer>;
  readonly cameraState: ViewportCameraState;
  readonly highlights: Map<RenderObjectId, HighlightState>;
  readonly activeWorkspaceMode: string;
}

/** Описание визуальной ручки гизмо */
export interface GizmoRenderDescriptor {
  readonly handleId: string;
  readonly kind: 'axis' | 'plane' | 'corner' | 'vertex';
  readonly transform: RenderTransform;
  readonly axisDirection?: [number, number, number];
  readonly highlightRole?: 'hoverOutline' | 'activeSelection' | 'default';
  readonly sizeMultiplier: number;
}

/** Описание маркера ошибки на сцене */
export interface DiagnosticOverlayDescriptor {
  readonly markerId: string;
  readonly targetPath: TypedEntityPath;
  readonly severity: 'error' | 'warning' | 'info';
  readonly geometryType: 'volume' | 'outline' | 'billboard';
  readonly boundsUm: { min: [number, number, number]; max: [number, number, number] };
}

/** Описание эфемерных оверлеев симуляции */
export interface RuntimeOverlayDescriptor {
  readonly overlayId: string;
  readonly type: 'spike-burst' | 'signal-path';
  readonly pointsWorld: [number, number, number][];
  readonly intensity: number; // 0.0 .. 1.0 (для затухания)
}

/** Полное состояние камеры */
export interface ViewportCameraState {
  readonly position: [number, number, number];
  readonly target: [number, number, number];
  readonly fov: number;
  readonly aspect: number;
  readonly projection: 'perspective' | 'orthographic';
  readonly zoom: number;
}

/** Временные настройки кадра */
export interface RenderFrameContext {
  /** Время выполнения кадра. Используется исключительно в контексте выполнения кадра для динамических эффектов и затухания; не влияет на формирование стабильных дескрипторов рендеринга. */
  readonly timestamp: number;
  readonly cameraState: ViewportCameraState;
  readonly activeWorkspaceMode: string;
  readonly enableShadows: boolean;
  readonly renderingQuality: 'low' | 'medium' | 'high';
}

/** Инструкция инвалидации для частичного обновления сцены */
export interface RenderInvalidation {
  readonly layersToRebuild: string[]; // Список измененных слоев
  readonly objectsToUpdate: RenderObjectId[]; // Список объектов с изменившимися трансформациями
  readonly forceFullClear: boolean;
}

/** Концептуальный дескриптор окна/поверхности отрисовки */
export interface RenderSurfaceHandle {
  readonly surfaceId: string;
  readonly viewportDimensions: { width: number; height: number };
}

/** Адаптер бэкенда рендеринга */
export interface RenderBackendAdapter {
  init(surface: RenderSurfaceHandle): void;
  updateScene(sceneModel: RenderSceneModel, invalidation: RenderInvalidation): void;
  updateCamera(cameraState: ViewportCameraState): void;
  updateGizmos(gizmos: GizmoRenderDescriptor[]): void;
  updateDiagnostics(diagnostics: DiagnosticOverlayDescriptor[]): void;
  updateRuntimeOverlays(overlays: RuntimeOverlayDescriptor[]): void;
  renderFrame(context: RenderFrameContext): void;
  dispose(): void;
}
```

---

## 5. Слои рендеринга (Render Layers)

Для структурирования сцены, оптимизации отрисовки прозрачных объектов и быстрого переключения видимости вся сцена делится на логические слои (`Render Layers`).

### Список слоев (в порядке отрисовки от задних к передним):
1. **`baseGridLayer`**: Фоновая воксельная сетка сцены с шагом привязки. Рисуется первой.
2. **`compositionLayer`**: Слой визуализации глобальных уровней сборки (Z-полосы высотных ограничений).
3. **`departmentContainerLayer`**: Оболочки департаментов. Рисуются как полупрозрачные боксы. Требуют сортировки прозрачности.
4. **`shardLayer`**: Сплошные трехмерные боксы биологических шардов. Основной объем геометрии.
5. **`socketLayer`**: Тонкие двухсторонние плоскости сокетов на гранях шардов.
6. **`tractLayer`**: Линии и трехмерные трубы кабельных трактов.
7. **`selectionHighlightLayer`**: Слой постобработки. Отрисовывает светящиеся контуры выделения (активные объекты и hover).
8. **`toolPreviewLayer`**: Временная геометрия инструментов (например, контур перетаскивания шарда, линия прокладываемого тракта).
9. **`generatedPreviewLayer`**: Оверлей сгенерированных алгоритмических предпросмотров (`generated algorithmic previews`: решений автороутера, алгоритмов компоновки, предпросмотра роста связей или результатов Baker-а) в виде полупрозрачных призрачных контуров.
10. **`gizmoLayer`**: Стрелки перемещения, плоскости изменения размеров и маркеры узлов (Handles).
11. **`diagnosticOverlayLayer`**: Трехмерные маркеры ошибок и предупреждений (коллизионные объемы роли `errorOverlay`, очертания предупреждений роли `warningOverlay`).
12. **`runtimeLayer`**: Спайковая активность, вспышки нейронов, пути прохождения сигналов (эфемерные частицы).
13. **`hudLayer`**: Двухмерный текстовый оверлей (метки сокетов, линейки размеров в µm), рисуемый поверх 3D.

> [!NOTE]
> В зависимости от активного `Workspace Mode`, слои могут полностью отключаться адаптером бэкенда для экономии производительности и чистоты интерфейса. Например, в режиме `Composition` слои `socketLayer`, `tractLayer` и `runtimeLayer` деактивируются.

---

## 6. Отображение сущностей (Entity Rendering)

Каждая логическая сущность из Store преобразуется в графический примитив по следующим правилам:

- **Composition Levels / Z-bands**: Визуализируются как тонкие полупрозрачные плоскости-сечения на границах высот уровней с нанесенной разметкой высот.
- **Departments (Департаменты)**: Отображаются как полупрозрачные ориентированные боксы (OBB) с тонкими светящимися ребрами. Внутренние шарды должны быть четко видны сквозь департамент.
- **Shards (Шарды)**: Рендерятся как сплошные непрозрачные параллелепипеды. Углы шарда могут иметь фаски. На ребра может наноситься сетка воксельного разрешения.
- **Shard Faces (Грани шардов)**: Подсвечиваются только в режиме редактирования сокетов при наведении луча.
- **Sockets (Сокеты)**: Визуализируются как тонкие двусторонние пластины, лежащие строго на плоскости грани шарда (смещение от грани на $10^{-3}$ µm для предотвращения Z-fighting).
- **Socket Samples / Pins (Порты)**: Для предотвращения просадки FPS при отображении сотен мелких пинов на сокете, они рендерятся как единая текстурированная плоскость сокета на больших расстояниях, а на близком расстоянии подгружаются как мелкие геометрические маркеры с использованием **GPU Instancing**.
- **Tracts (Тракты)**: Визуализируются как трехмерные цилиндрические сегменты (трубы) или прямоугольные короба. Точки изгиба трактов рендерятся в виде сферических сочленений (handles), доступных для выбора.
- **Generated Preview**: Отображаются в виде полупрозрачных призрачных силуэтов (`ghosts`) с прерывистыми очертаниями (dashed outlines) для репрезентации результатов алгоритмических генераторов.
- **Diagnostic Markers**: Визуализируются с использованием материалов ролей `errorOverlay` и `warningOverlay` (объемные полупрозрачные сферы коллизий или предупреждающие пиктограммы-биллборды, всегда повернутые к камере).
- **Runtime Overlays**: Отображаются с использованием систем частиц (Particle Systems) с эффектом постепенного затухания яркости со временем.

---

## 7. Подсветка выделения (Selection & Highlight)

Rendering Pipeline не решает, какие объекты выделены, а лишь пассивно отображает переданный `SelectionState`.

### Визуальные каналы отображения состояний выделения:
1. **Hover (Наведение)**:
   - Тонкий светящийся контур (роль `hoverOutline`) вокруг объекта.
   - Рендерится через шейдер постобработки в буфере кадра (stencil/depth outline).
2. **Active Selection (Активное выделение)**:
   - Яркий контур (роль `activeSelection`) вокруг всех объектов, находящихся в `SelectionStack.targets`.
   - Активный объект (по индексу `SelectionStack.activeIndex`) получает более интенсивное свечение или дополнительный маркер фокуса (роль `focusAccent`).
3. **Focus (Целевой объект инспектора)**:
   - Векторный маркер или рамка подсветки углов объекта (роль `focusAccent`).
4. **Disabled / Locked Tint (Заблокированные объекты)**:
   - Объекты с флагом `locked` рендерятся с наложением полупрозрачной маски (роль `disabledTint`) или со штриховкой материала, а также отображают пиктограмму замочка при наведении.
5. **Multi-selection markers**:
   - При групповом выделении рендерится общий полупрозрачный контейнер (bounding box), охватывающий все выбранные элементы.

> [!TIP]
> Для предотвращения мерцания (hover flicker) при быстром перемещении мыши на стыках объектов, рендерер использует состояние наведения, стабилизированное зоной гистерезиса в Selection Engine.

---

## 8. Отрисовка гизмо и превью (Tool Preview & Gizmos)

Манипуляторы ввода и геометрия предпросмотра поставляются из Tool System и рендерятся в слоях с наивысшим приоритетом глубины (всегда поверх основной сцены).

- **Gizmo Handles**:
  - Стрелки осей X/Y/Z (Move), угловые маркеры (Resize) и кольца вращения (Rotate).
  - Рендерятся без учета глубины сцены (depth test = false), чтобы гизмо никогда не перекрывалось шардами.
  - Каждая ручка гизмо имеет стабильный ID (например, `gizmo-move-x`), который напрямую мапится на ID объекта рендеринга. При попадании луча в этот ID, Selection Engine возвращает соответствующий хит.
- **Tool Preview**:
  - Рендерится как эфемерный оверлей (ghost mesh).
  - Визуальные стили (семантические роли материалов):
    - Роль `validPreview`: корректное транзитное положение;
    - Роль `invalidPreview`: коллизия или нарушение критического ограничения;
    - Роль `warningPreview`: предупреждение.

---

## 9. Оверлеи диагностики (Diagnostics Overlays)

Диагностические маркеры поставляются в рендерер из diagnostics layer и служат для пространственной отладки:

- **Error Volumes**: Полупрозрачные объемы (роль `errorOverlay`), заполняющие области пересечения (коллизий) шардов или департаментов.
- **Warning Outlines**: Пунктирные контуры (роль `warningOverlay`) вокруг объектов с некритическими нарушениями (например, шард висит в воздухе без привязки к уровню).
- **Orphaned Metadata**: Рендеринг оборванных связей трактов (концы кабелей, не подключенные к сокетам) в виде мигающих коннекторов.
- **Фильтрация видимости**: Диагностические оверлеи должны легко отключаться пользователем с помощью переключателей в панели Workspace Shell Layout. Отключение оверлеев не влияет на работу Constraint Engine.

---

## 10. Камера и состояние вьюпорта (Camera & Viewport)

Состояние камеры полностью отделено от биологической модели:

1. **Владение и персистентность состояния**:
   - Рендерер владеет исключительно низкоуровневыми объектами и матрицами камеры графического бэкенда.
   - Источником состояния камеры является контроллер вьюпорта (Viewport / Camera Controller). При сохранении сессии параметры камеры записываются в `axicad.project.json` в секцию `layout_state.camera`.
   - Сам рендерер не модифицирует файл проекта JSON. Сохранение параметров камеры помечает `project_json_dirty` во внешнем потоке управления проектом.
2. **Интерактивное движение**:
   - Во время вращения, панорамирования или масштабирования камеры изменения считаются эфемерным UI-состоянием и не инициируют запись на диск на каждый кадр.
3. **Режимы вьюпорта**:
   - **Перспективный (Perspective)**: Основной режим для пространственной компоновки.
   - **Ортографический (Orthographic)**: Режим чертежа (вид сверху/сбоку/спереди) для точного позиционирования сокетов.
   - **Focus / Fit Selection**: Функция быстрого центрирования камеры на выделенном объекте с автоматическим подбором дистанции фокусировки по габаритам AABB/OBB.

---

## 11. Инвалидация и жизненный цикл кадра (Invalidation & Frame Lifecycle)

Для обеспечения производительности 3D-редактора (целевые 60 FPS) запрещено перестраивать всю бэкенд-сцену на каждый кадр. Рендерер работает по схеме реактивной инвалидации слоев.

```
       Изменение Store / Ввод пользователя
                             │
                             ▼
       [ Derived State Invalidation Check ]
                             │
           ┌─────────────────┴──────────────┐
           │ Layer Invalidation?            │ Object Transform Only?
           ▼                                ▼
┌────────────────────────┐      ┌────────────────────────┐
│ Rebuild Layers         │      │ Update Object Matrices │
│ (Regenerate Geometry)  │      │ (Direct Transform Update)
└───────────┬────────────┘      └───────────┬────────────┘
            │                               │
            └───────────────┬───────────────┘
                            ▼
               [ Backend Adapter Sync GPU ]
                            │
                            ▼
                     [ Render Frame ]
```

### Жизненный цикл отрисовки:
1. **Обнаружение изменений**: При изменении Store или получении превью от Tool System вычисляется масштаб изменений:
   - Изменились только координаты перемещаемого объекта -> инвалидируется только его матрица трансформации (`objectsToUpdate`). Геометрия не перестраивается.
   - Изменился hover/focus -> обновляются параметры шейдера постобработки в `selectionHighlightLayer`. Очень дешевая операция.
   - Добавлен/удален шард -> инвалидируется весь `shardLayer` (`layersToRebuild`). Выполняется пересборка дескрипторов слоя.
2. **Синхронизация ресурсов GPU**: Адаптер бэкенда (`RenderBackendAdapter`) сверяет дескрипторы с созданными объектами GPU. Удаленные объекты уничтожаются (dispose геометрии и материалов для предотвращения утечек памяти), новые создаются, измененные обновляются.
3. **Отрисовка кадра (Render Frame)**: Выполняется один проход рендеринга сцены.

---

## 12. Требования к производительности (Performance Constraints)

Поскольку проекты AxiCAD могут содержать тысячи сокетов и десятки тысяч отдельных портов (socket samples), Rendering Pipeline накладывает ограничения на методы отрисовки:

- **Instanced Rendering (GPU Instancing)**: Все повторяющиеся мелкие элементы (порты сокетов, сомы нейронов внутри шарда, сегменты трактов) должны рендериться через инстансинг. Запрещено создавать отдельные бэкенд-меши для каждого пина.
- **LOD (Level of Detail)**:
  - На больших дистанциях сокет рендерится как простая плоская текстура. На средних дистанциях появляется сетка сэмплинга. На близких дистанциях активируется полная геометрия пинов.
  - Runtime-оверлеи спайков отключаются при отдалении камеры за пределы видимости шарда.
- **Spatial Culling**:
  - **Frustum Culling**: Объекты за пределами пирамиды видимости камеры автоматически исключаются из прохода рендеринга видеокарты.
  - **Occlusion Culling (Перспектива)**: Скрытие внутренних вокселей и сом нейронов, полностью перекрытых внешними гранями непрозрачного шарда.
- **Без обхода Store на каждый кадр**: Дескрипторы сцены должны строиться на основе мемоизированных селекторов. Рендерер не должен итерироваться по всему дереву Store в цикле анимации кадра (animation frame loop).

---

## 13. Детерминированность и визуальная стабильность (Determinism)

1. **Snapshot-детерминизм**:
   - Одинаковое состояние Store, параметры камеры и геометрические дескрипторы на входе гарантируют одинаковые дескрипторы рендеринга и стабильный порядок визуализации. Пиксельное тождество (pixel-level identity) не гарантируется между разными бэкендами, видеокартами и версиями драйверов.
2. **Стабильные ID объектов**:
   - `RenderObjectId` должны быть детерминированными и основываться на `entityPath` объекта. Нельзя использовать случайные числа (`Math.random()`) для ID объектов рендеринга.
3. **Стабильный порядок прозрачности (Transparency Sorting)**:
   - Для предотвращения визуальных артефактов (когда полупрозрачные оболочки департаментов перекрывают друг друга в неверном порядке при вращении камеры), рендерер должен выполнять сортировку прозрачных объектов по глубине перед отрисовкой кадра.

---

## 14. Материалы и визуальные роли (Materials & Visual Policy)

Rendering Pipeline не фиксирует жестко цвета и текстуры. Он оперирует абстрактными визуальными ролями материалов (semantic material roles):

- **`canonical`**: Материал по умолчанию для шардов и сокетов. Визуальные свойства определяются типом шарда или биологическим департаментом.
- **`transparentContainer`**: Материал оболочек департаментов и высотных Z-bands. Высокая прозрачность ($0.15 .. 0.3$), яркие ребра, отключенная запись в буфер глубины (depth write = false).
- **`activeSelection`**: Материал выделения с наложением эффекта свечения (emissive glow) и пульсацией.
- **`hoverOutline`**: Материал наведения с эффектом подсветки ребер.
- **`focusAccent`**: Материал фокуса инспектора.
- **`disabledTint`**: Материал заблокированного состояния со штриховкой или затемнением.
- **`invalidPreview`**: Материал некорректного превью со штриховкой коллизий.
- **`warningPreview`**: Предупреждающий материал предпросмотра.
- **`validPreview`**: Полупрозрачный материал корректного предпросмотра "призрака" ($opacity = 0.5$) с эффектом наложения сетки (wireframe).
- **`errorOverlay` / `warningOverlay`**: Диагностические материалы для индикации проблем на сцене.
- **`runtimeOverlay`**: Эфемерный материал систем частиц симуляции.

Финальные цветовые палитры, темы оформления (темная/светлая) и контрастность поступают в рендерер из темы интерфейса и дизайн-системы (UI Theme / Design System).

---

## 15. Границы источника истины (Source-of-Truth Boundaries)

Для сохранения чистоты архитектуры рендерер строго ограничен в своих полномочиях:

- **Рендерер не владеет TOML**: Он не знает структуры файлов конфигурации и правил Baker-а. Он получает вычисленные дескрипторы рендеринга и геометрии (`derived geometry descriptors / render descriptors`).
- **Рендерер не знает правил валидации**: Он не вычисляет пересечения. Если объект некорректен, рендерер получает готовый дескриптор `DiagnosticOverlayDescriptor`.
- **Рендерер не владеет камерой**: Рендерер владеет только низкоуровневыми ресурсами камеры бэкенда (матрицами вида/проекции GPU). Камера управляющего слоя находится в контроллере вьюпорта, а её сохранение помечает `project_json_dirty` во внешнем потоке управления проектом.

---

## 16. Профили рендеринга по рабочим пространствам (Workspace Profiles)

В зависимости от Workspace Mode, Rendering Pipeline активирует специализированные профили рендеринга:

### 16.1 Composition Profile
- **Слои**: Включены `baseGridLayer`, `compositionLayer`, `departmentContainerLayer`, `shardLayer`, `selectionHighlightLayer`, `gizmoLayer`, `diagnosticOverlayLayer`.
- **Оптимизация**: Сокеты и тракты полностью скрыты. Шарды рендерятся с максимальным LOD.
- **Оверлеи**: Рендеринг границ Z-bands.

### 16.2 Connectome Profile
- **Слои**: Включены `baseGridLayer`, `shardLayer` (с высокой прозрачностью), `socketLayer`, `tractLayer`, `selectionHighlightLayer`, `toolPreviewLayer`, `gizmoLayer`.
- **Оптимизация**: Включается GPU Instancing для портов сокетов. Шарды рендерятся полупрозрачными ("стеклянными") для обеспечения видимости прохождения кабелей сквозь них.
- **Оверлеи**: Подсветка доступных для подключения сокетов.

### 16.3 Growth & Inference Profile (Перспектива)
- **Слои**: Включены `shardLayer` (силуэты), `socketLayer`, `tractLayer`, `runtimeLayer`, `hudLayer`.
- **Оптимизация**: Активируется динамическое слияние частиц спайков.
- **Оверлеи**: Рендеринг векторов напряженности полей, графики потенциалов сом в HUD.

---

## 17. Открытые вопросы (Open Decisions)

В ходе проектирования Rendering Pipeline выделены следующие нерешенные вопросы, требующие обсуждения с Человеком:

1. **Реализовывать ли MVP строго на Three.js или сразу закладывать Rust/WGPU?**
   * *Контекст*: Three.js позволяет быстро запустить 3D в браузере с готовыми материалами и камерами. Rust/WGPU дает огромный буст производительности для отрисовки миллионов спайков и сокетов, но требует написания рендерера с нуля.
2. **Как рендерить перекрывающиеся прозрачные контейнеры (transparency order)?**
   * *Контекст*: Сортировка по глубине (depth sorting) в графических рендерерах спасает только для непересекающихся объектов. Если один департамент частично входит в другой, на границах возникнут визуальные швы. Нужно ли закладывать Order-Independent Transparency (OIT) на уровне шейдеров?
3. **Где хранить настройки пресетов камер (Camera Presets)?**
   * *Контекст*: Кнопки быстрого переключения камеры (например, "Вид сверху", "Фокус на Sensory"). Должны ли эти пресеты быть зашиты в код редактора или настраиваться в JSON проекта?
4. **Нужен ли буфер выбора на стороне GPU (Offscreen Picking Buffer)?**
   * *Контекст*: Для быстрого определения объекта под курсором (picking) можно рендерить сцену в невидимую текстуру, где каждый объект окрашен в свой уникальный цвет (ID). Однако даже при добавлении аппаратного Offscreen Picking Buffer в качестве графической оптимизации, Selection Engine полностью сохраняет за собой владение политиками выборки, фильтрацией и итоговым резолвингом целей.
5. **Где проходит граница между 3D HUD (метки в пространстве) и плоским UI?**
   * *Контекст*: Текстовые подписи к сокетам (например, "Socket A1") можно рендерить как 3D-текст или позиционировать двухмерные UI-элементы поверх окна просмотра. Плоские элементы проще стилизовать, но они могут отставать от камеры при быстром вращении.

---

## Changelog

| Дата | Версия | Описание изменений |
| :--- | :--- | :--- |
| 2026-06-27 | v0.3.0 | Синхронизирован enum RenderMaterialPolicy с семантическими ролями (activeSelection, hoverOutline и др.), удалены остатки цветовых описаний в слое диагностик, добавлен комментарий к timestamp в RenderFrameContext (frame execution context only), обновлена формулировка источника истины (получение derived geometry descriptors вместо триангулированной геометрии). |
| 2026-06-27 | v0.2.0 | Очищены backend-specific формулировки (внедрен RenderSurfaceHandle, нейтральный frame loop), исправлен поток Geometry/Spatial Service в RenderSceneModel, уточнены границы владения камерой (персистентность outside renderer), ослаблено требование пиксельного детерминизма в пользу одинаковых дескрипторов, переведены стили материалов на семантические роли (hoverOutline, activeSelection и др.), обобщен generatedPreviewLayer для алгоритмических генераторов, закреплена приоритетность Selection Engine над GPU picking buffer, добавлены типы RenderLayer, RenderSceneModel, HighlightState, RenderOverlay. |
| 2026-06-27 | v0.1.0 | Создан первый драфт спецификации Rendering Pipeline для AxiCAD. Описаны слои, сущности, интеграция, инвалидация слоев, производительность и открытые вопросы. |

---

## Ссылки на связанные документы
- [Спецификация геометрического и пространственного сервиса](geometry-spatial-service-spec-ru.md)
- [Спецификация ядра проверки ограничений (Constraint Engine)](constraint-engine-spec-ru.md)
- [Спецификация ядра выделения объектов (Selection Engine)](selection-engine-spec-ru.md)
- [Спецификация интерактивных инструментов (Tool System)](tool-system-spec-ru.md)
- [Спецификация реактивного хранилища и модели состояния редактора](editor-store-spec-ru.md)
- [Спецификация командной модели изменения состояния](command-mutation-spec-ru.md)
- [Спецификация предметного режима сборки Composition Workspace](composition-workspace-spec-ru.md)
- [Спецификация геометрической модели сокетов и трактов](socket-tract-geometry-spec-ru.md)
- [Каталог диагностик и спецификация ошибок AxiCAD](diagnostics-error-catalog-spec-ru.md)
- [Спецификация архитектуры интерфейса и зон раскладки](workspace-shell-layout-spec-ru.md)
