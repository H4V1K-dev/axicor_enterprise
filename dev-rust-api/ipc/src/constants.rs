//! IPC structural, directory, and naming prefix constants.

/// Night Phase spin wait timeout limit in seconds.
pub const NIGHT_PHASE_TIMEOUT_SECS: u64 = 10;

/// L2 cache line memory alignment constraint in bytes.
pub const SHM_ALIGNMENT: usize = 64;

/// OS hardware memory page alignment constraint in bytes.
pub const OS_PAGE_ALIGNMENT: usize = 4096;

/// Linux default directory for shared memory files.
pub const LINUX_SHM_DIR: &str = "/dev/shm";

/// Linux default directory for Unix Domain Sockets.
pub const LINUX_UDS_DIR: &str = "/tmp";

/// File prefix for State Shared Memory databases.
pub const FILE_PREFIX_STATE: &str = "axicor_state_";
