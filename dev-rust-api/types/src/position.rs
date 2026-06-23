//! 3D coordinate and neuron type representation packed into a single 32-bit register.

use crate::{Microns, Fraction, VoxelCoord};

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

    /// Alias to pack_raw for legacy code compatibility.
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32, type_id: u8) -> Self {
        Self::pack_raw(x, y, z, type_id)
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

/// Converts micrometers (um) to discrete voxel coordinates.
#[inline]
pub fn um_to_voxel(um: Microns, voxel_size_um: u32) -> VoxelCoord {
    (um / voxel_size_um as f32) as VoxelCoord
}

/// Converts normalized fraction to discrete voxel coordinates.
#[inline]
pub fn pct_to_voxel(pct: Fraction, world_dim_vox: u32) -> VoxelCoord {
    (pct * world_dim_vox as f32) as VoxelCoord
}

/// Converts discrete voxel coordinates to micrometers (um).
#[inline]
pub fn voxel_to_um(vox: VoxelCoord, voxel_size_um: u32) -> Microns {
    (vox as f32) * (voxel_size_um as f32)
}

/// Legacy helper to pack a position.
#[inline]
pub fn pack_position(x: u32, y: u32, z: u32, type_mask: u32) -> PackedPosition {
    PackedPosition::new(x, y, z, type_mask as u8)
}

/// Legacy helper to unpack a position.
#[inline]
pub fn unpack_position(p: PackedPosition) -> (u32, u32, u32, u32) {
    (p.x() as u32, p.y() as u32, p.z() as u32, p.type_id() as u32)
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

    #[test]
    fn test_packed_position_symmetry_and_boundaries() {
        // Test absolute boundary limits (max values)
        let max_pos = PackedPosition::pack_raw(1023, 1023, 255, 15);
        assert_eq!(max_pos.x(), 1023);
        assert_eq!(max_pos.y(), 1023);
        assert_eq!(max_pos.z(), 255);
        assert_eq!(max_pos.type_id(), 15);
        assert_eq!(max_pos.0, u32::MAX); // All 32 bits must be set

        // Test absolute zero limits
        let zero_pos = PackedPosition::pack_raw(0, 0, 0, 0);
        assert_eq!(zero_pos.x(), 0);
        assert_eq!(zero_pos.y(), 0);
        assert_eq!(zero_pos.z(), 0);
        assert_eq!(zero_pos.type_id(), 0);
        assert_eq!(zero_pos.0, 0);

        // Check alias constructor `new` behavior
        let alias_pos = PackedPosition::new(123, 456, 78, 9);
        assert_eq!(alias_pos.x(), 123);
        assert_eq!(alias_pos.y(), 456);
        assert_eq!(alias_pos.z(), 78);
        assert_eq!(alias_pos.type_id(), 9);

        // Determinism & Symmetry loop: check multiple values
        let mut x = 0;
        let mut y = 0;
        let mut z = 0;
        let mut t = 0;
        while x < 1024 {
            let pos = PackedPosition::pack_raw(x, y, z, t);
            assert_eq!(pos.x(), x);
            assert_eq!(pos.y(), y);
            assert_eq!(pos.z(), z);
            assert_eq!(pos.type_id(), t);

            x += 123;
            y = (y + 111) & 0x3FF;
            z = (z + 47) & 0xFF;
            t = (t + 3) & 0xF;
        }
    }

    #[test]
    fn test_coordinate_conversions() {
        // um_to_voxel: 50.0 um with voxel_size 25 -> 2 voxels
        assert_eq!(um_to_voxel(50.0, 25), 2);
        assert_eq!(um_to_voxel(12.5, 25), 0);

        // pct_to_voxel: 0.5 height of 100 voxels -> 50 voxels
        assert_eq!(pct_to_voxel(0.5, 100), 50);

        // voxel_to_um: 2 voxels with voxel_size 25 -> 50.0 um
        assert_eq!(voxel_to_um(2, 25), 50.0);

        // pack_position / unpack_position
        let p = pack_position(12, 34, 56, 7);
        let (x, y, z, t) = unpack_position(p);
        assert_eq!(x, 12);
        assert_eq!(y, 34);
        assert_eq!(z, 56);
        assert_eq!(t, 7);
    }
}
