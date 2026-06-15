#![no_std]

pub mod macros;

pub mod error;
pub mod fast_path;
pub mod io;

pub use error::*;
pub use fast_path::*;
pub use io::*;

/// Helper trait for zero-cost casting of POD types.
pub trait WireCast: bytemuck::Pod + bytemuck::Zeroable {
    /// Safe O(1) casting of a single struct to a raw byte slice.
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Safe O(1) casting of a struct slice to a raw byte slice.
    #[inline]
    fn slice_as_bytes(slice: &[Self]) -> &[u8] {
        bytemuck::cast_slice(slice)
    }

    /// Safe O(1) casting of a raw byte slice to a single struct reference.
    #[inline]
    fn from_bytes(buf: &[u8]) -> Result<&Self, WireError> {
        if buf.len() < core::mem::size_of::<Self>() {
            return Err(WireError::BufferTooSmall {
                expected: core::mem::size_of::<Self>(),
                actual: buf.len(),
            });
        }
        if buf.as_ptr() as usize % core::mem::align_of::<Self>() != 0 {
            return Err(WireError::AlignmentMismatch);
        }
        bytemuck::try_from_bytes(&buf[..core::mem::size_of::<Self>()])
            .map_err(|_| WireError::AlignmentMismatch)
    }

    /// Safe O(1) casting of a raw byte slice to a slice of structs.
    #[inline]
    fn slice_from_bytes(buf: &[u8]) -> Result<&[Self], WireError> {
        let size = core::mem::size_of::<Self>();
        if size == 0 {
            return Err(WireError::ValidationError("Zero-sized type not allowed for slice cast"));
        }
        if buf.len() % size != 0 {
            return Err(WireError::BufferTooSmall {
                expected: size,
                actual: buf.len(),
            });
        }
        if buf.as_ptr() as usize % core::mem::align_of::<Self>() != 0 {
            return Err(WireError::AlignmentMismatch);
        }
        bytemuck::try_cast_slice(buf)
            .map_err(|_| WireError::AlignmentMismatch)
    }
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> WireCast for T {}

// =============================================================================
// Compile-time Invariant Assertions (INV-WIRE-*)
// =============================================================================
const _: () = {
    // INV-WIRE-001: size_of and align_of constraints
    assert!(core::mem::size_of::<SpikeBatchHeaderV2>() == 16);
    assert!(core::mem::align_of::<SpikeBatchHeaderV2>() == 4);

    assert!(core::mem::size_of::<SpikeEventV2>() == 8);
    assert!(core::mem::align_of::<SpikeEventV2>() == 4);

    assert!(core::mem::size_of::<ExternalIoHeader>() == 20);
    assert!(core::mem::align_of::<ExternalIoHeader>() == 4);

    assert!(core::mem::size_of::<ControlPacket>() == 8);
    assert!(core::mem::align_of::<ControlPacket>() == 8);

    assert!(core::mem::size_of::<AxonHandoverEvent>() == 20);
    assert!(core::mem::align_of::<AxonHandoverEvent>() == 4);

    assert!(core::mem::size_of::<AxonHandoverPrune>() == 12);
    assert!(core::mem::align_of::<AxonHandoverPrune>() == 4);

    assert!(core::mem::size_of::<BakeRequest>() == 16);
    assert!(core::mem::align_of::<BakeRequest>() == 4);

    assert!(core::mem::size_of::<AxonHandoverAck>() == 16);
    assert!(core::mem::align_of::<AxonHandoverAck>() == 4);

    assert!(core::mem::size_of::<TelemetryFrameHeader>() == 16);
    assert!(core::mem::align_of::<TelemetryFrameHeader>() == 8);
};

// =============================================================================
// Unit Tests Block
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_telemetry_frame_header_layout() {
        assert_eq!(size_of::<TelemetryFrameHeader>(), 16);
        assert_eq!(align_of::<TelemetryFrameHeader>(), 8);
    }

    #[test]
    fn test_spike_batch_header_v2_layout() {
        assert_eq!(size_of::<SpikeBatchHeaderV2>(), 16);
        assert_eq!(align_of::<SpikeBatchHeaderV2>(), 4);
    }

    #[test]
    fn test_spike_event_v2_layout() {
        assert_eq!(size_of::<SpikeEventV2>(), 8);
        assert_eq!(align_of::<SpikeEventV2>(), 4);
    }

    #[test]
    fn test_external_io_header_layout() {
        assert_eq!(size_of::<ExternalIoHeader>(), 20);
        assert_eq!(align_of::<ExternalIoHeader>(), 4);
    }

    #[test]
    fn test_control_packet_layout() {
        assert_eq!(size_of::<ControlPacket>(), 8);
        assert_eq!(align_of::<ControlPacket>(), 8);
    }

    #[test]
    fn test_endian_roundtrip() {
        let header = SpikeBatchHeaderV2 {
            src_zone_hash: 0x12345678,
            dst_zone_hash: 0x9ABCDEF0,
            epoch: 0x11223344,
            chunk_idx: 0x5566,
            total_chunks: 0x7788,
        };
        let le_header = header.to_le();
        let recon = le_header.from_le();
        assert_eq!(header, recon);

        let event = SpikeEventV2 {
            ghost_id: 0x12345678,
            tick_offset: 0x9ABCDEF0,
        };
        assert_eq!(event.to_le().from_le(), event);

        let io_header = ExternalIoHeader {
            magic: *b"GSIO",
            zone_hash: 0x12345678,
            matrix_hash: 0x9ABCDEF0,
            payload_size: 0x11223344,
            global_reward: 0x5566,
            _padding: 0,
        };
        assert_eq!(io_header.to_le().from_le(), io_header);

        let ctrl = ControlPacket {
            magic: *b"DOPA",
            dopamine: 0x1234,
            _pad: 0,
        };
        assert_eq!(ctrl.to_le().from_le(), ctrl);
    }

    #[test]
    fn test_wire_cast_single() {
        let event = SpikeEventV2 {
            ghost_id: 42,
            tick_offset: 100,
        };
        let bytes = event.as_bytes();
        assert_eq!(bytes.len(), 8);

        let casted = SpikeEventV2::from_bytes(bytes).unwrap();
        assert_eq!(casted, &event);
    }

    #[test]
    fn test_wire_cast_slice() {
        let events = [
            SpikeEventV2 { ghost_id: 1, tick_offset: 10 },
            SpikeEventV2 { ghost_id: 2, tick_offset: 20 },
        ];
        let bytes = SpikeEventV2::slice_as_bytes(&events);
        assert_eq!(bytes.len(), 16);

        let casted = SpikeEventV2::slice_from_bytes(bytes).unwrap();
        assert_eq!(casted, &events);
    }

    #[test]
    fn test_wire_cast_errors() {
        let bytes = [0u8; 7];
        let cast_err = SpikeEventV2::from_bytes(&bytes);
        assert!(matches!(cast_err, Err(WireError::BufferTooSmall { expected: 8, actual: 7 })));

        let slice_err = SpikeEventV2::slice_from_bytes(&bytes);
        assert!(matches!(slice_err, Err(WireError::BufferTooSmall { expected: 8, actual: 7 })));
    }
}
