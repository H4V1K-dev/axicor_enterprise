# Documentation Rules

> Constitution of the AxiCAD design documentation system.
> Every spec, domain model, and vision document must follow these rules.

---

## 1. One Module = One File

Each logical module, domain entity, or subsystem gets its own dedicated file.
No mega-documents. If a spec grows beyond ~300 lines, it must be decomposed
into sub-modules, each with its own file.

## 2. No Duplication

If information belongs to another module — link to it, don't copy it.
The glossary (`GLOSSARY.md`) is the single source of truth for terminology.
Data structures are defined in exactly one spec; others reference them.

## 3. Status is Mandatory

Every spec and domain doc must have a `Status` field at the top:

| Status | Meaning |
|--------|---------|
| `Draft` | Initial sketch, not reviewed, may change drastically |
| `Review` | Ready for discussion, structure is stable |
| `Stable` | Approved and implemented, changes require ADR |
| `Deprecated` | Superseded or removed, kept for history |

## 4. Dependencies are Explicit

The `Dependencies` section is a directed graph. It must be acyclic (DAG).
Each dependency states *what* is used from the target module, not just a link.

Example:
```
- [coordinate-system](../domain/coordinate-system.md) — GridPosition type, voxel-to-world transform
```

## 5. Contract First

Design the contract (inputs, outputs, invariants) before describing behavior.
A spec with only sections 1–3 filled is already useful. A spec with only
section 5 filled and no contract is not.

## 6. Changelog in Every Spec

Not a git log. A human-readable history of *significant* changes.
Trivial typo fixes don't need entries. Structural or contract changes do.

## 7. ADR for "Why"

Any non-obvious architectural decision gets recorded as an ADR in `decisions/`.
Format: Context → Options → Decision → Consequences.
ADRs are append-only: once written, they are never deleted (only superseded).

## 8. INDEX.md Stays Current

When adding or removing a spec — update `INDEX.md` immediately.
The index is the project map. A stale index is worse than no index.

## 9. File Naming

- All filenames: `kebab-case.md` (lowercase, hyphens, no spaces)
- Template files: prefixed with `_template-` (e.g., `_template-spec.md`)
- ADR files: prefixed with sequential number (e.g., `001-decision-name.md`)

## 10. Language

- Document body: **English** (code comments, type names, contracts)
- Discussion notes and Open Questions: English or Russian — whatever is clearer
- Glossary terms: English with Russian explanation where helpful

## 11. Cross-References

Use relative markdown links between docs:
```markdown
See [Socket Model](../domain/socket.md) for socket geometry details.
```

Never use absolute paths. Never duplicate content — always link.

## 12. Templates

All new documents must be created from the appropriate template:
- Domain entities → `_template-domain.md`
- Technical specs → `_template-spec.md`
- ADRs → `_template-adr.md`

Templates live in the `docs/` root alongside this file.
