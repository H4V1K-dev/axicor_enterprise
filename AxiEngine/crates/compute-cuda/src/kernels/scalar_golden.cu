#include <cuda_runtime.h>
#include <stdio.h>
#include "axi_cuda_abi.h"

static_assert(AXI_SIZE_AxonsFileHeader == 16, "AXI_SIZE_AxonsFileHeader must be exactly 16 bytes");
static_assert(AXI_SIZE_BurstHeads8 % sizeof(unsigned int) == 0, "AXI_SIZE_BurstHeads8 must be a multiple of sizeof(unsigned int)");
static_assert(AXI_SIZE_BurstHeads8 / sizeof(unsigned int) == 8, "AXI_SIZE_BurstHeads8 must represent exactly 8 heads");

__constant__ unsigned char axi_variant_table_bytes[AXI_SIZE_VariantParameters * AXI_VARIANT_LUT_LEN];

// Helper functions to read from constant table.
// The constant table is stored as Rust POD little-endian bytes.
__device__ unsigned char read_variant_u8(unsigned int variant_idx, unsigned int field_offset) {
    size_t base = (size_t)variant_idx * AXI_SIZE_VariantParameters + field_offset;
    return axi_variant_table_bytes[base];
}
__device__ unsigned short read_variant_u16(unsigned int variant_idx, unsigned int field_offset) {
    size_t base = (size_t)variant_idx * AXI_SIZE_VariantParameters + field_offset;
    unsigned short val = 0;
    val |= (unsigned short)axi_variant_table_bytes[base + 0];
    val |= (unsigned short)axi_variant_table_bytes[base + 1] << 8;
    return val;
}
__device__ unsigned int read_variant_u32(unsigned int variant_idx, unsigned int field_offset) {
    size_t base = (size_t)variant_idx * AXI_SIZE_VariantParameters + field_offset;
    unsigned int val = 0;
    val |= (unsigned int)axi_variant_table_bytes[base + 0];
    val |= (unsigned int)axi_variant_table_bytes[base + 1] << 8;
    val |= (unsigned int)axi_variant_table_bytes[base + 2] << 16;
    val |= (unsigned int)axi_variant_table_bytes[base + 3] << 24;
    return val;
}
__device__ int read_variant_i32(unsigned int variant_idx, unsigned int field_offset) {
    unsigned int val = read_variant_u32(variant_idx, field_offset);
    return (int)val;
}

#define AXI_HEADS_PER_BURST (AXI_SIZE_BurstHeads8 / sizeof(unsigned int))

// Scalar GPU kernels
__global__ void propagate_head_kernel(unsigned int head, unsigned int v_seg, unsigned int* out) {
    bool is_active = (head ^ AXI_AXON_SENTINEL) >= v_seg;
    unsigned int mask = 0u - (unsigned int)is_active;
    *out = ((head + v_seg) & mask) | (AXI_AXON_SENTINEL & ~mask);
}

__global__ void active_tail_hit_kernel(unsigned int head, unsigned int seg_idx, unsigned int propagation_length, unsigned char* out) {
    unsigned int d = head - seg_idx;
    *out = (d < propagation_length) ? 1 : 0;
}

__global__ void propagate_uploaded_axons_kernel(unsigned int* heads, unsigned int total_heads, unsigned int v_seg) {
    unsigned int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < total_heads) {
        unsigned int head = heads[idx];
        bool is_active = (head ^ AXI_AXON_SENTINEL) >= v_seg;
        unsigned int mask = 0u - (unsigned int)is_active;
        heads[idx] = ((head + v_seg) & mask) | (AXI_AXON_SENTINEL & ~mask);
    }
}

__global__ void inject_and_propagate_axons_tick_kernel(
    unsigned int* heads,
    unsigned int total_axons,
    unsigned int v_seg,
    unsigned int shard_virtual_offset,
    unsigned int cmd_virtual_offset,
    unsigned int num_virtual_axons,
    const unsigned int* input_bitmask,
    unsigned int input_words_len,
    const unsigned int* incoming_spikes,
    unsigned int incoming_spikes_count
) {
    unsigned int a = blockIdx.x * blockDim.x + threadIdx.x;
    if (a < total_axons) {
        // 1. Virtual input check
        unsigned int virtual_injections = 0;
        unsigned long long global_axon_id = (unsigned long long)shard_virtual_offset + (unsigned long long)a;
        if (input_bitmask && global_axon_id >= (unsigned long long)cmd_virtual_offset) {
            unsigned long long k = global_axon_id - (unsigned long long)cmd_virtual_offset;
            if (k < (unsigned long long)num_virtual_axons) {
                unsigned long long word_idx = k / 32;
                unsigned int bit_idx = (unsigned int)(k % 32);
                if (word_idx < (unsigned long long)input_words_len && (input_bitmask[word_idx] & (1u << bit_idx)) != 0) {
                    virtual_injections = 1;
                }
            }
        }

        // 2. Incoming spikes check
        unsigned int incoming_injections = 0;
        if (incoming_spikes) {
            for (unsigned int s = 0; s < incoming_spikes_count; ++s) {
                if (virtual_injections + incoming_injections >= AXI_HEADS_PER_BURST) {
                    break;
                }
                if (incoming_spikes[s] == a) {
                    incoming_injections++;
                }
            }
        }

        // 3. Shift calculations
        unsigned int N = virtual_injections + incoming_injections;
        if (N > AXI_HEADS_PER_BURST) {
            N = AXI_HEADS_PER_BURST;
        }
        
        unsigned int local_heads[AXI_HEADS_PER_BURST];
        size_t base_idx = (size_t)a * AXI_HEADS_PER_BURST;
        for (unsigned int h = 0; h < AXI_HEADS_PER_BURST; ++h) {
            local_heads[h] = heads[base_idx + h];
        }

        unsigned int init_val = 0u - v_seg;
        unsigned int shifted_heads[AXI_HEADS_PER_BURST];
        for (unsigned int h = 0; h < AXI_HEADS_PER_BURST; ++h) {
            if (h < N) {
                shifted_heads[h] = init_val;
            } else {
                shifted_heads[h] = local_heads[h - N];
            }
        }

        // 4. Propagation and writeback
        for (unsigned int h = 0; h < AXI_HEADS_PER_BURST; ++h) {
            unsigned int head = shifted_heads[h];
            bool is_active = (head ^ AXI_AXON_SENTINEL) >= v_seg;
            unsigned int mask = 0u - (unsigned int)is_active;
            heads[base_idx + h] = ((head + v_seg) & mask) | (AXI_AXON_SENTINEL & ~mask);
        }
    }
}

