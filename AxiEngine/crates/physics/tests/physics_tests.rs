use physics::*;
use static_assertions::const_assert_eq;
use types::AXON_SENTINEL;

// Static assertions for fundamental physical constants
const_assert_eq!(MASS_TO_CHARGE_SHIFT, 16);
const_assert_eq!(MIN_WEIGHT_LIMIT, 1);
const_assert_eq!(MAX_WEIGHT_LIMIT, 2_140_000_000);
const_assert_eq!(INERTIA_RANK_SHIFT, 28);
const_assert_eq!(MAX_INERTIA_RANK, 7);
const_assert_eq!(DDS_PHASE_MOD, 65_536);
const_assert_eq!(DDS_PHASE_MASK, 0xFFFF);
const_assert_eq!(DDS_SCATTER_PRIME, 104_729);
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
fn test_dds_heartbeat_matrix() {
    // period 65536 -> 1, period 65537 -> 0
    assert_eq!(compile_dds_heartbeat(65536), 1);
    assert_eq!(compile_dds_heartbeat(65537), 0);

    // period == 0 or > 65536 -> heartbeat disabled (m = 0)
    assert_eq!(compile_dds_heartbeat(0), 0);
    assert_eq!(compile_dds_heartbeat(100000), 0);

    // period == 1 -> m = MAX_HEARTBEAT_M (65535)
    assert_eq!(compile_dds_heartbeat(1), MAX_HEARTBEAT_M);

    // period = 500 -> 65536 / 500 = 131
    assert_eq!(compile_dds_heartbeat(500), 131);

    // Large tick current_tick = u32::MAX as u64 + 1000 with exact comparison
    let large_tick = (u32::MAX as u64) + 1000;
    let m = compile_dds_heartbeat(500);
    let tid = 42u32;

    let actual_spike = heartbeat_spike(large_tick, m, tid);
    let expected_phase = ((large_tick.wrapping_mul(m as u64))
        .wrapping_add((tid as u64).wrapping_mul(DDS_SCATTER_PRIME)))
        & DDS_PHASE_MASK;
    let expected_spike = expected_phase < (m as u64);

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

    // i32::MIN safety test (apply_gsop_plasticity with i32::MIN without panic)
    let min_w = i32::MIN;
    let min_res = apply_gsop_plasticity(min_w, false, 0, 100, 0, 0, 0, 1, &inertia_curve);
    assert!(min_res < 0);

    // Sign preservation for positive / negative LTP
    let pos_ltp = apply_gsop_plasticity(1000, true, 100, 0, 0, 0, 0, 1, &inertia_curve);
    assert!(pos_ltp > 1000);

    let neg_ltp = apply_gsop_plasticity(-1000, true, 100, 0, 0, 0, 0, 1, &inertia_curve);
    assert!(neg_ltp < -1000); // absolute weight increases, sign stays negative

    // Dopamine D1/D2 reward/punishment cases
    // D1 reward amplifies potentiation
    let base_pot = apply_gsop_plasticity(1000, true, 100, 0, 0, 0, 0, 1, &inertia_curve);
    let d1_pot = apply_gsop_plasticity(1000, true, 100, 0, 128, 128, 0, 1, &inertia_curve);
    assert!(d1_pot > base_pot);

    // D2 reward suppresses depression
    let base_dep = apply_gsop_plasticity(1000, false, 0, 100, 0, 0, 0, 1, &inertia_curve);
    let d2_dep = apply_gsop_plasticity(1000, false, 0, 100, 128, 0, 128, 1, &inertia_curve);
    assert!(d2_dep > base_dep); // less weight lost

    // Negative weight under heavy depression test
    let neg_w = -100;
    let depressed = apply_gsop_plasticity(neg_w, false, 0, 1000, 0, 0, 0, 1, &inertia_curve);
    assert_eq!(depressed, -MIN_WEIGHT_LIMIT);
    assert_eq!(depressed, -1);

    // Headroom guard clamping test
    let pos_w = 2_139_999_900;
    let potentiated = apply_gsop_plasticity(pos_w, true, 1000, 0, 0, 0, 0, 1, &inertia_curve);
    assert_eq!(potentiated, MAX_WEIGHT_LIMIT);
}
