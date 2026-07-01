//! Topology and space-soma placement algorithms crate.

pub mod dto;
pub mod error;
mod growth;
mod placement;
mod synapses;

pub use dto::{
    AxonGrowthInput, AxonGrowthStopReason, AxonSegment, FormedSynapse, GrownAxonPath,
    LocalGrowthResult, LocalSynapsePlan, NeuronSynapseRow, PlacedSoma, SingleShardTopology,
    SingleShardTopologyInput, SynapseFormationInput, TopologyEngine,
};
pub use error::TopologyError;
