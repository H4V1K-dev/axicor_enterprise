# GSOP / DA Transfer Audit Report

## Phase H1: Parity Probe of Slot Update Math

### 1. Preregistration

#### Research Question
Are there mathematical or semantic differences between AxiEngine's production `apply_gsop_plasticity` and the legacy `cpu_apply_gsop` slot update math under identical inputs? If so, what is the magnitude and sign of the resulting weight change ($\Delta$mass) mismatch?

#### Prior Model and Competing Explanations
- **Prior Model:** We assume that AxiEngine's production GSOP implementation is a functionally equivalent port of the legacy `cpu_apply_gsop` logic, and that any failure of Cue Association (C4) is due to parameter scaling rather than structural logic discrepancies.
- **Competing Explanation:** The two rules contain structural logical differences, meaning AxiEngine does not compute the same plasticity updates on identical inputs. This logical mismatch is the root cause of the lack of learning dynamics in C4.

#### Experimental Fixture and Inputs
We evaluated both slot update algorithms under identical inputs. The fixture parameters are defined below with their biological and configuration provenance.

##### Parameter Provenance Table

| Parameter | Value / Range | Unit / Domain | Source / Reference |
|---|---|---|---|
| Initial Mass ($w$) | `3500 << 16` | Mass Domain | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| Potentiation Amount ($pot$) | `240` | Mass Domain | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| Depression Amount ($dep$) | `68` | Mass Domain | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| Dopamine Modulation ($DA$) | `0` and `50` | Mass Domain / Reward Scale | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| D1 receptor affinity | `192` | Scale Factor (128 = 1.0x) | `Axicor_Neuron-Lib/modernized/L4_spiny_VISl4_4.toml` |
| D2 receptor affinity | `128` | Scale Factor (128 = 1.0x) | `Axicor_Neuron-Lib/modernized/L4_spiny_VISl4_4.toml` |
| Inertia curve | `[128, 121, 116, 110, 105, 100, 95, 91]` | Rank multipliers | `Axicor_Neuron-Lib/modernized/L4_spiny_VISl4_4.toml` |
| Burst count multiplier | `1` | Integer | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| Signal propagation length | `20` | Voxels / Ticks | `Axicor_Neuron-Lib/modernized/L4_spiny_VISl4_4.toml` |
| Dendrite Segment Index | `100` | Segment Position | Documented Vector Setup |
| Fatigue Capacity | `18` | Ticks | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |
| Fatigue Level | `0` and `18` | Ticks | `artifacts/agent-tasks/inbox/L050_gsop_da_transfer_audit.md` Suggested |

##### Test Cases (Heads Setup)
We ran the above combinations against 4 different axonal heads configurations:
1. **Case A (Causal Spike)**: One causal head in window. `heads = [110, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL]`. Since `110 >= seg_idx (100)` and `110 - 100 <= prop (20)`.
2. **Case B (Anti-Causal Spike)**: One anti-causal head in window. `heads = [90, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL]`. Since `seg_idx (100) >= 90` and `100 - 90 <= prop (20)`.
3. **Case C (Both)**: One causal head and one anti-causal head in window. `heads = [110, 90, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL]`.
4. **Case D (Inactive)**: No active heads in window. `heads = [AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL, AXON_SENTINEL]`.

#### Primary Metrics
- **Synaptic weight change ($\Delta$mass):** Computed as $w_{\text{final}} - w_{\text{init}}$ for each rule.
- **Arithmetic equivalency:** Boolean flag indicating whether the final weights match ($w_{\text{legacy}} == w_{\text{axi}}$).

#### Success and Rejection Criteria
- **Support Criteria (Mismatch Found):** Any test case where the resulting $\Delta$mass is unequal ($w_{\text{legacy}} \neq w_{\text{axi}}$), demonstrating structural mathematical divergence.
- **Rejection Criteria (Bit Parity):** All test cases return bit-identical weight updates, rejecting structural mismatch in favor of scale issues.

