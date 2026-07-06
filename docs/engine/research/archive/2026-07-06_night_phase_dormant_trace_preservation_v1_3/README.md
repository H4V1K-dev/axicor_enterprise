# Night Phase Dormant Trace Preservation (v1.3)

This research experiment evaluates 5 trace preservation, slow decay, and age hysteresis policies in the Dormant Bank (`baseline_v1_2`, `dormant_trace_floor`, `dormant_slow_decay`, `dormant_age_hysteresis`, `combined_preservation`) across a 5-cycle stimulus schedule with explicit cohort tracking, dormant trace percentiles, and reactivation blocker breakdown.

## Status: DIAGNOSTIC / RESEARCH RESULT
- **Sprouting & Safety**: STABLE (0 Dale/dense/dup/runaway violations across all policies).
- **Key Discovery**: Disuse pruning occurs after STDP decay, so dormant synapses enter dormancy with `long_trace = 0`. Segment-level indexing blocks reactivation (`react_trace_failed = 19,487`). Source-soma level co-spiking indexing is recommended for v1.4.

## Evaluated Policies
1. **`baseline_v1_2`**: Standard decay `long_trace -= long_trace >> 7`, eviction at `dormant_age > 3`.
2. **`dormant_trace_floor`**: Preserves floor (`TRACE_FLOOR = 15`) if initial dormant trace >= 20.
3. **`dormant_slow_decay`**: Slow decay (`long_trace -= long_trace >> 10`).
4. **`dormant_age_hysteresis`**: Variable grace period (`max_age = 5` if `long_trace >= 20`, else `3`).
5. **`combined_preservation`**: Slow decay + floor + hysteresis.

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 5 policies and 5 cycles.
- `scripts/generate_plots_v1_3.py`: Visualization script for 4 comparison figures.
- `reports/report_v1_3.md`: Scientific report with detailed analysis.
- `images/cycle5_reactivation_comparison.png`: Cycle 5 reactivation comparison by policy.
- `images/reactivation_blockers.png`: Reactivation blocker breakdown by policy.
- `images/dormant_trace_distribution.png`: Dormant trace P90 trajectory over 5 cycles.
- `images/dormant_lifecycle.png`: Dormant Bank population dynamics over cycles.
