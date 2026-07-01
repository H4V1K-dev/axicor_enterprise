use crate::constants::{ARCHIVE_PAYLOAD_ALIGNMENT, AXIC_MAGIC};
use crate::error::VfsError;
use crate::path::validate_archive_path;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;

/// Represents a memory-mapped Read-Only `.axic` archive.
pub struct AxicArchive {
    mmap: Mmap,
    files: HashMap<String, FileEntry>,
    file_paths: Vec<String>,
}

struct FileEntry {
    offset: usize,
    size: usize,
}

/// Helper function to perform checked align up.
fn checked_align_up(val: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    val.checked_add(mask).map(|v| v & !mask)
}

impl AxicArchive {
    /// Opens an archive file from the host filesystem using a Read-Only memory map projection.
    ///
    /// # Errors
    ///
    /// Returns a [`VfsError`] if opening, mapping, or validating the archive fails.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, VfsError> {
        let file = File::open(path)?;

        // SAFETY: The memory map is created as Read-Only. We project the entire file container.
        // During the network night/day simulation process, the archive remains strictly immutable,
        // preventing any memory-access race conditions or modification hazards.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|_| VfsError::MapFailed)?;

        Self::parse(mmap)
    }

    fn parse(mmap: Mmap) -> Result<Self, VfsError> {
        let data = &mmap[..];

        if data.len() < 12 {
            return Err(VfsError::HeaderTooSmall);
        }

        let magic = &data[0..4];
        if magic != AXIC_MAGIC {
            return Err(VfsError::InvalidMagic);
        }

        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if version != 1 {
            return Err(VfsError::UnsupportedVersion);
        }

        let file_count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

        let toc_size = (file_count as u64)
            .checked_mul(272)
            .ok_or(VfsError::TocCountOverflow)?;

        let toc_end = 12u64
            .checked_add(toc_size)
            .ok_or(VfsError::OffsetOverflow)?;

        if (data.len() as u64) < toc_end {
            return Err(VfsError::TocOutOfBounds);
        }

        let payload_start = checked_align_up(toc_end, ARCHIVE_PAYLOAD_ALIGNMENT)
            .ok_or(VfsError::OffsetOverflow)? as usize;

        let mut files = HashMap::with_capacity(file_count);
        let mut file_paths = Vec::with_capacity(file_count);

        for i in 0..file_count {
            let entry_offset = 12 + i * 272;
            let path_bytes = &data[entry_offset..entry_offset + 256];

            // Find first nul terminator
            let nul_pos = path_bytes
                .iter()
                .position(|&b| b == 0)
                .ok_or(VfsError::PathNotTerminated)?;

            let path_str =
                std::str::from_utf8(&path_bytes[..nul_pos]).map_err(|_| VfsError::PathNotUtf8)?;

            validate_archive_path(path_str)?;

            let file_offset = u64::from_le_bytes(
                data[entry_offset + 256..entry_offset + 264]
                    .try_into()
                    .unwrap(),
            );
            let file_size = u64::from_le_bytes(
                data[entry_offset + 264..entry_offset + 272]
                    .try_into()
                    .unwrap(),
            );

            // Validations
            if file_offset < payload_start as u64 {
                return Err(VfsError::EntryOutOfBounds);
            }
            if file_offset % ARCHIVE_PAYLOAD_ALIGNMENT != 0 {
                return Err(VfsError::AlignmentViolation);
            }

            let file_end = file_offset
                .checked_add(file_size)
                .ok_or(VfsError::OffsetOverflow)?;

            if file_end > data.len() as u64 {
                return Err(VfsError::EntryOutOfBounds);
            }

            let path_string = path_str.to_string();
            if files.contains_key(&path_string) {
                return Err(VfsError::DuplicatePath);
            }

            files.insert(
                path_string.clone(),
                FileEntry {
                    offset: file_offset as usize,
                    size: file_size as usize,
                },
            );
            file_paths.push(path_string);
        }

        // Ensure the TOC is sorted lexicographically and has no duplicates (INV-VFS-006)
        for window in file_paths.windows(2) {
            if window[0] >= window[1] {
                return Err(VfsError::InvalidPath);
            }
        }

        Ok(Self {
            mmap,
            files,
            file_paths,
        })
    }

    /// Queries raw byte slices from the archive by its logical path.
    ///
    /// Returns `Some(&[u8])` if found, and `None` if not found or the path is invalid.
    pub fn get_file(&self, path: &str) -> Option<&[u8]> {
        if validate_archive_path(path).is_err() {
            return None;
        }
        let entry = self.files.get(path)?;
        Some(&self.mmap[entry.offset..entry.offset + entry.size])
    }

    /// Queries raw byte slices from the archive by its logical path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::FileNotFound`] or [`VfsError::InvalidPath`].
    pub fn require_file(&self, path: &str) -> Result<&[u8], VfsError> {
        validate_archive_path(path)?;
        let entry = self.files.get(path).ok_or(VfsError::FileNotFound)?;
        Ok(&self.mmap[entry.offset..entry.offset + entry.size])
    }

    /// Checks if the archive contains the file with the given logical path.
    pub fn contains(&self, path: &str) -> bool {
        if validate_archive_path(path).is_err() {
            return false;
        }
        self.files.contains_key(path)
    }

    /// Returns a lexicographically sorted iterator over all archive logical paths.
    pub fn list_files(&self) -> impl Iterator<Item = &str> {
        self.file_paths.iter().map(|s| s.as_str())
    }
}
