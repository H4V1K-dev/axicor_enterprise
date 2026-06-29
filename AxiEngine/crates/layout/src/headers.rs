//! C-ABI binary file header contracts for simulation state dumps, axon bursts, and path traces.

use crate::constants::{
    AXONS_FILE_VERSION, AXONS_MAGIC, PATHS_FILE_VERSION, PATHS_MAGIC, STATE_FILE_VERSION,
    STATE_MAGIC,
};
use bytemuck::{Pod, Zeroable};

/// Binary header structure for `.state` simulation dump files (16 bytes, 16-byte aligned).
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct StateFileHeader {
    /// Four-byte file type magic identifier (`AXST`).
    pub magic: [u8; 4],
    /// Binary format version number.
    pub version: u32,
    /// Aligned count of soma neurons (`padded_n`).
    pub padded_n: u32,
    /// Total count of axons allocated in the shard.
    pub total_axons: u32,
}

impl StateFileHeader {
    /// Creates a new `StateFileHeader` with standard magic and format version.
    #[inline(always)]
    pub const fn new(padded_n: u32, total_axons: u32) -> Self {
        Self {
            magic: STATE_MAGIC,
            version: STATE_FILE_VERSION,
            padded_n,
            total_axons,
        }
    }
}

/// Binary header structure for `.axons` spike propagation buffer files (16 bytes, 16-byte aligned).
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct AxonsFileHeader {
    /// Four-byte file type magic identifier (`AXAX`).
    pub magic: [u8; 4],
    /// Binary format version number.
    pub version: u32,
    /// Total count of active axons in the buffer.
    pub total_axons: u32,
    /// Explicit padding bytes to reach 16-byte alignment boundary.
    pub _padding: u32,
}

impl AxonsFileHeader {
    /// Creates a new `AxonsFileHeader` with standard magic and format version.
    #[inline(always)]
    pub const fn new(total_axons: u32) -> Self {
        Self {
            magic: AXONS_MAGIC,
            version: AXONS_FILE_VERSION,
            total_axons,
            _padding: 0,
        }
    }
}

/// Binary header structure for `.paths` axon 3D geometry trace files (16 bytes, 16-byte aligned).
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct PathsFileHeader {
    /// Four-byte file type magic identifier (`AXPT`).
    pub magic: [u8; 4],
    /// Binary format version number.
    pub version: u32,
    /// Total count of traced axon paths.
    pub total_axons: u32,
    /// Maximum segment capacity per axon (256).
    pub max_segments: u32,
}

impl PathsFileHeader {
    /// Creates a new `PathsFileHeader` with standard magic and format version.
    #[inline(always)]
    pub const fn new(total_axons: u32, max_segments: u32) -> Self {
        Self {
            magic: PATHS_MAGIC,
            version: PATHS_FILE_VERSION,
            total_axons,
            max_segments,
        }
    }
}