#### Runner and Reproducible Command
- **Runner:** `AxiEngine/crates/physics/tests/physics_tests.rs` under function `test_gsop_parity_probe`.
- **Command:** `cargo test -p physics --test physics_tests test_gsop_parity_probe`

#### Known Limitations
- The gate only tests single-synapse/slot arithmetic on a single tick. It does not evaluate full network-level learning dynamics, which are reserved for later gates H2–H5.

---

## Results

The parity probe unit test `test_gsop_parity_probe` was executed successfully. Below is the comparative results table:

| Case | Dopamine | Fatigue/Cap | Legacy Delta | Axi Delta | Equal? |
|---|---|---|---|---|---|
| Case A (Causal Only) | 0 | 0/18 | 240 | 120 | NO |
| Case A (Causal Only) | 0 | 18/18 | 0 | 52 | NO |
| Case A (Causal Only) | 50 | 0/18 | 315 | 157 | NO |
| Case A (Causal Only) | 50 | 18/18 | 0 | 139 | NO |
| Case B (Anti-Causal Only) | 0 | 0/18 | -68 | -34 | NO |
| Case B (Anti-Causal Only) | 0 | 18/18 | 0 | -102 | NO |
| Case B (Anti-Causal Only) | 50 | 0/18 | -18 | -9 | NO |
| Case B (Anti-Causal Only) | 50 | 18/18 | 0 | -27 | NO |
| Case C (Both) | 0 | 0/18 | 240 | 86 | NO |
| Case C (Both) | 0 | 18/18 | 0 | 18 | NO |
| Case C (Both) | 50 | 0/18 | 315 | 148 | NO |
| Case C (Both) | 50 | 18/18 | 0 | 130 | NO |
| Case D (Inactive) | 0 | 0/18 | -68 | 0 | NO |
| Case D (Inactive) | 0 | 18/18 | 0 | -68 | NO |
| Case D (Inactive) | 50 | 0/18 | -18 | 0 | NO |
| Case D (Inactive) | 50 | 18/18 | 0 | -18 | NO |

---

## Analysis of Mismatches

We identify four distinct structural differences between the legacy `cpu_apply_gsop` slot update math and production `apply_gsop_plasticity`:

### 1. Spatial Cooling
- **Legacy:** Does not scale the delta by the distance of the active head from the segment. An active head anywhere in the propagation window (`0..=prop`) yields a full constant pulse: `delta_pot` for causal hits, or `-delta_dep` for anti-causal hits (though anti-causal is handled as part of inactive/uncorrelated synapses).
- **AxiEngine:** Scales the delta linearly with the distance (`cooling = prop - distance`). For instance, at `distance = 10` and `prop = 20`, the delta is halved:
  $$\Delta_{\text{axi\_causal}} = \frac{\text{base\_ltp} \times 10}{20} = 120 \quad (\text{vs. } 240 \text{ in legacy})$$

### 2. Postsynaptic Spike LTD (Inactive Synapse Mismatch)
- **Legacy:** If a postsynaptic cell spikes, all incoming synapses that do not have an active causal head in the propagation window (`is_active == 0`) undergo depression by `-delta_dep` (`-68` when $DA=0$).
- **AxiEngine:** Inactive synapses (Case D, fatigue = 0) undergo **no change** ($\Delta\text{mass} = 0$). They only receive a depression penalty if their dendritic fatigue is greater than 0, or if they are in the anti-causal cooling window.
- **Impact:** In legacy, there is constant, widespread LTD on all inactive/uncorrelated synapses whenever a postsynaptic cell fires. This creates strong competitive LTD pressure, which forces active synapses to stand out. In AxiEngine, inactive/uncorrelated synapses remain unchanged ($\Delta\text{mass}=0$), preventing competitive depression of inactive tracks and weakening learning.

