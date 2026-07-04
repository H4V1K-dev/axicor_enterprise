//! Conformance unit tests for `mvp-cpu-replay` state plane access and axon head structures.

#![cfg(feature = "mvp-cpu-replay")]

use layout::{
    compute_state_offsets, BurstHeads8, VariantParameters, AXONS_FILE_VERSION, AXONS_MAGIC,
    STATE_FILE_VERSION, STATE_MAGIC, VARIANT_LUT_LEN,
};
use test_harness::{
    cpu_apply_gsop, cpu_apply_spike_batch, cpu_extract_telemetry, cpu_inject_inputs,
    cpu_propagate_axons, cpu_record_outputs, cpu_sort_and_prune, research_apply_gsop_plasticity,
    MvpAxonBuffer, MvpStateBuffer,
};
use types::AXON_SENTINEL;

#[test]
fn test_layout_offsets_and_header_integration() {
    let padded_n = 128;
    let total_axons = 32;
    let state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let expected_offsets = compute_state_offsets(padded_n);

    assert_eq!(state_buf.padded_n(), padded_n);
    assert_eq!(state_buf.total_axons(), total_axons);
    assert_eq!(state_buf.offsets(), &expected_offsets);
    assert_eq!(
        state_buf.as_bytes().len(),
        expected_offsets.total_state_size
    );

    // Verify 16-byte StateFileHeader fields
    let header = state_buf.header();
    assert_eq!(header.magic, STATE_MAGIC);
    assert_eq!(header.version, STATE_FILE_VERSION);
    assert_eq!(header.padded_n, padded_n as u32);
    assert_eq!(header.total_axons, total_axons as u32);
}

#[test]
fn test_state_buffer_from_raw_wrapping() {
    let padded_n = 64;
    let total_axons = 16;
    let mut original = MvpStateBuffer::new(padded_n, total_axons);

    original.write_soma_voltage(5, -65000);
    original.write_soma_flags(5, 0x01);
    original.write_dendrite_weight(2, 5, 450);

    let raw_bytes = original.as_bytes().to_vec();
    let reloaded = MvpStateBuffer::from_raw(padded_n, total_axons, raw_bytes);

    assert_eq!(reloaded.padded_n(), padded_n);
    assert_eq!(reloaded.total_axons(), total_axons);
    assert_eq!(reloaded.header().magic, STATE_MAGIC);
    assert_eq!(reloaded.read_soma_voltage(5), -65000);
    assert_eq!(reloaded.read_soma_flags(5), 0x01);
    assert_eq!(reloaded.read_dendrite_weight(2, 5), 450);
}

#[test]
fn test_axon_heads_initialization_and_from_raw_wrapping() {
    let total_axons = 16;
    let mut axon_buf = MvpAxonBuffer::new(total_axons);

    assert_eq!(axon_buf.total_axons(), total_axons);
    assert_eq!(axon_buf.as_bytes().len(), 16 + total_axons * 32);
    assert_eq!(axon_buf.payload_bytes().len(), total_axons * 32);

    // Verify 16-byte AxonsFileHeader fields
    let header = axon_buf.header();
    assert_eq!(header.magic, AXONS_MAGIC);
    assert_eq!(header.version, AXONS_FILE_VERSION);
    assert_eq!(header.total_axons, total_axons as u32);

    // Verify all axon heads are initialized to AXON_SENTINEL
    for i in 0..total_axons {
        let head = axon_buf.read_head(i);
        assert_eq!(head.h0, AXON_SENTINEL);
        assert_eq!(head.h1, AXON_SENTINEL);
        assert_eq!(head.h2, AXON_SENTINEL);
        assert_eq!(head.h3, AXON_SENTINEL);
        assert_eq!(head.h4, AXON_SENTINEL);
        assert_eq!(head.h5, AXON_SENTINEL);
        assert_eq!(head.h6, AXON_SENTINEL);
        assert_eq!(head.h7, AXON_SENTINEL);
    }

    // Modify axon head 5 with distinct h0..h7 values
    let mut custom_head = BurstHeads8::empty(AXON_SENTINEL);
    custom_head.h0 = 100;
    custom_head.h1 = 101;
    custom_head.h2 = 102;
    custom_head.h3 = 103;
    custom_head.h4 = 104;
    custom_head.h5 = 105;
    custom_head.h6 = 106;
    custom_head.h7 = 107;

    axon_buf.write_head(5, custom_head);

    let raw_bytes = axon_buf.as_bytes().to_vec();
    let reloaded = MvpAxonBuffer::from_raw(total_axons, raw_bytes);

    assert_eq!(reloaded.total_axons(), total_axons);
    assert_eq!(reloaded.header().magic, AXONS_MAGIC);

    let read_back = reloaded.read_head(5);
    assert_eq!(read_back.h0, 100);
    assert_eq!(read_back.h1, 101);
    assert_eq!(read_back.h2, 102);
    assert_eq!(read_back.h3, 103);
    assert_eq!(read_back.h4, 104);
    assert_eq!(read_back.h5, 105);
    assert_eq!(read_back.h6, 106);
    assert_eq!(read_back.h7, 107);

    // Unmodified head 4 remains AXON_SENTINEL
    assert_eq!(reloaded.read_head(4).h0, AXON_SENTINEL);
}

