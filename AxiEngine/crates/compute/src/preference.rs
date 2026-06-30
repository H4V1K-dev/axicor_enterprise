//! Compute backend selection preference model.

/// Selection preference for compute execution engine backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendPreference {
    /// Automatic detection in order: CUDA -> HIP -> CPU.
    Auto,
    /// Multi-threaded CPU reference backend.
    Cpu,
    /// NVIDIA CUDA high-performance backend.
    Cuda {
        /// Target device ID.
        device_id: u32,
    },
    /// AMD ROCm/HIP high-performance backend.
    Hip {
        /// Target device ID.
        device_id: u32,
    },
    /// Test Mock backend.
    Mock,
}
