use std::fmt;

#[derive(Debug)]
pub enum RuntimeError {
    DaemonTimeout,
    ComputeError(compute_api::ComputeApiError),
    CheckpointLoad(std::io::Error),
    UnstableWarmup,
    ChannelError,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DaemonTimeout => write!(f, "Daemon execution timeout occurred"),
            Self::ComputeError(e) => write!(f, "Compute backend error: {}", e),
            Self::CheckpointLoad(e) => write!(f, "Failed to load checkpoint: {}", e),
            Self::UnstableWarmup => write!(f, "Simulation state warmup unstable"),
            Self::ChannelError => write!(f, "Inter-thread channel communication error"),
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ComputeError(e) => Some(e),
            Self::CheckpointLoad(e) => Some(e),
            _ => None,
        }
    }
}
