//! Ahead-of-Time (AOT) Compiler Orchestrator for AxiEngine local shards.
//!
//! This crate coordinates the placement of somas, growth of local axons, and formation of synapses
//! into binary C-ABI layouts that can be directly mapped or uploaded to simulation engines.

pub mod dto;
pub mod error;
pub mod pipeline;

pub use dto::{LocalShardArtifacts, LocalShardBakeInput, LocalShardBakeReport};
pub use error::BakerError;
pub use pipeline::bake_local_shard;
