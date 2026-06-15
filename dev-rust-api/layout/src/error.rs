//! Layout boundary and FFI verification errors.

/// Error enum representing data layout and memory boundary violations.
///
/// Under zero-allocation rules, no dynamic heap strings or memory allocation is allowed
/// within Layer 1. `IOError` carries a static string reference (`&'static str`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CAbiBoundaryError {
    /// Invalid size or zero dimensions for padded_n / total_axons.
    InvalidSize,
    /// Mismatch between expected and actual file/blob sizes.
    SizeMismatch {
        expected: usize,
        actual: usize,
    },
    /// Alignment validation failure in memory addresses.
    AlignmentViolation {
        expected_align: usize,
        actual_addr: usize,
    },
    /// File I/O or shared memory mapping failure.
    IOError(&'static str),
}