__global__ void compute_input_current_probe_kernel(
    const void* state_ptr,
    const void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_targets,
    unsigned int off_weights,
    unsigned int off_flags,
    int* out_i_in
) {
    unsigned int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < padded_n) {
        unsigned int sum = 0;
        const unsigned int* targets = (const unsigned int*)((const char*)state_ptr + off_targets);
        const int* weights = (const int*)((const char*)state_ptr + off_weights);
        const unsigned char* flags = (const unsigned char*)((const char*)state_ptr + off_flags);
        const unsigned int* axons_heads = (const unsigned int*)((const char*)axons_ptr + AXI_SIZE_AxonsFileHeader);

        unsigned char flags_val = flags[idx];
        unsigned int variant_idx = (flags_val & AXI_SOMA_TYPE_MASK) >> AXI_SOMA_TYPE_SHIFT;
        if (variant_idx >= AXI_VARIANT_LUT_LEN) {
            variant_idx = AXI_VARIANT_LUT_LEN - 1;
        }
        unsigned int propagation_length = (unsigned int)read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_signal_propagation_length);

        for (unsigned int d = 0; d < AXI_MAX_DENDRITES; ++d) {
            size_t target_idx = (size_t)d * padded_n + idx;
            unsigned int raw = targets[target_idx];
            if (raw == 0 || raw == AXI_EMPTY_PIXEL) {
                continue;
            }

            unsigned int axon_q = raw & 0x00FFFFFF;
            if (axon_q < 1 || axon_q > AXI_MAX_AXON_ID + 1) {
                continue;
            }
            unsigned int axon_id = axon_q - 1;
            if (axon_id >= total_axons) {
                continue;
            }

            unsigned int seg_idx = (raw >> 24) & 0xFF;

            // Check active tail hits
            bool hit = false;
            const unsigned int* heads = axons_heads + (size_t)axon_id * AXI_HEADS_PER_BURST;
            for (unsigned int h = 0; h < AXI_HEADS_PER_BURST; ++h) {
                unsigned int head = heads[h];
                unsigned int d_val = head - seg_idx;
                if (d_val < propagation_length) {
                    hit = true;
                    break;
                }
            }

            if (hit) {
                int weight = weights[target_idx];
                int charge = weight >> AXI_MASS_TO_CHARGE_SHIFT;
                sum = sum + (unsigned int)charge;
            }
        }

        out_i_in[idx] = (int)sum;
    }
}

