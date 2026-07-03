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
