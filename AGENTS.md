# Project agent instructions

Project-specific agent skills live under `.axi-SKILLS/` and are versioned with the repository.

Before creating, updating, synchronizing, or reviewing a Rust crate specification under `docs/engine/spec_L*/`, read and follow the complete skill at:

`.axi-SKILLS/write-axiengine-crate-spec/SKILL.md`

Use that skill to distinguish normative architecture from current implementation, preserve ownership boundaries and invariant IDs, map requirements to tests, and keep `docs/engine/INDEX.md` synchronized. Do not treat unresolved entries in `docs/engine/review.md` as accepted decisions.

Before planning, conducting, continuing, analyzing, narrating, auditing, or archiving AxiEngine research under `docs/engine/research/`, or creating related research-only runners in `AxiEngine/crates/test-harness/`, read and follow the complete skill at:

`.axi-SKILLS/conduct-axiengine-research/SKILL.md`

Use that skill to preserve parameter provenance and preregistration, accumulate gates inside durable programs, maintain the continuous research narrative, distinguish invalid experiments from rejected hypotheses, route code and specification blockers explicitly, and synchronize research status and evidence links.

Before planning or implementing production Rust changes under `AxiEngine/crates/`, including creating crates, changing public APIs, module structure, manifests, features, backends, lifecycle behavior, validation, errors, rustdoc, or tests, read and follow the complete skill at:

`.axi-SKILLS/implement-axiengine-rust-change/SKILL.md`

Use that skill proportionally: keep local changes lightweight, expand the impact audit for contract changes, and use the full crate-profile and architecture workflow only when boundaries actually move. For research-only runners in `AxiEngine/crates/test-harness/`, the research skill governs the experiment while this skill governs authorized Rust implementation quality.

Before creating, splitting, updating, synchronizing, or reviewing executable task contracts under `artifacts/agent-tasks/inbox/`, or changing the active handoff in `artifacts/agent-tasks/QUEUE.md` and `artifacts/agent-tasks/README.md`, read and follow the complete skill at:

`.axi-SKILLS/author-axiengine-agent-task/SKILL.md`

Use that skill to turn project intent into one bounded decision contract, name authoritative sources and applicable skills, distinguish requirements from preferences, define protected stop conditions, derive acceptance from semantic risk, and keep task routing synchronized. Research tasks must agree with the research lifecycle; implementation tasks must agree with crate ownership and proportional Rust verification.