### 3. Fatigue Penalty vs. Legacy Refractory Gating
- **Legacy:** If the dendritic timer (fatigue) is greater than 0, the slot update is skipped entirely. No fatigue penalty is subtracted from the delta when updating.
- **AxiEngine:** The slot update is computed, but a fatigue penalty is subtracted:
  $$\text{fatigue\_penalty} = \frac{\text{fat} \times \text{base\_ltd}}{\text{capacity}}$$
  At maximum fatigue (`18/18`), this subtracts `-68` (when $DA=0$).

### 4. Causal/Anti-Causal Co-existence
- **Legacy:** A synapse is either potentiated (if at least one head has passed, `is_active != 0`) or depressed (if `is_active == 0`). Both LTP and LTD cannot be accumulated in a single tick.
- **AxiEngine:** Accumulates both LTP and LTD by scanning all heads (e.g. Case C: causal head at 110 yields $+120$, anti-causal head at 90 yields $-34$, net delta $+86$).

---

## Verdict

Hypothesis **H1 is SUPPORTED**. There is a severe, material mismatch in both magnitude and direction of slot update changes between AxiEngine's production GSOP rule and legacy slot-update math. Specifically, the absence of widespread postsynaptic LTD on inactive/uncorrelated synapses in AxiEngine is a structural rule difference and a **candidate mechanism** for weak competitive differentiation. H1 alone does **not** establish that this mismatch caused C4 behavioral failure.

---

## Single Next Step

**Phase H2: Event Counters Audit**  
Since H1 confirmed that the rule math is structurally different, we will design a runner/test to analyze event counters and net weight changes under task-like trials to see if LTP and LTD largely cancel (wash) out in the absence of competitive LTD.

---

## Phase H2: LTP/LTD Event Counters Audit (Wash)

### 1. Preregistration

#### Research Question
Under a task-like presynaptic and postsynaptic spike schedule, do AxiEngine's production GSOP plasticity updates ($\Delta$mass) on matched and unmatched synapses undergo a mutual cancellation (wash), resulting in near-zero net weight differentiation?

#### Prior Model and Competing Explanations
- **Prior Model:** Based on H1 findings (specifically, spatial cooling and co-addition of LTP/LTD), we hypothesize that under realistic spiking activity, opposing LTP and LTD updates largely cancel each other out, leaving net weight changes flat.
- **Competing Explanations:**
  1. **True Wash:** Opposing LTP and LTD events occur with high frequency and magnitude but cancel each other out, resulting in a Wash Index near 1.0.
  2. **Near-Zero Event Rate:** Weights remain flat simply because presynaptic and postsynaptic spikes rarely align to trigger updates.
  3. **Fatigue Dominance:** The net weight change is negative because dendritic fatigue accumulates and dominates the update, driving weights down.
  4. **Post-Spike Sparsity:** Postsynaptic spikes are too sparse to drive significant cumulative weight changes.

#### Experimental Fixture and Inputs
We construct a synthetic 330-tick trial simulating a single postsynaptic L4 neuron with 12 dendritic slots:
- **7 Matched Synapses:** Preferential inputs that receive stochastic presynaptic spikes ($P_{\text{spike}} = 0.11$ per tick) during ticks 0..20 (the input phase of the trial).
- **5 Unmatched Synapses:** Non-preferential inputs that receive 0 presynaptic spikes.
- **Postsynaptic Firing:** The L4 neuron fires at ticks 40, 120, and 200, triggering GSOP weight updates on those ticks.
- **Dendritic Fatigue:** Propagates and recovers according to production constants (spike cost 50, recovery 1/tick).

##### Parameter Provenance Table

| Parameter | Value | Unit / Domain | Source / Reference |
|---|---|---|---|
| L4 `gsop_potentiation` | `240` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `gsop_depression` | `68` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `fatigue_capacity` | `18` | Ticks | Winner L4 GSOP / LP4 |
| Fatigue spike cost | `50` | Ticks | Production constants |
| Fatigue recovery rate | `1` | Ticks / tick | Production constants |
| dopamine | `0` and `50` | Neuromod Scale | LP C3/C4 |
| Initial Mass ($w$) | `3500 << 16` | Mass Domain | LP4 / v1.4 |
| Signal propagation length | `20` | Ticks | modernized profile |
| Dendrite Segment Index | `10` | Segment Position | Typical L4 segment |

