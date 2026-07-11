---
name: author-axiengine-agent-task
description: Create, refine, audit, split, and synchronize executable AxiEngine task contracts under artifacts/agent-tasks. Use when turning an idea, research frontier, accepted design, review finding, specification change, bug, or implementation objective into an inbox task; when deciding task boundaries and dependencies; when writing acceptance gates, exact verification commands, source-of-truth maps, stop conditions, research checkpoints, or handoff requirements; and when updating QUEUE.md or the current agent-task README to route work to a fresh agent.
---

# Author an AxiEngine Agent Task

Turn project intent into an executable contract that lets a fresh agent act quickly without inventing architecture, evidence rules, or completion semantics. Write the task for execution and review, not as a motivational brief or a second copy of the project documentation.

## Resolve the task frontier

Before drafting, identify:

1. the current project state and why this task is next;
2. the single decision boundary or observable outcome the task must reach;
3. authoritative sources that already own architecture, parameters, invariants, and status;
4. prerequisites that are actually complete;
5. work intentionally deferred to later tasks;
6. the evidence a reviewer will use to accept or reject completion.

Inspect the relevant specification, design, research status, code, tests, prior task verdicts, and queue entry. Use targeted search. Do not make the executor rediscover contradictions the task author could resolve now.

If the objective is still an architectural choice rather than executable work, author a design or decision task instead of disguising the choice as implementation.

## Choose one task profile

Select the primary profile and read [references/task-profiles.md](references/task-profiles.md):

- **Implementation**: production Rust behavior, crate structure, API, manifests, or tests.
- **Research**: a preregistered gate that changes understanding.
- **Specification/documentation**: a normative contract or consistency change.
- **Review/audit**: evidence-backed assessment without unauthorized mutation.
- **Design/decision**: resolve an open architecture boundary before implementation.

Mixed tasks are allowed only when the parts form one atomic outcome. A research task may include a research-only runner; a contract change may require code plus synchronized specification. Split work when parts have independent verdicts, different authority, or can safely land separately.

Name the applicable project skills explicitly in task metadata. Do not paste their complete instructions into the task.

## Define the task boundary

Write one primary objective in terms of an observable transition:

```text
Current state -> authorized change or experiment -> acceptance evidence
```

Separate four kinds of content:

- **Normative**: behavior or structure already decided by authoritative sources.
- **Required work**: concrete deliverables needed to reach the objective.
- **Preferred approach**: guidance the executor may adapt while preserving the contract.
- **Non-normative hints**: examples and likely source locations that must not become accidental requirements.

Do not prescribe internal implementation details unless they preserve ownership, compatibility, safety, determinism, or a reviewed decision. Do not leave public behavior, ownership, units, feature policy, or verdict semantics to the executor's taste.

## Build the source-of-truth map

List only sources the executor must actually use, in priority order. For each source, state what it owns:

- task/request: scope and acceptance;
- active design or handoff: accepted architecture;
- crate specification: normative contract and invariants;
- research program/report: evidence and frozen parameters;
- production code: as-built behavior;
- tests: executable evidence, not architecture authority;
- legacy: read-only algorithmic knowledge only.

Use repository-relative paths. Avoid machine-specific roots such as `W:\Workspace`. Cite exact sections, types, functions, gates, or decisions when a large source is involved.

## Write the executable contract

Read [references/task-contract.md](references/task-contract.md) and include the applicable sections. Every READY task must make these facts discoverable:

- identity, status, type, priority, dependencies, and applicable skills;
- context and one primary objective;
- authoritative sources and owned facts;
- ordered scope and deliverables;
- invariants, constraints, and forbidden shortcuts;
- explicit out-of-scope work;
- acceptance evidence and exact verification commands;
- stop/ask conditions for unresolved protected boundaries;
- required handoff content and claim limits.

State paths and commands exactly enough to run, while allowing the executor to adjust a placeholder only when the task says how to resolve it.

## Author research tasks safely

Apply `conduct-axiengine-research` while authoring research work. Define one active gate at a time even when the program roadmap contains later hypotheses.

Require before target results:

- question, prior model, competing explanations, controls, provenance, metrics, verdict rules, limits, and exact planned command;
- registration and artifact topology consistent with the cumulative program;
- `narrative.md` from the start for a multi-gate program, unless the task explicitly and honestly defines a short single-gate study;
- one next action after the gate, not speculative execution of the entire roadmap.

For parity, transfer, replay, or legacy comparisons, require an input-equivalence table covering values and outer control flow: enable flags, lifecycle state, timer/refractory gates, early exits, sentinels, conversions, clamps, and mutation timing. If the helper models only inner arithmetic, prohibit whole-path `identical inputs` claims.

Require sanity assertions or independent output validation. State that a green runner proves execution only. Require invalid-run preservation and correction notes rather than silent replacement.

Keep the claim boundary aligned with the gate. A component mismatch may motivate a behavioral follow-up but cannot be tasked to explain behavioral failure without that follow-up evidence.

## Author implementation tasks safely

Apply `implement-axiengine-rust-change` while authoring production Rust work. State the affected crates and whether the task is local, contract-level, or architectural.

Fix in the task any already-decided public API, ownership, lifecycle, ABI, feature, `no_std`, safety, dependency, and compatibility requirements. Require the executor to inspect all relevant initializers, matches, trait implementations, re-exports, feature-gated call sites, and consumers.

Acceptance must pair each changed behavior with proof at the appropriate level. Require proportional format, build/check, test, and clippy commands with exact packages, features, targets, and test names. Do not demand `--workspace --all-targets` by habit when narrower evidence is sufficient.

Research-only runners must follow the research artifact and side-effect rules. Ordinary regression tests must not write fixed repository artifacts or contain machine-specific paths.

## Make acceptance reviewable

Read [references/acceptance-and-handoff.md](references/acceptance-and-handoff.md). Write acceptance as observable facts, not activities:

- weak: `implement validation`, `tests green`, `update docs`;
- strong: `invalid padded_n returns AlignmentViolation`, named test and exact command, affected spec/index entries agree.

Distinguish mandatory gates from optional polish. State what a failed or null outcome means. A research rejection may be valid DONE; an implementation test failure is not completion unless the task is diagnostic.

Do not let a command prove more than its asserts. Require the handoff to state what was and was not established.

## Synchronize task routing

Place executable tasks under `artifacts/agent-tasks/inbox/`. Keep designs, monospecs, and durable decision documents outside inbox. Use `done/` only after acceptance according to the active workflow; use `reviews/` for reviewer verdicts.

When changing the active frontier, synchronize as one transaction:

- the task file and its status;
- `QUEUE.md` dependency/order state;
- `artifacts/agent-tasks/README.md` current handoff pointer;
- referenced research or design status only when the task authoring request includes that mutation.

Do not mark a program complete merely because one gate task is complete. Do not leave a DONE task as the active inbox pointer.

## Validate the task

Run from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .axi-SKILLS/author-axiengine-agent-task/scripts/check-agent-task.ps1 -Path artifacts/agent-tasks/inbox/<task>.md -Strict
```

Use non-strict mode to audit legacy tasks without failing on conventions introduced later. Then manually verify:

- a fresh agent can identify the first action without broad rediscovery;
- no unresolved architecture choice is hidden inside implementation wording;
- every number and strong claim has an owner or is explicitly unknown;
- artifact layout agrees with the applicable skill;
- commands and acceptance cover the actual semantic risk;
- scope is small enough for one coherent verdict;
- the task does not encode a desired result as experimental success.

Report the task files and routing files changed, validator result, remaining open decisions, and why the chosen boundary is atomic.
