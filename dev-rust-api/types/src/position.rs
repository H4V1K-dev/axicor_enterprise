//! 3D coordinate and neuron type representation packed into a single 32-bit register.

/// Packed 3D position and type of a neuron.
///
/// Layout in memory (C-ABI equivalent to a raw `u32` register):
/// `[Type_ID(4b) | Z(8b) | Y(10b) | X(10b)]`
///
/// This structure enforces strict alignment and layout requirements for GPU
/// transaction coalescing and zero-copy host-to-device FFI transfers.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PackedPosition(pub u32);

impl PackedPosition {
    /// Packs raw coordinate and type fields into a single 32-bit `PackedPosition`.
    ///
    /// # Bit Bleed Prevention (E-001)
    /// This method enforces strict masking of all input arguments to prevent bits from
    /// bleeding into adjacent fields. If any coordinate exceeds its allocated bit width,
    /// it wraps around (modular arithmetic) according to its bitmask constraint.
    ///
    /// - `x`: Bounded to 10 bits (`& 0x3FF`).
    /// - `y`: Bounded to 10 bits (`& 0x3FF`).
    /// - `z`: Bounded to 8 bits (`& 0xFF`).
    /// - `type_id`: Bounded to 4 bits (`& 0xF`).
    #[inline]
    pub const fn pack_raw(x: u32, y: u32, z: u32, type_id: u8) -> Self {
        Self(((type_id as u32 & 0xF) << 28) | ((z & 0xFF) << 20) | ((y & 0x3FF) << 10) | (x & 0x3FF))
    }

    /// Unpacks and returns the 10-bit X coordinate.
    #[inline]
    pub const fn x(&self) -> u32 {
        self.0 & 0x3FF
    }

    /// Unpacks and returns the 10-bit Y coordinate.
    #[inline]
    pub const fn y(&self) -> u32 {
        (self.0 >> 10) & 0x3FF
    }

    /// Unpacks and returns the 8-bit Z coordinate.
    #[inline]
    pub const fn z(&self) -> u32 {
        (self.0 >> 20) & 0xFF
    }

    /// Unpacks and returns the 4-bit neuron type identifier.
    #[inline]
    pub const fn type_id(&self) -> u8 {
        ((self.0 >> 28) & 0xF) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_packed_position_memory_layout() {
        // INV-TYPES-001: size_of::<PackedPosition>() == 4 bytes
        assert_eq!(size_of::<PackedPosition>(), 4);
        assert_eq!(align_of::<PackedPosition>(), 4);
    }

    #[test]
    fn test_bit_bleed_masks() {
        // E-001: Verify that coordinates exceeding their bit limits do not corrupt neighboring fields
        // X = 1025 (1025 & 0x3FF = 1)
        // Y = 2048 (2048 & 0x3FF = 0)
        // Z = 257  (257 & 0xFF = 1)
        // Type = 18 (18 & 0xF = 2)
        let pos = PackedPosition::pack_raw(1025, 2048, 257, 18);
        assert_eq!(pos.x(), 1);
        assert_eq!(pos.y(), 0);
        assert_eq!(pos.z(), 1);
        assert_eq!(pos.type_id(), 2);
    }
}
