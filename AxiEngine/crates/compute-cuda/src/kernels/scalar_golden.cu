#include <cuda_runtime.h>
#include "axi_cuda_abi.h"

static_assert(AXI_SIZE_AxonsFileHeader == 16, "AXI_SIZE_AxonsFileHeader must be exactly 16 bytes");
static_assert(AXI_SIZE_BurstHeads8 % sizeof(unsigned int) == 0, "AXI_SIZE_BurstHeads8 must be a multiple of sizeof(unsigned int)");
static_assert(AXI_SIZE_BurstHeads8 / sizeof(unsigned int) == 8, "AXI_SIZE_BurstHeads8 must represent exactly 8 heads");


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

#define AXI_HEADS_PER_BURST (AXI_SIZE_BurstHeads8 / sizeof(unsigned int))

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


__constant__ unsigned char axi_variant_table_bytes[AXI_SIZE_VariantParameters * AXI_VARIANT_LUT_LEN];

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

}
