//! Target connection representation packed into a single 32-bit register.

/// Packed target pointer of a dendrite.
///
/// Layout in memory:
/// `[Segment_Offset(8b) | Axon_ID + 1(24b)]`
///
/// Enforces `#[repr(transparent)]` to guarantee C-ABI layout equivalence with a raw `u32` register.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PackedTarget(pub u32);

impl PackedTarget {
    /// Packs the axon ID and segment offset into a single 32-bit `PackedTarget`.
    ///
    /// # Zero-Index Trap Prevention (E-002)
    /// A hardware zero in the shared memory representation indicates the absence of a
    /// connection (`None`). To prevent a valid connection to `axon_id = 0` from being
    /// treated as a null connection, we increment `axon_id` by 1 during packing.
    /// Consequently, `axon_id = 0` is packed as `1` in memory, and an hardware `0`
    /// safely represents the lack of a target connection.
    ///
    /// - `axon_id`: Bounded to 24 bits (`& 0x00FFFFFF`).
    /// - `segment_offset`: Bounded to 8 bits (`& 0xFF`).
    #[inline]
    pub const fn pack(axon_id: u32, segment_offset: u32) -> Self {
        Self((segment_offset << 24) | ((axon_id + 1) & 0x00FFFFFF))
    }

    /// Unpacks and returns the 24-bit axon identifier, applying saturating subtraction.
    ///
    /// The saturating subtraction ensures that a packed value of `0` (representing `None`)
    /// returns `0` safely without underflowing or panicking.
    #[inline]
    pub const fn axon_id(&self) -> u32 {
        (self.0 & 0x00FFFFFF).saturating_sub(1)
    }

    /// Unpacks and returns the 8-bit segment offset.
    #[inline]
    pub const fn segment_offset(&self) -> u32 {
        self.0 >> 24
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_packed_target_memory_layout() {
        // INV-TYPES-002: size_of::<PackedTarget>() == 4 bytes
        assert_eq!(size_of::<PackedTarget>(), 4);
        assert_eq!(align_of::<PackedTarget>(), 4);
    }

    #[test]
    fn test_packed_target_zero_index_trap() {
        // E-002: Verify that packing axon_id=0, segment_offset=0 results in binary 1.
        let target = PackedTarget::pack(0, 0);
        assert_eq!(target.0, 1);

        // Verify that unpacking returns axon_id=0 and segment_offset=0.
        assert_eq!(target.axon_id(), 0);
        assert_eq!(target.segment_offset(), 0);

        // Verify that an actual binary 0 in memory unpacks to axon_id=0 (None equivalent).
        let null_target = PackedTarget(0);
        assert_eq!(null_target.axon_id(), 0);
        assert_eq!(null_target.segment_offset(), 0);
    }
}
