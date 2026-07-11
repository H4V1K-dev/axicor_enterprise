use core::mem::{align_of, offset_of, size_of};
use wire::*;

#[test]
fn test_dto_sizes_and_alignments() {
    assert_eq!(size_of::<SpikeEventV2>(), 8);
    assert_eq!(align_of::<SpikeEventV2>(), 4);

    assert_eq!(size_of::<SpikeBatchHeaderV2>(), 16);
    assert_eq!(align_of::<SpikeBatchHeaderV2>(), 4);

    assert_eq!(size_of::<ExternalIoHeader>(), 20);
    assert_eq!(align_of::<ExternalIoHeader>(), 4);

    assert_eq!(size_of::<RouteUpdate>(), 24);
    assert_eq!(align_of::<RouteUpdate>(), 8);

    assert_eq!(size_of::<ControlPacket>(), 8);
    assert_eq!(align_of::<ControlPacket>(), 4);

    assert_eq!(size_of::<TelemetryFrameHeader>(), 16);
    assert_eq!(align_of::<TelemetryFrameHeader>(), 4);

    assert_eq!(size_of::<AxonHandoverEvent>(), 20);
    assert_eq!(align_of::<AxonHandoverEvent>(), 4);

    assert_eq!(size_of::<AxonHandoverAck>(), 16);
    assert_eq!(align_of::<AxonHandoverAck>(), 4);

    assert_eq!(size_of::<AxonHandoverPrune>(), 12);
    assert_eq!(align_of::<AxonHandoverPrune>(), 4);
}

#[test]
fn test_magic_constants_le() {
    assert_eq!(MAGIC_GSIO, u32::from_le_bytes(*b"GSIO"));
    assert_eq!(MAGIC_GSOO, u32::from_le_bytes(*b"GSOO"));
    assert_eq!(MAGIC_ROUT, u32::from_le_bytes(*b"ROUT"));
    assert_eq!(MAGIC_DOPA, u32::from_le_bytes(*b"DOPA"));
    assert_eq!(MAGIC_TELE, u32::from_le_bytes(*b"TELE"));
}

#[test]
fn test_external_io_header_size() {
    assert_eq!(size_of::<ExternalIoHeader>(), 20);
}

#[test]
fn test_payload_size_mismatch_rejected() {
    let packet = [0u8; 30];
    let res = payload_slice(&packet, 20, 15);
    assert_eq!(res, Err(WireError::PayloadSizeMismatch));

    let res_ok = payload_slice(&packet, 20, 10);
    assert!(res_ok.is_ok());
    assert_eq!(res_ok.unwrap().len(), 10);
}

#[test]
fn test_unaligned_buffer_safe_read() {
    let mut buffer = [0u8; 100];
    let header = ExternalIoHeader::new(MAGIC_GSIO, 0x11223344, 0x55667788, 50, 5);

    // Copy to buffer at unaligned offset (1)
    let bytes = bytemuck::bytes_of(&header);
    buffer[1..1 + bytes.len()].copy_from_slice(bytes);

    // Read unaligned
    let read_header: ExternalIoHeader = try_read_header(&buffer[1..]).unwrap();
    assert_eq!(read_header.magic, MAGIC_GSIO);
    assert_eq!(read_header.zone_hash, 0x11223344);
    assert_eq!(read_header.matrix_hash, 0x55667788);
    assert_eq!(read_header.payload_size, 50);
    assert_eq!(read_header.global_reward, 5);
}

#[test]
fn test_spike_events_array_alignment() {
    let count = 10;
    let len = spike_events_payload_len(count).unwrap();
    assert_eq!(len, 80);
    assert_eq!(len % 8, 0);
}

#[test]
fn test_spike_batch_header_semantics() {
    // Heartbeat: total_chunks == 0 && chunk_idx == 0
    let heartbeat = SpikeBatchHeaderV2::new(0, 0, 1, 0, 0);
    assert_eq!(heartbeat.chunk_idx, 0);
    assert_eq!(heartbeat.total_chunks, 0);

    // ACK: chunk_idx == 0xFFFF && total_chunks == 0
    let ack = SpikeBatchHeaderV2::new(0, 0, 1, 0xFFFF, 0);
    assert_eq!(ack.chunk_idx, 0xFFFF);
    assert_eq!(ack.total_chunks, 0);
}

#[test]
fn test_route_update_layout() {
    assert_eq!(size_of::<RouteUpdate>(), 24);
    assert_eq!(offset_of!(RouteUpdate, cluster_secret), 16);
}

#[test]
fn test_zeroed_padding_on_construct() {
    let header = ExternalIoHeader::new(MAGIC_GSIO, 1, 2, 3, 4);
    assert_eq!(header._padding, 0);

    let ctrl = ControlPacket::new(10);
    assert_eq!(ctrl._pad, 0);

    let tele = TelemetryFrameHeader::new(100, 20);
    assert_eq!(tele._padding, 0);

    let event = AxonHandoverEvent::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    assert_eq!(event._padding, 0);
}

#[test]
fn test_helpers_zero_allocations() {
    let header = ExternalIoHeader::new(MAGIC_GSIO, 1, 2, 3, 4);
    let bytes = bytemuck::bytes_of(&header);

    // We call helpers. Since we are no_std, they cannot allocate.
    // We just verify they work as expected.
    let read: ExternalIoHeader = try_read_header(bytes).unwrap();
    assert_eq!(read.magic, MAGIC_GSIO);
    assert!(validate_external_io_len(&read, 23).is_ok());
    assert!(validate_external_io_len(&read, 24).is_err());
    assert_eq!(wire_size_of::<ExternalIoHeader>(), 20);
}
