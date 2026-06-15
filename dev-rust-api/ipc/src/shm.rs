//! Shared memory validation and Zero-Copy slice extraction.

use crate::error::IpcError;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Validates the raw SHM pointer alignment and header C-ABI contract.
///
/// # Panics
/// Panics with `FATAL C-ABI BOUNDARY` prefix if `ptr` is not 64-byte aligned (INV-IPC-001).
///
/// # Errors
/// Returns `IpcError::InvalidHeaderMagic` if magic, version, or dendrite_slots
/// do not match the expected C-ABI contract (INV-CROSS-008 / E-030).
///
/// # Safety
/// Caller must guarantee that `ptr` points to a valid, mapped region of at least
/// `size_of::<layout::ShmHeader>()` bytes that will remain live for `'static`.
pub fn validate_shm_header(ptr: *mut u8) -> Result<&'static mut layout::ShmHeader, IpcError> {
    // INV-IPC-001: L2 cache line alignment guard
    if ptr as usize % crate::constants::SHM_ALIGNMENT != 0 {
        panic!(
            "FATAL C-ABI BOUNDARY: Raw SHM pointer {:p} is not 64-byte aligned",
            ptr
        );
    }

    let hdr = unsafe { &mut *(ptr as *mut layout::ShmHeader) };

    // INV-CROSS-008: C-ABI version match
    if hdr.magic != layout::SHM_MAGIC
        || hdr.version != layout::SHM_VERSION
        || hdr.dendrite_slots != layout::MAX_DENDRITES as u32
    {
        return Err(IpcError::InvalidHeaderMagic);
    }

    Ok(hdr)
}

/// Extracts 5 typed slices from a flat C-ABI shared memory buffer in O(1).
///
/// Implements ipc_spec.md §6.8 Zero-Copy slice extraction.
///
/// # Safety
/// Caller must guarantee:
/// - `shm_ptr` points to a valid mapped region covering all offsets referenced by `hdr`.
/// - Offsets and counts in `hdr` are correct and within bounds.
/// - No other mutable references to the same memory regions exist.
pub unsafe fn extract_slices(
    shm_ptr: *mut u8,
    hdr: &layout::ShmHeader,
) -> (
    &mut [i32],
    &mut [u32],
    &[u8],
    &mut [wire::AxonHandoverEvent],
    &mut [wire::AxonHandoverPrune],
) {
    let padded_n = hdr.padded_n as usize;
    let slots = hdr.dendrite_slots as usize;

    unsafe {
        let w_ptr = shm_ptr.add(hdr.weights_offset as usize) as *mut i32;
        let t_ptr = shm_ptr.add(hdr.targets_offset as usize) as *mut u32;
        let f_ptr = shm_ptr.add(hdr.flags_offset as usize) as *const u8;

        let h_ptr = shm_ptr.add(hdr.handovers_offset as usize) as *mut wire::AxonHandoverEvent;
        let p_ptr = shm_ptr.add(hdr.prunes_offset as usize) as *mut wire::AxonHandoverPrune;

        (
            std::slice::from_raw_parts_mut(w_ptr, slots * padded_n),
            std::slice::from_raw_parts_mut(t_ptr, slots * padded_n),
            std::slice::from_raw_parts(f_ptr, padded_n),
            std::slice::from_raw_parts_mut(h_ptr, hdr.handovers_count as usize),
            std::slice::from_raw_parts_mut(p_ptr, hdr.prunes_count as usize),
        )
    }
}

/// Lock-free state machine for coordinating Night Phase transitions.
///
/// It wraps a raw pointer to an atomic state variable stored inside
/// the shared memory layout.
pub struct ShmStateMachine {
    pub state_ptr: *const AtomicU8,
}

unsafe impl Send for ShmStateMachine {}
unsafe impl Sync for ShmStateMachine {}

