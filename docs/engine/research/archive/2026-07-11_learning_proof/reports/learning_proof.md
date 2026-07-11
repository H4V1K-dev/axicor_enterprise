# Learning Proof Program — Cumulative Report

Status: **COMPLETE — REJECTED IN CURRENT SCOPE** (C0-C3 completed historically; C4 completed and rejected)
Slug: `learning_proof`
Parent Program: `LEARNING_PROOF_MONOSPEC.md`

This document serves as the single source of truth for cumulative evidence and preregistrations across the Learning Proof (LP) program.

---

## 1. Historical Record (C0 - C3)

### LP-0: Controllability (C0)
* **Question:** Can we reliably enable and disable weight updates via the `plasticity_enabled` switch without affecting base electrical physics?
* **Result:** **SUPPORTED**. Verification of FNV-1a checksums on the weight plane before/after day compute runs showed bit-perfect conservation when `plasticity_enabled = false`.

### LP-1: Plasticity Causality (C1)
* **Question:** Does the production GSOP/STDP rule strengthen correlated pathways compared to control pathways?
* **Result:** **SUPPORTED WITH CAVEATS**. 
  * Initial Weight (mass): `6553600` (exact `100.00` uV)
  * Final Correlated Weight (mass): `6555263` (`+1663` delta)
  * Final Control Weight (mass): `6554192` (`+592` delta)
  * Verdict: Relative potentiation bias confirmed across seeds 42/100/2026.

### LP-2: Retention after Freeze (C2)
* **Question:** Do weight changes and relative correlation bias persist after switching off plasticity?
* **Result:** **SUPPORTED**. During the frozen evaluation phase, the weight checksums matched bit-perfectly before and after running compute, with the correlation bias ($w_{correlated} > w_{control}$) successfully retained.

### LP-3: Dopamine Causality (C3)
* **Question:** Is the weight potentiation causal to the dopamine reward schedule?
* **Result:** **SUPPORTED**. Ablation runs with `dopamine = 0` during training resulted in no weight differentiation ($\Delta w_{corr} \approx \Delta w_{ctrl} \approx 0$), proving that neuromodulation is necessary for pathway stabilization.

---

## 2. LP-4 (C4) Calibration Transfer Dossier

L040 uses the calibrated microcircuit as the frozen baseline. We transfer parameters from the manual controlled-topology runner of Phase A.

### 2.1 Carried Parameters & Provenance Table

| Parameter | Calibrated Value | Source Reference | Description / Mapping |
| :--- | :--- | :--- | :--- |
| **L4 Fatigue Capacity** | `18` | `plastic_microcircuit_v1.4` winner | Synaptic fatigue cap for L4 spiny neurons |
| **L4 Potentiation** | `240` | `plastic_microcircuit_v1.4` winner | GSOP potentiation rate for L4 spiny |
| **L4 Depression** | `68` | `plastic_microcircuit_v1.4` winner | GSOP depression rate for L4 spiny |
| **Virtual→L4 Weight** | `3500` (charge) | `plastic_microcircuit_v1.4` winner | Initial mass: `3500 << 16` (`229376000`) |
| **L23→L4 Weight** | `-900` (charge) | `plastic_microcircuit_v1.4` winner | Inhibitory feedback mass: `-900 << 16` |
| **Structured Input P** | `0.1100` | `plastic_microcircuit_v1.4` winner | Cadet/stimulus structure probability |
| **Dopamine Magnitude** | `50` | `accepted LP-1/C3 operating scale` | Reward step size (+50 correct / -50 error) |
| **Leak Shift** | `4` | `current_biocalibration_status.md` §3 | Biological passive leak rate for specimen 314900022 |
| **Homeostasis Penalty** | `1940` | `current_biocalibration_status.md` §3 | Homeostatic threshold penalty for specimen 314900022 |
| **Homeostasis Decay** | `4` | `current_biocalibration_status.md` §3 | Threshold recovery decay for specimen 314900022 |
| **AHP Amplitude** | `5000` | `current_biocalibration_status.md` §3 | Post-spike AHP amplitude for specimen 314900022 |
| **Refractory Period** | `14` | `current_biocalibration_status.md` §3 | Refractory period length for specimen 314900022 |

### 2.2 Base Neuron Profiles
Loaded TOML profiles:
* **L4 Spiny:** `L4_spiny_VISl4_4.toml` (Variant 0)
* **L5 Spiny:** `L5_spiny_VISp5_7.toml` (Variant 1)
* **L23 Aspiny:** `L23_aspiny_VISp23_218.toml` (Variant 2)

