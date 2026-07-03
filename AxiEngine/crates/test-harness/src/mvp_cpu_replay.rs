//! Isolated MVP CPU state and axon blob layout access scaffold.
//!
//! Provides structured accessors to `.state` and `.axons` binary memory blobs using
//! standard `layout` offsets, headers, and column-major matrix indexing (`slot * padded_n + tid`).

use layout::{
    calculate_state_blob_size, compute_state_offsets, AxonsFileHeader, BurstHeads8,
    StateFileHeader, StateOffsets, VariantParameters, AXONS_FILE_VERSION, AXONS_MAGIC,
    MAX_DENDRITES, STATE_FILE_VERSION, STATE_MAGIC, VARIANT_LUT_LEN,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MvpDendriteSlot {
    target: u32,
    weight: i32,
    timer: u8,
}

/// Prunes weak/dead dendrite synapses and sorts active synapses by weight magnitude in descending order.
///
/// For each neuron `tid` in `0..padded_n`:
/// 1. Resets burst count bits `1..=3` in `soma_flags[tid]` (`flag & 0xF1`).
/// 2. Reads 128 dendrite slots at column index `slot * padded_n + tid`.
/// 3. Retains slots where `target != 0` AND `weight.unsigned_abs() >= ((prune_threshold.unsigned_abs() as u32) << 16)`.
/// 4. Sorts active slots to the front in descending order of absolute weight (`abs(weight)`).
/// 5. Writes back target, weight, and timer planes to the `MvpStateBuffer`.
pub fn cpu_sort_and_prune(state_buf: &mut MvpStateBuffer, prune_threshold: i16) {
    let padded_n = state_buf.padded_n();
    let threshold_mass = (prune_threshold.unsigned_abs() as u32) << 16;

    for tid in 0..padded_n {
        let flag = state_buf.read_soma_flags(tid);
        state_buf.write_soma_flags(tid, flag & 0xF1);

        let mut slots = [MvpDendriteSlot {
            target: 0,
            weight: 0,
            timer: 0,
        }; MAX_DENDRITES];

        #[allow(clippy::needless_range_loop)]
        for slot in 0..MAX_DENDRITES {
            let target = state_buf.read_dendrite_target(slot, tid);
            let weight = state_buf.read_dendrite_weight(slot, tid);
            let timer = state_buf.read_dendrite_timer(slot, tid);

            if target != 0 && weight.unsigned_abs() >= threshold_mass {
                slots[slot] = MvpDendriteSlot {
                    target,
                    weight,
                    timer,
                };
            }
        }

        slots.sort_unstable_by(|a, b| {
            let a_alive = a.target != 0;
            let b_alive = b.target != 0;

            if a_alive && !b_alive {
                std::cmp::Ordering::Less
            } else if !a_alive && b_alive {
                std::cmp::Ordering::Greater
            } else if a_alive && b_alive {
                b.weight.unsigned_abs().cmp(&a.weight.unsigned_abs())
            } else {
                std::cmp::Ordering::Equal
            }
        });

        #[allow(clippy::needless_range_loop)]
        for slot in 0..MAX_DENDRITES {
            state_buf.write_dendrite_target(slot, tid, slots[slot].target);
            state_buf.write_dendrite_weight(slot, tid, slots[slot].weight);
            state_buf.write_dendrite_timer(slot, tid, slots[slot].timer);
        }
    }
}

/// Applies Global Synaptic Optimization Protocol (GSOP) plastic weight updates to spiking somas.
///
/// For each spiking neuron `tid` (`soma_flags & 0x01 != 0`):
/// 1. Reads burst count multiplier `(flags >> 1) & 0x07` (minimum 1).
/// 2. Reads profile `var_id = (flags >> 4) & 0x0F` from `variant_table`.
/// 3. Iterates over 128 dendrite slots:
///    - Skips if dendrite timer > 0.
///    - Breaks if `target_packed == 0` (hardware trap).
///    - Skips if weight == 0.
///    - Extracts `seg_idx = target_packed >> 24` and `raw_id = target_packed & 0x00FFFFFF`.
///    - Breaks if `raw_id == 0` (zero-index trap).
///    - Computes `axon_id = raw_id - 1`, skipping if `axon_id >= total_axons`.
///    - Checks active tail hit across all 8 axon heads using `wrapping_sub(seg_idx) <= prop`.
///    - Calculates inertia rank `(abs_w >> 28).min(7)` and applies dopamine modulation (D1/D2).
///    - Adjusts weight magnitude, clamping bottom to 0 and top to `2_140_000_000`, preserving weight sign.
pub fn cpu_apply_gsop(
    state_buf: &mut MvpStateBuffer,
    axon_buf: &MvpAxonBuffer,
    variant_table: &[VariantParameters; VARIANT_LUT_LEN],
    dopamine: i16,
) {
    let padded_n = state_buf.padded_n();
    let total_axons = axon_buf.total_axons();

    for tid in 0..padded_n {
        let flags = state_buf.read_soma_flags(tid);

        // Early Exit: Spiking somas only
        if (flags & 0x01) == 0 {
            continue;
        }

        let burst_count = (flags >> 1) & 0x07;
        let burst_mult = if burst_count > 0 {
            burst_count as i32
        } else {
            1
        };

        let var_id = ((flags >> 4) & 0x0F) as usize;
        let p = &variant_table[var_id];

        for slot in 0..MAX_DENDRITES {
            let timer = state_buf.read_dendrite_timer(slot, tid);
            if timer > 0 {
                continue;
            }

            let target_packed = state_buf.read_dendrite_target(slot, tid);
            if target_packed == 0 {
                break; // Hardware Trap
            }

            let w = state_buf.read_dendrite_weight(slot, tid);
            if w == 0 {
                continue; // Night Phase Pruned
            }

            let seg_idx = target_packed >> 24;
            let raw_id = target_packed & 0x00FFFFFF;
            if raw_id == 0 {
                break; // Zero-Index Trap
            }

            let axon_id = (raw_id - 1) as usize;
            if axon_id >= total_axons {
                continue;
            }

            let h = axon_buf.read_head(axon_id);
            let prop = p.signal_propagation_length as u32;

            // Branchless 8-head Hit Detection
            let is_active = ((h.h0.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h1.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h2.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h3.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h4.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h5.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h6.wrapping_sub(seg_idx) <= prop) as i32)
                | ((h.h7.wrapping_sub(seg_idx) <= prop) as i32);

            let sign = if w >= 0 { 1 } else { -1 };
            let abs_w = w.abs();

            // 1. Inertia Rank (0..7)
            let mut rank = (abs_w >> 28) as usize;
            if rank > 7 {
                rank = 7;
            }
            let inertia = p.inertia_curve[rank] as i32;

            // 2. Dopamine modulation (D1 boosts LTP, D2 suppresses LTD)
            let pot_mod = ((dopamine as i32) * (p.d1_affinity as i32)) >> 7;
            let dep_mod = ((dopamine as i32) * (p.d2_affinity as i32)) >> 7;

            let raw_pot = (p.gsop_potentiation as i32) + pot_mod;
            let raw_dep = (p.gsop_depression as i32) - dep_mod;

            let final_pot = raw_pot & !(raw_pot >> 31); // max(0, val)
            let final_dep = raw_dep & !(raw_dep >> 31); // max(0, val)

            let delta_pot = (final_pot * inertia * burst_mult) >> 7;
            let delta_dep = (final_dep * inertia * burst_mult) >> 7;

            let mut delta = if is_active != 0 {
                delta_pot
            } else {
                -delta_dep
            };

            // Fixed Slot Decay = 1.0x
            delta = (delta * 128) >> 7;

            // 3. Apply & Clamp to Mass Domain Limits
            let mut new_abs = abs_w + delta;
            new_abs &= !(new_abs >> 31); // Clamp bottom to 0
            if new_abs > 2140000000 {
                new_abs = 2140000000;
            } // Headroom guard

            state_buf.write_dendrite_weight(slot, tid, new_abs * sign);
        }
    }
}