impl ShmStateMachine {
    /// Creates a new `ShmStateMachine` wrapping a raw pointer to an atomic state variable.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid and points to a live `AtomicU8`
    /// instance for the duration of this machine's usage.
    pub unsafe fn new(state_ptr: *const AtomicU8) -> Self {
        Self { state_ptr }
    }

    /// Spin-waits until the state at `state_ptr` matches the `expected` state value,
    /// or until the specified `timeout` has elapsed.
    ///
    /// This method enforces invariant INV-IPC-003 by implementing a timeout limit
    /// to avoid infinite loops/deadlocks when one of the processes halts or crashes.
    ///
    /// # Errors
    /// Returns `IpcError::Timeout` if the state does not transition to `expected` within the timeout duration.
    pub fn wait_for_state(&self, expected: u8, timeout: Duration) -> Result<(), IpcError> {
        let start = Instant::now();
        unsafe {
            loop {
                // INV-IPC-003: Read with Acquire ordering to establish synchronization
                // with the writer process that modified the state.
                let current = (*self.state_ptr).load(Ordering::Acquire);
                if current == expected {
                    return Ok(());
                }
                if start.elapsed() > timeout {
                    return Err(IpcError::Timeout);
                }
                std::hint::spin_loop();
            }
        }
    }

    /// Performs an atomic compare-and-swap (CAS) transition from `expected` to `new` state.
    ///
    /// This method enforces state transition rules and prevents data races, respecting R-002.
    ///
    /// # Errors
    /// Returns `IpcError::StateConflict` if the current state value does not match `expected`.
    pub fn transition(&self, expected: u8, new: u8) -> Result<(), IpcError> {
        unsafe {
            (*self.state_ptr)
                .compare_exchange(
                    expected,
                    new,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                )
                .map(|_| ())
                .map_err(|_| IpcError::StateConflict)
        }
    }

    /// Transitions the state machine from `Idle` to `NightStart` to prepare for the daemon.
    ///
    /// # Errors
    /// Returns `IpcError::StateConflict` if the current state is not `Idle`.
    pub fn prepare_for_daemon(&self) -> Result<(), IpcError> {
        self.transition(layout::ShmState::Idle as u8, layout::ShmState::NightStart as u8)
    }

    /// Spin-waits until the daemon has finished processing, expecting the state to transit to `NightDone`.
    ///
    /// # Errors
    /// Returns `IpcError::Timeout` if the daemon does not complete before the timeout.
    pub fn wait_for_daemon(&self, timeout: Duration) -> Result<(), IpcError> {
        self.wait_for_state(layout::ShmState::NightDone as u8, timeout)
    }

    /// Unconditionally forces the state machine into the `Error` state.
    ///
    /// Uses Release ordering to ensure all previous writes are visible before the state update.
    pub fn mark_error(&self) {
        unsafe {
            (*self.state_ptr).store(layout::ShmState::Error as u8, Ordering::Release);
        }
    }
}

/// Ping-pong buffer swapchain for conflict-free input transferring.
///
/// This structure guarantees conflict-free concurrent access between the GPU reader
/// and the network writer by swapping buffer pointers atomically.
/// It satisfies invariant **INV-IPC-005** by utilizing `Ordering::AcqRel` for pointer swapping,
/// preventing data races (**R-003**) and ensuring memory visibility.
pub struct InputSwapchain {
    pub active_ptr: AtomicPtr<u8>,
    pub back_ptr: AtomicPtr<u8>,
}

impl InputSwapchain {
    /// Creates a new `InputSwapchain` with the given active and back buffer pointers.
    pub fn new(active: *mut u8, back: *mut u8) -> Self {
        Self {
            active_ptr: AtomicPtr::new(active),
            back_ptr: AtomicPtr::new(back),
        }
    }

    /// Swaps the active and back buffer pointers atomically.
    ///
    /// The swap uses `Ordering::AcqRel` on `active_ptr` to publish the new active pointer and
    /// acquire any writes performed on the back pointer, satisfying **R-003**.
    pub fn swap(&self) -> *mut u8 {
        self.active_ptr.swap(self.back_ptr.load(Ordering::Relaxed), Ordering::AcqRel)
    }
}

