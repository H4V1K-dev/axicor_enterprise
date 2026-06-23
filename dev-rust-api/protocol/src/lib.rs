#![no_std]

extern crate alloc;

pub mod error;
pub mod math;
pub mod reassembly;

pub use error::*;
pub use math::*;// (Note: EpochAction, validate_epoch_math, validate_epoch, calculate_fragmentation, verify_cast_guards)
pub use reassembly::*;

#[cfg(test)]
mod tests {
    use super::*;
    use wire::{SpikeBatchHeaderV2, SpikeEventV2};

    #[test]
    fn test_epoch_validator_drops_old() {
        // E-114: Packet from the past exceeds tolerance -> AmnesiaDrop
        let action = validate_epoch_math(10, 20, 5, 100);
        assert_eq!(action, EpochAction::AmnesiaDrop);

        // Within tolerance -> Accept
        let action = validate_epoch_math(17, 20, 5, 100);
        assert_eq!(action, EpochAction::Accept);

        // Future within threshold -> Accept
        let action = validate_epoch_math(25, 20, 5, 100);
        assert_eq!(action, EpochAction::Accept);

        // Future exceeds threshold -> SelfHealingFastForward
        let action = validate_epoch_math(150, 20, 5, 100);
        assert_eq!(action, EpochAction::SelfHealingFastForward(150));

        // u32 Wrap-around check: packet is 0xFFFF_FFFA (old), node is 5, tolerance is 5
        // node_epoch - packet_epoch = 5 - (-6) = 11 > 5 -> AmnesiaDrop
        let action = validate_epoch_math(0xFFFF_FFFA, 5, 5, 100);
        assert_eq!(action, EpochAction::AmnesiaDrop);

        // u32 Wrap-around check: packet is 0xFFFF_FFFD (old but within tolerance), node is 2, tolerance is 5
        // node_epoch - packet_epoch = 2 - (-3) = 5 <= 5 -> Accept
        let action = validate_epoch_math(0xFFFF_FFFD, 2, 5, 100);
        assert_eq!(action, EpochAction::Accept);
    }

    #[test]
    fn test_verify_cast_alignment() {
        // Allocate a well-aligned buffer (e.g. alignment of 4 or 8)
        #[repr(align(8))]
        struct AlignedBuf([u8; 32]);
        let buf = AlignedBuf([0; 32]);

        // buf.0[0..8] is aligned to 8-byte boundary, which is valid for SpikeEventV2 (align 4)
        let res = verify_cast_guards::<SpikeEventV2>(&buf.0[0..8]);
        assert!(res.is_ok());

        // buf.0[1..9] is misaligned (address is odd) -> AlignmentMismatch
        let res = verify_cast_guards::<SpikeEventV2>(&buf.0[1..9]);
        assert_eq!(res, Err(error::ProtocolError::AlignmentMismatch));

        // buf.0[0..7] is not a multiple of size (8) -> BufferTooSmall
        let res = verify_cast_guards::<SpikeEventV2>(&buf.0[0..7]);
        assert_eq!(res, Err(error::ProtocolError::BufferTooSmall { expected: 8, actual: 7 }));
    }

    #[test]
    fn test_reassembly_duplicates() {
        let mut buffer = ReassemblyBuffer::new(4);
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 100,
            chunk_idx: 0,
            total_chunks: 2,
        };

        let spikes = [
            SpikeEventV2 { ghost_id: 1, tick_offset: 10 },
            SpikeEventV2 { ghost_id: 2, tick_offset: 20 },
        ];
        let payload = bytemuck::cast_slice::<SpikeEventV2, u8>(&spikes);

        // First insert of chunk 0 should succeed
        let res1 = buffer.insert_chunk(&header, payload);
        assert!(res1.is_ok());
        assert_eq!(res1.unwrap(), None);

        // Second insert of chunk 0 (duplicate) should fail with DuplicateFragment (E-111)
        let res2 = buffer.insert_chunk(&header, payload);
        assert!(matches!(res2, Err(error::ProtocolError::DuplicateFragment { batch_id: 42, chunk_idx: 0 })));

