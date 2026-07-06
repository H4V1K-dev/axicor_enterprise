# Night Phase Age+Trace Eviction Micro-Gate (v1.6c)

This research experiment mechanically validates the `age+trace` branch of the Dormant Bank eviction state machine.

## Key Goals
- **Synthetic Initialization**: Initialize 100 dormant synapses with `dormant_age = 0` and `long_trace = 0`.
- **Target and Global Caps**: Configured high bounds so that cap-based evictions do not interfere:
  - Global cap: 500 synapses
  - Target cap: 10 synapses per target
- **Trace Decay & Age Increment**: Verify that age increments occur before eviction checking, causing eviction when `dormant_age > MAX_DORMANT_AGE` (where `MAX_DORMANT_AGE = 2`).
- **Strict Verification**: Assert that:
  - Entries remain dormant in Cycle 1 and Cycle 2.
  - All 100 entries transition to the `Dead` category in Cycle 3.
  - Final dead count equals exactly 100.
  - Eviction reason count `age_trace` equals exactly 100, and target/global evictions remain 0.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 4 cycles.
- `scripts/generate_plots_v1_6c.py`: Python script utilizing `matplotlib` to render metrics.
- `reports/report_v1_6c.md`: Scientific report detailing validation.
- `images/age_trace_eviction_gate.png`: Generated plot showing the clean transition from Dormant to Dead.
