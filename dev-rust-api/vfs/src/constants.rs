//! Hardware and file layout constants for Axicor `.axic` archives.

/// Little-Endian magic signature value `"AXIC"` (0x43495841).
pub const AXIC_MAGIC: u32 = 0x43495841;

/// Size of physical page in OS (4096 bytes) for page-aligned structures.
pub const OS_PAGE_SIZE: usize = 4096;

/// Total size of a single Table of Contents (TOC) entry (272 bytes).
/// 256 bytes for path + 8 bytes offset + 8 bytes size.
pub const TOC_ENTRY_SIZE: usize = 272;

/// Standard archive header prefix size (12 bytes).
/// 4 bytes magic + 4 bytes version + 4 bytes file count.
pub const AXIC_HEADER_SIZE: usize = 12;
