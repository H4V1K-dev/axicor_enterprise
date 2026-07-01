//! Crate DTOs and API facades.

use crate::error::TopologyError;
use config::ShardConfig;
use types::{MasterSeed, PackedPosition, PackedTarget};

/// Input parameters for single-shard topology generation.
#[derive(Debug, Clone)]
pub struct SingleShardTopologyInput<'a> {
    /// Reference to the validated shard configuration.
    pub config: &'a ShardConfig,
    /// Root generation seed for deterministic pseudo-random choices.
    pub seed: MasterSeed,
}

/// Placed soma coordinate payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedSoma {
    /// Unique sequential identifier of the soma (within `0..total_somas`).
    pub soma_id: u32,
    /// Identifier mapping to index in `ShardConfig.neuron_types`.
    pub variant_id: u8,
    /// Packed position containing coordinates and cached variant type.
    pub position: PackedPosition,
}

/// Product of the deterministic single-shard topology generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleShardTopology {
    /// Flat array of placed somas in sorted order.
    pub somas: Vec<PlacedSoma>,
}

// ==========================================
// DTO: Local Axon Growth
// ==========================================

/// Input parameters for single-shard local axon growth.
#[derive(Debug, Clone)]
pub struct AxonGrowthInput<'a> {
    /// Reference to the validated shard configuration.
    pub config: &'a ShardConfig,
    /// Reference to the generated topology of somas.
    pub topology: &'a SingleShardTopology,
    /// Root generation seed for deterministic pseudo-random choices.
    pub seed: MasterSeed,
}

/// A single discrete voxel segment along the trajectory of an axon path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxonSegment {
    /// Voxel position in discrete grid.
    pub position: PackedPosition,
    /// Order index of this segment in the path (ranging from 1 upwards).
    pub segment_offset: u8,
}

/// Stop reasons representing completion states of axon growth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxonGrowthStopReason {
    /// Path reached maximum configured segment limits.
    MaxLengthReached,
    /// Path met shard spatial boundaries.
    BoundaryReached,
    /// Path blocked by other neuron somas, origin coordinate, or self-intersection.
    Blocked,
    /// Source soma position was originally out of grid boundaries.
    SourceOutOfBounds,
}

/// Fully grown path representing sequential segments of an axon from a source soma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrownAxonPath {
    /// ID of the source neuron soma.
    pub soma_id: u32,
    /// Array of ordered segments along the path (origin soma voxel not included).
    pub segments: Vec<AxonSegment>,
    /// Reason indicating why the path generation stopped.
    pub stop_reason: AxonGrowthStopReason,
}

/// Complete collection of grown axon paths for a shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGrowthResult {
    /// Flat array of grown axon paths, matching the order of input somas.
    pub axons: Vec<GrownAxonPath>,
}

/// Entrypoint facade for geometry and topology processing.
pub struct TopologyEngine;

impl TopologyEngine {
    /// Deterministically generates the spatial topology (soma placement) of a single shard.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError`] if layer constraints or coordinate limitations are violated.
    pub fn generate_single_shard_topology(
        input: &SingleShardTopologyInput,
    ) -> Result<SingleShardTopology, TopologyError> {
        crate::placement::generate_single_shard_topology(input)
    }

    /// Deterministically grows local axon paths within a single shard.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError`] if soma variant configurations are unknown or invalid.
    pub fn grow_local_axons(input: &AxonGrowthInput) -> Result<LocalGrowthResult, TopologyError> {
        crate::growth::grow_local_axons(input)
    }

    /// Deterministically builds the plan of local synaptic connections.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError`] if parameters or inputs are inconsistent.
    pub fn form_local_synapses(
        input: &SynapseFormationInput,
    ) -> Result<LocalSynapsePlan, TopologyError> {
        crate::synapses::form_local_synapses(input)
    }
}

// ==========================================
// DTO: Local Synapse Formation (Stage B2)
// ==========================================

/// Input parameters for single-shard local synapse formation.
#[derive(Debug, Clone)]
pub struct SynapseFormationInput<'a> {
    /// Reference to the validated shard configuration.
    pub config: &'a ShardConfig,
    /// Reference to the generated topology of somas.
    pub topology: &'a SingleShardTopology,
    /// Reference to the results of axon growth.
    pub growth: &'a LocalGrowthResult,
    /// Voxel size of the grid in micrometers.
    pub voxel_size_um: f32,
    /// Root generation seed for deterministic pseudo-random choices.
    pub seed: MasterSeed,
}

/// Connectome plan of local synaptic contacts within a single shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSynapsePlan {
    /// Ordered rows of synaptic connections per target soma.
    pub rows: Vec<NeuronSynapseRow>,
    /// Total count of successfully established live synapse contacts in the shard.
    pub total_live_synapses: usize,
    /// Total count of connection candidates dropped due to MAX_DENDRITES limit.
    pub dropped_candidates: usize,
}

/// Row representing all active incoming synapses of a target neuron soma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeuronSynapseRow {
    /// ID of the target soma.
    pub target_soma_id: u32,
    /// Established active synapse connections (capped to 128 slots).
    pub slots: Vec<FormedSynapse>,
}

/// Parameters of an established active synaptic connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedSynapse {
    /// Index of the dendrite slot (ranging from 0 to 127).
    pub dendrite_slot: u8,
    /// Packed target identifying the source axon and segment offset.
    pub target: PackedTarget,
    /// Initial synaptic weight mass value (preserving excitability/inhibitory sign).
    pub weight: i32,
    /// Synaptic timer value (always initialized to 0).
    pub timer: u8,
    /// ID of the source soma neuron.
    pub source_soma_id: u32,
    /// Axon identifier (identical to source_soma_id in local domain).
    pub axon_id: u32,
    /// Offset of the axon segment.
    pub segment_offset: u8,
}
