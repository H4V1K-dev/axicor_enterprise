// Старая версия от хх.05.2026


use rayon::prelude::*;
use genesis_core::layout::BurstHeads8;
use genesis_core::constants::AXON_SENTINEL;
use crate::ffi::ShardVramPtrs;

// =============================================================================
// §2.1 cpu_propagate_axons
// =============================================================================

/// Ядро 3: Безусловный сдвиг голов всех аксонов.
/// DOD FIX: Математика Branchless (vpaddd) для AVX2 векторизации.
pub fn cpu_propagate_axons(
    axon_heads: &mut [BurstHeads8],
    v_seg: u32,
) {
    axon_heads.par_chunks_exact_mut(2).for_each(|chunk| {
        for head in chunk {
            head.h0 = head.h0.wrapping_add(v_seg * ((head.h0 != AXON_SENTINEL) as u32));
            head.h1 = head.h1.wrapping_add(v_seg * ((head.h1 != AXON_SENTINEL) as u32));
            head.h2 = head.h2.wrapping_add(v_seg * ((head.h2 != AXON_SENTINEL) as u32));
            head.h3 = head.h3.wrapping_add(v_seg * ((head.h3 != AXON_SENTINEL) as u32));
            head.h4 = head.h4.wrapping_add(v_seg * ((head.h4 != AXON_SENTINEL) as u32));
            head.h5 = head.h5.wrapping_add(v_seg * ((head.h5 != AXON_SENTINEL) as u32));
            head.h6 = head.h6.wrapping_add(v_seg * ((head.h6 != AXON_SENTINEL) as u32));
            head.h7 = head.h7.wrapping_add(v_seg * ((head.h7 != AXON_SENTINEL) as u32));
        }
    });
}

// =============================================================================
// §2.2 cpu_apply_spike_batch
// =============================================================================

/// Ядро 2: Инъекция сетевых спайков.
/// DOD FIX: Burst-сдвиг (имитация сдвигового регистра) + Темпоральный синхронизм.
/// Убран Rayon — линейный проход в L1 кэше дешевле Work-Stealing оверхеда.
pub fn cpu_apply_spike_batch(
    axon_heads: &mut [BurstHeads8],
    schedule_indices: &[u32],
    v_seg: u32,
) {
    for &ghost_id in schedule_indices {
        if let Some(head) = axon_heads.get_mut(ghost_id as usize) {
            // Аппаратный сдвиг голов (Spec 01 §1.4.3)
            head.h7 = head.h6;
            head.h6 = head.h5;
            head.h5 = head.h4;
            head.h4 = head.h3;
            head.h3 = head.h2;
            head.h2 = head.h1;
            head.h1 = head.h0;
            // Инициализация h0 с поправкой на темпоральный синхронизм
            head.h0 = 0u32.wrapping_sub(v_seg); 
        }
    }
}

// =============================================================================
// §2.3 cpu_inject_inputs
// =============================================================================

/// Ядро 1: Инъекция сенсорных данных (Виртуальные аксоны).
/// DOD FIX: SIMD-friendly битмаск-сканирование.
pub fn cpu_inject_inputs(
    axon_heads: &mut [BurstHeads8],
    input_bitmask: &[u32],
    virtual_offset: u32,
    num_virtual_axons: u32,
    v_seg: u32,
) {
    for tid in 0..num_virtual_axons as usize {
        let word_idx = tid / 32;
        let bit_idx = tid % 32;
        // Извлекаем бит
        if (input_bitmask[word_idx] >> bit_idx) & 1 != 0 {
            if let Some(head) = axon_heads.get_mut(virtual_offset as usize + tid) {
                head.h7 = head.h6;
                head.h6 = head.h5;
                head.h5 = head.h4;
                head.h4 = head.h3;
                head.h3 = head.h2;
                head.h2 = head.h1;
                head.h1 = head.h0;
                head.h0 = 0u32.wrapping_sub(v_seg);
            }
        }
    }
}

// =============================================================================
// §2.4 cpu_record_outputs
// =============================================================================

