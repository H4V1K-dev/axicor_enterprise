# GSOP / DA Transfer Audit

Status: `decision_complete` + **T015 production competitive LTD landed** + **L053 network differentiation SUPPORTED**  
Started: 2026-07-12  

## Research Question

Why does calibrated GSOP + DA plasticity not move task-level choice in Cue Association (2AFC)?

## Answer

| Finding | Impact |
|---|---|
| **H1 SUPPORTED** | Old Axi GSOP had no competitive LTD on inactive slots (unmatched stayed flat). |
| **H2 REJECTED** | Not LTP/LTD wash under delayed-post. |
| **H3 REJECTED** | Fatigue @ DA=50 does not kill net LTP; DA-off → net LTD. |
| **T015 DONE** | Production `apply_gsop_plasticity`: no causal hit → full `base_ltd`. Unit proof green. |
| **L053 SUPPORTED** | Post-T015 unmatched network weights depress relative to matched (unmatched net < 0). |

**C4 remains REJECTED** until a behavioral re-probe after this rule change.  
**H4 cancelled. H5** only if weights still cannot move choice after a network probe.

## Phase table

| Phase | Status | Verdict |
|---|---|---|
| H1–H3 | Completed | See above |
| H4 | Cancelled | — |
| H5 | Deferred | After network probe if needed |
| T015 competitive LTD | **Landed in production** | Unit proof green |
| L053 network weight differentiation | Completed | **SUPPORTED** |

## Next

```text
L053 differentiation is SUPPORTED. 
The next step is to run a short C4 re-run candidate (behavioral Cue Association task) to see if the restored differentiation recovers task-level learning accuracy (>= 70%).
```

No new H-ladder.

## Commands

```powershell
# from AxiEngine/
cargo test -p physics --test physics_tests test_competitive_depression_proof
cargo test -p test-harness --test lp4_task_learning_tests --features full-chain-probe,mvp-cpu-replay test_network_weight_differentiation_probe -- --nocapture
```

## Links

- [narrative.md](narrative.md)
- [reports/gsop_da_transfer_audit.md](reports/gsop_da_transfer_audit.md)
- [current_biocalibration_status.md](../../../current_biocalibration_status.md)
