//! Local synapse formation implementation (Stage B2).

use crate::dto::{FormedSynapse, LocalSynapsePlan, NeuronSynapseRow, SynapseFormationInput};
use crate::error::TopologyError;
use types::PackedTarget;

#[derive(Clone, Copy, Debug)]
struct SynapseCandidate {
    distance_sq: i64,
    segment_offset: u8,
    source_soma_id: u32,
    axon_id: u32,
    source_type_id: u8,
}

/// Deterministically builds the plan of local synaptic connections.
///
/// # Errors
///
/// Returns [`TopologyError`] if parameters or inputs are inconsistent.
pub fn form_local_synapses(
    input: &SynapseFormationInput,
) -> Result<LocalSynapsePlan, TopologyError> {
    // 1. Validation checks
    if input.growth.axons.len() != input.topology.somas.len() {
        return Err(TopologyError::CapacityOverflow);
    }
    for (idx, axon_path) in input.growth.axons.iter().enumerate() {
        if input.topology.somas[idx].soma_id != axon_path.soma_id {
            return Err(TopologyError::CapacityOverflow);
        }
    }

    if !input.voxel_size_um.is_finite() || input.voxel_size_um <= 0.0 {
        return Err(TopologyError::InvalidGrowthParameter {
            variant_id: 0,
            field: "voxel_size_um",
        });
    }

    let config = input.config;
    let topology = input.topology;
    let growth = input.growth;

    // Verify all target and source soma variant ids exist
    for soma in &topology.somas {
        if (soma.variant_id as usize) >= config.neuron_types.len() {
            return Err(TopologyError::UnknownNeuronType {
                variant_id: soma.variant_id,
            });
        }
    }

    // 2. Candidate collection per target soma
    let mut soma_candidates: Vec<Vec<SynapseCandidate>> = vec![Vec::new(); topology.somas.len()];

    for axon_path in &growth.axons {
        let source_soma_id = axon_path.soma_id;
        let axon_id = source_soma_id;

        if source_soma_id > types::MAX_AXON_ID {
            return Err(TopologyError::InvalidSynapseTarget {
                axon_id: source_soma_id,
                segment_offset: 0,
            });
        }

        let source_soma = topology
            .somas
            .get(source_soma_id as usize)
            .ok_or(TopologyError::CapacityOverflow)?;

        if source_soma.soma_id != source_soma_id {
            // Somas should be ordered and match indexing
            return Err(TopologyError::CapacityOverflow);
        }

        let source_variant_id = source_soma.variant_id;
        let source_type = &config.neuron_types[source_variant_id as usize];

        for segment in &axon_path.segments {
            if segment.segment_offset == 0 {
                return Err(TopologyError::InvalidSynapseTarget {
                    axon_id,
                    segment_offset: 0,
                });
            }

            let x_seg = segment.position.x() as i32;
            let y_seg = segment.position.y() as i32;
            let z_seg = segment.position.z() as i32;

            for (target_idx, target_soma) in topology.somas.iter().enumerate() {
                if source_soma_id == target_soma.soma_id {
                    // self-synapses are forbidden
                    continue;
                }

                let target_type = &config.neuron_types[target_soma.variant_id as usize];

                // Whitelist check
                let whitelist = &target_type.growth.dendrite_whitelist;
                let allowed = if whitelist.is_empty() {
                    true
                } else {
                    whitelist.iter().any(|name| name == &source_type.name)
                };

                if !allowed {
                    continue;
                }

                // Radius conversion and check
                let dendrite_radius_um = target_type.growth.dendrite_radius_um;
                if !dendrite_radius_um.is_finite() || dendrite_radius_um <= 0.0 {
                    return Err(TopologyError::InvalidGrowthParameter {
                        variant_id: target_soma.variant_id,
                        field: "dendrite_radius_um",
                    });
                }

                let ratio = (dendrite_radius_um as f64) / (input.voxel_size_um as f64);
                let radius_voxels_f = ratio.ceil();
                if !radius_voxels_f.is_finite() || radius_voxels_f <= 0.0 {
                    return Err(TopologyError::InvalidGrowthParameter {
                        variant_id: target_soma.variant_id,
                        field: "dendrite_radius_um",
                    });
                }

                // Compute max possible distance squared in voxel space for the current shard dimensions
                let w = config.dimensions.w as i64;
                let d = config.dimensions.d as i64;
                let h = config.dimensions.h as i64;

                let max_dx = w.checked_sub(1).ok_or(TopologyError::CapacityOverflow)?;
                let max_dy = d.checked_sub(1).ok_or(TopologyError::CapacityOverflow)?;
                let max_dz = h.checked_sub(1).ok_or(TopologyError::CapacityOverflow)?;

                let max_dx2 = max_dx
                    .checked_mul(max_dx)
                    .ok_or(TopologyError::CapacityOverflow)?;
                let max_dy2 = max_dy
                    .checked_mul(max_dy)
                    .ok_or(TopologyError::CapacityOverflow)?;
                let max_dz2 = max_dz
                    .checked_mul(max_dz)
                    .ok_or(TopologyError::CapacityOverflow)?;

                let max_dist_sq = max_dx2
                    .checked_add(max_dy2)
                    .and_then(|sum| sum.checked_add(max_dz2))
                    .ok_or(TopologyError::CapacityOverflow)?;

                let is_huge_radius = (radius_voxels_f * radius_voxels_f) >= (max_dist_sq as f64);
                let radius_voxels_sq = if is_huge_radius {
                    max_dist_sq
                } else {
                    let radius_voxels = radius_voxels_f as i64;
                    radius_voxels
                        .checked_mul(radius_voxels)
                        .ok_or(TopologyError::CapacityOverflow)?
                };

                let x_soma = target_soma.position.x() as i32;
                let y_soma = target_soma.position.y() as i32;
                let z_soma = target_soma.position.z() as i32;

                let dx = x_seg
                    .checked_sub(x_soma)
                    .ok_or(TopologyError::CapacityOverflow)?;
                let dy = y_seg
                    .checked_sub(y_soma)
                    .ok_or(TopologyError::CapacityOverflow)?;
                let dz = z_seg
                    .checked_sub(z_soma)
                    .ok_or(TopologyError::CapacityOverflow)?;

                let dx2 = dx.checked_mul(dx).ok_or(TopologyError::CapacityOverflow)?;
                let dy2 = dy.checked_mul(dy).ok_or(TopologyError::CapacityOverflow)?;
                let dz2 = dz.checked_mul(dz).ok_or(TopologyError::CapacityOverflow)?;

                let dist_sq = dx2
                    .checked_add(dy2)
                    .and_then(|sum| sum.checked_add(dz2))
                    .ok_or(TopologyError::CapacityOverflow)?;

                if (dist_sq as i64) <= radius_voxels_sq {
                    soma_candidates[target_idx].push(SynapseCandidate {
                        distance_sq: dist_sq as i64,
                        segment_offset: segment.segment_offset,
                        source_soma_id,
                        axon_id,
                        source_type_id: source_variant_id,
                    });
                }
            }
        }
    }

    // 3. Sorting, ranking, capping and packaging
    let mut rows = Vec::with_capacity(topology.somas.len());
    let mut total_live_synapses = 0;
    let mut total_dropped_candidates = 0;

    for (target_idx, target_soma) in topology.somas.iter().enumerate() {
        let candidates = &mut soma_candidates[target_idx];

        candidates.sort_by(|a, b| {
            let cmp = a.distance_sq.cmp(&b.distance_sq);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
            let cmp = a.segment_offset.cmp(&b.segment_offset);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
            let cmp = a.source_soma_id.cmp(&b.source_soma_id);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
            a.axon_id.cmp(&b.axon_id)
        });

        let total_candidates = candidates.len();
        let cap = std::cmp::min(total_candidates, layout::MAX_DENDRITES);
        let dropped = total_candidates.saturating_sub(cap);
        total_dropped_candidates += dropped;

        let mut slots = Vec::with_capacity(cap);

        for (slot_idx, cand) in candidates.iter().enumerate().take(cap) {
            let packed_target = PackedTarget::try_pack(cand.axon_id, cand.segment_offset as u32)
                .map_err(|_| TopologyError::InvalidSynapseTarget {
                    axon_id: cand.axon_id,
                    segment_offset: cand.segment_offset,
                })?;

            if packed_target.is_zero_none() || packed_target.is_tombstone() {
                return Err(TopologyError::InvalidSynapseTarget {
                    axon_id: cand.axon_id,
                    segment_offset: cand.segment_offset,
                });
            }

            let source_type = &config.neuron_types[cand.source_type_id as usize];
            let initial = source_type.gsop.initial_synapse_weight as i32;
            let base_mass = if initial > 0 {
                initial
                    .checked_shl(physics::MASS_TO_CHARGE_SHIFT)
                    .ok_or(TopologyError::CapacityOverflow)?
            } else {
                physics::MIN_WEIGHT_LIMIT
            };

            let weight = if source_type.gsop.is_inhibitory {
                base_mass
                    .checked_neg()
                    .ok_or(TopologyError::CapacityOverflow)?
            } else {
                base_mass
            };

            slots.push(FormedSynapse {
                dendrite_slot: slot_idx as u8,
                target: packed_target,
                weight,
                timer: 0,
                source_soma_id: cand.source_soma_id,
                axon_id: cand.axon_id,
                segment_offset: cand.segment_offset,
            });
        }

        total_live_synapses += slots.len();
        rows.push(NeuronSynapseRow {
            target_soma_id: target_soma.soma_id,
            slots,
        });
    }

    Ok(LocalSynapsePlan {
        rows,
        total_live_synapses,
        dropped_candidates: total_dropped_candidates,
    })
}
