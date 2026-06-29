//! Compact 8-bit neuron soma state flag and runtime cache mirror.

use bytemuck::{Pod, Zeroable};

/// Bitmask for spiking flag (bit 0).
pub const SOMA_SPIKING_MASK: u8 = 0x01;
/// Bit shift for spiking flag.
pub const SOMA_SPIKING_SHIFT: u8 = 0;

/// Bitmask for burst count (bits 1..3).
pub const SOMA_BURST_MASK: u8 = 0x0E;
/// Bit shift for burst count.
pub const SOMA_BURST_SHIFT: u8 = 1;

/// Bitmask for runtime cached neuron profile variant type index (bits 4..7).
pub const SOMA_TYPE_MASK: u8 = 0xF0;
/// Bit shift for runtime cached neuron profile variant type index.
pub const SOMA_TYPE_SHIFT: u8 = 4;

/// Compact 8-bit neuron soma state flag for simulation hot path and SoA array alignment.
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct SomaFlags(pub u8);

impl SomaFlags {
    /// Constructs a new `SomaFlags` instance.
    #[inline(always)]
    pub const fn new(spiking: bool, burst_count: u8, type_id: u8) -> Self {
        let spk = (spiking as u8) & SOMA_SPIKING_MASK;
        let clamped_burst = if burst_count > 7 { 7 } else { burst_count };
        let burst = (clamped_burst << SOMA_BURST_SHIFT) & SOMA_BURST_MASK;
        let typ = (type_id << SOMA_TYPE_SHIFT) & SOMA_TYPE_MASK;
        Self(spk | burst | typ)
    }

    /// Returns `true` if the soma is currently in a spiking state.
    #[inline(always)]
    pub const fn spiking(&self) -> bool {
        (self.0 & SOMA_SPIKING_MASK) != 0
    }

    /// Returns the burst counter value (0..7).
    #[inline(always)]
    pub const fn burst_count(&self) -> u8 {
        (self.0 & SOMA_BURST_MASK) >> SOMA_BURST_SHIFT
    }

    /// Returns the runtime cached neuron profile variant type index (0..15).
    #[inline(always)]
    pub const fn type_id(&self) -> u8 {
        (self.0 & SOMA_TYPE_MASK) >> SOMA_TYPE_SHIFT
    }

    /// Mutates the spiking flag while strictly preserving `SOMA_TYPE_MASK`.
    #[inline(always)]
    pub fn set_spiking(&mut self, spiking: bool) {
        if spiking {
            self.0 |= SOMA_SPIKING_MASK;
        } else {
            self.0 &= !SOMA_SPIKING_MASK;
        }
    }

    /// Mutates the burst count while strictly preserving `SOMA_TYPE_MASK` and saturating at 7.
    #[inline(always)]
    pub fn set_burst_count(&mut self, count: u8) {
        let clamped = count.min(7);
        self.0 = (self.0 & !SOMA_BURST_MASK) | ((clamped << SOMA_BURST_SHIFT) & SOMA_BURST_MASK);
    }
}
