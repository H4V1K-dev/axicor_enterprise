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

    // 1. All Sentinels -> Weight unchanged
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
    assert_eq!(w_sentinel, 1000);

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
    // dist_ltd = 3 - 1 = 2 < 5, cooling = 5 - 2 = 3
    // base_ltd = (40 * 128 * 1) / 128 = 40
    // delta_ltd = (40 * 3) / 5 = 24 -> net_delta = -24
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
    assert_eq!(w_in_anticausal, 976);

    // 6. Anti-Causal LTD Out-of-Window (head 1, seg 7, prop 5)
    // dist_ltd = 7 - 1 = 6 >= 5 -> cooling = 0
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
    assert_eq!(w_out_anticausal, 1000);

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
    // base_ltd = (40 * 128 * 1) / 128 = 40.
    // fatigue_penalty = (255 * 40) / 255 = 40.
    // net_delta = 0 - 0 - 40 = -40.
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
    assert_eq!(w_fat_full, 960);

    // 9. Partial Fatigue Penalty (fatigue 25, capacity 50)
    // fatigue_penalty = (25 * 40) / 50 = 20 -> net_delta = -20
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
    assert_eq!(w_fat_partial_50, 980);

    // 10. Varying Capacity 1: fatigue 0 (penalty 0) vs fatigue 1 (penalty 40)
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
    assert_eq!(w_fat_cap1_zero, 1000);

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
    assert_eq!(w_fat_cap1_full, 960);

    // 11. Partial Fatigue near Capacity (fatigue 254, capacity 255)
    // fatigue_penalty = (254 * 40) / 255 = 39 -> net_delta = -39
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
    assert_eq!(w_fat_near_cap, 961);

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
