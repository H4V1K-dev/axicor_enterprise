use physics::*;
use static_assertions::const_assert_eq;
use types::AXON_SENTINEL;

// Static assertions for fundamental physical constants
const_assert_eq!(MASS_TO_CHARGE_SHIFT, 16);
const_assert_eq!(MIN_WEIGHT_LIMIT, 1);
const_assert_eq!(MAX_WEIGHT_LIMIT, 2_140_000_000);
const_assert_eq!(INERTIA_RANK_SHIFT, 28);
const_assert_eq!(MAX_INERTIA_RANK, 7);
const_assert_eq!(HEARTBEAT_PHASE_MOD, 65_536);
const_assert_eq!(HEARTBEAT_PHASE_MASK, 0xFFFF);
const_assert_eq!(HEARTBEAT_SCATTER_PRIME, 104_729);
const_assert_eq!(MAX_HEARTBEAT_M, 65_535);

#[test]
fn test_compute_v_seg_matrix() {
    // Valid integer velocities v_seg = 1, 2, 5
    assert_eq!(compute_v_seg(0.1, 1000, 20.0, 5).unwrap(), 1);
    assert_eq!(compute_v_seg(0.2, 1000, 20.0, 5).unwrap(), 2);
    assert_eq!(compute_v_seg(0.5, 1000, 20.0, 5).unwrap(), 5);

    // Fractional 1.4 -> Err (e.g. voxel_size_um = 15.0 yields 18.666...)
    assert!(compute_v_seg(1.4, 1000, 15.0, 5).is_err());

    // Zero segment_length_voxels -> Err
    assert!(compute_v_seg(1.0, 1000, 20.0, 0).is_err());

    // Exact v_seg = 256 -> Err (SegmentVelocityOutOfBounds)
    assert_eq!(
        compute_v_seg(25.6, 1000, 100.0, 1),
        Err(PhysicsError::SegmentVelocityOutOfBounds)
    );
}

#[test]
fn test_stochastic_heartbeat_matrix() {
    // period 65536 -> 1, period 65537 -> 0
    assert_eq!(compile_stochastic_heartbeat_threshold(65536), 1);
    assert_eq!(compile_stochastic_heartbeat_threshold(65537), 0);

    // period == 0 or > 65536 -> heartbeat disabled (m = 0)
    assert_eq!(compile_stochastic_heartbeat_threshold(0), 0);
    assert_eq!(compile_stochastic_heartbeat_threshold(100000), 0);

    // period == 1 -> m = MAX_HEARTBEAT_M (65535)
    assert_eq!(compile_stochastic_heartbeat_threshold(1), MAX_HEARTBEAT_M);

    // period = 500 -> 65536 / 500 = 131
    assert_eq!(compile_stochastic_heartbeat_threshold(500), 131);

    // Large tick current_tick = u32::MAX as u64 + 1000 with exact comparison
    let large_tick = (u32::MAX as u64) + 1000;
    let m = compile_stochastic_heartbeat_threshold(500);
    let tid = 42u32;

    let actual_spike = heartbeat_spike(large_tick, m, tid);
    let expected_rnd = stochastic_hash(large_tick, tid) & HEARTBEAT_PHASE_MASK;
    let expected_spike = expected_rnd < (m as u64);

    assert_eq!(actual_spike, expected_spike);

    // Check heartbeat_spike for period == 1 (always true)
    for tick in 0..100 {
        assert!(heartbeat_spike(tick, MAX_HEARTBEAT_M, 42));
    }

    // Check heartbeat_spike for period == 0 (always false)
    for tick in 0..100 {
        assert!(!heartbeat_spike(tick, 0, 42));
    }
}

#[test]
fn test_spike_birth_and_sentinel() {
    let v_seg_1 = 1;
    let head_0_v1 = initial_axon_head(v_seg_1);
    assert_eq!(head_0_v1, 0u32.wrapping_sub(1));
    let head_1_v1 = propagate_head(head_0_v1, v_seg_1);
    assert_eq!(head_1_v1, 0);

    let v_seg_2 = 2;
    let head_0_v2 = initial_axon_head(v_seg_2);
    assert_eq!(head_0_v2, 0u32.wrapping_sub(2));
    let head_1_v2 = propagate_head(head_0_v2, v_seg_2);
    assert_eq!(head_1_v2, 0);

    // Sentinel stopping checks
    assert_eq!(propagate_head(AXON_SENTINEL, 1), AXON_SENTINEL);
    assert_eq!(propagate_head(AXON_SENTINEL, 2), AXON_SENTINEL);

    // Magnetic Sentinel Trap check for near-sentinel value (AXON_SENTINEL + 1)
    assert_eq!(propagate_head(AXON_SENTINEL + 1, 2), AXON_SENTINEL);
}

