use thiserror::Error;

/// Error types returned by the Virtual File System operations.
#[derive(Debug, Error)]
pub enum VfsError {
    /// The file does not start with the mandatory 'AXIC' signature.
    #[error("Invalid magic signature")]
    InvalidMagic,

    /// The archive container version is not supported by this engine version.
    #[error("Unsupported binary format version")]
    UnsupportedVersion,

    /// The archive header size is smaller than the minimum 12 bytes.
    #[error("Archive header too small")]
    HeaderTooSmall,

    /// The TOC offset points outside of the archive container bounds.
    #[error("TOC offset out of bounds")]
    TocOutOfBounds,

    /// The number of files listed in the header is invalid or overflows index boundaries.
    #[error("TOC entries count overflow")]
    TocCountOverflow,

    /// An entry payload offset or size exceeds the archive container boundaries.
    #[error("TOC entry payload offset/size out of bounds")]
    EntryOutOfBounds,

    /// An integer overflow occurred while computing TOC size or entry alignments.
    #[error("Integer overflow calculating offsets")]
    OffsetOverflow,

    /// A payload offset is not aligned to the mandatory 4096-byte boundary.
    #[error("Payload offset or size alignment violation")]
    AlignmentViolation,

    /// The normalized path length exceeds the limit of 255 bytes.
    #[error("Path exceeds maximum length of 255 bytes")]
    PathTooLong,

    /// The path bytes do not represent a valid UTF-8 sequence.
    #[error("Path string is not valid UTF-8")]
    PathNotUtf8,

    /// The path entry inside the TOC is missing a nul terminator.
    #[error("Path missing nul terminator")]
    PathNotTerminated,

    /// The path violates the isolation namespace constraints.
    #[error("Path violates archive namespace rules")]
    InvalidPath,

    /// Multiple entries inside the archive share the same normalized path.
    #[error("Duplicate path in archive TOC")]
    DuplicatePath,

    /// The requested file does not exist in the archive.
    #[error("File not found in archive")]
    FileNotFound,

    /// An underlying standard OS I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to establish a memory map projection for the archive file.
    #[error("Failed to map file")]
    MapFailed,
}
