use crate::dto::{LocalShardArtifacts, LocalShardBakeInput, LocalShardBakeReport};
use crate::error::BakerError;
use bytemuck::bytes_of;
use config::validate_shard;
use layout::{
    align_to_padded_n, calculate_paths_file_size, calculate_paths_matrix_offset,
    calculate_state_blob_size, compute_state_offsets, AxonsFileHeader, BurstHeads8,
    PathsFileHeader, StateFileHeader, VariantParameters, MAX_DENDRITES, MAX_SEGMENTS_PER_AXON,
    VARIANT_LUT_LEN,
};
use topology::{SynapseFormationInput, TopologyEngine};
use types::{PackedPosition, SomaFlags, AXON_SENTINEL};

/// Performs baking of a local single shard in memory.
pub fn bake_local_shard(
    input: &LocalShardBakeInput,
) -> Result<(LocalShardArtifacts, LocalShardBakeReport), BakerError> {
    // 1. Validate the shard configuration
    validate_shard(input.shard_config)?;

    // 2. Local Topology Generation
    let single_shard_input = topology::SingleShardTopologyInput {
        config: input.shard_config,
        seed: input.master_seed,
    };
    let single_shard_topology = TopologyEngine::generate_single_shard_topology(&single_shard_input)
        .map_err(BakerError::TopologyError)?;

    let axon_growth_input = topology::AxonGrowthInput {
        config: input.shard_config,
        topology: &single_shard_topology,
        seed: input.master_seed,
    };
    let local_growth_result =
        TopologyEngine::grow_local_axons(&axon_growth_input).map_err(BakerError::TopologyError)?;

    let synapse_formation_input = SynapseFormationInput {
        config: input.shard_config,
        topology: &single_shard_topology,
        growth: &local_growth_result,
        voxel_size_um: input.voxel_size_um,
        seed: input.master_seed,
    };
    let synapse_plan = TopologyEngine::form_local_synapses(&synapse_formation_input)
        .map_err(BakerError::TopologyError)?;

    // 3. Compile VariantParameters table
    let mut variant_table = [bytemuck::Zeroable::zeroed(); VARIANT_LUT_LEN];
    for (variant_id, nt) in input.shard_config.neuron_types.iter().enumerate() {
        if variant_id >= VARIANT_LUT_LEN {
            return Err(BakerError::LayoutError);
        }

        let heartbeat_m =
            physics::compile_dds_heartbeat(nt.spontaneous.spontaneous_firing_period_ticks as u64);

        let mut inertia_curve = [0u8; 8];
        inertia_curve.copy_from_slice(&nt.gsop.inertia_curve);

        let vp = VariantParameters {
            threshold: nt.membrane.threshold,
            rest_potential: nt.membrane.rest_potential,
            leak_shift: nt.membrane.leak_shift,
            homeostasis_penalty: nt.homeostasis.homeostasis_penalty,
            spontaneous_firing_period_ticks: nt.spontaneous.spontaneous_firing_period_ticks,
            initial_synapse_weight: nt.gsop.initial_synapse_weight,
            gsop_potentiation: nt.gsop.gsop_potentiation,
            gsop_depression: nt.gsop.gsop_depression,
            homeostasis_decay: nt.homeostasis.homeostasis_decay,
            refractory_period: nt.timing.refractory_period,
            synapse_refractory_period: nt.timing.synapse_refractory_period,
            signal_propagation_length: nt.signal.signal_propagation_length,
            is_inhibitory: if nt.gsop.is_inhibitory { 1 } else { 0 },
            inertia_curve,
            ahp_amplitude: nt.membrane.ahp_amplitude,
            _pad1: [0; 6],
            adaptive_leak_min_shift: nt.adaptive_leak.adaptive_leak_min_shift,
            adaptive_leak_gain: nt.adaptive_leak.adaptive_leak_gain,
            adaptive_mode: nt.adaptive_leak.adaptive_mode,
            _leak_pad: [0; 3],
            d1_affinity: nt.dopamine.d1_affinity,
            d2_affinity: nt.dopamine.d2_affinity,
            heartbeat_m,
        };

        variant_table[variant_id] = vp;
    }

    // 4. Build .state blob
    let total_somas = single_shard_topology.somas.len();
    let total_axons = total_somas;
    let padded_n = align_to_padded_n(total_somas);

    let total_somas_u32 = u32::try_from(total_somas).map_err(|_| BakerError::LayoutError)?;
    let total_axons_u32 = u32::try_from(total_axons).map_err(|_| BakerError::LayoutError)?;
    let padded_n_u32 = u32::try_from(padded_n).map_err(|_| BakerError::LayoutError)?;

    let state_size = calculate_state_blob_size(padded_n);

    let mut state_blob = vec![0u8; state_size];
    let state_header = StateFileHeader::new(padded_n_u32, total_axons_u32);
    state_blob[0..16].copy_from_slice(bytes_of(&state_header));

    let offsets = compute_state_offsets(padded_n);

    // Fill SoA Voltage plane
    {
        let mut voltage_i32 = vec![0i32; padded_n];
        for soma in &single_shard_topology.somas {
            let variant = &variant_table[soma.variant_id as usize];
            voltage_i32[soma.soma_id as usize] = variant.rest_potential;
        }
        state_blob[offsets.off_voltage..offsets.off_voltage + padded_n * 4]
            .copy_from_slice(bytemuck::cast_slice(&voltage_i32));
    }

    // Fill SoA Flags plane
    {
        let flags_slice = &mut state_blob[offsets.off_flags..offsets.off_flags + padded_n];
        for soma in &single_shard_topology.somas {
            let flags = SomaFlags::new(false, 0, soma.variant_id);
            flags_slice[soma.soma_id as usize] = flags.0;
        }
    }

    // Fill SoA Soma to Axon plane
    {
        let mut s2a_u32 = vec![0u32; padded_n];
        for soma in &single_shard_topology.somas {
            s2a_u32[soma.soma_id as usize] = soma.soma_id;
        }
        state_blob[offsets.off_s2a..offsets.off_s2a + padded_n * 4]
            .copy_from_slice(bytemuck::cast_slice(&s2a_u32));
    }

    // Fill Dendrites matrices planes
    {
        let mut targets_u32 = vec![0u32; MAX_DENDRITES * padded_n];
        let mut weights_i32 = vec![0i32; MAX_DENDRITES * padded_n];
        let mut dtimers_u8 = vec![0u8; MAX_DENDRITES * padded_n];

        for row in &synapse_plan.rows {
            let target_soma_index = row.target_soma_id as usize;
            for formed in &row.slots {
                let slot = formed.dendrite_slot as usize;
                let idx = slot * padded_n + target_soma_index;
                targets_u32[idx] = formed.target.0;
                weights_i32[idx] = formed.weight;
                dtimers_u8[idx] = formed.timer;
            }
        }

        state_blob[offsets.off_targets..offsets.off_targets + MAX_DENDRITES * padded_n * 4]
            .copy_from_slice(bytemuck::cast_slice(&targets_u32));

        state_blob[offsets.off_weights..offsets.off_weights + MAX_DENDRITES * padded_n * 4]
            .copy_from_slice(bytemuck::cast_slice(&weights_i32));

        state_blob[offsets.off_dtimers..offsets.off_dtimers + MAX_DENDRITES * padded_n]
            .copy_from_slice(&dtimers_u8);
    }

    // 5. Build .axons blob
    let heads_body_size = total_axons
        .checked_mul(std::mem::size_of::<BurstHeads8>())
        .ok_or(BakerError::LayoutError)?;
    let axons_size = 16_usize
        .checked_add(heads_body_size)
        .ok_or(BakerError::LayoutError)?;

    let mut axons_blob = vec![0u8; axons_size];
    let axons_header = AxonsFileHeader::new(total_axons_u32);
    axons_blob[0..16].copy_from_slice(bytes_of(&axons_header));

    {
        let heads = vec![BurstHeads8::empty(AXON_SENTINEL); total_axons];
        axons_blob[16..].copy_from_slice(bytemuck::cast_slice(&heads));
    }

    // 6. Build .paths blob
    let paths_size = calculate_paths_file_size(total_axons);
    let mut paths_blob = vec![0u8; paths_size];
    let paths_header = PathsFileHeader::new(total_axons_u32, MAX_SEGMENTS_PER_AXON as u32);
    paths_blob[0..16].copy_from_slice(bytes_of(&paths_header));

    // Fill lengths plane
    {
        let mut lengths = vec![0u16; total_axons];
        for path in &local_growth_result.axons {
            let path_len =
                u16::try_from(1 + path.segments.len()).map_err(|_| BakerError::LayoutError)?;
            lengths[path.soma_id as usize] = path_len;
        }
        paths_blob[16..16 + total_axons * 2].copy_from_slice(bytemuck::cast_slice(&lengths));
    }

    // Fill coordinate matrix
    {
        let mut matrix_pos = vec![PackedPosition(0); total_axons * MAX_SEGMENTS_PER_AXON];

        for path in &local_growth_result.axons {
            let base_idx = path.soma_id as usize * MAX_SEGMENTS_PER_AXON;

            // Slot 0 holds origin soma position
            let soma = &single_shard_topology.somas[path.soma_id as usize];
            matrix_pos[base_idx] = soma.position;

            // Slots segment_offset hold segments positions
            for segment in &path.segments {
                let idx = base_idx + segment.segment_offset as usize;
                matrix_pos[idx] = segment.position;
            }
        }

        let matrix_offset = calculate_paths_matrix_offset(total_axons);
        paths_blob[matrix_offset..matrix_offset + total_axons * MAX_SEGMENTS_PER_AXON * 4]
            .copy_from_slice(bytemuck::cast_slice(&matrix_pos));
    }

    let artifacts = LocalShardArtifacts {
        state_blob,
        axons_blob,
        paths_blob,
        variant_table,
    };

    let total_synapses_u32 =
        u32::try_from(synapse_plan.total_live_synapses).map_err(|_| BakerError::LayoutError)?;

    let report = LocalShardBakeReport {
        total_somas: total_somas_u32,
        total_axons: total_axons_u32,
        total_synapses: total_synapses_u32,
        dropped_candidates: synapse_plan.dropped_candidates as u64,
    };

    Ok((artifacts, report))
}

