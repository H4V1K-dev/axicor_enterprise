//! Validation helpers for compute backend DTO inspection and verification.

use crate::dto::{
    BackendMaintenanceMut, BackendMaintenanceRef, DayBatchCmd, ShardAllocSpec, ShardSnapshotMut,
    ShardUpload,
};
use crate::error::ComputeApiError;

/// Computes the expected binary blob physical size for an `.axons` file given the axon count.
///
/// Delegates to layout crate for size formula.
/// Overflow returns `Err(ComputeApiError::InvalidShape)`.
#[inline]
pub fn expected_axons_blob_size(total_axons: u32) -> Result<usize, ComputeApiError> {
    layout::calculate_axons_blob_size(total_axons).ok_or(ComputeApiError::InvalidShape)
}

/// Validates simulation shard allocation specification parameters.
#[inline]
#[allow(clippy::manual_is_multiple_of)]
pub fn validate_alloc_spec(spec: &ShardAllocSpec) -> Result<(), ComputeApiError> {
    if spec.padded_n == 0 {
        return Err(ComputeApiError::InvalidShape);
    }
    if (spec.padded_n as usize) % layout::PADDED_N_ALIGNMENT != 0 {
        return Err(ComputeApiError::AlignmentViolation);
    }
    Ok(())
}

/// Validates binary upload blob physical buffer sizes against allocation specifications.
#[inline]
pub fn validate_upload(
    spec: &ShardAllocSpec,
    upload: &ShardUpload<'_>,
) -> Result<(), ComputeApiError> {
    validate_alloc_spec(spec)?;
    let expected_state = layout::calculate_state_blob_size(spec.padded_n as usize);
    if upload.state_blob.len() != expected_state {
        return Err(ComputeApiError::SizeMismatch);
    }
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if upload.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::SizeMismatch);
    }
    Ok(())
}

/// Validates maintenance export buffer sizes against allocation specifications.
#[inline]
pub fn validate_maintenance_export(
    spec: &ShardAllocSpec,
    maintenance: &BackendMaintenanceMut<'_>,
) -> Result<(), ComputeApiError> {
    validate_alloc_spec(spec)?;
    let expected_state = layout::calculate_state_blob_size(spec.padded_n as usize);
    if maintenance.state_blob.len() != expected_state {
        return Err(ComputeApiError::SizeMismatch);
    }
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if maintenance.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::SizeMismatch);
    }
    Ok(())
}

/// Validates maintenance import buffer sizes against allocation specifications.
#[inline]
pub fn validate_maintenance_import(
    spec: &ShardAllocSpec,
    maintenance: &BackendMaintenanceRef<'_>,
) -> Result<(), ComputeApiError> {
    validate_alloc_spec(spec)?;
    let expected_state = layout::calculate_state_blob_size(spec.padded_n as usize);
    if maintenance.state_blob.len() != expected_state {
        return Err(ComputeApiError::SizeMismatch);
    }
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if maintenance.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::SizeMismatch);
    }
    Ok(())
}

/// Validates day batch execution command payloads and slice lengths.
pub fn validate_day_batch_cmd(cmd: &DayBatchCmd<'_>) -> Result<(), ComputeApiError> {
    if cmd.sync_batch_ticks == 0 {
        return Err(ComputeApiError::InvalidBatch);
    }
    if cmd
        .tick_base
        .checked_add(cmd.sync_batch_ticks as u64 - 1)
        .is_none()
    {
        return Err(ComputeApiError::InvalidBatch);
    }
    if cmd.v_seg == 0 || cmd.v_seg > 255 {
        return Err(ComputeApiError::InvalidBatch);
    }
    let batch_ticks = cmd.sync_batch_ticks as usize;
    if cmd.incoming_spike_counts.len() != batch_ticks {
        return Err(ComputeApiError::InvalidBatch);
    }
    if cmd.output_spike_counts.len() != batch_ticks {
        return Err(ComputeApiError::InvalidBatch);
    }

    let max_spikes = cmd.max_spikes_per_tick as usize;
    for &count in cmd.incoming_spike_counts {
        if count > cmd.max_spikes_per_tick {
            return Err(ComputeApiError::CapacityExceeded);
        }
    }

    let min_spike_buf_len = batch_ticks
        .checked_mul(max_spikes)
        .ok_or(ComputeApiError::CapacityExceeded)?;

    if let Some(spikes) = cmd.incoming_spikes {
        if spikes.len() < min_spike_buf_len {
            return Err(ComputeApiError::CapacityExceeded);
        }
    } else {
        for &count in cmd.incoming_spike_counts {
            if count != 0 {
                return Err(ComputeApiError::InvalidBatch);
            }
        }
    }

    if cmd.output_spikes.len() < min_spike_buf_len {
        return Err(ComputeApiError::CapacityExceeded);
    }

    if let Some(mask) = cmd.input_bitmask {
        if cmd.num_virtual_axons > 0 {
            let required_words = (cmd.num_virtual_axons as usize).div_ceil(32);
            if (cmd.input_words_per_tick as usize) < required_words {
                return Err(ComputeApiError::InvalidBatch);
            }
        }

        let min_mask_len = (cmd.input_words_per_tick as usize)
            .checked_mul(batch_ticks)
            .ok_or(ComputeApiError::CapacityExceeded)?;
        if mask.len() < min_mask_len {
            return Err(ComputeApiError::InvalidBatch);
        }
    }

    if cmd.mapped_soma_ids.len() != cmd.num_outputs as usize {
        return Err(ComputeApiError::InvalidBatch);
    }

    Ok(())
}

/// Validates diagnostic snapshot mutable target buffers against expected shard sizes.
#[inline]
pub fn validate_snapshot_buffers(
    spec: &ShardAllocSpec,
    snapshot: &ShardSnapshotMut<'_>,
) -> Result<(), ComputeApiError> {
    validate_alloc_spec(spec)?;
    let expected_state = layout::calculate_state_blob_size(spec.padded_n as usize);
    if snapshot.state_blob.len() != expected_state {
        return Err(ComputeApiError::InvalidDebugProbeBounds);
    }
    let expected_axons = expected_axons_blob_size(spec.total_axons)?;
    if snapshot.axons_blob.len() != expected_axons {
        return Err(ComputeApiError::InvalidDebugProbeBounds);
    }
    Ok(())
}
