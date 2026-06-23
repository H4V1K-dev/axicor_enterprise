unsafe extern "C" {
    pub fn cudaSetDevice(device: i32) -> i32;
    pub fn cudaDeviceGetAttribute(value: *mut i32, attr: i32, device: i32) -> i32;
    pub fn cudaStreamCreate(pStream: *mut *mut std::ffi::c_void) -> i32;
    pub fn cudaStreamDestroy(stream: *mut std::ffi::c_void) -> i32;
    pub fn cudaStreamSynchronize(stream: *mut std::ffi::c_void) -> i32;
    
    pub fn cudaMalloc(devPtr: *mut *mut std::ffi::c_void, size: usize) -> i32;
    pub fn cudaFree(devPtr: *mut std::ffi::c_void) -> i32;
    pub fn cudaMallocHost(ptr: *mut *mut std::ffi::c_void, size: usize) -> i32;
    pub fn cudaFreeHost(ptr: *mut std::ffi::c_void) -> i32;
    
    pub fn cudaMemcpyAsync(
        dst: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        count: usize,
        kind: i32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn cudaMemsetAsync(
        devPtr: *mut std::ffi::c_void,
        value: i32,
        count: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn cuda_launch_update_neurons(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        current_tick: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn cuda_launch_propagate_axons(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn cuda_launch_apply_gsop(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
}

pub const CUDA_DEV_ATTR_COMPUTE_CAPABILITY_MAJOR: i32 = 13;
pub const CUDA_DEV_ATTR_COMPUTE_CAPABILITY_MINOR: i32 = 14;
pub const CUDA_DEV_ATTR_WARP_SIZE: i32 = 20;

pub const CUDA_MEMCPY_HOST_TO_DEVICE: i32 = 1;
pub const CUDA_MEMCPY_DEVICE_TO_HOST: i32 = 2;
