#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

pub mod error;
pub mod offsets;

pub use error::*;
pub use offsets::*;

// =============================================================================
// Constants and Magic Numbers
// =============================================================================

/// Hardware hard limit for the number of dendritic slots per neuron.
pub const MAX_DENDRITES: usize = 128;

/// Magic number for path tracing geometry file `.paths` ("PATH").
pub const PATHS_MAGIC: u32 = 0x50415448;

/// Maximum length of an axon segment path.
pub const MAX_SEGMENTS_PER_AXON: usize = 256;

/// Magic number for Night Phase shared memory protocol ("AXIC").
pub const SHM_MAGIC: u32 = 0x41584943;

/// Current supported version of Night Phase IPC protocol.
pub const SHM_VERSION: u8 = 4;

/// Magic number for electrophysiology shared memory ("EPHY").
pub const EPHYS_MAGIC: u32 = 0x45504859;

/// Magic number for snapshot checkpoint headers ("SNAP").
pub const SNAP_MAGIC: u32 = 0x50414E53;

/// Magic number for state dump file `.state` ("GSNS").
pub const STATE_MAGIC: u32 = 0x534E5347;

/// Magic number for axons dump file `.axons` ("GSAX").
pub const AXONS_MAGIC: u32 = 0x58415347;

/// Max capacity of the axon handover queue per night.
pub const MAX_HANDOVERS_PER_NIGHT: usize = 10000;

/// Max capacity of the synapse pruning queue per night.
pub const MAX_PRUNES_PER_NIGHT: usize = 10000;

/// Byte size of the C-ABI AxonHandoverEvent.
pub const AXON_HANDOVER_EVENT_SIZE: usize = 20;

/// Byte size of the C-ABI AxonHandoverPrune.
pub const AXON_HANDOVER_PRUNE_SIZE: usize = 12;

/// Maximum number of simultaneously recorded or stimulated neurons.
pub const MAX_EPHYS_TARGETS: usize = 16;

/// Trace buffer length for electrophysiology membrane voltage records.
pub const MAX_EPHYS_TICKS: usize = 10000;

// =============================================================================
// Structures and Memory Layout
// =============================================================================

/// Behavior variant parameters profile for GLIF and GSOP dynamics.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VariantParameters {
    pub threshold: i32,
    pub rest_potential: i32,
    pub leak_shift: u32,
    pub homeostasis_penalty: i32,
    pub spontaneous_firing_period_ticks: u32,
    pub initial_synapse_weight: u16,
    pub gsop_potentiation: u16,
    pub gsop_depression: u16,
    pub homeostasis_decay: u16,
    pub refractory_period: u8,
    pub synapse_refractory_period: u8,
    pub signal_propagation_length: u8,
    pub is_inhibitory: u8,
    pub inertia_curve: [u8; 8],
    pub ahp_amplitude: u16,
    pub _pad: [u8; 6],
    pub adaptive_leak_min_shift: i32,
    pub adaptive_leak_gain: u16,
    pub adaptive_mode: u8,
    pub _leak_pad: [u8; 3],
    pub d1_affinity: u8,
    pub d2_affinity: u8,
    pub heartbeat_m: u32,
}

/// Shift register of active signal wavefront heads inside an individual axon.
#[repr(C, align(32))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BurstHeads8 {
    pub h0: u32,
    pub h1: u32,
    pub h2: u32,
    pub h3: u32,
    pub h4: u32,
    pub h5: u32,
    pub h6: u32,
    pub h7: u32,
}

impl BurstHeads8 {
    #[inline]
    pub const fn empty(sentinel: u32) -> Self {
        Self {
            h0: sentinel,
            h1: sentinel,
            h2: sentinel,
            h3: sentinel,
            h4: sentinel,
            h5: sentinel,
            h6: sentinel,
            h7: sentinel,
        }
    }
}

/// Host-side SoA state of a shard.
/// Used for baking and disk I/O.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardStateSoA {
    pub padded_n: usize, // Must be multiple of 32 (Warp Alignment)

    // --- Soma Hot State ---
    pub voltage: Vec<i32>,
    pub flags: Vec<u8>,
    pub threshold_offset: Vec<i32>,
    pub refractory_timer: Vec<u8>,

    // --- Columnar Dendrites (Size = MAX_DENDRITES * padded_n) ---
    pub dendrite_targets: Vec<u32>, // Dense ID + Segment Offset
    pub dendrite_weights: Vec<i32>,
    pub dendrite_timers: Vec<u8>,

    // --- Axon Heads (Size = total_axons) ---
    pub axon_heads: Vec<BurstHeads8>,
}

