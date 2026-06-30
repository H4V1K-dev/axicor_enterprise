//! Private Native FFI boundary for `compute-cuda`.

use compute_api::ComputeApiError;

extern "C" {
    pub fn axi_cuda_probe_device(device_id: u32) -> i32;
    pub fn axi_cuda_propagate_head(input_head: u32, v_seg: u32, out: *mut u32) -> i32;
    pub fn axi_cuda_active_tail_hit(
        head: u32,
        seg_idx: u32,
        propagation_length: u32,
        out: *mut u8,
    ) -> i32;
}

/// Maps native C API return code to `ComputeApiError`.
pub fn map_cuda_error(code: i32) -> ComputeApiError {
    match code {
        -1 => ComputeApiError::UnsupportedBackend,
        -2 => ComputeApiError::OutOfMemory,
        -3 => ComputeApiError::KernelLaunchFailed,
        -4 => ComputeApiError::SynchronizeFailed,
        -5 => ComputeApiError::DmaFailed,
        _ => ComputeApiError::VendorError { code },
    }
}
