//! Error handling types for the Virtual File System.

/// Custom error type representing all possible failures during VFS operations.
#[derive(Debug)]
pub enum VfsError {
    /// System I/O error during file access.
    IoError(std::io::Error),

    /// OS memory-mapping failure.
    MmapFailed(std::io::Error),

    /// Invalid archive magic signature header.
    InvalidMagic {
        /// The expected magic bytes.
        expected: [u8; 4],
        /// The actual magic bytes found in the file.
        actual: [u8; 4],
    },

    /// Unsupported version of the `.axic` format.
    InvalidVersion(u32),

    /// Target packaging path is not a directory.
    NotADirectory(std::path::PathBuf),

    /// TOC descriptor specifies a memory segment extending beyond file boundaries.
    OutOfBounds {
        /// Offset from start of file in bytes.
        offset: usize,
        /// Size of payload segment in bytes.
        size: usize,
        /// Total size of the archive file in bytes.
        archive_size: usize,
    },

    /// Target file offset is not OS page aligned (4096 bytes).
    AlignmentViolation {
        /// Logical path of the file.
        path: String,
        /// Specified offset value in bytes.
        offset: usize,
    },

    /// Relative file path length exceeds the 256-byte constraint.
    PathTooLong(String),

    /// Requested file was not found in the archive's TOC.
    FileNotFound(String),

    /// Name decoding fails due to non-UTF-8 path content in TOC.
    Utf8Error(std::str::Utf8Error),

    /// Path collision: duplicate file path found in the TOC table.
    DuplicatePath(String),

    /// Address overlap violation between two packaged files in the archive.
    OverlapViolation {
        /// Path of the first file.
        path_a: String,
        /// Path of the second file.
        path_b: String,
        /// Offset of the first file in bytes.
        offset_a: usize,
        /// Size of the first file in bytes.
        size_a: usize,
        /// Offset of the second file in bytes.
        offset_b: usize,
        /// Size of the second file in bytes.
        size_b: usize,
    },
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "VFS I/O error: {}", err),
            Self::MmapFailed(err) => write!(f, "VFS mmap failed: {}", err),
            Self::InvalidMagic { expected, actual } => {
                write!(
                    f,
                    "VFS invalid magic: expected {:?}, got {:?}",
                    expected, actual
                )
            }
            Self::InvalidVersion(version) => write!(f, "VFS invalid version: {}", version),
            Self::NotADirectory(path) => write!(f, "VFS path is not a directory: {:?}", path),
            Self::OutOfBounds {
                offset,
                size,
                archive_size,
            } => {
                write!(
                    f,
                    "VFS out of bounds: range [{}..{}] exceeds archive size {}",
                    offset,
                    offset + size,
                    archive_size
                )
            }
            Self::AlignmentViolation { path, offset } => {
                write!(
                    f,
                    "VFS alignment violation: file '{}' at offset {} is not OS page aligned",
                    path, offset
                )
            }
            Self::PathTooLong(path) => {
                write!(f, "VFS path too long (max 256 bytes): '{}'", path)
            }
            Self::FileNotFound(path) => write!(f, "VFS file not found: '{}'", path),
            Self::Utf8Error(err) => write!(f, "VFS UTF-8 path decoding error: {}", err),
            Self::DuplicatePath(path) => {
                write!(f, "VFS duplicate path in TOC: '{}'", path)
            }
            Self::OverlapViolation {
                path_a,
                path_b,
                offset_a,
                size_a,
                offset_b,
                size_b,
            } => {
                write!(
                    f,
                    "VFS overlap violation: file '{}' [{}..{}] overlaps with file '{}' [{}..{}]",
                    path_a,
                    offset_a,
                    offset_a + size_a,
                    path_b,
                    offset_b,
                    offset_b + size_b
                )
            }
        }
    }
}

impl std::error::Error for VfsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            Self::MmapFailed(err) => Some(err),
            Self::Utf8Error(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for VfsError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<std::str::Utf8Error> for VfsError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}
