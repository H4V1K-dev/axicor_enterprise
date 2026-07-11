---
name: implement-axiengine-rust-change
description: Implement, extend, refactor, or repair production Rust code in the AxiEngine workspace while preserving the intended crate profile, module ownership, public API, rustdoc, invariants, feature behavior, and verification discipline. Use when creating a crate, adding or changing Rust types, traits, algorithms, backends, runtime orchestration, validation, errors, manifests, features, tests, or public exports under AxiEngine/crates. Also use for implementation plans that will mutate AxiEngine Rust code. For research-only runners, apply conduct-axiengine-research as the governing workflow and use this skill only for the Rust implementation quality of the authorized runner.
---

# Implement AxiEngine Rust Change

Treat an implementation as a coherent contract change, not an isolated code insertion. Preserve the crate's role, expose the smallest intentional API, write rustdoc with the code, and prove the affected behavior at the cheapest sufficient level.

## Resolve authority and scope

Read the repository `GEMINI.md` and obey its implementation contract. Resolve evidence in this order:

1. the user's current task and acceptance criteria;
2. an explicitly named active design or handoff;
3. the relevant normative crate specification under `docs/engine/spec_L*/`;
4. accepted cross-cutting decisions in `docs/engine/review.md` and neighboring crate specifications;
5. current source and tests as evidence of the as-built state;
6. `axicor-master` only as a read-only source of algorithmic knowledge, never structure or API design.

Do not turn research findings into production behavior unless the task authorizes implementation. When a production change requires editing a crate specification, read and apply `write-axiengine-crate-spec` before changing the specification.

Before editing, state the smallest intended scope and distinguish:

- behavior required by the governing contract;
- mechanical consequences such as call-site or initializer updates;
- nearby debt that must remain untouched;
- genuine ambiguity that changes ownership, public behavior, ABI, safety, or lifecycle semantics.

Ask only about genuine contract ambiguity. Continue through ordinary implementation choices that are already constrained by the task, specification, neighboring code, and Rust idioms.

## Select implementation depth

Choose depth by semantic impact, not line count.

### Local change

Use for private logic, a contained bug fix, or an implementation detail that preserves public contracts, crate ownership, manifests, features, ABI, and lifecycle.

- Inspect the owning module, direct callers, local invariants, and nearest tests.
- Preserve the existing module shape.
- Run targeted formatting, compilation, linting, and tests.
- Do not perform a crate-wide redesign review.

### Contract change

Use when changing a public item, error, DTO, trait, export, serialized form, configuration field, feature, or behavior consumed outside the owning module.

- Read the full relevant crate specification and inspect direct consumers.
- Search all constructors, pattern matches, implementations, re-exports, feature-gated call sites, tests, and rustdoc references.
- Update implementation, rustdoc, validation, errors, and tests as one transaction.
- Reconcile specification impact explicitly; never hide contract drift.

### Crate or architecture change

Use for a new crate, ownership movement, backend boundary, lifecycle redesign, new external dependency, new handwritten `unsafe`, or ABI change.

- Build an inbound/outbound dependency and ownership map before editing.
- Read neighboring specifications and `docs/engine/INDEX.md`.
- Confirm authority for architectural decisions not already fixed by the task or normative sources.
- Validate all affected packages, features, targets, formats, and integration boundaries.

Do not escalate a local change merely because the repository is complex. Escalate when an observable contract or architectural boundary moves.

## Identify the crate profile

Classify every affected crate before choosing module structure. Read [references/crate-profiles.md](references/crate-profiles.md) for a new crate, a cross-module change, or uncertainty about structure. Use profiles as constraints and heuristics, not templates to copy mechanically.

Record the relevant posture:

- layer and exclusive responsibility;
- library, binary, facade, backend, or pipeline role;
- `no_std`, allocation, and side-effect posture;
- public API and re-export convention;
- ABI, determinism, lifecycle, concurrency, and platform constraints;
- allowed dependency direction and feature surface.

Follow the stable convention of the affected crate. Do not normalize wildcard versus explicit re-exports, module visibility, naming, or file layout outside the requested change.

## Build the impact map

Use targeted search before opening large files. Inspect definitions and semantic consumers, not only filenames.

For every changed contract, find as applicable:

- constructors and struct literals;
- enum matches and error conversions;
- trait implementations and dynamic dispatch boundaries;
- public re-exports and downstream imports;
- serialization, binary layout, FFI, shared memory, or wire uses;
- feature-gated and target-gated code;
- mock, CPU, CUDA, runtime, CLI, and test-harness consumers;
- unit, integration, doctest, snapshot, and compile-time assertions;
- rustdoc links and crate specification claims.

