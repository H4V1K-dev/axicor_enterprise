# Evidence Policy

## Contents

1. Evidence dimensions
2. Claim states
3. Conflict rules
4. Writing discipline

## Evidence dimensions

Do not collapse all project artifacts into one precedence list. Evaluate each claim across distinct dimensions:

| Dimension | Primary evidence | Meaning |
|---|---|---|
| Normative intent | Approved target spec and explicit resolved decisions | What the architecture requires |
| Implementation reality | Cargo manifests, Rust source, generated bindings, executable behavior | What currently exists |
| Verification reality | Tests, compile-time assertions, fixtures, probes | What is currently proven |
| Architecture context | `INDEX.md` and direct neighbor specs | Where ownership and dependency direction belong |
| Unresolved intent | `review.md` and deferred/open sections | What must not be silently decided |

Treat dates, versions, and status labels as evidence about lifecycle, not as proof that content is correct.

## Claim states

Classify material claims before writing:

- **Implemented**: present in current code or artifacts.
- **Verified**: covered by an existing test or deterministic check.
- **Normative**: required by the governing specification even if verification is incomplete.
- **Planned**: part of an explicit future contract or stage.
- **Deferred**: intentionally excluded from the current stage.
- **Resolved**: backed by an accepted architectural decision.
- **Open**: disputed or underspecified; belongs in review debt.

A claim can hold more than one state, such as normative + implemented but unverified.

## Conflict rules

- Do not rewrite an approved contract merely because the current code differs.
- Do not describe planned behavior as implemented merely because the spec is detailed.
- Do not use a stale `INDEX.md` entry to override a newer target spec; synchronize the registry when authorized.
- Do not infer an architectural decision from whichever implementation happened to land first.
- Do not retain a resolved issue only in `review.md`; propagate the resolution to owning specs and verification requirements.
- Do not duplicate a foreign constant or policy. Name the owner and specify the local dependency on it.

When intent remains unclear, preserve the discrepancy and formulate a `REV-<DOMAIN>-<NNN>` question with source, impact, affected specs, and acceptance criteria.

## Writing discipline

Use exact names for crates, features, Rust items, files, formats, and tests. Prefer testable statements over aspirations.

Weak:

> Крейт должен быть быстрым и безопасным.

Contractual:

> `compute-cpu` не выполняет динамические аллокации внутри потикового цикла и возвращает `ComputeApiError` для невалидного `VramHandle` без паники.

Use strong modality only when the evidence supports it. Otherwise use an explicit status marker such as `Planned`, `Deferred`, or `Open`.