#### Primary Metrics
- **LTP-Positive Count:** Number of slot updates where $\Delta\text{mass} > 0$.
- **LTD-Negative Count:** Number of slot updates where $\Delta\text{mass} < 0$.
- **Summed LTP mass change ($\sum \text{LTP}$):** Cumulative positive mass changes.
- **Summed LTD mass change ($\sum \text{LTD}$):** Cumulative negative mass changes.
- **Summed Net Mass change ($\sum \text{net\_mass}$):** Cumulative net signed change in mass.
- **Wash Index:** Defined as $1 - \frac{|\sum \text{net\_mass}|}{\sum \text{LTP} + \sum |\text{LTD}|}$ (if the denominator is $>0$; otherwise 0). A value close to 1.0 indicates near-complete mutual cancellation.
- **Synapse-Class Net Change:** Mean $\Delta\text{mass}$ on matched vs. unmatched synapses.

#### Success and Rejection Criteria
- **SUPPORTED (True Wash):** Wash Index $\ge 0.50$ AND the mean net mass change difference between matched and unmatched synapses is $< 500$ mass units.
- **REJECTED (True Wash):** Wash Index $< 0.50$ OR matched synapses show clear net directional differentiation from unmatched synapses ($\ge 500$ mass units).

#### Runner and Reproducible Command
- **Runner:** `AxiEngine/crates/physics/tests/physics_tests.rs` under function `test_gsop_h2_event_counters_wash`.
- **Command:** `cargo test -p physics --test physics_tests test_gsop_h2_event_counters_wash -- --nocapture`

#### Known Limitations
- H2 evaluates the event balance at a single-neuron level under a synthetic task-like schedule (delayed postsynaptic spikes at ticks 40/120/200 after a 0..20 input window). This schedule systematically under-exercises anti-causal LTD and concurrent LTP/LTD co-add.
- It does not model full recurrent microcircuit dynamics or C4 accuracy.
- A wash REJECT under this schedule does not rule out wash under overlapping pre/post firing (follow-up H3-style schedules).

---

## H2 Results and Analysis

The Phase H2 event counters wash audit was executed successfully. The empirical results across 100 independent trials are summarized in the table below:

| Condition | Synapse Class | LTP Count | LTD Count | Sum LTP | Sum LTD | Net Change | Wash Index | Mean Net Change |
|---|---|---|---|---|---|---|---|---|
| Normal | Matched | 490 | 0 | 66327 | 0 | 66327 | 0.0000 | 94.75 |
| Normal | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 |
| DA-off | Matched | 490 | 0 | 50772 | 0 | 50772 | 0.0000 | 72.53 |
| DA-off | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 |
| Plasticity-off | Matched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 |
| Plasticity-off | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 |

### Analysis of Findings

1. **Zero Wash (No LTD Cancellation):** 
   Under the task-like schedule where presynaptic inputs occur during the first 20 ticks and postsynaptic spikes are at ticks 40, 120, and 200, the **Wash Index is exactly 0.0000**. Because the postsynaptic spikes occur long after the presynaptic spikes, we observe exactly `0` LTD events. All spikes fall inside the causal LTP window, causing only positive weight changes.
2. **Absence of Competitive LTD:**
   Unmatched synapses experience exactly `0` weight change. Under AxiEngine's production rule, inactive/uncorrelated synapses do not receive any LTD penalty unless they have fatigue or fall in the anti-causal window. Because they are not stimulated, they remain at 0 fatigue and experience no change. This is a crucial difference from the legacy rule, which would depress all inactive synapses by `-delta_dep` upon postsynaptic firing, creating strong competitive differentiation pressure.
