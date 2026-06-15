//! Memory offsets and size calculations for high-performance SoA layout.

/// Stored offsets for SoA (Structure of Arrays) state buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateOffsets {
    pub soma_voltage: usize,
    pub flags: usize,
    pub threshold_offset: usize,
    pub timers: usize,
    pub soma_to_axon: usize,
    pub dendrite_targets: usize,
    pub dendrite_weights: usize,
    pub dendrite_timers: usize,
    pub total_size: usize,
}

/// Aligns a value upwards to a 64-byte boundary.
///
/// Under Zero Warp Divergence and L2 cache line optimization rules,
/// this is implemented branchless using bitwise arithmetic.
#[inline]
pub const fn align_offset(offset: usize) -> usize {
    (offset + 63) & !63
}

/// Align neuron count to nearest multiple of 64 threads (hardware warp / wavefront boundary).
#[inline]
pub const fn align_to_warp(n: usize) -> usize {
    (n + 63) & !63
}

/// Computes the flat columnar array index for SoA access.
///
/// flat_idx = slot * padded_n + neuron_idx.
#[inline]
pub const fn columnar_idx(padded_n: usize, neuron_idx: usize, slot: usize) -> usize {
    if neuron_idx >= padded_n {
        panic!("columnar_idx: neuron_idx >= padded_n");
    }
    if slot >= 128 {
        panic!("columnar_idx: slot >= 128");
    }
    slot * padded_n + neuron_idx
}

/// Computes all array starting offsets within the state file / memory buffer.
#[allow(clippy::identity_op)]
pub const fn compute_state_offsets(padded_n: usize) -> StateOffsets {
    if padded_n % 32 != 0 {
        panic!("compute_state_offsets: padded_n must be a multiple of 32");
    }

    let mut current = 0;

    let soma_voltage = current;
    current = align_offset(current + padded_n * 4); // i32

    let flags = current;
    current = align_offset(current + padded_n * 1); // u8

    let threshold_offset = current;
    current = align_offset(current + padded_n * 4); // i32

    let timers = current;
    current = align_offset(current + padded_n * 1); // u8

    let soma_to_axon = current;
    current = align_offset(current + padded_n * 4); // u32

    let dendrite_targets = current;
    current = align_offset(current + padded_n * 128 * 4); // u32 (PackedTarget)

    let dendrite_weights = current;
    current = align_offset(current + padded_n * 128 * 4); // i32 (Mass Domain)

    let dendrite_timers = current;
    current = align_offset(current + padded_n * 128 * 1); // u8

    StateOffsets {
        soma_voltage,
        flags,
        threshold_offset,
        timers,
        soma_to_axon,
        dendrite_targets,
        dendrite_weights,
        dendrite_timers,
        total_size: current,
    }
}

/// Returns aligned neuron count and total state file payload size in bytes.
///
/// Returns `(0, 0)` if `neuron_count` is 0.
#[inline]
pub const fn calculate_state_blob_size(neuron_count: usize) -> (usize, usize) {
    if neuron_count == 0 {
        (0, 0)
    } else {
        let padded_n = align_to_warp(neuron_count);
        let offsets = compute_state_offsets(padded_n);
        (padded_n, 64 + offsets.total_size) // 16 bytes for StateFileHeader + 48 bytes padding for 64-byte alignment
    }
}

/// Returns the byte offset of the paths matrix in a `.paths` file.
#[inline]
pub const fn calculate_paths_matrix_offset(total_axons: usize) -> usize {
    if total_axons == 0 {
        16 // Only the header size
    } else {
        let header_and_lengths = 16 + total_axons;
        (header_and_lengths + 63) & !63
    }
}

/// Returns total size of `.paths` file in bytes.
#[inline]
pub const fn calculate_paths_file_size(total_axons: usize) -> usize {
    if total_axons == 0 {
        16
    } else {
        let matrix_offset = calculate_paths_matrix_offset(total_axons);
        matrix_offset + total_axons * 256 * 4 // total_axons * MAX_SEGMENTS * 4B PackedPosition
    }
}

/// Computes the total shared memory size for IPC, aligned to OS page size (4096 bytes).
pub const fn shm_size(padded_n: usize) -> usize {
    if padded_n == 0 {
        return 0;
    }
    if padded_n % 32 != 0 {
        panic!("shm_size: padded_n must be a multiple of 32");
    }

    let weights_bytes = padded_n * 128 * 4;
    let targets_bytes = padded_n * 128 * 4;
    let handovers_bytes = 10000 * 20; // 10000 elements * 20B
    let prunes_bytes = 10000 * 12;    // 10000 elements * 12B
    let flags_bytes = (padded_n + 63) & !63;
    let voltage_bytes = padded_n * 4;
    let threshold_bytes = padded_n * 4;
    let timers_bytes = (padded_n + 63) & !63;

    let total_bytes = 128 // ShmHeader size
        + weights_bytes
        + targets_bytes
        + handovers_bytes
        + prunes_bytes
        + flags_bytes
        + voltage_bytes
        + threshold_bytes
        + timers_bytes;

    (total_bytes + 4095) & !4095
}
