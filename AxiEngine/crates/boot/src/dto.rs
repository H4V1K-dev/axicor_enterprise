//! Input and output configuration models for single-shard archive boot stages.

use std::path::PathBuf;

/// Input parameters for loading single-shard `.axic` package.
pub struct LocalShardBootInput {
    /// The OS path to the `.axic` file.
    pub archive_path: PathBuf,
    /// Global offset index assigned to this shard's neuron space.
    pub virtual_offset: u32,
    /// The count of ghost neuron parameters allocated.
    pub total_ghosts: u32,
}

/// Input parameters for loading and directly bootstrapping compute execution engine.
pub struct LocalShardComputeInput {
    /// The OS path to the `.axic` file.
    pub archive_path: PathBuf,
    /// Preferred hardware compute platform.
    pub backend_preference: compute::BackendPreference,
    /// Global offset index assigned to this shard's neuron space.
    pub virtual_offset: u32,
    /// The count of ghost neuron parameters allocated.
    pub total_ghosts: u32,
}

/// An owned memory bundle representing loaded and validated single-shard contents.
pub struct LocalShardBootBundle {
    /// Memory allocation details for Compute HAL.
    pub spec: compute_api::ShardAllocSpec,
    /// Owned state array bytes.
    pub state_blob: Vec<u8>,
    /// Owned pulse status headers.
    pub axons_blob: Vec<u8>,
    /// Owned layout path coordinates list.
    pub paths_blob: Vec<u8>,
    /// Safe aligned table of variant parameter presets.
    pub variant_table: [layout::VariantParameters; layout::VARIANT_LUT_LEN],
}

impl LocalShardBootBundle {
    /// Borrow this bundle as temporary slices for uploading into the compute pipeline.
    pub fn upload(&self) -> compute_api::ShardUpload<'_> {
        compute_api::ShardUpload {
            state_blob: &self.state_blob,
            axons_blob: &self.axons_blob,
            variant_table: &self.variant_table,
        }
    }

    pub fn into_working_state(self) -> runtime::HostWorkingState {
        let padded_n = self.spec.padded_n;
        let total_axons = self.spec.total_axons;
        let total_ghosts = self.spec.total_ghosts;

        runtime::HostWorkingState {
            state_blob: self.state_blob,
            axons_blob: self.axons_blob,
            paths_blob: self.paths_blob,
            padded_n,
            total_axons,
            total_ghosts,
        }
    }
}

/// Extension trait for HostWorkingState to bootstrap it from a LocalShardBootBundle.
pub trait HostWorkingStateBootExt {
    /// Constructs a durable HostWorkingState from a loaded boot bundle.
    fn from_boot_bundle(bundle: LocalShardBootBundle) -> Self;
}

impl HostWorkingStateBootExt for runtime::HostWorkingState {
    fn from_boot_bundle(bundle: LocalShardBootBundle) -> Self {
        bundle.into_working_state()
    }
}

/// Extension trait for LocalRuntime to construct it from a loaded boot bundle.
pub trait LocalRuntimeBootExt {
    /// Constructs a LocalRuntime from a loaded boot bundle, including durable working state.
    fn from_boot_bundle(
        engine: compute::ShardEngine,
        config: runtime::LocalRuntimeConfig,
        bundle: LocalShardBootBundle,
    ) -> Result<Self, runtime::RuntimeError>
    where
        Self: Sized;
}

impl LocalRuntimeBootExt for runtime::LocalRuntime {
    fn from_boot_bundle(
        engine: compute::ShardEngine,
        config: runtime::LocalRuntimeConfig,
        bundle: LocalShardBootBundle,
    ) -> Result<Self, runtime::RuntimeError> {
        let working = bundle.into_working_state();
        Self::new_with_state(engine, config, working)
    }
}
