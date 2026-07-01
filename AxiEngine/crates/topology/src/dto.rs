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
}
