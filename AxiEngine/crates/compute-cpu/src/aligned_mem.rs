//! Private 64-byte aligned heap memory allocation module.

use core::slice;
use std::alloc::{alloc_zeroed, dealloc, Layout};

/// Heap-allocated raw byte buffer aligned to 64-byte (`CACHE_LINE_BYTES`) boundaries.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: Layout,
}

// SAFETY: AlignedBuffer owns unique heap-allocated memory and does not share aliasable pointers across threads.
unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

impl AlignedBuffer {
    /// Allocates a zero-initialized buffer of `size` bytes with 64-byte alignment.
    ///
    /// Returns `Err(())` if layout construction or memory allocation fails.
    #[allow(clippy::result_unit_err)]
    pub fn new(size: usize) -> Result<Self, ()> {
        if size == 0 {
            let layout = Layout::from_size_align(0, 64).map_err(|_| ())?;
            return Ok(Self {
                ptr: core::ptr::null_mut(),
                layout,
            });
        }
        let layout = Layout::from_size_align(size, 64).map_err(|_| ())?;
        // SAFETY: layout is constructed with valid non-zero size and valid alignment 64.
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(());
        }
        Ok(Self { ptr, layout })
    }

    /// Returns a shared slice view of the aligned memory buffer.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        if self.ptr.is_null() || self.layout.size() == 0 {
            &[]
        } else {
            // SAFETY: ptr is guaranteed non-null, aligned to 64 bytes, and valid for layout.size() bytes.
            unsafe { slice::from_raw_parts(self.ptr, self.layout.size()) }
        }
    }

    /// Returns a mutable slice view of the aligned memory buffer.
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        if self.ptr.is_null() || self.layout.size() == 0 {
            &mut []
        } else {
            // SAFETY: ptr is guaranteed non-null, aligned to 64 bytes, uniquely owned, and valid for layout.size() bytes.
            unsafe { slice::from_raw_parts_mut(self.ptr, self.layout.size()) }
        }
    }

    /// Returns the physical size in bytes of the allocated buffer.
    #[inline]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.layout.size()
    }

    /// Returns `true` if the buffer has length 0.
    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.layout.size() == 0
    }

    /// Returns the raw pointer address to verify physical 64-byte alignment in tests.
    #[inline]
    #[allow(dead_code)]
    pub fn ptr_address(&self) -> usize {
        self.ptr as usize
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.layout.size() > 0 {
            // SAFETY: ptr was allocated with alloc_zeroed using self.layout and has not been freed yet.
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_64b_alignment() {
        for &size in &[1, 64, 65, 4096] {
            let buf = AlignedBuffer::new(size).expect("allocation failed");
            assert_eq!(
                buf.ptr_address() % 64,
                0,
                "Buffer of size {} must be 64-byte aligned",
                size
            );
            assert_eq!(buf.len(), size);
            assert!(!buf.is_empty());
        }
    }
}