/// Ядро 6: Вывод активности сом (RecordReadout).
/// DOD FIX: Безусловная запись для предотвращения "эффекта грязного буфера".
/// Добавлена проверка на 0xFFFF_FFFF (пустой пиксель).
pub fn cpu_record_outputs(
    soma_flags: &[u8],
    mapped_soma_ids: &[u32],
    output_history: &mut [u8],
    current_tick: u32,
    total_mapped_somas: u32,
) {
    let tick_offset = (current_tick as usize) * (total_mapped_somas as usize);
    for (i, &soma_id) in mapped_soma_ids.iter().enumerate() {
        // Защита от EMPTY_PIXEL (0xFFFF_FFFF)
        if soma_id != 0xFFFF_FFFF {
            if let Some(&flag) = soma_flags.get(soma_id as usize) {
                if let Some(out) = output_history.get_mut(tick_offset + i) {
                    // Безусловная запись 0 или 1 (LTM/WM state)
                    *out = flag & 0x01;
                }
            }
        }
    }
}

// =============================================================================
// §2.4 cpu_update_neurons (The Hot Loop)
// =============================================================================

/// Ядро 4: Интеграция GLIF, дендритное дерево и гомеостаз.
/// DOD FIX: Raw pointer index iteration (Zero-Cost). Branchless математика.
pub unsafe fn cpu_update_neurons(
    ptrs: &ShardVramPtrs,
    padded_n: u32,
    current_tick: u32,
    v_seg: u32,
) {
    use crate::bindings::VARIANT_LUT;

    (0..padded_n as usize).into_par_iter().for_each(|tid| {
        // 1. Распаковка типа + загрузка параметров (1 такт L1)
        let flags_ptr = ptrs.soma_flags.add(tid);
        let mut flag = *flags_ptr;
        let var_id = (flag >> 4) & 0x0F;
        let p = &VARIANT_LUT.variants[var_id as usize];

        let timer_ptr = ptrs.timers.add(tid);
        let timer = *timer_ptr;

        flag &= !0x01; // Очищаем флаг спайка

        // 2. Рефрактерность сомы - Early Exit (~90% тиков)
        if timer > 0 {
            *timer_ptr = timer - 1;
            *flags_ptr = flag;
            return;
        }

        let mut current_voltage = *ptrs.soma_voltage.add(tid);
        let mut i_in = 0;
        let prop = p.signal_propagation_length as u32;

        // 3. Columnar Dendrite Loop: 128 слотов (Coalesced Access / Gather)
        for slot in 0..128 {
            let col_idx = slot * (padded_n as usize) + tid;
            let target_packed = *ptrs.dendrite_targets.add(col_idx);

            // Hardware Trap: Пустой слот означает конец активных связей
            if target_packed == 0 { break; } 

            let d_timer_ptr = ptrs.dendrite_timers.add(col_idx);
            if *d_timer_ptr > 0 {
                *d_timer_ptr -= 1;
                continue;
            }

            let axon_id = (target_packed & 0x00FFFFFF).saturating_sub(1);
            let seg_idx = target_packed >> 24;

            let h = *ptrs.axon_heads.add(axon_id as usize);

            // Branchless 8-head Hit Detection (Без jmp/br инструкций)
            let hit = ((h.h0.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h1.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h2.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h3.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h4.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h5.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h6.wrapping_sub(seg_idx) <= prop) as i32) |
                      ((h.h7.wrapping_sub(seg_idx) <= prop) as i32);

            if hit != 0 {
                let weight = *ptrs.dendrite_weights.add(col_idx);
                // Shift Mass Domain (i32) to Charge Domain (микровольты)
                i_in += weight >> 16; 
                *d_timer_ptr = p.synapse_refractory_period;
            }
        }

        // 4. Гомеостаз (Soft Limit)
        let t_off_ptr = ptrs.threshold_offset.add(tid);
        let mut thresh_offset = *t_off_ptr;
        let decayed = thresh_offset - p.homeostasis_decay as i32;
        thresh_offset = decayed & !(decayed >> 31); // Branchless max(0, val)

        // 5. GLIF Утечка (Branchless clamp)
        let diff = current_voltage - p.rest_potential;
        let sign = (diff > 0) as i32 - (diff < 0) as i32;
        let abs_mask = diff >> 31;
        let mut leaked_abs = (diff ^ abs_mask).wrapping_sub(abs_mask) - p.leak_rate;
        leaked_abs &= !(leaked_abs >> 31);
        current_voltage = p.rest_potential + (sign * leaked_abs);

        current_voltage += i_in;

        let eff_thresh = p.threshold + thresh_offset;
        let is_glif_spiking = (current_voltage >= eff_thresh) as i32;

        // 6. Спонтанная активность (Heartbeat DDS)
        let period = p.spontaneous_firing_period_ticks;
        let is_heartbeat = if period > 0 && ((current_tick + (tid as u32 * 104729)) % period) == 0 { 1 } else { 0 };

        let final_spike = is_glif_spiking | is_heartbeat;

        // 7. Сброс мембраны и таймеров
        current_voltage = final_spike * p.rest_potential + (1 - final_spike) * current_voltage;
        thresh_offset += final_spike * p.homeostasis_penalty;
        *timer_ptr = (final_spike * p.refractory_period as i32 + (1 - final_spike) * timer as i32) as u8;

        // 8. Выстрел аксона (Burst Shift)
        if final_spike != 0 {
            let my_axon = *ptrs.soma_to_axon.add(tid);
            if my_axon != 0xFFFFFFFF {
                let h_ptr = ptrs.axon_heads.add(my_axon as usize);
                let mut h = *h_ptr;
                h.h7 = h.h6;
                h.h6 = h.h5;
                h.h5 = h.h4;
                h.h4 = h.h3;
                h.h3 = h.h2;
                h.h2 = h.h1;
                h.h1 = h.h0;
                h.h0 = 0u32.wrapping_sub(v_seg);
                *h_ptr = h;
            }
        }

        // 9. Запись обратно в VRAM (Zero-Warp Divergence pattern)
        *ptrs.soma_voltage.add(tid) = current_voltage;
        *t_off_ptr = thresh_offset;

        // 10. Burst-Dependent Plasticity (BDP) счетчик
        let mut burst_count = (flag >> 1) & 0x07;
        burst_count = (final_spike as u8) * (burst_count + (burst_count < 7) as u8);
        flag = (flag & 0xF0) | (burst_count << 1) | (final_spike as u8);
        *flags_ptr = flag;
    });
}

