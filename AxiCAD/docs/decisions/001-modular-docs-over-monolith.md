# ADR-001: Modular Documentation Over Monolithic Design Document

> Chose a system of isolated spec files over a single monolithic design document.

## Status: Accepted

## Date: 2026-06-26

## Context

Starting a new 3D editor project (AxiCAD) from scratch. An initial draft
proposed a single Design Document with 15 chapters covering everything from
product vision to testing strategy.

Past experience showed that monolithic design documents:
- Become stale within days of active development
- Have no clear ownership (who maintains chapter 7 vs chapter 12?)
- Cannot be updated independently (editing sockets requires reading the whole doc)
- Grow until nobody reads them

## Options Considered

### Option A: Single Monolithic Document
- Pros: Everything in one place, traditional format
- Cons: Doesn't scale, no isolation, unclear ownership, stale fast

### Option B: Modular Spec System (one module = one file)
- Pros: Isolation, clear ownership, independent updates, DAG dependencies
- Cons: Requires discipline (index maintenance, cross-references)

## Decision

Option B — modular spec system with three tiers (vision / domain / specs),
governed by a RULES.md constitution and linked via INDEX.md.

## Consequences

- Every new module requires creating a spec file from template
- INDEX.md must be kept in sync (manual overhead)
- Cross-references use relative links (may break on restructuring)
- Documentation mirrors code modularity — intentional alignment
