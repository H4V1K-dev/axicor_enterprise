//! Runtime Orchestration Crate for AxiEngine.
//!
//! Provides the execution orchestration interfaces for single-shard local day loops.

pub mod dto;
pub mod error;
pub mod local;

pub use dto::{
    LocalRuntimeConfig, RuntimeBatchInput, RuntimeBatchReport, RuntimeState, RuntimeStats,
};
pub use error::RuntimeError;
pub use local::LocalRuntime;