/// Lock-free swapchain tracking the latest ready batch for output aggregation.
///
/// This structure tracks the index of the latest completed batch produced by the GPU
/// and allows async networking readers to consume it. If the network reader is slow,
/// older batches are overwritten by newer ones, handling buffer saturation (**E-036**).
/// It uses a compare-and-swap (CAS) loop to prevent races among multiple async readers (**R-010**).
pub struct OutputSwapchain {
    pub latest_ready: AtomicU32,
    pub last_read: AtomicU32,
}

impl OutputSwapchain {
    /// Creates a new `OutputSwapchain` with indices initialized to 0.
    pub fn new() -> Self {
        Self {
            latest_ready: AtomicU32::new(0),
            last_read: AtomicU32::new(0),
        }
    }

    /// Notifies the swapchain that the GPU has finished writing a batch.
    ///
    /// Stores the batch index using `Ordering::Release` so that readers can safely observe
    /// the written data.
    pub fn notify_gpu_batch_done(&self, batch_idx: u32) {
        self.latest_ready.store(batch_idx, Ordering::Release);
    }

    /// Attempts to read the index of the latest ready batch.
    ///
    /// Loads the latest ready index using `Ordering::Acquire` and uses a CAS loop
    /// with `compare_exchange_weak` to update `last_read`, preventing data races (**R-010**)
    /// between multiple async readers.
    pub fn try_read_latest(&self) -> Option<u32> {
        let mut current_read = self.last_read.load(Ordering::Relaxed);
        loop {
            let latest = self.latest_ready.load(Ordering::Acquire);
            if latest > current_read {
                match self.last_read.compare_exchange_weak(
                    current_read,
                    latest,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(latest),
                    Err(actual) => current_read = actual,
                }
            } else {
                return None;
            }
        }
    }
}

impl Default for OutputSwapchain {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-mapped raw buffer wrapper representing the shared memory region.
///
/// This structure encapsulates `memmap2::MmapMut` and enforces hardware
/// pointer alignment constraints (**INV-IPC-001**).
pub struct MappedShm {
    pub mmap: memmap2::MmapMut,
}

impl MappedShm {
    /// Creates a new `MappedShm` mapping the provided file.
    ///
    /// It enforces L2 cache line alignment constraint (**INV-IPC-001**) by verifying
    /// that the starting memory address is a multiple of 64.
    ///
    /// # Errors
    /// Returns `IpcError::Io` on memory mapping failure.
    pub fn new(file: &std::fs::File, _size: usize) -> Result<Self, IpcError> {
        let mmap = unsafe { memmap2::MmapMut::map_mut(file)? };
        if mmap.as_ptr() as usize % 64 != 0 {
            panic!("FATAL C-ABI BOUNDARY: OS mmap pointer is not 64-byte aligned");
        }
        Ok(Self { mmap })
    }
}

/// Shared memory manager wrapping the business state files and metadata.
pub struct ShmManager {
    pub mapped: MappedShm,
}

impl ShmManager {
    #[cfg(target_os = "linux")]
    /// Cold starts the shared memory region on Linux.
    ///
    /// This method enforces variant **INV-IPC-002** by ensuring old, potentially
    /// poisoned shared memory files are unlinked prior to segment creation.
    ///
    /// # Errors
    /// Returns `IpcError` if clean creation, mapping, or size allocation fails.
    pub fn create_cold(zone_hash: u32, padded_n: usize) -> Result<Self, IpcError> {
        use std::os::unix::io::FromRawFd;

        // Calculate size dynamically
        let shm_size = layout::shm_size(padded_n);
        let offsets = layout::compute_state_offsets(padded_n);
        let required_size = 128 + offsets.total_size + 10000 * 20 + 10000 * 12;
        let final_shm_size = shm_size.max(required_size);

        let path = crate::utils::shm_file_path(zone_hash);
        let path_str = path.to_str().ok_or_else(|| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path"))
        })?;

