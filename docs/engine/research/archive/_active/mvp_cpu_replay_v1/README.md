# MVP CPU Replay v1

Status: running
Started: 2026-07-04
Completed: N/A

## Question
Can we reproduce the legacy MVP CPU tick-loop 1:1 in an isolated environment before introducing physics modifications?

## Purpose
Establish an isolated, reproducible technical baseline for CPU tick-loop execution by porting legacy MVP CPU functions step-by-step into a test-only harness feature, verifying bit-for-bit state plane equivalence against fixtures.

## Inputs
Raw sources in `sources/`:
- `sources/old_cpu.rs` (Primary source for CPU tick-loop behavior)
- `sources/old_physics.cu` (Context only)
- `sources/old_bindings.cu` (Context only)
- `sources/older_cpu.rs` (Context only)
- `sources/older_physics.cu` (Context only)
- `sources/older_bindings.cu` (Context only)

## Method & Planned Scope

### Scope of MVP Functions to Port
The full set of MVP CPU functions planned for isolated porting:
1. `cpu_propagate_axons`
2. `cpu_apply_spike_batch`
3. `cpu_inject_inputs`
4. `cpu_record_outputs`
5. `cpu_update_neurons`
6. `cpu_apply_gsop`
7. `cpu_extract_telemetry`
8. `cpu_sort_and_prune`

> **Note on Task 1 Scope**: Task 1 covers strictly the access scaffold for `.state` SoA planes and `.axons` binary blobs. Functional logic transfer begins in Task 2.

### Edge Case & Parity Contracts
- `cpu_propagate_axons`: Implements exact 1:1 MVP parity using `chunks_exact_mut(2)`. Valid production axon head buffers must have an even length. Any trailing odd element in an odd-length slice is left unprocessed.
- `cpu_inject_inputs`: Uses a deliberate safety guard (`.get(word_idx)`) to prevent panics when `input_bitmask` is shorter than `(num_virtual_axons + 31) / 32`. Virtual axons without matching bitmask words remain unchanged.

### Step-by-Step Execution Plan
1. Organize active research directory and register status in `docs/engine/research/current_biocalibration_status.md`.
2. Prepare test-only harness location under `crates/test-harness` with feature flag `mvp-cpu-replay`.
3. Implement `.state` and `.axons` blob-compatible wrappers (`MvpStateBuffer`, `MvpAxonBuffer`) adhering to `layout` offsets, headers, and column-major matrix indexing (`slot * padded_n + tid`). [COMPLETED - Task 1]
4. Incrementally port CPU logic functions starting with simple utilities (`cpu_propagate_axons`, `cpu_apply_spike_batch`, `cpu_inject_inputs`, `cpu_record_outputs`), followed by telemetry/GSOP, and finally `cpu_update_neurons` hot loop. [COMPLETED - Task 2 simple functions]
5. Run parity tests against fixtures and generate mismatch reports if deviations occur.

## Planned Code Location
- `crates/test-harness/src/mvp_cpu_replay.rs`
- `crates/test-harness/tests/mvp_cpu_replay.rs`
- Feature flag: `mvp-cpu-replay` in `crates/test-harness/Cargo.toml`

## Planned Tests
- Layout offsets integration check against `layout::compute_state_offsets`.
- Axon `.axons` blob header (`AxonsFileHeader`) and `AXON_SENTINEL` initialization.
- Read/write access tests for `h0..h7` ring buffers within `.axons` payload.
- Dendrite slot indexing verification (`slot * padded_n + tid`).
- Step-by-step parity tests against legacy fixtures.

## Expected Result
Bitwise identical state plane outputs between the ported test-only CPU runner and legacy MVP CPU execution.

## Current Open Questions
1. How do edge cases in legacy `cpu_update_neurons` handle unaligned or non-multiple `padded_n` buffers?
2. Are there any implicit contract shifts between legacy `ShardVramPtrs` plane alignment and current `layout::StateOffsets` calculation?

## Outputs
- README: `docs/engine/research/archive/_active/mvp_cpu_replay_v1/README.md`
- Test-only harness module: `crates/test-harness/src/mvp_cpu_replay.rs`
- Integration tests: `crates/test-harness/tests/mvp_cpu_replay.rs`