3. **Extremely Damped Update Magnitude:**
   Although matched synapses undergo LTP, the total cumulative weight change is extremely small. In the Normal condition, matched synapses only gain +66,327 mass units across 100 trials (which equates to less than 1 charge-domain unit per trial!). This confirms that the learning updates in AxiEngine are highly damped by spatial cooling and inertia.

### Verdict

Hypothesis **H2 is REJECTED** under the preregistered delayed-post schedule: Wash Index = 0.0000 (no opposing LTD mass), and matched vs unmatched net mass clearly separates (+66327 vs 0 across 100 trials). Opposing LTP/LTD cancellation is **not** the active explanation for flat trajectories **in this fixture**.

Secondary observations (not substitute C4 evidence): matched LTP is small in charge units (~0.01 charge-domain mass-equivalent per trial across matched slots), and unmatched slots stay at Δmass = 0 (consistent with H1 inactive-slot behavior).

### Claim Boundary
- H2 establishes: no LTP/LTD wash under **delayed-post** synthetic single-neuron schedule with frozen v1.4 rates.
- H2 does **not** establish: C4 accuracy failure cause; full-microcircuit event balance; wash under overlapping pre/post; authorization to change production GSOP.

---

## Single Next Step

**Phase H3: Fatigue Dominance Audit**  
Since H2 showed that LTD is 0 when postsynaptic spikes are delayed, we will proceed to H3 to audit fatigue penalties and net updates under a postsynaptic spike schedule that occurs *during* the presynaptic input window (e.g. postsynaptic spikes at ticks 5, 10, 15), where fatigue is actively accumulated.

---

## Phase H3: Fatigue Dominance under Overlapping Pre/Post

### 1. Preregistration

#### Research Question
Under an overlapping presynaptic and postsynaptic spike schedule (where postsynaptic spikes occur inside the input window), does the accumulated dendritic fatigue penalty dominate the net weight update ($\Delta$mass), and how does it change the matched vs. unmatched weight balance?

#### Prior Model and Competing Explanations
- **Prior Model:** When postsynaptic spikes occur during the presynaptic input window, presynaptic spikes build up dendritic fatigue. At the time of the postsynaptic spike, the high fatigue level ($fat > 0$) causes a fatigue penalty to be subtracted from the weight, dominating the net update and driving weight changes negative.
- **Competing Explanations:**
  1. **Fatigue Dominance:** The fatigue penalty dominates the update, driving matched synapses to net depression ($\sum \text{net\_mass} < 0$) despite causal spikes.
  2. **Residual LTP Wins:** Causal LTP is larger than the fatigue penalty, resulting in positive net updates ($\sum \text{net\_mass} > 0$).
  3. **Anti-Causal LTD Appears:** Spikes occurring after a postsynaptic spike (anti-causal window) also contribute to weight depression, cooperating with the fatigue penalty.

#### Experimental Fixture and Inputs
We adapt the H2 fixture with the following modification:
- **Postsynaptic Firing:** The postsynaptic spikes occur at ticks **5, 10, and 15**, which overlap directly with the presynaptic input window (ticks 0..20).
- All other parameters (7 matched synapses with $P_{\text{spike}} = 0.11$ in 0..20, 5 unmatched synapses, initial weight `3500 << 16`, fatigue capacity 18, cost 50, recovery 1/tick) remain identical.

##### Parameter Provenance Table

| Parameter | Value | Unit / Domain | Source / Reference |
|---|---|---|---|
| L4 `gsop_potentiation` | `240` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `gsop_depression` | `68` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `fatigue_capacity` | `18` | Ticks | Winner L4 GSOP / LP4 |
| Fatigue spike cost | `50` | Ticks | Production constants |
| Fatigue recovery rate | `1` | Ticks / tick | Production constants |
| dopamine | `0` and `50` | Neuromod Scale | LP C3/C4 |
| Initial Mass ($w$) | `3500 << 16` | Mass Domain | LP4 / v1.4 |
| Signal propagation length | `20` | Ticks | modernized profile |
| Dendrite Segment Index | `10` | Segment Position | Typical L4 segment |

