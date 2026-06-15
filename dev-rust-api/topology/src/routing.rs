//! Ghost Atlas Routing — maps 2D GXO soma pixels to target-shard soma IDs.
//!
//! # Invariants
//! - **E-079 (Hardware Empty Pixel)**: If no soma is found in a Z-column,
//!   `target_ghost_id` MUST be set to `types::EMPTY_PIXEL` (0xFFFF_FFFF).
//!   Failing to do so causes the GPU to waste ALU cycles on dead pixels.
//! - **INV-BAKER-003 (BTreeMap Determinism)**: The soma index MUST use
//!   `BTreeMap<u32, usize>` (ordered, deterministic) instead of `HashMap`.
//!   `HashMap`'s non-deterministic hasher breaks Bit-to-Bit Identity of
//!   the generated `.axic` archive.

use std::collections::BTreeMap;

use config::EntryZ;
use types::PackedPosition;
use wire::GhostConnection;

use crate::error::TopologyError;

/// Builds a deterministic O(log N) index mapping packed position keys to soma indices.
///
/// # INV-BAKER-003 (BTreeMap Determinism)
/// Uses `BTreeMap` (not `HashMap`) to guarantee a stable, insertion-order-independent
/// traversal for bit-to-bit reproducible `.axic` archive generation.
fn build_soma_index(soma_positions: &[PackedPosition]) -> BTreeMap<u32, usize> {
    let mut index = BTreeMap::new();
    for (soma_idx, pos) in soma_positions.iter().enumerate() {
        index.insert(pos.0, soma_idx);
    }
    index
}

