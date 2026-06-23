unsafe extern "C" {
    pub fn hipSetDevice(device: i32) -> i32;
    pub fn hipDeviceGetAttribute(value: *mut i32, attr: i32, device: i32) -> i32;
    pub fn hipStreamCreate(pStream: *mut *mut std::ffi::c_void) -> i32;
    pub fn hipStreamDestroy(stream: *mut std::ffi::c_void) -> i32;
    pub fn hipStreamSynchronize(stream: *mut std::ffi::c_void) -> i32;
    
    pub fn hipMalloc(devPtr: *mut *mut std::ffi::c_void, size: usize) -> i32;
    pub fn hipFree(devPtr: *mut std::ffi::c_void) -> i32;
    pub fn hipHostMalloc(ptr: *mut *mut std::ffi::c_void, size: usize, flags: u32) -> i32;
    pub fn hipHostFree(ptr: *mut std::ffi::c_void) -> i32;
    
    pub fn hipMemcpyAsync(
        dst: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        count: usize,
        kind: i32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn hipMemsetAsync(
        devPtr: *mut std::ffi::c_void,
        value: i32,
        count: usize,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn hip_launch_update_neurons(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        current_tick: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn hip_launch_propagate_axons(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
    
    pub fn hip_launch_apply_gsop(
        vram: layout::ShardVramPtrs,
        padded_n: u32,
        v_seg: u32,
        stream: *mut std::ffi::c_void,
    ) -> i32;
}

pub const HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR: i32 = 48;
pub const HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR: i32 = 49;
pub const HIP_DEVICE_ATTRIBUTE_WARP_SIZE: i32 = 10;

pub const HIP_MEMCPY_HOST_TO_DEVICE: i32 = 1;
pub const HIP_MEMCPY_DEVICE_TO_HOST: i32 = 2;
pub const HIP_HOST_MALLOC_DEFAULT: u32 = 0;