__global__ void apply_glif_membrane_probe_kernel(
    void* state_ptr,
    unsigned int padded_n,
    unsigned int off_voltage,
    unsigned int off_flags,
    unsigned int off_thresh,
    unsigned int off_timers,
    const int* i_in_device
) {
    unsigned int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < padded_n) {
        int* soma_voltage = (int*)((char*)state_ptr + off_voltage);
        unsigned char* soma_flags = (unsigned char*)((char*)state_ptr + off_flags);
        int* threshold_offset = (int*)((char*)state_ptr + off_thresh);
        unsigned char* timers = (unsigned char*)((char*)state_ptr + off_timers);

        unsigned char flags = soma_flags[idx];
        unsigned int type_id = (flags & AXI_SOMA_TYPE_MASK) >> AXI_SOMA_TYPE_SHIFT;
        unsigned int variant_idx = type_id;
        if (variant_idx >= AXI_VARIANT_LUT_LEN) {
            variant_idx = AXI_VARIANT_LUT_LEN - 1;
        }

        // Load variant parameters via generated offsets
        int threshold = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_threshold);
        int rest_potential = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_rest_potential);
        unsigned int leak_shift = read_variant_u32(variant_idx, AXI_OFFSET_VariantParameters_leak_shift);
        int homeostasis_penalty = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_homeostasis_penalty);
        int homeostasis_decay_val = (int)read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_homeostasis_decay);
        unsigned char refractory_period = read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_refractory_period);
        unsigned short ahp_amplitude = read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_ahp_amplitude);
        int adaptive_leak_min_shift = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_adaptive_leak_min_shift);
        int adaptive_leak_gain_val = (int)read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_adaptive_leak_gain);
        int adaptive_mode_val = (int)read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_adaptive_mode);

        // 1. Decay threshold offset
        int thresh_offset = threshold_offset[idx];
        int decayed = (int)((unsigned int)thresh_offset - (unsigned int)homeostasis_decay_val);
        thresh_offset = decayed & ~(decayed >> 31);
        threshold_offset[idx] = thresh_offset;

        unsigned char timer = timers[idx];
        unsigned char old_burst = (flags & AXI_SOMA_BURST_MASK) >> AXI_SOMA_BURST_SHIFT;

        if (timer > 0) {
            // Refractory period: decrement timer, voltage unchanged, no spike
            timers[idx] = timer - 1;
            // Write back flags: clear spiking (bit 0), type & burst preserved
            soma_flags[idx] = (flags & AXI_SOMA_TYPE_MASK) | (flags & AXI_SOMA_BURST_MASK);
        } else {
            // Integrate voltage
            int voltage = soma_voltage[idx];
            int i_in = i_in_device[idx];

            long long adaptive_sub = ((long long)thresh_offset * (long long)adaptive_leak_gain_val) / 256 * (long long)adaptive_mode_val;
            long long current_shift = (long long)leak_shift - adaptive_sub;
            if (current_shift < (long long)adaptive_leak_min_shift) {
                current_shift = (long long)adaptive_leak_min_shift;
            }
            if (current_shift < 0) current_shift = 0;
            if (current_shift > 63) current_shift = 63;
            unsigned int shift = (unsigned int)current_shift;

            long long v_diff = (long long)voltage - (long long)rest_potential;
            int delta_v_leak = (int)(v_diff >> shift);

            int v_new = (int)((unsigned int)voltage + (unsigned int)i_in - (unsigned int)delta_v_leak);

            int v_th_eff = (int)((unsigned int)threshold + (unsigned int)thresh_offset);
            bool is_glif = (v_new >= v_th_eff);

            if (is_glif) {
                soma_voltage[idx] = (int)((unsigned int)rest_potential - (unsigned int)ahp_amplitude);
                timers[idx] = refractory_period;
                threshold_offset[idx] = (int)((unsigned int)thresh_offset + (unsigned int)homeostasis_penalty);
                
                unsigned char new_burst = old_burst + 1;
                if (new_burst > 7) {
                    new_burst = 7;
                }
                // Spiking is true (bit 0 = 1), set burst and preserve type
                soma_flags[idx] = (flags & AXI_SOMA_TYPE_MASK) | AXI_SOMA_SPIKING_MASK | ((new_burst << AXI_SOMA_BURST_SHIFT) & AXI_SOMA_BURST_MASK);
            } else {
                soma_voltage[idx] = v_new;
                // Spiking is false (bit 0 = 0), set old burst and preserve type
                soma_flags[idx] = (flags & AXI_SOMA_TYPE_MASK) | ((old_burst << AXI_SOMA_BURST_SHIFT) & AXI_SOMA_BURST_MASK);
            }
        }
    }
}

