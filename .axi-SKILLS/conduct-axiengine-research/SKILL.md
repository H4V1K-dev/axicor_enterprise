---
name: conduct-axiengine-research
description: Plan, conduct, continue, analyze, narrate, audit, and close cumulative AxiEngine research programs. Use for biological calibration, neuron or microcircuit experiments, learning proofs, growth and night-phase studies, parameter sweeps, ablations, simulation probes, research-only Rust runners, evidence synthesis, research narratives, failed or null results, resuming work after code changes, deciding whether a result requires more research or production work, archiving completed research, and promoting supported findings into code or specifications. Apply to docs/engine/research, related artifacts, tools/research, and AxiEngine test-harness research code.
---

# Conduct AxiEngine Research

Produce a change in understanding, not merely a collection of outputs. Run two coupled loops throughout the work:

- **Evidence loop**: question -> preregistration -> experiment -> observations -> verdict.
- **Meaning loop**: prior model -> tension -> encounter with evidence -> revised model -> next discriminating action.

Preserve both loops in durable project artifacts. Treat failed and null outcomes as evidence.

## Resolve the research system

Locate the repository root, then read the complete `docs/engine/research/RULES.md` before research actions. Use these paths as the operating system:

- `docs/engine/research/current_biocalibration_status.md`: concise program map and current frontier.
- `docs/engine/research/archive/_active/<program_slug>/`: active cumulative research programs.
- `docs/engine/research/archive/YYYY-MM-DD_<slug>/`: completed decision-boundary archives.
- `AxiEngine/crates/test-harness/`: Rust research runners and reusable harness support.
- `tools/research/`: intentionally reusable research tools.
- `artifacts/`: generated traces, tables, and machine outputs.
- `docs/engine/spec_L*/` and production source: governing contracts and implementation evidence.

Read [references/research-lifecycle.md](references/research-lifecycle.md) before creating folders or changing program state. Read [references/narrative-method.md](references/narrative-method.md) before creating or updating a cumulative narrative. Read [references/evidence-and-diagnosis.md](references/evidence-and-diagnosis.md) before preregistering, interpreting, or routing a blocker.

## Select the operating mode

Choose the smallest mode that completes the request:

1. **Orient**: reconstruct the current question, evidence chain, open tensions, and next gate without mutation.
2. **Start**: register a program or study, establish provenance and baseline, create the living manuscript before the first run, write preregistration, and prepare the runner.
3. **Execute**: write the causal bridge into the next gate, run the preregistered matrix, preserve commands and outputs, and update the manuscript before opening another gate.
4. **Analyze**: separate observations from interpretation, apply gates, diagnose failure, and update the knowledge state.
5. **Continue**: extend the same program with the next discriminating gate instead of creating a new folder.
6. **Resume after code work**: preserve pre-fix evidence, record the code boundary, rerun the unchanged gate, and distinguish pre-fix from post-fix evidence.
7. **Narrate**: continue a living manuscript, or reconstruct a completed history in explicitly labeled retrospective mode without inventing missing rationale.
8. **Close or archive**: finalize verdict and narrative, synchronize the map, validate links, and move the program at a real decision boundary.
9. **Promote**: prepare a supported finding for production code or specification work. Do not silently turn research behavior into production behavior.

Review first when the user asks only for an explanation, diagnosis, or status. Modify research artifacts and research-only runners when the request includes conducting or continuing research. Modify production code only when the user also authorizes implementation.

## Choose the right research unit

Continue an existing program unless the primary question becomes independently meaningful.

- **Program**: a durable line of inquiry such as Learning Proof, Growth v2, or Night Phase.
- **Study or gate**: a falsifiable step inside the program, such as C4 task learning.
- **Run**: one seed, condition, ablation, horizon, or sweep candidate.
- **Decision boundary**: a durable verdict that changes project direction or creates an independently reproducible package.

Create a new active folder for a program or independent decision package, not for every version, seed, correction, or parameter sweep. Record small iterations in the program's evidence ledger and cumulative narrative.

## Reconstruct context before acting

Read the smallest complete evidence set:

1. Read the program README, narrative, relevant study material, and current status entry.
2. Follow links to prior archives that establish parameters, baselines, winners, or rejected paths.
3. Inspect exact runner code, production functions, profile TOML, specs, tests, and generated schemas used by the experiment.
4. Search for every numeric parameter and record its provenance and domain.
5. Identify what is implemented, what is a research variant, and what is only hypothesized.
6. State the current model, known contradictions, and the precise knowledge gap.
7. For retrospective reconstruction, inspect chronological evidence: task revisions, git history and diffs, runner/test evolution, report timestamps, archived preregistrations, and decision records.

Do not infer missing rationale from a polished final result. Mark unknown historical reasoning as unknown.

## Preregister the gate

