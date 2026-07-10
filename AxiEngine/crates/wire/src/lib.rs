//! Layer 1 binary data contracts and network serialization formats for AxiEngine.
//!
//! This crate defines the C-ABI layouts of network payloads and cross-shard event representations.
//! It is strictly `no_std`, `no_alloc`, and side-effect free.

#![no_std]

/// Magic signature for external IO sensor inputs ("GSIO").
pub const MAGIC_GSIO: u32 = u32::from_le_bytes(*b"GSIO");

/// Magic signature for external IO actuator outputs ("GSOO").
pub const MAGIC_GSOO: u32 = u32::from_le_bytes(*b"GSOO");

/// Magic signature for routing table updates ("ROUT").
pub const MAGIC_ROUT: u32 = u32::from_le_bytes(*b"ROUT");

/// Magic signature for dopamine plastic modulation ("DOPA").
pub const MAGIC_DOPA: u32 = u32::from_le_bytes(*b"DOPA");

/// Magic signature for shard telemetry frames ("TELE").
pub const MAGIC_TELE: u32 = u32::from_le_bytes(*b"TELE");

/// Representation of a single spike event in the spike batch payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpikeEventV2 {
    /// Global identifier of the target ghost axon.
    pub ghost_id: u32,
    /// Simulation tick offset inside the current batch.
    pub tick_offset: u32,
}

impl SpikeEventV2 {
    /// Creates a new spike event DTO.
    pub const fn new(ghost_id: u32, tick_offset: u32) -> Self {
        Self {
            ghost_id,
            tick_offset,
        }
    }
}

/// Binary header structure for spike propagation batch packets.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpikeBatchHeaderV2 {
    /// FNV-1a hash of the source zone configuration.
    pub src_zone_hash: u32,
    /// FNV-1a hash of the target zone configuration.
    pub dst_zone_hash: u32,
    /// Simulation epoch identifier.
    pub epoch: u32,
    /// Fragment index in the L7 segmentation layer.
    pub chunk_idx: u16,
    /// Total number of fragments in the L7 batch.
    pub total_chunks: u16,
}

impl SpikeBatchHeaderV2 {
    /// Creates a new spike batch header DTO.
    pub const fn new(
        src_zone_hash: u32,
        dst_zone_hash: u32,
        epoch: u32,
        chunk_idx: u16,
        total_chunks: u16,
    ) -> Self {
        Self {
            src_zone_hash,
            dst_zone_hash,
            epoch,
            chunk_idx,
            total_chunks,
        }
    }
}

/// Binary header structure for external input/output mapping matrices.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExternalIoHeader {
    /// Magic signature identifier, either GSIO or GSOO.
    pub magic: u32,
    /// FNV-1a hash of the associated zone configuration.
    pub zone_hash: u32,
    /// Hash verification of the matrix configuration layout.
    pub matrix_hash: u32,
    /// Length of the raw binary payload matrix in bytes.
    pub payload_size: u32,
    /// Global reinforcement learning reward signal.
    pub global_reward: i16,
    /// Explicit structure alignment padding.
    pub _padding: u16,
}

impl ExternalIoHeader {
    /// Creates a new external IO header DTO with zeroed padding.
    pub const fn new(
        magic: u32,
        zone_hash: u32,
        matrix_hash: u32,
        payload_size: u32,
        global_reward: i16,
    ) -> Self {
        Self {
            magic,
            zone_hash,
            matrix_hash,
            payload_size,
            global_reward,
            _padding: 0,
        }
    }
}

/// Message payload updating the network routing tables across shards.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RouteUpdate {
    /// Magic signature identifier, strictly ROUT.
    pub magic: u32,
    /// FNV-1a hash of the updating zone config.
    pub zone_hash: u32,
    /// New IPv4 address in Little-Endian byte order.
    pub new_ipv4: u32,
    /// New UDP target port number.
    pub new_port: u16,
    /// MTU limit configuration for the link.
    pub mtu: u16,
    /// Security validation secret code for the cluster.
    pub cluster_secret: u64,
}

impl RouteUpdate {
    /// Creates a new route update DTO.
    pub const fn new(
        zone_hash: u32,
        new_ipv4: u32,
        new_port: u16,
        mtu: u16,
        cluster_secret: u64,
    ) -> Self {
        Self {
            magic: MAGIC_ROUT,
            zone_hash,
            new_ipv4,
            new_port,
            mtu,
            cluster_secret,
        }
    }
}

