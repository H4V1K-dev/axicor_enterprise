use std::sync::atomic::{AtomicU32, AtomicBool, AtomicUsize, Ordering};
use crate::error::NetError;

/// Hard limit on the wait timeout (in milliseconds) for peers in the BSP barrier.
pub const BSP_TIMEOUT_MS: u64 = 500;

/// Bulk Synchronous Parallel coordinator barrier.
pub struct BspBarrier {
    /// The current globally confirmed epoch of the simulation cluster.
    pub global_epoch: AtomicU32,
    /// Recovery flag set if a deadlock timeout is encountered.
    pub is_poisoned: AtomicBool,
    /// Count of expected peers to complete before stepping.
    pub expected_peers: usize,
    /// Count of peers that completed their epoch packet transfer.
    pub completed_peers: AtomicUsize,
    /// The waiting strategy profile when spin-waiting.
    pub wait_strategy: transport::WaitStrategy,
}

impl BspBarrier {
    /// Create a new BspBarrier.
    pub fn new(initial_epoch: u32, expected_peers: usize, wait_strategy: transport::WaitStrategy) -> Self {
        Self {
            global_epoch: AtomicU32::new(initial_epoch),
            is_poisoned: AtomicBool::new(false),
            expected_peers,
            completed_peers: AtomicUsize::new(0),
            wait_strategy,
        }
    }

    /// Synchronize the epoch across all registered neighboring peer nodes.
    pub fn sync_and_swap(&self, current_epoch: u32) -> Result<u32, NetError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(BSP_TIMEOUT_MS);

        while self.completed_peers.load(Ordering::Acquire) < self.expected_peers {
            if start.elapsed() > timeout {
                self.is_poisoned.store(true, Ordering::Release);
                return Err(NetError::Timeout { zone_hash: 0 });
            }
            self.wait_strategy.wait();
        }

        let next_epoch = current_epoch + 1;
        self.global_epoch.store(next_epoch, Ordering::Release);
        self.completed_peers.store(0, Ordering::Release);
        Ok(next_epoch)
    }

    /// Increment completed peer counter.
    pub fn increment_completed_peers(&self) {
        self.completed_peers.fetch_add(1, Ordering::Release);
    }

    /// Reset completed peer counter.
    pub fn reset_completed_peers(&self) {
        self.completed_peers.store(0, Ordering::Release);
    }
}