Before observing target results, record:

- research question and why it changes a project decision;
- prior model and competing explanations;
- baseline and parameter provenance;
- independent and dependent variables;
- controls, ablations, seeds, scales, and horizons;
- primary metrics and physiological or architectural sanity gates;
- success, weakening, and rejection criteria;
- exact runner, command, feature flags, and planned outputs;
- known limitations and what the gate cannot establish.

Mark exploratory work explicitly. Do not convert an exploratory observation into confirmation without a later preregistered gate. Never weaken thresholds after seeing results.

## Implement and execute safely

Prefer unmodified production paths when testing production claims. If a hypothesis requires altered behavior, create the smallest explicit research variant in the test harness, name it after the program or gate, and document the semantic delta from production.

Record exact commands, revisions when available, random seeds, feature flags, input datasets, units, and artifact locations. Preserve negative conditions and partial runs when they affect interpretation. Do not patch production physics to make a gate pass.

If an execution defect invalidates the experiment, label the run invalid rather than negative. Correct the experiment and rerun the same preregistered gate without rewriting its original expectation.

## Analyze and diagnose

Separate three layers:

1. **Observed**: direct measurements, traces, tables, plots, and failures.
2. **Interpreted**: mechanisms supported or weakened by those observations.
3. **Decided**: the gate verdict and the resulting project action.

Compare every primary condition with its baseline and controls. Report effect sizes and units, not only PASS/FAIL. Apply the evidence vocabulary from `RULES.md` conservatively.

Classify the next bottleneck as one of:

- model or hypothesis rejected;
- evidence insufficient or underpowered;
- parameter provenance gap;
- experiment or runner defect;
- test-harness limitation;
- production implementation bug;
- architecture or specification gap;
- supported result ready for promotion.

Use `references/evidence-and-diagnosis.md` to route the next action. Do not use more experimentation to hide a code defect, and do not use a code change to rescue a rejected hypothesis.

## Maintain the research narrative

For a multi-gate program, create `narrative.md` before the first experimental run and maintain it as a living scientific manuscript. Let `README.md` remain a compact operational map and let reports retain exact protocols, metrics, tables, and verdict records. A short one-gate study may keep its narrative in README while that remains readable.

Before each meaningful gate, write why the prior result created the next question and why the chosen experiment can distinguish the live explanations. After the gate, write the encounter with evidence, the model revision, and the consequence before starting the next gate. Include failed gates and code detours.

Write the manuscript body as connected explanatory prose. Do not use repeated cards such as `prior belief / experimental choice / observation / verdict` as the main narrative; keep that structure in the evidence ledger. Tables and bullets may support the argument but must not replace it. Link material claims to the exact report, runner, figure, or decision record.

When the program predates the skill, label the manuscript as a retrospective reconstruction. Rebuild chronology from preserved evidence and mark undocumented motives as unknown or reconstructed. Do not imitate contemporaneous narration that did not exist.

Preserve the dominant language of the program unless the user chooses another. Rules requiring English code and code comments do not automatically determine the language of Markdown research prose.

Do not rewrite history to make the path appear inevitable. Add retrospective corrections while preserving original expectations. Follow `references/narrative-method.md` and apply its article-quality review before closure.

## Handle code detours and return

When research exposes a code or architecture problem:

1. Freeze and document the evidence that revealed it.
2. Reduce it to the smallest reproducible case.
3. Classify whether the gate is invalid, blocked, or still a valid negative result.
4. Record the required code or specification action in README and narrative.
5. Stop before production mutation unless implementation is authorized.
6. After the change, record its revision and semantic effect.
7. Rerun the original gate unchanged when still valid.
8. If the change alters the hypothesis, baseline, or acceptance semantics, preregister a new gate.
9. Return to the cumulative narrative and explain how the detour changed the research model.

## Close atomically

At a decision boundary, update the program as one transaction:

- README status, dates, commands, outputs, verdict, and next action;
- cumulative narrative and current synthesis;
- evidence tables, reports, and durable figures;
- `current_biocalibration_status.md` summary, evidence level, and links;
- archive folder name when the program is truly complete;
- references from dependent research, specs, or proposals.

Do not archive a multi-gate program while its narrative is only a gate summary or QA outline. Do not leave a terminal program under `_active`, a report saying `planned` after execution, or status entries pointing at obsolete folders.

## Validate and hand off

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .axi-SKILLS/conduct-axiengine-research/scripts/check-research-consistency.ps1
```

Then report:

- the question and gate actually tested;
- the knowledge transition: before -> evidence -> after;
- the verdict and confidence level;
- the diagnosed bottleneck;
- whether the next action is research, harness work, production code, specification work, or archive;
- files and artifacts created or updated;
- commands and validations run;
- unresolved contradictions or missing provenance.
