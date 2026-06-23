use std::sync::RwLock;
use slotmap::{SlotMap, DefaultKey, Key, KeyData};
use compute_api::{GpuBackend, VramHandle, ShardLayout, DayBatchCmd, BatchResult, OutputFrame, TelemetryFrame, GhostPatch, ComputeApiError};
use layout::VariantParameters;
use crate::memory::ShardCpuResources;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

/// CPU compute backend implementing the `GpuBackend` interface.
///
/// Under INV-COMPUTE-CPU-001, this backend encapsulates shard allocations in an internal
/// thread-safe registry (`SlotMap` inside `RwLock`), preventing cross-shard access and ensuring
/// absolute memory isolation between shards.
pub struct CpuBackend {
    pub(crate) resources: RwLock<SlotMap<DefaultKey, ShardCpuResources>>,
}

impl CpuBackend {
    /// Initializes a new CPU-based computing backend.
    pub fn new() -> Result<Self, ComputeApiError> {
        Ok(Self {
            resources: RwLock::new(SlotMap::new()),
        })
    }

    /// Downloads the entire raw memory state of the shard.
    pub fn download_raw_state(&self, handle: &VramHandle) -> Result<Vec<u8>, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;
        Ok(resource.as_slice().to_vec())
    }
}

pub(crate) fn key_to_handle(key: DefaultKey) -> VramHandle {
    VramHandle(key.data().as_ffi())
}

pub(crate) fn handle_to_key(handle: &VramHandle) -> DefaultKey {
    KeyData::from_ffi(handle.0).into()
}

impl GpuBackend for CpuBackend {
    /// Allocates a 64-byte aligned flat state buffer in host RAM for a simulation shard.
    ///
    /// Under INV-COMPUTE-CPU-002, the shard layout neuron count `padded_n` must be a multiple of 64
    /// to ensure alignment with CPU cache lines, preventing cache false-sharing and enabling SIMD.
    fn alloc_shard(&self, layout: &ShardLayout) -> Result<VramHandle, ComputeApiError> {
        // Padded neuron count must be a multiple of 64
        if layout.padded_n % 64 != 0 {
            return Err(ComputeApiError::InvalidLayout);
        }

        // Compute state offsets and the total monolithic size needed
        let offsets = layout::compute_state_offsets(layout.padded_n as usize);

        // Allocate the resources with 64-byte alignment
        let resource = ShardCpuResources::new(offsets.total_size, layout.clone())?;

        let mut guard = self.resources.write().map_err(|_| ComputeApiError::DeviceLost)?;
        let key = guard.insert(resource);
        Ok(key_to_handle(key))
    }

    fn upload_state(&self, handle: &VramHandle, state: &[u8]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let mut guard = self.resources.write().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get_mut(key).ok_or(ComputeApiError::InvalidHandle)?;
        let slice = resource.as_mut_slice();
        let len = state.len().min(slice.len());
        slice[..len].copy_from_slice(&state[..len]);
        Ok(())
    }

    fn upload_variants(&self, handle: &VramHandle, variants: &[VariantParameters]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        let mut vp = resource.variant_params.lock().unwrap();
        let len = variants.len().min(vp.len());
        vp[..len].copy_from_slice(&variants[..len]);
        Ok(())
    }

