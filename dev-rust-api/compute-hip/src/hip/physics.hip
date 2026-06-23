#include "core_math.h"
#include <hip/hip_runtime.h>

extern "C" {
    struct __attribute__((aligned(64))) VariantParameters {
        int32_t threshold;
        int32_t rest_potential;
        uint32_t leak_shift;
        int32_t homeostasis_penalty;
        uint32_t spontaneous_firing_period_ticks;
        uint16_t initial_synapse_weight;
        uint16_t gsop_potentiation;
        uint16_t gsop_depression;
        uint16_t homeostasis_decay;
        uint8_t refractory_period;
        uint8_t synapse_refractory_period;
        uint8_t signal_propagation_length;
        uint8_t is_inhibitory;
        uint8_t inertia_curve[8];
        uint16_t ahp_amplitude;
        uint8_t _pad[6];
        int32_t adaptive_leak_min_shift;
        uint16_t adaptive_leak_gain;
        uint8_t adaptive_mode;
        uint8_t _leak_pad[3];
        uint8_t d1_affinity;
        uint8_t d2_affinity;
        uint32_t heartbeat_m;
    };

    struct __attribute__((aligned(32))) BurstHeads8 {
        uint32_t h0;
        uint32_t h1;
        uint32_t h2;
        uint32_t h3;
        uint32_t h4;
        uint32_t h5;
        uint32_t h6;
        uint32_t h7;
    };

    struct ShardVramPtrs {
        int32_t* soma_voltage;
        uint8_t* flags;
        int32_t* threshold_offset;
        uint8_t* timers;
        uint32_t* soma_to_axon;
        uint32_t* dendrite_targets;
        int32_t* dendrite_weights;
        uint8_t* dendrite_timers;
        void* axon_heads;
        const void* variant_params;
    };
}

__global__ void kernel_update_neurons(ShardVramPtrs vram, uint32_t padded_n, uint32_t current_tick, uint32_t v_seg) {
    uint32_t tid = hipBlockIdx_x * hipBlockDim_x + hipThreadIdx_x;
    if (tid >= padded_n) return;

    uint8_t flags = vram.flags[tid];
    uint8_t variant_id = (flags >> 4) & 0x0F;

    const VariantParameters* vp_array = (const VariantParameters*)vram.variant_params;
    VariantParameters P = vp_array[variant_id];

    int32_t thresh_offset = vram.threshold_offset[tid];
    thresh_offset = update_homeostasis(thresh_offset, P.homeostasis_decay, false, 0);

    uint8_t timer = vram.timers[tid];
    if (timer > 0) {
        vram.timers[tid] = timer - 1;
        vram.threshold_offset[tid] = thresh_offset;
        return; // Early Exit
    }

    int32_t I_in = 0;
    uint32_t prop = P.signal_propagation_length;
    uint32_t* heads = (uint32_t*)vram.axon_heads;

    #pragma unroll(4)
    for (uint32_t slot = 0; slot < 128; slot++) {
        uint32_t idx = slot * padded_n + tid;
        uint32_t target_packed = vram.dendrite_targets[idx];
        if (target_packed == 0) break;

        uint32_t raw_id = target_packed & 0x00FFFFFF;
        if (raw_id == 0) continue;
        uint32_t target_id = raw_id - 1;
        uint32_t offset = target_id * 8;

        uint32_t seg_offset = target_packed >> 24;
        uint32_t hit = 0;
        
        hit |= (heads[offset + 0] - seg_offset) < prop;
        hit |= (heads[offset + 1] - seg_offset) < prop;
        hit |= (heads[offset + 2] - seg_offset) < prop;
        hit |= (heads[offset + 3] - seg_offset) < prop;
        hit |= (heads[offset + 4] - seg_offset) < prop;
        hit |= (heads[offset + 5] - seg_offset) < prop;
        hit |= (heads[offset + 6] - seg_offset) < prop;
        hit |= (heads[offset + 7] - seg_offset) < prop;

        I_in += (vram.dendrite_weights[idx] >> 16) * hit;
    }

    int32_t V = vram.soma_voltage[tid];
    V = compute_glif(V, P.rest_potential, P.leak_shift, I_in);

    uint32_t phase = ((current_tick * P.heartbeat_m) + (tid * 104729)) & 0xFFFF;
    bool is_heartbeat = (P.heartbeat_m > 0 && phase < P.heartbeat_m);
    bool is_glif = V >= (P.threshold + thresh_offset);
    uint8_t final_spike = is_glif | is_heartbeat;

    if (final_spike) {
        uint32_t local_axon = vram.soma_to_axon[tid];
        uint32_t offset = local_axon * 8;
        
        heads[offset + 7] = heads[offset + 6];
        heads[offset + 6] = heads[offset + 5];
        heads[offset + 5] = heads[offset + 4];
        heads[offset + 4] = heads[offset + 3];
        heads[offset + 3] = heads[offset + 2];
        heads[offset + 2] = heads[offset + 1];
        heads[offset + 1] = heads[offset + 0];
        heads[offset + 0] = (uint32_t)0 - v_seg;
    }

    uint8_t burst_count = (flags >> 1) & 0x07;
    uint8_t next_burst = final_spike * (burst_count + (burst_count < 7));
    vram.flags[tid] = (flags & 0xF0) | (next_burst << 1) | final_spike;
    vram.soma_voltage[tid] = final_spike ? (P.rest_potential - (int32_t)P.ahp_amplitude) : V;
    vram.timers[tid] = final_spike ? P.refractory_period : timer;
    vram.threshold_offset[tid] = thresh_offset + (final_spike * P.homeostasis_penalty);
}