/// Routes a 2D GXO source matrix to target-shard soma IDs via Z-column descent.
///
/// # Invariants
/// - **E-079 (Hardware Empty Pixel)**: Any pixel with no matching soma in its Z-column
///   receives `target_ghost_id = types::EMPTY_PIXEL` (0xFFFF_FFFF) to signal an
///   immediate early-exit in GPU I/O compute kernels.
/// - **INV-BAKER-003 (BTreeMap Determinism)**: The internal soma index is built with
///   `BTreeMap` to ensure deterministic traversal order across all platforms, preserving
///   bit-to-bit identity of the generated `.axic` archive.
///
/// # Parameters
/// - `source_gxo_somas`: Flat row-major 2D array of source soma IDs (len = `width * height`).
/// - `width`, `height`: Dimensions of the source 2D matrix.
/// - `target_bounds`: `(max_x, max_y, max_z)` voxel extents of the target shard.
/// - `entry_z`: Vertical entry direction (`Top` → descend Z-max..0, `Bottom`/`Mid` → ascend 0..Z-max).
/// - `target_type_idx`: Neuron type index to match in the target shard.
/// - `soma_positions`: Packed position array of all somas in the target shard.
pub fn route_ghost_atlas(
    source_gxo_somas: &[u32],
    width: u32,
    height: u32,
    target_bounds: (u32, u32, u32),
    entry_z: EntryZ,
    target_type_idx: u8,
    soma_positions: &[PackedPosition],
) -> Result<Vec<GhostConnection>, TopologyError> {
    let index = build_soma_index(soma_positions);
    let (max_x, max_y, max_z) = target_bounds;

    let mut connections = Vec::with_capacity((width * height) as usize);

    for py in 0..height {
        for px in 0..width {
            // Normalize pixel coordinates to [0.0, 1.0)
            let u = px as f32 / width as f32;
            let v = py as f32 / height as f32;

            // Project onto target shard voxel grid
            let target_x = (u * max_x as f32) as u32;
            let target_y = (v * max_y as f32) as u32;

            // Z-column descent: search for the first matching soma
            // E-079: fall back to EMPTY_PIXEL if none found
            let found_soma_id = match entry_z {
                EntryZ::Top => {
                    // Descend from max_z - 1 down to 0 (top-down axon entry)
                    let mut found = None;
                    for z in (0..max_z).rev() {
                        let key = PackedPosition::pack_raw(target_x, target_y, z, target_type_idx).0;
                        if let Some(&soma_idx) = index.get(&key) {
                            found = Some(soma_idx as u32);
                            break;
                        }
                    }
                    found.unwrap_or(types::EMPTY_PIXEL)
                }
                EntryZ::Bottom | EntryZ::Mid => {
                    // Ascend from 0 up to max_z - 1 (bottom-up or mid axon entry)
                    let mut found = None;
                    for z in 0..max_z {
                        let key = PackedPosition::pack_raw(target_x, target_y, z, target_type_idx).0;
                        if let Some(&soma_idx) = index.get(&key) {
                            found = Some(soma_idx as u32);
                            break;
                        }
                    }
                    found.unwrap_or(types::EMPTY_PIXEL)
                }
            };

            connections.push(GhostConnection {
                src_soma_id: source_gxo_somas[(py * width + px) as usize],
                target_ghost_id: found_soma_id,
            });
        }
    }

    Ok(connections)
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::PackedPosition;

    /// Builds a minimal soma_positions array with one soma at (x=2, y=3, z=5, type=1)
    fn make_somas(x: u32, y: u32, z: u32, type_id: u8) -> Vec<PackedPosition> {
        vec![PackedPosition::pack_raw(x, y, z, type_id)]
    }

    #[test]
    fn test_empty_pixel_when_no_soma_found() {
        // Grid 1x1, soma does not match target_type_idx=2 (soma is type 1)
        let somas = make_somas(5, 5, 10, 1);
        let source = vec![42u32];
        let result = route_ghost_atlas(
            &source,
            1,
            1,
            (10, 10, 20),
            EntryZ::Bottom,
            2, // type mismatch → no match
            &somas,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        // E-079: must write EMPTY_PIXEL when no soma found
        assert_eq!(result[0].target_ghost_id, types::EMPTY_PIXEL);
        assert_eq!(result[0].src_soma_id, 42);
    }

    #[test]
    fn test_bottom_finds_lowest_z() {
        // Two somas at same (x, y) but different z; Bottom should find z=1 first
        let somas = vec![
            PackedPosition::pack_raw(0, 0, 1, 0),
            PackedPosition::pack_raw(0, 0, 5, 0),
        ];
        let source = vec![99u32];
        let result = route_ghost_atlas(
            &source,
            1,
            1,
            (10, 10, 10),
            EntryZ::Bottom,
            0,
            &somas,
        )
        .unwrap();

        // Bottom ascends from z=0; first hit at z=1 → soma_idx=0
        assert_eq!(result[0].target_ghost_id, 0);
    }

    #[test]
    fn test_top_finds_highest_z() {
        // Two somas at same (x, y) but different z; Top should find z=5 first (descending)
        let somas = vec![
            PackedPosition::pack_raw(0, 0, 1, 0),
            PackedPosition::pack_raw(0, 0, 5, 0),
        ];
        let source = vec![77u32];
        let result = route_ghost_atlas(
            &source,
            1,
            1,
            (10, 10, 10),
            EntryZ::Top,
            0,
            &somas,
        )
        .unwrap();

        // Top descends from z=9; first hit at z=5 → soma_idx=1
        assert_eq!(result[0].target_ghost_id, 1);
    }

    #[test]
    fn test_output_length_matches_grid() {
        let somas = make_somas(0, 0, 0, 0);
        let source: Vec<u32> = (0..9).collect();
        let result = route_ghost_atlas(
            &source,
            3,
            3,
            (10, 10, 10),
            EntryZ::Bottom,
            0,
            &somas,
        )
        .unwrap();
        assert_eq!(result.len(), 9);
    }

    #[test]
    fn test_btreemap_determinism() {
        // INV-BAKER-003: same input must always produce the same output
        let somas = vec![
            PackedPosition::pack_raw(3, 3, 2, 1),
            PackedPosition::pack_raw(3, 3, 8, 1),
        ];
        let source = vec![1u32];
        let r1 = route_ghost_atlas(&source, 1, 1, (10, 10, 10), EntryZ::Bottom, 1, &somas).unwrap();
        let r2 = route_ghost_atlas(&source, 1, 1, (10, 10, 10), EntryZ::Bottom, 1, &somas).unwrap();
        assert_eq!(r1, r2);
    }
}
