//! Shadow shared memory manager for zero-copy replication.

use crate::error::IpcError;
use std::sync::atomic::{AtomicU8, Ordering};

/// Manager for active/backup shadow shared memory regions.
pub struct ShadowShmManager {
    pub shadow_a: crate::shm::MappedShm,
    pub shadow_b: crate::shm::MappedShm,
    #[cfg(target_os = "linux")]
    pub fd_a: std::os::unix::io::RawFd,
    #[cfg(target_os = "linux")]
    pub fd_b: std::os::unix::io::RawFd,
    pub latest_written: AtomicU8,
}

impl ShadowShmManager {
    /// Cold starts and allocates the shadow memory regions.
    ///
    /// Computes the required size via `layout::shm_size` and sets up both segments.
    #[cfg(target_os = "linux")]
    pub fn allocate_shadows(zone_hash: u32, padded_n: usize) -> Result<Self, IpcError> {
        use std::os::unix::io::FromRawFd;

        let shm_size = layout::shm_size(padded_n);

        // Path for "a"
        let path_a = crate::utils::shadow_file_path(zone_hash, "a");
        let path_str_a = path_a.to_str().ok_or_else(|| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;
        let c_path_a = std::ffi::CString::new(path_str_a).map_err(|_| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path"))
        })?;

        // Path for "b"
        let path_b = crate::utils::shadow_file_path(zone_hash, "b");
        let path_str_b = path_b.to_str().ok_or_else(|| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;
        let c_path_b = std::ffi::CString::new(path_str_b).map_err(|_| {
            IpcError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Null byte in path"))
        })?;

        let fd_a = crate::platform::create_clean_shm(&c_path_a, shm_size)?;
        let fd_a_for_file = unsafe { libc::dup(fd_a) };
        if fd_a_for_file < 0 {
            unsafe { libc::close(fd_a); }
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        let file_a = unsafe { std::fs::File::from_raw_fd(fd_a_for_file) };
        let shadow_a = crate::shm::MappedShm::new(&file_a, shm_size)?;

        let fd_b = crate::platform::create_clean_shm(&c_path_b, shm_size)?;
        let fd_b_for_file = unsafe { libc::dup(fd_b) };
        if fd_b_for_file < 0 {
            unsafe {
                libc::close(fd_a);
                libc::close(fd_b);
            }
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        let file_b = unsafe { std::fs::File::from_raw_fd(fd_b_for_file) };
        let shadow_b = crate::shm::MappedShm::new(&file_b, shm_size)?;

        Ok(Self {
            shadow_a,
            shadow_b,
            fd_a,
            fd_b,
            latest_written: AtomicU8::new(0),
        })
    }

    /// Cold starts and allocates the shadow memory regions.
    ///
    /// Computes the required size via `layout::shm_size` and sets up both segments.
    #[cfg(not(target_os = "linux"))]
    pub fn allocate_shadows(zone_hash: u32, padded_n: usize) -> Result<Self, IpcError> {
        let shm_size = layout::shm_size(padded_n);

        // Path for "a"
        let path_a = crate::utils::shadow_file_path(zone_hash, "a");
        if let Some(parent) = path_a.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&path_a);
        let file_a = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path_a)?;
        file_a.set_len(shm_size as u64)?;
        let shadow_a = crate::shm::MappedShm::new(&file_a, shm_size)?;

        // Path for "b"
        let path_b = crate::utils::shadow_file_path(zone_hash, "b");
        if let Some(parent) = path_b.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&path_b);
        let file_b = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path_b)?;
        file_b.set_len(shm_size as u64)?;
        let shadow_b = crate::shm::MappedShm::new(&file_b, shm_size)?;

        Ok(Self {
            shadow_a,
            shadow_b,
            latest_written: AtomicU8::new(0),
        })
    }

    /// Replicates the latest written buffer content to the destination TCP socket.
    ///
    /// # Invariants and Edge Cases
    /// - **R-006**: Evacuation during write. Loads the active buffer index with
    ///   `Ordering::Acquire` to guarantee visibility of all written bytes before replicating.
    #[cfg(target_os = "linux")]
    pub fn replicate_latest(
        &self,
        tcp_socket_fd: std::os::unix::io::RawFd,
        count: usize,
    ) -> Result<usize, IpcError> {
        let latest = self.latest_written.load(Ordering::Acquire);
        let in_fd = if latest == 0 { self.fd_a } else { self.fd_b };
        let mut offset = 0;
        crate::platform::zero_copy_sendfile(tcp_socket_fd, in_fd, &mut offset, count)
    }

    /// Replicates the latest written buffer content to the destination TCP socket.
    ///
    /// # Invariants and Edge Cases
    /// - **R-006**: Evacuation during write. Stubs out zero-copy TCP replication on Windows.
    #[cfg(target_os = "windows")]
    pub fn replicate_latest(
        &self,
        _tcp_socket_handle: std::os::windows::io::RawHandle,
        _count: usize,
    ) -> Result<usize, IpcError> {
        Err(IpcError::ReplicationFailed)
    }

    /// Atomically updates the active written buffer index.
    ///
    /// Stores the index using `Ordering::Release` to guarantee that all writes to the buffer
    /// are visible to a thread loading the index with Acquire.
    pub fn mark_written(&self, buffer_idx: u8) {
        self.latest_written.store(buffer_idx, Ordering::Release);
    }
}

#[cfg(target_os = "linux")]
impl Drop for ShadowShmManager {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd_a);
            libc::close(self.fd_b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_allocation_and_toggling() {
        let zone_hash = 123;
        let padded_n = 1024;
        
        let manager = ShadowShmManager::allocate_shadows(zone_hash, padded_n).expect("Failed to allocate shadows");
        assert_eq!(manager.latest_written.load(Ordering::Relaxed), 0);
        
        manager.mark_written(1);
        assert_eq!(manager.latest_written.load(Ordering::Relaxed), 1);
        
        manager.mark_written(0);
        assert_eq!(manager.latest_written.load(Ordering::Relaxed), 0);
        
        // Clean up shadow files if they exist
        let path_a = crate::utils::shadow_file_path(zone_hash, "a");
        let path_b = crate::utils::shadow_file_path(zone_hash, "b");
        let _ = std::fs::remove_file(path_a);
        let _ = std::fs::remove_file(path_b);
    }
}
