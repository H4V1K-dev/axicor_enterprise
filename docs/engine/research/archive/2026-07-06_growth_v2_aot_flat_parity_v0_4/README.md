# Experiment 2026-07-06: Growth v2 AOT-to-Flat Runtime Compile Parity v0.4

## Overview
This experiment verifies that the rich AOT (Ahead-of-Time) topology of the continuous Growth v2 model—which contains terminal branching, detailed branch morphology, and touch-based synapses—compiles into a research flat-tree runtime segment contract (flat segment indices plus parent pointers) and propagates spike activity with 100% mathematical parity.

This is not yet a claim that the current production linear axon counter supports branching unchanged. It proves the required flat semantics and identifies the minimal extra topology needed for branched runtime execution: root segment activation and parent/child segment propagation, or an equivalent separate-stream compile policy.

We test across two configurations:
- **Clean Case**: 1.5 um dendrite capture radius, up to 2 arbors of length 2, representing sparse branch morphology.
- **Dense Stress Case**: default large dendrite radius (using baseline 10-12 um), up to 3 arbors of length 3, ensuring dense connection structures including the critical `L4_spiny -> L5_spiny` projections.

Both configurations are simulated under three deterministic spike stimulation patterns (Single Tick Burst, Staggered Wave, and Repeated Sparse Pulses) and compared tick-by-tick to verify zero missing/extra events, zero tick mismatches, and zero target/dendrite index discrepancies.

## Directory Structure
- `AxiEngine/crates/test-harness/tests/baker_growth_v2_flat_parity.rs`: Integration test simulating the growth, flat compilation, and step-by-step propagation.
- `reports/growth_v2_aot_flat_parity_v0_4.md`: The detailed scientific report.
- `scripts/plot_growth_v2_aot_flat_parity.py`: Python visualization script.
- `artifacts/`: Contains `growth_v2_aot_flat_parity_plot_data.json` containing serialized coordinates and event logs.
- `images/`:
  - `3d_clean_morphology.png`: 3D view of somas and axons showing sparse arbors.
  - `3d_dense_morphology.png`: 3D view of somas and axons showing highly branched arbors.
  - `3d_synapse_comparison.png`: 3D spatial view of synapses formed under Clean and Dense cases.
  - `3d_stimulated_somas.png`: 3D view of the deterministic stimulated soma subset.
  - `projection_heatmap.png`: Clean vs Dense source-target projection matrices.
  - `degree_histograms.png`: Fan-in and out-degree distributions.
  - `parity_error_heatmap.png`: Zero-error parity summary for all tested cases and spike patterns.
  - `clean_event_raster.png`: Spike raster plot comparing AOT Oracle vs. Flat Runtime for the Clean Case.
  - `dense_event_raster.png`: Spike raster plot comparing AOT Oracle vs. Flat Runtime for the Dense Case.
  - `pattern_1_2_event_counts.png`: Firing event count per tick comparing AOT vs. Flat for Pattern 1 and 2.
  - `pattern_3_event_counts.png`: Firing event count per tick comparing AOT vs. Flat for Pattern 3.

## Quick Start
Run the parity verification test suite:
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test baker_growth_v2_flat_parity run_growth_v2_flat_parity -- --nocapture
```

Generate plots:
```bash
.venv/bin/python3 docs/engine/research/archive/2026-07-06_growth_v2_aot_flat_parity_v0_4/scripts/plot_growth_v2_aot_flat_parity.py
```
