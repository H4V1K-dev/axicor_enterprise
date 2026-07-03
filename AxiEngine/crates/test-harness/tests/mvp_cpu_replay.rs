//! Conformance unit tests for `mvp-cpu-replay` state plane access and axon head structures.

#![cfg(feature = "mvp-cpu-replay")]

use layout::{
    compute_state_offsets, BurstHeads8, AXONS_FILE_VERSION, AXONS_MAGIC, STATE_FILE_VERSION,
    STATE_MAGIC,
};
use test_harness::{MvpAxonBuffer, MvpStateBuffer};
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
