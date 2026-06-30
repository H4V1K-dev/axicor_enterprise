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

    pub fn axi_cuda_alloc_bytes(size: usize, out_ptr: *mut *mut u8) -> i32;
    pub fn axi_cuda_free(ptr: *mut u8) -> i32;
    pub fn axi_cuda_copy_h2d(dst: *mut u8, src: *const u8, size: usize) -> i32;
    pub fn axi_cuda_copy_d2h(dst: *mut u8, src: *const u8, size: usize) -> i32;
    pub fn axi_cuda_upload_variant_table(src: *const u8, size: usize) -> i32;
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
