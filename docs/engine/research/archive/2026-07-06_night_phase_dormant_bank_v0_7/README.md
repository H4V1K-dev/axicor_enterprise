# Night Phase Dormant/Cold Bank Stress Test (v0.7)

This research experiment evaluates the concept of a Dormant/Cold Storage Bank to preserve structural option value during high pruning pressure.

## Objective
Synapses selected for pruning are not immediately deleted but demoted to a Dormant Bank (Cold Storage). These synapses do not participate in day-phase transmission (meaning they do not transmit charge or consume dendrite slots). At Night 2, a reactivation pass tries to restore them to active status if trace and day-activity evidence is met.

## Directory Structure
- `artifacts/plot_data.json`: Serialized JSON plotting data containing active, dormant, and deleted counts, memory retention ratios, and dormant traces.
- `scripts/generate_plots_v0_7.py`: Python script utilizing Matplotlib/Numpy to plot comparative graphs.
- `reports/night_phase_dormant_bank_v0_7.md`: Scientific report summarizing results and limitations.
