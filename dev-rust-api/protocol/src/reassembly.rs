use crate::error::ProtocolError;

/// Maximum spikes in a single batch (E-112).
pub const MAX_BATCH_SPIKES: usize = 65536;

/// Flat session slot for reassembling a fragmented packet.
pub struct ReassemblySlot {
    pub src_zone_hash: u32,
    pub epoch: u32,
    pub total_chunks: u16,
    pub received_chunks: u16,
    /// Bitmask of received chunks (16 * 64 = 1024 chunks maximum). Zero-alloc!
    pub chunk_mask: [u64; 16],
    /// Flat pre-allocated buffer for spikes of the current batch.
    pub payload_buffer: alloc::vec::Vec<wire::SpikeEventV2>,
    /// Track the maximum spikes per chunk (determined from any full chunk).
    pub max_spikes_per_chunk: usize,
    /// Track the size of the last chunk.
    pub last_chunk_len: usize,
    /// Temporary storage for the last chunk's spikes if it arrives before max_spikes_per_chunk is known.
    pub last_chunk_spikes: alloc::vec::Vec<wire::SpikeEventV2>,
}

impl ReassemblySlot {
    /// Create a new empty slot.
    pub fn new() -> Self {
        Self {
            src_zone_hash: 0,
            epoch: 0,
            total_chunks: 0,
            received_chunks: 0,
            chunk_mask: [0; 16],
            payload_buffer: alloc::vec![wire::SpikeEventV2 { ghost_id: 0, tick_offset: 0 }; MAX_BATCH_SPIKES],
            max_spikes_per_chunk: 0,
            last_chunk_len: 0,
            last_chunk_spikes: alloc::vec![wire::SpikeEventV2 { ghost_id: 0, tick_offset: 0 }; MAX_BATCH_SPIKES],
        }
    }

    /// Reset slot state in-place for a new reassembly session, preserving vector capacities.
    pub fn reset(&mut self, src_zone_hash: u32, epoch: u32, total_chunks: u16) {
        self.src_zone_hash = src_zone_hash;
        self.epoch = epoch;
        self.total_chunks = total_chunks;
        self.received_chunks = 0;
        self.chunk_mask = [0; 16];
        self.max_spikes_per_chunk = 0;
        self.last_chunk_len = 0;
    }
}

impl Default for ReassemblySlot {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-allocated ring buffer for reassembling L7 chunks.
pub struct ReassemblyBuffer {
    /// Fixed pool of reassembly slots.
    pub slots: alloc::vec::Vec<ReassemblySlot>,
}

impl ReassemblyBuffer {
    /// Create a new reassembly buffer with a fixed number of slots.
    pub fn new(capacity: usize) -> Self {
        let mut slots = alloc::vec::Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(ReassemblySlot::new());
        }
        Self { slots }
    }

    /// Evict (reset) a slot in the buffer, discarding any incomplete reassembly.
    pub fn evict_slot(&mut self, idx: usize) {
        if idx < self.slots.len() {
            self.slots[idx].reset(0, 0, 0);
        }
    }

    /// Records the receipt of a chunk index inside a slot's bitmask in O(1) time.
    ///
    /// # Errors
    /// - `ProtocolError::InvalidFragmentIndex` if chunk_idx >= total_chunks.
    /// - `ProtocolError::DuplicateFragment` if this chunk index was already received.
    pub fn process_chunk(slot: &mut ReassemblySlot, chunk_idx: u16) -> Result<(), ProtocolError> {
        if chunk_idx >= slot.total_chunks {
            return Err(ProtocolError::InvalidFragmentIndex {
                index: chunk_idx,
                total: slot.total_chunks,
            });
        }

        let mask_idx = (chunk_idx >> 6) as usize;
        let bit_idx = chunk_idx & 63;

        if (slot.chunk_mask[mask_idx] & (1u64 << bit_idx)) != 0 {
            return Err(ProtocolError::DuplicateFragment {
                batch_id: slot.src_zone_hash,
                chunk_idx,
            });
        }

        slot.chunk_mask[mask_idx] |= 1u64 << bit_idx;
        slot.received_chunks += 1;

        Ok(())
    }

