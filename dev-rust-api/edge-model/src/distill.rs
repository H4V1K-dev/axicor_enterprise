//! WTA Distillation and Memory Split operations.

use crate::error::EdgeError;
use crate::{EdgeConfig, EdgeModel};

/// WTA Distillation
///
/// # WTA Distillation Invariants
/// - **INV-EDGE-001**: Parameterized synapse limit (WTA Max Synapses).
/// - **INV-EDGE-003**: Preservation of biological sign (Dale's Law Integrity).
fn wta_distill(dendrites: &[(u32, i32)], target_slots: usize) -> Vec<(u32, i32)> {
    let mut active: Vec<(u32, i32)> = dendrites
        .iter()
        .filter(|&&(t, _)| t != types::EMPTY_PIXEL)
        .cloned()
        .collect();

    // Sort by absolute value of weight in descending order
    // Using unsigned_abs() to prevent overflow of i32::MIN
    active.sort_by_key(|&(_, w)| std::cmp::Reverse(w.unsigned_abs()));

    active.truncate(target_slots);

    while active.len() < target_slots {
        active.push((types::EMPTY_PIXEL, 0));
    }

    active
}

/// Flash Padding
///
/// # Flash Padding Invariant
/// - **INV-EDGE-002**: 64KB MMU alignment of Flash partition (Execute-In-Place Padding).
fn pad_flash_image(flash_data: &mut Vec<u8>) {
    let raw_size = flash_data.len();
    // Fast bitwise rounding up to nearest multiple of 65536
    let padded_size = (raw_size + 65535) & !65535;
    flash_data.resize(padded_size, 0);
}

