#include <cuda_runtime.h>
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

extern "C" {

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

} // extern "C"