/// Control payload carrying dopamine concentrations for plasticity.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ControlPacket {
    /// Magic signature identifier, strictly DOPA.
    pub magic: u32,
    /// Current concentration level of dopamine.
    pub dopamine: i16,
    /// Explicit structure alignment padding.
    pub _pad: u16,
}

impl ControlPacket {
    /// Creates a new dopamine control packet DTO with zeroed padding.
    pub const fn new(dopamine: i16) -> Self {
        Self {
            magic: MAGIC_DOPA,
            dopamine,
            _pad: 0,
        }
    }
}

/// Binary header structure describing a telemetry frame package.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TelemetryFrameHeader {
    /// Magic signature identifier, strictly TELE.
    pub magic: u32,
    /// Monotonically increasing simulation tick identifier.
    pub tick: u32,
    /// Number of active spike events compiled into the payload.
    pub spikes_count: u32,
    /// Explicit structure alignment padding.
    pub _padding: u32,
}

impl TelemetryFrameHeader {
    /// Creates a new telemetry frame header DTO with zeroed padding.
    pub const fn new(tick: u32, spikes_count: u32) -> Self {
        Self {
            magic: MAGIC_TELE,
            tick,
            spikes_count,
            _padding: 0,
        }
    }
}

/// Cross-shard biological axon handover growth parameter event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AxonHandoverEvent {
    /// FNV-1a hash of the origin zone.
    pub origin_zone_hash: u32,
    /// Unique local identifier of the growth source axon.
    pub local_axon_id: u32,
    /// Boundary voxel coordinate X of growth entry.
    pub entry_x: u16,
    /// Boundary voxel coordinate Y of growth entry.
    pub entry_y: u16,
    /// Steer vector direction coordinate X.
    pub vector_x: i8,
    /// Steer vector direction coordinate Y.
    pub vector_y: i8,
    /// Steer vector direction coordinate Z.
    pub vector_z: i8,
    /// Numerical mask specifying biological types.
    pub type_mask: u8,
    /// Remaining biological growth length steps.
    pub remaining_length: u16,
    /// Boundary voxel coordinate Z of growth entry.
    pub entry_z: u8,
    /// Explicit structure alignment padding.
    pub _padding: u8,
}

impl AxonHandoverEvent {
    /// Creates a new axon handover event DTO with zeroed padding.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        origin_zone_hash: u32,
        local_axon_id: u32,
        entry_x: u16,
        entry_y: u16,
        vector_x: i8,
        vector_y: i8,
        vector_z: i8,
        type_mask: u8,
        remaining_length: u16,
        entry_z: u8,
    ) -> Self {
        Self {
            origin_zone_hash,
            local_axon_id,
            entry_x,
            entry_y,
            vector_x,
            vector_y,
            vector_z,
            type_mask,
            remaining_length,
            entry_z,
            _padding: 0,
        }
    }
}

/// Symmetrical acknowledgement message confirming handover integration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AxonHandoverAck {
    /// FNV-1a hash of the target configuration zone.
    pub target_zone_hash: u32,
    /// FNV-1a hash of the receiver configuration zone.
    pub receiver_zone_hash: u32,
    /// Source identification index of the handover event.
    pub src_axon_id: u32,
    /// Allocated global identifier of the destination ghost axon.
    pub dst_ghost_id: u32,
}

impl AxonHandoverAck {
    /// Creates a new axon handover ack DTO.
    pub const fn new(
        target_zone_hash: u32,
        receiver_zone_hash: u32,
        src_axon_id: u32,
        dst_ghost_id: u32,
    ) -> Self {
        Self {
            target_zone_hash,
            receiver_zone_hash,
            src_axon_id,
            dst_ghost_id,
        }
    }
}

/// Pruning request message signal to tear down target ghost connections.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AxonHandoverPrune {
    /// FNV-1a hash of the target configuration zone.
    pub target_zone_hash: u32,
    /// FNV-1a hash of the receiver configuration zone.
    pub receiver_zone_hash: u32,
    /// Identifier of the destination ghost axon to prune.
    pub dst_ghost_id: u32,
}

