# Specification Structure

## Contents

1. Mandatory contract spine
2. Conditional sections
3. Crate profiles
4. Invariants and tests
5. Style rules

## Mandatory contract spine

Use this semantic order. Renumber sections to fit the existing target document without gratuitous churn.

1. **Identification**: crate name, binary name when applicable, layer, type, `no_std`, concise responsibility, version, date, and status when needed.
2. **Scope or staged scope**: implemented stage, planned stages, and explicit exclusions when the contract evolves incrementally.
3. **Stack and environment**: inbound dependencies, outbound consumers, external dependencies, feature flags, and forbidden dependencies or operations.
4. **Ownership boundaries**: exclusive local ownership, delegated responsibilities, and prohibited duplication.
5. **Domain contract**: select only the sections required by the crate profile.
6. **Errors and validation**: rejection rules, panic policy, recovery, and externally observable failures.
7. **Required invariants**: stable, atomic, testable properties.
8. **Golden tests / required test matrix**: evidence required to establish the contract.
9. **Open or deferred debt**: unresolved items only.
10. **Resolved decisions**: accepted decisions with traceable `REV-*` identifiers when they materially explain the contract.

Identification, environment, and test requirements are always required. Ownership and invariants are required for library and infrastructure crates unless the document explicitly explains why they do not apply.

## Conditional sections

Add sections only when contractually relevant:

- public API and DTO registry;
- constants and physical limits;
- binary formats, ABI sizes, alignment, and endianness;
- formulas and numeric boundary behavior;
- state machines and permitted transitions;
- lifecycle and teardown behavior;
- execution stages and ordering;
- concurrency, memory ordering, and single-writer policy;
- determinism and stable ordering keys;
- CLI grammar, output streams, files, and exit codes;
- platform isolation and feature-gated behavior;
- code generation or FFI parity;
- security and malformed-input handling.

## Crate profiles

Select one primary profile and any necessary secondary profiles:

| Profile | Contract emphasis |
|---|---|
| Pure math / physics | formulas, integer domains, overflow behavior, purity, determinism, hot-loop prohibitions |
| Types / ABI / layout / wire | representation, sizes, alignment, endianness, sentinels, safe parsing, cross-language parity |
| Configuration / validation | schema, defaults, identity, validation order, error paths, cross-field rules |
| HAL / compute backend | traits, handles, capabilities, resource lifecycle, batch contract, concurrency, backend parity |
| Algorithm / topology | inputs, outputs, stable ordering, capacity limits, staged algorithms, determinism |
| Storage / IPC / transport | formats, lifecycle, bounded resources, synchronization, recovery, platform isolation |
| Orchestrator / runtime | state transitions, sequencing, delegation, fault behavior, shutdown |
| CLI / process | grammar, exit codes, stdout/stderr policy, atomic output, delegated work, out-of-scope behavior |
| Test harness | fixtures, comparison policy, diagnostics, determinism, first-mismatch reporting |

Do not force irrelevant ABI, feature, or error sections into every profile. Preserve the shared reasoning sequence while adapting the domain middle.

## Invariants and tests

Write one property per invariant. Use stable declarations such as:

```text
INV-COMPUTE-CPU-004: Results are bit-identical for one and N worker threads.
```

Do not encode temporary version numbers in invariant IDs. Use stage-qualified IDs only when the stage is a durable part of the contract.

For each critical invariant, specify at least one of:

- an existing named unit or integration test;
- a compile-time assertion;
- a cross-backend differential test;
- a format golden vector;
- a static dependency/API check;
- an explicitly required future test for planned functionality.

Distinguish an existing test from a required test that has not been implemented.

## Style rules

- Write Russian prose with exact code identifiers in backticks.
- Introduce useful English terms in parentheses; do not alternate terminology randomly.
- Use tables for registries and exact mappings, lists for rules, formulas for numeric contracts, and diagrams only for state or dependency relationships.
- Prefer concise normative statements. Explain rationale when it prevents future reinterpretation.
- Avoid promotional language, vague quality claims, and implementation-history narration.
- Preserve existing section and invariant identifiers during synchronization unless their semantics truly change.