impl ShardStateSoA {
    /// Initializes a new shard.
    /// - `padded_n`: number of neurons (multiple of 32).
    /// - `total_axons`: total axons (local + ghost + virtual).
    pub fn new(padded_n: usize, total_axons: usize) -> Self {
        assert!(
            padded_n % 32 == 0,
            "padded_n must be warp-aligned (multiple of 32)"
        );

        Self {
            padded_n,
            voltage: vec![0; padded_n],
            flags: vec![0; padded_n],
            threshold_offset: vec![0; padded_n],
            refractory_timer: vec![0; padded_n],

            dendrite_targets: vec![0; MAX_DENDRITES * padded_n],
            dendrite_weights: vec![0; MAX_DENDRITES * padded_n],
            dendrite_timers: vec![0; MAX_DENDRITES * padded_n],

            axon_heads: vec![BurstHeads8::empty(types::AXON_SENTINEL); total_axons],
        }
    }

    /// Calculates flat index for Coalesced Access on GPU
    #[inline(always)]
    pub fn columnar_idx(padded_n: usize, neuron_idx: usize, slot: usize) -> usize {
        assert!(neuron_idx < padded_n && slot < MAX_DENDRITES,
            "columnar_idx: neuron_idx={neuron_idx} >= padded_n={padded_n} or slot={slot} >= {MAX_DENDRITES}");
        slot * padded_n + neuron_idx
    }
}

/// Packs Axon_ID and segment offset.
/// Applies +1 to Axon_ID so target == 0 always means "empty slot".
#[inline(always)]
pub const fn pack_dendrite_target(axon_id: u32, segment_offset: u32) -> u32 {
    if axon_id >= 0x00FF_FFFF {
        panic!("CRITICAL: Axon ID exceeds 24 bits");
    }
    if segment_offset >= 256 {
        panic!("CRITICAL: Segment offset exceeds 8 bits");
    }

    (segment_offset << 24) | ((axon_id + 1) & 0x00FF_FFFF)
}

/// Extracts Axon_ID (accounting for -1 reverse shift).
#[inline(always)]
pub const fn unpack_axon_id(target_packed: u32) -> u32 {
    (target_packed & 0x00FF_FFFF).saturating_sub(1)
}

/// Extracts segment offset [0..255].
#[inline(always)]
pub const fn unpack_segment_offset(target_packed: u32) -> u32 {
    target_packed >> 24
}

/// CUDA-compatible structure for SoA FFI (Legacy compatibility).
/// Contains raw pointers to Host/Device memory.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VramState {
    pub padded_n: u32,
    pub total_axons: u32,

    // Soma Hot State
    pub voltage: *mut i32,
    pub flags: *mut u8,
    pub threshold_offset: *mut i32,
    pub refractory_timer: *mut u8,
    pub soma_to_axon: *mut u32,

    // Dendrites (Size: 128 * padded_n)
    pub dendrite_targets: *mut u32,
    pub dendrite_weights: *mut i32,
    pub dendrite_timers: *mut u8,

    // Axons (Size: total_axons)
    pub axon_heads: *mut BurstHeads8,

    // I/O & Telemetry
    pub input_bitmask: *mut u32,
    pub output_history: *mut u8,
    pub telemetry_count: *mut u32,
    pub telemetry_spikes: *mut u32,
}

