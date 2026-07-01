//! Crate DTOs and API facades.

use crate::error::TopologyError;
use config::ShardConfig;
use types::{MasterSeed, PackedPosition};

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
}
