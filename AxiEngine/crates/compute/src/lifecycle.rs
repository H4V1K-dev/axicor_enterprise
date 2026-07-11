//! Lifecycle states definition.

/// Represents the execution lifecycle states of a ShardEngine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Context initialized, backend chosen, memory not allocated.
    Created,
    /// Memory successfully allocated, handle generated.
    Allocated,
    /// Shard data uploaded, ready for simulation execution.
    Running,
    /// Context in maintenance mode for connectome synaptogenesis updates.
    Maintenance,
    /// Context torn down, resources cleared.
    TornDown,
}
