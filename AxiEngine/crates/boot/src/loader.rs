//! Loader implementation for single-shard archive packages.

use bytemuck::Zeroable;
use layout::{
    calculate_paths_file_size, calculate_state_blob_size, AxonsFileHeader, PathsFileHeader,
    StateFileHeader, VariantParameters, MAX_SEGMENTS_PER_AXON, PADDED_N_ALIGNMENT, VARIANT_LUT_LEN,
};
use vfs::AxicArchive;

use crate::constants::{
    AXONS_ARCHIVE_PATH, PATHS_ARCHIVE_PATH, STATE_ARCHIVE_PATH, VARIANT_TABLE_ARCHIVE_PATH,
};
use crate::dto::{LocalShardBootBundle, LocalShardBootInput, LocalShardComputeInput};
use crate::error::BootError;

/// Opens the shard archive, reads and copies its contents to owned buffers,
/// performs alignment-safe ABI header validation, and performs compute validation.
///
/// # Errors
///
/// Returns a [`BootError`] if VFS, layout validations, or compute-api validations fail.
pub fn load_local_shard_archive(
    input: &LocalShardBootInput,
) -> Result<LocalShardBootBundle, BootError> {
    // 1. Open the archive
    let archive = AxicArchive::open(&input.archive_path)?;

    // 2. Fetch the required files. Extra files are ignored.
    let state_bytes =
        archive
            .get_file(STATE_ARCHIVE_PATH)
            .ok_or(BootError::MissingRequiredFile {
                path: STATE_ARCHIVE_PATH,
            })?;
    let axons_bytes =
        archive
            .get_file(AXONS_ARCHIVE_PATH)
            .ok_or(BootError::MissingRequiredFile {
                path: AXONS_ARCHIVE_PATH,
            })?;
    let paths_bytes =
        archive
            .get_file(PATHS_ARCHIVE_PATH)
            .ok_or(BootError::MissingRequiredFile {
                path: PATHS_ARCHIVE_PATH,
            })?;
    let variant_table_bytes =
        archive
            .get_file(VARIANT_TABLE_ARCHIVE_PATH)
            .ok_or(BootError::MissingRequiredFile {
                path: VARIANT_TABLE_ARCHIVE_PATH,
            })?;

    // 3. Make owned copies of the blobs
    let state_blob = state_bytes.to_vec();
    let axons_blob = axons_bytes.to_vec();
    let paths_blob = paths_bytes.to_vec();

    // 4. Alignment-safe parsing and verification of state.bin
    if state_blob.len() < std::mem::size_of::<StateFileHeader>() {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "File is too short for StateFileHeader",
        });
    }
    let state_header: StateFileHeader =
        bytemuck::pod_read_unaligned(&state_blob[0..std::mem::size_of::<StateFileHeader>()]);

    if state_header.magic != layout::STATE_MAGIC {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "Invalid state file magic",
        });
    }
    if state_header.version != layout::STATE_FILE_VERSION {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "Invalid state file format version",
        });
    }
    let padded_n = state_header.padded_n;
    if padded_n == 0 {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "padded_n must be non-zero",
        });
    }
    if !(padded_n as usize).is_multiple_of(PADDED_N_ALIGNMENT) {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "padded_n must be aligned",
        });
    }
    let expected_state_size = calculate_state_blob_size(padded_n as usize);
    if state_blob.len() != expected_state_size {
        return Err(BootError::InvalidArtifact {
            path: STATE_ARCHIVE_PATH,
            reason: "State blob size mismatch",
        });
    }

    // 5. Alignment-safe parsing and verification of axons.bin
    if axons_blob.len() < std::mem::size_of::<AxonsFileHeader>() {
        return Err(BootError::InvalidArtifact {
            path: AXONS_ARCHIVE_PATH,
            reason: "File is too short for AxonsFileHeader",
        });
    }
    let axons_header: AxonsFileHeader =
        bytemuck::pod_read_unaligned(&axons_blob[0..std::mem::size_of::<AxonsFileHeader>()]);

    if axons_header.magic != layout::AXONS_MAGIC {
        return Err(BootError::InvalidArtifact {
            path: AXONS_ARCHIVE_PATH,
            reason: "Invalid axons file magic",
        });
    }
    if axons_header.version != layout::AXONS_FILE_VERSION {
        return Err(BootError::InvalidArtifact {
            path: AXONS_ARCHIVE_PATH,
            reason: "Invalid axons file format version",
        });
    }
    let total_axons = state_header.total_axons;
    if axons_header.total_axons != total_axons {
        return Err(BootError::InvalidArtifact {
            path: AXONS_ARCHIVE_PATH,
            reason: "Axons count mismatch with StateFileHeader",
        });
    }
    let expected_axons_size = compute_api::validation::expected_axons_blob_size(total_axons)?;
    if axons_blob.len() != expected_axons_size {
        return Err(BootError::InvalidArtifact {
            path: AXONS_ARCHIVE_PATH,
            reason: "Axons blob size mismatch",
        });
    }

    // 6. Alignment-safe parsing and verification of paths.bin
    if paths_blob.len() < std::mem::size_of::<PathsFileHeader>() {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "File is too short for PathsFileHeader",
        });
    }
    let paths_header: PathsFileHeader =
        bytemuck::pod_read_unaligned(&paths_blob[0..std::mem::size_of::<PathsFileHeader>()]);

    if paths_header.magic != layout::PATHS_MAGIC {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "Invalid paths file magic",
        });
    }
    if paths_header.version != layout::PATHS_FILE_VERSION {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "Invalid paths file format version",
        });
    }
    if paths_header.total_axons != total_axons {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "Paths axons count mismatch with StateFileHeader",
        });
    }
    if paths_header.max_segments != MAX_SEGMENTS_PER_AXON as u32 {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "Paths max_segments mismatch with MAX_SEGMENTS_PER_AXON",
        });
    }
    let expected_paths_size = calculate_paths_file_size(total_axons as usize);
    if paths_blob.len() != expected_paths_size {
        return Err(BootError::InvalidArtifact {
            path: PATHS_ARCHIVE_PATH,
            reason: "Paths blob size mismatch",
        });
    }

    // 7. Verification of variant_table.bin size and alignment-safe chunk copy
    let expected_vt_size = std::mem::size_of::<VariantParameters>() * VARIANT_LUT_LEN;
    if variant_table_bytes.len() != expected_vt_size {
        return Err(BootError::VariantTableSizeMismatch {
            expected: expected_vt_size,
            actual: variant_table_bytes.len(),
        });
    }

    let mut variant_table = [VariantParameters::zeroed(); VARIANT_LUT_LEN];
    let vp_size = std::mem::size_of::<VariantParameters>();
    for (i, item) in variant_table.iter_mut().enumerate() {
        let offset = i * vp_size;
        *item = bytemuck::pod_read_unaligned(&variant_table_bytes[offset..offset + vp_size]);
    }

    // 8. Build ShardAllocSpec
    let spec = compute_api::ShardAllocSpec {
        padded_n,
        total_axons,
        total_ghosts: input.total_ghosts,
        virtual_offset: input.virtual_offset,
    };

    let bundle = LocalShardBootBundle {
        spec,
        state_blob,
        axons_blob,
        paths_blob,
        variant_table,
    };

    // 9. Validate final upload via compute-api
    compute_api::validation::validate_upload(&bundle.spec, &bundle.upload())?;

    Ok(bundle)
}

/// Helper that loads the local shard archive and immediately bootstraps the thread-affine ShardEngine.
///
/// # Errors
///
/// Returns a [`BootError`] if loading or bootstrap initialization fails.
pub fn bootstrap_local_shard_engine(
    input: &LocalShardComputeInput,
) -> Result<(compute::ShardEngine, LocalShardBootBundle), BootError> {
    let boot_input = LocalShardBootInput {
        archive_path: input.archive_path.clone(),
        virtual_offset: input.virtual_offset,
        total_ghosts: input.total_ghosts,
    };

    let bundle = load_local_shard_archive(&boot_input)?;

    let engine = compute::ShardEngine::bootstrap(
        input.backend_preference.clone(),
        bundle.spec,
        bundle.upload(),
    )?;

    Ok((engine, bundle))
}
