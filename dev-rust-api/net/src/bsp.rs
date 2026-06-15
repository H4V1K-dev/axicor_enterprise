use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use crate::error::NetError;

/// Hard limit on the wait timeout (in milliseconds) for peers in the BSP barrier.
pub const BSP_TIMEOUT_MS: u64 = 500;

/// Bulk Synchronous Parallel coordinator barrier.
pub struct BspBarrier {
    /// The current globally confirmed epoch of the simulation cluster.
    pub global_epoch: AtomicU32,
    /// Recovery flag set if a deadlock timeout is encountered.
    pub is_poisoned: AtomicBool,
}

impl BspBarrier {
    /// Create a new BspBarrier with the given initial epoch.
    pub fn new(initial_epoch: u32) -> Self {
        Self {
            global_epoch: AtomicU32::new(initial_epoch),
            is_poisoned: AtomicBool::new(false),
        }
    }

    /// Synchronize the epoch across all registered neighboring peer nodes.
    ///
    /// Currently a mock stub returning next epoch.
    pub fn sync_and_swap(&self, current_epoch: u32) -> Result<u32, NetError> {
        let next_epoch = current_epoch + 1;
        self.global_epoch.store(next_epoch, Ordering::Relaxed);
        Ok(next_epoch)
    }
}
