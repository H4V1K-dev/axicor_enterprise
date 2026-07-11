# Safety and ABI boundaries

Read this file before changing `repr` types, raw pointers, allocation, FFI, shared memory, wire formats, binary artifacts, CUDA bindings, or any handwritten `unsafe`.

## Establish authority

Do not introduce handwritten `unsafe` unless the task explicitly authorizes it. If an otherwise correct implementation requires unsafe code, identify the exact operation, why safe alternatives do not satisfy the contract, and the proposed boundary before proceeding.

Treat these as contract changes unless authoritative sources prove otherwise:

- adding or changing `repr(C)`, `repr(transparent)`, packing, or alignment;
- changing field type, order, padding, size, or discriminant;
- changing magic values, versions, offsets, byte order, or serialization;
- adding `Send` or `Sync` through an unsafe implementation;
- changing allocation/deallocation layout or pointer lifetime;
- changing host/device or producer/consumer buffer formulas.

## Minimize the unsafe surface

- Keep raw operations in the smallest private module that owns the invariant.
- Expose a safe API that prevents invalid pointers, lengths, aliasing, or lifecycle use.
- Put a `// SAFETY:` comment immediately before each unsafe operation or implementation.
- State locally checkable facts: allocation provenance, size, alignment, initialization, bounds, aliasing, lifetime, thread access, and matching deallocation.
- Do not justify unsafe with broad claims such as “validated elsewhere” without naming the validation and preserved relation.
- Add `# Safety` to public unsafe functions and traits, specifying caller obligations completely.

Audit `Drop`, early errors, partial initialization, repeated cleanup, zero-length allocations, overflow, and panic paths. An unsafe block is not isolated if safe callers can violate its assumptions through ordinary API use.

## Preserve binary contracts

For every affected representation, verify as applicable:

- exact `size_of` and `align_of`;
- every contractually relevant `offset_of`;
- `Pod`/`Zeroable` or equivalent trait assumptions;
- explicit padding initialization;
- sentinel and reserved encodings;
- total safe decoding of arbitrary bytes or raw values;
- version and magic validation;
- checked size and offset arithmetic;
- producer/consumer agreement across crates and languages;
- compatibility or intentional migration of persisted fixtures.

Prefer compile-time assertions for stable sizes and traits, with runtime tests for offsets, bytes, roundtrips, invalid encodings, and interoperability.

## Review concurrency claims

Before an unsafe `Send` or `Sync` implementation, prove:

- ownership is unique or synchronization is sufficient;
- raw pointers cannot create unsynchronized aliasing;
- mutation paths respect thread access;
- destruction cannot race with use;
- referenced resources outlive all cross-thread access;
- backend or foreign APIs permit the claimed thread behavior.

Document the proof beside the implementation and test what can be observed, while recognizing that tests do not prove absence of undefined behavior.
