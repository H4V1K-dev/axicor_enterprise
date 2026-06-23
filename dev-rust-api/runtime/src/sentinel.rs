use crate::error::RuntimeError;

pub const WARMUP_TICKS_LIMIT: u32 = 100;

/// Monitoring of node and cluster health.
#[derive(Debug, Clone, Copy)]
pub struct Sentinel {
    /// Count of ticks spent in warmup (warmup loop).
    pub warmup_ticks: u32,
    /// Number of consecutive BSP barrier timeouts from neighbors.
    pub consecutive_timeouts: u32,
    /// Last mathematically confirmed epoch before failure.
    pub last_valid_epoch: u32,
}

impl Sentinel {
    pub fn new() -> Self {
        Self {
            warmup_ticks: 0,
            consecutive_timeouts: 0,
            last_valid_epoch: 0,
        }
    }

    pub fn start_warmup(&mut self) {
        self.warmup_ticks = 0;
    }

    pub fn verify_stability(&mut self, result: &compute_api::BatchResult) -> Result<(), RuntimeError> {
        // E-138: If membrane potentials do not stabilize within limits, return UnstableWarmup
        if self.warmup_ticks >= WARMUP_TICKS_LIMIT {
            return Err(RuntimeError::UnstableWarmup);
        }
        self.warmup_ticks += result.ticks_processed;
        Ok(())
    }

    pub fn end_warmup(&mut self) {
        self.warmup_ticks = 0;
    }
}

impl Default for Sentinel {
    fn default() -> Self {
        Self::new()
    }
}
