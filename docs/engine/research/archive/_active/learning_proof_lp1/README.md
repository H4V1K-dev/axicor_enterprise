# LP-1: Plasticity Causality — Preregistration README

| Field | Value |
|---|---|
| **Status** | `completed` (C1 **SUPPORTED with caveats**) |
| **Slug** | `learning_proof_lp1` |
| **Started** | 2026-07-11 |
| **Completed** | 2026-07-11 |
| **Code** | uncommitted at review time: `lp1_causality_tests` on `main` tip + local WIP |
| **Parent Program** | [Learning Proof Program](../learning_proof_program/README.md) |

---

## 1. Research Question

Усиливает ли production GSOP/STDP коррелированный eligible-путь сильнее сопоставимого control-пути при фиксированной topology?

---

## 2. Hypothesis

1. **Correlation Bias ($\Delta w_{\text{correlated}} > \Delta w_{\text{control}}$):** При `plasticity_enabled = true` и наличии спайковой активности сомы, непрерывные пресинаптические спайки на коррелированном аксоне 0 (при отсутствии спайков на контрольном аксоне 1) приведут к тому, что вес коррелированного синапса сомы `w_correlated` станет алгебраически больше веса контрольного синапса `w_control`.
2. **Dale / Sign Invariance:** На протяжении всего прогона знаки синаптических весов не меняются (возбуждающие синапсы остаются строго положительными, сохраняя Dale's Law).
3. **Weight Bound Preservation:** Синаптические веса остаются строго в пределах `[MIN_WEIGHT_LIMIT, MAX_WEIGHT_LIMIT]`.

---

## 3. Primary Metrics & Hard Failure Gates

### Primary Metrics
- **Correlated pathway delta weight ($\Delta w_{\text{correlated}}$):** Алгебраическое изменение веса синапса сомы 0 на аксоне 0 за 100 тиков.
- **Control pathway delta weight ($\Delta w_{\text{control}}$):** Алгебраическое изменение веса синапса сомы 0 на аксоне 1 за 100 тиков.
- **Absolute weight values ($w_{\text{correlated}}, w_{\text{control}}$):** Финальные значения весов в масс-домене (`i32`) и в exact-заряде (`weight >> 16`).

### Hard Failure Gates
- Любое изменение знака веса (возбуждающий синапс $\le 0$).
- Выход за пределы физических ограничений весов (или возникновение `NaN`).
- `$\Delta w_{\text{correlated}} \le \Delta w_{\text{control}}$` хотя бы на одном из тестируемых семян.

---

## 4. Success and Reject Thresholds

### Success Criteria
- **Pass (C1 — Local Causality):**
  - На всех пререгистрированных семенах (`42, 100, 2026`) по окончании 100 тиков симуляции выполняется условие `$\Delta w_{\text{correlated}} > \Delta w_{\text{control}}$`.
  - Все hard failure gates успешно пройдены (знаки сохранены, веса в рамках лимитов).

### Reject Criteria
- Вес контрольного синапса увеличивается быстрее или равен весу коррелированного хотя бы на одном семени.
- Нарушен закон Дейла или границы весов.

---

## 5. Network Preset & Configuration

### Minimal Fixed Preset
Для обеспечения полной контролируемости используется синтетическая сеть малого размера:

- **Размер сети:** `padded_n = 4`, `total_axons = 4`.
- **Профили нейронов:** Стандартный GLIF + GSOP параметры с `d1_affinity: 100`, `d2_affinity: 100`, `gsop_potentiation: 100`, `gsop_depression: 10`.
- **Начальные веса:** `initial_synapse_weight = 100 << 16` (`6553600`) для активных дендритных слотов (слот 0 и слот 1 сомы 0).
- **Количество дендритных слотов:** 128 слотов на сому.

### Horizon & Seeds
- **Tick Horizon:** 100 тиков (5 последовательных Day-батчей по 20 тиков).
- **Seeds:** `[42, 100, 2026]`.

### Mutation Isolation Table

| Property | Setting |
|---|---|
| Night prune/sprout/compact | **OFF** |
| Topology | Fixed (defined once at start) |
| Plasticity path under study | Weight plasticity |
| `plasticity_enabled` | `true` during training |

---

## 6. Execution Command

```bash
# Запуск тестов проверки причинно-следственной связи пластичности весов
cargo test -p test-harness --test lp1_causality_tests --features full-chain-probe -- --nocapture
```

---

## 7. Results & Observed deltas

| Seed | Initial Weight (mass) | Final Correlated Weight (mass) | Correlated Delta ($\Delta w_c$) | Final Control Weight (mass) | Control Delta ($\Delta w_{ctrl}$) | Verdict |
|---|---|---|---|---|---|---|
| **42** | `6553600` | `6555263` | `+1663` | `6554192` | `+592` | **PASS** |
| **100** | `6553600` | `6555263` | `+1663` | `6554192` | `+592` | **PASS** |
| **2026** | `6553600` | `6555263` | `+1663` | `6554192` | `+592` | **PASS** |

---

## 8. Verdict

### **SUPPORTED (C1 local / toy fixture) — with caveats**

**What is supported:** under this minimal fixed-topology harness, mass-domain $\Delta w_{\text{correlated}} > \Delta w_{\text{control}}$ holds on all three listed seeds with Dale/bounds green. Production Stage-6 GSOP path is exercised (`plasticity_enabled = true`, mass with non-zero charge).

**Caveats (do not over-claim):**

1. **Pseudo multi-seed.** All three seeds produce **identical** numeric deltas (`+1663` vs `+592`). Bake seed does not diversify the fixture (two hand-written synapses + fully deterministic incoming schedule). This is three repeats of the same dynamics, not independent network draws.
2. **Control is not silent.** Control still receives **positive** $\Delta w = +592$ despite **no** `incoming_spikes` on axon 1. Residual potentiation likely comes from other baker/spontaneous activity touching axon heads and/or GSOP geometry when the postsynaptic soma fires. C1 claim is **relative bias**, not “only correlated path changes.”
3. **Toy scale.** `padded_n≈4`, two dendrite slots, spontaneous postsynaptic drive — not L4/L5 microcircuit causality and not task learning.
4. **Charge-domain deltas not asserted.** Final charge remains `100` for both arms (`>> 16`); learning is proven in mass domain only at this horizon.
5. **Not program SUPPORTED.** Monospec program-level SUPPORTED still requires retention + reward + task (LP-2…LP-4).

### One Next Action
- **LP-2: Retention** — freeze after training (`plasticity_enabled = false`) and show correlated/control structure and behavior hold without further weight updates.

