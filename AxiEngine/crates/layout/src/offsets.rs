//! Runtime DTO definitions and const layout formulas for SoA state blobs and path files.

use crate::constants::{MAX_DENDRITES, MAX_SEGMENTS_PER_AXON};

/// Runtime data transfer object containing physical byte offsets for SoA state planes in `.state` blobs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateOffsets {
    /// Byte offset of `soma_voltage` plane (`i32`).
    pub off_voltage: usize,
    /// Byte offset of `soma_flags` plane (`u8`).
    pub off_flags: usize,
    /// Byte offset of `threshold_offset` plane (`i32`).
    pub off_thresh: usize,
    /// Byte offset of `timers` plane (`u8`).
    pub off_timers: usize,
    /// Byte offset of `soma_to_axon` plane (`u32`).
    pub off_s2a: usize,
    /// Byte offset of `dendrite_targets` matrix plane (`PackedTarget` / `u32`).
    pub off_targets: usize,
    /// Byte offset of `dendrite_weights` matrix plane (`i32`).
    pub off_weights: usize,
    /// Byte offset of `dendrite_timers` matrix plane (`u8`).
    pub off_dtimers: usize,
    /// Total calculated physical byte size of the `.state` dump blob including per-plane alignment padding.
    pub total_state_size: usize,
}

/// Aligns a byte count `x` up to the nearest 64-byte (`CACHE_LINE_BYTES`) boundary.
#[inline(always)]
pub const fn align64(x: usize) -> usize {
    (x + 63) & !63
}

/// Aligns a neuron allocation count `n` up to the neutral `PADDED_N_ALIGNMENT` (64) boundary.
#[inline(always)]
pub const fn align_to_padded_n(n: usize) -> usize {
    align64(n)
}

/// Historical alias for `align_to_padded_n`. Aligns count to 64-byte boundary.
#[inline(always)]
pub const fn align_to_warp(n: usize) -> usize {
    align64(n)
}

/// Computes physical byte offsets for all 8 SoA planes within a `.state` dump blob for a given `padded_n`.
///
/// Each plane starts strictly on a 64-byte cache line boundary (Per-Plane 64B Alignment standard).
#[inline(always)]
pub const fn compute_state_offsets(padded_n: usize) -> StateOffsets {
    let header_size = 16;
    let off_voltage = align64(header_size);
    let off_flags = align64(off_voltage + padded_n * 4);
    let off_thresh = align64(off_flags + padded_n);
    let off_timers = align64(off_thresh + padded_n * 4);
    let off_s2a = align64(off_timers + padded_n);
    let off_targets = align64(off_s2a + padded_n * 4);
    let off_weights = align64(off_targets + MAX_DENDRITES * padded_n * 4);
    let off_dtimers = align64(off_weights + MAX_DENDRITES * padded_n * 4);
    let total_state_size = align64(off_dtimers + MAX_DENDRITES * padded_n);

    StateOffsets {
        off_voltage,
        off_flags,
        off_thresh,
        off_timers,
        off_s2a,
        off_targets,
        off_weights,
        off_dtimers,
        total_state_size,
    }
}

/// Calculates the total physical binary size of a `.state` dump blob for a given `padded_n`.
#[inline(always)]
pub const fn calculate_state_blob_size(padded_n: usize) -> usize {
    compute_state_offsets(padded_n).total_state_size
}

/// Calculates the byte offset where the 3D position coordinate matrix begins inside a `.paths` file.
///
/// `lengths` elements are 16-bit integers (`u16`).
#[inline(always)]
pub const fn calculate_paths_matrix_offset(total_axons: usize) -> usize {
    align64(16 + total_axons * 2)
}

/// Calculates the total physical binary size of a `.paths` trace file for a given `total_axons` count.
#[inline(always)]
pub const fn calculate_paths_file_size(total_axons: usize) -> usize {
    let matrix_offset = calculate_paths_matrix_offset(total_axons);
    matrix_offset + total_axons * MAX_SEGMENTS_PER_AXON * 4
}
