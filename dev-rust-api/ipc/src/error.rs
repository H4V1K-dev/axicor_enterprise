//! Error types for inter-process communication.

use std::fmt;

/// Custom error type representing all possible failures during IPC operations.
#[derive(Debug)]
pub enum IpcError {
    /// System I/O error during file or socket access.
    Io(std::io::Error),
    /// Invalid header magic signature in shared memory.
    InvalidHeaderMagic,
    /// Timeout during Lock-Free status updates or spin waiting.
    Timeout,
    /// Atomic state conflict on state transition.
    StateConflict,
    /// Address or endpoint is already in use by another process.
    AddrInUse,
    /// The connection was closed or reset by the remote party.
    ConnectionReset,
    /// Zero-Copy replication of state databases failed.
    ReplicationFailed,
    /// Invalid or corrupted protocol packet format.
    InvalidProtocolPacket,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IPC I/O error: {}", err),
            Self::InvalidHeaderMagic => write!(f, "IPC invalid header magic signature"),
            Self::Timeout => write!(f, "IPC operation timeout"),
            Self::StateConflict => write!(f, "IPC state transition conflict"),
            Self::AddrInUse => write!(f, "IPC address is already in use"),
            Self::ConnectionReset => write!(f, "IPC connection reset by remote party"),
            Self::ReplicationFailed => write!(f, "IPC replication failed"),
            Self::InvalidProtocolPacket => write!(f, "IPC invalid protocol packet"),
        }
    }
}

impl std::error::Error for IpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for IpcError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