/// Packs compiled local shard binary artifacts into a single `.axic` archive buffer.
///
/// # Errors
///
/// Returns a [`BakerError`] if VFS packaging fails.
pub fn pack_local_shard_artifacts(artifacts: &LocalShardArtifacts) -> Result<Vec<u8>, BakerError> {
    use crate::dto::{
        AXONS_ARCHIVE_PATH, PATHS_ARCHIVE_PATH, STATE_ARCHIVE_PATH, VARIANT_TABLE_ARCHIVE_PATH,
    };
    use vfs::ArchiveEntry;

    let variant_table_bytes = bytemuck::cast_slice(&artifacts.variant_table);

    let entries = [
        ArchiveEntry {
            path: STATE_ARCHIVE_PATH,
            bytes: &artifacts.state_blob,
        },
        ArchiveEntry {
            path: AXONS_ARCHIVE_PATH,
            bytes: &artifacts.axons_blob,
        },
        ArchiveEntry {
            path: PATHS_ARCHIVE_PATH,
            bytes: &artifacts.paths_blob,
        },
        ArchiveEntry {
            path: VARIANT_TABLE_ARCHIVE_PATH,
            bytes: variant_table_bytes,
        },
    ];

    let packed = vfs::pack_entries(&entries)?;
    Ok(packed)
}

/// Bakes local shard topology and directly compiles it into an `.axic` container buffer.
///
/// # Errors
///
/// Returns a [`BakerError`] if baking or VFS packaging fails.
pub fn bake_local_shard_axic(
    input: &LocalShardBakeInput,
) -> Result<(Vec<u8>, LocalShardBakeReport), BakerError> {
    let (artifacts, report) = bake_local_shard(input)?;
    let axic_bytes = pack_local_shard_artifacts(&artifacts)?;
    Ok((axic_bytes, report))
}
