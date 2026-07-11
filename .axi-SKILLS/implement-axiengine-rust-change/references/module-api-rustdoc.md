# Module, API, and rustdoc method

## Choose ownership before files

Answer these questions before adding a module or public item:

1. Which crate exclusively owns the concept or invariant?
2. Is the new item a data contract, algorithm, validator, facade, backend mechanism, or process concern?
3. Who must call it directly?
4. Is its representation observable, or only its behavior?
5. Does exposing it prevent the owning crate from changing internals later?

Place the item at the lowest layer that owns the meaning, not the first crate that needs it. Avoid dependency inversion through convenience re-exports or duplicated DTOs.

## Shape modules by semantic cohesion

- Keep `lib.rs` focused on crate identity, crate-wide attributes, module declarations, and intentional re-exports.
- Create a module when a concept has independent invariants or a meaningful private implementation boundary.
- Keep a small cohesive implementation in its existing module instead of creating one-file taxonomy.
- Separate DTOs from execution only when the DTOs form a contract shared across implementations or consumers.
- Separate validation when it is reusable boundary logic; keep a private precondition beside the algorithm when it has no independent contract.
- Separate error types when they form the crate-wide error vocabulary.
- Prefer private modules plus selected re-exports for implementation crates. Preserve public modules when module paths are already part of the contract.

Do not restructure neighboring code solely to make a new change look symmetrical.

## Design public API deliberately

- Expose behavior and domain vocabulary, not internal storage or scheduling.
- Use constructors or validation when raw public fields would permit states the contract forbids. Preserve existing DTO openness when callers genuinely need struct literals.
- Use typed enums for closed semantic alternatives and typed errors for recoverable failures.
- Preserve total, panic-free decoding for untrusted or raw representations when possible.
- Use checked conversions and arithmetic at external or format boundaries.
- Keep backend-specific types behind shared traits or facade boundaries unless vendor control is itself public.
- Add derives only when their semantics are correct and useful; for example, `Eq` is not decoration and `Clone` may duplicate ownership expectations.
- Treat re-export paths, error variants, trait defaults, and feature availability as observable API.

## Write rustdoc as contract

Write rustdoc while shaping the item. Cover only applicable dimensions:

- purpose and owning layer;
- valid ranges, units, coordinate system, ordering, alignment, and capacity;
- sentinel, zero, empty, tombstone, reserved, or invalid encodings;
- ownership, borrowing, mutation, buffer length, and lifetime expectations;
- determinism, seeding, state preconditions, and side effects;
- compatibility, format version, and platform assumptions;
- error conditions and whether failure leaves state unchanged;
- panic conditions that callers can reach;
- caller obligations for an unsafe contract;
- a compiling example when it teaches correct use better than prose.

Use standard sections precisely:

- `# Errors` for each meaningful error family and triggering condition;
- `# Panics` for reachable panic conditions, including debug-only assertions when material;
- `# Safety` for every public `unsafe fn` or `unsafe trait`;
- `# Examples` only when the example is stable, useful, and testable.

Avoid documentation that merely expands the identifier into a sentence. Prefer “Aligned soma allocation count; non-zero and divisible by 64” over “The padded N value.”

## Keep documentation synchronized

When behavior changes, search for the public name and old semantic phrase across:

- item and module rustdoc;
- crate-level rustdoc;
- examples and doctests;
- error messages;
- crate specification and invariant mappings;
- downstream comments that state the old precondition.

Do not silently repair unrelated stale documentation outside scope. Report it as nearby debt unless leaving it unchanged would make the changed contract self-contradictory.
