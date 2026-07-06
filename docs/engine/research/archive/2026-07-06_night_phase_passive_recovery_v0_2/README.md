# Night Phase Passive Recovery (v0.2) Research Archive

This archive contains the documentation and design details for the Night Phase Passive Recovery (v0.2) research audit.

## Purpose
The purpose of this audit was to verify a minimal day/night cycle on the Growth v2 C17 topology winner (`Radius_9_Cap_96_Pair_2_ProjAware`), testing whether passive recovery (membrane, threshold, and fatigue relaxation) and synaptic weight decay preserve functional learning matched-bias without causing silence or runaway collapses.

## Contents
- `night_phase_passive_recovery_v0_2.md` — The detailed research report answering the core questions and presenting the results of the 3 tested policies.

## Key Outcomes
- **Physiological Excitability Restored**: Resetting threshold offsets and voltages homeostatically during the night restored network excitability, reducing Day 2 silence ticks from **2,623 to 2,036** ticks.
- **Matched-Bias Retained**: Passive recovery preserves learning matched-bias perfectly (retention ratio = **1.0000**).
- **Safe Weight Decay**: A 0.1% sign-preserving synaptic weight decay is dynamically stable, preserves Dale's Law (0 violations), and maintains high matched-bias retention (ratio = **0.9990**).
- **Next Step**: The next research phase is `Night Phase Weight Maintenance / Prune-Compact v0.3`.
