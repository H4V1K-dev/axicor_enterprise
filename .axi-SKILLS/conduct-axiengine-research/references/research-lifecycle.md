# Research Lifecycle

## Contents

1. Program state machine
2. Research unit decision
3. Durable artifact layout
4. Atomic transitions

## Program state machine

Use these states as a reasoning model even when legacy documents use different labels:

```text
idea -> registered -> preregistered -> running -> analyzed
     -> supported | weakened | rejected | inconclusive | invalid
     -> follow-up | blocked-by-code | ready-for-promotion | archived
```

- `invalid` means the experiment cannot answer its question because execution or design failed.
- `rejected` means valid evidence contradicted the hypothesis under stated conditions.
- `inconclusive` means the experiment was valid but lacked discriminating power.
- `blocked-by-code` means a production or harness boundary must change before the same question can be tested.

Do not conflate these outcomes.

## Research unit decision

Create a new program only when at least one holds:

- the primary question changes independently of the current program;
- the work needs its own baseline and preregistration lineage;
- the result will be consumed independently by multiple project areas;
- a durable decision boundary justifies a standalone reproducible package.

Keep work inside the current program when changing a seed, scale, ablation, parameter candidate, runner correction, or follow-up gate in the same causal chain.

## Durable artifact layout

For a cumulative multi-gate program, prefer:

```text
archive/_active/<program_slug>/
  README.md
  narrative.md
  studies/       # only for durable protocols or gate dossiers
  scripts/       # only program-specific scripts
  images/        # durable figures used in the argument
```

Keep README operational and compact:

- status and dates;
- current question and current gate;
- short verdict table for completed gates;
- exact active command;
- key outputs and links;
- current blocker and next action.

Keep `narrative.md` explanatory and cumulative. Keep raw machine output under repository `artifacts/` unless compact evidence is intentionally promoted.

Use this responsibility boundary throughout the lifecycle:

| Artifact | Update timing | Content boundary |
|---|---|---|
| README | every state transition | status, command, links, current blocker, next action |
| Narrative | before and after every meaningful gate | connected scientific argument and model evolution |
| Evidence report | after preregistration and runs | exact protocol, tables, metrics, controls, verdict record |
| Monospec/task contract | only when program rules change | gates, transition policy, definition of done |

A short single-gate study may use only README, scripts, and images. Do not create empty directories.

## Atomic transitions

When a gate changes state, synchronize all representations in the same task. Do not postpone narrative writing until final archival:

1. gate status in README;
2. new narrative movement;
3. evidence or report verdict;
4. current status map;
5. artifact and runner links;
6. next action.

Archive only when no active gate remains in that program. Use a completion date. If a follow-up remains part of the same causal question, keep the program active and mark the completed gate inside it.
