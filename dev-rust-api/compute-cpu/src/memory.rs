use std::alloc::{alloc_zeroed, dealloc, Layout};
use compute_api::{ComputeApiError, ShardLayout};

/// CPU-bound memory resource holding the flat aligned allocation block for a shard.
pub struct ShardCpuResources {
    /// Monolithic buffer containing the state of the shard, aligned to 64 bytes.
    pub raw_ptr: *mut u8,
    /// Size of the raw buffer in bytes.
    pub size: usize,
    /// Geometric layout information for the shard.
    pub layout: ShardLayout,
}

impl ShardCpuResources {
    /// Allocates a 64-byte aligned flat state buffer for the given layout and size.
    pub fn new(size: usize, layout: ShardLayout) -> Result<Self, ComputeApiError> {
        if size == 0 {
            return Ok(Self {
                raw_ptr: std::ptr::null_mut(),
                size: 0,
                layout,
            });
        }

        // Allocate memory aligned to 64 bytes (size of CPU cache line)
        // to prevent false sharing and ensure AVX/SIMD compatibility (INV-COMPUTE-CPU-002).
        let alloc_layout = Layout::from_size_align(size, 64)
            .map_err(|_| ComputeApiError::OutOfMemory)?;

        let ptr = unsafe { alloc_zeroed(alloc_layout) };
        if ptr.is_null() {
            return Err(ComputeApiError::OutOfMemory);
        }

        Ok(Self {
            raw_ptr: ptr,
            size,
            layout,
        })
    }

    /// Returns a mutable slice representation of the raw aligned memory buffer.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.size == 0 || self.raw_ptr.is_null() {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.raw_ptr, self.size) }
        }
    }

    /// Returns an immutable slice representation of the raw aligned memory buffer.
    pub fn as_slice(&self) -> &[u8] {
        if self.size == 0 || self.raw_ptr.is_null() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.raw_ptr, self.size) }
        }
    }

    /// Extracts non-overlapping mutable slices for key neuron arrays (voltage, flags, threshold_offset, timers).
    ///
    /// # Safety
    /// The caller must guarantee exclusive access to the memory shard buffer during the execution of this call.
    pub unsafe fn extract_soa_slices<'a>(&self) -> (
        &'a mut [i32],
        &'a mut [u8],
        &'a mut [i32],
        &'a mut [u8],
    ) {
        let offsets = layout::compute_state_offsets(self.layout.padded_n as usize);

        unsafe {
            let soma_voltage_ptr = self.raw_ptr.add(offsets.soma_voltage) as *mut i32;
            let flags_ptr = self.raw_ptr.add(offsets.flags) as *mut u8;
            let threshold_offset_ptr = self.raw_ptr.add(offsets.threshold_offset) as *mut i32;
            let timers_ptr = self.raw_ptr.add(offsets.timers) as *mut u8;

            let padded_n = self.layout.padded_n as usize;

            (
                std::slice::from_raw_parts_mut(soma_voltage_ptr, padded_n),
                std::slice::from_raw_parts_mut(flags_ptr, padded_n),
                std::slice::from_raw_parts_mut(threshold_offset_ptr, padded_n),
                std::slice::from_raw_parts_mut(timers_ptr, padded_n),
            )
        }
    }
}

impl Drop for ShardCpuResources {
    fn drop(&mut self) {
        if self.size > 0 && !self.raw_ptr.is_null() {
            // Reconstruct the exact Layout used for allocation and deallocate manually
            if let Ok(alloc_layout) = Layout::from_size_align(self.size, 64) {
                unsafe {
                    dealloc(self.raw_ptr, alloc_layout);
                }
            }
        }
    }
}

unsafe impl Send for ShardCpuResources {}
unsafe impl Sync for ShardCpuResources {}
