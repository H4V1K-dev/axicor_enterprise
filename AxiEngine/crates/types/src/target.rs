//! Packed synaptic target address and state inspector functions.

use crate::constants::{EMPTY_PIXEL, MAX_AXON_ID, MAX_SEGMENT_OFFSET};
use crate::error::TypeError;
use bytemuck::{Pod, Zeroable};

/// Packed target address of a dendritic synaptic contact.
/// Connects a soma dendrite to a specific axon and segment offset on it.
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct PackedTarget(pub u32);

impl PackedTarget {
    /// Inactive zero-initialized sentinel constant.
    pub const NONE: Self = Self(0);

    /// Pruned tombstone sentinel constant (`0xFFFF_FFFF`).
    pub const TOMBSTONE: Self = Self(EMPTY_PIXEL);

    /// Validates whether the provided `axon_id` and `segment_offset` are within allowable domain limits.
    #[inline(always)]
    pub const fn is_valid_target(axon_id: u32, segment_offset: u32) -> bool {
        axon_id <= MAX_AXON_ID && segment_offset <= MAX_SEGMENT_OFFSET
    }

    /// Checked constructor for target packing. Returns `Err(TypeError::TargetOutOfBounds)` if limits are exceeded.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::TargetOutOfBounds`] if `axon_id > 16_777_213` or `segment_offset > 255`.
    pub fn try_pack(axon_id: u32, segment_offset: u32) -> Result<Self, TypeError> {
        if Self::is_valid_target(axon_id, segment_offset) {
            Ok(Self::pack(axon_id, segment_offset))
        } else {
            Err(TypeError::TargetOutOfBounds {
                axon_id,
                segment_offset,
            })
        }
    }

    /// Fast total constructor for hot paths.
    ///
    /// # Panics
    ///
    /// In debug builds, panics via `debug_assert!` if inputs exceed allowed limits.
    #[inline(always)]
    pub const fn pack(axon_id: u32, segment_offset: u32) -> Self {
        debug_assert!(axon_id <= MAX_AXON_ID, "axon_id exceeds MAX_AXON_ID");
        debug_assert!(
            segment_offset <= MAX_SEGMENT_OFFSET,
            "segment_offset exceeds MAX_SEGMENT_OFFSET"
        );

        let axon_q = axon_id.wrapping_add(1) & 0x00FFFFFF;
        let seg_q = (segment_offset & 0xFF) << 24;
        Self(axon_q | seg_q)
    }

    /// Returns `true` if this target is the zero-init None sentinel (`0`).
    #[inline(always)]
    pub const fn is_zero_none(&self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if this target is the pruned tombstone sentinel (`EMPTY_PIXEL` / `0xFFFF_FFFF`).
    #[inline(always)]
    pub const fn is_tombstone(&self) -> bool {
        self.0 == EMPTY_PIXEL
    }

    /// Returns `true` if this target slot is inactive (`0` or `EMPTY_PIXEL`).
    /// Used by compute kernels for O(1) hardware Early Exit.
    #[inline(always)]
    pub const fn is_inactive(&self) -> bool {
        self.0 == 0 || self.0 == EMPTY_PIXEL
    }

    /// Returns `true` if this target slot is active (not inactive).
    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        !self.is_inactive()
    }

    /// Returns `true` if the raw bit pattern represents a valid inactive state or a valid live target.
    #[inline(always)]
    pub const fn is_valid_raw(&self) -> bool {
        if self.is_inactive() {
            true
        } else {
            let axon_q = self.0 & 0x00FFFFFF;
            axon_q >= 1 && axon_q <= MAX_AXON_ID + 1
        }
    }

    /// Returns `true` if the raw bit pattern represents a reserved or corrupt bit encoding.
    #[inline(always)]
    pub const fn is_reserved_encoding(&self) -> bool {
        !self.is_valid_raw()
    }

    /// Total safe unpacking method. Returns `None` for inactive, reserved, or corrupt slots.
    /// Guarantees complete absence of panics and underflow for any raw `u32` value.
    #[inline(always)]
    pub const fn unpack(&self) -> Option<(u32, u32)> {
        if self.is_inactive() {
            None
        } else {
            let axon_q = self.0 & 0x00FFFFFF;
            if axon_q == 0 || axon_q > MAX_AXON_ID + 1 {
                None
            } else {
                let axon_id = axon_q - 1;
                let segment_offset = (self.0 >> 24) & 0xFF;
                Some((axon_id, segment_offset))
            }
        }
    }

    /// Checked unpacking method for validators and AOT tooling.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::CorruptTarget`] if the bit pattern is a reserved or corrupt encoding.
    pub fn try_unpack(&self) -> Result<Option<(u32, u32)>, TypeError> {
        if self.is_reserved_encoding() {
            Err(TypeError::CorruptTarget { raw: self.0 })
        } else {
            Ok(self.unpack())
        }
    }
}
