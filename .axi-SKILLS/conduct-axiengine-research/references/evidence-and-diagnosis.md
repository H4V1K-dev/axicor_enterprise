# Evidence and Diagnosis

## Contents

1. Preregistration minimum
2. Evidence ledger
3. Verdict discipline
4. Bottleneck routing
5. Promotion boundary

## Preregistration minimum

Record before target results:

| Field | Required content |
|---|---|
| Question | One falsifiable question tied to a project decision |
| Prior model | Why the expected result is plausible |
| Competing explanations | At least the alternatives the controls can distinguish |
| Provenance | Path, revision when known, value, unit, and mass/charge domain |
| Baseline | Frozen comparison state |
| Conditions | Intervention, control, ablations, seeds, scale, horizon |
| Metrics | Primary effects plus safety and physiology gates |
| Verdict rules | Support, weaken, reject, invalid, or inconclusive conditions |
| Reproduction | Runner, command, flags, inputs, and output paths |
| Limits | Claims the experiment cannot establish |

If exploration is necessary, label it exploratory and use it to design a later gate rather than to claim confirmation.

## Evidence ledger

Maintain a compact cumulative table for gates or runs:

| ID | Date | Question | Baseline | Intervention | Controls | Result | Evidence level | Consequence |
|---|---|---|---|---|---|---|---|---|

Link each row to exact commands and durable evidence. Keep repeated seeds and conditions together instead of creating version folders.

## Verdict discipline

- Use `observed` for a direct single-run fact.
- Use `reproduced` for repeated or independently obtained behavior.
- Use `supported` when evidence favors a mechanism within scope.
- Use `confirmed` only after the full relevant loop and no known contradiction in scope.
- Use `weakened` when evidence reduces plausibility without decisive rejection.
- Use `rejected` when a valid discriminating experiment contradicts the preregistered hypothesis.
- Use `inconclusive` when the gate lacks power.
- Use `invalid` when execution or design prevents inference.

State conditions and scope with every strong verdict.

## Bottleneck routing

| Diagnosis | Required next action |
|---|---|
| Hypothesis rejected | Revise the model; do not tune until it passes |
| Evidence insufficient | Design a more discriminating or adequately powered gate |
| Provenance gap | Find a source or label an uncalibrated hypothesis; do not guess |
| Runner defect | Preserve invalid run, fix research code, rerun unchanged gate |
| Harness limitation | Add the minimum observable/control needed, then resume |
| Production bug | Create minimal reproduction; request or perform an authorized fix; rerun pre-fix gate |
| Architecture/spec gap | Record the missing contract and route to specification work |
| Supported result | Reproduce at required scope, then prepare promotion proposal |

When multiple diagnoses remain plausible, design the next action to distinguish them instead of choosing the most convenient story.

## Promotion boundary

Research may justify a production proposal but does not itself redefine production truth.

Before promotion, record:

- supported mechanism and scope;
- rejected alternatives;
- reproducibility evidence;
- parameter provenance;
- compatibility with crate ownership and specs;
- migration and regression risks;
- tests required in production;
- unresolved limitations.

After an authorized production change, rerun the research gate against the production path and update the narrative with the result.