__global__ void apply_glif_final_spike_probe_kernel(
    void* state_ptr,
    void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_voltage,
    unsigned int off_flags,
    unsigned int off_thresh,
    unsigned int off_timers,
    unsigned int off_s2a,
    const int* i_in_device,
    unsigned long long current_tick,
    unsigned int v_seg,
    const unsigned int* mapped_soma_ids,
    unsigned int num_outputs,
    unsigned int max_spikes_per_tick,
    unsigned int* output_spikes,
    unsigned int* output_count,
    unsigned int* generated_spikes_count,
    unsigned int* dropped_spikes_count
) {
    int* soma_voltage = (int*)((char*)state_ptr + off_voltage);
    unsigned char* soma_flags = (unsigned char*)((char*)state_ptr + off_flags);
    int* threshold_offset = (int*)((char*)state_ptr + off_thresh);
    unsigned char* timers = (unsigned char*)((char*)state_ptr + off_timers);
    const unsigned int* soma_to_axon = (const unsigned int*)((const char*)state_ptr + off_s2a);
    unsigned int* heads = (unsigned int*)((char*)axons_ptr + AXI_SIZE_AxonsFileHeader);

    for (unsigned int i = 0; i < padded_n; ++i) {
        unsigned char flags = soma_flags[i];
        unsigned int type_id = (flags & AXI_SOMA_TYPE_MASK) >> AXI_SOMA_TYPE_SHIFT;
        unsigned int variant_idx = type_id;
        if (variant_idx >= AXI_VARIANT_LUT_LEN) {
            variant_idx = AXI_VARIANT_LUT_LEN - 1;
        }

        // 1. Decay threshold offset
        int thresh_offset = threshold_offset[i];
        int homeostasis_decay_val = (int)read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_homeostasis_decay);
        int decayed_offset = (int)((unsigned int)thresh_offset - (unsigned int)homeostasis_decay_val);
        decayed_offset = decayed_offset & ~(decayed_offset >> 31);

        // 2. GLIF update
        int threshold = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_threshold);
        int rest_potential = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_rest_potential);
        unsigned int leak_shift = read_variant_u32(variant_idx, AXI_OFFSET_VariantParameters_leak_shift);
        int homeostasis_penalty = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_homeostasis_penalty);
        unsigned char refractory_period = read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_refractory_period);
        unsigned short ahp_amplitude = read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_ahp_amplitude);
        int adaptive_leak_min_shift = read_variant_i32(variant_idx, AXI_OFFSET_VariantParameters_adaptive_leak_min_shift);
        int adaptive_leak_gain_val = (int)read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_adaptive_leak_gain);
        int adaptive_mode_val = (int)read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_adaptive_mode);

        unsigned char timer = timers[i];
        bool is_glif = false;

        if (timer > 0) {
            timers[i] = timer - 1;
            threshold_offset[i] = decayed_offset;
            // voltage unchanged
            is_glif = false;
        } else {
            int voltage = soma_voltage[i];
            int i_in = i_in_device[i];

            long long adaptive_sub = ((long long)decayed_offset * (long long)adaptive_leak_gain_val) / 256 * (long long)adaptive_mode_val;
            long long current_shift = (long long)leak_shift - adaptive_sub;
            if (current_shift < (long long)adaptive_leak_min_shift) {
                current_shift = (long long)adaptive_leak_min_shift;
            }
            if (current_shift < 0) current_shift = 0;
            if (current_shift > 63) current_shift = 63;
            unsigned int shift = (unsigned int)current_shift;

            long long v_diff = (long long)voltage - (long long)rest_potential;
            int delta_v_leak = (int)(v_diff >> shift);

            int v_new = (int)((unsigned int)voltage + (unsigned int)i_in - (unsigned int)delta_v_leak);

            int v_th_eff = (int)((unsigned int)threshold + (unsigned int)decayed_offset);
            is_glif = (v_new >= v_th_eff);

            if (is_glif) {
                soma_voltage[i] = (int)((unsigned int)rest_potential - (unsigned int)ahp_amplitude);
                timers[i] = refractory_period;
                threshold_offset[i] = (int)((unsigned int)decayed_offset + (unsigned int)homeostasis_penalty);
            } else {
                soma_voltage[i] = v_new;
                threshold_offset[i] = decayed_offset;
            }
        }

        // 3. DDS heartbeat spike
        unsigned int heartbeat_m = read_variant_u32(variant_idx, AXI_OFFSET_VariantParameters_heartbeat_m);
        bool is_heartbeat = false;
        if (heartbeat_m == AXI_MAX_HEARTBEAT_M) {
            is_heartbeat = true;
        } else if (heartbeat_m == 0) {
            is_heartbeat = false;
        } else {
            unsigned long long phase = ((current_tick * (unsigned long long)heartbeat_m) +
                                       ((unsigned long long)i * AXI_DDS_SCATTER_PRIME)) & AXI_DDS_PHASE_MASK;
            is_heartbeat = (phase < (unsigned long long)heartbeat_m);
        }

        // 4. Final spike & soma_to_axon & output spikes
        bool final_spike = is_glif || is_heartbeat;
        unsigned char old_burst = (flags & AXI_SOMA_BURST_MASK) >> AXI_SOMA_BURST_SHIFT;
        unsigned char new_burst = old_burst;
        if (final_spike) {
            new_burst = old_burst + 1;
            if (new_burst > 7) {
                new_burst = 7;
            }
            *generated_spikes_count = *generated_spikes_count + 1;

            unsigned int axon_id = soma_to_axon[i];
            if (axon_id < total_axons) {
                size_t base_idx = (size_t)axon_id * AXI_HEADS_PER_BURST;
                for (int h = (int)AXI_HEADS_PER_BURST - 1; h > 0; --h) {
                    heads[base_idx + h] = heads[base_idx + h - 1];
                }
                heads[base_idx + 0] = 0u - v_seg;
            }

            bool is_mapped = false;
            for (unsigned int o = 0; o < num_outputs; ++o) {
                if (mapped_soma_ids[o] == i) {
                    is_mapped = true;
                    break;
                }
            }

            if (is_mapped) {
                unsigned int current_out_count = *output_count;
                if (current_out_count < max_spikes_per_tick) {
                    output_spikes[current_out_count] = i;
                    *output_count = current_out_count + 1;
                } else {
                    *dropped_spikes_count = *dropped_spikes_count + 1;
                }
            }
        }

        unsigned char new_flags = (flags & AXI_SOMA_TYPE_MASK) | ((new_burst << AXI_SOMA_BURST_SHIFT) & AXI_SOMA_BURST_MASK);
        if (final_spike) {
            new_flags |= AXI_SOMA_SPIKING_MASK;
        }
        soma_flags[i] = new_flags;
    }
}