#### Primary Metrics
- **LTP-Positive Count:** Number of slot updates where $\Delta\text{mass} > 0$.
- **LTD-Negative Count:** Number of slot updates where $\Delta\text{mass} < 0$.
- **Summed LTP mass change ($\sum \text{LTP}$):** Cumulative positive mass changes.
- **Summed LTD mass change ($\sum \text{LTD}$):** Cumulative negative mass changes.
- **Summed Net Mass change ($\sum \text{net\_mass}$):** Cumulative net signed change in mass.
- **Mean Fatigue at Postsynaptic Spikes:** Average fatigue value recorded for matched synapses at ticks 5, 10, and 15.
- **Wash Index:** $1 - \frac{|\sum \text{net\_mass}|}{\sum \text{LTP} + \sum |\text{LTD}|}$
- **Synapse-Class Net Change:** Mean $\Delta\text{mass}$ on matched vs. unmatched synapses.

#### Success and Rejection Criteria
- **SUPPORTED (Fatigue Dominance):** Matched synapses show negative net signed change ($\sum \text{net\_mass} < 0$) AND mean fatigue at postsynaptic ticks is $> 5$ ticks.
- **REJECTED (Fatigue Dominance):** Matched synapses show positive net signed change ($\sum \text{net\_mass} > 0$).

#### Runner and Reproducible Command
- **Runner:** `AxiEngine/crates/physics/tests/physics_tests.rs` under function `test_gsop_h3_fatigue_dominance`.
- **Command:** `cargo test -p physics --test physics_tests test_gsop_h3_fatigue_dominance -- --nocapture`

#### Known Limitations
- H3 evaluates the fatigue penalty balance at a single-neuron level under a synthetic overlapping pre/post schedule. It does not establish C4 behavior recovery or authorize production rewrites.

---

## H3 Results and Analysis

The Phase H3 fatigue dominance audit was executed successfully. The empirical results across 100 independent trials are summarized in the table below:

| Condition | Synapse Class | LTP Count | LTD Count | Sum LTP | Sum LTD | Net Change | Wash Index | Mean Fatigue | Mean Net Change |
|---|---|---|---|---|---|---|---|---|---|
| Normal | Matched | 426 | 1009 | 134873 | -33229 | 101644 | 0.3953 | 9.63 | 145.21 |
| Normal | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 | 0.00 |
| DA-off | Matched | 397 | 1038 | 60686 | -128360 | -67674 | 0.6420 | 9.63 | -96.68 |
| DA-off | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 | 0.00 |
| Plasticity-off | Matched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 9.63 | 0.00 |
| Plasticity-off | Unmatched | 0 | 0 | 0 | 0 | 0 | 0.0000 | 0.00 | 0.00 |

### Analysis of Findings

1. **Active Fatigue Accumulation (Mean Fatigue = 9.63):**
   Under the overlapping postsynaptic schedule (ticks 5, 10, 15), presynaptic spikes successfully drove dendritic fatigue to an average level of `9.63` ticks (out of `18` capacity) at the time of postsynaptic spikes.
2. **Dopamine Modulates Net Update Direction:**
   - In the **DA-off condition ($DA=0$)**, matched synapses underwent net depression (Net Change = -67,674, Mean Net Change = -96.68). The fatigue penalty and anti-causal LTD completely dominated, driving weights down.
   - In the **Normal condition ($DA=50$)**, matched synapses underwent net potentiation (Net Change = +101,644, Mean Net Change = +145.21). Causal LTP, boosted by dopamine, overcame the fatigue penalty.
3. **No Wash under Overlapping Spikes:**
   The Wash Index in the Normal condition was `0.3953` (less than the `0.50` wash threshold), indicating that while both LTP and LTD events occurred, they did not cancel out.

### Verdict

