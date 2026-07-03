//! Isolated MVP CPU state and axon blob layout access scaffold.
//!
//! Provides structured accessors to `.state` and `.axons` binary memory blobs using
//! standard `layout` offsets, headers, and column-major matrix indexing (`slot * padded_n + tid`).

use layout::{
    calculate_state_blob_size, compute_state_offsets, AxonsFileHeader, BurstHeads8,
    StateFileHeader, StateOffsets, AXONS_FILE_VERSION, AXONS_MAGIC, MAX_DENDRITES,
    STATE_FILE_VERSION, STATE_MAGIC,
};
use types::AXON_SENTINEL;

/// Safe access wrapper over raw byte buffer representing `.state` SoA planes for MVP CPU replay.
#[derive(Debug, Clone)]
pub struct MvpStateBuffer {
    padded_n: usize,
    total_axons: usize,
    offsets: StateOffsets,
    data: Vec<u8>,
}

impl MvpStateBuffer {
    /// Creates a new `MvpStateBuffer` for `padded_n` neurons and `total_axons` capacity,
    /// writing `StateFileHeader` to the first 16 bytes.
    pub fn new(padded_n: usize, total_axons: usize) -> Self {
        let offsets = compute_state_offsets(padded_n);
        let blob_size = calculate_state_blob_size(padded_n);
        let mut data = vec![0u8; blob_size];

        let header = StateFileHeader::new(padded_n as u32, total_axons as u32);
        data[0..4].copy_from_slice(&header.magic);
        data[4..8].copy_from_slice(&header.version.to_le_bytes());
        data[8..12].copy_from_slice(&header.padded_n.to_le_bytes());
        data[12..16].copy_from_slice(&header.total_axons.to_le_bytes());

        Self {
            padded_n,
            total_axons,
            offsets,
            data,
        }
    }

    /// Creates an `MvpStateBuffer` wrapping an existing raw binary `.state` byte buffer.
    ///
    /// # Panics
    /// Panics if buffer length, magic (`AXST`), version, `padded_n`, or `total_axons` mismatch expectations.
    pub fn from_raw(padded_n: usize, total_axons: usize, data: Vec<u8>) -> Self {
        let offsets = compute_state_offsets(padded_n);
        let required_size = calculate_state_blob_size(padded_n);
        assert!(
            data.len() >= required_size,
            "State buffer size {} is smaller than required {}",
            data.len(),
            required_size
        );

        let magic: [u8; 4] = data[0..4].try_into().unwrap();
        assert_eq!(
            magic, STATE_MAGIC,
            "State blob magic mismatch: expected {:?}, got {:?}",
            STATE_MAGIC, magic
        );

        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        assert_eq!(
            version, STATE_FILE_VERSION,
            "State blob version mismatch: expected {}, got {}",
            STATE_FILE_VERSION, version
        );

        let raw_padded_n = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        assert_eq!(
            raw_padded_n, padded_n,
            "State blob padded_n mismatch: expected {}, got {}",
            padded_n, raw_padded_n
        );

        let raw_total_axons = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        assert_eq!(
            raw_total_axons, total_axons,
            "State blob total_axons mismatch: expected {}, got {}",
            total_axons, raw_total_axons
        );

        Self {
            padded_n,
            total_axons,
            offsets,
            data,
        }
    }

    /// Returns padded neuron count `padded_n`.
    #[inline]
    pub fn padded_n(&self) -> usize {
        self.padded_n
    }

    /// Returns total axon count `total_axons`.
    #[inline]
    pub fn total_axons(&self) -> usize {
        self.total_axons
    }

    /// Reads and parses the 16-byte `StateFileHeader` from the blob start.
    #[inline]
    pub fn header(&self) -> StateFileHeader {
        let magic: [u8; 4] = self.data[0..4].try_into().unwrap();
        let version = u32::from_le_bytes(self.data[4..8].try_into().unwrap());
        let padded_n = u32::from_le_bytes(self.data[8..12].try_into().unwrap());
        let total_axons = u32::from_le_bytes(self.data[12..16].try_into().unwrap());
        StateFileHeader {
            magic,
            version,
            padded_n,
            total_axons,
        }
    }