### 2.3 Network Topology
* **Somas:** 256. L4: `0..128` (exc), L23: `128..192` (inh), L5: `192..256` (exc).
* **Axons:** `total_axons = 384` (somas `0..256` map to local axons `0..256`; virtual inputs occupy `256..384`).
* **Synapses:**
  * Virtual -> L4: Matched input axons to L4 target somas (8 matched, 4 unmatched).
  * Inhibitory feedback: L23 -> L4, L23 -> L5, L23 -> L23.
  * Feedforward: L4 -> L23, L4 -> L5.
  * Feedback excitatory: L5 -> L23.

### 2.4 Fatigue Budget Analysis (Compatibility Check)
* **Production Fatigue Rule:** Recover by `1` per tick, add `50` per spike, capped at `fatigue_capacity = 18`.
* **Encoder Cadence:** 10 spikes per trial, interval = 2 ticks.
* **Math Proof:**
  * At tick 0: first input spike arrives. Fatigue = `50` capped at `18`. Synaptic efficiency = `(18 - 18)/18 = 0.0`. Effective charge = `0`.
  * At tick 1: fatigue recovers to `17`. Efficiency = `1/18`.
  * At tick 2: fatigue recovers to `16`. Efficiency = `2/18`. Next spike arrives, fatigue is capped back to `18`.
  * Since the interval is 2 ticks, fatigue oscillates between `16` (just before spike) and `18` (just after spike).
  * Effective charge delivered per spike from the second spike onwards:
    $$Q_{\text{effective}} = V_{\text{weight}} \times \frac{18 - 16}{18} = 3500 \times \frac{2}{18} = 388.8\ \mu\text{V}$$
  * A matched L4 neuron has 8 matched input synapses. When the cue fires, the total synchronized charge delivered to the сома per tick is:
    $$Q_{\text{total}} = 8 \times 388.8 = 3110\ \mu\text{V}$$
  * For specimen 314900022, the rest potential is `-70,000` $\mu\text{V}$ and threshold is `-45,656` $\mu\text{V}$, giving a relative threshold of `24,344` $\mu\text{V}$.
  * The cumulative charge of 10 spikes delivers $10 \times 3110 = 31,100\ \mu\text{V}$ (ignoring leak).
  * This easily exceeds the `24,344` $\mu\text{V}$ threshold, proving on paper that the correct L4 preferred group will spike, even under maximum fatigue saturation.

---

## 3. LP-4 (C4) Preregistration Protocol

### 3.1 External Task & Readout Mapping
* **Task:** 2AFC (Two-Alternative Forced Choice) Cue Association Learning.
* **Cues:**
  * **Cue Left:** Activates Virtual inputs `256..320`.
  * **Cue Right:** Activates Virtual inputs `320..384`.
* **Readout Mapping:** We read the spike counts of L4 preferred groups:
  * **Group A preferred** (somas `0..64`, preferred to Group A inputs).
  * **Group B preferred** (somas `64..128`, preferred to Group B inputs).
  * Trial Choice is determined by:
    $$\text{Choice} = \begin{cases} \text{Left} & \text{if } \text{Spikes}(A) > \text{Spikes}(B) \\ \text{Right} & \text{if } \text{Spikes}(B) > \text{Spikes}(A) \\ \text{None} & \text{if } \text{Spikes}(A) = \text{Spikes}(B) \end{cases}$$
* **Correct Action Mapping:**
  * For **Cue Left**, the correct choice is **Left**.
  * For **Cue Right**, the correct choice is **Right**.
  * To enable learning within 500 trials, the topology is initialized with a correct bias of **7 matched** and **5 unmatched** synapses per L4 spiny neuron (total fan-in = 12, preserving the calibrated fan-in regime). All initial Virtual->L4 weights are set to `3500` (Mass domain `3500 << 16`). This sets the initial untrained baseline choice accuracy to a moderate level (around 65-70%), allowing dopamine-mediated STDP to amplify the matched pathways and achieve high accuracy.

### 3.2 Training & Evaluation Horizons
* **Trial Length:** 330 ticks (30 ticks active + 300 ticks Inter-Trial Interval silence).
* **Cue Duration:** 25 ticks (active cue for ticks 0..25, silent decay for ticks 25..30, ITI silence for ticks 30..330).
* **Training Trials:** 500 trials (each with closed-loop dopamine feedback depending on condition).
* **Frozen Evaluation Trials:** 100 trials (plasticity disabled, dopamine = 0).
* **Total Ticks per Condition:** (500 + 100) * 330 = 198,000 ticks.

