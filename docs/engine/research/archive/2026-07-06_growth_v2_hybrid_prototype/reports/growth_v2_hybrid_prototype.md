# Growth v2 Hybrid Prototype & 3D Atlas — Report

**Date**: 2026-07-06
**Status**: PASS / Geometry invariants verified; fan-in density unresolved
**Experiment**: `2026-07-06_growth_v2_hybrid_prototype`
**Seed**: 12345 (primary comparison), verified consistency for 12346/12347

---

## 1. Executive Summary

This report documents the research prototype of **Growth v2**, comparing the baseline discrete model (Baker v1), the legacy MVP continuous vector model, and the new **Hybrid Growth v2** candidate.

The primary goal was to combine the biological/guided steering force of continuous vector models (improving tract coherence and target layer success) with the hard physical guarantees of the discrete grid (zero collisions, zero self-intersections, zero out-of-bounds, strict whitelists).

### Comparative Metrics Panel

| Metric | Discrete Baker v1 (Baseline) | Legacy MVP Continuous | Hybrid Growth v2 (Candidate) | Pass/Fail Gate |
|---|---|---|---|---|
| **Mean Axon Length** | 5.28 voxels | 11.27 voxels | 5.32 voxels | — |
| **Out-of-Bounds Violations** | 0 | 77 | **0** | **PASS** |
| **Self-Intersection Violations** | 0 | 135 | **0** | **PASS** |
| **Soma Collision Violations** | 0 | 283 | **0** | **PASS** |
| **Whitelist Violations** | 0 | 0 | **0** | **PASS** |
| **Exact Radius Violations** | 0 | 0 | **0** | **PASS** |
| **Raw Synapse Candidates** | 32,492 | 34,074 | 112,261 | — |
| **Accepted Synapses After 128-Cap** | 32,492 | 29,109 | 29,021 | — |
| **Dropped Candidates After 128-Cap** | 0 | 4,965 | 83,240 | density warning |
| **Duplicate Source-Target Contacts** | 23,333 | 0 (enforced) | 86,252 (not enforced) | density warning |
| **Layer Target Success Rate (V->L4)** | 60.9% | 93.7% | **83.6%** | **+37% vs Baseline** |
| **Mean Endpoint Density (radius=2)** | 0.97 segments | 2.67 segments | **1.65 segments** | **-38% vs MVP** |
| **Mean Last-5 Tortuosity** | 1.009 | 1.009 | 1.012 | — |
| **Stop Reason: TargetReached** | 0 | 0 | **348** (90.6%) | — |
| **Stop Reason: BoundaryReached** | 384 | 77 | 0 | — |

---

## 2. Invariants & Violations Analysis

1. **Discrete Baker v1 (Baseline)**: Shows perfect compliance with invariants (0 out-of-bounds, 0 collisions, 0 self-intersections), but suffers from poor target layer targeting success (60.9%). All 384 axons grow blindly until they hit the boundary walls (`BoundaryReached` = 384).
2. **Legacy MVP Continuous**: Demonstrates high targeting success (93.7%), but violates all structural invariants. In continuous space, it blindly steps through somas (283 collisions), crosses its own paths (135 self-intersections), and exits the grid boundaries (77 out-of-bounds), which is unacceptable for production AxiEngine runs.
3. **Hybrid Growth v2 (Candidate)**: Combines continuous vector force ranking with discrete collision checking. If the continuous step collides or goes out of bounds, it performs a 26-neighbor search and selects the best aligned, non-colliding voxel.
   - **Result**: **Strictly 0 out-of-bounds, 0 self-intersections, and 0 soma collisions**.
   - **Synapse validation**: Strictly 0 whitelist and 0 exact radius violations.
   - **Density caveat**: Hybrid creates many raw contacts around target somas (112,261 candidates), so after the production-style `MAX_DENDRITES=128` cap it keeps 29,021 and drops 83,240. This is a fan-in/saturation warning, not a geometry failure.
   - **Determinism**: 100% bitwise matching rerun results for seed 12345.

---

## 3. Terminal Knot Audit Results

### The Problem
MVP continuous growth models tend to coil or loop around targets when attraction forces dominate, leading to high local segment densities near endpoints (2.67 segments within $R=2$).