#[test]
fn test_dendrite_slot_indexing_and_state_rw() {
    let padded_n = 64;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    // 1. Verify index formula: slot * padded_n + tid
    assert_eq!(state_buf.dendrite_index(0, 0), 0);
    assert_eq!(state_buf.dendrite_index(0, 15), 15);
    assert_eq!(state_buf.dendrite_index(1, 0), 64);
    assert_eq!(state_buf.dendrite_index(127, 63), 127 * 64 + 63);

    // 2. Read/Write soma planes
    state_buf.write_soma_voltage(10, -70000);
    state_buf.write_soma_flags(10, 0x15);
    state_buf.write_threshold_offset(10, 1500);
    state_buf.write_timer(10, 3);
    state_buf.write_soma_to_axon(10, 42);

    assert_eq!(state_buf.read_soma_voltage(10), -70000);
    assert_eq!(state_buf.read_soma_flags(10), 0x15);
    assert_eq!(state_buf.read_threshold_offset(10), 1500);
    assert_eq!(state_buf.read_timer(10), 3);
    assert_eq!(state_buf.read_soma_to_axon(10), 42);

    // 3. Read/Write dendrite matrices using slot * padded_n + tid
    state_buf.write_dendrite_target(0, 10, 0x01000005);
    state_buf.write_dendrite_weight(0, 10, 250);
    state_buf.write_dendrite_timer(0, 10, 2);

    state_buf.write_dendrite_target(127, 63, 0x02000099);
    state_buf.write_dendrite_weight(127, 63, -500);
    state_buf.write_dendrite_timer(127, 63, 7);

    assert_eq!(state_buf.read_dendrite_target(0, 10), 0x01000005);
    assert_eq!(state_buf.read_dendrite_weight(0, 10), 250);
    assert_eq!(state_buf.read_dendrite_timer(0, 10), 2);

    assert_eq!(state_buf.read_dendrite_target(127, 63), 0x02000099);
    assert_eq!(state_buf.read_dendrite_weight(127, 63), -500);
    assert_eq!(state_buf.read_dendrite_timer(127, 63), 7);
}

#[test]
fn test_cpu_propagate_axons() {
    let mut heads = [
        BurstHeads8::empty(AXON_SENTINEL),
        BurstHeads8::empty(AXON_SENTINEL),
    ];

    heads[0].h0 = 10;
    heads[0].h1 = AXON_SENTINEL; // Sentinel remains sentinel
    heads[0].h2 = u32::MAX - 2; // Wrapping test

    cpu_propagate_axons(&mut heads, 5);

    assert_eq!(heads[0].h0, 15);
    assert_eq!(heads[0].h1, AXON_SENTINEL);
    assert_eq!(heads[0].h2, 2); // Wrapped
    assert_eq!(heads[1].h0, AXON_SENTINEL);
}

#[test]
fn test_cpu_apply_spike_batch() {
    let mut heads = [
        BurstHeads8::empty(AXON_SENTINEL),
        BurstHeads8::empty(AXON_SENTINEL),
    ];
    heads[0].h0 = 10;
    heads[0].h1 = 20;

    let schedule_indices = [0, 999]; // 0 is valid, 999 is out-of-range
    cpu_apply_spike_batch(&mut heads, &schedule_indices, 3);

    assert_eq!(heads[0].h0, 0u32.wrapping_sub(3));
    assert_eq!(heads[0].h1, 10);
    assert_eq!(heads[0].h2, 20);
    assert_eq!(heads[1].h0, AXON_SENTINEL);
}

