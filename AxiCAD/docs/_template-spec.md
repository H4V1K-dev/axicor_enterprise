# [Module Name]

> One sentence: what this module does and why it exists.

## Status: Draft

## 1. Responsibility

What this module is responsible for.
What it is NOT responsible for (explicit boundaries).

## 2. Dependencies

- [dependency-name](../relative/path.md) — what is used and why
- Keep this list acyclic (DAG)

## 3. Contract

### Inputs
- What this module accepts (data types, formats, events)

### Outputs
- What this module produces (data types, events, side effects)

### Invariants
- What is always guaranteed by this module
- Pre-conditions and post-conditions

## 4. Data Model

Structures owned by this module.
Only what belongs here — reference other specs for shared types.

```
// Example pseudo-structure
struct ExampleState {
    field: Type,
}
```

## 5. Behavior

How the module works: algorithms, state transitions, processing rules.
Use diagrams, pseudocode, or step-by-step descriptions.

## 6. Edge Cases

Boundary conditions and how they are handled.

## 7. Open Questions

Unresolved technical questions about this module.

## 8. Changelog

| Date | Change |
|------|--------|
| YYYY-MM-DD | Created |