        // Ensure received_chunks is still 1
        assert_eq!(buffer.slots[0].received_chunks, 1);
    }

    #[test]
    fn test_reassembly_complete_out_of_order() {
        let mut buffer = ReassemblyBuffer::new(2);
        let header_chunk_1 = SpikeBatchHeaderV2 {
            src_zone_hash: 100,
            dst_zone_hash: 200,
            epoch: 50,
            chunk_idx: 1,
            total_chunks: 2,
        };
        let header_chunk_0 = SpikeBatchHeaderV2 {
            src_zone_hash: 100,
            dst_zone_hash: 200,
            epoch: 50,
            chunk_idx: 0,
            total_chunks: 2,
        };

        let spikes_0 = [
            SpikeEventV2 { ghost_id: 1, tick_offset: 10 },
            SpikeEventV2 { ghost_id: 2, tick_offset: 20 },
        ];
        let spikes_1 = [
            SpikeEventV2 { ghost_id: 3, tick_offset: 30 },
        ];

        let payload_0 = bytemuck::cast_slice::<SpikeEventV2, u8>(&spikes_0);
        let payload_1 = bytemuck::cast_slice::<SpikeEventV2, u8>(&spikes_1);

        // Insert chunk 1 (last chunk) first
        let res1 = buffer.insert_chunk(&header_chunk_1, payload_1).unwrap();
        assert!(res1.is_none());

        // Insert chunk 0 next
        let res2 = buffer.insert_chunk(&header_chunk_0, payload_0).unwrap();
        assert!(res2.is_some());

        let full_batch = res2.unwrap();
        assert_eq!(full_batch.len(), 3);
        assert_eq!(full_batch[0], spikes_0[0]);
        assert_eq!(full_batch[1], spikes_0[1]);
        assert_eq!(full_batch[2], spikes_1[0]);
    }

    #[test]
    fn test_validate_cluster_secret() {
        // E-116: Mismatch of cluster secret returns AuthFailure
        assert_eq!(validate_cluster_secret(12345, 12345), Ok(()));
        assert_eq!(validate_cluster_secret(12345, 67890), Err(ProtocolError::AuthFailure));
    }

    #[test]
    fn test_decode_io_packet() {
        let header = wire::ExternalIoHeader {
            magic: *b"IOPH",
            zone_hash: 1,
            matrix_hash: 2,
            payload_size: 4,
            global_reward: 10,
            _padding: 0,
        };
        let header_bytes = bytemuck::bytes_of(&header);
        let payload = [1u8, 2, 3, 4];

        let mut packet = alloc::vec::Vec::new();
        packet.extend_from_slice(header_bytes);
        packet.extend_from_slice(&payload);

        let (decoded_header, decoded_payload) = decode_io_packet(&packet).unwrap();
        assert_eq!(decoded_header.zone_hash, 1);
        assert_eq!(decoded_header.matrix_hash, 2);
        assert_eq!(decoded_header.payload_size, 4);
        assert_eq!(decoded_header.global_reward, 10);
        assert_eq!(decoded_payload, &payload[..]);
    }

    #[test]
    fn test_zero_alloc_hot_path() {
        // INV-PROTO-001: Ensure zero heap allocations during serialization and fragmentation
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 1,
            dst_zone_hash: 2,
            epoch: 3,
            chunk_idx: 0,
            total_chunks: 1,
        };
        let spikes = [SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let mut buf = [0u8; 100];
        let size = encode_spike_batch(&header, &spikes, &mut buf).unwrap();

        let (decoded_header, decoded_spikes) = decode_spike_batch(&buf[..size]).unwrap();
        assert_eq!(decoded_header.src_zone_hash, 1);
        assert_eq!(decoded_spikes.len(), 1);

        let fragments: alloc::vec::Vec<_> = fragment_spikes(header, &spikes, 1400).unwrap().collect();
        assert_eq!(fragments.len(), 1);
    }

    #[test]
    fn test_invalid_mtu_limits() {
        // E-108: MTU below minimum required (24 bytes) returns InvalidMtu
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 1,
            dst_zone_hash: 2,
            epoch: 3,
            chunk_idx: 0,
            total_chunks: 1,
        };
        let spikes = [SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let res = fragment_spikes(header, &spikes, 23);
        assert!(matches!(res, Err(ProtocolError::InvalidMtu { mtu: 23, min_required: 24 })));
    }

    #[test]
    fn test_reassembly_oob() {
        // E-106: Fragment index >= total_chunks returns InvalidFragmentIndex
        let mut buffer = ReassemblyBuffer::new(2);
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 100,
            chunk_idx: 5,
            total_chunks: 2,
        };
        let spikes = [SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let payload = bytemuck::cast_slice::<SpikeEventV2, u8>(&spikes);
        let res = buffer.insert_chunk(&header, payload);
        assert!(matches!(res, Err(ProtocolError::InvalidFragmentIndex { index: 5, total: 2 })));
    }

    #[test]
    fn test_reassembly_buffer_full() {
        // D-029: Reassembly buffer full of active slots returns ReassemblyBufferFull
        let mut buffer = ReassemblyBuffer::new(1);
        let header1 = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 100,
            chunk_idx: 0,
            total_chunks: 2,
        };
        let spikes = [SpikeEventV2 { ghost_id: 1, tick_offset: 10 }];
        let payload = bytemuck::cast_slice::<SpikeEventV2, u8>(&spikes);

        buffer.insert_chunk(&header1, payload).unwrap();

        let header2 = SpikeBatchHeaderV2 {
            src_zone_hash: 99,
            dst_zone_hash: 43,
            epoch: 100,
            chunk_idx: 0,
            total_chunks: 2,
        };
        let res = buffer.insert_chunk(&header2, payload);
        assert_eq!(res.err(), Some(ProtocolError::ReassemblyBufferFull));
    }

    #[test]
    fn test_batch_capacity_bounds() {
        // E-112: Estimated spikes exceed MAX_BATCH_SPIKES returns BatchCapacityExceeded
        let mut buffer = ReassemblyBuffer::new(1);
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 42,
            dst_zone_hash: 43,
            epoch: 100,
            chunk_idx: 0,
            total_chunks: 1,
        };
        let large_payload = alloc::vec![0u8; (MAX_BATCH_SPIKES + 1) * 8];
        let res = buffer.insert_chunk(&header, &large_payload);
        assert!(matches!(res, Err(ProtocolError::BatchCapacityExceeded { .. })));
    }
}

