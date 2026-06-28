# Product Overview

> AxiCAD is a declarative 3D layout editor for designing, visualizing,
> and debugging spiking neural network architectures in voxel space.

## Status: Draft

## 1. Problem Statement

Designing SNN (Spiking Neural Network) architectures requires reasoning about
spatial layout, connectivity, and resource constraints simultaneously.
Current workflows rely on code-only definitions that lack visual feedback,
making it hard to spot structural errors, overlapping regions, and routing
conflicts until runtime.

AxiCAD provides a visual environment where the network layout is the
primary artifact — editable, inspectable, and exportable as a declarative
TOML specification consumed by the Rust compute engine and Python SDK.

## 2. Target Users

- **Network Architect** — designs the spatial layout of layers, departments,
  shards, and their connectivity using the 3D editor.
- **Researcher** — imports existing layouts, tweaks parameters, runs
  simulations through the SDK, inspects results back in the editor.

## 3. Target Workflow

```
Design in AxiCAD  →  Export TOML  →  Compile (Rust)  →  Simulate  →  Inspect in AxiCAD
       ↑                                                                    |
       └────────────────────────────────────────────────────────────────────┘
```

The editor is the hub: design goes out as TOML, simulation results come
back for visualization and debugging.

## 4. Design Constraints

- All geometry is discrete (voxel grid), no floating-point positions
- Output format is TOML, structured by domain hierarchy
- Editor runs in browser (Vanilla JS + Three.js + WebAssembly)
- Pure algorithms must be portable to Rust (no DOM/Three.js dependencies)
- Binary compatibility with SomaFlags, PackedTarget, PackedPosition

## 5. Success Criteria

- [ ] A network layout can be fully designed in the editor without writing code
- [ ] Exported TOML is valid and accepted by the Rust compiler
- [ ] Round-trip: load → edit → save → load produces identical results
- [ ] All constraint violations are visible before export

## 6. Non-Goals

- Real-time simulation inside the editor (simulation runs externally)
- Collaborative multi-user editing
- Mobile support

## Changelog

| Date | Change |
|------|--------|
| 2026-06-26 | Created — initial draft |