#[test]
fn test_cpu_inject_inputs() {
    let mut heads = vec![BurstHeads8::empty(AXON_SENTINEL); 40];
    let virtual_offset = 5;
    let num_virtual_axons = 33; // Spans word 0 (bits 0..31) and word 1 (bit 0)

    let bitmask = [1u32 << 31, 1u32];

    cpu_inject_inputs(&mut heads, &bitmask, virtual_offset, num_virtual_axons, 2);

    let offset = virtual_offset as usize;
    assert_eq!(heads[offset + 30].h0, AXON_SENTINEL);
    assert_eq!(heads[offset + 31].h0, 0u32.wrapping_sub(2));
    assert_eq!(heads[offset + 32].h0, 0u32.wrapping_sub(2));
}

#[test]
fn test_cpu_record_outputs() {
    let soma_flags = [0x01, 0x00, 0x03, 0x00];
    let mapped_soma_ids = [0, 1, 2, 0xFFFF_FFFF];
    let total_mapped_somas = 4;
    let mut output_history = vec![0xFFu8; 8];

    cpu_record_outputs(
        &soma_flags,
        &mapped_soma_ids,
        &mut output_history,
        0,
        total_mapped_somas,
    );

    assert_eq!(output_history[0], 1);
    assert_eq!(output_history[1], 0);
    assert_eq!(output_history[2], 1);
    assert_eq!(output_history[3], 0xFF); // Skipped 0xFFFF_FFFF

    cpu_record_outputs(
        &soma_flags,
        &mapped_soma_ids,
        &mut output_history,
        1,
        total_mapped_somas,
    );
    assert_eq!(output_history[4], 1);
    assert_eq!(output_history[5], 0);
}

#[test]
fn test_cpu_extract_telemetry() {
    let soma_flags = [0x00, 0x01, 0x00, 0x11, 0x03, 0x00];
    let mut out_ids = [0u32; 2];

    let count = cpu_extract_telemetry(&soma_flags, &mut out_ids);

    assert_eq!(count, 2);
    assert_eq!(out_ids[0], 1);
    assert_eq!(out_ids[1], 3);
}

#[test]
fn test_cpu_propagate_axons_odd_length_tail() {
    let mut heads = [
        BurstHeads8::empty(AXON_SENTINEL),
        BurstHeads8::empty(AXON_SENTINEL),
        BurstHeads8::empty(AXON_SENTINEL), // Odd 3rd element
    ];
    heads[0].h0 = 10;
    heads[1].h0 = 20;
    heads[2].h0 = 30; // Odd tail

    cpu_propagate_axons(&mut heads, 5);

    // First pair (heads 0 and 1) is processed
    assert_eq!(heads[0].h0, 15);
    assert_eq!(heads[1].h0, 25);

    // Odd tail (head 2) is skipped by chunks_exact_mut(2) MVP parity contract
    assert_eq!(heads[2].h0, 30);
}

#[test]
fn test_cpu_inject_inputs_short_bitmask() {
    let mut heads = vec![BurstHeads8::empty(AXON_SENTINEL); 64];
    let virtual_offset = 0;
    let num_virtual_axons = 64; // Requires 2 bitmask words

    // Pass short bitmask with only 1 word (covers tid 0..31, missing word 1 for tid 32..63)
    let short_bitmask = [0xFFFF_FFFFu32];

    cpu_inject_inputs(
        &mut heads,
        &short_bitmask,
        virtual_offset,
        num_virtual_axons,
        4,
    );

    // tid 0..31 (word 0) are injected
    assert_eq!(heads[0].h0, 0u32.wrapping_sub(4));
    assert_eq!(heads[31].h0, 0u32.wrapping_sub(4));

    // tid 32..63 (missing word 1) safely skipped by safety guard without panic
    assert_eq!(heads[32].h0, AXON_SENTINEL);
    assert_eq!(heads[63].h0, AXON_SENTINEL);
}

