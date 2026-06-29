//! Unified error enumeration for compute HAL operations.

use core::fmt;

/// Unified error conditions returned by compute HAL operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeApiError {
    /// Provided VRAM handle is invalid or unknown to the backend.
    InvalidHandle,
    /// Provided VRAM handle belongs to a different backend instance.
    ForeignHandle,
    /// Attempted operation on a resource that has already been freed.
    AlreadyFreed,
    /// Specification or buffer shape parameters are invalid.
    InvalidShape,
    /// Memory alignment requirement violated.
    AlignmentViolation,
    /// Buffer physical byte size does not match expected layout formula.
    SizeMismatch,
    /// Requested batch size or spike buffer capacity exceeded.
    CapacityExceeded,
    /// Host or device memory allocation failed.
    OutOfMemory,
    /// Hardware device connection lost or unrecoverable hardware error occurred.
    DeviceLost,
    /// Vendor-specific hardware SDK error.
    VendorError {
        /// Vendor native error code.
        code: i32,
    },
    /// Direct Memory Access (DMA) transfer failed.
    DmaFailed,
    /// Hardware compute kernel launch failed.
    KernelLaunchFailed,
    /// Asynchronous stream synchronization failed.
    SynchronizeFailed,
    /// Requested compute backend kind is not supported or not compiled.
    UnsupportedBackend,
    /// Operation or diagnostic feature is not supported by this backend.
    UnsupportedFeature,
    /// Backend engine has not been initialized.
    BackendNotInitialized,
    /// Invalid parameters provided in day batch execution command.
    InvalidBatch,
    /// Diagnostic snapshot buffer bounds or shape are invalid.
    InvalidDebugProbeBounds,
}

impl fmt::Display for ComputeApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => f.write_str("invalid VRAM handle"),
            Self::ForeignHandle => f.write_str("VRAM handle belongs to foreign backend instance"),
            Self::AlreadyFreed => f.write_str("VRAM resource has already been freed"),
            Self::InvalidShape => f.write_str("invalid allocation specification or buffer shape"),
            Self::AlignmentViolation => f.write_str("memory alignment requirement violated"),
            Self::SizeMismatch => f.write_str("buffer size does not match expected layout formula"),
            Self::CapacityExceeded => {
                f.write_str("requested capacity or spike buffer limit exceeded")
            }
            Self::OutOfMemory => f.write_str("out of host or device memory"),
            Self::DeviceLost => f.write_str("hardware device lost or unrecoverable hardware fault"),
            Self::VendorError { code } => write!(f, "vendor SDK error (code: {code})"),
            Self::DmaFailed => f.write_str("DMA transfer failed"),
            Self::KernelLaunchFailed => f.write_str("kernel launch failed"),
            Self::SynchronizeFailed => f.write_str("stream synchronization failed"),
            Self::UnsupportedBackend => f.write_str("unsupported compute backend"),
            Self::UnsupportedFeature => f.write_str("unsupported feature or diagnostic probe"),
            Self::BackendNotInitialized => f.write_str("compute backend not initialized"),
            Self::InvalidBatch => f.write_str("invalid parameters in batch command"),
            Self::InvalidDebugProbeBounds => f.write_str("invalid debug snapshot buffer bounds"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ComputeApiError {}
