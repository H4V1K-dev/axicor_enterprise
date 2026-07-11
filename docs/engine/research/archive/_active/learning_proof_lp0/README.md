# LP-0: Frozen / Plasticity Controllability — Preregistration README

| Field | Value |
|---|---|
| **Status** | `running` (C0 partial: harness + gate implemented; multi-seed protocol not closed) |
| **Slug** | `learning_proof_lp0` |
| **Started** | 2026-07-11 |
| **Freeze Commit** | `b904a9255ca715d974f6dde50311c4e02a655909` (pre-LP0 code; LP0 implementation still uncommitted) |
| **Parent Program** | [Learning Proof Program](../learning_proof_program/README.md) |

---

## 1. Research Question

Можем ли мы гарантированно включать и выключать изменение синаптических весов в Day-фазе с помощью флага `plasticity_enabled`, не изменяя при этом электрическую динамику симуляции?

---

## 2. Hypothesis

1. **Frozen Isolation:** При установке `plasticity_enabled = false` контрольная сумма весов SoA-массивов (`dendrite_weights`) до и после Day-фазы совпадает побитово ($\Delta w = 0$).
2. **Plastic Integrity:** При установке `plasticity_enabled = true` под воздействием стимулирующей активности веса SoA-массива изменяются ($\Delta w \neq 0$) в соответствии с правилами GSOP/STDP.
3. **Dynamic Equivalence:** Включение или выключение флага `plasticity_enabled` не оказывает влияния на электрические процессы в сети: мембранный потенциал ($V_m$) и выходная спайковая активность ($Spikes$) при одинаковом входном паттерне и начальном состоянии сети совпадают побитово.

---

## 3. Primary Metric & Hard Failure Gates

### Primary Metrics
- **Weight Checksum Delta ($\Delta w$):** Побитовое сравнение вектора синаптических весов `dendrite_weights` до и после симуляции.
- **Electrical Parity ($\Delta V_m, \Delta Spikes$):** Разница в значениях мембранного потенциала и выходных спайков между прогоном с пластичностью и прогоном без пластичности при одинаковых начальных условиях.

### Hard Failure Gates
- Любое изменение весов ($\Delta w \neq 0$) при `plasticity_enabled = false`.
- Любое несовпадение электрической динамики ($\Delta V_m \neq 0$ или $\Delta Spikes \neq 0$) между прогоном с пластичностью и без нее.
- Любой недетерминизм между повторными прогонами с одним и тем же сидом.
- Наличие `NaN` или некорректных значений в SoA-массивах.

---

## 4. Success and Reject Thresholds

### Success Criteria
- **Pass (C0 — Controllability):** 
  - На всех тестовых сидах при `plasticity_enabled = false` изменение весов равно ровно нулю.
  - На всех тестовых сидах при `plasticity_enabled = true` изменение весов строго ненулевое.
  - Спайковые паттерны и значения потенциалов мембран полностью идентичны в обоих режимах (с точностью до бита).

### Reject Criteria
- Веса дрейфуют или обновляются при выключенной пластичности.
- Включение пластичности изменяет спайковые тайминги без прямого накопления изменений в весах (т.е. электрическая эквивалентность нарушена в первом же тике до изменения весов).

---

## 5. Network Preset & Configuration

### Minimal Fixed Preset
Для обеспечения полной контролируемости используется синтетическая сеть малого размера:

- **Размер сети:** `padded_n = 4`, `total_axons = 4`.
- **Профили нейронов:** Стандартный GLIF + GSOP параметры с ненулевыми константами потенциации/депрессии.
- **Начальные веса:** `initial_synapse_weight = 100` для всех активных дендритных слотов.
- **Количество дендритных слотов:** 128 слотов на сому.

### Horizon & Seeds
- **Tick Horizon:** 1000 тиков (5 последовательных Day-батчей по 200 тиков).
- **Seeds:** `[42, 100, 2026, 9999]`.

### Mutation Isolation
- **Night Phase structural mutations:** Pruning, sprouting, compaction полностью **выключены** (Night Phase OFF).
- Тестируется исключительно изолированный Day-pathway для синаптической пластичности.

---

## 6. Implementation (L001/L002) — current evidence

### Commands

```bash
cd AxiEngine
cargo test -p test-harness --test lp0_controllability_tests --features full-chain-probe
cargo test -p test-harness --features night-gates --test night_phase_vertical_slice
```

### What the harness actually proves today

| Claim | Status | Notes |
|---|---|---|
| Frozen checksum hold (`false`) | **PASS** (1 seed, 20 ticks) | FNV-1a over `dendrite_weights` plane |
| Plastic weights can change (`true`) | **PASS** (same fixture) | Mass-domain weights change under stimulus |
| Electrical bit-parity under flag flip | **WEAK / vacuous** | Fixture uses mass `50000` → charge `weight >> 16 == 0`, so Vm cannot feel weight delta |
| Multi-seed protocol §5 seeds | **NOT RUN** | README listed `[42,100,2026,9999]` / 1000 ticks — test uses seed bake 42, 20 ticks |
| Spikes parity explicit assert | **NOT ASSERTED** | Only voltage range compared |

### Design notes / debts

1. **Global `AtomicBool` in `physics`** (`set_plasticity_enabled` / `is_plasticity_enabled`) — process-wide; OK for single-runtime LP-0, risky for concurrent runtimes / direct `run_day_batch` without runtime.
2. Prefer eventual `DayBatchCmd.plasticity_enabled` (or equivalent) for thread-local correctness; not blocking LP-0.
3. CUDA path does not honor the flag (out of LP-0 scope; CPU-first).
4. `engine_mut` / `working_state_mut` exposed on `LocalRuntime` for harness — test surface, not product modes API.

### Honest LP-0 gate status

- **C0 controllability (minimal):** **PASS with caveats** — freeze/unfreeze of weight mass writes works on CPU via runtime config.
- **Do not claim full §5 preregistration closed** until multi-seed run set + non-zero charge electrical parity (or explicit acceptance that charge-domain parity is deferred to LP-1 fixture).
- **Next:** LP-1 causality fixture (correlated vs control), **not** Night sprout planner.

