//! Baker error types — §9.1

use std::fmt;

/// Errors that can occur during the .axic bake pipeline.
#[derive(Debug)]
pub enum BakerError {
    /// Required configuration file was not found at the specified path.
    ConfigNotFound(std::path::PathBuf),

    /// Sum of layer `height_pct` fields deviates from 1.0 beyond the 1e-4 tolerance.
    /// Violates INV-BAKER-001 (Anatomy Integrity Guard).
    InvalidLayerHeights {
        /// The actual computed sum.
        actual_sum: f32,
    },

    /// Sum of `share` fields in a layer's `composition` deviates from 1.0 beyond 1e-4.
    /// Violates INV-BAKER-001 (Anatomy Integrity Guard).
    InvalidComposition {
        /// Name of the offending layer.
        layer_name: String,
        /// The actual computed sum.
        actual_sum: f32,
    },

    /// Signal speed parameters violate the Integer Physics constraint (INV-CONFIG-003).
    /// Violates INV-BAKER-004 (Pre-Bake Guard).
    InvalidSignalSpeed(String),

    /// I/O error during configuration loading or archive writing.
    IOError(std::io::Error),
}

impl fmt::Display for BakerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigNotFound(path) => {
                write!(f, "Baker: config not found at '{}'", path.display())
            }
            Self::InvalidLayerHeights { actual_sum } => {
                write!(
                    f,
                    "Baker INV-BAKER-001: layer height_pct sum must be 1.0, got {}",
                    actual_sum
                )
            }
            Self::InvalidComposition { layer_name, actual_sum } => {
                write!(
                    f,
                    "Baker INV-BAKER-001: layer '{}' composition share sum must be 1.0, got {}",
                    layer_name, actual_sum
                )
            }
            Self::InvalidSignalSpeed(msg) => {
                write!(f, "Baker INV-BAKER-004: invalid signal speed — {}", msg)
            }
            Self::IOError(err) => {
                write!(f, "Baker I/O error: {}", err)
            }
        }
    }
}

impl std::error::Error for BakerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IOError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for BakerError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}