    /// Returns reference to computed layout `StateOffsets`.
    #[inline]
    pub fn offsets(&self) -> &StateOffsets {
        &self.offsets
    }

    /// Returns underlying raw byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns underlying mutable raw byte slice.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Computes column-major matrix index for dendrite slot and neuron ID (`slot * padded_n + tid`).
    ///
    /// # Panics
    /// Panics if `slot >= MAX_DENDRITES` (128) or `tid >= padded_n`.
    #[inline]
    pub fn dendrite_index(&self, slot: usize, tid: usize) -> usize {
        assert!(
            slot < MAX_DENDRITES,
            "Dendrite slot {} exceeds max {}",
            slot,
            MAX_DENDRITES
        );
        assert!(
            tid < self.padded_n,
            "Neuron ID {} exceeds padded_n {}",
            tid,
            self.padded_n
        );
        slot * self.padded_n + tid
    }

    /// Reads `soma_voltage` for neuron `tid`.
    #[inline]
    pub fn read_soma_voltage(&self, tid: usize) -> i32 {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_voltage + tid * 4;
        i32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Writes `soma_voltage` for neuron `tid`.
    #[inline]
    pub fn write_soma_voltage(&mut self, tid: usize, val: i32) {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_voltage + tid * 4;
        self.data[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    /// Reads `soma_flags` for neuron `tid`.
    #[inline]
    pub fn read_soma_flags(&self, tid: usize) -> u8 {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_flags + tid;
        self.data[off]
    }

    /// Writes `soma_flags` for neuron `tid`.
    #[inline]
    pub fn write_soma_flags(&mut self, tid: usize, val: u8) {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_flags + tid;
        self.data[off] = val;
    }

    /// Reads `threshold_offset` for neuron `tid`.
    #[inline]
    pub fn read_threshold_offset(&self, tid: usize) -> i32 {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_thresh + tid * 4;
        i32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Writes `threshold_offset` for neuron `tid`.
    #[inline]
    pub fn write_threshold_offset(&mut self, tid: usize, val: i32) {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_thresh + tid * 4;
        self.data[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    /// Reads `timers` for neuron `tid`.
    #[inline]
    pub fn read_timer(&self, tid: usize) -> u8 {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_timers + tid;
        self.data[off]
    }

    /// Writes `timers` for neuron `tid`.
    #[inline]
    pub fn write_timer(&mut self, tid: usize, val: u8) {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_timers + tid;
        self.data[off] = val;
    }

    /// Reads `soma_to_axon` mapping for neuron `tid`.
    #[inline]
    pub fn read_soma_to_axon(&self, tid: usize) -> u32 {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_s2a + tid * 4;
        u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Writes `soma_to_axon` mapping for neuron `tid`.
    #[inline]
    pub fn write_soma_to_axon(&mut self, tid: usize, val: u32) {
        assert!(tid < self.padded_n);
        let off = self.offsets.off_s2a + tid * 4;
        self.data[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    /// Reads `dendrite_targets` packed target value at `(slot, tid)`.
    #[inline]
    pub fn read_dendrite_target(&self, slot: usize, tid: usize) -> u32 {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_targets + col_idx * 4;
        u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Writes `dendrite_targets` packed target value at `(slot, tid)`.
    #[inline]
    pub fn write_dendrite_target(&mut self, slot: usize, tid: usize, val: u32) {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_targets + col_idx * 4;
        self.data[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    /// Reads `dendrite_weights` weight value at `(slot, tid)`.
    #[inline]
    pub fn read_dendrite_weight(&self, slot: usize, tid: usize) -> i32 {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_weights + col_idx * 4;
        i32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Writes `dendrite_weights` weight value at `(slot, tid)`.
    #[inline]
    pub fn write_dendrite_weight(&mut self, slot: usize, tid: usize, val: i32) {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_weights + col_idx * 4;
        self.data[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }

    /// Reads `dendrite_timers` timer value at `(slot, tid)`.
    #[inline]
    pub fn read_dendrite_timer(&self, slot: usize, tid: usize) -> u8 {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_dtimers + col_idx;
        self.data[off]
    }

    /// Writes `dendrite_timers` timer value at `(slot, tid)`.
    #[inline]
    pub fn write_dendrite_timer(&mut self, slot: usize, tid: usize, val: u8) {
        let col_idx = self.dendrite_index(slot, tid);
        let off = self.offsets.off_dtimers + col_idx;
        self.data[off] = val;
    }
}

/// Binary blob-compatible wrapper for `.axons` files (16-byte header + `total_axons * 32B` payload).
#[derive(Debug, Clone)]
pub struct MvpAxonBuffer {
    total_axons: usize,
    data: Vec<u8>,
}

impl MvpAxonBuffer {
    /// Creates a new `MvpAxonBuffer` for `total_axons`, initialized with `AxonsFileHeader` (16 bytes)
    /// followed by `BurstHeads8` ring buffers populated with `AXON_SENTINEL`.
    pub fn new(total_axons: usize) -> Self {
        let header = AxonsFileHeader::new(total_axons as u32);
        let blob_size = 16 + total_axons * 32;
        let mut data = vec![0u8; blob_size];

        data[0..4].copy_from_slice(&header.magic);
        data[4..8].copy_from_slice(&header.version.to_le_bytes());
        data[8..12].copy_from_slice(&header.total_axons.to_le_bytes());
        data[12..16].copy_from_slice(&header._padding.to_le_bytes());

        let empty_head = BurstHeads8::empty(AXON_SENTINEL);
        for i in 0..total_axons {
            let off = 16 + i * 32;
            data[off..off + 4].copy_from_slice(&empty_head.h0.to_le_bytes());
            data[off + 4..off + 8].copy_from_slice(&empty_head.h1.to_le_bytes());
            data[off + 8..off + 12].copy_from_slice(&empty_head.h2.to_le_bytes());
            data[off + 12..off + 16].copy_from_slice(&empty_head.h3.to_le_bytes());
            data[off + 16..off + 20].copy_from_slice(&empty_head.h4.to_le_bytes());
            data[off + 20..off + 24].copy_from_slice(&empty_head.h5.to_le_bytes());
            data[off + 24..off + 28].copy_from_slice(&empty_head.h6.to_le_bytes());
            data[off + 28..off + 32].copy_from_slice(&empty_head.h7.to_le_bytes());
        }

        Self { total_axons, data }
    }

    /// Creates an `MvpAxonBuffer` wrapping an existing binary `.axons` blob.
    ///
    /// # Panics
    /// Panics if buffer length, magic (`AXAX`), version, or `total_axons` mismatch expectations.
    pub fn from_raw(total_axons: usize, data: Vec<u8>) -> Self {
        let required_size = 16 + total_axons * 32;
        assert!(
            data.len() >= required_size,
            "Axon blob size {} is smaller than required {}",
            data.len(),
            required_size
        );

        let magic: [u8; 4] = data[0..4].try_into().unwrap();
        assert_eq!(
            magic, AXONS_MAGIC,
            "Axon blob magic mismatch: expected {:?}, got {:?}",
            AXONS_MAGIC, magic
        );

        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        assert_eq!(
            version, AXONS_FILE_VERSION,
            "Axon blob version mismatch: expected {}, got {}",
            AXONS_FILE_VERSION, version
        );

        let raw_total_axons = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        assert_eq!(
            raw_total_axons, total_axons,
            "Axon blob total_axons mismatch: expected {}, got {}",
            total_axons, raw_total_axons
        );

        Self { total_axons, data }
    }

    /// Returns total axon count.
    #[inline]
    pub fn total_axons(&self) -> usize {
        self.total_axons
    }

    /// Reads and parses the 16-byte `AxonsFileHeader` from the blob start.
    #[inline]
    pub fn header(&self) -> AxonsFileHeader {
        let magic: [u8; 4] = self.data[0..4].try_into().unwrap();
        let version = u32::from_le_bytes(self.data[4..8].try_into().unwrap());
        let total_axons = u32::from_le_bytes(self.data[8..12].try_into().unwrap());
        let _padding = u32::from_le_bytes(self.data[12..16].try_into().unwrap());
        AxonsFileHeader {
            magic,
            version,
            total_axons,
            _padding,
        }
    }

    /// Returns underlying raw byte slice of the entire `.axons` blob including header.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns underlying mutable raw byte slice.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns raw byte slice of axon heads payload (excluding the 16-byte header).
    #[inline]
    pub fn payload_bytes(&self) -> &[u8] {
        &self.data[16..]
    }

    /// Reads `BurstHeads8` ring buffer for `axon_id`.
    #[inline]
    pub fn read_head(&self, axon_id: usize) -> BurstHeads8 {
        assert!(
            axon_id < self.total_axons,
            "Axon ID {} exceeds total {}",
            axon_id,
            self.total_axons
        );
        let off = 16 + axon_id * 32;
        BurstHeads8 {
            h0: u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap()),
            h1: u32::from_le_bytes(self.data[off + 4..off + 8].try_into().unwrap()),
            h2: u32::from_le_bytes(self.data[off + 8..off + 12].try_into().unwrap()),
            h3: u32::from_le_bytes(self.data[off + 12..off + 16].try_into().unwrap()),
            h4: u32::from_le_bytes(self.data[off + 16..off + 20].try_into().unwrap()),
            h5: u32::from_le_bytes(self.data[off + 20..off + 24].try_into().unwrap()),
            h6: u32::from_le_bytes(self.data[off + 24..off + 28].try_into().unwrap()),
            h7: u32::from_le_bytes(self.data[off + 28..off + 32].try_into().unwrap()),
        }
    }

    /// Writes `BurstHeads8` ring buffer for `axon_id`.
    #[inline]
    pub fn write_head(&mut self, axon_id: usize, head: BurstHeads8) {
        assert!(
            axon_id < self.total_axons,
            "Axon ID {} exceeds total {}",
            axon_id,
            self.total_axons
        );
        let off = 16 + axon_id * 32;
        self.data[off..off + 4].copy_from_slice(&head.h0.to_le_bytes());
        self.data[off + 4..off + 8].copy_from_slice(&head.h1.to_le_bytes());
        self.data[off + 8..off + 12].copy_from_slice(&head.h2.to_le_bytes());
        self.data[off + 12..off + 16].copy_from_slice(&head.h3.to_le_bytes());
        self.data[off + 16..off + 20].copy_from_slice(&head.h4.to_le_bytes());
        self.data[off + 20..off + 24].copy_from_slice(&head.h5.to_le_bytes());
        self.data[off + 24..off + 28].copy_from_slice(&head.h6.to_le_bytes());
        self.data[off + 28..off + 32].copy_from_slice(&head.h7.to_le_bytes());
    }
}

// =============================================================================
// Standalone MVP CPU Functions
// =============================================================================

/// Advances axon propagation heads by `v_seg` segments for active (non-sentinel) heads.
///
/// Implements 1:1 legacy MVP parity by processing heads in pairs using `chunks_exact_mut(2)`.
/// Any trailing odd element in an odd-length `axon_heads` slice is skipped.
/// Valid production axon head buffers must have an even length.
pub fn cpu_propagate_axons(axon_heads: &mut [BurstHeads8], v_seg: u32) {
    for chunk in axon_heads.chunks_exact_mut(2) {
        for head in chunk {
            head.h0 = head
                .h0
                .wrapping_add(v_seg * ((head.h0 != AXON_SENTINEL) as u32));
            head.h1 = head
                .h1
                .wrapping_add(v_seg * ((head.h1 != AXON_SENTINEL) as u32));
            head.h2 = head
                .h2
                .wrapping_add(v_seg * ((head.h2 != AXON_SENTINEL) as u32));
            head.h3 = head
                .h3
                .wrapping_add(v_seg * ((head.h3 != AXON_SENTINEL) as u32));
            head.h4 = head
                .h4
                .wrapping_add(v_seg * ((head.h4 != AXON_SENTINEL) as u32));
            head.h5 = head
                .h5
                .wrapping_add(v_seg * ((head.h5 != AXON_SENTINEL) as u32));
            head.h6 = head
                .h6
                .wrapping_add(v_seg * ((head.h6 != AXON_SENTINEL) as u32));
            head.h7 = head
                .h7
                .wrapping_add(v_seg * ((head.h7 != AXON_SENTINEL) as u32));
        }
    }
}

/// Applies scheduled spike batch events to axon propagation ring buffers.
///
/// For each `ghost_id` in `schedule_indices`, shifts `h7 <- h6 ... h1 <- h0`
/// and sets `h0 = 0u32.wrapping_sub(v_seg)`. Out-of-range axon IDs are ignored.
pub fn cpu_apply_spike_batch(axon_heads: &mut [BurstHeads8], schedule_indices: &[u32], v_seg: u32) {
    for &ghost_id in schedule_indices {
        if let Some(head) = axon_heads.get_mut(ghost_id as usize) {
            head.h7 = head.h6;
            head.h6 = head.h5;
            head.h5 = head.h4;
            head.h4 = head.h3;
            head.h3 = head.h2;
            head.h2 = head.h1;
            head.h1 = head.h0;
            head.h0 = 0u32.wrapping_sub(v_seg);
        }
    }
}

/// Injects external stimulus input spikes into virtual axons based on a bitmask slice.
///
/// For each virtual axon `tid` in `0..num_virtual_axons`, checks bit `tid % 32` in `input_bitmask[tid / 32]`.
/// If set, performs a ring buffer shift on axon `virtual_offset + tid` and sets `h0 = 0u32.wrapping_sub(v_seg)`.
/// Out-of-range virtual axon indices are safely ignored.
///
/// If `input_bitmask` is shorter than `(num_virtual_axons + 31) / 32`, missing bitmask words are safely skipped
/// without panicking, leaving remaining virtual axons unchanged.
pub fn cpu_inject_inputs(
    axon_heads: &mut [BurstHeads8],
    input_bitmask: &[u32],
    virtual_offset: u32,
    num_virtual_axons: u32,
    v_seg: u32,
) {
    for tid in 0..num_virtual_axons as usize {
        let word_idx = tid / 32;
        let bit_idx = tid % 32;
        if let Some(&word) = input_bitmask.get(word_idx) {
            if (word >> bit_idx) & 1 != 0 {
                if let Some(head) = axon_heads.get_mut(virtual_offset as usize + tid) {
                    head.h7 = head.h6;
                    head.h6 = head.h5;
                    head.h5 = head.h4;
                    head.h4 = head.h3;
                    head.h3 = head.h2;
                    head.h2 = head.h1;
                    head.h1 = head.h0;
                    head.h0 = 0u32.wrapping_sub(v_seg);
                }
            }
        }
    }
}

/// Records dense output history state (`0` or `1`) for mapped somas at `current_tick`.
///
/// For each element in `mapped_soma_ids`, if `soma_id != 0xFFFF_FFFF` and `soma_id` exists in `soma_flags`,
/// writes `soma_flags[soma_id] & 0x01` to `output_history[current_tick * total_mapped_somas + i]`.
/// Overwrites existing target buffer values (writing both `0` and `1`).
pub fn cpu_record_outputs(
    soma_flags: &[u8],
    mapped_soma_ids: &[u32],
    output_history: &mut [u8],
    current_tick: u32,
    total_mapped_somas: u32,
) {
    let tick_offset = (current_tick as usize) * (total_mapped_somas as usize);
    for (i, &soma_id) in mapped_soma_ids.iter().enumerate() {
        if soma_id != 0xFFFF_FFFF {
            if let Some(&flag) = soma_flags.get(soma_id as usize) {
                if let Some(out) = output_history.get_mut(tick_offset + i) {
                    *out = flag & 0x01;
                }
            }
        }
    }
}

/// Extracts IDs of spiking somas (`soma_flags & 0x01 != 0`) in sequential ascending order.
///
/// Writes up to `out_ids.len()` spiking soma IDs into `out_ids`. Returns the total count of IDs recorded.
/// Does not panic if `out_ids` capacity is smaller than total spiking somas.
pub fn cpu_extract_telemetry(soma_flags: &[u8], out_ids: &mut [u32]) -> u32 {
    let mut count = 0;
    for (id, &flag) in soma_flags.iter().enumerate() {
        if (flag & 0x01) != 0 {
            if let Some(slot) = out_ids.get_mut(count) {
                *slot = id as u32;
                count += 1;
            }
        }
    }
    count as u32
}
