---
name: axi-research
description: >
  AxiEngine research continuity. Use for Learning Proof, microcircuit, biocalibration,
  plastic network work, L0xx reviews, any fatigue/GSOP/DA/weights/topology numbers.
  Archive-first provenance — never invent knobs for green tests. Triggers: /axi-research,
  learning proof, LP-4, C4, calibration, preregistration, research archive, microcircuit.
---

# AxiEngine research continuity

## Number hierarchy (non-negotiable)

1. **Prior research** — `docs/engine/research/archive/**`, `Axicor_Neuron-Lib/modernized/*.toml`, `docs/engine/spec_L*`, or explicit table in `artifacts/agent-tasks/inbox/*`.
2. **Missing** → **unknown**. Do not invent so a test passes.
3. **Fallback** → biological approximation only, labeled `hypothesis / uncalibrated` in the cumulative report.
4. **Forbidden** — knobs for green asserts; post-hoc softening of acceptance; DA/GSOP/virt_w “for speed”.

Cite path for every non-default numeric.

## Units

- Mass: `i32`. Charge path: usually `weight >> 16`.
- Charge config (e.g. virt_w=3500) written to mass plane → `3500 << 16`.

## Layout

```text
docs/engine/research/archive/_active/<slug>/
  README.md           # short landing
  reports/<slug>.md   # cumulative science
  scripts/ images/    # only if real
```

Gates = **sections**, not new folders. Tasks = `artifacts/agent-tasks/`. Generated = gitignored `artifacts/`.

## Learning Proof

- `_active/learning_proof/` + `reports/learning_proof.md` only.
- C0–C3 toy ≠ license for invented C4 fixtures.
- C4+ transfer **calibrated microcircuit** lineage (plastic v1.4/v1.5 winners) unless task opens a separate calibration study.

## STOP

No provenance → stop. Encoder/readout unknown → stop or narrow calibration follow-up. Do not hide new knobs inside C4.

## Orchestrator reject if

Uncited numbers, silent physics retune after failures, claim > measured, LP-N folder clutter.
