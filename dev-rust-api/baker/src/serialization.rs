//! C-ABI binary serializers for .state, .axons, and .paths dump files.

use crate::error::BakerError;

// ─────────────────────────────────────────────────────────────────────────────
// serialize_state
// ─────────────────────────────────────────────────────────────────────────────

/// Serializes SoA neuron state arrays into a `.state` binary blob.
///
/// # INV-LAYOUT-009 (64-byte Cache-Line Alignment)
/// The output buffer layout is:
/// - Bytes `0..16`: [`layout::StateFileHeader`].
/// - Bytes `16..64`: zero padding (48 bytes) to reach 64-byte alignment.
/// - Bytes `64 + offsets.*`: SoA arrays, each 64-byte aligned as computed by
///   [`layout::compute_state_offsets`].
///
/// The constant offset `64` is the source of truth established in
/// [`layout::calculate_state_blob_size`] (16 B header + 48 B padding).
///
/// # Errors
/// Returns [`BakerError::IOError`] (via [`std::io::Error::other`]) if any input
/// slice length does not match the expected `padded_n`-derived dimensions.
#[allow(clippy::too_many_arguments)]
pub fn serialize_state(
    padded_n: u32,
    total_axons: u32,
    voltage: &[i32],
    flags: &[u8],
    threshold_offset: &[i32],
    timers: &[u8],
    soma_to_axon: &[u32],
    dendrite_targets: &[u32],
    dendrite_weights: &[i32],
    dendrite_timers: &[u8],
) -> Result<Vec<u8>, BakerError> {
    let pn = padded_n as usize;
    let offsets = layout::compute_state_offsets(pn);
    // Data base = 64: 16 bytes StateFileHeader + 48 bytes padding (INV-LAYOUT-009).
    // Source of truth: layout::calculate_state_blob_size returns (padded_n, 64 + offsets.total_size).
    const DATA_BASE: usize = 64;
    let total_size = DATA_BASE + offsets.total_size;

    let mut buf = vec![0u8; total_size];

    // Write StateFileHeader at offset 0
    let header = layout::StateFileHeader {
        magic: *b"GSNS",
        version: 1,
        padded_n,
        total_axons,
    };
    buf[0..16].copy_from_slice(bytemuck::bytes_of(&header));
    // bytes 16..64 remain zero (48-byte alignment padding)

    // Copy each SoA array at 64 + offsets.* (all 64-byte aligned, INV-LAYOUT-009)
    copy_slice_i32(&mut buf, DATA_BASE + offsets.soma_voltage, voltage)?;
    copy_slice_u8(&mut buf, DATA_BASE + offsets.flags, flags)?;
    copy_slice_i32(&mut buf, DATA_BASE + offsets.threshold_offset, threshold_offset)?;
    copy_slice_u8(&mut buf, DATA_BASE + offsets.timers, timers)?;
    copy_slice_u32(&mut buf, DATA_BASE + offsets.soma_to_axon, soma_to_axon)?;
    copy_slice_u32(&mut buf, DATA_BASE + offsets.dendrite_targets, dendrite_targets)?;
    copy_slice_i32(&mut buf, DATA_BASE + offsets.dendrite_weights, dendrite_weights)?;
    copy_slice_u8(&mut buf, DATA_BASE + offsets.dendrite_timers, dendrite_timers)?;

    Ok(buf)
}

// ─────────────────────────────────────────────────────────────────────────────
// serialize_axons
// ─────────────────────────────────────────────────────────────────────────────