Hypothesis **H3 is REJECTED** under the Normal condition. Although fatigue accumulates during overlapping activity and generates significant LTD updates, the dopamine-boosted causal LTP remains strong enough to overcome the fatigue penalty, resulting in positive net weight updates (Net Change > 0). 

However, the fact that Net Change flips to negative under DA-off (-67,674) shows that dopamine neuromodulation is the critical switch controlling the direction of plasticity under realistic, high-fatigue firing regimes.

### Claim Boundary
- H3 establishes that under overlapping pre/post schedules with winner GSOP rates, fatigue does not prevent net potentiation under normal dopamine levels.
- H3 does not establish that task-level learning is resolved or that production GSOP updates are authorized.

---

## Phase L053: Network Weight Differentiation Probe (Post-T015)

### 1. Preregistration

#### Research Question
Under the post-T015 competitive LTD rule (inactive slots depressed by `base_ltd` during postsynaptic spikes), do unmatched/inactive synapse masses successfully depress relative to matched/causal synapses in a microcircuit network fixture under frozen rates, or do they remain flat?

#### Prior Model and Competing Explanations
- **Prior Model:** Pre-T015, unmatched synapses in network-like layouts remained completely flat ($\Delta\text{mass} = 0$) because the learning rule lacked competitive depression. T015 introduced component-level competitive LTD.
- **Competing Explanations:**
  1. **Differentiation Appears:** The post-T015 competitive LTD rule successfully depresses unmatched pathways, driving their net weight updates negative, thereby widening the differentiation gap between matched and unmatched synapses.
  2. **Damped Plasticity (Still Flat):** The updates are too highly damped by spatial cooling and inertia rank curves to produce significant net weight differences in the network.
  3. **Activity Too Sparse:** L4 target neurons do not fire enough postsynaptic spikes during the training phase to trigger sufficient competitive LTD updates on the unmatched synapses.

#### Experimental Fixture and Inputs
We use a microcircuit network training fixture configured similarly to the LP4 task learning test harness:
- **Topology:** L4 spiny neurons with 12 input synapses each (7 matched synapses from virtual axons, 5 unmatched synapses from virtual axons).
- **Spiking Cadence:** Presynaptic inputs stimulated at ticks 0, 2, ..., 18. Target neurons fire stochastically according to GLIF membrane dynamics.
- **Dopamine Delivery:** Dopamine is delivered dynamically based on choice (50 for correct, -50 for incorrect, or held at 0 in controls).
- **Initial weights:** `3500` (mass `3500 << 16`).
- **Condition Cases:**
  - **Normal**: Dynamic dopamine (50/-50) and active plasticity.
  - **DA-off**: Dopamine fixed at 0 and active plasticity.
  - **Plasticity-off (Control)**: Plasticity disabled.

##### Parameter Provenance Table

| Parameter | Value | Unit / Domain | Source / Reference |
|---|---|---|---|
| Initial Mass ($w$) | `3500 << 16` | Mass Domain | LP4 / v1.4 |
| L4 `gsop_potentiation` | `240` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `gsop_depression` | `68` | Mass Domain | Winner L4 GSOP / LP4 |
| L4 `fatigue_capacity` | `18` | Ticks | Winner L4 GSOP / LP4 |
| L4 `dopamine` | `0` or `50` / `-50` | Neuromod Scale | LP C3/C4 |
| Signal propagation length | `20` | Ticks | modernized profile |

#### Primary Metrics
- **Mean matched synapse mass change (Mean $\Delta\text{matched}$)**: average change in mass for matched connections post-training.
- **Mean unmatched synapse mass change (Mean $\Delta\text{unmatched}$)**: average change in mass for unmatched connections post-training.
- **Matched-Unmatched Gap**: calculated as $\text{Mean } \Delta\text{matched} - \text{Mean } \Delta\text{unmatched}$.

