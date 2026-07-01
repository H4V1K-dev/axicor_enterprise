use config::ShardConfig;
use layout::{VariantParameters, VARIANT_LUT_LEN};
use types::MasterSeed;

/// Input details for baking a local single shard.
pub struct LocalShardBakeInput<'a> {
    /// Reference to the parsed shard configuration.
    pub shard_config: &'a ShardConfig,
    /// Seed for random generators (somas placement, steering, etc.).
    pub master_seed: MasterSeed,
    /// Voxel size in micrometers.
    pub voxel_size_um: f32,
}

/// Compiled binary artifacts of the shard.
pub struct LocalShardArtifacts {
    /// Binary representation of the shard's neuron/dendrite state (SoA).
    pub state_blob: Vec<u8>,
    /// Binary representation of the active axons list (pulsing heads).
    pub axons_blob: Vec<u8>,
    /// Binary representation of the grown axon paths and lengths.
    pub paths_blob: Vec<u8>,
    /// The fixed-size neuron variants parameters lookup table (LUT).
    pub variant_table: [VariantParameters; VARIANT_LUT_LEN],
}

/// Statistics report of the baked shard.
pub struct LocalShardBakeReport {
    /// Total number of somas placed in the shard.
    pub total_somas: u32,
    /// Total number of axons grown (corresponds to total_somas for a local shard).
    pub total_axons: u32,
    /// Total number of synapses successfully formed.
    pub total_synapses: u32,
    /// Total number of candidate connections dropped due to limits (like MAX_DENDRITES).
    pub dropped_candidates: u64,
}

/// The archive path for the `.state` file.
pub const STATE_ARCHIVE_PATH: &str = "state.bin";
/// The archive path for the `.axons` file.
pub const AXONS_ARCHIVE_PATH: &str = "axons.bin";
/// The archive path for the `.paths` file.
pub const PATHS_ARCHIVE_PATH: &str = "paths.bin";
/// The archive path for the `VariantParameters` lookup table.
pub const VARIANT_TABLE_ARCHIVE_PATH: &str = "variant_table.bin";
