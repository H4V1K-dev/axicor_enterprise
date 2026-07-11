# Proportional verification matrix

Select rows that match the semantic impact. Exact commands depend on package features and targets; derive them from manifests and the task instead of copying examples blindly.

| Change | Required evidence | Additional impact search |
|---|---|---|
| Private pure algorithm | Focused unit tests; package check/test/clippy; fmt | Direct callers and boundary cases |
| Public function or type | Integration or public-path test; rustdoc review; package checks | Imports, re-exports, examples, downstream users |
| Struct field | Constructor/validation tests | Every struct literal, default, serializer, feature-gated initializer |
| Enum variant or error | Success/failure test | Every match, conversion, display/source mapping, protocol mapping |
| Trait method | Contract test through representative implementation | Every impl, mock, object-safe use, default behavior, backend parity |
| Feature or optional dependency | Check/test/clippy for affected feature combinations | `cfg` call sites, default feature behavior, downstream feature forwarding |
| `no_std` contract | No-default-features build/check and relevant tests | Accidental `std`/allocation imports and feature unification |
| ABI/layout/wire type | Size, alignment, offsets, POD traits, byte roundtrip/version tests | All readers/writers, persisted fixtures, FFI/shared-memory users |
| Unsafe or raw memory | Focused safety invariant tests plus normal package checks | Allocation/deallocation symmetry, aliasing, bounds, Send/Sync claims |
| Lifecycle/state machine | Transition table tests including invalid and repeated transitions | Exhaustive matches, cleanup, failure atomicity, restart/shutdown paths |
| Backend behavior | Shared contract tests and targeted backend tests | CPU/CUDA/mock parity, capabilities, unsupported behavior |
| Cross-crate pipeline | Targeted integration/E2E at the owning boundary | DTO conversion, error propagation, ordering, determinism, artifacts |
| CLI/service behavior | Library tests plus targeted process test when observable | Exit codes, diagnostics, shutdown, protocol compatibility |

## Command discipline

Run commands from `AxiEngine` unless the project task establishes another root. Prefer package-scoped commands such as:

```powershell
cargo fmt --all -- --check
cargo check -p <package> --features <affected-features>
cargo test -p <package> --features <affected-features> --test <target>
cargo clippy -p <package> --features <affected-features> --all-targets -- -D warnings
```

Adapt flags to the actual manifest. For `no_std`, verify the intended no-default-features posture. For feature-gated consumers, include the combinations specified by `GEMINI.md` and the task.

Do not pipe cargo verification through a filtering command that can mask its exit code. Capture verbose logs only for failures or when the task requires preserved evidence.

## Proof boundaries

State the narrowest valid conclusion:

- a unit test proves the tested algorithm and inputs, not backend parity;
- a mock proves facade behavior, not hardware execution;
- a CPU pass does not prove CUDA correctness;
- compilation proves type and feature coherence, not runtime semantics;
- deterministic replay for selected seeds does not prove all inputs;
- a benchmark observation is not a performance guarantee;
- green research assertions do not automatically prove a production or biological claim.
