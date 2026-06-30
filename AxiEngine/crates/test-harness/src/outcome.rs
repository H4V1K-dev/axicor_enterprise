//! Definition of test execution outcomes and diagnostic mismatch error types.

use compute_api::{BackendKind, ComputeApiError};

/// The outcome of running a conformance or differential test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessOutcome {
    /// The test scenario passed successfully.
    Passed,
    /// The test scenario failed with a specific mismatch or backend error.
    Failed(HarnessErrorKind),
    /// The target backend is optional and not available in the current environment.
    Skipped {
        /// The kind of backend that was skipped.
        backend: BackendKind,
        /// The reason why the backend execution was skipped.
        reason: String,
    },
}

/// The specific category of test failure encountered by the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessErrorKind {
    /// Mismatch in execution results between reference and target backend.
    ResultMismatch {
        /// The name of the fixture being executed.
        fixture_name: String,
        /// The tick index at which the mismatch occurred.
        tick: u64,
        /// The specific field of BatchResult that mismatched.
        field: &'static str,
        /// The expected value (from CpuBackend).
        expected: String,
        /// The actual value (from target backend).
        actual: String,
    },
    /// Mismatch in debug state snapshot buffers between reference and target backend.
    SnapshotMismatch {
        /// The name of the fixture being executed.
        fixture_name: String,
        /// The tick index at which the snapshot was taken.
        tick: u64,
        /// The plane name (e.g., "state_blob" or "axons_blob").
        plane: &'static str,
        /// The byte offset within the plane/blob where the mismatch occurred.
        offset: usize,
        /// The expected byte value.
        expected: u8,
        /// The actual byte value.
        actual: u8,
    },
    /// Mismatch in error mapping.
    ErrorMappingMismatch {
        /// The expected error.
        expected: ComputeApiError,
        /// The actual error.
        actual: ComputeApiError,
    },
    /// Mismatch in ABI size or alignment check.
    AbiMirrorMismatch {
        /// The name of the struct checked.
        struct_name: &'static str,
        /// Detailed mismatch reason.
        reason: &'static str,
    },
    /// The backend is unavailable.
    BackendUnavailable {
        /// The kind of backend.
        backend: BackendKind,
        /// Error description.
        reason: String,
    },
    /// The required feature flag is not compiled.
    FeatureNotCompiled {
        /// The name of the feature flag.
        feature: &'static str,
    },
    /// The backend returned an API error.
    BackendError(ComputeApiError),
    /// A generic or lifecycle orchestration facade error.
    FacadeError(String),
}
