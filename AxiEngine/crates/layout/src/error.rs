//! Error types for layout checks and structure validations.

/// Error conditions during memory layout validation or pointer offsets verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    /// Memory alignment requirement violated.
    AlignmentViolation,
    /// Buffer size does not match expected layout formula.
    SizeMismatch {
        /// The size expected by the formula.
        expected: usize,
        /// The actual buffer size provided.
        actual: usize,
    },
    /// The layout parameters are invalid (e.g. invalid shape).
    InvalidShape,
}
