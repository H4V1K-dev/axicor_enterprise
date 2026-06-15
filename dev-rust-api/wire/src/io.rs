//! Data Transfer Objects for external environment I/O (Sensors, Motors, and Dopamine R-STDP control).

crate::decl_wire! {
    /// Header for external environment sensor input or motor output packages.
    ///
    /// Size: exactly 20 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct ExternalIoHeader {
        pub magic: [u8; 4],
        pub zone_hash: u32,
        pub matrix_hash: u32,
        pub payload_size: u32,
        pub global_reward: i16,
        pub _padding: u16,
    }
}

impl ExternalIoHeader {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            magic: self.magic,
            zone_hash: self.zone_hash.to_le(),
            matrix_hash: self.matrix_hash.to_le(),
            payload_size: self.payload_size.to_le(),
            global_reward: self.global_reward.to_le(),
            _padding: self._padding.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            magic: self.magic,
            zone_hash: u32::from_le(self.zone_hash),
            matrix_hash: u32::from_le(self.matrix_hash),
            payload_size: u32::from_le(self.payload_size),
            global_reward: i16::from_le(self.global_reward),
            _padding: u16::from_le(self._padding),
        }
    }
}

crate::decl_wire! {
    /// Control packet for injecting global dopamine concentrations into the network.
    ///
    /// Size: exactly 8 bytes. Alignment: 8 bytes.
    #[derive(PartialEq, Eq)]
    #[repr(align(8))]
    pub struct ControlPacket {
        pub magic: [u8; 4],
        pub dopamine: i16,
        pub _pad: u16,
    }
}

impl ControlPacket {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            magic: self.magic,
            dopamine: self.dopamine.to_le(),
            _pad: self._pad.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            magic: self.magic,
            dopamine: i16::from_le(self.dopamine),
            _pad: u16::from_le(self._pad),
        }
    }
}

// =============================================================================
// Night Phase IPC DTO stubs (fields TBD — see PERSONAL/under_control.md)
// =============================================================================

crate::decl_wire! {
    /// Axon growth handover event transmitted from orchestrator to weaver-daemon.
    ///
    /// Size: exactly 20 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct AxonHandoverEvent {
        pub origin_zone_hash: u32,
        pub local_axon_id: u32,
        pub entry_x: u16,
        pub entry_y: u16,
        pub vector_x: i8,
        pub vector_y: i8,
        pub vector_z: i8,
        pub type_mask: u8,
        pub remaining_length: u16,
        pub entry_z: u8,
        pub _padding: u8,
    }
}

impl AxonHandoverEvent {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            origin_zone_hash: self.origin_zone_hash.to_le(),
            local_axon_id: self.local_axon_id.to_le(),
            entry_x: self.entry_x.to_le(),
            entry_y: self.entry_y.to_le(),
            vector_x: self.vector_x,
            vector_y: self.vector_y,
            vector_z: self.vector_z,
            type_mask: self.type_mask,
            remaining_length: self.remaining_length.to_le(),
            entry_z: self.entry_z,
            _padding: self._padding,
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            origin_zone_hash: u32::from_le(self.origin_zone_hash),
            local_axon_id: u32::from_le(self.local_axon_id),
            entry_x: u16::from_le(self.entry_x),
            entry_y: u16::from_le(self.entry_y),
            vector_x: self.vector_x,
            vector_y: self.vector_y,
            vector_z: self.vector_z,
            type_mask: self.type_mask,
            remaining_length: u16::from_le(self.remaining_length),
            entry_z: self.entry_z,
            _padding: self._padding,
        }
    }
}

crate::decl_wire! {
    /// Axon pruning command transmitted from orchestrator to weaver-daemon.
    ///
    /// Size: exactly 12 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct AxonHandoverPrune {
        pub target_zone_hash: u32,
        pub receiver_zone_hash: u32,
        pub dst_ghost_id: u32,
    }
}

impl AxonHandoverPrune {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            target_zone_hash: self.target_zone_hash.to_le(),
            receiver_zone_hash: self.receiver_zone_hash.to_le(),
            dst_ghost_id: self.dst_ghost_id.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            target_zone_hash: u32::from_le(self.target_zone_hash),
            receiver_zone_hash: u32::from_le(self.receiver_zone_hash),
            dst_ghost_id: u32::from_le(self.dst_ghost_id),
        }
    }
}

crate::decl_wire! {
    /// Bake request trigger package.
    ///
    /// Size: exactly 16 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct BakeRequest {
        pub magic: [u8; 4],
        pub zone_hash: u32,
        pub current_tick: u32,
        pub prune_threshold: i16,
        pub max_sprouts: u16,
    }
}