#### Success and Rejection Criteria
- **SUPPORTED (Differentiation Appears):** Unmatched synapses show negative net signed change ($\text{Mean } \Delta\text{unmatched} < 0$) AND the matched-unmatched gap $\ge 100$ mass units.
- **REJECTED:** Unmatched synapses remain flat ($\text{Mean } \Delta\text{unmatched} \ge -10$ mass units) OR the matched-unmatched gap is $< 100$ mass units.
- **INVALID:** If the `plasticity_off` control shows any non-zero weight changes ($\text{Mean } \Delta\text{matched} \neq 0$ or $\text{Mean } \Delta\text{unmatched} \neq 0$).

#### Runner and Reproducible Command
- **Runner:** `AxiEngine/crates/test-harness/tests/lp4_task_learning_tests.rs` under function `test_network_weight_differentiation_probe`.
- **Command:** `cargo test -p test-harness --test lp4_task_learning_tests --features full-chain-probe,mvp-cpu-replay test_network_weight_differentiation_probe -- --nocapture`

#### Known Limitations
- The probe only measures weight differentiation and does not guarantee that full task learning accuracy (>= 70%) is restored under these frozen rates.

### 2. Results and Analysis

We executed the network-level weight differentiation probe test case `test_network_weight_differentiation_probe` successfully. The results from seed 42 post-training are summarized below:

| Condition | Class | Post-Train Avg (Charge) | Mean Net Change (Mass Domain) |
|---|---|---|---|
| plasticity_off | Matched | 3500.00 | 0.00 |
| plasticity_off | Unmatched | 3500.00 | 0.00 |
| normal | Matched | 3499.00 | -65462.86 |
| normal | Unmatched | 3498.94 | -69734.40 |

#### Metrics Analysis
- **Mean $\Delta\text{matched}$:** $-65,462.86$ mass units (approx $-1.00$ charge units).
- **Mean $\Delta\text{unmatched}$:** $-69,734.40$ mass units (approx $-1.064$ charge units).
- **Matched-Unmatched Gap:** $+4,271.54$ mass units (approx $+0.064$ charge units).
- **Control Validation:** Under the `plasticity_off` condition, the post-train weights for both matched and unmatched synapses remained exactly at `3500.00` charge units ($\Delta\text{mass} = 0.0$), validating the control.

#### Observations
1. **Unmatched Pathways Depress Downward:** Under the new post-T015 competitive LTD rule, unmatched/inactive synapses successfully depressed relative to their initial weight ($\text{Mean } \Delta\text{unmatched} = -69,734.40 < 0$). This is a major change from pre-T015 behavior where unmatched pathways remained flat at 0 change.
2. **Clear Differentiation Emerges:** Because unmatched synapses depressed further than matched synapses, a clear differentiation gap of $+4,271.54$ mass units was established.
3. **General Downward Drift:** Both matched and unmatched weights drifted downward because postsynaptic spikes occurred throughout the 330-tick trial, whereas presynaptic stimulation was restricted to 0..20, causing extensive out-of-sync competitive depression. However, the correlation of matched inputs in the early phase protected them from maximum depression relative to unmatched inputs.

### 3. Verdict

Hypothesis **L053 is SUPPORTED**. The post-T015 competitive LTD rule successfully drives unmatched weights downward in a network-like microcircuit under frozen rates, establishing a clear differentiation gap ($\ge 100$ mass units).

---

## Program decision (stop research ladder)

**H4 cancelled. H5** only if weights still cannot move choice after a network probe.

| Priority | Action | Status |
|---|---|---|
| P0 | Production competitive / inactive-slot LTD | **DONE (T015)** — `apply_gsop_plasticity`; `test_competitive_depression_proof` |
| P1 | Network-level matched vs unmatched mass under frozen rates | **Next** |
| P2 | Optional C4-like re-run | Only after P1 shows real differentiation |
| — | Do **not** invent pot/DA scale to fake learning | forbidden |

Historical H1–H3 text above describes **pre-T015** production behavior where inactive Δmass was 0.