    /// Insert an L7 chunk into the buffer and return a slice of the full batch if reassembly completed.
    ///
    /// # Errors
    /// - `ProtocolError::AlignmentMismatch` or `ProtocolError::BufferTooSmall` if payload casting checks fail.
    /// - `ProtocolError::InvalidChunkCount` if total_chunks > 1024.
    /// - `ProtocolError::ReassemblyBufferFull` if all slots are occupied and no slot matches.
    /// - `ProtocolError::DuplicateFragment` if the chunk has already been received.
    /// - `ProtocolError::BatchCapacityExceeded` if the estimated spikes exceed the batch limit.
    pub fn insert_chunk(
        &mut self,
        header: &wire::SpikeBatchHeaderV2,
        payload: &[u8],
    ) -> Result<Option<&[wire::SpikeEventV2]>, ProtocolError> {
        // 1. Verify buffer alignment and size for SpikeEventV2 zero-copy cast
        crate::math::verify_cast_guards::<wire::SpikeEventV2>(payload)?;
        let spikes = bytemuck::cast_slice::<u8, wire::SpikeEventV2>(payload);

        // 2. Validate total chunk count
        if header.total_chunks > 1024 {
            return Err(ProtocolError::InvalidChunkCount {
                total_chunks: header.total_chunks,
                max_allowed: 1024,
            });
        }

        // 3. Find matching active slot
        let mut slot_idx = None;
        for i in 0..self.slots.len() {
            if self.slots[i].total_chunks > 0
                && self.slots[i].src_zone_hash == header.src_zone_hash
                && self.slots[i].epoch == header.epoch
            {
                slot_idx = Some(i);
                break;
            }
        }

        let idx = match slot_idx {
            Some(i) => i,
            None => {
                // Find first inactive slot
                let mut empty_idx = None;
                for i in 0..self.slots.len() {
                    if self.slots[i].total_chunks == 0 {
                        empty_idx = Some(i);
                        break;
                    }
                }
                match empty_idx {
                    Some(i) => {
                        self.slots[i].reset(header.src_zone_hash, header.epoch, header.total_chunks);
                        i
                    }
                    None => return Err(ProtocolError::ReassemblyBufferFull),
                }
            }
        };

        let slot = &mut self.slots[idx];

        // 4. Record chunk index in bitmask (checks for duplicates)
        Self::process_chunk(slot, header.chunk_idx)?;

        // 5. Copy spikes to payload_buffer without any dynamic allocations (OOM guard)
        if header.total_chunks == 1 {
            slot.last_chunk_len = spikes.len();
            let start_idx = 0;
            let end_idx = spikes.len();
            if end_idx > slot.payload_buffer.len() {
                return Err(ProtocolError::BatchCapacityExceeded {
                    max_spikes: slot.payload_buffer.len(),
                    actual_spikes: end_idx,
                });
            }
            slot.payload_buffer[start_idx..end_idx].copy_from_slice(spikes);
        } else if header.chunk_idx == header.total_chunks - 1 {
            // Last chunk
            slot.last_chunk_len = spikes.len();
            if slot.max_spikes_per_chunk > 0 {
                let start_idx = (header.total_chunks as usize - 1) * slot.max_spikes_per_chunk;
                let end_idx = start_idx + spikes.len();
                if end_idx > slot.payload_buffer.len() {
                    return Err(ProtocolError::BatchCapacityExceeded {
                        max_spikes: slot.payload_buffer.len(),
                        actual_spikes: end_idx,
                    });
                }
                slot.payload_buffer[start_idx..end_idx].copy_from_slice(spikes);
            } else {
                if spikes.len() > slot.last_chunk_spikes.len() {
                    return Err(ProtocolError::BatchCapacityExceeded {
                        max_spikes: slot.last_chunk_spikes.len(),
                        actual_spikes: spikes.len(),
                    });
                }
                slot.last_chunk_spikes[..spikes.len()].copy_from_slice(spikes);
            }
        } else {
            // Full intermediate chunk
            let chunk_len = spikes.len();
            if slot.max_spikes_per_chunk == 0 {
                slot.max_spikes_per_chunk = chunk_len;
                // If the last chunk arrived out-of-order, place it now
                if slot.last_chunk_len > 0 {
                    let start_idx = (header.total_chunks as usize - 1) * chunk_len;
                    let end_idx = start_idx + slot.last_chunk_len;
                    if end_idx > slot.payload_buffer.len() {
                        return Err(ProtocolError::BatchCapacityExceeded {
                            max_spikes: slot.payload_buffer.len(),
                            actual_spikes: end_idx,
                        });
                    }
                    slot.payload_buffer[start_idx..end_idx]
                        .copy_from_slice(&slot.last_chunk_spikes[..slot.last_chunk_len]);
                }
            }
            let start_idx = header.chunk_idx as usize * slot.max_spikes_per_chunk;
            let end_idx = start_idx + chunk_len;
            if end_idx > slot.payload_buffer.len() {
                return Err(ProtocolError::BatchCapacityExceeded {
                    max_spikes: slot.payload_buffer.len(),
                    actual_spikes: end_idx,
                });
            }
            slot.payload_buffer[start_idx..end_idx].copy_from_slice(spikes);
        }

        // 6. Check if reassembly is complete and return correct slice length
        if slot.received_chunks == slot.total_chunks {
            let total_len = (slot.total_chunks as usize - 1) * slot.max_spikes_per_chunk + slot.last_chunk_len;
            Ok(Some(&slot.payload_buffer[..total_len]))
        } else {
            Ok(None)
        }
    }
}
