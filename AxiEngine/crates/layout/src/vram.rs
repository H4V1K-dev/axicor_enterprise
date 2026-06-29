//! C-ABI data transfer object (DTO) containing VRAM device memory pointers for compute kernel launches.

use crate::burst::BurstHeads8;

/// C-ABI contract transferring raw device memory pointers between host orchestrator and compute backends.
///
/// SAFETY: This structure contains raw mutable device pointers. It strictly implements `#[repr(C)]`
/// to guarantee bit-for-bit layout matching across FFI boundaries. It MUST NOT implement `bytemuck::Pod`
/// or undergo raw byte casting.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShardVramPtrs {
    /// Pointer to soma voltage plane (`padded_n * 4B`).
    pub soma_voltage: *mut i32,
    /// Pointer to soma flags plane (`padded_n * 1B`).
    pub soma_flags: *mut u8,
    /// Pointer to threshold offset plane (`padded_n * 4B`).
    pub threshold_offset: *mut i32,
    /// Pointer to soma refractory timer plane (`padded_n * 1B`).
    pub timers: *mut u8,
    /// Pointer to soma-to-axon routing table plane (`padded_n * 4B`).
    pub soma_to_axon: *mut u32,
    /// Pointer to dendritic targets matrix plane (`MAX_DENDRITES * padded_n * 4B`).
    pub dendrite_targets: *mut u32,
    /// Pointer to dendritic weights matrix plane (`MAX_DENDRITES * padded_n * 4B`).
    pub dendrite_weights: *mut i32,
    /// Pointer to dendritic refractory timers matrix plane (`MAX_DENDRITES * padded_n * 1B`).
    pub dendrite_timers: *mut u8,
    /// Pointer to axon propagation heads ring buffer plane (`total_axons * 32B`).
    pub axon_heads: *mut BurstHeads8,
}

impl ShardVramPtrs {
    /// Creates a new `ShardVramPtrs` instance with all raw pointers initialized to null.
    #[inline(always)]
    pub const fn null() -> Self {
        Self {
            soma_voltage: core::ptr::null_mut(),
            soma_flags: core::ptr::null_mut(),
            threshold_offset: core::ptr::null_mut(),
            timers: core::ptr::null_mut(),
            soma_to_axon: core::ptr::null_mut(),
            dendrite_targets: core::ptr::null_mut(),
            dendrite_weights: core::ptr::null_mut(),
            dendrite_timers: core::ptr::null_mut(),
            axon_heads: core::ptr::null_mut(),
        }
    }
}