#[test]
fn test_active_tail_hit_boundaries() {
    let heads = [
        100,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let prop_len = 10;

    // d = 0 (head == seg_idx) -> hit == true
    assert!(active_tail_hit(&heads, 100, prop_len));

    // d = prop - 1 (100 - 91 = 9 < 10) -> hit == true
    assert!(active_tail_hit(&heads, 91, prop_len));

    // d = prop (100 - 90 = 10 < 10 is false) -> hit == false
    assert!(!active_tail_hit(&heads, 90, prop_len));

    // All-sentinel heads -> false
    let all_sentinel = [AXON_SENTINEL; 8];
    assert!(!active_tail_hit(&all_sentinel, 10, prop_len));
}

#[test]
fn test_glif_and_homeostasis_lifecycle() {
    let voltage = -70;
    let i_in = 10;
    let rest_potential = -70;
    let thresh_offset = 5;
    let leak_shift = 4;
    let updated = update_glif_voltage(
        voltage,
        i_in,
        rest_potential,
        thresh_offset,
        leak_shift,
        0,
        1,
        0,
    );
    assert_eq!(updated, -60);

    // Extreme boundary panic-free tests (INV-PHYS-004)
    let _res_min = update_glif_voltage(i32::MIN, 0, i32::MAX, 0, 4, 0, 1, 0);
    let _res_max = update_glif_voltage(i32::MAX, 0, i32::MIN, 0, 4, 0, 1, 0);

    // Homeostasis decay non-negative clamping
    assert_eq!(homeostasis_decay(10, 3), 7);
    assert_eq!(homeostasis_decay(2, 5), 0);
}

#[test]
fn test_gsop_math_comprehensive() {
    let inertia_curve = [128, 128, 128, 128, 128, 128, 128, 128];

    // Inertia rank boundaries for abs_w = 0, 268435455, 268435456, 2140000000
    assert_eq!(inertia_rank(0), 0);
    assert_eq!(inertia_rank(268_435_455), 0);
    assert_eq!(inertia_rank(268_435_456), 1);
    assert_eq!(inertia_rank(2_140_000_000), 7);

    // Mass to charge conversion (weight >> 16)
    assert_eq!(weight_to_charge(65536), 1);
    assert_eq!(weight_to_charge(-65536), -1);
    assert_eq!(weight_to_charge(32768), 0);

    let active_heads = [
        0,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let ltd_heads = [
        1,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let inactive_heads = [AXON_SENTINEL; 8];

    // i32::MIN safety test (apply_gsop_plasticity with i32::MIN without panic)
    let min_w = i32::MIN;
    let min_res = apply_gsop_plasticity(
        min_w,
        &inactive_heads,
        0,
        5,
        0,
        255,
        0,
        100,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert!(min_res < 0);

    // Sign preservation for positive / negative LTP
    let pos_ltp = apply_gsop_plasticity(
        1000,
        &active_heads,
        0,
        5,
        0,
        255,
        100,
        0,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert!(pos_ltp > 1000);

    let neg_ltp = apply_gsop_plasticity(
        -1000,
        &active_heads,
        0,
        5,
        0,
        255,
        100,
        0,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert!(neg_ltp < -1000); // absolute weight increases, sign stays negative

    // Dopamine D1/D2 reward/punishment cases
    // D1 reward amplifies potentiation
    let base_pot = apply_gsop_plasticity(
        1000,
        &active_heads,
        0,
        5,
        0,
        255,
        100,
        0,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    let d1_pot = apply_gsop_plasticity(
        1000,
        &active_heads,
        0,
        5,
        0,
        255,
        100,
        0,
        128,
        128,
        0,
        1,
        &inertia_curve,
    );
    assert!(d1_pot > base_pot);

    // D2 reward suppresses depression
    let base_dep = apply_gsop_plasticity(
        1000,
        &ltd_heads,
        2,
        5,
        0,
        255,
        0,
        100,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    let d2_dep = apply_gsop_plasticity(
        1000,
        &ltd_heads,
        2,
        5,
        0,
        255,
        0,
        100,
        128,
        0,
        128,
        1,
        &inertia_curve,
    );
    assert!(d2_dep > base_dep); // less weight lost

    // Negative weight under heavy depression test
    let neg_w = -100;
    let depressed = apply_gsop_plasticity(
        neg_w,
        &ltd_heads,
        2,
        5,
        0,
        255,
        0,
        1000,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(depressed, -MIN_WEIGHT_LIMIT);
    assert_eq!(depressed, -1);

    // Headroom guard clamping test
    let pos_w = 2_139_999_900;
    let potentiated = apply_gsop_plasticity(
        pos_w,
        &active_heads,
        0,
        5,
        0,
        255,
        1000,
        0,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(potentiated, MAX_WEIGHT_LIMIT);
}

#[test]
fn test_synaptic_fatigue_helpers() {
    // Fatigue recovery
    assert_eq!(recover_fatigue(50), 49);
    assert_eq!(recover_fatigue(0), 0);

    // Available fatigue capacity
    assert_eq!(fatigue_available(50, 255), 205);
    assert_eq!(fatigue_available(255, 255), 0);
    assert_eq!(fatigue_available(0, 255), 255);

    // Synaptic weight attenuation
    assert_eq!(apply_synaptic_fatigue(1000, 0, 255), 1000);
    assert_eq!(apply_synaptic_fatigue(1000, 255, 255), 0);
    assert_eq!(apply_synaptic_fatigue(1000, 127, 255), 501);

    // Spike fatigue addition
    assert_eq!(fatigue_after_spike(0, 255), 50);
    assert_eq!(fatigue_after_spike(220, 255), 255);
}

#[test]
fn test_stdp_soft_peak_exact_superposition() {
    let inertia_curve = [128; 8];
    // Exact Soft Peak: head == seg_idx (head = 2, seg_idx = 2)
    // Both dist_ltp = 0 < prop (5) and dist_ltd = 0 < prop (5).
    // cooling = 5 - 0 = 5.
    // base_ltp = (100 * 128 * 1) / 128 = 100.
    // base_ltd = (40 * 128 * 1) / 128 = 40.
    // ltp_delta = (100 * 5) / 5 = 100.
    // ltd_delta = (40 * 5) / 5 = 40.
    // net_delta = 100 - 40 = 60.
    let heads = [
        2,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];

    let updated_w = apply_gsop_plasticity(
        1000,
        &heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(updated_w, 1060);
}

#[test]
fn test_stdp_golden_matrix_comprehensive() {
    let inertia_curve = [128; 8];

    // 1. All Sentinels -> Undergoes competitive depression (LTD delta of -40)
    let sentinel_heads = [AXON_SENTINEL; 8];
    let w_sentinel = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_sentinel, 960);

    // 2. Prop == 0 -> Plasticity disabled
    let heads = [
        2,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_prop_zero = apply_gsop_plasticity(
        1000,
        &heads,
        2,
        0,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_prop_zero, 1000);

    // 3. Out of Window Causal (head_u64 - seg_u64 >= prop)
    let out_causal_heads = [
        7,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ]; // head 7, seg 2, dist_ltp = 5 >= prop 5 -> 0 cooling
    let w_out_causal = apply_gsop_plasticity(
        1000,
        &out_causal_heads,
        2,
        5,
        0,
        255,
        100,
        0,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_out_causal, 1000);

    // 4. In-Window Causal with Spatial Cooling (head 4, seg 2, prop 5)
    // dist_ltp = 2 < 5, cooling = 5 - 2 = 3
    // base_ltp = (100 * 128 * 1) / 128 = 100
    // delta_ltp = (100 * 3) / 5 = 60
    let in_causal_heads = [
        4,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_in_causal = apply_gsop_plasticity(
        1000,
        &in_causal_heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_in_causal, 1060);

    // 5. Anti-Causal LTD In-Window with Spatial Cooling (head 1, seg 3, prop 5)
    // No causal head present -> undergoes full competitive depression (LTD delta of -40)
    let in_anticausal_heads = [
        1,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_in_anticausal = apply_gsop_plasticity(
        1000,
        &in_anticausal_heads,
        3,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_in_anticausal, 960);

    // 6. Anti-Causal LTD Out-of-Window (head 1, seg 7, prop 5)
    // No causal head present -> undergoes competitive depression (LTD delta of -40)
    let out_anticausal_heads = [
        1,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_out_anticausal = apply_gsop_plasticity(
        1000,
        &out_anticausal_heads,
        7,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_out_anticausal, 960);

    // 7. Multi-Head Summation (Head 0: 4 (dist 2 -> +60), Head 1: 3 (dist 1 -> +80))
    // Total delta = 60 + 80 = 140 -> 1140
    let multi_heads = [
        4,
        3,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_multi_head = apply_gsop_plasticity(
        1000,
        &multi_heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_multi_head, 1140);

    // 8. Full Fatigue Penalty: fatigue == capacity (255)
    // No causal head -> base_ltd = 40.
    // fatigue_penalty = (255 * 40) / 255 = 40.
    // net_delta = 0 - 40 - 40 = -80.
    let w_fat_full = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        255,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_fat_full, 920);

    // 9. Partial Fatigue Penalty (fatigue 25, capacity 50)
    // No causal head -> base_ltd = 40.
    // fatigue_penalty = (25 * 40) / 50 = 20 -> net_delta = -40 - 20 = -60.
    let w_fat_partial_50 = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        25,
        50,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_fat_partial_50, 940);

    // 10. Varying Capacity 1: fatigue 0 (penalty 0) vs fatigue 1 (penalty 40)
    // No causal head -> base_ltd = 40.
    let w_fat_cap1_zero = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        0,
        1,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_fat_cap1_zero, 960);

    let w_fat_cap1_full = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        1,
        1,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_fat_cap1_full, 920);

    // 11. Partial Fatigue near Capacity (fatigue 254, capacity 255)
    // No causal head -> base_ltd = 40.
    // fatigue_penalty = (254 * 40) / 255 = 39 -> net_delta = -40 - 39 = -79.
    let w_fat_near_cap = apply_gsop_plasticity(
        1000,
        &sentinel_heads,
        2,
        5,
        254,
        255,
        100,
        40,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_fat_near_cap, 921);

    // 12. Burst Multiplier: burst_count = 0 (treats as 1 -> +60) vs burst_count = 3 (multiplies base by 3 -> +180)
    let w_burst_0 = apply_gsop_plasticity(
        1000,
        &in_causal_heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        0,
        &inertia_curve,
    );
    assert_eq!(w_burst_0, 1060);

    let w_burst_3 = apply_gsop_plasticity(
        1000,
        &in_causal_heads,
        2,
        5,
        0,
        255,
        100,
        40,
        0,
        0,
        0,
        3,
        &inertia_curve,
    );
    assert_eq!(w_burst_3, 1180);
}

#[allow(clippy::too_many_arguments)]
fn legacy_apply_gsop_slot_math(
    w: i32,
    heads: &[u32; 8],
    seg_idx: u32,
    prop: u32,
    dopamine: i16,
    gsop_potentiation: i32,
    gsop_depression: i32,
    d1_affinity: i32,
    d2_affinity: i32,
    burst_count: u32,
    inertia_curve: &[i32; 8],
    timer: u8,
) -> i32 {
    if timer > 0 {
        return w;
    }

    let is_active = ((heads[0].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[1].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[2].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[3].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[4].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[5].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[6].wrapping_sub(seg_idx) <= prop) as i32)
        | ((heads[7].wrapping_sub(seg_idx) <= prop) as i32);

    let sign = if w >= 0 { 1 } else { -1 };
    let abs_w = w.abs();

    let mut rank = (abs_w >> 28) as usize;
    if rank > 7 {
        rank = 7;
    }
    let inertia = inertia_curve[rank];

    let pot_mod = ((dopamine as i32) * d1_affinity) >> 7;
    let dep_mod = ((dopamine as i32) * d2_affinity) >> 7;

    let raw_pot = gsop_potentiation + pot_mod;
    let raw_dep = gsop_depression - dep_mod;

    let final_pot = raw_pot & !(raw_pot >> 31);
    let final_dep = raw_dep & !(raw_dep >> 31);

    let burst_mult = if burst_count > 0 {
        burst_count as i32
    } else {
        1
    };

    let delta_pot = (final_pot * inertia * burst_mult) >> 7;
    let delta_dep = (final_dep * inertia * burst_mult) >> 7;

    let mut delta = if is_active != 0 {
        delta_pot
    } else {
        -delta_dep
    };

    delta = (delta * 128) >> 7;

    let mut new_abs = abs_w + delta;
    new_abs &= !(new_abs >> 31);
    if new_abs > 2_140_000_000 {
        new_abs = 2_140_000_000;
    }

    new_abs * sign
}

#[test]
fn test_gsop_parity_probe() {
    let w_init = 3500 << 16;
    let gsop_pot = 240;
    let gsop_dep = 68;
    let d1_aff = 192;
    let d2_aff = 128;
    let inertia_curve = [128, 121, 116, 110, 105, 100, 95, 91];
    let burst_count = 1;
    let prop = 20;
    let seg_idx = 100;

    let head_cases = [
        (
            "Case A (Causal Only)",
            [
                110,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
            ],
        ),
        (
            "Case B (Anti-Causal Only)",
            [
                90,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
            ],
        ),
        (
            "Case C (Both)",
            [
                110,
                90,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
                AXON_SENTINEL,
            ],
        ),
        ("Case D (Inactive)", [AXON_SENTINEL; 8]),
    ];

    let dopamine_cases = [0, 50];
    let fatigue_cases = [(0, 18), (18, 18)];

    let mut markdown = String::new();
    markdown.push_str("| Case | Dopamine | Fatigue/Cap | Legacy Delta | Axi Delta | Equal? |\n");
    markdown.push_str("|---|---|---|---|---|---|\n");

    for (case_name, heads) in &head_cases {
        for &da in &dopamine_cases {
            for &(fat, cap) in &fatigue_cases {
                let w_legacy_final = legacy_apply_gsop_slot_math(
                    w_init,
                    heads,
                    seg_idx,
                    prop,
                    da as i16,
                    gsop_pot,
                    gsop_dep,
                    d1_aff,
                    d2_aff,
                    burst_count,
                    &inertia_curve,
                    fat,
                );

                let w_axi_final = apply_gsop_plasticity(
                    w_init,
                    heads,
                    seg_idx,
                    prop,
                    fat,
                    cap,
                    gsop_pot,
                    gsop_dep,
                    da,
                    d1_aff,
                    d2_aff,
                    burst_count,
                    &inertia_curve,
                );

                let delta_legacy = w_legacy_final - w_init;
                let delta_axi = w_axi_final - w_init;
                let equal = w_legacy_final == w_axi_final;

                markdown.push_str(&format!(
                    "| {} | {} | {}/{} | {} | {} | {} |\n",
                    case_name,
                    da,
                    fat,
                    cap,
                    delta_legacy,
                    delta_axi,
                    if equal { "YES" } else { "NO" }
                ));
            }
        }
    }

    // Optional durable table: repo-relative artifacts/ when present (never host-absolute).
    println!("{markdown}");
    let mut artifacts_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/physics -> crates -> AxiEngine -> workspace root
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.push("artifacts");
    if artifacts_dir.is_dir() {
        let file_path = artifacts_dir.join("gsop_da_transfer_audit_table.md");
        std::fs::write(&file_path, &markdown).expect("write H1 parity table");
        println!("Wrote results to {}", file_path.display());
    }
}

#[test]
fn test_gsop_h2_event_counters_wash() {
    struct SimpleRng {
        state: u64,
    }
    impl SimpleRng {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }
        fn next_u32(&mut self) -> u32 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.state >> 32) as u32
        }
        fn next_f32(&mut self) -> f32 {
            (self.next_u32() & 0xffffff) as f32 / 16777216.0
        }
    }

    let w_init = 3500 << 16;
    let gsop_pot = 240;
    let gsop_dep = 68;
    let fatigue_capacity = 18;
    let d1_aff = 192;
    let d2_aff = 128;
    let inertia_curve = [128, 121, 116, 110, 105, 100, 95, 91];
    let burst_count = 1;
    let prop = 20;
    let seg_idx = 10;

    let postsynaptic_spikes = [40, 120, 200];

    let conditions = [
        ("Normal", 50, true),
        ("DA-off", 0, true),
        ("Plasticity-off", 50, false),
    ];

    let mut markdown = String::new();
    markdown.push_str("| Condition | Synapse Class | LTP Count | LTD Count | Sum LTP | Sum LTD | Net Change | Wash Index | Mean Net Change |\n");
    markdown.push_str("|---|---|---|---|---|---|---|---|---|\n");

    for (cond_name, dopamine, plasticity_enabled) in &conditions {
        let mut rng = SimpleRng::new(42);

        let mut m_ltp_count = 0;
        let mut m_ltd_count = 0;
        let mut m_ltp_mass = 0i64;
        let mut m_ltd_mass = 0i64;
        let mut matched_net_mass = 0i64;

        let mut u_ltp_count = 0;
        let mut u_ltd_count = 0;
        let mut u_ltp_mass = 0i64;
        let mut u_ltd_mass = 0i64;
        let mut unmatched_net_mass = 0i64;

        let num_trials = 100;

        for _trial in 0..num_trials {
            let mut weights = vec![w_init; 12];
            let mut fatigue = vec![0u8; 12];
            let mut heads = vec![[AXON_SENTINEL; 8]; 12];

            for t in 0..330 {
                // 1. Recover fatigue
                for fat in fatigue.iter_mut() {
                    *fat = fat.saturating_sub(1);
                }

                // 2. Propagate heads
                for h_arr in heads.iter_mut() {
                    for h in h_arr.iter_mut() {
                        if *h != AXON_SENTINEL {
                            *h = h.wrapping_add(1);
                        }
                    }
                }

                // 3. Generate presynaptic spikes (during 0..20 for matched)
                if t <= 20 {
                    for syn_idx in 0..12 {
                        let is_matched = syn_idx < 7;
                        if is_matched {
                            if rng.next_f32() < 0.1100 {
                                let h_arr = &mut heads[syn_idx];
                                h_arr[7] = h_arr[6];
                                h_arr[6] = h_arr[5];
                                h_arr[5] = h_arr[4];
                                h_arr[4] = h_arr[3];
                                h_arr[3] = h_arr[2];
                                h_arr[2] = h_arr[1];
                                h_arr[1] = h_arr[0];
                                h_arr[0] = 0;

                                fatigue[syn_idx] =
                                    fatigue[syn_idx].saturating_add(50).min(fatigue_capacity);
                            }
                        }
                    }
                }

                // 4. Apply plasticity at postsynaptic spikes
                if postsynaptic_spikes.contains(&t) {
                    for syn_idx in 0..12 {
                        let old_w = weights[syn_idx];
                        let is_matched = syn_idx < 7;

                        let new_w = if *plasticity_enabled {
                            apply_gsop_plasticity(
                                old_w,
                                &heads[syn_idx],
                                seg_idx,
                                prop,
                                fatigue[syn_idx],
                                fatigue_capacity,
                                gsop_pot,
                                gsop_dep,
                                *dopamine,
                                d1_aff,
                                d2_aff,
                                burst_count,
                                &inertia_curve,
                            )
                        } else {
                            old_w
                        };

                        weights[syn_idx] = new_w;
                        let delta = (new_w - old_w) as i64;

                        if is_matched {
                            if delta > 0 {
                                m_ltp_count += 1;
                                m_ltp_mass += delta;
                            } else if delta < 0 {
                                m_ltd_count += 1;
                                m_ltd_mass += delta;
                            }
                            matched_net_mass += delta;
                        } else {
                            if delta > 0 {
                                u_ltp_count += 1;
                                u_ltp_mass += delta;
                            } else if delta < 0 {
                                u_ltd_count += 1;
                                u_ltd_mass += delta;
                            }
                            unmatched_net_mass += delta;
                        }
                    }
                }
            }
        }

        let m_denom = m_ltp_mass + m_ltd_mass.abs();
        let m_wash = if m_denom > 0 {
            1.0 - (matched_net_mass.abs() as f64 / m_denom as f64)
        } else {
            0.0
        };
        let m_mean_net = matched_net_mass as f64 / (num_trials * 7) as f64;

        let u_denom = u_ltp_mass + u_ltd_mass.abs();
        let u_wash = if u_denom > 0 {
            1.0 - (unmatched_net_mass.abs() as f64 / u_denom as f64)
        } else {
            0.0
        };
        let u_mean_net = unmatched_net_mass as f64 / (num_trials * 5) as f64;

        // Sanity: control and delayed-post branch selection (execution proof beyond cargo exit).
        if !*plasticity_enabled {
            assert_eq!(m_ltp_count, 0);
            assert_eq!(m_ltd_count, 0);
            assert_eq!(m_ltp_mass, 0);
            assert_eq!(m_ltd_mass, 0);
            assert_eq!(matched_net_mass, 0);
            assert_eq!(u_ltp_count, 0);
            assert_eq!(u_ltd_count, 0);
            assert_eq!(u_ltp_mass, 0);
            assert_eq!(u_ltd_mass, 0);
            assert_eq!(unmatched_net_mass, 0);
        } else if *cond_name == "Normal" {
            assert!(
                m_ltp_count > 0,
                "Normal must enter matched LTP under delayed-post schedule"
            );
            assert!(
                m_ltd_count > 0,
                "matched slots can produce LTD when out-of-sync under competitive depression"
            );
            assert!(
                unmatched_net_mass < 0,
                "unmatched slots must depress under competitive LTD"
            );
            assert!(m_wash <= 1.0, "wash index must be valid");
            assert!(
                matched_net_mass - unmatched_net_mass >= 500,
                "matched vs unmatched net mass must separate under prereg threshold"
            );
        }

        markdown.push_str(&format!(
            "| {} | Matched | {} | {} | {} | {} | {} | {:.4} | {:.2} |\n",
            cond_name,
            m_ltp_count,
            m_ltd_count,
            m_ltp_mass,
            m_ltd_mass,
            matched_net_mass,
            m_wash,
            m_mean_net
        ));
        markdown.push_str(&format!(
            "| {} | Unmatched | {} | {} | {} | {} | {} | {:.4} | {:.2} |\n",
            cond_name,
            u_ltp_count,
            u_ltd_count,
            u_ltp_mass,
            u_ltd_mass,
            unmatched_net_mass,
            u_wash,
            u_mean_net
        ));
    }

    // Six result rows: 3 conditions × matched/unmatched.
    assert_eq!(markdown.lines().count(), 2 + 6, "header + six data rows");
    println!("{markdown}");
    let mut artifacts_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.push("artifacts");
    if artifacts_dir.is_dir() {
        let file_path = artifacts_dir.join("gsop_h2_wash_audit_table.md");
        std::fs::write(&file_path, &markdown).expect("write H2 wash table");
        println!("Wrote results to {}", file_path.display());
    }
}

#[test]
fn test_gsop_h3_fatigue_dominance() {
    struct SimpleRng {
        state: u64,
    }
    impl SimpleRng {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }
        fn next_u32(&mut self) -> u32 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.state >> 32) as u32
        }
        fn next_f32(&mut self) -> f32 {
            (self.next_u32() & 0xffffff) as f32 / 16777216.0
        }
    }

    let w_init = 3500 << 16;
    let gsop_pot = 240;
    let gsop_dep = 68;
    let fatigue_capacity = 18;
    let d1_aff = 192;
    let d2_aff = 128;
    let inertia_curve = [128, 121, 116, 110, 105, 100, 95, 91];
    let burst_count = 1;
    let prop = 20;
    let seg_idx = 10;

    let postsynaptic_spikes = [5, 10, 15];

    let conditions = [
        ("Normal", 50, true),
        ("DA-off", 0, true),
        ("Plasticity-off", 50, false),
    ];

    let mut markdown = String::new();
    markdown.push_str("| Condition | Synapse Class | LTP Count | LTD Count | Sum LTP | Sum LTD | Net Change | Wash Index | Mean Fatigue | Mean Net Change |\n");
    markdown.push_str("|---|---|---|---|---|---|---|---|---|---|\n");

    for (cond_name, dopamine, plasticity_enabled) in &conditions {
        let mut rng = SimpleRng::new(42);

        let mut m_ltp_count = 0;
        let mut m_ltd_count = 0;
        let mut m_ltp_mass = 0i64;
        let mut m_ltd_mass = 0i64;
        let mut matched_net_mass = 0i64;
        let mut sum_fatigue_at_spikes = 0u64;
        let mut count_fatigue_at_spikes = 0;

        let mut u_ltp_count = 0;
        let mut u_ltd_count = 0;
        let mut u_ltp_mass = 0i64;
        let mut u_ltd_mass = 0i64;
        let mut unmatched_net_mass = 0i64;

        let num_trials = 100;

        for _trial in 0..num_trials {
            let mut weights = vec![w_init; 12];
            let mut fatigue = vec![0u8; 12];
            let mut heads = vec![[AXON_SENTINEL; 8]; 12];

            for t in 0..330 {
                // 1. Recover fatigue
                for fat in fatigue.iter_mut() {
                    *fat = fat.saturating_sub(1);
                }

                // 2. Propagate heads
                for h_arr in heads.iter_mut() {
                    for h in h_arr.iter_mut() {
                        if *h != AXON_SENTINEL {
                            *h = h.wrapping_add(1);
                        }
                    }
                }

                // 3. Generate presynaptic spikes (during 0..20 for matched)
                if t <= 20 {
                    for syn_idx in 0..12 {
                        let is_matched = syn_idx < 7;
                        if is_matched {
                            if rng.next_f32() < 0.1100 {
                                let h_arr = &mut heads[syn_idx];
                                h_arr[7] = h_arr[6];
                                h_arr[6] = h_arr[5];
                                h_arr[5] = h_arr[4];
                                h_arr[4] = h_arr[3];
                                h_arr[3] = h_arr[2];
                                h_arr[2] = h_arr[1];
                                h_arr[1] = h_arr[0];
                                h_arr[0] = 0;

                                fatigue[syn_idx] =
                                    fatigue[syn_idx].saturating_add(50).min(fatigue_capacity);
                            }
                        }
                    }
                }

                // 4. Apply plasticity at postsynaptic spikes
                if postsynaptic_spikes.contains(&t) {
                    for syn_idx in 0..12 {
                        let old_w = weights[syn_idx];
                        let is_matched = syn_idx < 7;

                        if is_matched {
                            sum_fatigue_at_spikes += fatigue[syn_idx] as u64;
                            count_fatigue_at_spikes += 1;
                        }

                        let new_w = if *plasticity_enabled {
                            apply_gsop_plasticity(
                                old_w,
                                &heads[syn_idx],
                                seg_idx,
                                prop,
                                fatigue[syn_idx],
                                fatigue_capacity,
                                gsop_pot,
                                gsop_dep,
                                *dopamine,
                                d1_aff,
                                d2_aff,
                                burst_count,
                                &inertia_curve,
                            )
                        } else {
                            old_w
                        };

                        weights[syn_idx] = new_w;
                        let delta = (new_w - old_w) as i64;

                        if is_matched {
                            if delta > 0 {
                                m_ltp_count += 1;
                                m_ltp_mass += delta;
                            } else if delta < 0 {
                                m_ltd_count += 1;
                                m_ltd_mass += delta;
                            }
                            matched_net_mass += delta;
                        } else {
                            if delta > 0 {
                                u_ltp_count += 1;
                                u_ltp_mass += delta;
                            } else if delta < 0 {
                                u_ltd_count += 1;
                                u_ltd_mass += delta;
                            }
                            unmatched_net_mass += delta;
                        }
                    }
                }
            }
        }

        // Sanity assertions for controls
        if !*plasticity_enabled {
            assert_eq!(m_ltp_count, 0);
            assert_eq!(m_ltd_count, 0);
            assert_eq!(m_ltp_mass, 0);
            assert_eq!(m_ltd_mass, 0);
            assert_eq!(matched_net_mass, 0);
            assert_eq!(u_ltp_count, 0);
            assert_eq!(u_ltd_count, 0);
            assert_eq!(u_ltp_mass, 0);
            assert_eq!(u_ltd_mass, 0);
            assert_eq!(unmatched_net_mass, 0);
        } else if *cond_name == "Normal" {
            // Normal condition must have active fatigue
            assert!(
                sum_fatigue_at_spikes > 0,
                "H3 must have non-zero fatigue at overlapping post ticks"
            );
        }

        let m_denom = m_ltp_mass + m_ltd_mass.abs();
        let m_wash = if m_denom > 0 {
            1.0 - (matched_net_mass.abs() as f64 / m_denom as f64)
        } else {
            0.0
        };
        let m_mean_net = matched_net_mass as f64 / (num_trials * 7) as f64;
        let m_mean_fatigue = sum_fatigue_at_spikes as f64 / count_fatigue_at_spikes.max(1) as f64;

        let u_denom = u_ltp_mass + u_ltd_mass.abs();
        let u_wash = if u_denom > 0 {
            1.0 - (unmatched_net_mass.abs() as f64 / u_denom as f64)
        } else {
            0.0
        };
        let u_mean_net = unmatched_net_mass as f64 / (num_trials * 5) as f64;

        markdown.push_str(&format!(
            "| {} | Matched | {} | {} | {} | {} | {} | {:.4} | {:.2} | {:.2} |\n",
            cond_name,
            m_ltp_count,
            m_ltd_count,
            m_ltp_mass,
            m_ltd_mass,
            matched_net_mass,
            m_wash,
            m_mean_fatigue,
            m_mean_net
        ));
        markdown.push_str(&format!(
            "| {} | Unmatched | {} | {} | {} | {} | {} | {:.4} | {:.2} | {:.2} |\n",
            cond_name,
            u_ltp_count,
            u_ltd_count,
            u_ltp_mass,
            u_ltd_mass,
            unmatched_net_mass,
            u_wash,
            0.0,
            u_mean_net
        ));
    }

    // Six result rows: 3 conditions × matched/unmatched.
    assert_eq!(markdown.lines().count(), 2 + 6, "header + six data rows");
    println!("{markdown}");
    let mut artifacts_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.pop();
    artifacts_dir.push("artifacts");
    if artifacts_dir.is_dir() {
        let file_path = artifacts_dir.join("gsop_h3_fatigue_dominance_table.md");
        std::fs::write(&file_path, &markdown).expect("write H3 fatigue table");
        println!("Wrote results to {}", file_path.display());
    }
}

#[test]
fn test_competitive_depression_proof() {
    let inertia_curve = [128; 8];

    // 1. Inactive/sentinel heads -> assert Δmass < 0
    let w_init = 1000;
    let sentinel_heads = [AXON_SENTINEL; 8];
    let w_sentinel = apply_gsop_plasticity(
        w_init,
        &sentinel_heads,
        2,   // seg_idx
        5,   // prop
        0,   // fatigue
        255, // fatigue_capacity
        100, // potentiation
        40,  // depression
        0,   // dopamine
        0,   // d1
        0,   // d2
        1,   // burst
        &inertia_curve,
    );
    let delta_inactive = w_sentinel - w_init;
    assert!(
        delta_inactive < 0,
        "Inactive/sentinel heads must result in negative delta mass"
    );

    // 2. Causal head in window -> potentiation still possible
    let causal_heads = [
        4,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
        AXON_SENTINEL,
    ];
    let w_causal = apply_gsop_plasticity(
        w_init,
        &causal_heads,
        2,   // seg_idx (dist_ltp = 2 < prop 5 -> causal hit)
        5,   // prop
        0,   // fatigue
        255, // fatigue_capacity
        100, // potentiation
        40,  // depression
        0,   // dopamine
        0,   // d1
        0,   // d2
        1,   // burst
        &inertia_curve,
    );
    let delta_causal = w_causal - w_init;
    assert!(
        delta_causal > 0,
        "Causal head in window must result in positive delta mass (potentiation)"
    );

    // 3. Short matched vs unmatched schedule -> assert unmatched net < matched net (or unmatched net < 0)
    assert!(
        delta_inactive < delta_causal,
        "Unmatched delta must be strictly less than matched/causal delta"
    );
    assert!(delta_inactive < 0, "Unmatched delta must be negative");
}
