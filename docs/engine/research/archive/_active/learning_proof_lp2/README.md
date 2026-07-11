# LP-2: Retention after freeze — Preregistration README

| Field | Value |
|---|---|
| **Status** | `completed` |
| **Slug** | `learning_proof_lp2` |
| **Started** | 2026-07-11 |
| **Completed** | 2026-07-11 |
| **Parent Program** | [Learning Proof Program](../learning_proof_program/README.md) |

---

## 1. Research Question

Сохраняется ли приобретённое изменение весов (и относительный corr>ctrl bias) после отключения пластичности?

---

## 2. Hypothesis

1. **Weight Permanence ($\Delta w_{\text{eval}} = 0$):** После переключения `plasticity_enabled = false` при продолжении симуляции (хоть под стимулом, хоть без) веса синапсов `w_correlated` и `w_control` остаются абсолютно побитово инвариантными, а их дельта во время фазы тестирования равна нулю ($\Delta w = 0$).
2. **Relative Bias Preservation:** Выработанная в фазе обучения разница в весах сохраняется, то есть `w_correlated` на фазе тестирования остается строго больше, чем `w_control` ($w_{\text{correlated}} > w_{\text{control}}$).

---

## 3. Primary Metrics & Hard Failure Gates

### Primary Metrics
- **Evaluation phase weight delta ($\Delta w_{\text{eval}}$):** Изменение весов во время фазы тестирования (100 тиков) с отключенной пластичностью.
- **Correlated vs Control final weight comparison:** Финальные значения весов в конце фазы тестирования.
- **Dendrite weight plane checksum:** Контрольная сумма FNV-1a, взятая до и после фазы тестирования.

### Hard Failure Gates
- Любое изменение весов во время фазы тестирования ($\Delta w_{\text{eval}} \neq 0$, или несовпадение контрольных сумм).
- Нарушение закона Дейла.
- `$w_{\text{correlated}} \le w_{\text{control}}$` в конце фазы тестирования.

---

## 4. Success and Reject Thresholds

### Success Criteria
- **Pass (C2 — Retention):**
  - На всех семенах (`42, 100, 2026`) по окончании фазы тестирования выполняется `$\Delta w_{\text{eval}} = 0$` и `$w_{\text{correlated}} > w_{\text{control}}$`.
  - Все инварианты и лимиты весов соблюдены.

### Reject Criteria
- Веса меняются во время фазы тестирования.
- Относительный bias пропадает или переворачивается.

---

## 5. Network Preset & Configuration

### Minimal Fixed Preset
- Точно такая же сеть, как в LP-1 (`padded_n = 4`, `total_axons = 4`, standard GLIF + GSOP).
- Слот 0 сомы 0 привязан к `axon 0`, слот 1 привязан к `axon 1`.
- Начальный вес: `100 << 16` (`6553600`).

### Simulation Phases
1. **Training Phase:** `plasticity_enabled = true` на 100 тиков (5 Day-батчей по 20 тиков). Спайки идут непрерывно на аксоне 0, аксон 1 молчит.
2. **Freeze & Sync:** `plasticity_enabled` меняется на `false`. Запись контрольной суммы весов.
3. **Evaluation Phase:** `plasticity_enabled = false` еще на 100 тиков (5 Day-батчей по 20 тиков) при тех же входящих стимулах.

---

## 6. Execution Command

```bash
cargo test -p test-harness --test lp2_retention_tests --features full-chain-probe -- --nocapture
```

---

## 7. Results & Observed deltas

| Seed | Checksum (Trained) | Checksum (Eval) | Correlated final weight | Control final weight | Delta during Eval ($\Delta w_{\text{eval}}$) | Verdict |
|---|---|---|---|---|---|---|
| **42** | `83988a621033a790` | `83988a621033a790` | `6555263` | `6554192` | `0` | **PASS** |
| **100** | `248ea522566c85c0` | `248ea522566c85c0` | `6555263` | `6554192` | `0` | **PASS** |
| **2026** | `95f6bb77c3a4e77a` | `95f6bb77c3a4e77a` | `6555263` | `6554192` | `0` | **PASS** |

---

## 8. Verdict

### **SUPPORTED**
- По окончании 100 тиков фазы тестирования с `plasticity_enabled = false` веса не претерпели никаких изменений ($\Delta w_{\text{eval}} = 0$).
- Контрольные суммы до и после фазы оценки совпали побитово.
- Относительный bias полностью сохранен ($w_{\text{correlated}} > w_{\text{control}}$).

### One Next Action
- Перейти к **LP-3: Reward Ablations** (проверка влияния дофамина/reward-causality на результаты потенциирования).
