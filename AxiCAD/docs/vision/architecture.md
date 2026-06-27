# Architecture Overview

> High-level system architecture: layers, data flow, and responsibility boundaries.

## Status: Draft

## 1. Architectural Goals

- **Modularity** — each subsystem is a self-contained module with explicit contract
- **Separation of concerns** — interactive layer (UI, Three.js, state) vs pure algorithms (math, portable to Rust)
- **Acyclic dependencies** — module graph is a DAG, no circular imports
- **Portability** — pure algorithm modules have zero DOM/browser dependencies

## 2. High-Level Layers

```
┌─────────────────────────────────────────────────┐
│                   UI Shell                       │  Interactive layer
│         (panels, toolbar, dialogs)               │  (browser-dependent)
├─────────────────────────────────────────────────┤
│               Viewport / Renderer                │
│         (Three.js, camera, gizmos)               │
├─────────────────────────────────────────────────┤
│              Editor Core (State)                 │  Orchestration layer
│   (selection, tools, command history, events)    │  (JS, minimal deps)
├─────────────────────────────────────────────────┤
│             Pure Algorithm Layer                 │  Portable layer
│  (geometry, spatial index, constraints,          │  (zero DOM deps,
│   validation, serialization)                     │   Rust-ready)
└─────────────────────────────────────────────────┘
```

## 3. Responsibility Boundaries

| Layer | Knows about | Does NOT know about |
|-------|------------|-------------------|
| UI Shell | DOM, CSS, user events | Geometry math, serialization |
| Viewport | Three.js, camera, rendering | Business logic, file I/O |
| Editor Core | Domain model, tool state, commands | Rendering details, DOM |
| Pure Algorithms | Math, data structures, grid | Any browser API |

## 4. Data Flow

```
User Input → UI Shell → Editor Core → Pure Algorithms
                                    ↓
                              Domain State
                                    ↓
                    Viewport ← (reads state, renders)
```

- **Down:** user actions flow down through layers
- **Up:** state changes propagate up via events/observers
- **Lateral:** Pure Algorithms never call Editor Core or UI Shell

## 5. Dependency Rules

1. Upper layers may depend on lower layers (UI → Core → Algorithms)
2. Lower layers NEVER depend on upper layers
3. Horizontal dependencies within a layer are allowed if acyclic
4. Cross-layer dependency inversion via interfaces/callbacks where needed

## 6. Module Inventory

> Filled in as specs are written. See [INDEX.md](../INDEX.md) for current map.

## Changelog

| Date | Change |
|------|--------|
| 2026-06-26 | Created — initial layer diagram |
