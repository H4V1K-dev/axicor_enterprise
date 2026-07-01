//! Runtime error representations.

use crate::dto::RuntimeState;

/// Error type returned by the runtime coordination logic.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Failure propagated from the underlying compute facade.
    #[error("Compute execution error: {0}")]
    Compute(#[from] compute::ComputeError),

    /// Transition or operation attempted from an invalid runtime lifecycle state.
    #[error("Invalid state transition from {from:?} (expected {expected})")]
    InvalidState {
        /// The current state of the runtime.
        from: RuntimeState,
        /// The expected state/condition description.
        expected: &'static str,
    },

    /// Initialized with an engine not in the Running state.
    #[error("Invalid engine lifecycle state: expected Running, found {actual:?}")]
    InvalidEngineLifecycle {
        /// The current lifecycle state of the compute engine.
        actual: compute::LifecycleState,
    },

    /// The requested day batch ticks execution would cause tick counter overflow.
    #[error("Biological tick overflow: current={current}, sync={sync}")]
    TickOverflow {
        /// Current absolute tick index.
        current: u64,
        /// Requested batch tick step.
        sync: u32,
    },

    /// Arithmetic overflow or capacity limit exceeded in batch configuration parameters.
    #[error("Capacity limit exceeded or overflow: {reason}")]
    CapacityExceeded {
        /// Reason description.
        reason: &'static str,
    },

    /// Mismatched array or slice sizes supplied in batch inputs.
    #[error("Invalid input buffer dimensions for {field}: expected {expected}, found {actual}")]
    InvalidInputDimensions {
        /// Field name trigger.
        field: &'static str,
        /// The expected bounds size.
        expected: usize,
        /// The actual buffer slice size provided.
        actual: usize,
    },
}
