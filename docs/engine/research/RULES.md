# Research Folder Rules

Status: active rules for AxiEngine biological calibration research.

These rules exist to keep experiments reproducible after a year, not to produce paperwork.

## 1. Directory Contract

The research root must stay small:

```text
docs/engine/research/
  RULES.md
  current_biocalibration_status.md
  archive/
```

No loose experiment reports, scripts, CSV, JSON, PNG, temporary notes, or raw outputs should live directly in `docs/engine/research/`.

## 2. Main Index First

Before starting a concrete research experiment, register it in:

```text
docs/engine/research/current_biocalibration_status.md
```

The entry should state:

- the research question;
- why the experiment matters;
- expected behavior;
- what would confirm the hypothesis;
- what would weaken or reject the hypothesis;
- planned outputs;
- planned command or runner, if known.

If the experiment is exploratory, say so explicitly. Do not pretend the goal is sharper than it is.

## 3. Active Experiment Folder

All active work for a concrete experiment goes into:

```text
docs/engine/research/archive/_active/<experiment_slug>/
```

Use lowercase ASCII slugs:

```text
full_neuron_replay_314900022
ephys_probe_01_replay
dds_discharge_probe
```

Recommended structure:

```text
archive/_active/<experiment_slug>/
  README.md
  scripts/
  images/
```

This mirrors the existing archived research layout. Only create folders that are actually needed.
Create extra folders such as `reports/` or `notes/` only when the README becomes too large or the temporary notes are worth preserving.

## 4. What Goes Where

### README.md

The active `README.md` is the living lab note. It should contain:

- status: `planned`, `running`, `finished`, `superseded`, or `abandoned`;
- start date;
- owner/runner if useful;
- purpose;
- inputs;
- commands;
- expected result;
- observed result;
- final conclusion once finished.

### scripts/

Put executable research scripts here when they are specific to this experiment.

If a script is intentionally reusable and remains under `tools/research/`, archive a copy or record the exact source path and revision context in the experiment README.

### images/

Put committed plots and compact visual summaries here.

Images referenced by the main index or the experiment README should live in the experiment archive, not only in a temporary output location.

### Generated artifacts

Generated CSV/JSON/traces should not be loose inside `docs/engine/research/`.

Default location:

```text
artifacts/<experiment_slug>_*.csv
artifacts/<experiment_slug>_*.json
artifacts/<experiment_slug>_*.trace
```

or another clearly named path under the repository-level `artifacts/` directory.

Commit policy:

- commit Markdown reports;
- commit research scripts;
- commit compact images used by reports;
- do not commit bulky generated CSV/JSON/traces by default;
- commit compact generated tables only when they are intentionally promoted to durable evidence.

Large raw caches may stay outside the docs tree, but the README must point to them clearly.

## 5. Rust Tests and Production Code

Do not move Rust tests into the research archive.

If a Rust test, ignored test, benchmark, or harness was used, record in README:

- file path;
- test name;
- exact command;
- feature flags;
- whether it was temporary or should remain in the codebase.

Example:

```text
Rust runner:
- file: crates/test-harness/tests/legacy_baseline.rs
- test: test_legacy_representative_traces
- command: cargo test -p test-harness --features legacy-baseline --test legacy_baseline test_legacy_representative_traces -- --ignored --nocapture
```

### Research variants of production functions

Do not patch production physics just to test a research hypothesis.

If an experiment needs a modified production function, create an explicit research variant instead:

- copy the smallest necessary function or wrapper into the research runner/test harness;
- prefix the name with the experiment slug, for example `full_neuron_replay_314900022_update_glif_voltage`;
- record the original production path and function name in the experiment README;
- record the exact semantic difference from production;
- keep the variant local to the experiment until the result justifies a real production proposal.

Preferred Rust locations:

- one-off runner: `crates/test-harness/tests/<experiment_slug>.rs`;
- reusable helper for one research track: `crates/test-harness/src/research/<experiment_slug>.rs`, feature-gated if needed.

The research variant must not silently masquerade as production behavior. Every report must say whether a result came from unmodified production CPU or from a named research variant.

## 6. Completion / Archiving

When the experiment is complete, rename:

```text
archive/_active/<experiment_slug>/
```

to:

```text
archive/YYYY-MM-DD_<experiment_slug>/
```

Use the completion date, not the start date.

Then update the experiment README:

- final status;
- final outputs;
- short result summary;
- what was confirmed;
- what was weakened/rejected;
- what should happen next;
- links to important artifacts.

The README should be readable without opening every CSV.

## 7. Main Index After Completion

After archiving, update:

```text
current_biocalibration_status.md
```

Add or update:

- the short result;
- whether the hypothesis is alive, weakened, rejected, or needs a follow-up;
- link to the dated archive folder;
- link to one or two key visual/data artifacts.

The main status file is a map, not a full report.
Keep details in the experiment folder.

## 8. Link Rules

Use relative links inside Markdown.

After moving or archiving folders, verify links to:

- README files;
- scripts;
- images;
- CSV/JSON artifacts;
- external raw cache locations if referenced.

Images used in the main status file should be copied into the experiment archive, usually:

```text
archive/YYYY-MM-DD_<experiment_slug>/images/
```

Do not rely only on a global `artifacts/` path for key figures.

## 9. Report Discipline

Do not create a new top-level report for every small correction.

Small fixes should update:

- the active experiment README;
- the relevant archived README;
- or the main status file if the conclusion changes.

Create a separate report only when it has durable scientific value.

## 10. Evidence Levels

Use careful wording:

- `observed`: happened in one run;
- `reproduced`: happened in repeated runs or independent scripts;
- `supported`: evidence points toward it;
- `confirmed`: strong evidence and no known contradiction in the current scope;
- `rejected`: evidence contradicts it under the stated conditions;
- `deferred`: not tested yet.

Avoid saying a mechanism is "solved" until it passes the full relevant loop.

For neuron physics, a membrane-only probe does not confirm full-neuron behavior.

## 11. Minimum README Template

```markdown
# <Experiment Name>

Status: planned | running | finished | superseded | abandoned
Started: YYYY-MM-DD
Completed: YYYY-MM-DD or N/A

## Question

## Expectation

## Inputs

## Method

## Commands

## Outputs

## Result

## Interpretation

## Next Step
```

## 12. Cleanliness Checklist

Before calling an experiment finished:

- no loose experiment files remain in `docs/engine/research/`;
- experiment folder has a dated archive name;
- README has a useful summary;
- scripts are archived or clearly referenced;
- key images and compact CSV/JSON outputs are archived or linked;
- Rust test names and commands are recorded if used;
- main status file links to the archive and states the result;
- links were checked after the final move.
