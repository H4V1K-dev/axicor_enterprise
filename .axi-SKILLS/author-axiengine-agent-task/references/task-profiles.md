# Task profiles

## Implementation

Include:

- current versus target behavior;
- owning crate and neighboring consumers;
- public API and compatibility impact;
- feature, target, `no_std`, dependency, ABI, safety, lifecycle, and determinism constraints;
- required call-site and initializer search;
- named tests and proportional cargo commands;
- specification synchronization when authorized.

Avoid prescribing private file layout unless architecture owns it. Avoid full-workspace verification as a default acceptance ritual.

## Research

Include:

- program, active gate, prior evidence, and project decision affected;
- preregistration checkpoint before target results;
- baseline, intervention, controls, seeds, horizon, metrics, sanity gates, verdict rules, and limits;
- parameter provenance and unit/domain mapping;
- exact runner and artifact topology;
- invalid versus rejected versus inconclusive semantics;
- cumulative README, evidence report, living narrative, and status-map updates;
- one next action and a stop boundary.

For parity work, require whole-path semantic mapping, not only formula comparison. For promotion, separate research evidence from authorized production implementation.

## Specification or documentation

Include:

- create, synchronize, review, or resolve-debt mode;
- normative versus as-built evidence;
- affected invariant IDs, ownership boundaries, dependencies, formats, and tests;
- neighboring specifications and index impact;
- unresolved review debt that must remain open;
- consistency validator and link checks.

Do not authorize production changes implicitly through a documentation task.

## Review or audit

Include:

- exact review question and source set;
- severity or verdict vocabulary;
- read-only boundary and whether rerunning tests is allowed;
- evidence required for findings;
- baseline debt treatment;
- expected output location and whether fixes are explicitly out of scope.

Do not ask the reviewer to “improve anything found” unless mutation scope is intentionally broad and safe.

## Design or decision

Include:

- unresolved decision and why implementation is blocked;
- known constraints, alternatives, and evaluation criteria;
- consumers affected by the decision;
- required decision artifact and downstream spec/task updates;
- explicit prohibition on production implementation until accepted.

End at an accepted or clearly unresolved decision, not a speculative implementation plan.