__device__ int apply_gsop_plasticity(
    int weight,
    bool is_active,
    int gsop_potentiation,
    int gsop_depression,
    int dopamine,
    int d1_affinity,
    int d2_affinity,
    unsigned int burst_count,
    unsigned int variant_idx
) {
    int sign = 1 - ((weight >> 31) & 2);
    unsigned int abs_w = 0;
    if (weight == -2147483648) {
        abs_w = 2147483648u;
    } else {
        abs_w = (unsigned int)(weight < 0 ? -weight : weight);
    }

    unsigned int rank = abs_w >> AXI_INERTIA_RANK_SHIFT;
    if (rank > AXI_MAX_INERTIA_RANK) {
        rank = AXI_MAX_INERTIA_RANK;
    }

    size_t inertia_offset = AXI_OFFSET_VariantParameters_inertia_curve + rank;
    unsigned char inertia_curve_val = read_variant_u8(variant_idx, inertia_offset);
    long long inertia = (long long)inertia_curve_val;

    long long pot_mod = ((long long)dopamine * (long long)d1_affinity) / 128;
    long long dep_mod = ((long long)dopamine * (long long)d2_affinity) / 128;

    long long final_pot = (long long)gsop_potentiation + pot_mod;
    if (final_pot < 0) final_pot = 0;

    long long final_dep = (long long)gsop_depression - dep_mod;
    if (final_dep < 0) final_dep = 0;

    long long burst_mult = (long long)burst_count;
    if (burst_mult < 1) burst_mult = 1;

    long long delta_pot = (final_pot * inertia * burst_mult) / 128;
    long long delta_dep = (final_dep * inertia * burst_mult) / 128;

    long long active_mask = 0LL - (long long)is_active;
    long long delta = (delta_pot & active_mask) | ((-delta_dep) & ~active_mask);

    long long new_abs_raw = (long long)abs_w + delta;
    if (new_abs_raw < (long long)AXI_MIN_WEIGHT_LIMIT) {
        new_abs_raw = (long long)AXI_MIN_WEIGHT_LIMIT;
    } else if (new_abs_raw > (long long)AXI_MAX_WEIGHT_LIMIT) {
        new_abs_raw = (long long)AXI_MAX_WEIGHT_LIMIT;
    }
    unsigned int new_abs = (unsigned int)new_abs_raw;

    return (int)new_abs * sign;
}

__global__ void apply_gsop_plasticity_probe_kernel(
    void* state_ptr,
    const void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_targets,
    unsigned int off_weights,
    unsigned int off_flags,
    int dopamine
) {
    unsigned char* soma_flags = (unsigned char*)((char*)state_ptr + off_flags);
    unsigned int* dendrite_targets = (unsigned int*)((char*)state_ptr + off_targets);
    int* dendrite_weights = (int*)((char*)state_ptr + off_weights);
    const unsigned int* heads = (const unsigned int*)((const char*)axons_ptr + AXI_SIZE_AxonsFileHeader);

    for (unsigned int i = 0; i < padded_n; ++i) {
        unsigned char flags = soma_flags[i];
        if ((flags & AXI_SOMA_SPIKING_MASK) == 0) {
            continue;
        }

        unsigned int type_id = (flags & AXI_SOMA_TYPE_MASK) >> AXI_SOMA_TYPE_SHIFT;
        unsigned int variant_idx = type_id;
        if (variant_idx >= AXI_VARIANT_LUT_LEN) {
            variant_idx = AXI_VARIANT_LUT_LEN - 1;
        }

        unsigned int burst_count = (flags & AXI_SOMA_BURST_MASK) >> AXI_SOMA_BURST_SHIFT;

        int gsop_potentiation = read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_gsop_potentiation);
        int gsop_depression = read_variant_u16(variant_idx, AXI_OFFSET_VariantParameters_gsop_depression);
        int d1_affinity = read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_d1_affinity);
        int d2_affinity = read_variant_u8(variant_idx, AXI_OFFSET_VariantParameters_d2_affinity);
        unsigned int propagation_length = read_variant_u32(variant_idx, AXI_OFFSET_VariantParameters_signal_propagation_length);

        for (unsigned int d = 0; d < AXI_MAX_DENDRITES; ++d) {
            size_t synapse_idx = (size_t)d * padded_n + i;
            unsigned int raw_target = dendrite_targets[synapse_idx];
            if (raw_target == 0 || raw_target == AXI_EMPTY_PIXEL) {
                continue;
            }

            unsigned int axon_q = raw_target & 0x00FFFFFFu;
            if (axon_q == 0 || axon_q > AXI_MAX_AXON_ID + 1) {
                continue;
            }
            unsigned int axon_id = axon_q - 1;
            unsigned int segment_index = (raw_target >> 24) & 0xFFu;

            if (axon_id >= total_axons) {
                continue;
            }

            unsigned int heads_array[8];
            size_t base_idx = (size_t)axon_id * AXI_HEADS_PER_BURST;
            for (int h = 0; h < 8; ++h) {
                heads_array[h] = heads[base_idx + h];
            }

            bool is_active = false;
            for (int h = 0; h < 8; ++h) {
                unsigned int head = heads_array[h];
                unsigned int dist = head - segment_index;
                if (dist < propagation_length) {
                    is_active = true;
                    break;
                }
            }

            int w_old = dendrite_weights[synapse_idx];
            int w_new = apply_gsop_plasticity(
                w_old,
                is_active,
                gsop_potentiation,
                gsop_depression,
                dopamine,
                d1_affinity,
                d2_affinity,
                burst_count,
                variant_idx
            );
            dendrite_weights[synapse_idx] = w_new;
        }
    }
}

