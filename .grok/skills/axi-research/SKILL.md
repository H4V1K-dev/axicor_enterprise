---
name: axi-research
description: >
  AxiEngine research continuity skill. Use for Learning Proof, microcircuit,
  biocalibration, plastic network experiments, agent-tasks L0xx reviews, and any
  work that sets fatigue/GSOP/DA/weights/topology numbers. Enforces archive-first
  provenance so agents do not invent parameters. Triggers: /axi-research,
  learning proof, LP-4, C4, microcircuit, calibration, preregistration, research
  archive, "from prior research", orchestrator review of research tasks.
---

# AxiEngine research continuity

## Goal

Keep research **cumulative** and **sourced**. Green tests are not knowledge.

## Number hierarchy (non-negotiable)

1. **Prior research** — `docs/engine/research/archive/**`, profiles in `Axicor_Neuron-Lib/modernized/`, production specs under `docs/engine/spec_L*`, or an explicit table in `artifacts/agent-tasks/inbox/*`.
2. **Missing** → state **unknown**. Do not invent.
3. **Fallback** → biological approximation only, labeled `hypothesis / uncalibrated` in the cumulative report.
4. **Forbidden** → knobs chosen so the assert passes; silent post-hoc acceptance softening; DA/GSOP/virt_w “for speed”.

Cite path (+ commit if known) for every non-default numeric.

## Units

- Mass plane: `i32` synaptic mass.
- Charge / current path: typically `weight >> 16`.
- If config is charge-domain (e.g. virt_w = 3500) and code writes mass plane → `3500 << 16`.

## Layout (one research line)

```text
docs/engine/research/archive/_active/<slug>/
  README.md                 # short: question, phase table, conclusion, links
  reports/<slug>.md         # cumulative scientific report
  scripts/ images/          # only if real
```

- Gates/phases/tasks are **sections**, not new `_active` folders.
- Executor instructions only in `artifacts/agent-tasks/` (inbox/reviews/done).
- Generated CSV/JSON → gitignored `artifacts/`, re-run via commands in the report.

## Learning Proof specifically

- Package: `_active/learning_proof/` + `reports/learning_proof.md`.
- C0–C3 toy evidence does **not** authorize new invented fixtures for C4+.
- C4+ must transfer **calibrated microcircuit** lineage (e.g. plastic v1.4/v1.5 winners) unless the task explicitly opens a new calibration study.
- Append results to the cumulative report; refresh only the short README table.

## Orchestrator accept bar

Reject delivery if:

- uncited numbers entered the runner;
- physics/winner changed after seeing failures without a logged calibration follow-up;
- claim level > what was measured;
- new research folders/files-per-gate clutter.

Accept null/WEAKENED if honest and sourced.

## Agent stop conditions

STOP and ask (do not improvise) if:

- required encoder/readout/horizon not determined by prior research or task table;
- mass/charge encoding unclear;
- task asks for product architecture (Training modes) before evidence allows it.

## Minimal checklist before code

- [ ] Read task + RULES §0 + active report
- [ ] Provenance table drafted
- [ ] No uncited knobs
- [ ] Units consistent
- [ ] Evidence goes to cumulative report, not a new LP-N folder
