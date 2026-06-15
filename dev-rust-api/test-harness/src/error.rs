use std::fmt;
use compute_api::ComputeApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DifferentialTestError {
    /// Failed to initialize backend or allocate shard memory.
    BackendInitFailed(ComputeApiError),
    /// Master seed values mismatch between CPU and GPU configurations.
    SeedDiscrepancy { cpu_seed: u64, gpu_seed: u64 },
    /// Structural layout configuration mismatch.
    LayoutMismatch,
    /// Memory state discrepancy found during bit-exact comparison.
    StateMismatch { tick: u64, offset: usize, cpu_val: u8, gpu_val: u8 },
    /// DMA transfer failure when downloading/uploading memory state.
    DmaFailure(ComputeApiError),
}

impl fmt::Display for DifferentialTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendInitFailed(err) => write!(f, "Backend initialization failed: {}", err),
            Self::SeedDiscrepancy { cpu_seed, gpu_seed } => {
                write!(f, "Seed discrepancy: CPU seed = {}, GPU seed = {}", cpu_seed, gpu_seed)
            }
            Self::LayoutMismatch => write!(f, "Structural layout configuration mismatch"),
            Self::StateMismatch { tick, offset, cpu_val, gpu_val } => {
                write!(
                    f,
                    "State mismatch at tick {}, offset {}: CPU value = {:#04x}, GPU value = {:#04x}",
                    tick, offset, cpu_val, gpu_val
                )
            }
            Self::DmaFailure(err) => write!(f, "DMA transfer failure: {}", err),
        }
    }
}

impl std::error::Error for DifferentialTestError {}
