# Learning Proof

Status: archived (COMPLETE — C4 REJECTED IN CURRENT SCOPE)  
Started: 2026-07-11  
Completed: 2026-07-11  

## Question

Может ли небольшая сеть AxiEngine после периода пластичности улучшить заранее определённое внешнее поведение и сохранить улучшение при выключенной пластичности?

## Current Status

| Phase | Status | Short result |
|---|---|---|
| C0 — controllability | partial / caveats | weight freeze works |
| C1 — local causality | supported on toy | corr Δ > control Δ |
| C2 — retention | supported (weights) | frozen eval checksum hold |
| C3 — dopamine | supported on toy | DA modulates bias |
| C4 — external task | COMPLETE — REJECTED IN CURRENT SCOPE | Normal did not improve relative to baseline and did not beat ablations |
| C5 — structural | not entered | prerequisite C4 not satisfied |

## Current Conclusion

C0–C3 supported plastic knobs on toy. C4 (external task learning) was rejected because weight updates under frozen biocalibration parameters are too small to shift SNN choice behavior within 500 trials.


## Outputs

- [Cumulative report](reports/learning_proof.md)
- [Research narrative](narrative.md)
- Harness: `lp0_controllability_tests`, `lp1_causality_tests`, `lp2_retention_tests`, `lp3_reward_ablations_tests`
- Generated outputs: gitignored `artifacts/`