impl BakeRequest {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            magic: self.magic,
            zone_hash: self.zone_hash.to_le(),
            current_tick: self.current_tick.to_le(),
            prune_threshold: self.prune_threshold.to_le(),
            max_sprouts: self.max_sprouts.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            magic: self.magic,
            zone_hash: u32::from_le(self.zone_hash),
            current_tick: u32::from_le(self.current_tick),
            prune_threshold: i16::from_le(self.prune_threshold),
            max_sprouts: u16::from_le(self.max_sprouts),
        }
    }
}

crate::decl_wire! {
    /// Acknowledgment of axon handover event.
    ///
    /// Size: exactly 16 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct AxonHandoverAck {
        pub target_zone_hash: u32,
        pub receiver_zone_hash: u32,
        pub src_axon_id: u32,
        pub dst_ghost_id: u32,
    }
}

impl AxonHandoverAck {
    /// Convert all numeric fields from native to little-endian representation in place.
    pub fn to_le(self) -> Self {
        Self {
            target_zone_hash: self.target_zone_hash.to_le(),
            receiver_zone_hash: self.receiver_zone_hash.to_le(),
            src_axon_id: self.src_axon_id.to_le(),
            dst_ghost_id: self.dst_ghost_id.to_le(),
        }
    }

    /// Convert all numeric fields from little-endian to native representation in place.
    pub fn from_le(self) -> Self {
        Self {
            target_zone_hash: u32::from_le(self.target_zone_hash),
            receiver_zone_hash: u32::from_le(self.receiver_zone_hash),
            src_axon_id: u32::from_le(self.src_axon_id),
            dst_ghost_id: u32::from_le(self.dst_ghost_id),
        }
    }
}

crate::decl_wire! {
    /// Ghost atlas routing connection: maps a source GXO soma to a target-shard soma ID.
    ///
    /// Size: exactly 8 bytes. Alignment: 4 bytes.
    #[derive(PartialEq, Eq)]
    pub struct GhostConnection {
        /// Source soma ID from the GXO matrix pixel.
        pub src_soma_id: u32,
        /// Target shard soma ID. Set to `EMPTY_PIXEL` (0xFFFF_FFFF) if no soma found (E-079).
        pub target_ghost_id: u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_axon_handover_event_layout() {
        assert_eq!(size_of::<AxonHandoverEvent>(), 20);
        assert_eq!(align_of::<AxonHandoverEvent>(), 4);
    }

    #[test]
    fn test_axon_handover_prune_layout() {
        assert_eq!(size_of::<AxonHandoverPrune>(), 12);
        assert_eq!(align_of::<AxonHandoverPrune>(), 4);
    }

    #[test]
    fn test_bake_request_layout() {
        assert_eq!(size_of::<BakeRequest>(), 16);
        assert_eq!(align_of::<BakeRequest>(), 4);
    }

    #[test]
    fn test_axon_handover_ack_layout() {
        assert_eq!(size_of::<AxonHandoverAck>(), 16);
        assert_eq!(align_of::<AxonHandoverAck>(), 4);
    }

    #[test]
    fn test_handover_endian_roundtrip() {
        let event = AxonHandoverEvent {
            origin_zone_hash: 0x12345678,
            local_axon_id: 0x9ABCDEF0,
            entry_x: 0x1122,
            entry_y: 0x3344,
            vector_x: -5,
            vector_y: 12,
            vector_z: -120,
            type_mask: 0xAA,
            remaining_length: 0x5566,
            entry_z: 0xBB,
            _padding: 0,
        };
        assert_eq!(event.to_le().from_le(), event);

        let prune = AxonHandoverPrune {
            target_zone_hash: 0x12345678,
            receiver_zone_hash: 0x9ABCDEF0,
            dst_ghost_id: 0x11223344,
        };
        assert_eq!(prune.to_le().from_le(), prune);

        let req = BakeRequest {
            magic: *b"BAKE",
            zone_hash: 0x12345678,
            current_tick: 0x9ABCDEF0,
            prune_threshold: -1234,
            max_sprouts: 5678,
        };
        assert_eq!(req.to_le().from_le(), req);

        let ack = AxonHandoverAck {
            target_zone_hash: 0x12345678,
            receiver_zone_hash: 0x9ABCDEF0,
            src_axon_id: 0x11223344,
            dst_ghost_id: 0x55667788,
        };
        assert_eq!(ack.to_le().from_le(), ack);
    }
}


