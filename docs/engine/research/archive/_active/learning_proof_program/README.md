# Learning Proof Program — Entry Dossier (L000)

| Field | Value |
|---|---|
| **Status** | `active` (LP-0 preregistered; active experiment running) |
| **Slug** | `learning_proof_program` |
| **Started** | 2026-07-11 |
| **Program SoT** | `artifacts/agent-tasks/LEARNING_PROOF_MONOSPEC.md` |
| **Rules** | `docs/engine/research/RULES.md` |
| **Purpose** | Kill-or-continue gate: can AxiEngine learn and retain useful change? |

This folder is the living lab note for the Learning Proof program. Detailed LP experiments get their own `_active/<lp_slug>/` when registered (L001+).

### Active Experiments
- **[LP-0: Frozen / Plasticity Controllability](../learning_proof_lp0/README.md)**


---

## 1. Research question (program-level)

Can a small AxiEngine network, after a bounded plasticity window, solve a pre-registered task better than its untrained baseline and keep that improvement with plasticity off?

Milestone ends with one honest verdict: **SUPPORTED** / **WEAKENED** / **REJECTED IN CURRENT SCOPE**.

This is a project-level evidence gate, not an infrastructure epic.

---

## 2. Entry gate checklist (L000)

| Requirement | Verdict | Evidence |
|---|---|---|
| Local in-proc Night v1 done (agreed scope) | **PASS** | R1–R8 landed; design rev 1.5 |
| Phase 6/8 topology plans; Phase 7 not fixed-target stub | **PASS** | `plan_sprouts` + weaver Phase 7 apply (R6) |
| Local Night invariants / determinism gates green | **PASS** | `night-gates` 9/9 @ freeze commit |
| Day CPU path reproducible on fixed seed | **PASS** (baseline suite) | mock harness + CPU backend + physics GSOP unit tests |
| Stable small-network preset known | **ACCEPTED for LP-0** | minimal harness preset (see §5); not full L4/L5 showcase |
| Static L4/L5 balance (research step 2.4) | **EXPLICITLY ACCEPTED AS DEBT** | open; non-blocker for LP-0…LP-2 on minimal preset |
| AxiEngine baseline tests green | **PASS** | freeze commands below |

**Not blockers (parked):** process SHM (R9), CUDA/HIP Night, auto `night_interval`, distributed ghosts, production checkpoint format.

---

## 3. Night freeze baseline

| Field | Value |
|---|---|
| **Freeze commit** | `b904a9255ca715d974f6dde50311c4e02a655909` (`b904a92`) |
| **Message** | `feat: add test-harness crate with night phase vertical slice integration tests` |
| **Scope closed** | in-proc Day→Maintenance→Night→import→Day; HostWorkingState; Faulted; prune gate; plan_sprouts; G-POISON/RO/DET/DENSE; G-DALE skip honest |
| **Design SoT** | `artifacts/agent-tasks/DESIGN_NIGHT_PHASE.md` rev 1.5 |

### Night gates (re-run at freeze)

```text
cd AxiEngine
cargo test -p test-harness --features night-gates --test night_phase_vertical_slice
```

Result @ freeze: **9 passed** (4 base slice + 5 gates including G-DALE skip).

### Known Night limitations (do not scope-creep into LP-0…LP-4)

1. G-RO/DET primarily with `growth_context: None` (synthetic empty-path geometry).
2. G-DALE is explicit skip / always-pass honesty, not full Dale enforcement.
3. Process SHM Night (R9) not implemented.
4. No production checkpoint format.
5. **LP-0…LP-4 must keep Night structural mutation OFF** (causal isolation).

---

## 4. Day / compute baseline

### Commands recorded at freeze

```text
cd AxiEngine

# Default harness matrix (mock)
cargo test -p test-harness --features mock
# → harness_tests: 18 passed

# Night vertical slice + biology gates
cargo test -p test-harness --features night-gates --test night_phase_vertical_slice
# → 9 passed

# Production GSOP math (unit)
cargo test -p physics --test physics_tests
# → includes test_gsop_math_comprehensive, test_stdp_golden_matrix_comprehensive, …

# CPU backend smoke
cargo test -p compute-cpu
```

### Day path notes

- Hot path: `compute-cpu` Stage 6 always invokes `physics::apply_gsop_plasticity` on spiking postsynaptic somas when dendrite slots are active.
- Global neuromod: `DayBatchCmd.dopamine: i16`.
- There is **no** production `plasticity_enabled` flag yet (LP-0 / L002 work).
- Soft-freeze used in static microcircuit research: `gsop_potentiation = 0`, `gsop_depression = 0` (still runs Stage 6; zero pulses → no mass change if dopamine mods also yield zero).

---

## 5. Network preset policy (LP-0 anchor)

### Decision (L000)

**First learning anchor is NOT** the full L4/L2-3/L5 balance-optimized microcircuit (research step 2.4).

**First anchor is a minimal fixed harness preset** sized for controllability:

| Property | LP-0 requirement |
|---|---|
| Topology | fixed between frozen and plastic runs |
| Size | small (order tens of somas / few axons; exact N in L001 README) |
| Profiles | production GLIF + GSOP params with non-zero pot/dep when plastic |
| Stimulus | deterministic seedable pattern (exact generator in L001) |
| Night prune/sprout | **disabled** |
| Encoder/readout | optional for LP-0; required from LP-2/LP-4 as needed |

Full microcircuit physiology (N=64…512, L4/L5 rates) remains a **separate biology ladder**. It may later supply a stronger LP-1/LP-4 preset only after 2.4 closes or a subset is re-accepted.