/// Serializes burst heads into a `.axons` binary blob.
///
/// # INV-LAYOUT-004 (BurstHeads8 32-byte Alignment)
/// [`layout::BurstHeads8`] has `align(32)` and size 32 bytes. The
/// [`layout::AxonsFileHeader`] occupies bytes 0..16, followed by 16 bytes of
/// explicit zero padding, so the first `BurstHeads8` element begins at byte
/// offset 32 — satisfying the hardware warp alignment requirement.
pub fn serialize_axons(
    total_axons: u32,
    heads: &[layout::BurstHeads8],
) -> Result<Vec<u8>, BakerError> {
    // Layout: [AxonsFileHeader 16B][padding 16B][heads × 32B]
    let buf_size = 32 + (total_axons as usize) * 32;
    let mut buf = vec![0u8; buf_size];

    // Write AxonsFileHeader at offset 0
    let header = layout::AxonsFileHeader {
        magic: *b"GSAX",
        version: 1,
        total_axons,
        _padding: 0,
    };
    buf[0..16].copy_from_slice(bytemuck::bytes_of(&header));
    // bytes 16..32 remain zero (explicit alignment padding for BurstHeads8)

    // Copy BurstHeads8 array starting at offset 32 (INV-LAYOUT-004)
    let heads_bytes: &[u8] = bytemuck::cast_slice(heads);
    let dst_end = 32 + heads_bytes.len();
    if dst_end > buf.len() {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_axons: heads slice exceeds allocated buffer",
        )));
    }
    buf[32..dst_end].copy_from_slice(heads_bytes);

    Ok(buf)
}

// ─────────────────────────────────────────────────────────────────────────────
// serialize_paths
// ─────────────────────────────────────────────────────────────────────────────

/// Serializes axon path geometry into a `.paths` binary blob.
///
/// Layout:
/// - Bytes `0..16`: [`layout::PathsFileHeader`]
/// - Bytes `16..matrix_offset`: `path_lengths` (u8 per axon), zero-padded to 64-byte boundary.
/// - Bytes `matrix_offset..end`: `matrix` (packed [`types::PackedPosition`] u32 values).
pub fn serialize_paths(
    total_axons: u32,
    path_lengths: &[u8],
    matrix: &[types::PackedPosition],
) -> Result<Vec<u8>, BakerError> {
    let ta = total_axons as usize;
    let matrix_offset = layout::calculate_paths_matrix_offset(ta);
    let buf_size = matrix_offset + matrix.len() * 4;
    let mut buf = vec![0u8; buf_size];

    // Write PathsFileHeader at offset 0
    let header = layout::PathsFileHeader {
        magic: layout::PATHS_MAGIC,
        version: 1,
        total_axons,
        max_segments: layout::MAX_SEGMENTS_PER_AXON as u32,
    };
    buf[0..16].copy_from_slice(bytemuck::bytes_of(&header));

    // Write path_lengths starting at offset 16
    let lengths_end = 16 + path_lengths.len();
    if lengths_end > matrix_offset {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_paths: path_lengths overruns matrix_offset",
        )));
    }
    buf[16..lengths_end].copy_from_slice(path_lengths);

    // Write matrix starting at matrix_offset
    let matrix_bytes: &[u8] = bytemuck::cast_slice(matrix);
    let matrix_end = matrix_offset + matrix_bytes.len();
    if matrix_end > buf.len() {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_paths: matrix overruns buffer",
        )));
    }
    buf[matrix_offset..matrix_end].copy_from_slice(matrix_bytes);

    Ok(buf)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn copy_slice_u8(buf: &mut [u8], offset: usize, src: &[u8]) -> Result<(), BakerError> {
    let end = offset + src.len();
    if end > buf.len() {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_state: u8 slice overruns buffer",
        )));
    }
    buf[offset..end].copy_from_slice(src);
    Ok(())
}

#[inline]
fn copy_slice_i32(buf: &mut [u8], offset: usize, src: &[i32]) -> Result<(), BakerError> {
    let bytes: &[u8] = bytemuck::cast_slice(src);
    let end = offset + bytes.len();
    if end > buf.len() {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_state: i32 slice overruns buffer",
        )));
    }
    buf[offset..end].copy_from_slice(bytes);
    Ok(())
}

