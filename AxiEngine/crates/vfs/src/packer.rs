use crate::constants::{ARCHIVE_PAYLOAD_ALIGNMENT, AXIC_MAGIC};
use crate::error::VfsError;
use crate::path::validate_archive_path;
use std::collections::HashSet;

/// A single entry to be packed into the archive.
pub struct ArchiveEntry<'a> {
    /// The normalized archive path (e.g., "model.toml").
    pub path: &'a str,
    /// The raw file bytes.
    pub bytes: &'a [u8],
}

/// Helper function to perform checked align up.
fn checked_align_up(val: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    val.checked_add(mask).map(|v| v & !mask)
}

/// Packs a collection of entries into a single memory buffer.
///
/// The returned buffer is formatted as an `.axic` container.
///
/// # Errors
///
/// Returns a [`VfsError`] if any path is invalid, duplicate, or if offset overflow occurs.
pub fn pack_entries(entries: &[ArchiveEntry<'_>]) -> Result<Vec<u8>, VfsError> {
    let mut unique_paths = HashSet::with_capacity(entries.len());

    // 1. Validate entries
    for entry in entries {
        validate_archive_path(entry.path)?;
        if !unique_paths.insert(entry.path) {
            return Err(VfsError::DuplicatePath);
        }
    }

    // 2. Sort entries lexicographically by path
    let mut sorted_entries: Vec<&ArchiveEntry<'_>> = entries.iter().collect();
    sorted_entries.sort_by(|a, b| a.path.cmp(b.path));

    let file_count = sorted_entries.len();

    // Empty archive policy: return exactly 12 bytes header (no payload, no padding)
    if file_count == 0 {
        let mut output = Vec::with_capacity(12);
        output.extend_from_slice(AXIC_MAGIC);
        output.extend_from_slice(&1u32.to_le_bytes()); // version = 1
        output.extend_from_slice(&0u32.to_le_bytes()); // file_count = 0
        return Ok(output);
    }

    let file_count_u32 = u32::try_from(file_count).map_err(|_| VfsError::TocCountOverflow)?;

    // 3. Compute size of header + TOC
    let toc_size = (file_count as u64)
        .checked_mul(272)
        .ok_or(VfsError::TocCountOverflow)?;

    let header_and_toc_size = 12u64
        .checked_add(toc_size)
        .ok_or(VfsError::OffsetOverflow)?;

    let payload_start = checked_align_up(header_and_toc_size, ARCHIVE_PAYLOAD_ALIGNMENT)
        .ok_or(VfsError::OffsetOverflow)?;

    // 4. Compute file offsets and sizes
    let mut offsets = Vec::with_capacity(file_count);
    let mut current_offset = payload_start;

    for entry in &sorted_entries {
        let size = entry.bytes.len() as u64;
        offsets.push((current_offset, size));

        let next_offset = current_offset
            .checked_add(size)
            .ok_or(VfsError::OffsetOverflow)?;

        current_offset = checked_align_up(next_offset, ARCHIVE_PAYLOAD_ALIGNMENT)
            .ok_or(VfsError::OffsetOverflow)?;
    }

    // 5. Serialize into output buffer
    let buffer_size = {
        let last_idx = file_count - 1;
        let (last_offset, last_size) = offsets[last_idx];
        last_offset
            .checked_add(last_size)
            .ok_or(VfsError::OffsetOverflow)?
    };

    let buffer_size_usize = usize::try_from(buffer_size).map_err(|_| VfsError::OffsetOverflow)?;
    let mut output = Vec::with_capacity(buffer_size_usize);

    // Write Header
    output.extend_from_slice(AXIC_MAGIC);
    output.extend_from_slice(&1u32.to_le_bytes()); // version = 1
    output.extend_from_slice(&file_count_u32.to_le_bytes());

    // Write TOC
    for (i, entry) in sorted_entries.iter().enumerate() {
        let (offset, size) = offsets[i];

        // Write path (256 bytes)
        let mut path_buf = [0u8; 256];
        path_buf[..entry.path.len()].copy_from_slice(entry.path.as_bytes());
        output.extend_from_slice(&path_buf);

        // Write offset & size
        output.extend_from_slice(&offset.to_le_bytes());
        output.extend_from_slice(&size.to_le_bytes());
    }

    // Fill padding until payload starts
    let current_len = output.len() as u64;
    if current_len < payload_start {
        let padding_needed =
            usize::try_from(payload_start - current_len).map_err(|_| VfsError::OffsetOverflow)?;
        output.resize(output.len() + padding_needed, 0x00);
    }

    // Write file payloads with padding
    for (i, entry) in sorted_entries.iter().enumerate() {
        let (offset, _size) = offsets[i];

        // Ensure current buffer len matches calculated file offset
        if (output.len() as u64) != offset {
            return Err(VfsError::AlignmentViolation);
        }

        output.extend_from_slice(entry.bytes);

        // Pad to next boundary
        let current_len = output.len() as u64;
        let next_boundary = checked_align_up(current_len, ARCHIVE_PAYLOAD_ALIGNMENT)
            .ok_or(VfsError::OffsetOverflow)?;

        if current_len < next_boundary {
            let padding_needed = usize::try_from(next_boundary - current_len)
                .map_err(|_| VfsError::OffsetOverflow)?;
            output.resize(output.len() + padding_needed, 0x00);
        }
    }

    Ok(output)
}
