# Executable task contract

Use this structure selectively; omit empty sections and rename headings to match the task language. Preserve the semantic responsibilities.

## Identity block

Record:

```text
STATUS: DRAFT | READY | BLOCKED | ACCEPTED
TYPE: IMPLEMENTATION | RESEARCH | SPECIFICATION | REVIEW | DESIGN
PRIORITY: P0 | P1 | P2 when useful
DEPENDS: accepted task, decision, gate, or none
SKILLS: exact .axi-SKILLS/<name>/SKILL.md paths
NORMATIVE: exact sources and sections
REPO: repository-relative working root
```

Do not use `DONE` to mean both “executor says finished” and “reviewer accepted.” Preserve an established workflow when reviewing legacy tasks, but for new task systems distinguish execution completion from acceptance.

## Context and objective

Explain in a short paragraph why this task is next and what uncertainty or missing behavior it closes. State one primary objective as an observable result. Keep program-level aspirations outside the task objective.

## Sources and authority

List each required source with the fact it owns. Mark legacy read-only. Mark code as as-built evidence when a specification owns the target.

## Scope and ordered deliverables

Order work by dependency and verification:

1. prerequisite inspection or preregistration;
2. smallest authorized mutation or experiment;
3. evidence collection;
4. synchronized status/documentation update;
5. handoff.

Use checkpoints when later work must not begin before review or evidence. Write an explicit `STOP` boundary.

## Invariants and constraints

State non-negotiable ownership, units, domains, safety, determinism, compatibility, frozen parameters, and forbidden shortcuts. Link the owning source instead of duplicating large rules.

Separate `required` from `prefer` and `hints`. An example must not silently become a mandated architecture.

## Out of scope

Name attractive adjacent work that the executor might otherwise absorb. Identify its future owner or task when known.

## Acceptance

Map each required outcome to observable evidence:

| Outcome | Evidence | Exact command or artifact | Claim limit |
|---|---|---|---|

Include failure semantics. State whether rejection/null is valid completion, whether a blocker requires a follow-up task, and which failures invalidate the run.

## Stop and ask conditions

Require a question before unapproved architecture, public API, external dependency, handwritten unsafe, ABI break, parameter invention, threshold weakening, or production mutation outside scope.

## Handoff

Require changed files, exact commands and outcomes, verdict, caveats, unresolved contradictions, next action, and what evidence does not prove. Do not request full terminal logs.