#[inline]
fn copy_slice_u32(buf: &mut [u8], offset: usize, src: &[u32]) -> Result<(), BakerError> {
    let bytes: &[u8] = bytemuck::cast_slice(src);
    let end = offset + bytes.len();
    if end > buf.len() {
        return Err(BakerError::IOError(std::io::Error::other(
            "serialize_state: u32 slice overruns buffer",
        )));
    }
    buf[offset..end].copy_from_slice(bytes);
    Ok(())
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::PackedPosition;

    const PN: u32 = 64; // smallest valid padded_n (multiple of 32, aligned to 64 by align_to_warp)

    fn zero_state_slices() -> (
        Vec<i32>, Vec<u8>, Vec<i32>, Vec<u8>,
        Vec<u32>, Vec<u32>, Vec<i32>, Vec<u8>,
    ) {
        let n = PN as usize;
        (
            vec![0i32; n],                    // voltage
            vec![0u8; n],                     // flags
            vec![0i32; n],                    // threshold_offset
            vec![0u8; n],                     // timers
            vec![0u32; n],                    // soma_to_axon
            vec![0u32; n * 128],              // dendrite_targets
            vec![0i32; n * 128],              // dendrite_weights
            vec![0u8; n * 128],               // dendrite_timers
        )
    }

    #[test]
    fn test_serialize_state_header_magic() {
        let (v, fl, th, tm, s2a, dt, dw, dti) = zero_state_slices();
        let buf = serialize_state(PN, 0, &v, &fl, &th, &tm, &s2a, &dt, &dw, &dti).unwrap();
        assert_eq!(&buf[0..4], b"GSNS");
    }

    #[test]
    fn test_serialize_state_alignment() {
        // INV-LAYOUT-009: DATA_BASE = 64 (16B header + 48B padding).
        // offsets.* start from 0, so absolute buffer offsets = 64 + offsets.* — all 64-byte aligned.
        const DATA_BASE: usize = 64;
        let offsets = layout::compute_state_offsets(PN as usize);
        assert_eq!((DATA_BASE + offsets.soma_voltage) % 64, 0);
        assert_eq!((DATA_BASE + offsets.flags) % 64, 0);
        assert_eq!((DATA_BASE + offsets.threshold_offset) % 64, 0);
        assert_eq!((DATA_BASE + offsets.timers) % 64, 0);
        assert_eq!((DATA_BASE + offsets.soma_to_axon) % 64, 0);
        assert_eq!((DATA_BASE + offsets.dendrite_targets) % 64, 0);
        assert_eq!((DATA_BASE + offsets.dendrite_weights) % 64, 0);
        assert_eq!((DATA_BASE + offsets.dendrite_timers) % 64, 0);
    }

    #[test]
    fn test_serialize_axons_header_and_alignment() {
        let heads = vec![layout::BurstHeads8 {
            h0: 1, h1: 2, h2: 3, h3: 4, h4: 5, h5: 6, h6: 7, h7: 8,
        }];
        let buf = serialize_axons(1, &heads).unwrap();
        // Header magic at 0
        assert_eq!(&buf[0..4], b"GSAX");
        // INV-LAYOUT-004: heads start at offset 32
        assert_eq!(buf.len(), 64); // 32 (header+padding) + 1 * 32
        // h0 = 1 little-endian at offset 32
        let h0 = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        assert_eq!(h0, 1);
    }

    #[test]
    fn test_serialize_paths_header_and_matrix_offset() {
        let lengths = vec![10u8; 4];
        let matrix = vec![PackedPosition::pack_raw(1, 2, 3, 0); 4];
        let buf = serialize_paths(4, &lengths, &matrix).unwrap();
        // PathsFileHeader magic
        assert_eq!(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]), layout::PATHS_MAGIC);
        // matrix starts at 64-byte aligned offset
        let matrix_offset = layout::calculate_paths_matrix_offset(4);
        assert_eq!(matrix_offset % 64, 0);
        // first matrix element at matrix_offset
        let first = u32::from_le_bytes([
            buf[matrix_offset], buf[matrix_offset+1],
            buf[matrix_offset+2], buf[matrix_offset+3],
        ]);
        assert_eq!(first, PackedPosition::pack_raw(1, 2, 3, 0).0);
    }
}