#[test]
fn test_cpu_sort_and_prune_threshold_kills() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    // prune_threshold = 10 -> threshold_mass = 10 << 16 = 655360
    let prune_threshold = 10i16;

    // Slot 0: weight below threshold_mass (9 << 16 = 589824) -> should be killed
    state_buf.write_dendrite_target(0, 0, 100);
    state_buf.write_dendrite_weight(0, 0, 9 << 16);
    state_buf.write_dendrite_timer(0, 0, 5);

    // Slot 1: weight at threshold_mass (10 << 16 = 655360) -> kept
    state_buf.write_dendrite_target(1, 0, 200);
    state_buf.write_dendrite_weight(1, 0, 10 << 16);
    state_buf.write_dendrite_timer(1, 0, 3);

    cpu_sort_and_prune(&mut state_buf, prune_threshold);

    // Slot 0 becomes the surviving slot 1 (target 200, weight 10 << 16)
    assert_eq!(state_buf.read_dendrite_target(0, 0), 200);
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 10 << 16);
    assert_eq!(state_buf.read_dendrite_timer(0, 0), 3);

    // Slot 1 becomes killed dead slot (target 0, weight 0, timer 0)
    assert_eq!(state_buf.read_dendrite_target(1, 0), 0);
    assert_eq!(state_buf.read_dendrite_weight(1, 0), 0);
    assert_eq!(state_buf.read_dendrite_timer(1, 0), 0);
}

#[test]
fn test_cpu_sort_and_prune_sort_desc_by_abs_weight() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    state_buf.write_dendrite_target(0, 0, 1);
    state_buf.write_dendrite_weight(0, 0, 500 << 16);

    state_buf.write_dendrite_target(1, 0, 2);
    state_buf.write_dendrite_weight(1, 0, -(800 << 16)); // Negative strong weight

    state_buf.write_dendrite_target(2, 0, 3);
    state_buf.write_dendrite_weight(2, 0, 200 << 16);

    cpu_sort_and_prune(&mut state_buf, 0);

    // Sorted descending by abs(weight): -800 (800), 500 (500), 200 (200)
    assert_eq!(state_buf.read_dendrite_target(0, 0), 2);
    assert_eq!(state_buf.read_dendrite_weight(0, 0), -(800 << 16)); // Negative sign preserved

    assert_eq!(state_buf.read_dendrite_target(1, 0), 1);
    assert_eq!(state_buf.read_dendrite_weight(1, 0), 500 << 16);

    assert_eq!(state_buf.read_dendrite_target(2, 0), 3);
    assert_eq!(state_buf.read_dendrite_weight(2, 0), 200 << 16);

    assert_eq!(state_buf.read_dendrite_target(3, 0), 0);
}

#[test]
fn test_cpu_sort_and_prune_burst_reset_preserves_type_and_spike_bit() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    // 0xFF = 1111_1111 (spike bit 0 = 1, burst bits 1..3 = 111, type bits 4..7 = 1111)
    state_buf.write_soma_flags(0, 0xFF);

    cpu_sort_and_prune(&mut state_buf, 0);

    // 0xFF & 0xF1 = 0xF1 (1111_0001) -> burst bits reset to 0, spike & type preserved
    assert_eq!(state_buf.read_soma_flags(0), 0xF1);
}

#[test]
fn test_cpu_sort_and_prune_empty_array() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    cpu_sort_and_prune(&mut state_buf, 10);

    for slot in 0..128 {
        assert_eq!(state_buf.read_dendrite_target(slot, 0), 0);
        assert_eq!(state_buf.read_dendrite_weight(slot, 0), 0);
        assert_eq!(state_buf.read_dendrite_timer(slot, 0), 0);
    }
}

#[test]
fn test_cpu_sort_and_prune_full_array_no_prune() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    // Fill all 128 slots with increasing weights
    for slot in 0..128 {
        state_buf.write_dendrite_target(slot, 0, (slot + 1) as u32);
        state_buf.write_dendrite_weight(slot, 0, ((slot + 1) as i32) * (10 << 16));
    }

    cpu_sort_and_prune(&mut state_buf, 0);

    // Slot 0 should have highest weight (128 * (10 << 16))
    assert_eq!(state_buf.read_dendrite_target(0, 0), 128);
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 128 * (10 << 16));

    // Slot 127 should have lowest weight (1 * (10 << 16))
    assert_eq!(state_buf.read_dendrite_target(127, 0), 1);
    assert_eq!(state_buf.read_dendrite_weight(127, 0), 1 * (10 << 16));
}