extern "C" {

int axi_cuda_apply_gsop_plasticity_probe(
    void* state_ptr,
    const void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_targets,
    unsigned int off_weights,
    unsigned int off_flags,
    int dopamine
) {
    if (padded_n > 0 && state_ptr == nullptr) {
        return -1;
    }
    if (total_axons > 0 && axons_ptr == nullptr) {
        return -1;
    }

    apply_gsop_plasticity_probe_kernel<<<1, 1>>>(
        state_ptr,
        axons_ptr,
        padded_n,
        total_axons,
        off_targets,
        off_weights,
        off_flags,
        dopamine
    );

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) {
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        return -4;
    }

    return 0;
}

int axi_cuda_probe_device(unsigned int device_id) {
    int device_count = 0;
    cudaError_t err = cudaGetDeviceCount(&device_count);
    if (err != cudaSuccess) {
        return -1;
    }
    if (device_id >= (unsigned int)device_count) {
        return -1;
    }
    err = cudaSetDevice(device_id);
    if (err != cudaSuccess) {
        return -1;
    }
    return 0;
}

int axi_cuda_propagate_head(unsigned int input_head, unsigned int v_seg, unsigned int* out) {
    unsigned int* d_out = nullptr;
    cudaError_t err = cudaMalloc(&d_out, sizeof(unsigned int));
    if (err != cudaSuccess) {
        return -2;
    }

    propagate_head_kernel<<<1, 1>>>(input_head, v_seg, d_out);
    
    err = cudaGetLastError();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -4;
    }

    err = cudaMemcpy(out, d_out, sizeof(unsigned int), cudaMemcpyDeviceToHost);
    cudaFree(d_out);
    if (err != cudaSuccess) {
        return -5;
    }

    return 0;
}

int axi_cuda_active_tail_hit(unsigned int head, unsigned int seg_idx, unsigned int propagation_length, unsigned char* out) {
    unsigned char* d_out = nullptr;
    cudaError_t err = cudaMalloc(&d_out, sizeof(unsigned char));
    if (err != cudaSuccess) {
        return -2;
    }

    active_tail_hit_kernel<<<1, 1>>>(head, seg_idx, propagation_length, d_out);

    err = cudaGetLastError();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -4;
    }

    err = cudaMemcpy(out, d_out, sizeof(unsigned char), cudaMemcpyDeviceToHost);
    cudaFree(d_out);
    if (err != cudaSuccess) {
        return -5;
    }

    return 0;
}

int axi_cuda_alloc_bytes(size_t size, void** out_ptr) {
    if (!out_ptr) return -1;
    cudaError_t err = cudaMalloc(out_ptr, size);
    if (err != cudaSuccess) {
        return -2;
    }
    return 0;
}

int axi_cuda_free(void* ptr) {
    if (!ptr) return 0;
    cudaError_t err = cudaFree(ptr);
    if (err != cudaSuccess) {
        return -1;
    }
    return 0;
}

int axi_cuda_copy_h2d(void* dst, const void* src, size_t size) {
    if (!dst || !src) return -1;
    cudaError_t err = cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice);
    if (err != cudaSuccess) {
        return -5;
    }
    return 0;
}

int axi_cuda_copy_d2h(void* dst, const void* src, size_t size) {
    if (!dst || !src) return -1;
    cudaError_t err = cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost);
    if (err != cudaSuccess) {
        return -5;
    }
    return 0;
}

int axi_cuda_upload_variant_table(const void* src, size_t size) {
    if (!src) return -1;
    if (size > AXI_SIZE_VariantParameters * AXI_VARIANT_LUT_LEN) {
        return -1;
    }
    cudaError_t err = cudaMemcpyToSymbol(axi_variant_table_bytes, src, size);
    if (err != cudaSuccess) {
        return -5;
    }
    return 0;
}

int axi_cuda_propagate_uploaded_axons(void* axons_ptr, unsigned int total_axons, unsigned int v_seg) {
    if (!axons_ptr) {
        return -1;
    }

    size_t heads_per_burst = AXI_SIZE_BurstHeads8 / sizeof(unsigned int);
    size_t total_heads = (size_t)total_axons * heads_per_burst;

    if (total_heads == 0) {
        return 0;
    }

    size_t threads_per_block = 256;
    if (total_axons > 0 && total_heads / total_axons != heads_per_burst) {
        return -1;
    }
    if (total_heads > 0xFFFFFFFF - (threads_per_block - 1)) {
        return -1;
    }

    unsigned int* heads = (unsigned int*)((char*)axons_ptr + AXI_SIZE_AxonsFileHeader);
    unsigned int total_heads_u32 = (unsigned int)total_heads;
    unsigned int threads_per_block_u32 = (unsigned int)threads_per_block;
    unsigned int blocks = (total_heads_u32 + threads_per_block_u32 - 1) / threads_per_block_u32;

    propagate_uploaded_axons_kernel<<<blocks, threads_per_block_u32>>>(heads, total_heads_u32, v_seg);

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) {
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        return -4;
    }

    return 0;
}

