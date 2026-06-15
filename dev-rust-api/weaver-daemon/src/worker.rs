//! Night Phase processing pipeline for the weaver daemon.

/// Daemon context holding host-RAM state of growing axons.
pub struct DaemonContext {
    /// Local CPU-side state of all growing axon tips (Night Phase only).
    pub living_axons: Vec<topology::types::LivingAxon>,
}

/// Executes the full Night Phase processing pipeline for one shard.
///
/// # Pipeline Order
/// 1. Nudge living axons (axon growth step).
/// 2. Collect sprouting candidates (spiked somas).
/// 3. Sprout new synapses via `topology::sprouting::sprout_connections`.
/// 4. Apply new synapses to SoA weight/target arrays (E-092: max_sprouts cap).
/// 5. Prune low-weight synapses (`crate::compaction::prune_synapses`).
/// 6. Defragment dendrite arrays (`crate::compaction::compact_dendrites`).
///
/// # Safety
/// `shm_ptr` must point to a valid, fully initialized shared memory region whose
/// total size covers all SoA array extents implied by `hdr.padded_n` and
/// `hdr.dendrite_slots`. The caller guarantees exclusive write access during the
/// Night Phase (no GPU kernel is running concurrently).
///
/// # Errors
/// Returns `anyhow::Error` on any invariant violation or IO failure.
#[allow(clippy::too_many_arguments)]
pub fn process_night(
    ctx: &mut DaemonContext,
    shm_ptr: *mut u8,
    hdr: &layout::ShmHeader,
    blueprints: &config::BlueprintsConfig,
    bounds: (u32, u32, u32),
    prune_threshold: i16,
    max_sprouts: u32,
    soma_positions: &[types::PackedPosition],
) -> Result<(), anyhow::Error> {
    let padded_n = hdr.padded_n as usize;
    let slots = hdr.dendrite_slots as usize;
    let total_synaptic = padded_n * slots;

    // ── Phase 1: Extract mutable SoA slices from shared memory (unsafe boundary) ─
    //
    // SAFETY: Caller guarantees shm_ptr points to a valid, fully-mapped SHM region
    // with exclusive access. Offsets from hdr.* are set by ipc::MockShmAllocator
    // and are guaranteed aligned to 64-byte cache lines (INV-IPC-001).
    let (weights, targets, soma_flags) = unsafe {
        let w_ptr = shm_ptr.add(hdr.weights_offset as usize) as *mut i32;
        let t_ptr = shm_ptr.add(hdr.targets_offset as usize) as *mut u32;
        let f_ptr = shm_ptr.add(hdr.flags_offset as usize) as *const u8;

        let weights = std::slice::from_raw_parts_mut(w_ptr, total_synaptic);
        let targets = std::slice::from_raw_parts_mut(t_ptr, total_synaptic);
        let soma_flags = std::slice::from_raw_parts(f_ptr, padded_n);

        (weights, targets, soma_flags)
    };

    // ── Phase 2: Nudge living axons (INV-TOPO-009, INV-TOPO-010) ─────────────
    let _growth_events = topology::tracing::nudge_living_axons(
        &mut ctx.living_axons,
        soma_flags,
        bounds,
        1.0,
    );
    // TODO: process GrowthEvents (Ghost Handovers & Prunes) into SHM queues.

    // ── Phase 3: Build spatial grid from current axon tip positions ───────────
    let segments: Vec<topology::types::AxonSegment> = ctx
        .living_axons
        .iter()
        .map(|axon| topology::types::AxonSegment {
            axon_id: axon.axon_id as u32,
            type_idx: 0, // type resolved from SHM blueprint lookup in future phase
            pos: axon.tip_uvw,
        })
        .collect();
    let grid = topology::types::SpatialGrid::build(segments);

    // ── Phase 4: Collect spiked soma indices (flag bit 0 = spike) ────────────
    let active_somas: Vec<usize> = soma_flags
        .iter()
        .enumerate()
        .filter(|&(_i, &flag)| (flag & 0x01) != 0)
        .map(|(i, _)| i)
        .collect();

    // ── Phase 5: Sprout new synapses (INV-TOPO-004..INV-TOPO-007) ────────────
    let new_synapses = topology::sprouting::sprout_connections(
        &active_somas,
        targets,
        padded_n,
        &grid,
        blueprints,
        prune_threshold,
        soma_positions,
    );

    // ── Phase 6: Apply new synapses with E-092 cap ───────────────────────────
    //
    // E-092: Apply at most max_sprouts synapses per Night Phase cycle.
    // Exceeding this limit would corrupt the SoA layout by overflowing slot bounds.
    let mut applied = 0u32;
    for syn in &new_synapses {
        if applied >= max_sprouts {
            break; // E-092: hard cap on per-cycle sprout budget
        }
        let flat_idx = syn.slot_idx * padded_n + syn.soma_idx;
        if flat_idx < total_synaptic {
            weights[flat_idx] = syn.weight;
            targets[flat_idx] = syn.target_packed;
            applied += 1;
        }
    }

    tracing::debug!(
        applied,
        max_sprouts,
        active_somas = active_somas.len(),
        "Night Phase: synapses applied"
    );

    // ── Phase 7: Prune weak synapses (§6.1) ──────────────────────────────────
    crate::compaction::prune_synapses(weights, targets, prune_threshold);

    // ── Phase 8: Compact dendrite arrays (§6.2, INV-WDAEMON-003) ─────────────
    crate::compaction::compact_dendrites(weights, targets, padded_n, slots);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{PackedPosition, PackedTarget};
    use config::{
        BlueprintsConfig, NeuronType, GsopParams, MembraneParams, TimingParams,
        SignalParams, HomeostasisParams, AdaptiveLeakParams, DopamineParams, SpontaneousParams,
    };

    #[repr(C, align(64))]
    struct AlignedBlock {
        data: [u8; 4096],
    }

    fn mock_neuron_type(name: &str, is_inhibitory: bool, gsop_potentiation: u16) -> NeuronType {
        NeuronType {
            name: name.to_string(),
            membrane: MembraneParams {
                threshold: 10,
                rest_potential: 0,
                leak_shift: 1,
            },
            timings: TimingParams {
                refractory_period: 2,
                synapse_refractory_period: 2,
            },
            signal: SignalParams {
                signal_propagation_length: 1,
            },
            homeostasis: HomeostasisParams {
                homeostasis_penalty: 0,
                homeostasis_decay: 0,
            },
            adaptive_leak: AdaptiveLeakParams {
                adaptive_leak_min_shift: 0,
                adaptive_leak_gain: 0,
                adaptive_mode: 0,
            },
            dopamine: DopamineParams {
                d1_affinity: 0,
                d2_affinity: 0,
            },
            gsop: GsopParams {
                gsop_potentiation,
                gsop_depression: 0,
                is_inhibitory,
                inertia_curve: vec![0],
            },
            spontaneous: SpontaneousParams {
                spontaneous_firing_period_ticks: 0,
            },
        }
    }

    #[test]
    fn test_process_night_full_cycle() {
        let blueprints = BlueprintsConfig {
            neuron_types: vec![
                mock_neuron_type("Exc", false, 15),
            ],
        };

        let soma_positions = vec![
            PackedPosition::pack_raw(10, 10, 10, 0), // soma 0
            PackedPosition::pack_raw(20, 20, 20, 0), // soma 1
        ];

        let mut ctx = DaemonContext {
            living_axons: vec![
                topology::types::LivingAxon {
                    axon_id: 100,
                    soma_idx: 0,
                    tip_uvw: PackedPosition::pack_raw(11, 10, 10, 0).0,
                    forward_dir: glam::Vec3::new(-1.0, 0.0, 0.0),
                    remaining_steps: 5,
                    last_night_active: false,
                },
            ],
        };

        let mut block = AlignedBlock { data: [0u8; 4096] };
        let shm_ptr = block.data.as_mut_ptr();

        let mut hdr = unsafe { std::mem::zeroed::<layout::ShmHeader>() };
        hdr.magic = layout::SHM_MAGIC;
        hdr.version = layout::SHM_VERSION;
        hdr.padded_n = 2;
        hdr.dendrite_slots = 4;
        hdr.weights_offset = 128;
        hdr.targets_offset = 192;
        hdr.flags_offset = 256;

        unsafe {
            let hdr_ptr = shm_ptr as *mut layout::ShmHeader;
            std::ptr::write(hdr_ptr, hdr);

            // Set soma_flags: soma 0 spiked (0x01), soma 1 did not (0x00)
            let f_ptr = shm_ptr.add(hdr.flags_offset as usize) as *mut u8;
            *f_ptr.add(0) = 0x01;
            *f_ptr.add(1) = 0x00;
        }

        let res = process_night(
            &mut ctx,
            shm_ptr,
            unsafe { &*(shm_ptr as *const layout::ShmHeader) },
            &blueprints,
            (100, 100, 100),
            10, // prune_threshold
            10, // max_sprouts
            &soma_positions,
        );

        assert!(res.is_ok());

        // Extract weights and targets to verify results
        let total_synaptic = 2 * 4;
        let w_ptr = unsafe { shm_ptr.add(hdr.weights_offset as usize) as *mut i32 };
        let t_ptr = unsafe { shm_ptr.add(hdr.targets_offset as usize) as *mut u32 };
        let weights = unsafe { std::slice::from_raw_parts(w_ptr, total_synaptic) };
        let targets = unsafe { std::slice::from_raw_parts(t_ptr, total_synaptic) };

        // The living axon grew (remaining steps decreased, active last night)
        assert!(ctx.living_axons[0].last_night_active);
        assert_eq!(ctx.living_axons[0].remaining_steps, 4);

        // A new connection sprouted at soma 0, slot 0 (flat index 0)
        // target axon 100, weight 15 shifted
        assert_eq!(PackedTarget(targets[0]).axon_id(), 100);
        assert_eq!(weights[0], 15 << 16);
    }
}