#[test]
fn test_cpu_sort_and_prune_multi_tid_independence() {
    let padded_n = 64;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);

    // tid 0: slot 0 weight below threshold -> killed
    state_buf.write_dendrite_target(0, 0, 10);
    state_buf.write_dendrite_weight(0, 0, 1 << 16);

    // tid 1: slot 0 weight above threshold -> kept
    state_buf.write_dendrite_target(0, 1, 20);
    state_buf.write_dendrite_weight(0, 1, 50 << 16);

    cpu_sort_and_prune(&mut state_buf, 10);

    // tid 0 pruned
    assert_eq!(state_buf.read_dendrite_target(0, 0), 0);

    // tid 1 preserved independently
    assert_eq!(state_buf.read_dendrite_target(0, 1), 20);
    assert_eq!(state_buf.read_dendrite_weight(0, 1), 50 << 16);
}

fn test_variant_table() -> [VariantParameters; VARIANT_LUT_LEN] {
    [VariantParameters {
        threshold: 1000,
        rest_potential: -70000,
        leak_shift: 6,
        homeostasis_penalty: 50,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 100,
        gsop_potentiation: 128,
        gsop_depression: 64,
        homeostasis_decay: 1,
        refractory_period: 2,
        synapse_refractory_period: 0,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [128; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 64,
        d2_affinity: 64,
        heartbeat_m: 0,
    }; VARIANT_LUT_LEN]
}

#[test]
fn test_cpu_apply_gsop_non_spiking_unchanged() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x00);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 5;
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100000);
}

#[test]
fn test_cpu_apply_gsop_active_ltp_exact() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    // Active head: h0=2, seg=2 -> min_dist = 0 <= prop(5) -> 100% decay factor
    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2;
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Exact LTP delta: pot=128, inertia=128, burst=1, min_dist=0 -> delta = 128
    // 100000 + 128 = 100128
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100128);
}

#[test]
fn test_cpu_apply_gsop_spatial_cooling_attenuation() {
    let padded_n = 16;
    let total_axons = 16;
    let variants = test_variant_table();

    let mut custom_variants = variants;
    custom_variants[0].signal_propagation_length = 20; // prop = 20

    // Case 1: min_dist = 0 (head = 2, seg = 2) -> max LTP (decay_factor = 20/20 = 100%) -> delta = 128
    let mut state_max = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_max = MvpAxonBuffer::new(total_axons);
    state_max.write_soma_flags(0, 0x01);
    state_max.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_max.write_dendrite_weight(0, 0, 100000);
    let mut head_max = BurstHeads8::empty(AXON_SENTINEL);
    head_max.h0 = 2; // min_dist = 2 - 2 = 0
    axon_max.write_head(0, head_max);
    cpu_apply_gsop(&mut state_max, &axon_max, &custom_variants, 0);
    assert_eq!(state_max.read_dendrite_weight(0, 0), 100128);

    // Case 2: min_dist = prop / 2 = 10 (head = 12, seg = 2) -> 50% LTP (decay_factor = 10/20) -> delta = 64
    let mut state_half = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_half = MvpAxonBuffer::new(total_axons);
    state_half.write_soma_flags(0, 0x01);
    state_half.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_half.write_dendrite_weight(0, 0, 100000);
    let mut head_half = BurstHeads8::empty(AXON_SENTINEL);
    head_half.h0 = 12; // min_dist = 12 - 2 = 10
    axon_half.write_head(0, head_half);
    cpu_apply_gsop(&mut state_half, &axon_half, &custom_variants, 0);
    assert_eq!(state_half.read_dendrite_weight(0, 0), 100064);
}

#[test]
fn test_cpu_apply_gsop_inactive_ltd_exact() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Exact LTD delta with Spatial Cooling (min_d_ltd = 1, prop = 5):
    // decay_factor = 5 - 1 = 4 -> delta = - (64 * 128 * 1 * 4) / (128 * 5) = -51
    // 100000 - 51 = 99949
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 99949);
}

#[test]
fn test_cpu_apply_gsop_d1_exact() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_dist = 0
    axon_buf.write_head(0, head);

    // Dopamine 100 adjusts type 0 inertia to 128 - 100 = 28
    // pot_mod = (100 * 64) / 128 = 50 -> final_pot = 128 + 50 = 178
    // delta = (178 * 28 * 1 * 5) / (128 * 5) = 38 -> 100000 + 38 = 100038
    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 100);

    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100038);
}