impl VramState {
    /// Initializes a `VramState` FFI struct from `ShardStateSoA`.
    /// 
    /// # Safety
    /// Caller must guarantee `soa` is not moved, resized, or dropped
    /// while `VramState` is in use.
    #[inline(always)]
    pub unsafe fn from_soa(soa: &mut ShardStateSoA) -> Self {
        Self {
            padded_n: soa.padded_n as u32,
            total_axons: soa.axon_heads.len() as u32,

            voltage: soa.voltage.as_mut_ptr(),
            flags: soa.flags.as_mut_ptr(),
            threshold_offset: soa.threshold_offset.as_mut_ptr(),
            refractory_timer: soa.refractory_timer.as_mut_ptr(),
            soma_to_axon: core::ptr::null_mut(),

            dendrite_targets: soa.dendrite_targets.as_mut_ptr(),
            dendrite_weights: soa.dendrite_weights.as_mut_ptr(),
            dendrite_timers: soa.dendrite_timers.as_mut_ptr(),

            axon_heads: soa.axon_heads.as_mut_ptr(),

            input_bitmask: core::ptr::null_mut(),
            output_history: core::ptr::null_mut(),
            telemetry_count: core::ptr::null_mut(),
            telemetry_spikes: core::ptr::null_mut(),
        }
    }
}

/// State machine flags for Night Phase IPC.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShmState {
    Idle = 0,
    NightStart = 1,
    Sprouting = 2,
    NightDone = 3,
    Error = 4,
}

/// Header for the shared memory region during Night Phase IPC.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShmHeader {
    pub magic: u32,
    pub version: u8,
    pub state: u8,
    pub _pad: u16,
    pub padded_n: u32,
    pub dendrite_slots: u32,
    pub weights_offset: u32,
    pub targets_offset: u32,
    pub epoch: u64,
    pub total_axons: u32,
    pub handovers_offset: u32,
    pub handovers_count: u32,
    pub zone_hash: u32,
    pub prunes_offset: u32,
    pub prunes_count: u32,
    pub incoming_prunes_count: u32,
    pub flags_offset: u32,
    pub voltage_offset: u32,
    pub threshold_offset_offset: u32,
    pub timers_offset: u32,
    pub _reserved: [u32; 13],
}

/// Header structure for `.state` binary dumps.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StateFileHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub padded_n: u32,
    pub total_axons: u32,
}

/// Header structure for `.axons` binary dumps.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AxonsFileHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub total_axons: u32,
    pub _padding: u32,
}

/// Header structure for `.paths` binary dumps.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PathsFileHeader {
    pub magic: u32,
    pub version: u32,
    pub total_axons: u32,
    pub max_segments: u32,
}

/// Header structure for snapshot checkpoints (Self-Healing).
#[repr(C, align(32))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShardStateHeader {
    pub magic: u32,
    pub zone_hash: u32,
    pub tick: u32,
    pub _padding1: u32,
    pub payload_size: u64,
    pub _padding2: [u64; 1],
}

/// Memory layout of electrophysiology shared memory buffer.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EphysShm {
    pub magic: u32,
    pub state: u32,
    pub count: u32,
    pub max_ticks: u32,
    pub current_tick: u32,
    pub _pad: [u32; 11],
    pub target_tids: [u32; 16],
    pub injection_uv: [i32; 16],
    pub out_trace: [i32; 160000],
}

unsafe impl bytemuck::Zeroable for EphysShm {}
unsafe impl bytemuck::Pod for EphysShm {}

/// GPU FFI memory pointer table for direct execution.
///
/// This structure provides a unified C-ABI representation of raw pointers pointing
/// to GPU VRAM regions (voltage, flags, thresholds, timers, synapses).
///
/// # Safety
/// This is a legal exception to the "no raw pointers" guideline, designed explicitly
/// to service invariant `INV-CROSS-007` (FFI boundary synchronization) where Rust-allocated
/// layout parameters and state offsets are mapped to GPU buffers.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShardVramPtrs {
    pub soma_voltage: *mut i32,
    pub flags: *mut u8,
    pub threshold_offset: *mut i32,
    pub timers: *mut u8,
    pub soma_to_axon: *mut u32,
    pub dendrite_targets: *mut u32,
    pub dendrite_weights: *mut i32,
    pub dendrite_timers: *mut u8,
    pub axon_heads: *mut BurstHeads8,
    pub variant_params: *const VariantParameters,
}