    fn run_day_batch(&self, handle: &VramHandle, cmd: &DayBatchCmd<'_>) -> Result<BatchResult, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // E-062: Validate spike_counts slice length
        if cmd.spike_counts.len() as u32 != cmd.sync_batch_ticks {
            return Err(ComputeApiError::InvalidLayout);
        }

        // Validate bitmask length
        if let Some(mask) = cmd.input_bitmask {
            let required_bytes = ((cmd.num_virtual_axons as usize + 7) / 8) * cmd.sync_batch_ticks as usize;
            if mask.len() < required_bytes {
                return Err(ComputeApiError::InvalidLayout);
            }
        }

        // Cache last sizes for output download
        resource.last_num_outputs.store(cmd.num_outputs, std::sync::atomic::Ordering::Relaxed);
        resource.last_sync_batch_ticks.store(cmd.sync_batch_ticks, std::sync::atomic::Ordering::Relaxed);

        // Clear output history and resize to appropriate buffer
        let total_output_bytes = (cmd.num_outputs * cmd.sync_batch_ticks) as usize;
        {
            let mut history = resource.output_history.lock().unwrap();
            history.clear();
            history.resize(total_output_bytes, 0);
        }

        // Extract slices
        let (voltage, flags, threshold_offset, timers, soma_to_axon, dendrite_targets, dendrite_weights, dendrite_timers) = unsafe {
            resource.extract_all_slices()
        };

        let mut axon_heads = resource.axon_heads.lock().unwrap();
        let variant_params = resource.variant_params.lock().unwrap();

        let padded_n = resource.layout.padded_n as usize;

        // Tick loop
        for tick_idx in 0..cmd.sync_batch_ticks {
            let current_tick = cmd.tick_base + tick_idx;

            // 1. Inject inputs (sensors)
            if let Some(mask) = cmd.input_bitmask {
                let bytes_per_tick = (cmd.num_virtual_axons as usize + 7) / 8;
                let tick_offset = tick_idx as usize * bytes_per_tick;
                let tick_mask = &mask[tick_offset..tick_offset + bytes_per_tick];
                
                for tid in 0..cmd.num_virtual_axons as usize {
                    let byte_idx = tid / 8;
                    let bit_idx = tid % 8;
                    if (tick_mask[byte_idx] >> bit_idx) & 1 != 0 {
                        let axon_idx = (cmd.virtual_offset as usize) + tid;
                        if let Some(head) = axon_heads.get_mut(axon_idx) {
                            head.h7 = head.h6;
                            head.h6 = head.h5;
                            head.h5 = head.h4;
                            head.h4 = head.h3;
                            head.h3 = head.h2;
                            head.h2 = head.h1;
                            head.h1 = head.h0;
                            head.h0 = 0u32.wrapping_sub(cmd.v_seg);
                        }
                    }
                }
            }

            // 2. Propagate axons (dendritic signal propagation)
            // Parallel execution over physical chunk blocks using Rayon
            use rayon::prelude::*;
            axon_heads.par_chunks_exact_mut(2).for_each(|chunk| {
                for head in chunk {
                    head.h0 = head.h0.wrapping_add(cmd.v_seg * ((head.h0 != 0x80000000) as u32));
                    head.h1 = head.h1.wrapping_add(cmd.v_seg * ((head.h1 != 0x80000000) as u32));
                    head.h2 = head.h2.wrapping_add(cmd.v_seg * ((head.h2 != 0x80000000) as u32));
                    head.h3 = head.h3.wrapping_add(cmd.v_seg * ((head.h3 != 0x80000000) as u32));
                    head.h4 = head.h4.wrapping_add(cmd.v_seg * ((head.h4 != 0x80000000) as u32));
                    head.h5 = head.h5.wrapping_add(cmd.v_seg * ((head.h5 != 0x80000000) as u32));
                    head.h6 = head.h6.wrapping_add(cmd.v_seg * ((head.h6 != 0x80000000) as u32));
                    head.h7 = head.h7.wrapping_add(cmd.v_seg * ((head.h7 != 0x80000000) as u32));
                }
            });

            // 3. Update GLIF neurons
            let raw_voltages = voltage.as_mut_ptr() as usize;
            let raw_flags = flags.as_mut_ptr() as usize;
            let raw_thresh = threshold_offset.as_mut_ptr() as usize;
            let raw_timers = timers.as_mut_ptr() as usize;
            let raw_soma_to_axon = soma_to_axon.as_ptr() as usize;
            let raw_targets = dendrite_targets.as_ptr() as usize;
            let raw_weights = dendrite_weights.as_ptr() as usize;
            let raw_dtimers = dendrite_timers.as_mut_ptr() as usize;
            let raw_axon_heads = axon_heads.as_mut_ptr() as usize;
            let raw_variants = variant_params.as_ptr() as usize;
            let axon_heads_len = axon_heads.len();
            (0..padded_n).into_par_iter().for_each(|tid| {
                unsafe {
                    let flags_ptr = (raw_flags as *mut u8).add(tid);
                    let mut flag = *flags_ptr;
                    let var_id = types::extract_variant_id(flag);
                    let p = &*(raw_variants as *const layout::VariantParameters).add(var_id);

                    let timer_ptr = (raw_timers as *mut u8).add(tid);
                    let timer = *timer_ptr;

                    // Clear last tick's spike flag
                    flag &= !types::FLAG_IS_SPIKING;

                    if timer > 0 {
                        *timer_ptr = timer - 1;
                        *flags_ptr = flag;
                        return;
                    }

                    let voltage_ptr = (raw_voltages as *mut i32).add(tid);
                    let mut current_voltage = *voltage_ptr;
                    let mut i_in = 0;
                    let prop = p.signal_propagation_length as u32;

                    // Synapse activation gather loop
                    for slot in 0..128 {
                        let col_idx = slot * padded_n + tid;
                        let target_packed = *(raw_targets as *const u32).add(col_idx);
                        let target = types::PackedTarget(target_packed);
                        if target.0 == 0 {
                            break;
                        }

                        let d_timer_ptr = (raw_dtimers as *mut u8).add(col_idx);
                        if *d_timer_ptr > 0 {
                            *d_timer_ptr -= 1;
                            continue;
                        }

                        let axon_id = target.axon_id() as usize;
                        let seg_idx = target.segment_offset();

                        if axon_id >= axon_heads_len {
                            continue;
                        }
                        let h = *(raw_axon_heads as *const layout::BurstHeads8).add(axon_id);
                        let prop_len = prop + 1;
                        let hit = (physics::tail::is_in_active_tail(h.h0, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h1, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h2, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h3, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h4, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h5, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h6, seg_idx, prop_len) as i32)
                            | (physics::tail::is_in_active_tail(h.h7, seg_idx, prop_len) as i32);

                        if hit != 0 {
                            let weight = *(raw_weights as *const i32).add(col_idx);
                            i_in += weight >> 16;
                            *d_timer_ptr = p.synapse_refractory_period;
                        }
                    }

                    // Homeostasis decay
                    let t_off_ptr = (raw_thresh as *mut i32).add(tid);
                    let mut thresh_offset = *t_off_ptr;
                    thresh_offset = physics::glif::update_homeostasis(thresh_offset, p.homeostasis_decay, false, 0);

                    // Adaptive leak
                    let mut current_shift = p.leak_shift;
                    if p.adaptive_mode == 1 {
                        let adaptive_sub = (thresh_offset * (p.adaptive_leak_gain as i32)) >> 8;
                        let mut new_shift = (current_shift as i32) - adaptive_sub;
                        let lower_bound = p.adaptive_leak_min_shift;
                        if new_shift < lower_bound {
                            new_shift = lower_bound;
                        }
                        current_shift = if new_shift < 0 { 0 } else { new_shift as u32 };
                    }

                    // GLIF membrane dynamics integration and leak
                    current_voltage = physics::glif::compute_glif(current_voltage, p.rest_potential, current_shift, i_in);

                    let eff_thresh = p.threshold + thresh_offset;
                    let is_glif_spiking = (current_voltage >= eff_thresh) as i32;

                    // Heartbeat firing generator
                    let phase = ((current_tick as u64) * (p.heartbeat_m as u64) + (tid as u64) * 104729) & 0xFFFF;
                    let is_heartbeat = if p.heartbeat_m > 0 && phase < (p.heartbeat_m as u64) {
                        1
                    } else {
                        0
                    };

                    let final_spike = is_glif_spiking | is_heartbeat;

                    // AHP and reset
                    let reset_v = p.rest_potential - (p.ahp_amplitude as i32);
                    current_voltage = final_spike * reset_v + (1 - final_spike) * current_voltage;
                    thresh_offset += final_spike * p.homeostasis_penalty;
                    *timer_ptr = (final_spike * p.refractory_period as i32 + (1 - final_spike) * timer as i32) as u8;

                    // Axon burst trigger
                    if final_spike != 0 {
                        let my_axon = *(raw_soma_to_axon as *const u32).add(tid);
                        if my_axon != 0xFFFFFFFF && (my_axon as usize) < axon_heads_len {
                            let h_ptr = (raw_axon_heads as *mut layout::BurstHeads8).add(my_axon as usize);
                            let mut h = *h_ptr;
                            h.h7 = h.h6;
                            h.h6 = h.h5;
                            h.h5 = h.h4;
                            h.h4 = h.h3;
                            h.h3 = h.h2;
                            h.h2 = h.h1;
                            h.h1 = h.h0;
                            h.h0 = physics::tail::initial_axon_head(cmd.v_seg);
                            *h_ptr = h;
                        }
                    }

                    *voltage_ptr = current_voltage;
                    *t_off_ptr = thresh_offset;

                    // Plasticity state tracking
                    let mut sf = types::SomaFlags(flag);
                    let mut burst_count = sf.burst_count();
                    burst_count = (final_spike as u8) * (burst_count + (burst_count < 7) as u8);
                    sf = sf.with_burst_count(burst_count).with_spiking(final_spike != 0);
                    flag = sf.0;
                    *flags_ptr = flag;
                }
            });

            // 4. Record outputs for the current tick
            if cmd.num_outputs > 0 {
                let tick_offset = tick_idx as usize * cmd.num_outputs as usize;
                let mut history = resource.output_history.lock().unwrap();
                for i in 0..cmd.num_outputs as usize {
                    let soma_id = cmd.mapped_soma_ids.get(i).copied().unwrap_or(0xFFFF_FFFF);
                    if soma_id != 0xFFFF_FFFF && (soma_id as usize) < padded_n {
                        let flag = flags[soma_id as usize];
                        if let Some(out) = history.get_mut(tick_offset + i) {
                            *out = types::SomaFlags(flag).is_spiking() as u8;
                        }
                    } else if soma_id == 0xFFFF_FFFF {
                        // Fallback: if no mapping is provided, route neuron i % padded_n to outputs
                        // to ensure at least some raw spikes reach the client (INV-COMPUTE-CPU-005)
                        let fallback_neuron = i % padded_n;
                        let flag = flags[fallback_neuron];
                        if let Some(out) = history.get_mut(tick_offset + i) {
                            *out = types::SomaFlags(flag).is_spiking() as u8;
                        }
                    }
                }
            }
        }

        // Apply GSOP synaptic plasticity rules at the end of the batch
        if cmd.global_dopamine != 0 {
            let raw_flags = flags.as_mut_ptr() as usize;
            let raw_variants = variant_params.as_ptr() as usize;
            let raw_dtimers = dendrite_timers.as_mut_ptr() as usize;
            let raw_targets = dendrite_targets.as_ptr() as usize;
            let raw_weights = dendrite_weights.as_mut_ptr() as usize;
            let raw_axon_heads = axon_heads.as_mut_ptr() as usize;
            let axon_heads_len = axon_heads.len();
            (0..padded_n).into_par_iter().for_each(|tid| {
                unsafe {
                    let flags = *(raw_flags as *const u8).add(tid);
                    let sf = types::SomaFlags(flags);
                    if !sf.is_spiking() {
                        return;
                    }

                    let burst_count = sf.burst_count();
                    let burst_mult = if burst_count > 0 { burst_count as i32 } else { 1 };

                    let var_id = types::extract_variant_id(flags);
                    let p = &*(raw_variants as *const layout::VariantParameters).add(var_id);

                    for slot in 0..128 {
                        let col_idx = slot * padded_n + tid;

                        let timer = *(raw_dtimers as *const u8).add(col_idx);
                        if timer > 0 {
                            continue;
                        }

                        let target_packed = *(raw_targets as *const u32).add(col_idx);
                        if target_packed == 0 {
                            break;
                        }

                        let weight_ptr = (raw_weights as *mut i32).add(col_idx);
                        let w = *weight_ptr;
                        if w == 0 {
                            continue;
                        }

                        let target = types::PackedTarget(target_packed);
                        let seg_idx = target.segment_offset();
                        let axon_id = target.axon_id() as usize;
                        if axon_id >= axon_heads_len {
                            continue;
                        }
                        let h = *(raw_axon_heads as *const layout::BurstHeads8).add(axon_id);
                        let prop = p.signal_propagation_length as u32;

                        let prop_len = prop + 1;
                        let is_active = physics::tail::is_in_active_tail(h.h0, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h1, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h2, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h3, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h4, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h5, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h6, seg_idx, prop_len)
                            || physics::tail::is_in_active_tail(h.h7, seg_idx, prop_len);

                        let rank = physics::gsop::inertia_rank(w);
                        let inertia = p.inertia_curve[rank] as i32;

                        let new_w = physics::gsop::compute_gsop_weight(
                            w,
                            cmd.global_dopamine,
                            p.d1_affinity,
                            p.d2_affinity,
                            p.gsop_potentiation,
                            p.gsop_depression,
                            inertia,
                            is_active,
                            burst_mult,
                            0,
                        );

                        *weight_ptr = new_w;
                    }
                }
            });
        }

        Ok(BatchResult {
            ticks_processed: cmd.sync_batch_ticks,
            is_warmup: false,
        })
    }

