//! Topology and space-soma placement algorithms crate.

pub mod dto;
pub mod error;
mod placement;

pub use dto::{PlacedSoma, SingleShardTopology, SingleShardTopologyInput, TopologyEngine};
pub use error::TopologyError;
