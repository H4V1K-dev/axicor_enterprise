use crate::error::ProtocolError;

/// Verdict returned by the biological epoch validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochAction {
    /// The packet is valid, epoch matches or is within tolerance.
    Accept,
    /// The packet is from the past (Biological Amnesia). Must be dropped.
    AmnesiaDrop,
    /// The packet is from the distant future (Self-Healing). Force local epoch fast-forward.
    SelfHealingFastForward(u32),
}

/// Default tolerance (jitter window) for network epochs.
pub const DEFAULT_TOLERANCE: u32 = 5;

/// Default threshold (fast-forward window) for initiating Self-Healing.
pub const DEFAULT_SELF_HEALING_THRESHOLD: u32 = 100;

/// Biological validation of time.
///
/// Uses the default tolerance and self-healing threshold parameters.
#[inline]
pub fn validate_epoch(packet_epoch: u32, node_epoch: u32) -> EpochAction {
    validate_epoch_math(
        packet_epoch,
        node_epoch,
        DEFAULT_TOLERANCE,
        DEFAULT_SELF_HEALING_THRESHOLD,
    )
}

/// Biological validation of time with custom tolerance and self-healing threshold.
///
/// Under INV-PROTO-003, network packets from the past are dropped to preserve causality,
/// while packets from the distant future trigger node self-healing.
#[inline]
pub fn validate_epoch_math(
    packet_epoch: u32,
    node_epoch: u32,
    tolerance: u32,
    self_healing_threshold: u32,
) -> EpochAction {
    if packet_epoch == node_epoch {
        return EpochAction::Accept;
    }

    // Wrap-around subtraction handles u32 ring overflow correctly
    let delta = packet_epoch.wrapping_sub(node_epoch);

    // If the 31st bit is 1, then packet_epoch < node_epoch (mathematically in the past)
    if (delta & 0x8000_0000) != 0 {
        let past_diff = node_epoch.wrapping_sub(packet_epoch);
        if past_diff <= tolerance {
            EpochAction::Accept // Slight lag allowed to compensate for network jitter (E-113)
        } else {
            EpochAction::AmnesiaDrop // Too old, trigger biological amnesia (E-114)
        }
    } else {
        // Otherwise the packet is in the future
        if delta > self_healing_threshold {
            EpochAction::SelfHealingFastForward(packet_epoch) // Node is lagging, trigger self-healing (E-115)
        } else {
            EpochAction::Accept // Slightly ahead, accept and buffer
        }
    }
}

/// Computes the number of L7 chunks and spikes per chunk for a given MTU.
///
/// Under INV-PROTO-004, the generator must not produce fragments exceeding MTU.
///
/// # Errors
/// Returns `ProtocolError::InvalidMtu` if MTU is less than 24 bytes (header size 16 + at least 1 spike of size 8),
/// or `ProtocolError::InvalidChunkCount` if the total chunks exceed 1024.
#[inline]
pub fn calculate_fragmentation(
    mtu: usize,
    total_spikes: usize,
) -> Result<(usize, u16), ProtocolError> {
    let header_size = 16;
    let event_size = 8;
    let min_required = header_size + event_size; // 24

    if mtu < min_required {
        return Err(ProtocolError::InvalidMtu {
            mtu,
            min_required,
        });
    }

    // Determine how many spikes fit into one UDP packet payload
    let max_spikes = (mtu - header_size) / event_size;

    // Fast integer division rounding up
    let total_chunks = if total_spikes == 0 {
        1
    } else {
        (total_spikes + max_spikes - 1) / max_spikes
    };

    if total_chunks > 1024 {
        return Err(ProtocolError::InvalidChunkCount {
            total_chunks: total_chunks as u16,
            max_allowed: 1024,
        });
    }

    Ok((max_spikes, total_chunks as u16))
}

/// Verifies that a buffer satisfies size and alignment requirements for zero-copy casting.
///
/// # Why `align - 1` is used:
/// `bytemuck`'s slice casting requires that the memory address of the input buffer is a multiple of the
/// target type's alignment. To check this on heterogenous platforms (e.g. ARM/ESP32) and prevent panics from
/// within `bytemuck` (or hardware level alignment exceptions), we verify that `address % align == 0`.
/// Because the alignment of any standard Rust type is guaranteed to be a power of two, we can optimize the
/// modulo check `address & (align - 1)` to run in a single CPU cycle instead of using division.
/// If `(buf.as_ptr() as usize) & (align - 1) != 0`, we return `ProtocolError::AlignmentMismatch` rather than
/// letting `bytemuck` panic or trigger an hardware exception.
///
/// # Errors
/// - `ProtocolError::AlignmentMismatch` if the buffer is not aligned properly in memory.
/// - `ProtocolError::BufferTooSmall` if the buffer length is not a multiple of `size_of::<T>()`.
#[inline]
pub fn verify_cast_guards<T>(buf: &[u8]) -> Result<(), ProtocolError> {
    let align = core::mem::align_of::<T>();
    let size = core::mem::size_of::<T>();

    // Pointer alignment check via bitwise mask
    if (buf.as_ptr() as usize) & (align - 1) != 0 {
        return Err(ProtocolError::AlignmentMismatch);
    }

    // Payload size multiple check
    if buf.len() % size != 0 {
        return Err(ProtocolError::BufferTooSmall {
            expected: size,
            actual: buf.len(),
        });
    }

    Ok(())
}

