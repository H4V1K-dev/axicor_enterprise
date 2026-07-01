//! Boot pipeline errors definitions.

/// Errors that can occur during the execution of the Stage A boot pipeline.
#[derive(Debug, thiserror::Error)]
pub enum BootError {
    /// Errors propagated from the virtual filesystem.
    #[error("VFS error: {0}")]
    Vfs(#[from] vfs::VfsError),

    /// Validation failures returned by the compute-api logic.
    #[error("Compute API validation error: {0:?}")]
    ComputeApi(compute_api::ComputeApiError),

    /// Failures in engine bootstrap orchestration.
    #[error("Compute engine bootstrap error: {0}")]
    Compute(#[from] compute::ComputeError),

    /// A required logical file was not resolved in the archive.
    #[error("Missing required file in archive: {path}")]
    MissingRequiredFile {
        /// The logical path of the missing resource.
        path: &'static str,
    },

    /// The header contents or structural size mismatch of a validated blob.
    #[error("Invalid artifact header/content for {path}: {reason}")]
    InvalidArtifact {
        /// Logical path of the corrupted file.
        path: &'static str,
        /// Detail of validation rule failure.
        reason: &'static str,
    },

    /// The variant parameters LUT file size is mismatching.
    #[error("Variant parameters table size mismatch (expected {expected}, found {actual})")]
    VariantTableSizeMismatch {
        /// Expected size in bytes.
        expected: usize,
        /// Actual parsed size of logical entry.
        actual: usize,
    },
}

impl From<compute_api::ComputeApiError> for BootError {
    fn from(err: compute_api::ComputeApiError) -> Self {
        BootError::ComputeApi(err)
    }
}
