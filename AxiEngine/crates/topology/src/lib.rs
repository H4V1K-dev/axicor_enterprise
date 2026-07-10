//! Topology and space-soma placement algorithms crate.

pub mod dto;
pub mod error;
mod growth;
pub mod night_planning;
mod placement;
mod synapses;

pub use dto::{
    AxonGrowthInput, AxonGrowthStopReason, AxonSegment, FormedSynapse, GrownAxonPath,
    LocalGrowthResult, LocalSynapsePlan, NeuronSynapseRow, PlacedSoma, SingleShardTopology,
    SingleShardTopologyInput, SynapseFormationInput, TopologyEngine,
};
pub use error::TopologyError;
pub use night_planning::{
    build_compaction_plan, choose_dendrite_slot, cmp_rank, compute_power_fixed,
    compute_sprout_score, generate_jitter_unit, plan_pruning, CompactionPlan, GhostHandoverDraft,
    SproutRankKey, SproutWeightParams,
};