int axi_cuda_inject_and_propagate_axons_tick(
    void* axons_ptr,
    unsigned int total_axons,
    unsigned int v_seg,
    unsigned int shard_virtual_offset,
    unsigned int cmd_virtual_offset,
    unsigned int num_virtual_axons,
    const unsigned int* input_bitmask,
    unsigned int input_words_len,
    const unsigned int* incoming_spikes,
    unsigned int incoming_spikes_count
) {
    if (!axons_ptr) {
        return -1;
    }

    if (total_axons == 0) {
        return 0;
    }

    size_t threads_per_block = 256;
    if (total_axons > 0xFFFFFFFF - (threads_per_block - 1)) {
        return -1;
    }

    unsigned int* heads = (unsigned int*)((char*)axons_ptr + AXI_SIZE_AxonsFileHeader);
    unsigned int threads_per_block_u32 = (unsigned int)threads_per_block;
    unsigned int blocks = (total_axons + threads_per_block_u32 - 1) / threads_per_block_u32;

    inject_and_propagate_axons_tick_kernel<<<blocks, threads_per_block_u32>>>(
        heads,
        total_axons,
        v_seg,
        shard_virtual_offset,
        cmd_virtual_offset,
        num_virtual_axons,
        input_bitmask,
        input_words_len,
        incoming_spikes,
        incoming_spikes_count
    );

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) {
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        return -4;
    }

    return 0;
}

int axi_cuda_compute_input_current_probe(
    const void* state_ptr,
    const void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_targets,
    unsigned int off_weights,
    unsigned int off_flags,
    int* out_i_in_host,
    unsigned int out_len
) {
    if (!state_ptr || !axons_ptr || !out_i_in_host) {
        return -1;
    }
    if (out_len < padded_n) {
        return -1;
    }

    int* d_out = nullptr;
    cudaError_t err = cudaMalloc(&d_out, padded_n * sizeof(int));
    if (err != cudaSuccess) {
        return -2;
    }

    err = cudaMemset(d_out, 0, padded_n * sizeof(int));
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -5;
    }

    unsigned int threads_per_block = 256;
    unsigned int blocks = (padded_n + threads_per_block - 1) / threads_per_block;

    compute_input_current_probe_kernel<<<blocks, threads_per_block>>>(
        state_ptr,
        axons_ptr,
        padded_n,
        total_axons,
        off_targets,
        off_weights,
        off_flags,
        d_out
    );

    err = cudaGetLastError();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -3;
    }

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) {
        cudaFree(d_out);
        return -4;
    }

    err = cudaMemcpy(out_i_in_host, d_out, padded_n * sizeof(int), cudaMemcpyDeviceToHost);
    cudaFree(d_out);

    if (err != cudaSuccess) {
        return -5;
    }

    return 0;
}

int axi_cuda_apply_glif_membrane_probe(
    void* state_ptr,
    unsigned int padded_n,
    unsigned int off_voltage,
    unsigned int off_flags,
    unsigned int off_thresh,
    unsigned int off_timers,
    const int* i_in_host,
    unsigned int i_in_len
) {
    if (!state_ptr || !i_in_host) {
        return -1;
    }
    if (i_in_len < padded_n) {
        return -1;
    }

    int* d_i_in = nullptr;
    cudaError_t err = cudaMalloc(&d_i_in, padded_n * sizeof(int));
    if (err != cudaSuccess) {
        return -2;
    }

    err = cudaMemcpy(d_i_in, i_in_host, padded_n * sizeof(int), cudaMemcpyHostToDevice);
    if (err != cudaSuccess) {
        cudaFree(d_i_in);
        return -5;
    }

    unsigned int threads_per_block = 256;
    unsigned int blocks = (padded_n + threads_per_block - 1) / threads_per_block;

    apply_glif_membrane_probe_kernel<<<blocks, threads_per_block>>>(
        state_ptr,
        padded_n,
        off_voltage,
        off_flags,
        off_thresh,
        off_timers,
        d_i_in
    );

    err = cudaGetLastError();
    if (err != cudaSuccess) {
        cudaFree(d_i_in);
        return -3;
    }

    err = cudaDeviceSynchronize();
    cudaFree(d_i_in);

    if (err != cudaSuccess) {
        return -4;
    }

    return 0;
}