#[test]
fn test_cpu_apply_gsop_d2_exact() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    // Dopamine 100 adjusts type 0 inertia to 28
    // dep_mod = (100 * 64) / 128 = 50 -> final_dep = 64 - 50 = 14
    // decay_factor = 5 - 1 = 4 -> delta = - (14 * 28 * 1 * 4) / (128 * 5) = -2
    // 100000 - 2 = 99998
    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 100);

    assert_eq!(state_buf.read_dendrite_weight(0, 0), 99998);
}

#[test]
fn test_cpu_apply_gsop_variant_id_selection() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let mut variants = test_variant_table();

    // Set variant 1 with double potentiation
    variants[1].gsop_potentiation = 256;

    // soma_flags = (1 << 4) | 0x01 = 0x11 -> var_id = 1
    state_buf.write_soma_flags(0, 0x11);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_dist = 0
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Variant 1 pot=256 -> delta = 256 -> 100000 + 256 = 100256
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100256);
}

#[test]
fn test_cpu_apply_gsop_top_clamp() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 2_139_999_950);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_dist = 0
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // 2_139_999_950 + 128 = 2_140_000_078 -> clamped to 2_140_000_000
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 2_140_000_000);
}

#[test]
fn test_cpu_apply_gsop_bottom_clamp() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 10); // Small initial weight

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // 10 - 51 = -41 -> clamped to MIN_WEIGHT_LIMIT (1)
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 1);
}

#[test]
fn test_cpu_apply_gsop_timer_before_zero_target() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);

    // Slot 0: timer > 0 and target = 0 (timer check must come before target == 0 break!)
    state_buf.write_dendrite_timer(0, 0, 5);
    state_buf.write_dendrite_target(0, 0, 0);

    // Slot 1: valid target and weight
    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Slot 0 skipped via timer > 0 continue without breaking loop; Slot 1 processed (depressed to 99949)
    assert_eq!(state_buf.read_dendrite_weight(1, 0), 99949);
}

#[test]
fn test_cpu_apply_gsop_zero_weight_continues() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);

    // Slot 0: target valid, but weight = 0 (must continue, not break!)
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 0);

    // Slot 1: target valid, weight = 100000
    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Slot 0 remains 0; Slot 1 processed (depressed to 99949)
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 0);
    assert_eq!(state_buf.read_dendrite_weight(1, 0), 99949);
}

#[test]
fn test_cpu_apply_gsop_active_tail_hit_via_h7() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    // Set active head hit on h7, h0..h6 remain sentinel
    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h7 = 2; // min_dist = 0
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    // Active hit via h7 potentiates weight to 100128
    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100128);
}

#[test]
fn test_cpu_apply_gsop_timer_skip() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_timer(0, 0, 3);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, 100000);

    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100000);
    assert_eq!(state_buf.read_dendrite_weight(1, 0), 99949);
}

#[test]
fn test_cpu_apply_gsop_zero_target_break() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);

    state_buf.write_dendrite_target(0, 0, 0);
    state_buf.write_dendrite_weight(0, 0, 100000);

    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    assert_eq!(state_buf.read_dendrite_weight(1, 0), 100000);
}

#[test]
fn test_cpu_apply_gsop_raw_id_zero_break() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);

    state_buf.write_dendrite_target(0, 0, 5 << 24);
    state_buf.write_dendrite_weight(0, 0, 100000);

    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    assert_eq!(state_buf.read_dendrite_weight(1, 0), 100000);
}