__global__ void kernel_propagate_axons(ShardVramPtrs vram, uint32_t padded_n, uint32_t v_seg) {
    uint32_t tid = hipBlockIdx_x * hipBlockDim_x + hipThreadIdx_x;
    if (tid >= padded_n) return;

    uint32_t* heads = (uint32_t*)vram.axon_heads;
    uint32_t offset = tid * 8;
    
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        uint32_t h = heads[offset + i];
        uint32_t step = v_seg * ((h ^ 0x80000000) >= v_seg);
        heads[offset + i] = h + step;
    }
}

__global__ void kernel_apply_gsop(ShardVramPtrs vram, uint32_t padded_n, uint32_t v_seg) {
    uint32_t tid = hipBlockIdx_x * hipBlockDim_x + hipThreadIdx_x;
    if (tid >= padded_n) return;
}

extern "C" hipError_t hip_launch_update_neurons(ShardVramPtrs vram, uint32_t padded_n, uint32_t current_tick, uint32_t v_seg, hipStream_t stream) {
    uint32_t threads = 256;
    uint32_t blocks = (padded_n + threads - 1) / threads;
    hipLaunchKernelGGL(kernel_update_neurons, dim3(blocks), dim3(threads), 0, stream, vram, padded_n, current_tick, v_seg);
    return hipGetLastError();
}

extern "C" hipError_t hip_launch_propagate_axons(ShardVramPtrs vram, uint32_t padded_n, uint32_t v_seg, hipStream_t stream) {
    uint32_t threads = 256;
    uint32_t blocks = (padded_n + threads - 1) / threads;
    hipLaunchKernelGGL(kernel_propagate_axons, dim3(blocks), dim3(threads), 0, stream, vram, padded_n, v_seg);
    return hipGetLastError();
}

extern "C" hipError_t hip_launch_apply_gsop(ShardVramPtrs vram, uint32_t padded_n, uint32_t v_seg, hipStream_t stream) {
    uint32_t threads = 256;
    uint32_t blocks = (padded_n + threads - 1) / threads;
    hipLaunchKernelGGL(kernel_apply_gsop, dim3(blocks), dim3(threads), 0, stream, vram, padded_n, v_seg);
    return hipGetLastError();
}