// =============================================================================
// Compile-time Invariant Assertions (INV-LAYOUT-*)
// =============================================================================
const _: () = {
    // INV-LAYOUT-001: size_of::<VariantParameters>() == 64 and align_of::<VariantParameters>() == 64
    assert!(core::mem::size_of::<VariantParameters>() == 64);
    assert!(core::mem::align_of::<VariantParameters>() == 64);

    // INV-LAYOUT-002: size_of::<BurstHeads8>() == 32 and align_of::<BurstHeads8>() == 32
    assert!(core::mem::size_of::<BurstHeads8>() == 32);
    assert!(core::mem::align_of::<BurstHeads8>() == 32);

    // INV-LAYOUT-003: size_of::<StateFileHeader>() == 16
    assert!(core::mem::size_of::<StateFileHeader>() == 16);

    // INV-LAYOUT-004: size_of::<AxonsFileHeader>() == 16
    assert!(core::mem::size_of::<AxonsFileHeader>() == 16);

    // INV-LAYOUT-005: size_of::<PathsFileHeader>() == 16
    assert!(core::mem::size_of::<PathsFileHeader>() == 16);

    // INV-LAYOUT-006: size_of::<ShmHeader>() == 128
    assert!(core::mem::size_of::<ShmHeader>() == 128);
    assert!(core::mem::align_of::<ShmHeader>() == 64);

    // INV-LAYOUT-010: size_of::<ShardStateHeader>() == 32 and align_of::<ShardStateHeader>() == 32
    assert!(core::mem::size_of::<ShardStateHeader>() == 32);
    assert!(core::mem::align_of::<ShardStateHeader>() == 32);

    // INV-LAYOUT-011: size_of::<EphysShm>() == 640192 and align_of::<EphysShm>() == 64
    assert!(core::mem::size_of::<EphysShm>() == 640192);
    assert!(core::mem::align_of::<EphysShm>() == 64);

    // INV-CROSS-007: size_of::<ShardVramPtrs>() matches 10 pointers
    assert!(core::mem::size_of::<ShardVramPtrs>() == 10 * core::mem::size_of::<*mut u8>());
    assert!(core::mem::align_of::<ShardVramPtrs>() == core::mem::align_of::<*mut u8>());
};

