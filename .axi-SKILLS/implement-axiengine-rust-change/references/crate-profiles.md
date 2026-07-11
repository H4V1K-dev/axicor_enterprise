# AxiEngine crate profiles

Select the closest profile from normative responsibility and actual constraints. A crate may combine profiles; identify the dominant role and apply secondary constraints only where relevant.

## Foundational primitives and physics

Representative crates: `types`, `physics`.

- Prefer `no_std` and allocation-free behavior only when the specification requires it.
- Keep algorithms pure, deterministic, and integer-based where the domain contract requires it.
- Place constants, packed primitives, domain errors, and independent algorithms in focused modules.
- Avoid runtime orchestration, filesystem access, parsing, backend selection, and hidden global policy.
- Re-export the intended vocabulary from the crate root according to the crate's established convention.
- Prove packed widths, sentinels, domain bounds, total decoding, and mathematical edge cases.

## Binary layout and wire contracts

Representative crates: `layout`, `wire`.

- Treat representation, byte order, size, alignment, offsets, padding, magic values, and versions as public contracts.
- Keep structures POD-compatible and allocation-free when required.
- Separate physical layouts from semantic runtime behavior.
- Make padding explicit when it is part of the format.
- Use compile-time size/alignment/trait assertions and runtime offset or byte-roundtrip tests.
- Coordinate every format change with all producers, consumers, stored artifacts, and the owning specification.

## API and hardware abstraction contracts

Representative crate: `compute-api`.

- Separate traits, DTOs, opaque handles, capabilities, validation, backend identity, and unified errors.
- Keep implementation and vendor details out of the contract crate.
- Prefer safe, allocation-free contracts when specified; use `forbid(unsafe_code)` where established.
- Validate shapes, ownership, handles, capacities, and optional buffers before dispatch.
- Give unsupported optional operations explicit error behavior.
- Test the public contract through a mock or minimal implementation without coupling to a vendor backend.

## Backend implementations

Representative crates: `compute-cpu`, `compute-cuda`.

- Keep the public surface narrow: configuration and the shared backend implementation are usually enough.
- Hide resources, memory management, kernels, scheduling, and simulation machinery in private modules.
- Preserve parity with the shared API while allowing backend-specific internals.
- Localize platform and `unsafe` code; expose safe ownership and lifecycle behavior.
- Test resource ownership, invalid handles, allocation and cleanup, shared validation, and backend parity relevant to the change.
- Do not leak vendor-specific features into shared contract crates.

## Facades and runtime orchestration

Representative crates: `compute`, `runtime`, `boot`.

- Model lifecycle and invalid transitions explicitly.
- Own backend selection, resource lifetime, error translation, and coordination at the documented layer.
- Keep low-level physics, layout formulas, parsing, and vendor mechanisms in their owning crates.
- Update every state match and transition when changing lifecycle enums.
- Test success, failure, cleanup, repeated calls, overflow, and state preservation.
- Treat feature-gated backend construction and mock paths as part of the impact map.

## Domain pipelines and configuration

Representative crates: `config`, `topology`, `baker`.

- Separate input/output DTOs, validation, typed errors, deterministic stages, and the public facade when those boundaries are real.
- Keep stage internals private unless another crate has a documented reason to call them directly.
- Preserve deterministic ordering and seed derivation.
- Validate configuration as early as the owning layer permits, without duplicating deeper invariants.
- Keep serialization DTOs distinct from compiled runtime or binary layout representations when semantics differ.
- Test invalid configurations, deterministic replay, stage boundaries, and produced artifact contracts.

## Process binaries and services

Representative crates: `node`, `baker-cli`, `weaver-daemon`.

- Keep argument parsing, process startup, logging, exit mapping, and service wiring at the binary boundary.
- Move reusable domain behavior into the owning library crate.
- Preserve stable exit codes, protocol behavior, shutdown, and operational diagnostics.
- Test reusable logic below `main`; use process-level tests only for observable CLI or daemon behavior.

## Test harness and research runners

Representative crate: `test-harness`.

- Distinguish regression infrastructure from research-only probes.
- Apply `conduct-axiengine-research` to research programs, preregistration, evidence, and research-runner semantics.
- Reuse production paths for production claims; label semantic deviations explicitly.
- Keep reusable fixtures and analysis support separate from one-off gate runners.
- Do not promote research behavior into production contracts through shared helper drift.
