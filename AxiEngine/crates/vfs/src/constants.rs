/// The mandatory alignment boundary for payload data offsets within the archive.
pub const ARCHIVE_PAYLOAD_ALIGNMENT: u64 = 4096;

/// The file container signature sequence.
pub const AXIC_MAGIC: &[u8; 4] = b"AXIC";