int axi_cuda_apply_glif_final_spike_probe(
    void* state_ptr,
    void* axons_ptr,
    unsigned int padded_n,
    unsigned int total_axons,
    unsigned int off_voltage,
    unsigned int off_flags,
    unsigned int off_thresh,
    unsigned int off_timers,
    unsigned int off_s2a,
    const int* i_in_host,
    unsigned int i_in_len,
    unsigned long long current_tick,
    unsigned int v_seg,
    const unsigned int* mapped_soma_ids_host,
    unsigned int num_outputs,
    unsigned int max_spikes_per_tick,
    unsigned int* output_spikes_host,
    unsigned int* output_spike_counts_host,
    unsigned int* generated_spikes_count_host,
    unsigned int* dropped_spikes_count_host
) {
    if (!state_ptr || !axons_ptr || !i_in_host || !output_spikes_host || 
        !output_spike_counts_host || !generated_spikes_count_host || !dropped_spikes_count_host) {
        return -1;
    }
    if (i_in_len < padded_n) {
        return -1;
    }
    if (num_outputs > 0 && !mapped_soma_ids_host) {
        return -1;
    }

    // Temporary device allocations
    int* d_i_in = nullptr;
    unsigned int* d_mapped_soma_ids = nullptr;
    unsigned int* d_output_spikes = nullptr;
    unsigned int* d_output_count = nullptr;
    unsigned int* d_generated_spikes_count = nullptr;
    unsigned int* d_dropped_spikes_count = nullptr;

    cudaError_t err = cudaMalloc(&d_i_in, padded_n * sizeof(int));
    if (err != cudaSuccess) return -2;

    if (num_outputs > 0 && mapped_soma_ids_host) {
        err = cudaMalloc(&d_mapped_soma_ids, num_outputs * sizeof(unsigned int));
        if (err != cudaSuccess) {
            cudaFree(d_i_in);
            return -2;
        }
    }

    if (max_spikes_per_tick > 0) {
        err = cudaMalloc(&d_output_spikes, max_spikes_per_tick * sizeof(unsigned int));
        if (err != cudaSuccess) {
            cudaFree(d_i_in);
            if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
            return -2;
        }
    }

    err = cudaMalloc(&d_output_count, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_err;

    err = cudaMalloc(&d_generated_spikes_count, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_err;

    err = cudaMalloc(&d_dropped_spikes_count, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_err;

    // Memcpy host to device
    err = cudaMemcpy(d_i_in, i_in_host, padded_n * sizeof(int), cudaMemcpyHostToDevice);
    if (err != cudaSuccess) goto cleanup_dma_err;

    if (num_outputs > 0 && mapped_soma_ids_host) {
        err = cudaMemcpy(d_mapped_soma_ids, mapped_soma_ids_host, num_outputs * sizeof(unsigned int), cudaMemcpyHostToDevice);
        if (err != cudaSuccess) goto cleanup_dma_err;
    }

    err = cudaMemset(d_output_count, 0, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_dma_err;

    err = cudaMemset(d_generated_spikes_count, 0, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_dma_err;

    err = cudaMemset(d_dropped_spikes_count, 0, sizeof(unsigned int));
    if (err != cudaSuccess) goto cleanup_dma_err;

    // Launch single-threaded kernel
    apply_glif_final_spike_probe_kernel<<<1, 1>>>(
        state_ptr,
        axons_ptr,
        padded_n,
        total_axons,
        off_voltage,
        off_flags,
        off_thresh,
        off_timers,
        off_s2a,
        d_i_in,
        current_tick,
        v_seg,
        d_mapped_soma_ids,
        num_outputs,
        max_spikes_per_tick,
        d_output_spikes,
        d_output_count,
        d_generated_spikes_count,
        d_dropped_spikes_count
    );

    err = cudaGetLastError();
    if (err != cudaSuccess) goto cleanup_launch_err;

    err = cudaDeviceSynchronize();
    if (err != cudaSuccess) goto cleanup_sync_err;

    // Copy results back
    unsigned int h_output_count;
    err = cudaMemcpy(&h_output_count, d_output_count, sizeof(unsigned int), cudaMemcpyDeviceToHost);
    if (err != cudaSuccess) goto cleanup_dma_err;
    *output_spike_counts_host = h_output_count;

    if (h_output_count > 0 && max_spikes_per_tick > 0) {
        unsigned int copy_len = h_output_count;
        if (copy_len > max_spikes_per_tick) {
            copy_len = max_spikes_per_tick;
        }
        err = cudaMemcpy(output_spikes_host, d_output_spikes, copy_len * sizeof(unsigned int), cudaMemcpyDeviceToHost);
        if (err != cudaSuccess) goto cleanup_dma_err;
    }

    err = cudaMemcpy(generated_spikes_count_host, d_generated_spikes_count, sizeof(unsigned int), cudaMemcpyDeviceToHost);
    if (err != cudaSuccess) goto cleanup_dma_err;

    err = cudaMemcpy(dropped_spikes_count_host, d_dropped_spikes_count, sizeof(unsigned int), cudaMemcpyDeviceToHost);
    if (err != cudaSuccess) goto cleanup_dma_err;

    // Free device memory
    cudaFree(d_i_in);
    if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
    if (d_output_spikes) cudaFree(d_output_spikes);
    cudaFree(d_output_count);
    cudaFree(d_generated_spikes_count);
    cudaFree(d_dropped_spikes_count);
    return 0;

cleanup_launch_err:
    cudaFree(d_i_in);
    if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
    if (d_output_spikes) cudaFree(d_output_spikes);
    cudaFree(d_output_count);
    cudaFree(d_generated_spikes_count);
    cudaFree(d_dropped_spikes_count);
    return -3;

cleanup_sync_err:
    cudaFree(d_i_in);
    if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
    if (d_output_spikes) cudaFree(d_output_spikes);
    cudaFree(d_output_count);
    cudaFree(d_generated_spikes_count);
    cudaFree(d_dropped_spikes_count);
    return -4;

cleanup_dma_err:
    cudaFree(d_i_in);
    if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
    if (d_output_spikes) cudaFree(d_output_spikes);
    cudaFree(d_output_count);
    cudaFree(d_generated_spikes_count);
    cudaFree(d_dropped_spikes_count);
    return -5;

cleanup_err:
    cudaFree(d_i_in);
    if (d_mapped_soma_ids) cudaFree(d_mapped_soma_ids);
    if (d_output_spikes) cudaFree(d_output_spikes);
    if (d_output_count) cudaFree(d_output_count);
    if (d_generated_spikes_count) cudaFree(d_generated_spikes_count);
    if (d_dropped_spikes_count) cudaFree(d_dropped_spikes_count);
    return -2;
}

} // extern "C"