#[test]
fn test_cpu_apply_gsop_out_of_range_axon_continue() {
    let padded_n = 16;
    let total_axons = 10;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);

    state_buf.write_dendrite_target(0, 0, (1 << 24) | 999);
    state_buf.write_dendrite_weight(0, 0, 100000);

    state_buf.write_dendrite_target(1, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(1, 0, 100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 1; // min_d_ltd = 2 - 1 = 1
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    assert_eq!(state_buf.read_dendrite_weight(0, 0), 100000);
    assert!(state_buf.read_dendrite_weight(1, 0) < 100000);
}

#[test]
fn test_cpu_apply_gsop_negative_weight_keeps_sign() {
    let padded_n = 16;
    let total_axons = 16;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = test_variant_table();

    state_buf.write_soma_flags(0, 0x01);
    state_buf.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf.write_dendrite_weight(0, 0, -100000);

    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_dist = 0
    axon_buf.write_head(0, head);

    cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

    let new_weight = state_buf.read_dendrite_weight(0, 0);
    assert_eq!(new_weight, -100128);
}

#[test]
fn test_cpu_apply_gsop_dopamine_d1_d2_modulation() {
    let padded_n = 16;
    let total_axons = 16;
    let variants = test_variant_table();

    let mut state_buf_base = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf_base = MvpAxonBuffer::new(total_axons);
    state_buf_base.write_soma_flags(0, 0x01);
    state_buf_base.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf_base.write_dendrite_weight(0, 0, 100000);
    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_d_ltp = 0
    axon_buf_base.write_head(0, head);
    cpu_apply_gsop(&mut state_buf_base, &axon_buf_base, &variants, 0);

    let mut state_buf_dop = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf_dop = MvpAxonBuffer::new(total_axons);
    state_buf_dop.write_soma_flags(0, 0x01);
    state_buf_dop.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_buf_dop.write_dendrite_weight(0, 0, 100000);
    axon_buf_dop.write_head(0, head);
    cpu_apply_gsop(&mut state_buf_dop, &axon_buf_dop, &variants, 100);

    assert_eq!(state_buf_base.read_dendrite_weight(0, 0), 100128);
    assert_eq!(state_buf_dop.read_dendrite_weight(0, 0), 100038);
}

#[test]
fn test_cpu_apply_gsop_burst_multiplier() {
    let padded_n = 16;
    let total_axons = 16;
    let variants = test_variant_table();

    let mut state_single = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_single = MvpAxonBuffer::new(total_axons);
    state_single.write_soma_flags(0, 0x01);
    state_single.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_single.write_dendrite_weight(0, 0, 100000);
    let mut head = BurstHeads8::empty(AXON_SENTINEL);
    head.h0 = 2; // min_d_ltp = 0
    axon_single.write_head(0, head);
    cpu_apply_gsop(&mut state_single, &axon_single, &variants, 0);

    let mut state_burst = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_burst = MvpAxonBuffer::new(total_axons);
    state_burst.write_soma_flags(0, (3 << 1) | 0x01);
    state_burst.write_dendrite_target(0, 0, (2 << 24) | 1);
    state_burst.write_dendrite_weight(0, 0, 100000);
    axon_burst.write_head(0, head);
    cpu_apply_gsop(&mut state_burst, &axon_burst, &variants, 0);

    let delta_single = state_single.read_dendrite_weight(0, 0) - 100000;
    let delta_burst = state_burst.read_dendrite_weight(0, 0) - 100000;

    assert_eq!(delta_single, 128);
    assert_eq!(delta_burst, 384);
}

#[test]
fn test_research_apply_gsop_plasticity_bidirectional_stdp() {
    let inertia_curve = [128i32; 8];
    let weight = 100000;
    let prop = 20;

    // Scenario 1: Causal passed spike (min_d_ltp = 0, min_d_ltd = u32::MAX) -> Max LTP (+128)
    let w_ltp = research_apply_gsop_plasticity(
        weight,
        0,
        u32::MAX,
        prop,
        128,
        64,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_ltp - weight, 128);

    // Scenario 2: Anti-causal approaching spike (min_d_ltp = u32::MAX, min_d_ltd = 0) -> Max LTD (-64)
    let w_ltd = research_apply_gsop_plasticity(
        weight,
        u32::MAX,
        0,
        prop,
        128,
        64,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_ltd - weight, -64);

    // Scenario 3: Complete miss (min_d_ltp = prop + 1, min_d_ltd = prop + 1) -> No change (delta 0)
    let w_miss = research_apply_gsop_plasticity(
        weight,
        prop + 1,
        prop + 1,
        prop,
        128,
        64,
        0,
        0,
        0,
        1,
        &inertia_curve,
    );
    assert_eq!(w_miss - weight, 0);
}

#[test]
fn test_bidirectional_stdp_six_scenarios() {
    let padded_n = 16;
    let total_axons = 16;
    let mut variants = test_variant_table();
    variants[0].signal_propagation_length = 80; // prop = 80
    variants[0].gsop_potentiation = 128;
    variants[0].gsop_depression = 64;

    // seg_idx = 140, axon_id = 0 -> target = (140 << 24) | 1
    let target = (140 << 24) | 1;

    // 1. Scenario 1: no active spikes
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 100000);
    }

    // 2. Scenario 2: 8 spikes trailing from soma, first on segment 1, interval 10
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let mut axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        // Spikes behind (head < 140): 1, 11, 21, 31, 41, 51, 61, 71
        let mut head = BurstHeads8::empty(AXON_SENTINEL);
        head.h0 = 1;
        head.h1 = 11;
        head.h2 = 21;
        head.h3 = 31;
        head.h4 = 41;
        head.h5 = 51;
        head.h6 = 61;
        head.h7 = 71;
        axon_buf.write_head(0, head);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        // min_d_ltd = 140 - 71 = 69
        // decay_factor = 80 - 69 = 11
        // LTD delta = (64 * 128 * 11) / (128 * 80) = 8
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 99992);
    }

    // 3. Scenario 3: all 8 spikes ahead, closest immediately after synapse (140), interval 15
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let mut axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        // Spikes ahead (head >= 140): 140, 155, 170, 185, 200, 215, 230, 245
        let mut head = BurstHeads8::empty(AXON_SENTINEL);
        head.h0 = 140;
        head.h1 = 155;
        head.h2 = 170;
        head.h3 = 185;
        head.h4 = 200;
        head.h5 = 215;
        head.h6 = 230;
        head.h7 = 245;
        axon_buf.write_head(0, head);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        // min_d_ltp = 140 - 140 = 0
        // LTP delta = (128 * 128 * 80) / (128 * 80) = 128
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 100128);
    }

    // 4. Scenario 4: 4 spikes left and 4 right, closest at 5 segments, interval 20
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let mut axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        // Left (LTD): 135, 115, 95, 75
        // Right (LTP): 145, 165, 185, 205
        let mut head = BurstHeads8::empty(AXON_SENTINEL);
        head.h0 = 135;
        head.h1 = 115;
        head.h2 = 95;
        head.h3 = 75;
        head.h4 = 145;
        head.h5 = 165;
        head.h6 = 185;
        head.h7 = 205;
        axon_buf.write_head(0, head);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        // min_d_ltd = 140 - 135 = 5 -> decay = 75 -> LTD delta = (64 * 75) / 80 = 60
        // min_d_ltp = 145 - 140 = 5 -> decay = 75 -> LTP delta = (128 * 75) / 80 = 120
        // Total delta = 120 - 60 = 60
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 100060);
    }

    // 5. Scenario 5: 2 spikes left, 6 spikes right, closest at 15 segments, interval 30
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let mut axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        // Left (LTD): 125, 95
        // Right (LTP): 155, 185, 215, 245, 275, 305
        let mut head = BurstHeads8::empty(AXON_SENTINEL);
        head.h0 = 125;
        head.h1 = 95;
        head.h2 = 155;
        head.h3 = 185;
        head.h4 = 215;
        head.h5 = 245;
        head.h6 = 275;
        head.h7 = 305;
        axon_buf.write_head(0, head);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        // min_d_ltd = 15 -> decay = 65 -> LTD delta = 52
        // min_d_ltp = 15 -> decay = 65 -> LTP delta = 104
        // Total delta = 104 - 52 = 52
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 100052);
    }

    // 6. Scenario 6: 6 spikes left, 2 spikes right, closest at 15 segments, interval 30
    {
        let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
        let mut axon_buf = MvpAxonBuffer::new(total_axons);
        state_buf.write_soma_flags(0, 0x01);
        state_buf.write_dendrite_target(0, 0, target);
        state_buf.write_dendrite_weight(0, 0, 100000);

        // Left (LTD): 125, 95, 65, 35, 5, AXON_SENTINEL (placeholder for sixth)
        // Right (LTP): 155, 185
        let mut head = BurstHeads8::empty(AXON_SENTINEL);
        head.h0 = 125;
        head.h1 = 95;
        head.h2 = 65;
        head.h3 = 35;
        head.h4 = 5;
        head.h5 = AXON_SENTINEL;
        head.h6 = 155;
        head.h7 = 185;
        axon_buf.write_head(0, head);

        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);
        // Should be identical to Scenario 5
        assert_eq!(state_buf.read_dendrite_weight(0, 0), 100052);
    }
}
