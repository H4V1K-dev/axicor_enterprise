# Milestones

> Roadmap and delivery stages for AxiCAD.

## Status: Draft

## MVP (Milestone 1)

Goal: Minimal viable editor that can create a simple network layout and export valid TOML.

- [ ] Voxel grid rendering and camera navigation
- [ ] Place / move / delete shards on grid
- [ ] Layer and department hierarchy in UI
- [ ] Basic socket placement on shard boundaries
- [ ] Export to TOML (single file, flat structure)
- [ ] Load from TOML

## Milestone 2

Goal: Constraints, validation, and tract routing.

- [ ] Constraint engine (overlap checks, boundary rules)
- [ ] Validation engine with visual error display
- [ ] Tract routing between sockets
- [ ] Undo / redo (command history)
- [ ] Multi-file TOML export

## Milestone 3

Goal: Polish and integration with Rust compiler / Python SDK.

- [ ] Round-trip validation (export → compile → re-import)
- [ ] Simulation result overlay
- [ ] Performance optimization for large scenes
- [ ] Step-through debugging visualization

## Deferred

- Collaborative editing
- Plugin system
- Custom shader materials
- Animation timeline

## Changelog

| Date | Change |
|------|--------|
| 2026-06-26 | Created — initial roadmap sketch |
