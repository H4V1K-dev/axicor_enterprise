//! C-ABI circular buffer layout for axon spike propagation heads.

use bytemuck::{Pod, Zeroable};
use types::AxonHead;

/// Ring buffer containing the 8 most recent axon spike head segment coordinates.
///
/// 32-byte aligned to enable single-transaction L1 cache vector fetches.
#[repr(C, align(32))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct BurstHeads8 {
    /// Propagation head slot 0.
    pub h0: AxonHead,
    /// Propagation head slot 1.
    pub h1: AxonHead,
    /// Propagation head slot 2.
    pub h2: AxonHead,
    /// Propagation head slot 3.
    pub h3: AxonHead,
    /// Propagation head slot 4.
    pub h4: AxonHead,
    /// Propagation head slot 5.
    pub h5: AxonHead,
    /// Propagation head slot 6.
    pub h6: AxonHead,
    /// Propagation head slot 7.
    pub h7: AxonHead,
}

impl BurstHeads8 {
    /// Creates a new `BurstHeads8` ring buffer initialized with the specified inactive sentinel head value.
    #[inline(always)]
    pub const fn empty(sentinel: AxonHead) -> Self {
        Self {
            h0: sentinel,
            h1: sentinel,
            h2: sentinel,
            h3: sentinel,
            h4: sentinel,
            h5: sentinel,
            h6: sentinel,
            h7: sentinel,
        }
    }
}