    fn download_output(&self, handle: &VramHandle) -> Result<OutputFrame, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        let num_outputs = resource.last_num_outputs.load(std::sync::atomic::Ordering::Relaxed);
        let sync_batch_ticks = resource.last_sync_batch_ticks.load(std::sync::atomic::Ordering::Relaxed);

        if num_outputs == 0 || sync_batch_ticks == 0 {
            return Ok(OutputFrame {
                data: vec![],
                num_outputs: 0,
                sync_batch_ticks: 0,
            });
        }

        let history = resource.output_history.lock().unwrap();
        Ok(OutputFrame {
            data: history.clone(),
            num_outputs,
            sync_batch_ticks,
        })
    }

    fn download_telemetry(&self, handle: &VramHandle) -> Result<TelemetryFrame, ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Download telemetry
        Ok(TelemetryFrame {
            active_soma_ids: vec![],
            total_spikes: 0,
        })
    }

    fn patch_ghosts(&self, handle: &VramHandle, patches: &[GhostPatch]) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // INV-COMPUTE-API-006, E-063: Check ghost capacity bounds
        for patch in patches {
            match patch {
                GhostPatch::Add { dst_ghost, .. } | GhostPatch::Prune { dst_ghost } => {
                    if *dst_ghost >= resource.layout.total_ghosts {
                        return Err(ComputeApiError::CapacityExceeded);
                    }
                }
            }
        }

        Ok(())
    }

    fn run_sort_and_prune(&self, handle: &VramHandle, _prune_threshold: i16) -> Result<(), ComputeApiError> {
        let key = handle_to_key(handle);
        let guard = self.resources.read().map_err(|_| ComputeApiError::DeviceLost)?;
        let _resource = guard.get(key).ok_or(ComputeApiError::InvalidHandle)?;

        // Stub: Run sort and prune
        Ok(())
    }

    fn free(&self, handle: VramHandle) {
        let key = handle_to_key(&handle);
        if let Ok(mut guard) = self.resources.write() {
            guard.remove(key);
        }
    }
}