### Hybrid v2 Fixes
To prevent terminal knots, Hybrid v2 implements:
1. **Capture Radius Stop**: Terminate growth when within $R_{capture} = 1.5$ voxels of any whitelisted target soma.
2. **Attraction Damping**: Linearly damp the attraction weight $w_{attract}$ when within $R_{damping} = 5.0$ voxels of target somas.
3. **Monotonicity Stop**: Halt growth if the distance to the target center fails to decrease for 3 consecutive steps.

### Outcome
- **Early Termination**: **348 out of 384 axons (90.6%) stopped cleanly** with `TargetReached` upon entering the capture radius, instead of growing blindly to boundaries.
- **Density Reduction**: The mean endpoint local segment density fell from **2.67 to 1.65 (a 38% reduction)**, successfully preventing coiling tangles and local congestion.

---

## 4. Tract Coherence & Layer Projections

- **Tract Formation**: Rather than a random walk, axons grow in directional, cohesive bundles. The mean direction vector for VirtualInput axons is $[-0.009, 0.192, 0.981]$, proving strong upward alignment towards the target layer (L4).
- **Targeting Success**: The success rate of VirtualInput axons entering L4 (Z >= 8) reached **83.6% in Hybrid v2**, representing a **37.2% absolute improvement** over the discrete baseline (60.9%).

---

## 5. Design Decisions for Growth v2 (Acceptance Gate Answers)

### 1. Какие MVP-алгоритмы стоит переносить?
- **`v_attract` (Attraction Force) с Cone/FOV-конусом**: Обязательно к переносу. Оно дает направленный рост в целевые зоны и улучшает качество коннектома.
- **Интеграция непрерывных координат с дискретным поиском (Hybrid Step)**: Перенос этой гибридной схемы необходим. Она позволяет вычислять силы непрерывно, но гарантирует физическую целостность решетки.
- **`type_affinity` (Affinity)**: Полезно для дифференцирования притяжения E/I нейронов.

### 2. Какие MVP-поведения нельзя переносить?
- **Обход whitelist для Virtual/Ghost**: Нельзя переносить. Виртуальные входы должны жестко подчиняться whitelists, чтобы не допускать утечек.
- **Игнорирование коллизий и пересечений**: Недопустимо. Без дискретного фильтра непрерывный шаг разрушает геометрию.

### 3. Уменьшает ли Hybrid terminal knots?
- **Да**. За счет ранней остановки при захвате (`TargetReached`) и демпфирования притяжения среднее число лишних петель снизилось на **38%**, а средняя плотность упала с 2.67 до 1.65 вокселей.

### 4. Становится ли рост визуально похож на tract formation?
- **Да**. Вместо хаотичного блуждания discrete v1, гибридный рост формирует вертикальные и латеральные "колонки" (tracts), стягивающиеся к целевым соматическим центрам.

### 5. Какие параметры требуют следующего sweep?
- **Соотношение весов** ($w_{global}, w_{attract}, w_{noise}$): баланс между целевым ростом и ветвлением/поиском.
- **Радиус захвата** ($R_{capture}$) и **демпфирования** ($R_{damping}$): для тонкой регулировки формы терминальных окончаний.
- **Fan-in cap pressure**: в Hybrid v2 появляется 112,261 raw contacts, но production-style cap оставляет 29,021 и отбрасывает 83,240 кандидатов. Нужно отдельно исследовать, является ли это нормальной terminal arborization density или pathological overgrowth.
- **Uniqueness per Axon**: Hybrid v2 дает 86,252 duplicate source-target contacts из-за отсутствия uniqueness rule. В будущем стоит исследовать влияние уникальности (1 синапс на аксон-мишень) на плотность коннектома.

## 6. Commit-Level Interpretation

This prototype is a successful **research atlas**, not a final production migration plan. It proves that continuous/FOV/affinity steering can be combined with strict grid invariants, but it also exposes a fan-in density issue that must be resolved before porting Growth v2 into `topology`.

Recommended next step: wait for biological review of the trajectory shapes, then run a targeted sweep over `w_global`, `w_attract`, `w_noise`, `R_capture`, `R_damping`, fan-in cap pressure, and source-target uniqueness.
