//! Unified error structures and conversions for the compute facade.

use crate::lifecycle::LifecycleState;
use compute_api::{BackendKind, ComputeApiError};
use std::fmt;

/// Unified error type for the compute facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeError {
    /// The requested backend is optional and not compiled.
    FeatureNotCompiled {
        /// The name of the Cargo feature.
        feature: &'static str,
    },
    /// The backend is compiled but the device/driver is unavailable.
    BackendUnavailable {
        /// The kind of backend.
        backend: BackendKind,
        /// Clear reason description.
        reason: String,
    },
    /// No backends in Auto preference chain were compiled or available.
    NoBackendAvailable,
    /// Invalid lifecycle state transition attempt.
    InvalidLifecycleState {
        /// Current state of the engine.
        current: LifecycleState,
        /// Expected state or action constraint.
        expected: &'static str,
    },
    /// Engine memory segment is poisoned, preventing exit from maintenance.
    ImportPoisoned,
    /// Direct API error returned by the underlying compute backend.
    ApiError(ComputeApiError),
}

impl From<ComputeApiError> for ComputeError {
    fn from(err: ComputeApiError) -> Self {
        Self::ApiError(err)
    }
}

impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeatureNotCompiled { feature } => {
                write!(f, "compute feature '{}' is not compiled", feature)
            }
            Self::BackendUnavailable { backend, reason } => {
                write!(f, "backend {:?} is unavailable: {}", backend, reason)
            }
            Self::NoBackendAvailable => write!(f, "no backend is available"),
            Self::InvalidLifecycleState { current, expected } => {
                write!(
                    f,
                    "invalid lifecycle state: current={:?}, expected={}",
                    current, expected
                )
            }
            Self::ImportPoisoned => {
                write!(
                    f,
                    "engine memory segment is poisoned, cannot exit maintenance"
                )
            }
            Self::ApiError(err) => write!(f, "compute API error: {:?}", err),
        }
    }
}

impl std::error::Error for ComputeError {}