### Stability evidence available today

| Source | What it shows | Limit |
|---|---|---|
| Static microcircuit physiology v1 (N=64) | no silence/runaway under static pot=0 | not a plastic learning claim |
| v1.1 / v1.2 | Vm health improved; L5 recruitment partial; L4 over-inhibited on some configs | step 2.4 open |
| Physics GSOP unit tests | rule math + golden STDP matrices | not network task learning |

---

## 6. Plasticity inventory (production)

| Mechanism | Location | Mutates weights? | Controllable today? |
|---|---|---|---|
| **GSOP / All-to-All Spatial STDP** | `physics::apply_gsop_plasticity` (`crates/physics/src/gsop.rs`) | yes | params only (pot/dep, DA, affinities, inertia) |
| **Stage 6 call site** | `compute-cpu::simulation` Stage 6 | yes (writes `dendrite_weights`) | **no master off switch** |
| **Dopamine modulation** | `DayBatchCmd.dopamine` → pot/dep mods via D1/D2 | scales LTP/LTD | yes per batch |
| **Fatigue (charge path)** | `apply_synaptic_fatigue` / timers | attenuates effective charge; not durable mass rule alone | always-on physiology |
| **Night prune** | topology + weaver Phase 6/apply | removes / zeros slots | must stay **off** for LP-0…4 |
| **Night sprout** | `plan_sprouts` + Phase 7 | new connections | must stay **off** for LP-0…4 |
| **CUDA GSOP** | `compute-cuda` probe/native | yes (parity path) | out of LP-0 scope; CPU first |

### Weight plane

- Storage: `dendrite_weights` SoA plane (`i32` mass domain) inside `.state` blob.
- Mass→charge: `weight >> 16` (`weight_to_charge`).
- Bounds: `MIN_WEIGHT_LIMIT` / `MAX_WEIGHT_LIMIT` in `physics::constants`.
- Snapshot: `ShardSnapshotMut` / maintenance export for checksum candidates.

### Missing for LP-0 (intentional L002 work)

1. Explicit `plasticity_enabled: bool` (or equivalent) that skips weight writes without changing electrical dynamics.
2. Reproducible weight checksum helper (harness-level OK).
3. Counters: LTP/LTD/no-change hits, saturation/floor (may be experiment-only instrumentation).

---

## 7. Observability inventory

| Signal | Available without semantic change? | Notes |
|---|---|---|
| Output spikes / counts | yes | `DayBatchCmd` output buffers + `BatchResult` |
| Generated / dropped spike counts | yes | `BatchResult` |
| Full state / axons snapshot | yes | `debug_snapshot` / maintenance export |
| Dendrite weights plane | yes via snapshot parse | no first-class API checksum yet |
| Dopamine command value | yes | caller-controlled per batch |
| Firing rates / rasters | research harness only | microcircuit runners in `full_neuron_replay.rs` |
| LTP/LTD event counters | **no** | add in L002 as non-semantic counters if needed |
| Reward timeline | **no product API** | LP-3 experiment must define delivery |
| Provenance (seed, commit, flags) | partial | must be mandatory in LP README + run manifest |

---

## 8. Known debts (accepted; not accidental scope)

| ID | Debt | Handling |
|---|---|---|
| **D-2.4** | L4/L5 static balance not closed | Accepted for LP-0…LP-2 on minimal preset; reopen before claiming full-microcircuit plastic biology |
| **D-NO-FLAG** | No production plasticity master switch | L002 delivers minimal gate |
| **D-CKPT** | No production trained-weight checkpoint | LP-2 may use harness snapshot only |
| **D-R9** | Process SHM Night | Parked; not needed for learning proof |
| **D-DALE** | G-DALE skip | Night claim only; LP checks sign non-flip as hard gate |
| **D-PATH-GATES** | G-RO/DET weak with empty growth paths | Night polish; irrelevant while structural Night off |
| **D-CUDA** | Learning proof is CPU-first | CUDA parity later |
| **D-MODES** | No FrozenInference/Training runtime modes | Forbidden until LP-4+ positive evidence |

---

## 9. Program state machine

```text
DORMANT  →  READY (this dossier, 2026-07-11)
         →  ACTIVE (exactly one LP; start with LP-0 after L001)
```

| Next task | Result |
|---|---|
| **L001** | Register LP-0 in index + `_active/learning_proof_lp0/README.md` preregistration |
| **L002** | `plasticity_enabled` + weight checksum instrumentation |
| **L003** | LP-0 run set + C0 verdict |

---

## 10. What would confirm / weaken / reject (program-level)

| Outcome | Meaning for the project |
|---|---|
| **SUPPORTED** (through C3–C4) | Core learning hypothesis alive → continue engine + later modes / LP-5 Night structural |
| **WEAKENED** | Weights move but no durable/task/reward win → stop product modes; open narrow mechanism research |
| **REJECTED IN CURRENT SCOPE** | No useful learning under fixed mechanisms → honest stop/pivot beats more infrastructure |

**Hard rule:** LP-0/1 alone do not “save” the project. Project-level continue needs reward/task evidence (LP-3/LP-4). LP-0 only proves the harness can freeze and unfreeze weight change.

---

## 11. Explicit non-goals until verdict

- R9 process SHM Night
- CUDA Night / autosched / production checkpoint
- Runtime Training/Inference mode architecture
- CartPole before LP-3
- Mixing Night prune/sprout into LP-0…LP-4
- Closing biology step 2.4 as a side quest unless LP preset depends on it
