use std::fmt;

/// Errors returned by the edge-model component.
#[derive(Debug)]
pub enum EdgeError {
    /// The simulation archive is empty.
    EmptyArchive,
    /// The archive has invalid layout or is missing required files.
    InvalidSourceArchive,
    /// The requested budget of dendrite slots is out of range.
    InvalidDendriteLimit(usize),
    /// Flash alignment up to 64KB MMU page boundary failed.
    MmuAlignmentFailed,
    /// Standard I/O error occurred.
    IoError(std::io::Error),
}

impl fmt::Display for EdgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeError::EmptyArchive => write!(f, "Source simulation archive is empty"),
            EdgeError::InvalidSourceArchive => write!(f, "Invalid source simulation archive structure"),
            EdgeError::InvalidDendriteLimit(k) => write!(f, "Invalid target dendrite slots budget K={}, must be 1..=128", k),
            EdgeError::MmuAlignmentFailed => write!(f, "Failed to pad Flash image to 64KB MMU page boundary"),
            EdgeError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for EdgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EdgeError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EdgeError {
    fn from(err: std::io::Error) -> Self {
        EdgeError::IoError(err)
    }
}
