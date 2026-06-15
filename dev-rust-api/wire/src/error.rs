//! Error types for the wire serialization protocol.

/// Errors that can occur during zero-copy serialization/deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Invalid binary identifier (magic number).
    InvalidMagic { expected: [u8; 4], actual: [u8; 4] },
    /// Buffer size is insufficient for the requested type cast.
    BufferTooSmall { expected: usize, actual: usize },
    /// Alignment of the source buffer does not match the target type's alignment requirements.
    AlignmentMismatch,
    /// Format version mismatch.
    VersionMismatch { expected: u32, actual: u32 },
    /// Semantic validation error (e.g. invalid offset, incorrect payload size).
    ValidationError(&'static str),
}
