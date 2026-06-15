//! Soma state representation packed into a single 8-bit register.

/// Soma flags containing state indicators and type identifier.
///
/// Layout in memory:
/// `[Type_ID(4b) | Burst_Count(3b) | Is_Spiking(1b)]`
///
/// Enforces `#[repr(transparent)]` to guarantee C-ABI layout equivalence with a raw `u8` register.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SomaFlags(pub u8);

impl SomaFlags {
    /// Packs the type ID, burst count, and spike flag into a single 8-bit `SomaFlags`.
    ///
    /// # Burst-count saturation clamp (E-003)
    /// The burst count is represented using 3 bits (values 0..7). To protect BDP-plasticity
    /// calculations from rollover failures (where an 8th spike would reset the counter to 0),
    /// this function clamps the `burst_count` argument to a maximum of `7` before packing.
    ///
    /// - `type_id`: Bounded to 4 bits (`& 0x0F`).
    /// - `burst_count`: Bounded to 3 bits and clamped to 7 (`.min(7) & 0x07`).
    /// - `is_spiking`: Encoded as a single bit.
    #[inline]
    pub const fn pack(type_id: u8, burst_count: u8, is_spiking: bool) -> Self {
        let burst = if burst_count < 7 { burst_count } else { 7 };
        Self(((type_id & 0x0F) << 4) | ((burst & 0x07) << 1) | (is_spiking as u8))
    }

    /// Unpacks and returns the 4-bit type identifier.
    #[inline]
    pub const fn type_id(&self) -> u8 {
        self.0 >> 4
    }

    /// Unpacks and returns the 3-bit burst count.
    #[inline]
    pub const fn burst_count(&self) -> u8 {
        (self.0 >> 1) & 0x07
    }

    /// Unpacks and returns the spike flag.
    #[inline]
    pub const fn is_spiking(&self) -> bool {
        (self.0 & 0x01) != 0
    }

    /// Returns a new `SomaFlags` with the spike flag mutated.
    #[inline]
    pub const fn with_spiking(self, is_spiking: bool) -> Self {
        Self((self.0 & !0x01) | (is_spiking as u8))
    }

    /// Returns a new `SomaFlags` with the burst count mutated.
    ///
    /// # Burst-count saturation clamp (E-003)
    /// Clamps the `count` argument to `7` before mutating to prevent bit overrun or counter rollover,
    /// safeguarding BDP-plasticity calculations.
    #[inline]
    pub const fn with_burst_count(self, count: u8) -> Self {
        let burst = if count < 7 { count } else { 7 };
        Self((self.0 & !0x0E) | ((burst & 0x07) << 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_soma_flags_memory_layout() {
        // INV-TYPES-003: size_of::<SomaFlags>() == 1 byte
        assert_eq!(size_of::<SomaFlags>(), 1);
        assert_eq!(align_of::<SomaFlags>(), 1);
    }

    #[test]
    fn test_burst_saturation() {
        // E-003: Verify that packing with burst_count = 8 clamps the burst count to 7
        let flags = SomaFlags::pack(5, 8, true);
        assert_eq!(flags.type_id(), 5);
        assert_eq!(flags.burst_count(), 7);
        assert_eq!(flags.is_spiking(), true);

        // Verify that updating a flag with a burst_count of 10 clamps it to 7
        let updated = flags.with_burst_count(10);
        assert_eq!(updated.type_id(), 5);
        assert_eq!(updated.burst_count(), 7);
        assert_eq!(updated.is_spiking(), true);
        
        // Verify that other bits (like type_id and is_spiking) are unaffected by saturation clamp
        let updated_zero = updated.with_burst_count(0);
        assert_eq!(updated_zero.type_id(), 5);
        assert_eq!(updated_zero.burst_count(), 0);
        assert_eq!(updated_zero.is_spiking(), true);
    }
}
