//! Orchestrator library for Night Phase spatial growth and synaptogenesis tasks.
//!
//! This crate implements the 10-phase pipeline to apply modifications to somatic/dendritic
//! Structure of Arrays (SoA) memory slices on the host or in shared memory.

use std::time::Instant;

// Re-exports of control DTOs from `ipc` to avoid circular dependencies
pub use ipc::{WeaverGrowthContext, WeaverJobRequest, WeaverReport};

/// Direct target sources for simulation state modifications.
pub enum NightBufferSource<'a> {
    /// Linux/Windows Operating System shared memory segment.
    ShmAttachment {
        /// Platform-independent segment formatting name.
        shm_name: String,
    },
    /// Direct in-process mutable views over RAM blocks.
    HostSlices(layout::NightWorkingViewMut<'a>),
}

/// Specialized internal errors for daemon workflow orchestration.
#[derive(Debug, thiserror::Error)]
pub enum WeaverError {
    /// An error occurred during IPC coordination or transition.
    #[error("IPC transaction failed: {0}")]
    Ipc(#[from] ipc::IpcError),

    /// Validation of byte slices or structure offsets failed.
    #[error("Memory layout validation failed: {0:?}")]
    Layout(layout::LayoutError),

    /// A generic pipeline validation logic mismatch.
    #[error("Pipeline validation error: {0}")]
    Validation(String),

    /// An error occurred during topological calculations.
    #[error("Topology calculation failed: {0}")]
    Topology(String),
}

/// Orchestrates the 10-phase biological update loop for Night Phase growth.
///
/// Returns the generated report along with a list of cross-shard handover events.
pub fn run_night_pipeline(
    req: &WeaverJobRequest,
    _context: Option<&WeaverGrowthContext>,
    source: &mut NightBufferSource<'_>,
) -> Result<(WeaverReport, Vec<wire::AxonHandoverEvent>), WeaverError> {
    tracing::info!("Starting T006 Night Phase pipeline skeleton");
    let start_time = Instant::now();

    // Phase 1: Attach
    tracing::info!("Phase 1: Attach target buffer");
    let mut shm_opt = match source {
        NightBufferSource::ShmAttachment { shm_name: _ } => {
            let segment = ipc::ShmSegment::attach(req.zone_hash)?;
            Some(segment)
        }
        NightBufferSource::HostSlices(_) => None,
    };

    // Helper block to isolate mutable borrow of the target view and handle errors cleanly
    let execute_result =
        (|| -> Result<(u32, u32, u32, Vec<wire::AxonHandoverEvent>), WeaverError> {
            // Phase 2: Validate & Phase 3: AcquireNightState
            tracing::info!("Phase 2: Validate structure parameters & Phase 3: Acquire state");
            if let Some(ref mut segment) = shm_opt {
                let len = segment.len() as u64;
                ipc::validate_header(segment.header(), len).map_err(WeaverError::Ipc)?;
                segment
                    .try_transition(ipc::NightState::NightStart, ipc::NightState::Sprouting)
                    .map_err(WeaverError::Ipc)?;
            } else if let NightBufferSource::HostSlices(ref mut view) = source {
                layout::offsets::validate_night_working_view(
                    view.state_blob.len(),
                    view.axons_blob.len(),
                    view.paths_blob.as_ref().map(|p| p.len()),
                    view.padded_n,
                    view.total_axons,
                )
                .map_err(WeaverError::Layout)?;
            }

            // Obtain unified mutable view
            let view = match shm_opt {
                Some(ref mut segment) => segment.as_working_view_mut(),
                None => match source {
                    NightBufferSource::HostSlices(ref mut view) => layout::NightWorkingViewMut {
                        padded_n: view.padded_n,
                        total_axons: view.total_axons,
                        total_ghosts: view.total_ghosts,
                        state_blob: &mut *view.state_blob,
                        axons_blob: &mut *view.axons_blob,
                        paths_blob: view.paths_blob.as_deref_mut(),
                        offsets: view.offsets,
                    },
                    _ => unreachable!(),
                },
            };

            // Phase 4: ActivityScan
            tracing::info!("Phase 4: Scanning somatic flags and activity parameters");
            let padded_n = view.padded_n as usize;

            // Phase 5: SpatialRebuild
            tracing::info!("Phase 5: Rebuilding spatial geometry indexes");
            // TODO T007: replace with spatial index initialization when T007 lands

            // Phase 6: Prune
            tracing::info!("Phase 6: Executing synapse pruning plans");
            let off_targets = view.offsets.off_targets;
            let off_weights = view.offsets.off_weights;
            let off_dtimers = view.offsets.off_dtimers;

            let state_bytes = view.state_blob;
            let (_, rest) = state_bytes.split_at_mut(off_targets);
            let (targets_bytes, rest) = rest.split_at_mut(off_weights - off_targets);
            let (weights_bytes, timers_bytes) = rest.split_at_mut(off_dtimers - off_weights);

            let targets_slice = bytemuck::cast_slice_mut::<u8, u32>(
                &mut targets_bytes[..layout::MAX_DENDRITES * padded_n * 4],
            );
            let weights_slice = bytemuck::cast_slice_mut::<u8, i32>(
                &mut weights_bytes[..layout::MAX_DENDRITES * padded_n * 4],
            );
            let timers_slice = &mut timers_bytes[..layout::MAX_DENDRITES * padded_n];

            let prune_limit = req.prune_threshold as i32;
            let mut pruned_count = 0;

            for d in 0..layout::MAX_DENDRITES {
                for i in 0..padded_n {
                    let idx = d * padded_n + i;
                    let target = targets_slice[idx];
                    if target != types::EMPTY_PIXEL {
                        let w = weights_slice[idx];
                        if w < prune_limit {
                            targets_slice[idx] = types::EMPTY_PIXEL;
                            weights_slice[idx] = 0;
                            timers_slice[idx] = 0;
                            pruned_count += 1;
                        }
                    }
                }
            }

            // Phase 7: Sprout
            tracing::info!("Phase 7: Executing synapse sprouting plans");
            // Find empty slots and sprout synapses up to max_sprouts limit
            let mut sprouted_count = 0;
            'sprout_loop: for d in 0..layout::MAX_DENDRITES {
                for i in 0..padded_n {
                    if sprouted_count >= req.max_sprouts {
                        break 'sprout_loop;
                    }
                    let idx = d * padded_n + i;
                    if targets_slice[idx] == types::EMPTY_PIXEL {
                        // Generate a stub target: axon 10, segment 1
                        let target_val = 10u32 | (1u32 << 24);
                        targets_slice[idx] = target_val;
                        weights_slice[idx] = req.initial_synapse_weight;
                        timers_slice[idx] = 0;
                        sprouted_count += 1;
                    }
                }
            }

            // Phase 8: Compact
            tracing::info!("Phase 8: Executing synapse compaction plans");
            // Compact columns in-place, pushing EMPTY_PIXEL slots to the tail of each soma column
            let mut compacted_count = 0;
            for i in 0..padded_n {
                let mut active_slots = std::vec::Vec::new();
                for d in 0..layout::MAX_DENDRITES {
                    let idx = d * padded_n + i;
                    if targets_slice[idx] != types::EMPTY_PIXEL {
                        active_slots.push((
                            targets_slice[idx],
                            weights_slice[idx],
                            timers_slice[idx],
                        ));
                    }
                }

                for d in 0..layout::MAX_DENDRITES {
                    let idx = d * padded_n + i;
                    if let Some(&(t, w, tm)) = active_slots.get(d) {
                        if targets_slice[idx] != t {
                            compacted_count += 1;
                        }
                        targets_slice[idx] = t;
                        weights_slice[idx] = w;
                        timers_slice[idx] = tm;
                    } else {
                        targets_slice[idx] = types::EMPTY_PIXEL;
                        weights_slice[idx] = 0;
                        timers_slice[idx] = 0;
                    }
                }
            }

            // Phase 9: GhostHandover
            tracing::info!("Phase 9: Collecting cross-shard ghost handovers");
            // Generate an empty handover events list for skeleton test compliance
            let handovers = std::vec::Vec::new();

            Ok((pruned_count, compacted_count, sprouted_count, handovers))
        })();

    match execute_result {
        Ok((pruned, compacted, sprouted, handovers)) => {
            let duration_us = start_time.elapsed().as_micros() as u64;

            // Phase 10: Commit
            tracing::info!("Phase 10: Commit changes and complete Night Phase pipeline");
            if let Some(ref mut segment) = shm_opt {
                segment.try_transition(ipc::NightState::Sprouting, ipc::NightState::NightDone)?;
            }

            let report = WeaverReport {
                shard_id: req.shard_id,
                night_epoch: req.night_epoch,
                pruned_count: pruned,
                compacted_count: compacted,
                sprouted_count: sprouted,
                ghost_handovers_count: handovers.len() as u32,
                duration_us,
            };

            Ok((report, handovers))
        }
        Err(e) => {
            tracing::error!("Error occurred during pipeline execution. Poisoning segment.");
            if let Some(ref mut segment) = shm_opt {
                segment.force_error();
            }
            Err(e)
        }
    }
}