// =============================================================================
// §2.5 cpu_apply_gsop
// =============================================================================

/// Ядро 5: Пластичность GSOP.
/// DOD FIX: Branchless-аппроксимация STDP. Zero-Warp Divergence.
pub unsafe fn cpu_apply_gsop(
    ptrs: &ShardVramPtrs,
    padded_n: u32,
    dopamine: i16,
) {
    use crate::bindings::VARIANT_LUT;

    (0..padded_n as usize).into_par_iter().for_each(|tid| {
        let flags = *ptrs.soma_flags.add(tid);
        
        // Early Exit: Если нейрон не спайкнул, пластичность не применяется
        if (flags & 0x01) == 0 { return; }

        let burst_count = (flags >> 1) & 0x07;
        let burst_mult = if burst_count > 0 { burst_count as i32 } else { 1 };

        let var_id = (flags >> 4) & 0x0F;
        let p = &VARIANT_LUT.variants[var_id as usize];

        for slot in 0..128 {
            let col_idx = slot * (padded_n as usize) + tid;

            let timer = *ptrs.dendrite_timers.add(col_idx);
            if timer > 0 {
                // Если таймер > 0, мы декрементировали его в UpdateNeurons.
                // Синапс спит, пропускаем тяжелую математику.
                continue; 
            }

            let target_packed = *ptrs.dendrite_targets.add(col_idx);
            if target_packed == 0 { break; } // Hardware Trap: конец живых связей

            let weight_ptr = ptrs.dendrite_weights.add(col_idx);
            let w = *weight_ptr;
            if w == 0 { continue; } // Мертвая связь (ждет Night Phase Pruning)

            let seg_idx = target_packed >> 24;
            let axon_id = (target_packed & 0x00FFFFFF).saturating_sub(1);
            let h = *ptrs.axon_heads.add(axon_id as usize);
            let prop = p.signal_propagation_length as u32;

            // Branchless 8-head Hit Detection
            let is_active = ((h.h0.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h1.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h2.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h3.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h4.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h5.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h6.wrapping_sub(seg_idx) <= prop) as i32) |
                            ((h.h7.wrapping_sub(seg_idx) <= prop) as i32);

            let sign = if w >= 0 { 1 } else { -1 };
            let abs_w = w.abs();

            // 1. Inertia Rank (1 такт, Branchless)
            let mut rank = (abs_w >> 27) as usize;
            if rank > 15 { rank = 15; }
            let inertia = p.inertia_curve[rank] as i32;

            // 2. Dopamine modulation (D1 boosts LTP, D2 suppresses LTD on reward)
            let pot_mod = ((dopamine as i32) * (p.d1_affinity as i32)) >> 7;
            let dep_mod = ((dopamine as i32) * (p.d2_affinity as i32)) >> 7;

            let raw_pot = (p.gsop_potentiation as i32) + pot_mod;
            let raw_dep = (p.gsop_depression as i32) - dep_mod;

            let final_pot = raw_pot & !(raw_pot >> 31); // max(0, val)
            let final_dep = raw_dep & !(raw_dep >> 31); // max(0, val)

            // Умножение ДО сдвига строго как в CUDA-эквиваленте
            let delta_pot = (final_pot * inertia * burst_mult) >> 7;
            let delta_dep = (final_dep * inertia * burst_mult) >> 7;

            // Causal Delta. Если спайк сомы совпал с Active Tail (is_active) -> LTP, иначе LTD.
            let mut delta = if is_active != 0 { delta_pot } else { -delta_dep };
            
            // Fixed Slot Decay = 1.0x
            delta = (delta * 128) >> 7;

            // 3. Apply & Clamp to Mass Domain Limits
            let mut new_abs = abs_w + delta;
            new_abs &= !(new_abs >> 31); // Branchless clamp bottom to 0
            if new_abs > 2140000000 { new_abs = 2140000000; } // Headroom guard

            *weight_ptr = new_abs * sign;
        }
    });
}

