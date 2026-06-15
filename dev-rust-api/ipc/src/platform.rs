//! Platform-dependent shared memory operations.

#[cfg(target_os = "linux")]
/// Creates a clean, zeroed shared memory region of the specified size on Linux.
///
/// This satisfies invariant **INV-IPC-002** by unlinking any existing
/// shared memory segment under the same name before creating a new one.
///
/// # Errors
/// Returns `crate::IpcError::Io` if `shm_unlink`, `shm_open`, or `ftruncate` fails.
pub fn create_clean_shm(path: &std::ffi::CStr, size: usize) -> Result<std::os::unix::io::RawFd, crate::IpcError> {
    // INV-IPC-002: Evict potentially poisoned SHM from a previous run
    unsafe {
        libc::shm_unlink(path.as_ptr());
    }

    let fd = unsafe {
        libc::shm_open(
            path.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
            0o666,
        )
    };
    if fd < 0 {
        return Err(crate::IpcError::Io(std::io::Error::last_os_error()));
    }

    let res = unsafe { libc::ftruncate(fd, size as libc::off_t) };
    if res < 0 {
        let err = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(crate::IpcError::Io(err));
    }

    Ok(fd)
}

#[cfg(target_os = "linux")]
/// Perform a zero-copy sendfile transfer from a source file descriptor to a destination socket descriptor.
///
/// # Invariants and Edge Cases
/// - **INV-IPC-007**: Zero-Copy replication via kernel-space shadow replication using `sendfile`.
/// - **E-035**: Network congestion or connection reset during zero-copy replication.
///   If network is congested (`EAGAIN`) or connection is broken (`EPIPE`), returns `Err(IpcError::ReplicationFailed)`.
pub fn zero_copy_sendfile(
    out_fd: std::os::unix::io::RawFd,
    in_fd: std::os::unix::io::RawFd,
    offset: &mut libc::off_t,
    count: usize,
) -> Result<usize, crate::IpcError> {
    let res = unsafe { libc::sendfile(out_fd, in_fd, offset, count) };
    if res < 0 {
        let err = std::io::Error::last_os_error();
        let raw = err.raw_os_error().unwrap_or(0);
        // E-035: Сеть перегружена или обрыв соединения
        if raw == libc::EAGAIN || raw == libc::EPIPE {
            return Err(crate::IpcError::ReplicationFailed);
        }
        return Err(crate::IpcError::Io(err));
    }
    Ok(res as usize)
}

#[cfg(target_os = "windows")]
/// Placeholder for Windows platforms where `sendfile` is not natively supported.
///
/// # Invariants and Edge Cases
/// - **INV-IPC-007**: Zero-Copy replication via kernel-space shadow replication using `sendfile` (POSIX-specific).
/// - **E-035**: Returns `Err(IpcError::ReplicationFailed)` unconditionally.
pub fn zero_copy_sendfile(
    _out_handle: std::os::windows::io::RawHandle,
    _in_handle: std::os::windows::io::RawHandle,
    _offset: &mut i64,
    _count: usize,
) -> Result<usize, crate::IpcError> {
    Err(crate::IpcError::ReplicationFailed)
}
