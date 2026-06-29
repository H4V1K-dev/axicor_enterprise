//! Configuration parameters for the CpuBackend implementation.

/// Configuration options for [`CpuBackend`](crate::CpuBackend).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CpuBackendConfig {
    /// Optional explicit worker thread count for the internal Rayon thread pool.
    ///
    /// If set to `Some(n)` where `n > 0`, exactly `n` worker threads will be created.
    /// If `None` or `Some(0)`, Rayon will select the default thread count matching host logical CPU cores.
    pub thread_count: Option<usize>,
}
