# AxiEngine — Спецификации (`INDEX.md`)

> Версия: 2.8 | Дата: 2026-06-29

---

## §1. Архитектурный граф

```mermaid
graph TD
    subgraph L0["Слой 0"]
        types["types (v2.1)"]
        physics["physics (v2.0)"]
    end
    subgraph L1["Слой 1"]
        layout["layout (v2.0)"]
        config["config (v2.0)"]
        wire["wire (v2.0)"]
    end
    subgraph L2["Слой 2"]
        ipc["ipc (v2.0)"]
        vfs["vfs (v2.0)"]
    end
    subgraph L3["Слой 3"]
        compute_api["compute-api (v2.0)"]
        compute["compute (v2.0)"]
        compute_cpu["compute-cpu (v2.0)"]
        compute_cuda["compute-cuda (v2.0)"]
        compute_hip["compute-hip (v2.0)"]
        test_harness["test-harness (v2.0)"]
    end
    subgraph L4["Слой 4"]
        topology["topology (v2.0)"]
        baker["baker (v2.0)"]
        baker_cli["baker-cli (v2.0)"]
    end

    types --> layout
    types --> config
    types --> wire
    types --> ipc
    types --> vfs
    types --> compute_api
    types --> compute_cpu
    types --> compute_cuda
    types --> compute_hip
    types --> test_harness
    types --> topology
    types --> baker
    physics --> config
    physics --> compute_cpu
    physics --> compute_cuda
    physics --> compute_hip
    physics --> baker
    layout --> ipc
    wire --> ipc
    layout --> compute_api
    layout --> test_harness
    layout --> topology
    layout --> baker
    config --> topology
    config --> baker
    vfs --> baker
    topology --> baker
    baker --> baker_cli
    compute_api --> compute
    compute_api --> compute_cpu
    compute_api --> compute_cuda
    compute_api --> compute_hip
    compute_api --> test_harness
    compute_cpu --> test_harness

    classDef active fill:#1e3a8a,stroke:#3b82f6,stroke-width:2px,color:#fff;
    class types,physics,layout,config,wire,ipc,vfs,compute_api,compute,compute_cpu,compute_cuda,compute_hip,test_harness,topology,baker,baker_cli active;
```

---

## §2. Реестр спецификаций

### Слой 0 (Layer 0: Primitives & Pure Math)
`no_std`, 0 аллокаций.

| Крейт | Спецификация | Статус | Назначение |
|---|---|---|---|
| `types` | [types_spec.md](spec_L0/types_spec.md) | **Draft v2.1** | Атомарные типы (`Tick`, `Voltage`), packed ABI (`PackedPosition`, `PackedTarget`, `SomaFlags`), seed/hash, константы. |
| `physics` | [physics_spec.md](spec_L0/physics_spec.md) | **Draft v2.0** | Математика GLIF, AHP, homeostasis, Active Tail, GSOP, DDS heartbeat, `v_seg`. |

### Слой 1 (Layer 1: Data Contracts & Deserialization)

| Крейт | Спецификация | Статус | Назначение |
|---|---|---|---|
| `layout` | [layout_spec.md](spec_L1/layout_spec.md) | **Draft v2.0** | C-ABI макеты физической памяти (`VariantParameters`), выравнивание плоскостей SoA и заголовки файлов. |
| `config` | [config_spec.md](spec_L1/config_spec.md) | **Draft v2.0** | Serde/TOML DTO, парсинг и "Shift-Left" валидация DSL (`model.toml`, `department.toml`, `shard.toml`). |
| `wire` | [wire_spec.md](spec_L1/wire_spec.md) | **Draft v2.0** | C-ABI структуры сетевых и IPC пакетов, magic-константы, выравнивание, Little-Endian политика и `no-alloc` хелперы. |

### Слой 2 (Layer 2: Infrastructure & OS Isolation)

| Крейт | Спецификация | Статус | Назначение |
|---|---|---|---|
| `ipc` | [ipc_spec.md](spec_L2/ipc_spec.md) | **Draft v2.0** | Жизненный цикл SHM/mmap, атомарные переходы Ночной Фазы (CAS), двойной буфер Swapchain и изоляция OS системных вызовов. |
| `vfs` | [vfs_spec.md](spec_L2/vfs_spec.md) | **Draft v2.0** | Контейнерный формат `.axic`, оглавление TOC, Read-Only mmap отображение, нормализация путей и примитивы экстракции. |

### Слой 3 (Layer 3: Hardware Acceleration & Compute Abstraction)

| Крейт | Спецификация | Статус | Назначение |
|---|---|---|---|
| `compute-api` | [compute_api_spec.md](spec_L3/compute_api_spec.md) | **Draft v2.0** | Аппаратно-независимый HAL контракт бэкендов вычислений (`ComputeBackend`), непрозрачные VRAM handles и DTO команд. |
| `compute` | [compute_spec.md](spec_L3/compute_spec.md) | **Draft v2.0** | Фасад вычислений `ShardEngine`, автовыбор бэкендов (`BackendPreference`) и оркестрация жизненного цикла шарда. |
| `compute-cpu` | [compute_cpu_spec.md](spec_L3/compute_cpu_spec.md) | **Draft v2.0** | Многопоточная CPU-реализация `ComputeBackend` на базе Rayon, выровненные ресурсы хоста и проверочная реализация. |
| `compute-cuda` | [compute_cuda_spec.md](spec_L3/compute_cuda_spec.md) | **Draft v2.0** | Высокопроизводительная CUDA-реализация `ComputeBackend` на базе NVIDIA Runtime API и неблокирующих стримов. |
| `compute-hip` | [compute_hip_spec.md](spec_L3/compute_hip_spec.md) | **Draft v2.0** | Высокопроизводительная AMD ROCm/HIP реализация `ComputeBackend` на базе Wave64 вейвфронтов и неблокирующих стримов. |
| `test-harness` | [test_harness_spec.md](spec_L3/test_harness_spec.md) | **Draft v2.0** | Вспомогательный тестовый крейт для дифференциальных проверок `ComputeBackend`, фикстур и контроля ABI-зеркал. |

### Слой 4 (Layer 4: Geometry, Growth & Connectome Generation)

| Крейт | Спецификация | Статус | Назначение |
|---|---|---|---|
| `topology` | [topology_spec.md](spec_L4/topology_spec.md) | **Draft v2.0** | Чистый алгоритмический крейт пространственной геометрии, детерминированного размещения сом, пространственной сетки и роста аксонов. |
| `baker` | [baker_spec.md](spec_L4/baker_spec.md) | **Draft v2.0** | Оркестратор компиляции AOT, координация фаз сборки, генерация бинарных блобов по `layout` и упаковка `.axic`. |
| `baker-cli` | [baker_cli_spec.md](spec_L4/baker_cli_spec.md) | **Draft v2.0** | Консольная утилита и sidecar-интерфейс для запуска `baker`, вывода отчетов/прогресса и управления флагами. |

---

## §3. Реестры

- **Инварианты и ошибки**: [troubleshooting.md](troubleshooting.md)
- **Вопросы и замечания к ревью**: [review.md](review.md)
