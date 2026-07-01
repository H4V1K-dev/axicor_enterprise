//! Error types for the configuration parsing and validation.

use thiserror::Error;

/// Central error type for all configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Errors occurring during parsing of TOML files.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Errors occurring during semantic and physical validation of configurations.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Errors signaling that an unsupported feature configuration was used.
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// Errors originating from file I/O.
    #[error("I/O error: {0}")]
    IoError(String),
}
