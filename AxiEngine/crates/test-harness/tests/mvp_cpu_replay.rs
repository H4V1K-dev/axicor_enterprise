//! Conformance unit tests for `mvp-cpu-replay` state plane access and axon head structures.

#![cfg(feature = "mvp-cpu-replay")]

use layout::{
    compute_state_offsets, BurstHeads8, AXONS_FILE_VERSION, AXONS_MAGIC, STATE_FILE_VERSION,
    STATE_MAGIC,
};
use test_harness::{
    cpu_apply_spike_batch, cpu_extract_telemetry, cpu_inject_inputs, cpu_propagate_axons,
    cpu_record_outputs, MvpAxonBuffer, MvpStateBuffer,
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
