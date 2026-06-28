# AxiEngine — Спецификации (`INDEX.md`)

> Версия: 2.1 | Дата: 2026-06-29

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
    end

    types --> layout
    types --> config
    types --> wire
    types --> ipc
    types --> vfs
    types --> compute_api
    physics --> config
    layout --> ipc
    wire --> ipc
    layout --> compute_api
    compute_api --> compute

    classDef active fill:#1e3a8a,stroke:#3b82f6,stroke-width:2px,color:#fff;
    class types,physics,layout,config,wire,ipc,vfs,compute_api,compute active;
```

---

## §2. Реестр спецификаций

### Слой 0 (Layer 0: Primitives & Pure Math)
`no_std`, 0 аллокаций.

| Крейт | Спецификация | Статус | Назначение |
|-------|--------------|--------|------------|
| `types` | [types_spec.md](spec_L0/types_spec.md) | **Draft v2.1** | Атомарные типы (`Tick`, `Voltage`), packed ABI (`PackedPosition`, `PackedTarget`, `SomaFlags`), seed/hash, константы. |
| `physics` | [physics_spec.md](spec_L0/physics_spec.md) | **Draft v2.0** | Математика GLIF, AHP, homeostasis, Active Tail, GSOP, DDS heartbeat, `v_seg`. |

### Слой 1 (Layer 1: Data Contracts & Deserialization)

| Крейт | Спецификация | Статус | Назначение |
|-------|--------------|--------|------------|
| `layout` | [layout_spec.md](spec_L1/layout_spec.md) | **Draft v2.0** | C-ABI макеты физической памяти (`VariantParameters`), выравнивание плоскостей SoA и заголовки файлов. |
| `config` | [config_spec.md](spec_L1/config_spec.md) | **Draft v2.0** | Serde/TOML DTO, парсинг и "Shift-Left" валидация DSL (`model.toml`, `department.toml`, `shard.toml`). |
| `wire` | [wire_spec.md](spec_L1/wire_spec.md) | **Draft v2.0** | C-ABI структуры сетевых и IPC пакетов, magic-константы, выравнивание, Little-Endian политика и `no-alloc` хелперы. |

### Слой 2 (Layer 2: Infrastructure & OS Isolation)

| Крейт | Спецификация | Статус | Назначение |
|-------|--------------|--------|------------|
| `ipc` | [ipc_spec.md](spec_L2/ipc_spec.md) | **Draft v2.0** | Жизненный цикл SHM/mmap, атомарные переходы Ночной Фазы (CAS), двойной буфер Swapchain и изоляция OS системных вызовов. |
| `vfs` | [vfs_spec.md](spec_L2/vfs_spec.md) | **Draft v2.0** | Контейнерный формат `.axic`, оглавление TOC, Read-Only mmap отображение, нормализация путей и примитивы экстракции. |

### Слой 3 (Layer 3: Hardware Acceleration & Compute Abstraction)

| Крейт | Спецификация | Статус | Назначение |
|-------|--------------|--------|------------|
| `compute-api` | [compute_api_spec.md](spec_L3/compute_api_spec.md) | **Draft v2.0** | Аппаратно-независимый HAL контракт бэкендов вычислений (`ComputeBackend`), непрозрачные VRAM handles и DTO команд. |
| `compute` | [compute_spec.md](spec_L3/compute_spec.md) | **Draft v2.0** | Фасад вычислений `ShardEngine`, автовыбор бэкендов (`BackendPreference`) и оркестрация жизненного цикла шарда. |

---

## §3. Реестры

- **Инварианты и ошибки**: [troubleshooting.md](troubleshooting.md)
- **Вопросы и замечания к ревью**: [review.md](review.md)
