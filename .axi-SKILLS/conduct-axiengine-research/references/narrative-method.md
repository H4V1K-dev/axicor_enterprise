# Research Narrative Method

## Contents

1. Artifact boundaries
2. Two narrative modes
3. Living manuscript workflow
4. Retrospective reconstruction
5. Scientific article anatomy
6. Prose, formulas, and figures
7. Historical integrity
8. Article-quality review

## Artifact boundaries

Keep four responsibilities distinct:

| Artifact | Primary question |
|---|---|
| `README.md` | Where is the program now, how is it run, and what happens next? |
| `narrative.md` | How and why did understanding change across the research journey? |
| Evidence report or ledger | What exact protocols, measurements, controls, and verdicts support the claims? |
| Program monospec or task contract | What are the gates, allowed transitions, and decision rules? |

Link instead of duplicating. Do not paste the evidence report into the narrative or turn the narrative into an expanded README.

## Two narrative modes

### Living manuscript (default)

Create the manuscript before the first experimental run. Write it alongside the research so motivation, surprise, uncertainty, and model revision are captured while they are known.

### Retrospective reconstruction (fallback)

Use only when the program predates the living manuscript. Label the document near the top:

```text
Mode: retrospective reconstruction from preserved evidence.
Undocumented historical motives are marked as unknown or reconstructed.
```

Reconstruction can produce a coherent scientific synthesis, but it cannot recover missing contemporaneous thought. Never present inferred rationale as a documented historical fact.

## Living manuscript workflow

At program registration, write:

- the project-level problem and why its answer matters;
- the initial model and established evidence;
- unresolved tensions and competing explanations;
- why the first gate is the next rational experiment;
- the preregistered expectation and links to the formal protocol.

Before each later gate, add a prose bridge from the previous result:

```text
The previous gate established X but left Y unresolved.
Because explanations A and B predict different behavior under Z,
the next experiment tests Z while holding the baseline fixed.
```

After each gate, write before opening another:

- what the system actually did;
- which expectation was met or contradicted;
- which explanations were removed or remained alive;
- how the working model changed;
- why the next research or engineering action follows.

Keep the original preregistration in the evidence layer. Revise the manuscript for clarity, but preserve wrong expectations and add retrospective corrections rather than erasing them.

## Retrospective reconstruction

Build a private chronology before drafting prose. Inspect:

- program and gate task specifications, including their revision history;
- git log and diffs for relevant tests, runners, configs, and reports;
- file timestamps only as supporting evidence, never as the sole chronology;
- preregistrations, commands, raw outputs, report tables, and archived figures;
- code or harness detours that changed what could be measured;
- decision records and status-map transitions.

For each transition, classify the rationale as:

- **documented**: explicitly preserved before or during the work;
- **supported reconstruction**: follows from multiple preserved sources;
- **unknown**: not recoverable without invention.

Do not expose the private chronology as a repetitive QA ledger. Use it to write a connected argument and link the evidence behind each material claim.

## Scientific article anatomy

Adapt section names to the program, but preserve this argumentative shape:

1. **Title and abstract**: state the problem, approach, decisive result, and bounded consequence. Update the abstract last.
2. **Introduction**: establish the project-level uncertainty and why available evidence was insufficient.
3. **Initial model and predictions**: explain mechanisms, assumptions, scales, and competing explanations.
4. **Experimental journey**: develop the argument through causal turns, not one identical template per gate.
5. **Decisive results**: integrate the measurements that changed interpretation; link full evidence tables.
6. **Discussion**: explain the revised model, alternatives not separated, and limits of inference.
7. **Engineering and research consequences**: locate the bottleneck and route the next action.
8. **Open horizon**: identify the smallest experiment or code/spec change that reduces uncertainty most.

Gate names may appear as subheadings when they mark genuine changes in the argument. Do not make every gate an identical card.

## Prose, formulas, and figures

Write the article body primarily as paragraphs. Use bullets for compact enumerations and tables for exact comparisons, never as a substitute for causal explanation.

Place a formula where it establishes an expectation or reveals a mismatch. Define variables, domains, units, assumptions, and what the calculation cannot prove.

Place a figure where it changes interpretation. Give each durable figure:

- the question it answers;
- axes, units, conditions, seeds, and aggregation;
- the visible result;
- the inference allowed;
- the limitation or alternative explanation.

Introduce every formula, table, and figure in prose and interpret it afterward. Do not use a gallery of unexplained plots. Do not claim that theoretical membrane reachability proves behavioral learning.

## Historical integrity

Preserve preregistered expectations even when wrong. Distinguish contemporaneous belief from present interpretation.

When later evidence corrects an earlier statement, add a clear retrospective passage:

```text
Retrospective correction: the earlier calculation established X,
but later evidence showed that it did not establish Y because...
```

Include failed and null results when they changed the path. Do not remove detours that explain why the final experiment became possible.

## Article-quality review

Do not call the manuscript complete until all answers are yes:

- If gate headings and labels are removed, does a coherent argument remain?
- Does each experiment arise from an unresolved consequence of the previous one?
- Does the text show the working model changing, not merely the status changing?
- Are observation, interpretation, and decision distinguishable without QA labels?
- Are material claims linked to exact evidence?
- Are formulas and figures part of the argument rather than decoration?
- Are failed expectations and unknown historical motives treated honestly?
- Does the discussion preserve competing explanations not separated by the experiment?
- Does the open horizon follow from the evidence and respect program gate rules?
- Is the narrative mostly connected prose rather than bullets, tables, or repeated cards?

If the answer to the first three questions is no, the document is a summary outline, not a scientific narrative.

