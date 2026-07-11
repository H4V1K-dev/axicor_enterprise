# Night Phase Contract & MVP Extraction (v0.1)

This folder contains the architectural contract and legacy MVP audit for the Night Phase in AxiEngine.

## Contents
- `night_phase_contract_v0_1.md` — Detailed architectural design, legacy audit, and state-plane contracts.

## Key Outcomes
- **Defined Role**: Night phase is classified as offline graph/state maintenance between day batches, not tick physics.
- **In-Process Preference**: For early research validation, IPC/daemon mechanisms are postponed in favor of direct in-process Rust integration.
- **Invariants Defined**: Established strict invariants: configurable per-pair cap control, Dense Target Rule, Dale's Law, and No Artificial Labels.
- **Next Step**: Mapped out `Night Phase Passive Recovery v0.2` as the minimal next executable step.
