#include <cuda_runtime.h>
#include "axi_cuda_abi.h"

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

}
