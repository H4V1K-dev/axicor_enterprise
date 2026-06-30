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
    pub fn axi_cuda_propagate_uploaded_axons(
        axons_ptr: *mut u8,
        total_axons: u32,
        v_seg: u32,
    ) -> i32;
    pub fn axi_cuda_inject_and_propagate_axons_tick(
        axons_ptr: *mut u8,
        total_axons: u32,
        v_seg: u32,
        shard_virtual_offset: u32,
        cmd_virtual_offset: u32,
        num_virtual_axons: u32,
        input_bitmask: *const u32,
        input_words_len: u32,
        incoming_spikes: *const u32,
        incoming_spikes_count: u32,
    ) -> i32;
    pub fn axi_cuda_compute_input_current_probe(
        state_ptr: *const u8,
        axons_ptr: *const u8,
        padded_n: u32,
        total_axons: u32,
        off_targets: u32,
        off_weights: u32,
        off_flags: u32,
        out_i_in_host: *mut i32,
        out_len: u32,
    ) -> i32;
    pub fn axi_cuda_apply_glif_membrane_probe(
        state_ptr: *mut u8,
        padded_n: u32,
        off_voltage: u32,
        off_flags: u32,
        off_thresh: u32,
        off_timers: u32,
        i_in_host: *const i32,
        i_in_len: u32,
    ) -> i32;

    pub fn axi_cuda_apply_glif_final_spike_probe(
        state_ptr: *mut u8,
        axons_ptr: *mut u8,
        padded_n: u32,
        total_axons: u32,
        off_voltage: u32,
        off_flags: u32,
        off_thresh: u32,
        off_timers: u32,
        off_s2a: u32,
        i_in_host: *const i32,
        i_in_len: u32,
        current_tick: u64,
        v_seg: u32,
        mapped_soma_ids_host: *const u32,
        num_outputs: u32,
        max_spikes_per_tick: u32,
        output_spikes_host: *mut u32,
        output_spike_counts_host: *mut u32,
        generated_spikes_count_host: *mut u32,
        dropped_spikes_count_host: *mut u32,
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
