# GSOP / DA Transfer Audit

Status: `running`  
Started: 2026-07-12  
Owner: research / orchestrator  

## Research Question

Why does calibrated GSOP + DA plasticity not move task-level choice in Cue Association (2AFC)? Is it due to transfer/rule differences between AxiEngine and legacy, or is it a pure scale difference?

## Experiment Phase Roadmap

| Phase | Hypothesis | Status | Verdict |
|---|---|---|---|
| **H1** | AxiEngine `apply_gsop_plasticity` ≠ legacy `cpu_apply_gsop` slot update math | Completed | Supported |
| **H2** | On task-like trials, LTP and LTD largely cancel (wash) | Completed | Rejected (delayed-post fixture) |
| **H3** | Fatigue penalty dominates net_delta at cap=18 / cost=50 | Planned | - |
| **H4** | DA delivery (constant vs end-trial vs on-correct) matters more than magnitude | Planned | - |
| **H5** | Mass→charge scale alone explains no behavioral shift even if rule matches legacy | Planned | - |

## Reproducible Commands

### Phase H1 slot update parity probe test:
```powershell
cargo test -p physics --test physics_tests test_gsop_parity_probe
```

### Phase H2 wash audit test:
```powershell
cargo test -p physics --test physics_tests test_gsop_h2_event_counters_wash -- --nocapture
```

## Links

- **Living Narrative:** [narrative.md](narrative.md)
- **Main Report:** [reports/gsop_da_transfer_audit.md](reports/gsop_da_transfer_audit.md)
- **Status Roadmap:** [current_biocalibration_status.md](../../../current_biocalibration_status.md)
