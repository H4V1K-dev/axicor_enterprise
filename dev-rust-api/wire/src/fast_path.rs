//! Data Transfer Objects for high-throughput UDP spike synchronization.

crate::decl_wire! {
    /// Header for L7-fragmented spike batches sent over UDP.
    ///
    /// Size: exactly 16 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct SpikeBatchHeaderV2 {
        pub src_zone_hash: u32,
        pub dst_zone_hash: u32,
        pub epoch: u32,
        pub chunk_idx: u16,
        pub total_chunks: u16,
    }
}

impl SpikeBatchHeaderV2 {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            src_zone_hash: self.src_zone_hash.to_le(),
            dst_zone_hash: self.dst_zone_hash.to_le(),
            epoch: self.epoch.to_le(),
            chunk_idx: self.chunk_idx.to_le(),
            total_chunks: self.total_chunks.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            src_zone_hash: u32::from_le(self.src_zone_hash),
            dst_zone_hash: u32::from_le(self.dst_zone_hash),
            epoch: u32::from_le(self.epoch),
            chunk_idx: u16::from_le(self.chunk_idx),
            total_chunks: u16::from_le(self.total_chunks),
        }
    }
}

crate::decl_wire! {
    /// A single spike event inside a UDP spike sync batch.
    ///
    /// Size: exactly 8 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct SpikeEventV2 {
        pub ghost_id: u32,
        pub tick_offset: u32,
    }
}

impl SpikeEventV2 {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            ghost_id: self.ghost_id.to_le(),
            tick_offset: self.tick_offset.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            ghost_id: u32::from_le(self.ghost_id),
            tick_offset: u32::from_le(self.tick_offset),
        }
    }
}

crate::decl_wire! {
    /// Header for binary telemetry frames sent over WebSockets.
    ///
    /// Size: exactly 16 bytes. Alignment: 8 bytes.
    #[derive(PartialEq, Eq)]
    #[repr(align(8))]
    pub struct TelemetryFrameHeader {
        pub magic: [u8; 4],
        pub tick: u32,
        pub count: u32,
        pub _padding: u32,
    }
}

impl TelemetryFrameHeader {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            magic: self.magic,
            tick: self.tick.to_le(),
            count: self.count.to_le(),
            _padding: self._padding.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            magic: self.magic,
            tick: u32::from_le(self.tick),
            count: u32::from_le(self.count),
            _padding: u32::from_le(self._padding),
        }
    }
}