// =============================================================================
// Unit Tests Block
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_variant_parameters_layout() {
        assert_eq!(size_of::<VariantParameters>(), 64);
        assert_eq!(align_of::<VariantParameters>(), 64);
    }

    #[test]
    fn test_burst_heads_layout() {
        assert_eq!(size_of::<BurstHeads8>(), 32);
        assert_eq!(align_of::<BurstHeads8>(), 32);
    }

    #[test]
    fn test_state_file_header_layout() {
        assert_eq!(size_of::<StateFileHeader>(), 16);
        let header = StateFileHeader {
            magic: *b"GSNS",
            version: 1,
            padded_n: 0,
            total_axons: 0,
        };
        assert_eq!(header.magic, *b"GSNS");
    }

    #[test]
    fn test_axons_file_header_layout() {
        assert_eq!(size_of::<AxonsFileHeader>(), 16);
        let header = AxonsFileHeader {
            magic: *b"GSAX",
            version: 1,
            total_axons: 0,
            _padding: 0,
        };
        assert_eq!(header.magic, *b"GSAX");
    }

    #[test]
    fn test_paths_file_header_layout() {
        assert_eq!(size_of::<PathsFileHeader>(), 16);
        let header = PathsFileHeader {
            magic: PATHS_MAGIC,
            version: 1,
            total_axons: 0,
            max_segments: 256,
        };
        assert_eq!(header.magic, 0x50415448);
    }

    #[test]
    fn test_shm_header_layout() {
        assert_eq!(size_of::<ShmHeader>(), 128);
        assert_eq!(align_of::<ShmHeader>(), 64);
        let header = ShmHeader {
            magic: SHM_MAGIC,
            version: SHM_VERSION,
            state: ShmState::Idle as u8,
            _pad: 0,
            padded_n: 0,
            dendrite_slots: 128,
            weights_offset: 0,
            targets_offset: 0,
            epoch: 0,
            total_axons: 0,
            handovers_offset: 0,
            handovers_count: 0,
            zone_hash: 0,
            prunes_offset: 0,
            prunes_count: 0,
            incoming_prunes_count: 0,
            flags_offset: 0,
            voltage_offset: 0,
            threshold_offset_offset: 0,
            timers_offset: 0,
            _reserved: [0; 13],
        };
        assert_eq!(header.magic, 0x41584943);
        assert_eq!(header.version, 4);
    }

    #[test]
    fn test_shard_state_header_layout() {
        assert_eq!(size_of::<ShardStateHeader>(), 32);
        assert_eq!(align_of::<ShardStateHeader>(), 32);
        let header = ShardStateHeader {
            magic: SNAP_MAGIC,
            zone_hash: 0,
            tick: 0,
            _padding1: 0,
            payload_size: 0,
            _padding2: [0],
        };
        assert_eq!(header.magic, 0x50414E53);
    }

    #[test]
    fn test_ephys_shm_layout() {
        assert_eq!(size_of::<EphysShm>(), 640192);
        assert_eq!(align_of::<EphysShm>(), 64);
        let header = EphysShm {
            magic: EPHYS_MAGIC,
            state: 0,
            count: 0,
            max_ticks: 10000,
            current_tick: 0,
            _pad: [0; 11],
            target_tids: [0; 16],
            injection_uv: [0; 16],
            out_trace: [0; 160000],
        };
        assert_eq!(header.magic, 0x45504859);
    }

    #[test]
    fn test_state_offset_calculations() {
        // INV-LAYOUT-009: Verify compute_state_offsets aligns all individual array offsets to 64 bytes
        let offsets = compute_state_offsets(128);
        assert_eq!(offsets.soma_voltage % 64, 0);
        assert_eq!(offsets.flags % 64, 0);
        assert_eq!(offsets.threshold_offset % 64, 0);
        assert_eq!(offsets.timers % 64, 0);
        assert_eq!(offsets.soma_to_axon % 64, 0);
        assert_eq!(offsets.dendrite_targets % 64, 0);
        assert_eq!(offsets.dendrite_weights % 64, 0);
        assert_eq!(offsets.dendrite_timers % 64, 0);
        assert_eq!(offsets.total_size % 64, 0);
    }

    #[test]
    fn test_columnar_idx() {
        assert_eq!(columnar_idx(1024, 5, 1), 1029);
        assert_eq!(columnar_idx(64, 0, 2), 128);
    }

    #[test]
    fn test_calculate_state_blob_size_zero() {
        // E-012: Zero neurons yields zero padded count and zero size
        assert_eq!(calculate_state_blob_size(0), (0, 0));
    }

    #[test]
    fn test_calculate_paths_file_size_zero() {
        // E-015: Zero axons yields minimum file size of 16 bytes (only the PathsFileHeader)
        assert_eq!(calculate_paths_file_size(0), 16);
    }

    #[test]
    fn test_shard_vram_ptrs_layout() {
        assert_eq!(size_of::<ShardVramPtrs>(), 10 * size_of::<*mut u8>());
        assert_eq!(align_of::<ShardVramPtrs>(), align_of::<*mut u8>());
    }

    #[test]
    fn test_shard_soa_allocation() {
        let n = 1024;
        let axons = 5000;
        let soa = ShardStateSoA::new(n, axons);

        assert_eq!(soa.padded_n, n);
        assert_eq!(soa.voltage.len(), n);
        assert_eq!(soa.dendrite_weights.len(), n * 128);
        assert_eq!(soa.axon_heads.len(), axons);
    }

    #[test]
    fn test_vram_state_pointer_mapping() {
        let mut soa = ShardStateSoA::new(32, 100);
        soa.voltage[0] = 42;
        soa.axon_heads[99] = BurstHeads8::empty(123);

        unsafe {
            let vram = VramState::from_soa(&mut soa);
            assert_eq!(vram.padded_n, 32);
            assert_eq!(vram.total_axons, 100);
            assert_eq!(*vram.voltage, 42);
            assert_eq!((*vram.axon_heads.add(99)).h0, 123);
        }
    }

    #[test]
    fn test_dendrite_target_packing() {
        let t0 = pack_dendrite_target(0, 0);
        assert_ne!(t0, 0);
        assert_eq!(unpack_axon_id(t0), 0);
        assert_eq!(unpack_segment_offset(t0), 0);

        let t_max = pack_dendrite_target(0x00FF_FFFE, 255);
        assert_eq!(unpack_axon_id(t_max), 0x00FF_FFFE);
        assert_eq!(unpack_segment_offset(t_max), 255);

        let t_mix = pack_dendrite_target(0x123456, 0xAB);
        assert_eq!(unpack_axon_id(t_mix), 0x123456);
        assert_eq!(unpack_segment_offset(t_mix), 0xAB);
    }
}
