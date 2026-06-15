use std::fmt;

/// Error enum representing deterministic hardware and translation failures from GPU backends.
///
/// Implements `Debug`, `Clone`, `PartialEq`, `Eq`, `Send`, and `Sync` to satisfy `INV-COMPUTE-API-005`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeApiError {
    /// The accelerator or host RAM ran out of memory during allocation.
    OutOfMemory,
    /// The limit of dynamic ghost capacity or routing slots was exceeded.
    CapacityExceeded,
    /// An invalid, uninitialized (0), or already freed VRAM handle was used.
    InvalidHandle,
    /// The GPU device lost connection, timed out (TDR), or experienced a hardware crash.
    DeviceLost,
    /// Input memory buffers or configuration layout did not match C-ABI alignment or constraint boundaries.
    InvalidLayout,
    /// An error occurred during Host-to-Device or Device-to-Host DMA memory copying.
    DmaTransferFailed,
    /// A vendor-specific FFI driver error occurred, capturing the underlying raw error code.
    VendorError(i32),
}

impl fmt::Display for ComputeApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "VRAM or RAM Out of Memory"),
            Self::CapacityExceeded => write!(f, "Capacity of dynamic routing exceeded"),
            Self::InvalidHandle => write!(f, "Invalid or already freed VRAM handle"),
            Self::DeviceLost => write!(f, "Device lost (TDR timeout or hardware failure)"),
            Self::InvalidLayout => write!(f, "Invalid memory layout or alignment"),
            Self::DmaTransferFailed => write!(f, "DMA transfer failed"),
            Self::VendorError(code) => write!(f, "Vendor-specific error code: {}", code),
        }
    }
}

impl std::error::Error for ComputeApiError {}