// =============================================================================
// §2.6 cpu_extract_telemetry
// =============================================================================

/// Ядро 7: Сбор спайков (Телеметрия).
/// DOD FIX: Строго последовательный проход. LLVM векторизует это в SIMD (pmovmskb),
/// что в L1/L2 кэше работает в разы быстрее, чем оверхед Rayon-планировщика и Atomics.
pub fn cpu_extract_telemetry(
    soma_flags: &[u8],
    out_ids: &mut [u32],
) -> u32 {
    let mut count = 0;
    
    // Никаких iter_mut или chunking. Чистое чтение + линейная запись.
    for (id, &flag) in soma_flags.iter().enumerate() {
        if (flag & 0x01) != 0 {
            // Защита от переполнения буфера телеметрии
            if let Some(slot) = out_ids.get_mut(count) {
                *slot = id as u32;
                count += 1;
            }
        }
    }
    
    count as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::cpu_allocate_shard;
    use crate::bindings::cpu_free_shard;
    use crate::bindings::VARIANT_LUT;
    use crate::bindings::TEST_MUTEX;
    use crate::ffi::ShardVramPtrs;
    use genesis_core::layout::VariantParameters;
    
    #[test]
    fn test_propagate_axons() {
        let mut heads = vec![BurstHeads8::empty(AXON_SENTINEL); 4];
        heads[0].h0 = 10;
        heads[1].h7 = 20;
        
        cpu_propagate_axons(&mut heads, 5);
        
        assert_eq!(heads[0].h0, 15);
        assert_eq!(heads[0].h1, AXON_SENTINEL);
        assert_eq!(heads[1].h7, 25);
    }

    #[test]
    fn test_burst_shift_spike() {
        let mut heads = vec![BurstHeads8::empty(AXON_SENTINEL); 1];
        heads[0].h0 = 100; // старый спайк
        
        cpu_apply_spike_batch(&mut heads, &[0], 5);
        
        assert_eq!(heads[0].h1, 100); // сдвинулся
        assert_eq!(heads[0].h0, 0u32.wrapping_sub(5)); // новый инициализирован
    }

    #[test]
    fn test_record_outputs_unconditional() {
        let flags = vec![0x00, 0x01, 0x00, 0x01];
        let mapped_ids = vec![1, 3];
        let mut history = vec![255; 4]; // Грязный буфер
        
        cpu_record_outputs(&flags, &mapped_ids, &mut history, 0, 2);
        assert_eq!(history[0], 1);
        assert_eq!(history[1], 1);
        
        // Теперь нейрон 1 выключен
        let flags_new = vec![0x00, 0x00, 0x00, 0x01];
        cpu_record_outputs(&flags_new, &mapped_ids, &mut history, 1, 2);
        assert_eq!(history[2], 0); // Должно быть 0, а не 255
        assert_eq!(history[3], 1);
    }

    #[test]
    fn test_update_neurons_spiking() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut ptrs: ShardVramPtrs = unsafe { std::mem::zeroed() };
        let padded_n = 64;
        let axons = 10;
        unsafe {
            cpu_allocate_shard(padded_n, axons, &mut ptrs);
            
            // Настройка VARIANT_LUT для типа 0
            let mut p = VariantParameters::default();
            p.threshold = 100;
            p.rest_potential = 0;
            p.leak_rate = 0;
            p.refractory_period = 5;
            p.homeostasis_penalty = 50;
            
            VARIANT_LUT.variants[0] = p;
            
            // Нейрон 0: Voltage = 150 (должен спайкнуть)
            *ptrs.soma_voltage.add(0) = 150;
            *ptrs.soma_flags.add(0) = 0 << 4; // Type 0
            *ptrs.soma_to_axon.add(0) = 0;    // Axon 0
            
            // Тик 1
            cpu_update_neurons(&ptrs, padded_n, 1, 1);
            
            // Проверка спайка
            assert_eq!((*ptrs.soma_flags.add(0)) & 0x01, 1, "Neuron 0 must spike");
            assert_eq!((*ptrs.soma_voltage.add(0)), 0, "Voltage must reset to rest_potential");
            assert_eq!((*ptrs.timers.add(0)), 5, "Refractory timer must be set");
            assert_eq!((*ptrs.threshold_offset.add(0)), 50, "Homeostasis penalty applied");
            
            // Проверка выстрела аксона
            let h = *ptrs.axon_heads.add(0);
            assert_eq!(h.h0, 0u32.wrapping_sub(1), "Axon head h0 must be initialized with temporal sync");
            
            cpu_free_shard(&mut ptrs);
        }
    }

    #[test]
    fn test_apply_gsop_potentiation() {
        let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut ptrs: ShardVramPtrs = unsafe { std::mem::zeroed() };
        let padded_n = 64;
        let axons = 10;
        unsafe {
            cpu_allocate_shard(padded_n, axons, &mut ptrs);
            
            let mut p = VariantParameters::default();
            p.gsop_potentiation = 1000;
            p.gsop_depression = 500;
            p.d1_affinity = 128; // 1.0x
            p.inertia_curve[0] = 128; // 1.0x
            p.signal_propagation_length = 5;
            VARIANT_LUT.variants[0] = p;
            
            // Нейрон 0 спайкнул
            *ptrs.soma_flags.add(0) = (0 << 4) | 0x01;
            
            // Синапс 0 в слоте 0: вес 1000 (Mass Domain)
            let old_w_full = 1000 << 16;
            *ptrs.dendrite_weights.add(0) = old_w_full;
            // Цель: Axon 1, сегмент 0
            *ptrs.dendrite_targets.add(0) = (0 << 24) | 2; // saturating_sub(1) -> axon 1
            
            // Axon 1 имеет спайк на h0 = 0
            (*ptrs.axon_heads.add(1)).h0 = 0;
            
            // Apply GSOP с дофамином +200
            cpu_apply_gsop(&ptrs, padded_n, 200);
            
            let new_w_full = *ptrs.dendrite_weights.add(0);
            assert!(new_w_full > old_w_full, "Weight should increase (LTP). New: {}, Old: {}", new_w_full, old_w_full);
            
            cpu_free_shard(&mut ptrs);
        }
    }

    #[test]
    fn test_extract_telemetry() {
        let mut flags = vec![0u8; 10000];
        // Ставим рандомный мусор в другие биты
        for i in 0..10000 { flags[i] = ((i % 15) as u8) << 4; }
        
        flags[42] |= 0x01;
        flags[1337] |= 0x01;
        flags[9999] |= 0x01;
        
        let mut out_ids = vec![0u32; 10000];
        let count = cpu_extract_telemetry(&flags, &mut out_ids);
        
        assert_eq!(count, 3);
        assert_eq!(out_ids[0], 42);
        assert_eq!(out_ids[1], 1337);
        assert_eq!(out_ids[2], 9999);
    }
}