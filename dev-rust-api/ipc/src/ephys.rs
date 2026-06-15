//! Electrophysiology management and 4-Phase Handshake transitions.

use crate::error::IpcError;
use std::sync::atomic::{AtomicU32, Ordering};

/// Electrophysiology shared memory buffer gateway.
pub struct EphysManager {
    pub mapped: crate::shm::MappedShm,
    pub ptr: *mut layout::EphysShm,
}

unsafe impl Send for EphysManager {}
unsafe impl Sync for EphysManager {}

impl EphysManager {
    /// Cold starts the electrophysiology shared memory region on Linux.
    #[cfg(target_os = "linux")]
    pub fn create_cold(zone_hash: u32) -> Result<Self, IpcError> {
        use std::os::unix::io::FromRawFd;

        let size = 640_192;
        let path = crate::utils::ephys_shm_path(zone_hash);
        let path_str = path.to_str().ok_or_else(|| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path"))
        })?;

        let fd = crate::platform::create_clean_shm(&c_path, size)?;
        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        let mapped = crate::shm::MappedShm::new(&file, size)?;
        let ptr = mapped.mmap.as_ptr() as *mut layout::EphysShm;

        let ephys = unsafe { &mut *ptr };
        ephys.magic = 0x45504859; // EPHY
        ephys.state = 0; // Idle

        Ok(Self { mapped, ptr })
    }

    /// Cold starts the electrophysiology shared memory region on non-Linux platforms (e.g. Windows).
    #[cfg(not(target_os = "linux"))]
    pub fn create_cold(zone_hash: u32) -> Result<Self, IpcError> {
        let size = 640_192;
        let path = crate::utils::ephys_shm_path(zone_hash);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.set_len(size as u64)?;

        let mapped = crate::shm::MappedShm::new(&file, size)?;
        let ptr = mapped.mmap.as_ptr() as *mut layout::EphysShm;

        let ephys = unsafe { &mut *ptr };
        ephys.magic = 0x45504859; // EPHY
        ephys.state = 0; // Idle

        Ok(Self { mapped, ptr })
    }

    /// Enforces the electrophysiology 4-step atomic handshake with the Python SDK.
    ///
    /// # Invariants and Edge Cases
    /// - **INV-IPC-006**: Ephys 4-Step Handshake - Transition strictly follows:
    ///   Idle (0) -> Trigger (1) -> Busy (2) -> Done (3) -> Idle (0).
    ///   Uses `Acquire` ordering to read state, and `Release` to transition states.
    pub fn lock_and_execute_ephys<F>(&self, mut orchestrator_task: F)
    where
        F: FnMut(&mut layout::EphysShm),
    {
        let ephys = unsafe { &mut *self.ptr };
        let state_ptr = &ephys.state as *const u32 as *const AtomicU32;

        // INV-IPC-006: Read state with Acquire memory barrier
        if unsafe { (*state_ptr).load(Ordering::Acquire) } == 1 {
            // Lock buffer for writing: transition to Busy (2) via Release
            unsafe { (*state_ptr).store(2, Ordering::Release); }

            // Execute local orchestrator tasks (e.g. inject currents, write out traces)
            orchestrator_task(ephys);

            // Relinquish control back to Python SDK: transition to Done (3) via Release
            unsafe { (*state_ptr).store(3, Ordering::Release); }
        }
    }

    /// Reset the handshake state unconditionally to Idle (0).
    ///
    /// # Invariants and Edge Cases
    /// - **D-005**: Ephys client degradation or crash. Unconditionally resets
    ///   the handshake state to Idle (0) using a `Release` barrier to unblock future connections.
    pub fn reset_ephys_state_on_timeout(&self) {
        let ephys = unsafe { &mut *self.ptr };
        let state_ptr = &ephys.state as *const u32 as *const AtomicU32;
        unsafe {
            (*state_ptr).store(0, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephys_handshake_transitions() {
        let zone_hash = 777;
        let manager = EphysManager::create_cold(zone_hash).expect("Failed to create EphysManager");
        
        let ephys = unsafe { &mut *manager.ptr };
        let state_ptr = &ephys.state as *const u32 as *const AtomicU32;
        
        // Initially should be 0 (Idle) and magic should be set correctly
        assert_eq!(ephys.magic, 0x45504859);
        assert_eq!(unsafe { (*state_ptr).load(Ordering::Relaxed) }, 0);
        
        // Test lock_and_execute_ephys when state is Idle (0) - task should NOT execute
        let mut executed = false;
        manager.lock_and_execute_ephys(|_| {
            executed = true;
        });
        assert!(!executed);
        assert_eq!(unsafe { (*state_ptr).load(Ordering::Relaxed) }, 0);
        
        // Simulate Python SDK setting state to Trigger (1)
        unsafe { (*state_ptr).store(1, Ordering::SeqCst); }
        
        // Test lock_and_execute_ephys when state is Trigger (1)
        manager.lock_and_execute_ephys(|e| {
            executed = true;
            // Inside task, state should be Busy (2)
            let current_state = &e.state as *const u32 as *const AtomicU32;
            assert_eq!(unsafe { (*current_state).load(Ordering::Relaxed) }, 2);
        });
        assert!(executed);
        // After execution, state should be Done (3)
        assert_eq!(unsafe { (*state_ptr).load(Ordering::Relaxed) }, 3);
        
        // Test reset_ephys_state_on_timeout
        manager.reset_ephys_state_on_timeout();
        assert_eq!(unsafe { (*state_ptr).load(Ordering::Relaxed) }, 0);
        
        // Clean up
        let path = crate::utils::ephys_shm_path(zone_hash);
        let _ = std::fs::remove_file(path);
    }
}
