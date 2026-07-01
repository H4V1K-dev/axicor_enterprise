//! Topology and space-soma placement algorithms crate.

pub mod dto;
pub mod error;
mod growth;
mod placement;

pub use dto::{
    AxonGrowthInput, AxonGrowthStopReason, AxonSegment, GrownAxonPath, LocalGrowthResult,
    PlacedSoma, SingleShardTopology, SingleShardTopologyInput, TopologyEngine,
};
pub use error::TopologyError;