impl AxonHandoverPrune {
    /// Creates a new axon handover prune DTO.
    pub const fn new(target_zone_hash: u32, receiver_zone_hash: u32, dst_ghost_id: u32) -> Self {
        Self {
            target_zone_hash,
            receiver_zone_hash,
            dst_ghost_id,
        }
    }
}

// Compile-time assertions verifying size and alignments for ABI safety.
const _: () = assert!(core::mem::size_of::<SpikeEventV2>() == 8);
const _: () = assert!(core::mem::align_of::<SpikeEventV2>() == 4);

const _: () = assert!(core::mem::size_of::<SpikeBatchHeaderV2>() == 16);
const _: () = assert!(core::mem::align_of::<SpikeBatchHeaderV2>() == 4);

const _: () = assert!(core::mem::size_of::<ExternalIoHeader>() == 20);
const _: () = assert!(core::mem::align_of::<ExternalIoHeader>() == 4);

const _: () = assert!(core::mem::size_of::<RouteUpdate>() == 24);
const _: () = assert!(core::mem::align_of::<RouteUpdate>() == 8);

const _: () = assert!(core::mem::size_of::<ControlPacket>() == 8);
const _: () = assert!(core::mem::align_of::<ControlPacket>() == 4);

const _: () = assert!(core::mem::size_of::<TelemetryFrameHeader>() == 16);
const _: () = assert!(core::mem::align_of::<TelemetryFrameHeader>() == 4);

const _: () = assert!(core::mem::size_of::<AxonHandoverEvent>() == 20);
const _: () = assert!(core::mem::align_of::<AxonHandoverEvent>() == 4);

const _: () = assert!(core::mem::size_of::<AxonHandoverAck>() == 16);
const _: () = assert!(core::mem::align_of::<AxonHandoverAck>() == 4);

const _: () = assert!(core::mem::size_of::<AxonHandoverPrune>() == 12);
const _: () = assert!(core::mem::align_of::<AxonHandoverPrune>() == 4);

/// Error variants occurred during network payload parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Input buffer is too small to contain the requested header structure.
    BufferTooSmall,
    /// Alignment of the source buffer does not permit zero-copy cast.
    AlignmentMismatch,
    /// Magic signature field does not match expected constants.
    InvalidMagic,
    /// Version header mismatch.
    UnsupportedVersion,
    /// Payload slice size does not correspond to headers.
    PayloadSizeMismatch,
    /// Length field exceeds allowed limits.
    InvalidLength,
    /// Integer math overflow occurred.
    IntegerOverflow,
    /// Invalid packet type mapping identified.
    InvalidPacketKind,
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Helper returning compile-time size of type T.
#[inline(always)]
pub const fn wire_size_of<T>() -> usize {
    core::mem::size_of::<T>()
}

/// Attempts copying an unaligned byte buffer into a clean structure layout.
pub fn try_read_header<T: bytemuck::Pod>(bytes: &[u8]) -> Result<T, WireError> {
    if bytes.len() < core::mem::size_of::<T>() {
        return Err(WireError::BufferTooSmall);
    }
    unsafe { Ok(core::ptr::read_unaligned(bytes.as_ptr() as *const T)) }
}

/// Safely extracts the reference to payload bytes trailing the headers.
pub fn payload_slice(
    packet: &[u8],
    header_size: usize,
    payload_size: usize,
) -> Result<&[u8], WireError> {
    let total_required = header_size
        .checked_add(payload_size)
        .ok_or(WireError::IntegerOverflow)?;
    if packet.len() < total_required {
        return Err(WireError::PayloadSizeMismatch);
    }
    Ok(&packet[header_size..total_required])
}

/// Validates overall packet size matches internal payload records.
pub fn validate_external_io_len(
    header: &ExternalIoHeader,
    packet_len: usize,
) -> Result<(), WireError> {
    let expected = 20usize
        .checked_add(header.payload_size as usize)
        .ok_or(WireError::IntegerOverflow)?;
    if packet_len != expected {
        return Err(WireError::PayloadSizeMismatch);
    }
    Ok(())
}

/// Helper calculating expected payload buffer size for event count.
pub fn spike_events_payload_len(event_count: usize) -> Result<usize, WireError> {
    event_count.checked_mul(8).ok_or(WireError::IntegerOverflow)
}
