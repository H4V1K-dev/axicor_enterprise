# Night Phase Soma-Indexed Dormant Reactivation (v1.4)

This research experiment evaluates soma-level co-spiking evidence indexing (`soma_spike_buckets: HashSet<(u32, usize)>`) across 4 reactivation policies (`segment_index_baseline_v1_3`, `soma_bucket_cofire`, `soma_bucket_plus_trace`, `soma_bucket_plus_trace_plus_slot_pressure`) over a 5-cycle stimulus schedule to test whether soma-level co-spiking restores dormant memory.

## Status: DIAGNOSTIC / NEGATIVE RESULT
- **Safety Gates**: 0 Dale/dense/dup/runaway violations across all 20 evaluations.
- **Key Discovery**: Soma co-firing increases reactivation marginally (from 9 to 17 total, 2 to 6 rare), but 88.4% of candidates (19,475) still fail evidence (`react_evidence_failed`) because dormant links do not conduct signals to trigger target spikes during the day.
- **Next Step (v1.5)**: Move to Pair-History / Structural Mass Memory v1.5.

## Evaluated Policies
1. **`segment_index_baseline_v1_3`**: Segment-level indexing `(source_soma_id, flat_segment_idx, bucket)`.
2. **`soma_bucket_cofire`**: Pure source/target soma bucket co-spiking.
3. **`soma_bucket_plus_trace`**: Soma cofire OR `long_trace >= 20`.
4. **`soma_bucket_plus_trace_plus_slot_pressure`**: Soma cofire + trace + strict slot limit (`target_count < 64`).

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across 4 policies and 5 cycles.
- `scripts/generate_plots_v1_4.py`: Visualization script for 3 comparison figures.
- `reports/report_v1_4.md`: Scientific report with detailed analysis.
- `images/reactivation_comparison.png`: Cycle 5 reactivation comparison by policy.
- `images/blocker_breakdown.png`: Rejection blocker breakdown by policy.
- `images/rare_cohort_lifecycle.png`: Rare cohort active and reactivated dynamics over 5 cycles.
