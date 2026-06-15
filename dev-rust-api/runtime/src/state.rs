/// State of the runtime execution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Initial startup and resource allocation phase.
    Booting,
    /// Simulation loop executing active day batches.
    Running,
    /// Offline parameters optimization and baking updates phase.
    Night,
    /// Recovery and stabilization loop after node restarts.
    Resurrection,
    /// Explicit grace period releasing drivers and handles.
    Shutdown,
}
