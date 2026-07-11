# Night Phase MVP Eviction Stress Report (v1.6b)

**Status**: `SUCCESS / TARGET+GLOBAL BOUNDED EVICTION VALIDATED`  
**Execution Date**: 2026-07-06  
**Test Harness**: [night_phase_mvp_eviction_stress_v1_6b.rs](file:///home/alex/AI_Home/workflow/AxiEngine/crates/test-harness/tests/night_phase_mvp_eviction_stress_v1_6b.rs)  
**Plot Data**: [plot_data.json](file:///home/alex/AI_Home/workflow/docs/engine/research/archive/2026-07-06_night_phase_mvp_eviction_stress_v1_6b/artifacts/plot_data.json)

---

## 1. Executive Summary

In the baseline consolidated v1.6 run, the Dormant Bank was under-utilized (peak of 44 synapses, age=0, 0 evictions), leaving bounded eviction under load unproven.

**v1.6b successfully validated target-cap and global-cap eviction under intense load.** By raising the `PRUNE_WEIGHT_THRESHOLD` to $1490 \times 2^{16}$ (extremely close to the initial weight of $1500 \times 2^{16}$), Hebbian learning immediately classified 11,403 depressed synapses as "weak" in Cycle 1. 

The bounded eviction policy successfully:
1. **Bounded the Dormant Bank**: Restricted the bank size to exactly 200 synapses (global limit) and exactly 3 synapses per target (target limit).
2. **Evicted Excess Synapses**: Evicted 11,203 synapses to the cumulative dead count in Cycle 1.
3. **Maintained Structural Integrity**: The active synapse count remained stable around 34,500–35,000 synapses, and 0 safety gate violations (Dale, dense, duplicate, runaway, or geometry) occurred across all 12 cycles.

The age+trace eviction path is implemented but was not exercised in this stress run (`age_trace = 0` for all cycles), because retained dormant entries kept non-zero `long_trace` and were displaced by target/global caps first.

---

## 2. Quantitative Results Table

| Cycle | Active Synapses | Dormant Synapses | Cumulative Dead (Evicted) | Pruned-to-Dormant | Sprouted Count | Global Evictions | Target Evictions |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | 35,728 | 200 | 11,203 | 11,403 | 478 | 350 | 10,853 |
| **2** | 35,296 | 200 | 11,943 | 740 | 308 | 95 | 645 |
| **3** | 35,364 | 200 | 12,111 | 168 | 236 | 42 | 126 |
| **4** | 35,298 | 200 | 12,621 | 510 | 444 | 125 | 385 |
| **5** | 35,377 | 200 | 12,755 | 134 | 213 | 43 | 91 |
| **6** | 35,151 | 200 | 13,252 | 497 | 271 | 228 | 269 |
| **7** | 35,154 | 200 | 13,582 | 330 | 333 | 93 | 237 |
| **8** | 35,174 | 200 | 13,726 | 144 | 164 | 62 | 82 |
| **9** | 34,868 | 200 | 14,409 | 683 | 377 | 142 | 541 |
| **10** | 34,843 | 200 | 14,869 | 460 | 435 | 164 | 296 |
| **11** | 34,900 | 200 | 15,062 | 193 | 250 | 79 | 114 |
| **12** | 34,560 | 200 | 15,871 | 809 | 469 | 290 | 519 |

---

## 3. Analysis & Key Insights

### 3.1. Target/Global Bounded Eviction Performance
The target and global bounds successfully held under massive pressure:
* **Target Cap Eviction**: A target neuron can hold a maximum of 3 dormant incoming connections. Due to layers receiving a high volume of prunes, target cap eviction was the primary filter (evicting 10,853 synapses in Cycle 1).
* **Global Cap Eviction**: Synapses surviving the target cap filter are sorted by `long_trace` (highest first) and `dormant_age` (youngest first). The global cap of 200 was filled, causing the remaining 350 synapses to be evicted to dead in Cycle 1.
* **Age/Trace Eviction**: Max age was set to 2, but age+trace eviction did not fire in this run (`age_trace = 0` across all cycles). Retained dormant entries kept non-zero `long_trace`; cap eviction handled pressure first. This path still needs a separate trace-decay/aging micro-test if we want to validate it directly.

### 3.2. Sprouting Inequality
Under high pruning stress, sprouting was triggered on under-recruited target somas. The new metric `sprout_target_gini` shows an inequality range of `0.84` to `0.92`, indicating that sprouting was focused specifically on a subset of heavily under-recruited targets rather than uniform distribution, which is correct homeostatic behavior.

### 3.3. Active Synapse Safety Gates
Despite intense pruning (hundreds of synapses pruned per cycle), the network's active synapses stabilized around **34,500**. This demonstrates that the safety gates (`min_target_active_count = 5`, `min_projection_active_count = 2`) successfully blocked pruning once active synapse density reached safe thresholds. 

---

## 4. Visual Evidence

* **Lifecycle Counts**: Plots active vs dormant vs dead counts under stress, demonstrating stable active counts and log-linear dead count growth.
* **Eviction Metrics**: Visualizes dormant bank counts, max dormant age, and a bar chart detailing reasons (target vs global caps) per cycle.
* **Sprouting Gini & Inequality**: Tracks the Gini coefficient and target count of sprouting.
