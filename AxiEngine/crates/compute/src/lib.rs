//! Compute execution facade for AxiEngine.
//!
//! Exposes ShardEngine to coordinate allocation, execution, and lifetime
//! of compute backend shards (CPU, CUDA, HIP, Mock).

pub mod engine;
pub mod error;
pub mod lifecycle;
pub mod preference;

#[cfg(feature = "mock")]
pub mod mock;

pub use engine::ShardEngine;
pub use error::ComputeError;
pub use lifecycle::LifecycleState;
pub use preference::BackendPreference;
