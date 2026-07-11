---
name: write-axiengine-crate-spec
description: Create, update, synchronize, or review normative AxiEngine Rust crate specifications under docs/engine/spec_L*. Use when documenting a new crate, aligning an existing crate spec with Cargo manifests, Rust APIs, tests, adjacent layer contracts, INDEX.md, or review.md, resolving documented architecture decisions, defining ownership boundaries and invariants, building required test matrices, or auditing specification drift. Do not use for research reports or AxiCAD product specifications unless the task explicitly asks to apply the crate-spec contract style.
---

# Write AxiEngine Crate Specifications

Treat a crate specification as an executable architecture contract, not as a README or a prose mirror of the implementation. Recover the contract from project evidence, distinguish normative intent from implementation reality, and make disagreements explicit.

## Resolve the repository

Locate the repository root before reading or writing. Expect these project paths relative to it:

- `AxiEngine/Cargo.toml`: workspace membership.
- `AxiEngine/crates/<crate>/Cargo.toml`: dependency and feature evidence.
- `AxiEngine/crates/<crate>/src/`: implementation and public API evidence.
- `AxiEngine/crates/<crate>/tests/` and `examples/`: verification evidence.
- `docs/engine/INDEX.md`: architecture graph, layer, status, and registry.
- `docs/engine/review.md`: unresolved questions and architectural debt.
- `docs/engine/troubleshooting.md`: cross-system operational invariants.
- `docs/engine/spec_L*/`: target and neighboring normative specifications.

Read [references/evidence-policy.md](references/evidence-policy.md) before interpreting disagreements. Read [references/spec-structure.md](references/spec-structure.md) before drafting or restructuring a specification.

## Select the operating mode

Choose one mode from the request and evidence:

1. **Create**: define a new normative contract. Mark unimplemented surfaces as planned or staged; never present them as implemented.
2. **Synchronize**: align a specification with an intentional code or architecture change. Preserve still-valid decisions and IDs.
3. **Review**: compare the specification, code, tests, index, and neighboring contracts. Report drift without changing files unless the user requested edits.
4. **Resolve debt**: apply an explicitly accepted `REV-*` decision to every affected specification, invariant, test requirement, and registry entry. Do not invent the resolution.

If the request is ambiguous between review and mutation, review first. Do not modify code merely because a specification exposes implementation drift unless the user also requested implementation.

## Build the evidence map

Inspect the smallest complete evidence set before writing:

1. Read the full target specification when it exists.
2. Read the crate manifest, crate root, public modules, public DTOs/traits/errors, tests, and relevant examples.
3. Read direct dependency specifications and the specifications of important outbound consumers.
4. Read the matching `INDEX.md` entry and relevant `review.md` items.
5. Search the repository for every public name, constant, format magic, invariant ID, and `REV-*` ID that the specification will assert.
6. Classify each material claim as implemented, normative, planned, deferred, resolved, or open.

Use targeted search before opening large files. For source files, inspect definitions and call sites instead of relying only on crate-level rustdoc.

## Model the contract

Derive these facts before drafting:

- identity, layer, crate type, `no_std`/`alloc` posture, and lifecycle status;
- staged scope and explicit out-of-scope behavior;
- inbound dependencies, outbound consumers, external dependencies, features, and forbidden dependencies;
- exclusive ownership and responsibilities delegated to neighboring crates;
- public contract: DTOs, traits, state machines, formats, algorithms, formulas, errors, and validation rules;
- determinism, memory, ABI, concurrency, safety, and platform constraints;
- stable invariants and the tests that prove them;
- unresolved debt and already resolved decisions.

Do not start from headings and fill them mechanically. First construct the contract model, then select the applicable structure and profile from `references/spec-structure.md`.

## Write the specification

Preserve the established AxiEngine style:

- Write explanatory prose in Russian while retaining exact Rust names and useful English architecture terms.
- Use normative words such as `обязан`, `запрещено`, and `строго` only for supported contract requirements.
- State both positive ownership and negative boundaries. Identify the neighboring owner when forbidding a responsibility.
- Separate current implementation from target contract, future stage, deferred scope, and open question.
- Keep a single source of truth for constants, formulas, binary layouts, and policies; make consumers reference the owning crate.
- Assign stable `INV-<DOMAIN>-<NNN>` IDs to critical, testable properties. Preserve existing IDs unless their meaning is removed.
- Map critical invariants to named tests or explicit required-test descriptions.
- Keep resolved decisions traceable to `REV-*` items. Move unresolved disagreements to `review.md` rather than disguising them as decisions.
- Describe implementation details only when they are contractually observable, required for safety/determinism/ABI, or necessary to prevent architectural drift.
- Keep section numbering and terminology internally consistent.

Do not copy large neighboring sections. State the local obligation and link or name the owning contract.

## Reconcile conflicts

When evidence disagrees:

1. Record the conflicting normative and implementation facts.
2. Determine whether an explicit status, stage, or resolved decision explains the difference.
3. Fix mechanical drift when intent is unambiguous, such as an outdated index version in an authorized edit task.
4. Preserve both sides and create or update review debt when intent is not established.
5. Ask the user only when choosing a side would create a new architecture decision.

Never silently make code authoritative over an approved contract or make an aspirational spec authoritative over current implementation status.

## Validate the result

Run the project consistency checker from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .axi-SKILLS/write-axiengine-crate-spec/scripts/check-spec-consistency.ps1
```

Then verify manually:

- every asserted dependency matches the manifest or is clearly planned;
- every asserted public symbol exists or is clearly planned;
- numeric constants, layouts, formulas, error variants, and CLI exit codes match evidence;
- invariant IDs are unique declarations and retain their meanings;
- each critical invariant has a verification path;
- `INDEX.md` version, layer, status, and purpose agree with the target spec;
- links and `REV-*` references resolve;
- no unresolved question is presented as a final decision.

Report the files changed, validations run, and remaining normative-versus-implementation disagreements.

