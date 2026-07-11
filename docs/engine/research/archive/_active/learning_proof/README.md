# Learning Proof

Status: active  
Started: 2026-07-11

## Question

Может ли небольшая сеть AxiEngine после периода пластичности улучшить заранее определённое внешнее поведение и сохранить улучшение при выключенной пластичности?

## Current Status

| Phase | Status | Short result |
|---|---|---|
| C0 — controllability | partial / caveats | weight freeze works |
| C1 — local causality | supported on toy | corr Δ > control Δ |
| C2 — retention | supported (weights) | frozen eval checksum hold |
| C3 — dopamine | supported on toy | DA modulates bias |
| C4 — external task | **preregistration review** | see report |
| C5 — structural | deferred | after C4 |

## Current Conclusion

C0–C3 = plastic knobs on toy, not task learning. Program verdict open until C4.

## Outputs

- [Cumulative report](reports/learning_proof.md)
- Harness: `lp0_controllability_tests`, `lp1_causality_tests`, `lp2_retention_tests`, `lp3_reward_ablations_tests`
- Generated outputs: gitignored `artifacts/`
