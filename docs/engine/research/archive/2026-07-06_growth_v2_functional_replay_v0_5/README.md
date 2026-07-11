# Growth v2 Functional Topology Replay (v0.5)

This research experiment validates functional replay dynamics, signal propagation, somatic GLIF integration, and GSOP plasticity of three Growth v2 axon morphology candidates (Sparse Clean, Dense Stress, and Balanced Functional).

The replay uses the research flat-tree parent-pointer contract from v0.4 (`flat_segment_idx + parents[]`). It is not a production runtime migration. The Balanced candidate passes functional replay, but still carries a fan-in cap-pressure caveat.

## Setup & Verification

### Prerequisites
- AxiEngine workspace with Rust toolchain installed.
- Python 3.x with `numpy` and `matplotlib` inside the workspace `.venv`.

### Running the Rust Simulation Replay
Run the functional replay integration test target:
```bash
cargo test -p test-harness --features "cpu mvp-cpu-replay baker-probe" --test baker_growth_v2_replay -- --nocapture
```
This runs 10,000 ticks of static replay followed by 10,000 ticks of plastic replay for all three candidates. It exports compiled morphologies, synapse metadata, activity rates, and weight changes to:
`artifacts/growth_v2_functional_replay_plot_data.json`

### Generating Scientific Plots
Activate the workspace virtual environment and execute the plotting script:
```bash
.venv/bin/python3 docs/engine/research/archive/2026-07-06_growth_v2_functional_replay_v0_5/scripts/plot_growth_v2_functional_replay.py
```
This generates 11 scientific plots inside the `images/` directory.

## Directory Structure
- `README.md`: Setup and run instructions.
- `reports/growth_v2_functional_replay_v0_5.md`: Detailed scientific report.
- `scripts/plot_growth_v2_functional_replay.py`: Matplotlib plotting script.
- `images/`: Rendered analysis panels.
- `artifacts/`: Ignored folder containing the raw simulation JSON data.