        let fd = crate::platform::create_clean_shm(&c_path, final_shm_size)?;
        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        let mapped = MappedShm::new(&file, final_shm_size)?;
        let mut this = Self { mapped };
        this.initialize_header(padded_n);

        Ok(this)
    }

    #[cfg(not(target_os = "linux"))]
    /// Cold starts the shared memory region on non-Linux platforms (e.g. Windows).
    ///
    /// # Errors
    /// Returns `IpcError` if file creation, resizing, or mapping fails.
    pub fn create_cold(zone_hash: u32, padded_n: usize) -> Result<Self, IpcError> {
        // Calculate size dynamically
        let shm_size = layout::shm_size(padded_n);
        let offsets = layout::compute_state_offsets(padded_n);
        let required_size = 128 + offsets.total_size + 10000 * 20 + 10000 * 12;
        let final_shm_size = shm_size.max(required_size);

        let path = crate::utils::shm_file_path(zone_hash);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Emulate clean unlink by removing any existing file
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;

        file.set_len(final_shm_size as u64)?;

        let mapped = MappedShm::new(&file, final_shm_size)?;
        let mut this = Self { mapped };
        this.initialize_header(padded_n);

        Ok(this)
    }

    /// Initializes the `ShmHeader` defaults and field offsets in the mapped buffer.
    fn initialize_header(&mut self, padded_n: usize) {
        let hdr = unsafe { &mut *(self.mapped.mmap.as_mut_ptr() as *mut layout::ShmHeader) };
        hdr.magic = layout::SHM_MAGIC;
        hdr.version = layout::SHM_VERSION;
        hdr.state = layout::ShmState::Idle as u8;
        hdr.padded_n = padded_n as u32;
        hdr.dendrite_slots = layout::MAX_DENDRITES as u32;

        let offsets = layout::compute_state_offsets(padded_n);
        hdr.weights_offset = (offsets.dendrite_weights + 128) as u32;
        hdr.targets_offset = (offsets.dendrite_targets + 128) as u32;
        hdr.flags_offset = (offsets.flags + 128) as u32;
        hdr.voltage_offset = (offsets.soma_voltage + 128) as u32;
        hdr.threshold_offset_offset = (offsets.threshold_offset + 128) as u32;
        hdr.timers_offset = (offsets.timers + 128) as u32;

        let handovers_offset = 128 + offsets.total_size;
        hdr.handovers_offset = handovers_offset as u32;
        hdr.handovers_count = 10000;

        let prunes_offset = handovers_offset + 10000 * 20;
        hdr.prunes_offset = prunes_offset as u32;
        hdr.prunes_count = 10000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 64-byte-aligned heap-allocated buffer for SHM simulation in tests.
    #[repr(C, align(64))]
    struct AlignedBlock {
        data: [u8; 256],
    }

    /// Helper: allocate a 64-byte-aligned zeroed buffer on the heap.
    fn aligned_buffer() -> Box<AlignedBlock> {
        Box::new(AlignedBlock { data: [0u8; 256] })
    }

    /// Write a valid ShmHeader at the start of the buffer.
    fn write_valid_header(buf: &mut [u8]) {
        assert!(buf.len() >= std::mem::size_of::<layout::ShmHeader>());
        let hdr = unsafe { &mut *(buf.as_mut_ptr() as *mut layout::ShmHeader) };
        hdr.magic = layout::SHM_MAGIC;
        hdr.version = layout::SHM_VERSION;
        hdr.dendrite_slots = layout::MAX_DENDRITES as u32;
        hdr.padded_n = 0;
    }

    #[test]
    #[should_panic(expected = "FATAL C-ABI BOUNDARY")]
    fn test_shm_alignment_validation() {
        // Allocate aligned buffer, then offset by 1 to misalign
        let buf = aligned_buffer();
        let misaligned_ptr = unsafe { buf.data.as_ptr().add(1) as *mut u8 };
        // Must panic
        let _ = validate_shm_header(misaligned_ptr);
    }

    #[test]
    fn test_shm_header_validation_abi() {
        let mut buf = aligned_buffer();

        // Case 1: bad magic
        {
            write_valid_header(&mut buf.data);
            let hdr = unsafe { &mut *(buf.data.as_mut_ptr() as *mut layout::ShmHeader) };
            hdr.magic = 0xDEADBEEF;
            let res = validate_shm_header(buf.data.as_mut_ptr());
            assert!(res.is_err());
            match res.unwrap_err() {
                IpcError::InvalidHeaderMagic => {}
                other => panic!("Expected InvalidHeaderMagic, got {:?}", other),
            }
        }

        // Case 2: bad version
        {
            write_valid_header(&mut buf.data);
            let hdr = unsafe { &mut *(buf.data.as_mut_ptr() as *mut layout::ShmHeader) };
            hdr.version = 99;
            let res = validate_shm_header(buf.data.as_mut_ptr());
            assert!(res.is_err());
            match res.unwrap_err() {
                IpcError::InvalidHeaderMagic => {}
                other => panic!("Expected InvalidHeaderMagic, got {:?}", other),
            }
        }

        // Case 3: bad dendrite_slots
        {
            write_valid_header(&mut buf.data);
            let hdr = unsafe { &mut *(buf.data.as_mut_ptr() as *mut layout::ShmHeader) };
            hdr.dendrite_slots = 64;
            let res = validate_shm_header(buf.data.as_mut_ptr());
            assert!(res.is_err());
            match res.unwrap_err() {
                IpcError::InvalidHeaderMagic => {}
                other => panic!("Expected InvalidHeaderMagic, got {:?}", other),
            }
        }

        // Case 4: valid header
        {
            write_valid_header(&mut buf.data);
            let res = validate_shm_header(buf.data.as_mut_ptr());
            assert!(res.is_ok());
            let hdr = res.unwrap();
            assert_eq!(hdr.magic, layout::SHM_MAGIC);
            assert_eq!(hdr.version, layout::SHM_VERSION);
            assert_eq!(hdr.dendrite_slots, layout::MAX_DENDRITES as u32);
        }
    }

    #[test]
    fn test_shm_state_machine_transition_success() {
        let state = AtomicU8::new(layout::ShmState::Idle as u8);
        let sm = unsafe { ShmStateMachine::new(&state as *const AtomicU8) };

        // Idle -> NightStart (prepare_for_daemon)
        assert!(sm.prepare_for_daemon().is_ok());
        assert_eq!(state.load(Ordering::Relaxed), layout::ShmState::NightStart as u8);

        // NightStart -> Sprouting (transition)
        assert!(sm.transition(layout::ShmState::NightStart as u8, layout::ShmState::Sprouting as u8).is_ok());
        assert_eq!(state.load(Ordering::Relaxed), layout::ShmState::Sprouting as u8);

        // Sprouting -> NightDone (transition)
        assert!(sm.transition(layout::ShmState::Sprouting as u8, layout::ShmState::NightDone as u8).is_ok());
        assert_eq!(state.load(Ordering::Relaxed), layout::ShmState::NightDone as u8);

        // wait_for_daemon should succeed immediately or within timeout
        assert!(sm.wait_for_daemon(Duration::from_millis(50)).is_ok());

        // mark_error
        sm.mark_error();
        assert_eq!(state.load(Ordering::Relaxed), layout::ShmState::Error as u8);
    }

    #[test]
    fn test_shm_state_machine_conflict() {
        let state = AtomicU8::new(layout::ShmState::Idle as u8);
        let sm = unsafe { ShmStateMachine::new(&state as *const AtomicU8) };

        // Attempting to transition from NightStart to Sprouting when state is Idle should fail
        let res = sm.transition(layout::ShmState::NightStart as u8, layout::ShmState::Sprouting as u8);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), IpcError::StateConflict));
        assert_eq!(state.load(Ordering::Relaxed), layout::ShmState::Idle as u8);
    }

    #[test]
    fn test_shm_state_machine_timeout() {
        let state = AtomicU8::new(layout::ShmState::Idle as u8);
        let sm = unsafe { ShmStateMachine::new(&state as *const AtomicU8) };

        // Wait for NightDone while state is Idle, timeout 1ms
        let res = sm.wait_for_state(layout::ShmState::NightDone as u8, Duration::from_millis(1));
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), IpcError::Timeout));
    }

    #[test]
    fn test_input_swapchain_swap() {
        let mut buf_a = 0u8;
        let mut buf_b = 0u8;
        let ptr_a = &mut buf_a as *mut u8;
        let ptr_b = &mut buf_b as *mut u8;

        let swapchain = InputSwapchain::new(ptr_a, ptr_b);
        assert_eq!(swapchain.active_ptr.load(Ordering::Relaxed), ptr_a);
        assert_eq!(swapchain.back_ptr.load(Ordering::Relaxed), ptr_b);

        // First swap: active should become ptr_b
        let old_active = swapchain.swap();
        assert_eq!(old_active, ptr_a);
        assert_eq!(swapchain.active_ptr.load(Ordering::Relaxed), ptr_b);

        // Change back_ptr to a new buffer
        let mut buf_c = 0u8;
        let ptr_c = &mut buf_c as *mut u8;
        swapchain.back_ptr.store(ptr_c, Ordering::Relaxed);

        // Second swap: active should become ptr_c, old active returned should be ptr_b
        let old_active = swapchain.swap();
        assert_eq!(old_active, ptr_b);
        assert_eq!(swapchain.active_ptr.load(Ordering::Relaxed), ptr_c);
    }

    #[test]
    fn test_output_swapchain_saturating_drop() {
        let swapchain = OutputSwapchain::new();

        // 1. Initial read should return None
        assert_eq!(swapchain.try_read_latest(), None);

        // 2. Notify batch 1 done
        swapchain.notify_gpu_batch_done(1);

        // 3. First read should return Some(1)
        assert_eq!(swapchain.try_read_latest(), Some(1));

        // 4. Repeated read should return None
        assert_eq!(swapchain.try_read_latest(), None);

        // 5. Saturating Drop: GPU finishes batch 2, and then batch 3, before the network reads it
        swapchain.notify_gpu_batch_done(2);
        swapchain.notify_gpu_batch_done(3);

        // 6. Network reader should receive batch 3 (batch 2 is dropped/skipped)
        assert_eq!(swapchain.try_read_latest(), Some(3));

        // 7. Repeated read should return None
        assert_eq!(swapchain.try_read_latest(), None);
    }

    #[test]
    fn test_shm_manager_create_cold() {
        let manager = ShmManager::create_cold(12345, 128);
        assert!(manager.is_ok());
        let manager = manager.unwrap();

        let hdr = unsafe { &*(manager.mapped.mmap.as_ptr() as *const layout::ShmHeader) };
        assert_eq!(hdr.magic, layout::SHM_MAGIC);
        assert_eq!(hdr.version, layout::SHM_VERSION);
        assert_eq!(hdr.state, layout::ShmState::Idle as u8);
        assert_eq!(hdr.padded_n, 128);
        assert_eq!(hdr.dendrite_slots, layout::MAX_DENDRITES as u32);
        
        // Check slice extraction is valid on this buffer
        let (w, t, f, h, p) = unsafe { extract_slices(manager.mapped.mmap.as_ptr() as *mut u8, hdr) };
        assert_eq!(w.len(), 128 * 128);
        assert_eq!(t.len(), 128 * 128);
        assert_eq!(f.len(), 128);
        assert_eq!(h.len(), 10000);
        assert_eq!(p.len(), 10000);
    }
}
