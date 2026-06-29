//! Packed 3D soma coordinates and variant type packing.

use crate::constants::{MAX_TYPE_ID, MAX_VOXEL_X, MAX_VOXEL_Y, MAX_VOXEL_Z};
use crate::error::TypeError;
use bytemuck::{Pod, Zeroable};

/// Packed 3D coordinate of neuron soma and binary index of its type within a shard.
/// Packed into a single 32-bit register (`u32`).
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct PackedPosition(pub u32);

impl PackedPosition {
    /// Validates whether the provided coordinates and variant type index are within allowable hardware limits.
    #[inline(always)]
    pub const fn is_valid_coords(x: u32, y: u32, z: u32, type_id: u8) -> bool {
        x <= MAX_VOXEL_X
            && y <= MAX_VOXEL_Y
            && z <= MAX_VOXEL_Z
            && (type_id as u32) <= (MAX_TYPE_ID as u32)
    }

    /// Checked constructor for boundary validation. Returns `Err(TypeError::PositionOutOfBounds)` if limits are exceeded.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::PositionOutOfBounds`] if `x > 1023`, `y > 1023`, `z > 255`, or `type_id > 15`.
    pub fn try_new(x: u32, y: u32, z: u32, type_id: u8) -> Result<Self, TypeError> {
        if Self::is_valid_coords(x, y, z, type_id) {
            Ok(Self::new(x, y, z, type_id))
        } else {
            Err(TypeError::PositionOutOfBounds { x, y, z, type_id })
        }
    }

    /// Fast total constructor for hot paths.
    ///
    /// # Panics
    ///
    /// In debug builds, panics via `debug_assert!` if coordinates or `type_id` exceed hardware limits.
    #[inline(always)]
    pub const fn new(x: u32, y: u32, z: u32, type_id: u8) -> Self {
        debug_assert!(x <= MAX_VOXEL_X, "X coordinate exceeds MAX_VOXEL_X");
        debug_assert!(y <= MAX_VOXEL_Y, "Y coordinate exceeds MAX_VOXEL_Y");
        debug_assert!(z <= MAX_VOXEL_Z, "Z coordinate exceeds MAX_VOXEL_Z");
        debug_assert!(
            (type_id as u32) <= (MAX_TYPE_ID as u32),
            "Type_ID exceeds MAX_TYPE_ID"
        );

        let x_q = x & 0x3FF;
        let y_q = y & 0x3FF;
        let z_q = z & 0xFF;
        let t_q = (type_id as u32) & 0xF;
        Self(x_q | (y_q << 10) | (z_q << 20) | (t_q << 28))
    }

    /// Extracts the X coordinate (bits 0..9, 0..1023).
    #[inline(always)]
    pub const fn x(&self) -> u16 {
        (self.0 & 0x3FF) as u16
    }

    /// Extracts the Y coordinate (bits 10..19, 0..1023).
    #[inline(always)]
    pub const fn y(&self) -> u16 {
        ((self.0 >> 10) & 0x3FF) as u16
    }

    /// Extracts the Z coordinate (bits 20..27, 0..255).
    #[inline(always)]
    pub const fn z(&self) -> u8 {
        ((self.0 >> 20) & 0xFF) as u8
    }

    /// Extracts the neuron profile variant identifier (bits 28..31, 0..15).
    #[inline(always)]
    pub const fn type_id(&self) -> u8 {
        ((self.0 >> 28) & 0xF) as u8
    }
}