Treat a new field, variant, method, feature, or invariant as incomplete until its relevant call sites and proofs are accounted for.

## Design the smallest coherent change

Read [references/module-api-rustdoc.md](references/module-api-rustdoc.md) before adding public API, creating modules, or restructuring a crate.

Prefer these boundaries when they fit the crate profile:

- keep `lib.rs` as crate identity and intentional export surface;
- place domain data contracts separately from execution machinery when they serve multiple implementations;
- centralize validation at the boundary where invalid input first becomes observable;
- use typed errors and checked arithmetic instead of hidden panics;
- keep backend-specific mechanisms behind shared contracts;
- keep process wiring in binaries and reusable behavior in libraries;
- make illegal or ambiguous states harder to construct when the contract permits it;
- avoid abstractions, configuration, dependencies, and features that only anticipate possible future work.

Do not invent architecture from neighboring code when the specification already owns the decision. Do not copy visible historical inconsistencies as conventions.

## Implement code and documentation together

Write all code, identifiers, comments, error messages, and rustdoc in English. Preserve user-facing language only where an established product contract requires it.

For every new or changed public item, write rustdoc that explains its purpose and relevant invariants. Include units, ranges, sentinels, ordering, ownership, lifetime, determinism, state requirements, buffer shape, or compatibility obligations where relevant. Add `# Errors`, `# Panics`, `# Safety`, and useful compiling examples when applicable.

Keep comments causal: explain why an invariant holds or why a non-obvious choice exists. Do not embed task IDs, stage labels, temporary planning language, or a paraphrase of the code into production comments.

Preserve error semantics and validation order when observable. Avoid `unwrap`, `expect`, and `panic!` in library or runtime paths unless the governing contract establishes impossibility and the choice is documented.

For ABI, wire, raw memory, or `unsafe` work, read [references/safety-and-abi.md](references/safety-and-abi.md) before editing. Never introduce handwritten `unsafe` without explicit authority.

Keep tests hermetic by default. Do not embed machine-specific absolute paths or write fixed repository artifacts from an ordinary regression test. A research runner that intentionally emits artifacts must resolve repository-relative paths portably, document the side effect and output contract, and normally remain isolated or ignored so routine package tests do not rewrite evidence.

## Prove the changed contract

Read [references/verification-matrix.md](references/verification-matrix.md) when the change crosses modules, crates, features, targets, ABI, backends, or lifecycle states.

Choose the cheapest sufficient proof:

- unit test for private algorithms and boundary cases;
- integration test for public crate behavior;
- compile-time size, alignment, trait, or layout assertion for binary contracts;
- doctest for a genuinely useful public example;
- targeted cross-crate or end-to-end test only for a cross-crate promise;
- feature/target matrix only for affected conditional paths.

Test invariants and failure behavior, not implementation trivia. Cover boundary values, checked overflow, invalid state, buffer sizes, sentinel values, determinism, and cleanup when the changed contract depends on them.

For comparison or research runners, verify fixture construction and branch selection with assertions or independent output validation. A `#[test]` that only prints or writes a table is a runner, not proof that the comparison was constructed correctly.

Run the advisory source audit from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .axi-SKILLS/implement-axiengine-rust-change/scripts/check-rust-change.ps1
```

Then run proportional `cargo fmt`, `cargo check` or `cargo build`, `cargo test`, and `cargo clippy` commands for the affected packages, features, targets, and tests. Preserve the real cargo exit code. Do not use a full workspace sweep when targeted evidence is sufficient, and do not claim broader coverage than the commands establish.

## Stop at protected boundaries

Pause before the affected part when completing it would require an unapproved:

- ownership or dependency-direction change;
- public contract choice not resolved by authoritative sources;
- external dependency or feature policy;
- handwritten `unsafe` boundary;
- ABI or persistent/wire format break;
- production semantic change derived only from research;
- specification decision disguised as an implementation detail.

Implement independent, unambiguous parts when safe. Describe the exact conflict, affected contract, and available choices without selecting architecture on the user's behalf.

## Hand off precisely

Report:

- changed files and the contract each change implements;
- exact build, test, clippy, and formatting commands;
- passed, failed, and ignored counts or concise build outcomes;
- features and targets actually covered;
- external dependencies, authorized `unsafe`, or legacy knowledge used;
- specification drift, unresolved ambiguity, and untouched nearby debt;
- what the evidence proves and what it does not prove.

Do not use a green test suite as evidence for an unstated architectural, performance, biological, or cross-platform claim.
