# Night Phase Full Lifecycle (v1.2)

This research experiment evaluates the multi-stage night-phase lifecycle (`Day learning -> pruning -> Dormant Bank -> indexed reactivation -> stochastic sprouting -> next day replay`) across 5 day/night cycles under a varied stimulus schedule with explicit cohort tracking, real dormant eviction, and reactivation blocker breakdown.

## Status: PARTIAL / DIAGNOSTIC RESULT
- **Sprouting Churn**: STABLE (after initial pruning, sprouting maintains the active population in the ~3.6k-5.6k range, with full-lifecycle Gini bounded at ~0.39-0.46).
- **Dormant Reactivation**: NOT YET VALIDATED (long trace decay during absent cycles limits dormant reactivation).

## Objective & Stimulus Schedule
- **Topology**: Baker C17 (`post_prune_headroom`) initialized with 22,037 synapses.
- **Cycles 1–2**: Context A + B active.
- **Cycles 3–4**: Context A active only (Context B absent).
- **Cycle 5**: Context A + B active (Context B returns).

## Evaluated Policies
1. **`passive_night_baseline`**: Day replay + STDP learning + passive recovery.
2. **`dormant_reactivation_only`**: Pruning + Dormant Bank + indexed reactivation (COLLAPSED by Cycle 4).
3. **`sprouting_only`**: Pruning + stochastic geometry sprouting (STABLE).
4. **`full_lifecycle`**: Pruning + Dormant Bank + indexed reactivation + stochastic geometry sprouting (PARTIAL DIAGNOSTIC).

## Directory Structure
- `artifacts/plot_data.json`: Serialized metrics across all 4 policies and 5 cycles.
- `scripts/generate_plots_v1_2.py`: Matplotlib script generating visual comparisons.
- `reports/report_v1_2.md`: Detailed scientific report.
- `images/lifecycle_counts.png`: Population dynamics over cycles.
- `images/rare_path_retention.png`: Context B rare-path active cohort mean weight over cycles.
- `images/structural_health.png`: Fan-in Gini coefficient over cycles.
- `images/blocker_breakdown.png`: Rejection blocker breakdown per cycle.
