# Acceptance and handoff

## Derive acceptance from risk

For every material risk, name the proof:

| Risk | Suitable proof |
|---|---|
| Wrong local algorithm | Focused unit test with boundary and failure cases |
| Public contract drift | Integration test plus consumer/call-site audit |
| ABI or wire drift | Size, alignment, offsets, bytes, version, producer/consumer checks |
| Feature break | Exact affected feature combinations and targets |
| Lifecycle error | Valid, invalid, repeated, failure, and cleanup transitions |
| Research fixture error | Sanity assertions, control-flow mapping, durable raw measurements |
| Overclaim | Explicit claim boundary and unresolved alternatives |
| Status drift | Atomic README/report/index/task/queue consistency check |

Acceptance should fail when the promised outcome is absent. A cargo command with no relevant assertion is execution evidence, not outcome evidence.

## Write exact commands

Specify working directory, package, features, target, named test, and important runner flags. Allow adaptation only when the task explains how the executor determines the replacement.

Do not filter cargo output through a pipeline that masks its exit code. Request concise summaries rather than complete logs.

## Separate mandatory and optional work

Use mandatory acceptance for the decision boundary. Put refactors, polish, broader sweeps, and future integrations into non-blocking notes or later tasks. Never let optional work silently become necessary for a DONE verdict.

## Define completion semantics

- Implementation: required behavior exists and the proportional verification matrix passes.
- Research: the preregistered gate has an honest supported/weakened/rejected/inconclusive/invalid verdict; a negative result may be valid completion.
- Review: findings are evidenced and scoped; fixes are not implied.
- Design: the decision is accepted or the remaining blocker is precisely stated.
- Specification: normative artifacts and registries agree, with unresolved debt preserved.

## Require a bounded handoff

The executor reports:

- outcome and current status;
- changed files or reviewed sources;
- exact commands and concise results;
- evidence-backed verdict;
- caveats, baseline debt, and untouched scope;
- new authority or follow-up required;
- claims not established by the work.

The handoff must be sufficient for a reviewer to reproduce the acceptance decision without reconstructing the entire task history.
