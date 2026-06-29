//! Definition of hardware and mock compute backend identifiers.

/// Identifies the underlying hardware execution engine or mock implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Multi-threaded host CPU execution backend.
    Cpu,
    /// NVIDIA CUDA hardware acceleration backend.
    Cuda,
    /// AMD ROCm/HIP hardware acceleration backend.
    Hip,
    /// Mock execution backend for testing and validation.
    Mock,
}
