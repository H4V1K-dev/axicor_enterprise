use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use layout::ShardVramPtrs;

std::thread_local! {
    pub static FAIL_MALLOC: AtomicBool = const { AtomicBool::new(false) };
    pub static FAIL_PINNED_ALLOC: AtomicBool = const { AtomicBool::new(false) };
    pub static SIMULATE_DEVICE_LOST: AtomicBool = const { AtomicBool::new(false) };
    pub static COMPUTE_CAPABILITY_MAJOR: AtomicI32 = const { AtomicI32::new(9) };
    pub static COMPUTE_CAPABILITY_MINOR: AtomicI32 = const { AtomicI32::new(0) };
}

pub static DEV_WARP_SIZE: AtomicI32 = AtomicI32::new(64);
pub static DEVICE_COUNT: AtomicI32 = AtomicI32::new(1);

// Standard HIP runtime error codes
pub const HIP_SUCCESS: i32 = 0;
pub const HIP_ERROR_MEMORY_ALLOCATION: i32 = 2;
pub const HIP_ERROR_DEVICE_LOST: i32 = 3;
pub const HIP_ERROR_INVALID_VALUE: i32 = 1;
pub const HIP_ERROR_INVALID_RESOURCE_HANDLE: i32 = 400;
pub const HIP_ERROR_LAUNCH_OUT_OF_RESOURCES: i32 = 701;
pub const HIP_ERROR_NOT_SUPPORTED: i32 = 801;

pub const HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR: i32 = 48;
pub const HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR: i32 = 49;
pub const HIP_DEVICE_ATTRIBUTE_WARP_SIZE: i32 = 10;

unsafe fn mock_alloc_64(size: usize) -> *mut c_void {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let total_size = size + 64;
    let layout = match Layout::from_size_align(total_size, 64) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    let ptr = unsafe { alloc_zeroed(layout) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        *(ptr as *mut usize) = total_size;
        ptr.add(64) as *mut c_void
    }
}

unsafe fn mock_free_64(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let real_ptr = (ptr as *mut u8).sub(64);
        let total_size = *(real_ptr as *const usize);
        if let Ok(layout) = Layout::from_size_align(total_size, 64) {
            dealloc(real_ptr, layout);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipSetDevice(device: i32) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let count = DEVICE_COUNT.load(Ordering::Relaxed);
    if device < 0 || device >= count {
        return HIP_ERROR_INVALID_VALUE;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipGetDeviceCount(count: *mut i32) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    if count.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }
    unsafe {
        *count = DEVICE_COUNT.load(Ordering::Relaxed);
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipDeviceGetAttribute(value: *mut i32, attr: i32, device: i32) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let count = DEVICE_COUNT.load(Ordering::Relaxed);
    if device < 0 || device >= count || value.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }

    unsafe {
        match attr {
            HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR => {
                *value = COMPUTE_CAPABILITY_MAJOR.with(|f| f.load(Ordering::Relaxed));
            }
            HIP_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR => {
                *value = COMPUTE_CAPABILITY_MINOR.with(|f| f.load(Ordering::Relaxed));
            }
            HIP_DEVICE_ATTRIBUTE_WARP_SIZE => {
                *value = DEV_WARP_SIZE.load(Ordering::Relaxed);
            }
            _ => return HIP_ERROR_INVALID_VALUE,
        }
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipMalloc(dev_ptr: *mut *mut c_void, size: usize) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let fail_alloc = FAIL_MALLOC.with(|f| f.load(Ordering::Relaxed));
    if fail_alloc {
        return HIP_ERROR_MEMORY_ALLOCATION;
    }
    if dev_ptr.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }

    let ptr = unsafe { mock_alloc_64(size) };
    if ptr.is_null() {
        return HIP_ERROR_MEMORY_ALLOCATION;
    }
    unsafe {
        *dev_ptr = ptr;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipFree(dev_ptr: *mut c_void) -> i32 {
    if dev_ptr.is_null() {
        return HIP_SUCCESS;
    }
    unsafe {
        mock_free_64(dev_ptr);
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipHostMalloc(ptr: *mut *mut c_void, size: usize, _flags: u32) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let fail_pinned = FAIL_PINNED_ALLOC.with(|f| f.load(Ordering::Relaxed));
    if fail_pinned {
        return HIP_ERROR_MEMORY_ALLOCATION;
    }
    if ptr.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }

    let allocated = unsafe { mock_alloc_64(size) };
    if allocated.is_null() {
        return HIP_ERROR_MEMORY_ALLOCATION;
    }
    unsafe {
        *ptr = allocated;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipHostFree(ptr: *mut c_void) -> i32 {
    if ptr.is_null() {
        return HIP_SUCCESS;
    }
    unsafe {
        mock_free_64(ptr);
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipStreamCreate(p_stream: *mut *mut c_void) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    if p_stream.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }
    unsafe {
        *p_stream = 0x517ea3 as *mut c_void;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipStreamDestroy(stream: *mut c_void) -> i32 {
    let _ = stream;
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipStreamSynchronize(stream: *mut c_void) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let _ = stream;
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipDeviceSynchronize() -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipMemcpyAsync(
    dst: *mut c_void,
    src: *const c_void,
    count: usize,
    _kind: i32,
    _stream: *mut c_void,
) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    if dst.is_null() || src.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, count);
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipMemsetAsync(
    dev_ptr: *mut c_void,
    value: i32,
    count: usize,
    _stream: *mut c_void,
) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    if dev_ptr.is_null() {
        return HIP_ERROR_INVALID_VALUE;
    }
    unsafe {
        std::ptr::write_bytes(dev_ptr as *mut u8, value as u8, count);
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipGetLastError() -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hipGetErrorString(error: i32) -> *const c_char {
    match error {
        HIP_SUCCESS => b"hipSuccess\0".as_ptr() as *const c_char,
        HIP_ERROR_MEMORY_ALLOCATION => b"hipErrorMemoryAllocation\0".as_ptr() as *const c_char,
        HIP_ERROR_DEVICE_LOST => b"hipErrorDeviceLost\0".as_ptr() as *const c_char,
        HIP_ERROR_NOT_SUPPORTED => b"hipErrorNotSupported\0".as_ptr() as *const c_char,
        _ => b"hipErrorUnknown\0".as_ptr() as *const c_char,
    }
}

// Mock launcher functions matching C-ABI declarations in physics.hip / lib.rs

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hip_launch_update_neurons(
    vram: ShardVramPtrs,
    padded_n: u32,
    current_tick: u32,
    v_seg: u32,
    _stream: *mut c_void,
) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    if padded_n == 99968 {
        return HIP_ERROR_LAUNCH_OUT_OF_RESOURCES;
    }
    let _ = (vram, padded_n, current_tick, v_seg);
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hip_launch_propagate_axons(
    vram: ShardVramPtrs,
    padded_n: u32,
    v_seg: u32,
    _stream: *mut c_void,
) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let _ = (vram, padded_n, v_seg);
    HIP_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hip_launch_apply_gsop(
    vram: ShardVramPtrs,
    padded_n: u32,
    v_seg: u32,
    _stream: *mut c_void,
) -> i32 {
    let dev_lost = SIMULATE_DEVICE_LOST.with(|f| f.load(Ordering::Relaxed));
    if dev_lost {
        return HIP_ERROR_DEVICE_LOST;
    }
    let _ = (vram, padded_n, v_seg);
    HIP_SUCCESS
}