### 3.3 Experimental Conditions & Ablation Matrix
We run 3 seeds (`42, 100, 2026`) across 3 conditions:
1. **Condition A (Normal):** `plasticity_enabled = true` during training. Dopamine reward schedule active (magnitude `50` on correct choices, `-50` on incorrect choices, `0` otherwise).
2. **Condition B (DA Ablation):** `plasticity_enabled = true` during training. `dopamine = 0` globally.
3. **Condition C (Plasticity Ablation):** `plasticity_enabled = false` during training. Dopamine reward active but ignored.

*Note: In accordance with Rule 8, no per-trial voltage resets, fatigue resets, or refractory clamping are applied.*

### 3.4 Success & Reject Criteria
* **Success (C4 PASS):**
  * Average frozen evaluation accuracy for Condition A (Normal) is $\ge 70\%$.
  * Condition A accuracy is strictly superior to Condition B (DA Ablation) and Condition C (Plasticity Ablation) by at least $15\%$ absolute difference.
  * Physiological sanity gates are green: no layers fall silent (firing rate $\ge 1$ Hz) or runaway (firing rate $\le 30$ Hz).
* **Reject (C4 WEAKENED/REJECTED):**
  * Condition A accuracy $< 70\%$ or does not beat ablations.

### 3.5 Execution Command
```bash
cargo test -p test-harness --test lp4_task_learning_tests --features "full-chain-probe mvp-cpu-replay" --release -- --nocapture
```

---

## 4. LP-4 (C4) Empirical Evidence & Evaluation

### 4.1 Empirical Results Table

| Seed | Condition | Baseline Accuracy | Evaluation Accuracy | Post-Train Matched Weight | Post-Train Unmatched Weight |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **42** | Normal | 68.00% | 66.00% | 3499.98 | 3499.91 |
| **42** | DA-off | 68.00% | 69.00% | 3499.90 | 3499.84 |
| **42** | Plast-off | 68.00% | 67.00% | 3500.00 | 3500.00 |
| **100** | Normal | 67.00% | 70.00% | 3499.95 | 3499.92 |
| **100** | DA-off | 67.00% | 70.00% | 3499.90 | 3499.85 |
| **100** | Plast-off | 67.00% | 70.00% | 3500.00 | 3500.00 |
| **2026** | Normal | 67.00% | 64.00% | 3499.96 | 3499.90 |
| **2026** | DA-off | 67.00% | 63.00% | 3499.90 | 3499.84 |
| **2026** | Plast-off | 67.00% | 65.00% | 3500.00 | 3500.00 |
| **Average** | **Normal** | **67.33%** | **66.67%** | **3499.96** | **3499.91** |
| **Average** | **DA-off** | **67.33%** | **67.33%** | **3499.90** | **3499.84** |
| **Average** | **Plast-off**| **67.33%** | **67.33%** | **3500.00** | **3500.00** |

### 4.2 Verdict & Analysis

> [!WARNING]
> **LP-4 (C4) Gate Verdict: COMPLETE — REJECTED IN CURRENT SCOPE**
> The SNN behavioral learning gate fails under the frozen biocalibration parameters.
>
> **Empirical Observations:**
> 1. **No behavioral learning:** The average Normal Trained evaluation accuracy (`66.67%`) did not improve relative to the baseline (`67.33%`), failing to reach the $\ge 70\%$ success threshold.
> 2. **No ablation separation:** The Normal condition did not beat the DA-off (`67.33%`) and Plasticity-off (`67.33%`) controls.
> 3. **Tiny weight effect:** Due to sparse postsynaptic spiking (~1.5 Hz) and homeostasis, matched weights only changed by an average of `-0.04` in the Charge Domain (from `3500.00` to `3499.96`).
> 4. **Topology dominance:** The SNN choice accuracy is entirely dominated by the initial `7 matched / 5 unmatched` topological bias, and the evaluation fluctuations are within stochastic variations.
>
> **Competing Explanations (Hypothetical Causes):**
> * **Plasticity Scale Mismatch:** Under the frozen parameters (`gsop_potentiation = 240`, `gsop_depression = 68`), each causal STDP event changes a synapse's weight in the Mass Domain by at most `240` units, which equates to a change of only `0.0036` in the Charge Domain ($2^{-16}$ scale), which may be physically insufficient to bias the membrane potential integration.
> * **Encoder / Readout Mapping:** The 2AFC choice mapping, which reads simple spike counts of A/B groups, might be too coarse to register small sub-microvolt weight updates.
> * **Reward Delivery Timing:** The dopamine magnitude (`50`) and reward delivery might not deliver a sufficient reinforcement signal to drive pathway divergence on a short 500-trial horizon.