/// Iterator for slicing a spike batch into MTU-compliant fragments.
#[derive(Debug, Clone)]
pub struct SpikeFragmentIterator<'a> {
    header: wire::SpikeBatchHeaderV2,
    spikes: &'a [wire::SpikeEventV2],
    max_spikes_per_chunk: usize,
    total_chunks: u16,
    current_chunk: u16,
}

impl<'a> Iterator for SpikeFragmentIterator<'a> {
    type Item = (wire::SpikeBatchHeaderV2, &'a [wire::SpikeEventV2]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_chunk >= self.total_chunks {
            return None;
        }

        let start = (self.current_chunk as usize) * self.max_spikes_per_chunk;
        let end = core::cmp::min(start + self.max_spikes_per_chunk, self.spikes.len());
        let chunk_spikes = &self.spikes[start..end];

        let mut chunk_header = self.header;
        chunk_header.chunk_idx = self.current_chunk;
        chunk_header.total_chunks = self.total_chunks;

        self.current_chunk += 1;
        Some((chunk_header, chunk_spikes))
    }
}

/// Prepares a spike batch for MTU fragmentation and returns a zero-copy iterator.
#[inline]
pub fn fragment_spikes<'a>(
    header: wire::SpikeBatchHeaderV2,
    spikes: &'a [wire::SpikeEventV2],
    mtu: usize,
) -> Result<SpikeFragmentIterator<'a>, ProtocolError> {
    let (max_spikes, total_chunks) = calculate_fragmentation(mtu, spikes.len())?;
    Ok(SpikeFragmentIterator {
        header,
        spikes,
        max_spikes_per_chunk: max_spikes,
        total_chunks,
        current_chunk: 0,
    })
}

/// Zero-copy decode a spike batch (header and spikes) from raw bytes.
#[inline]
pub fn decode_spike_batch(
    buf: &[u8],
) -> Result<(wire::SpikeBatchHeaderV2, &[wire::SpikeEventV2]), ProtocolError> {
    let header_size = core::mem::size_of::<wire::SpikeBatchHeaderV2>();
    if buf.len() < header_size {
        return Err(ProtocolError::BufferTooSmall {
            expected: header_size,
            actual: buf.len(),
        });
    }

    verify_cast_guards::<wire::SpikeBatchHeaderV2>(&buf[..header_size])?;
    let header = bytemuck::pod_read_unaligned::<wire::SpikeBatchHeaderV2>(&buf[..header_size]);
    let header = header.from_le();

    let payload = &buf[header_size..];
    verify_cast_guards::<wire::SpikeEventV2>(payload)?;
    let spikes = bytemuck::cast_slice::<u8, wire::SpikeEventV2>(payload);

    Ok((header, spikes))
}

/// Serialize a spike batch header and spikes into the output buffer with Little-Endian enforcement.
#[inline]
pub fn encode_spike_batch(
    header: &wire::SpikeBatchHeaderV2,
    spikes: &[wire::SpikeEventV2],
    buf: &mut [u8],
) -> Result<usize, ProtocolError> {
    let header_size = core::mem::size_of::<wire::SpikeBatchHeaderV2>();
    let event_size = core::mem::size_of::<wire::SpikeEventV2>();
    let required_size = header_size + spikes.len() * event_size;

    if buf.len() < required_size {
        return Err(ProtocolError::BufferTooSmall {
            expected: required_size,
            actual: buf.len(),
        });
    }

    // Write header in little-endian order
    let le_header = header.to_le();
    buf[..header_size].copy_from_slice(bytemuck::bytes_of(&le_header));

    // Write spikes in little-endian order
    for (i, spike) in spikes.iter().enumerate() {
        let le_spike = spike.to_le();
        let offset = header_size + i * event_size;
        buf[offset..offset + event_size].copy_from_slice(bytemuck::bytes_of(&le_spike));
    }

    Ok(required_size)
}

/// Verification of cluster secret for routing tables updates.
#[inline]
pub fn validate_cluster_secret(
    packet_secret: u64,
    expected_secret: u64,
) -> Result<(), ProtocolError> {
    if packet_secret == expected_secret {
        Ok(())
    } else {
        Err(ProtocolError::AuthFailure)
    }
}

/// Decodes an external I/O packet (sensors, motors) with zero-copy.
#[inline]
pub fn decode_io_packet(
    buf: &[u8],
) -> Result<(wire::ExternalIoHeader, &[u8]), ProtocolError> {
    let header_size = core::mem::size_of::<wire::ExternalIoHeader>();
    if buf.len() < header_size {
        return Err(ProtocolError::BufferTooSmall {
            expected: header_size,
            actual: buf.len(),
        });
    }

    verify_cast_guards::<wire::ExternalIoHeader>(&buf[..header_size])?;
    let header = bytemuck::pod_read_unaligned::<wire::ExternalIoHeader>(&buf[..header_size]);
    let header = header.from_le();

    let payload = &buf[header_size..];
    if payload.len() < header.payload_size as usize {
        return Err(ProtocolError::BufferTooSmall {
            expected: header.payload_size as usize,
            actual: payload.len(),
        });
    }

    Ok((header, &payload[..header.payload_size as usize]))
}
