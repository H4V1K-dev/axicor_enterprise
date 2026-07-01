use thiserror::Error;

/// Error type for the baker.
#[derive(Debug, Error)]
pub enum BakerError {
    /// Config validation errors.
    #[error("Configuration validation error: {0}")]
    ConfigError(#[from] config::ConfigError),

    /// Topology generation errors.
    #[error("Topology generation error: {0:?}")]
    TopologyError(topology::TopologyError),

    /// Layout alignment and sizing errors.
    #[error("Layout capacity/alignment error")]
    LayoutError,
}
