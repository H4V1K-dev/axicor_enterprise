//! Local hot-loop simulation engine for CpuBackend.

use crate::resource::HostResource;
use compute_api::{validation, BatchResult, ComputeApiError, DayBatchCmd};
use std::time::Instant;

/// Executes a day batch run on the provided host resource using the given thread pool.
pub fn run_day_batch(
    resource: &mut HostResource,
    cmd: DayBatchCmd<'_>,
    pool: &rayon::ThreadPool,
) -> Result<BatchResult, ComputeApiError> {
    if !resource.uploaded {
        return Err(ComputeApiError::InvalidBatch);
    }
    validation::validate_day_batch_cmd(&cmd)?;

    let start_time = Instant::now();
    let padded_n = resource.spec.padded_n as usize;
    let total_axons = resource.spec.total_axons as usize;
    let offsets = layout::compute_state_offsets(padded_n);

    let mut generated_spikes_count: u32 = 0;
    let mut output_spikes_written: u32 = 0;
    let mut dropped_spikes_count: u32 = 0;

    // Reset output_spike_counts to zero for all ticks in this batch
    for count in cmd
        .output_spike_counts
        .iter_mut()
        .take(cmd.sync_batch_ticks as usize)
    {
        *count = 0;
    }

    pool.install(|| {
        let state_bytes = resource.state_blob.as_slice_mut();
        let axons_bytes = resource.axons_blob.as_slice_mut();

        let (voltage_bytes, rest_state) = state_bytes.split_at_mut(offsets.off_flags);
        let soma_voltage =
            bytemuck::cast_slice_mut::<u8, i32>(&mut voltage_bytes[offsets.off_voltage..]);

        let (flags_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_thresh - offsets.off_flags);
        let soma_flags = &mut flags_bytes[..padded_n];

        let (thresh_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_timers - offsets.off_thresh);
        let threshold_offset =
            bytemuck::cast_slice_mut::<u8, i32>(&mut thresh_bytes[..padded_n * 4]);

        let (timers_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_s2a - offsets.off_timers);
        let timers = &mut timers_bytes[..padded_n];

        let (s2a_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_targets - offsets.off_s2a);
        let soma_to_axon = bytemuck::cast_slice::<u8, u32>(&s2a_bytes[..padded_n * 4]);

        let (targets_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_weights - offsets.off_targets);
        let dendrite_targets =
            bytemuck::cast_slice::<u8, u32>(&targets_bytes[..layout::MAX_DENDRITES * padded_n * 4]);

        let (weights_bytes, rest_state) =
            rest_state.split_at_mut(offsets.off_dtimers - offsets.off_weights);
        let dendrite_weights = bytemuck::cast_slice_mut::<u8, i32>(
            &mut weights_bytes[..layout::MAX_DENDRITES * padded_n * 4],
        );

        let dendrite_timers = &mut rest_state[..layout::MAX_DENDRITES * padded_n];

        let axon_heads =
            bytemuck::cast_slice_mut::<u8, u32>(&mut axons_bytes[16..16 + total_axons * 32]);

        for tick_idx in 0..cmd.sync_batch_ticks as usize {
            let current_tick = cmd.tick_base + tick_idx as u64;

            // Stage 2: Virtual Inputs Injection
            if let Some(bitmask) = cmd.input_bitmask {
                let words_per_tick = cmd.input_words_per_tick as usize;
                let start_w = tick_idx * words_per_tick;
                let end_w = start_w + words_per_tick;
                if end_w <= bitmask.len() {
                    let tick_mask = &bitmask[start_w..end_w];
                    for k in 0..cmd.num_virtual_axons as usize {
                        let word_idx = k / 32;
                        let bit_idx = k % 32;
                        if word_idx < tick_mask.len() && (tick_mask[word_idx] & (1 << bit_idx)) != 0
                        {
                            let global_axon_id = cmd.virtual_offset as usize + k;
                            let shard_virtual_offset = resource.spec.virtual_offset as usize;
                            if global_axon_id >= shard_virtual_offset {
                                let local_axon_id = global_axon_id - shard_virtual_offset;
                                if local_axon_id < total_axons {
                                    let base = local_axon_id * 8;
                                    axon_heads[base + 7] = axon_heads[base + 6];
                                    axon_heads[base + 6] = axon_heads[base + 5];
                                    axon_heads[base + 5] = axon_heads[base + 4];
                                    axon_heads[base + 4] = axon_heads[base + 3];
                                    axon_heads[base + 3] = axon_heads[base + 2];
                                    axon_heads[base + 2] = axon_heads[base + 1];
                                    axon_heads[base + 1] = axon_heads[base];
                                    axon_heads[base] = physics::initial_axon_head(cmd.v_seg);
                                }
                            }
                        }
                    }
                }
            }

            // Stage 3: Incoming Spikes Injection
            if let Some(spikes) = cmd.incoming_spikes {
                let mut count = cmd.incoming_spike_counts[tick_idx] as usize;
                count = count.min(cmd.max_spikes_per_tick as usize);
                let start_s = tick_idx * cmd.max_spikes_per_tick as usize;
                let end_s = start_s + count;
                if end_s <= spikes.len() {
                    for &axon_id in &spikes[start_s..end_s] {
                        let a_id = axon_id as usize;
                        if a_id < total_axons {
                            let base = a_id * 8;
                            axon_heads[base + 7] = axon_heads[base + 6];
                            axon_heads[base + 6] = axon_heads[base + 5];
                            axon_heads[base + 5] = axon_heads[base + 4];
                            axon_heads[base + 4] = axon_heads[base + 3];
                            axon_heads[base + 3] = axon_heads[base + 2];
                            axon_heads[base + 2] = axon_heads[base + 1];
                            axon_heads[base + 1] = axon_heads[base];
                            axon_heads[base] = physics::initial_axon_head(cmd.v_seg);
                        }
                    }
                }
            }

            // Stage 4: Active Tail Signal Propagation
            for a in 0..total_axons {
                let base = a * 8;
                for h in 0..8 {
                    axon_heads[base + h] = physics::propagate_head(axon_heads[base + h], cmd.v_seg);
                }
            }

            // Stage 5: Neuron State Update, Fatigue Recovery & Integration, Somatic Reset
            for i in 0..padded_n {
                let mut flags = types::SomaFlags(soma_flags[i]);
                let type_id = flags.type_id() as usize;
                let variant = &resource.variant_table[type_id.min(layout::VARIANT_LUT_LEN - 1)];

                threshold_offset[i] = physics::homeostasis_decay(
                    threshold_offset[i],
                    variant.homeostasis_decay as i32,
                );

                // 1. Fatigue Recovery & Charge Integration across live dendrites
                let mut i_in: i32 = 0;
                let is_refractory = timers[i] > 0;

                for d in 0..layout::MAX_DENDRITES {
                    let slot_idx = d * padded_n + i;
                    let raw_target = dendrite_targets[slot_idx];
                    if types::PackedTarget(raw_target).is_inactive() {
                        dendrite_timers[slot_idx] = 0;
                        continue;
                    }

                    let mut fatigue = physics::recover_fatigue(dendrite_timers[slot_idx]);

                    if !is_refractory {
                        if let Some((axon_id, seg_idx)) = types::PackedTarget(raw_target).unpack() {
                            let a_id = axon_id as usize;
                            if a_id < total_axons {
                                let base = a_id * 8;
                                let heads = [
                                    axon_heads[base],
                                    axon_heads[base + 1],
                                    axon_heads[base + 2],
                                    axon_heads[base + 3],
                                    axon_heads[base + 4],
                                    axon_heads[base + 5],
                                    axon_heads[base + 6],
                                    axon_heads[base + 7],
                                ];
                                if physics::active_tail_hit(
                                    &heads,
                                    seg_idx,
                                    variant.signal_propagation_length as u32,
                                ) {
                                    let w = dendrite_weights[slot_idx];
                                    let att_w = physics::apply_synaptic_fatigue(
                                        w,
                                        fatigue,
                                        variant.fatigue_capacity,
                                    );
                                    i_in = i_in.wrapping_add(physics::weight_to_charge(att_w));
                                    fatigue = physics::fatigue_after_spike(
                                        fatigue,
                                        variant.fatigue_capacity,
                                    );
                                }
                            }
                        }
                    }

                    dendrite_timers[slot_idx] = fatigue;
                }

                // 2. Membrane Potential & Spiking Evaluation
                let mut is_glif = false;
                if is_refractory {
                    timers[i] -= 1;
                } else {
                    let v_new = physics::update_glif_voltage(
                        soma_voltage[i],
                        i_in,
                        variant.rest_potential,
                        threshold_offset[i],
                        variant.leak_shift as i32,
                        variant.adaptive_leak_gain as i32,
                        variant.adaptive_leak_min_shift,
                        variant.adaptive_mode as i32,
                    );
                    is_glif = physics::is_glif_spike(v_new, variant.threshold, threshold_offset[i]);
                    if !is_glif {
                        soma_voltage[i] = v_new;
                    }
                }

                let is_heartbeat =
                    physics::heartbeat_spike(current_tick, variant.heartbeat_m, i as u32);
                let final_spike = is_glif || is_heartbeat;

                flags.set_spiking(final_spike);
                if final_spike {
                    soma_voltage[i] = variant
                        .rest_potential
                        .wrapping_sub(variant.ahp_amplitude as i32);
                    timers[i] = variant.refractory_period;
                    threshold_offset[i] =
                        threshold_offset[i].wrapping_add(variant.homeostasis_penalty);

                    flags.set_burst_count(flags.burst_count().saturating_add(1));
                    generated_spikes_count = generated_spikes_count.saturating_add(1);

                    if cmd.mapped_soma_ids.contains(&(i as u32)) {
                        let current_out_count = cmd.output_spike_counts[tick_idx];
                        if current_out_count < cmd.max_spikes_per_tick {
                            let out_offset = tick_idx * cmd.max_spikes_per_tick as usize
                                + current_out_count as usize;
                            if out_offset < cmd.output_spikes.len() {
                                cmd.output_spikes[out_offset] = i as u32;
                                cmd.output_spike_counts[tick_idx] = current_out_count + 1;
                                output_spikes_written = output_spikes_written.saturating_add(1);
                            }
                        } else {
                            dropped_spikes_count = dropped_spikes_count.saturating_add(1);
                        }
                    }
                }

                soma_flags[i] = flags.0;
            }

            // Stage 6: GSOP Plasticity Pass (All-to-All STDP)
            if physics::is_plasticity_enabled() {
                #[allow(clippy::needless_range_loop)]
                for i in 0..padded_n {
                    let flags = types::SomaFlags(soma_flags[i]);
                    if !flags.spiking() {
                        continue;
                    }

                    let type_id = flags.type_id() as usize;
                    let variant = &resource.variant_table[type_id.min(layout::VARIANT_LUT_LEN - 1)];
                    let burst_count = flags.burst_count();

                    let mut inertia_curve = [0i32; 8];
                    for (k, val) in variant.inertia_curve.iter().enumerate() {
                        inertia_curve[k] = *val as i32;
                    }

                    for d in 0..layout::MAX_DENDRITES {
                        let slot_idx = d * padded_n + i;
                        let raw_target = dendrite_targets[slot_idx];
                        if types::PackedTarget(raw_target).is_inactive() {
                            continue;
                        }

                        if let Some((axon_id, seg_idx)) = types::PackedTarget(raw_target).unpack() {
                            let a_id = axon_id as usize;
                            if a_id < total_axons {
                                let base = a_id * 8;
                                let heads = [
                                    axon_heads[base],
                                    axon_heads[base + 1],
                                    axon_heads[base + 2],
                                    axon_heads[base + 3],
                                    axon_heads[base + 4],
                                    axon_heads[base + 5],
                                    axon_heads[base + 6],
                                    axon_heads[base + 7],
                                ];
                                let w = dendrite_weights[slot_idx];
                                let fat = dendrite_timers[slot_idx];
                                let w_new = physics::apply_gsop_plasticity(
                                    w,
                                    &heads,
                                    seg_idx,
                                    variant.signal_propagation_length as u32,
                                    fat,
                                    variant.fatigue_capacity,
                                    variant.gsop_potentiation as i32,
                                    variant.gsop_depression as i32,
                                    cmd.dopamine as i32,
                                    variant.d1_affinity as i32,
                                    variant.d2_affinity as i32,
                                    burst_count as u32,
                                    &inertia_curve,
                                );
                                dendrite_weights[slot_idx] = w_new;
                            }
                        }
                    }
                }
            }

            // Stage 7: Local Spike Axon Head Emission
            for i in 0..padded_n {
                let flags = types::SomaFlags(soma_flags[i]);
                if flags.spiking() {
                    let axon_id = soma_to_axon[i];
                    if (axon_id as usize) < total_axons {
                        let base = (axon_id as usize) * 8;
                        axon_heads[base + 7] = axon_heads[base + 6];
                        axon_heads[base + 6] = axon_heads[base + 5];
                        axon_heads[base + 5] = axon_heads[base + 4];
                        axon_heads[base + 4] = axon_heads[base + 3];
                        axon_heads[base + 3] = axon_heads[base + 2];
                        axon_heads[base + 2] = axon_heads[base + 1];
                        axon_heads[base + 1] = axon_heads[base];
                        axon_heads[base] = physics::initial_axon_head(cmd.v_seg);
                    }
                }
            }
        }
    });

    let execution_time_us = start_time.elapsed().as_micros() as u64;

    Ok(BatchResult {
        ticks_executed: cmd.sync_batch_ticks,
        generated_spikes_count,
        output_spikes_written,
        dropped_spikes_count,
        execution_time_us,
    })
}