/// Converts a simulation archive into an EdgeModel by applying WTA distillation,
/// memory splitting, and padding constraints.
///
/// # Invariants
/// - **INV-EDGE-004**: Physical mutability isolation (Dual-Memory Split Isolation).
pub fn convert_archive(
    archive: &vfs::AxicArchive,
    config: &EdgeConfig,
) -> Result<EdgeModel, EdgeError> {
    let target_slots = config.target_dendrite_slots;
    if target_slots == 0 || target_slots > 128 {
        return Err(EdgeError::InvalidDendriteLimit(target_slots));
    }

    let state_bytes = archive
        .get_file("shard.state")
        .map_err(|_| EdgeError::InvalidSourceArchive)?;
    let axons_bytes = archive
        .get_file("shard.axons")
        .map_err(|_| EdgeError::InvalidSourceArchive)?;

    if state_bytes.len() < 16 {
        return Err(EdgeError::InvalidSourceArchive);
    }

    let state_header: &layout::StateFileHeader = bytemuck::from_bytes(&state_bytes[0..16]);
    let padded_n = state_header.padded_n as usize;
    let total_axons = state_header.total_axons as usize;

    if padded_n == 0 {
        return Err(EdgeError::EmptyArchive);
    }

    if padded_n % 32 != 0 {
        return Err(EdgeError::InvalidSourceArchive);
    }

    let offsets = layout::compute_state_offsets(padded_n);
    if state_bytes.len() < 64 + offsets.total_size {
        return Err(EdgeError::InvalidSourceArchive);
    }

    if axons_bytes.len() < 32 + total_axons * 32 {
        return Err(EdgeError::InvalidSourceArchive);
    }

    let payload = &state_bytes[64..];
    let voltage: &[i32] = bytemuck::cast_slice(&payload[offsets.soma_voltage .. offsets.soma_voltage + padded_n * 4]);
    let flags: &[u8] = &payload[offsets.flags .. offsets.flags + padded_n];
    let threshold_offset: &[i32] = bytemuck::cast_slice(&payload[offsets.threshold_offset .. offsets.threshold_offset + padded_n * 4]);
    let timers: &[u8] = &payload[offsets.timers .. offsets.timers + padded_n];
    let soma_to_axon: &[u32] = bytemuck::cast_slice(&payload[offsets.soma_to_axon .. offsets.soma_to_axon + padded_n * 4]);

    let dendrite_targets: &[u32] = bytemuck::cast_slice(&payload[offsets.dendrite_targets .. offsets.dendrite_targets + padded_n * 128 * 4]);
    let dendrite_weights: &[i32] = bytemuck::cast_slice(&payload[offsets.dendrite_weights .. offsets.dendrite_weights + padded_n * 128 * 4]);
    let dendrite_timers: &[u8] = &payload[offsets.dendrite_timers .. offsets.dendrite_timers + padded_n * 128];

    let axon_heads = &axons_bytes[32..32 + total_axons * 32];

    let mut new_dendrite_targets = vec![types::EMPTY_PIXEL; target_slots * padded_n];
    let mut new_dendrite_weights = vec![0i32; target_slots * padded_n];
    let mut new_dendrite_timers = vec![0u8; target_slots * padded_n];

    for n in 0..padded_n {
        let mut neuron_dendrites = Vec::with_capacity(128);
        for s in 0..128 {
            let idx = s * padded_n + n;
            neuron_dendrites.push((dendrite_targets[idx], dendrite_weights[idx]));
        }

        let distilled = wta_distill(&neuron_dendrites, target_slots);
        for (i, &(t, w)) in distilled.iter().enumerate() {
            let new_idx = i * padded_n + n;
            new_dendrite_targets[new_idx] = t;
            new_dendrite_weights[new_idx] = w;

            if t == types::EMPTY_PIXEL {
                new_dendrite_timers[new_idx] = 0;
            } else {
                let mut found_timer = 0;
                for s in 0..128 {
                    let old_idx = s * padded_n + n;
                    if dendrite_targets[old_idx] == t {
                        found_timer = dendrite_timers[old_idx];
                        break;
                    }
                }
                new_dendrite_timers[new_idx] = found_timer;
            }
        }
    }

    let mut sram_blob = Vec::new();
    sram_blob.extend_from_slice(bytemuck::cast_slice(voltage));
    sram_blob.extend_from_slice(flags);
    sram_blob.extend_from_slice(bytemuck::cast_slice(threshold_offset));
    sram_blob.extend_from_slice(timers);
    sram_blob.extend_from_slice(bytemuck::cast_slice(&new_dendrite_timers));
    sram_blob.extend_from_slice(axon_heads);

    let mut flash_blob = Vec::new();
    flash_blob.extend_from_slice(bytemuck::cast_slice(soma_to_axon));
    flash_blob.extend_from_slice(bytemuck::cast_slice(&new_dendrite_targets));
    flash_blob.extend_from_slice(bytemuck::cast_slice(&new_dendrite_weights));
    flash_blob.extend_from_slice(&[0u8; 1024]); // variant_params placeholder

    pad_flash_image(&mut flash_blob);

    Ok(EdgeModel {
        sram_blob,
        flash_blob,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_mock_archive(
        padded_n: u32,
        total_axons: u32,
        weights: Option<Vec<i32>>,
    ) -> (tempfile::TempDir, vfs::AxicArchive) {
        let temp_dir = tempdir().unwrap();
        let state_path = temp_dir.path().join("shard.state");
        let axons_path = temp_dir.path().join("shard.axons");

        let pn = padded_n as usize;
        let ta = total_axons as usize;

        let voltage = vec![0i32; pn];
        let flags = vec![0u8; pn];
        let threshold_offset = vec![0i32; pn];
        let timers = vec![0u8; pn];
        let soma_to_axon = vec![0u32; pn];
        
        let mut dendrite_targets = vec![0u32; pn * 128];
        for s in 0..128 {
            for n in 0..pn {
                let idx = s * pn + n;
                dendrite_targets[idx] = types::PackedTarget::pack(s as u32, 0).0;
            }
        }

        let dendrite_weights = weights.unwrap_or_else(|| vec![0i32; pn * 128]);
        let dendrite_timers = vec![0u8; pn * 128];

        let state_data = baker::serialization::serialize_state(
            padded_n,
            total_axons,
            &voltage,
            &flags,
            &threshold_offset,
            &timers,
            &soma_to_axon,
            &dendrite_targets,
            &dendrite_weights,
            &dendrite_timers,
        ).unwrap();
        std::fs::write(&state_path, state_data).unwrap();

        let heads = vec![layout::BurstHeads8 { h0: 0, h1: 0, h2: 0, h3: 0, h4: 0, h5: 0, h6: 0, h7: 0 }; ta];
        let axons_data = baker::serialization::serialize_axons(total_axons, &heads).unwrap();
        std::fs::write(&axons_path, axons_data).unwrap();

        let archive_path = temp_dir.path().join("archive.axic");
        vfs::pack_directory(temp_dir.path(), &archive_path).unwrap();

        let archive = vfs::AxicArchive::open(&archive_path).unwrap();

        (temp_dir, archive)
    }

    #[test]
    fn test_wta_distillation_repacks_correctly() {
        let ep = types::EMPTY_PIXEL;
        let input = vec![
            (10, 5),
            (20, -15),
            (ep, 100),
            (30, 2),
            (40, -8),
        ];
        
        let distilled = wta_distill(&input, 3);
        assert_eq!(distilled.len(), 3);
        assert_eq!(distilled[0], (20, -15));
        assert_eq!(distilled[1], (40, -8));
        assert_eq!(distilled[2], (10, 5));

        let padded = wta_distill(&input, 5);
        assert_eq!(padded.len(), 5);
        assert_eq!(padded[3], (30, 2));
        assert_eq!(padded[4], (ep, 0));
    }

    #[test]
    fn test_64kb_mmu_alignment() {
        let mut data = vec![1u8; 100];
        pad_flash_image(&mut data);
        assert_eq!(data.len(), 65536);
        assert_eq!(data[99], 1);
        assert_eq!(data[100], 0);

        let mut data2 = vec![1u8; 65536];
        pad_flash_image(&mut data2);
        assert_eq!(data2.len(), 65536);
    }

    #[test]
    fn test_sram_flash_partition_split() {
        let (_dir, archive) = create_mock_archive(32, 10, None);
        let config = EdgeConfig { target_dendrite_slots: 16 };
        let model = convert_archive(&archive, &config).unwrap();

        // SRAM size = 10 * N + K * N + 32 * A
        // = 10 * 32 + 16 * 32 + 32 * 10
        // = 320 + 512 + 320 = 1152 bytes
        assert_eq!(model.sram_blob.len(), 1152);

        // Flash size raw = 4 * N + 8 * K * N + 1024
        // = 4 * 32 + 8 * 16 * 32 + 1024
        // = 128 + 4096 + 1024 = 5248 bytes
        // Padded to 64KB = 65536 bytes
        assert_eq!(model.flash_blob.len(), 65536);
    }

    #[test]
    fn test_doa_and_sign_integrity() {
        let pn = 32;
        let mut weights = vec![0i32; pn * 128];
        
        weights[0 * pn + 0] = 5;
        weights[1 * pn + 0] = -15;
        weights[2 * pn + 0] = 10;

        let (_dir, archive) = create_mock_archive(pn as u32, 5, Some(weights));
        let config = EdgeConfig { target_dendrite_slots: 2 };
        let model = convert_archive(&archive, &config).unwrap();

        let weights_start = 128 + 256; // soma_to_axon (128) + targets (256)
        let new_weights: &[i32] = bytemuck::cast_slice(&model.flash_blob[weights_start .. weights_start + 2 * 32 * 4]);

        assert_eq!(new_weights[0 * pn + 0], -15);
        assert_eq!(new_weights[1 * pn + 0], 10);
    }

    #[test]
    fn test_invalid_dendrite_limit() {
        let (_dir, archive) = create_mock_archive(32, 1, None);
        let config_too_large = EdgeConfig { target_dendrite_slots: 129 };
        let res = convert_archive(&archive, &config_too_large);
        assert!(matches!(res, Err(EdgeError::InvalidDendriteLimit(129))));

        let config_zero = EdgeConfig { target_dendrite_slots: 0 };
        let res2 = convert_archive(&archive, &config_zero);
        assert!(matches!(res2, Err(EdgeError::InvalidDendriteLimit(0))));
    }

    #[test]
    fn test_empty_archive_returns_error() {
        let temp_dir = tempdir().unwrap();
        let state_path = temp_dir.path().join("shard.state");
        let axons_path = temp_dir.path().join("shard.axons");

        let header = layout::StateFileHeader {
            magic: *b"GSNS",
            version: 1,
            padded_n: 0,
            total_axons: 0,
        };
        std::fs::write(&state_path, bytemuck::bytes_of(&header)).unwrap();
        std::fs::write(&axons_path, &[0u8; 32]).unwrap();

        let archive_path = temp_dir.path().join("archive.axic");
        vfs::pack_directory(temp_dir.path(), &archive_path).unwrap();
        let archive = vfs::AxicArchive::open(&archive_path).unwrap();

        let config = EdgeConfig { target_dendrite_slots: 16 };
        let res = convert_archive(&archive, &config);
        assert!(matches!(res, Err(EdgeError::EmptyArchive)));
    }
}
