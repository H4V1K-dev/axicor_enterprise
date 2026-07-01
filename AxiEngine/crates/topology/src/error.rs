//! Crate error types.

/// Validation and processing errors for the single-shard topology generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// Placed soma coordinate exceeds allowed voxel grid boundaries.
    VoxelBoundsOverflow {
        /// X coordinate provided.
        x: u32,
        /// Y coordinate provided.
        y: u32,
        /// Z coordinate provided.
        z: u32,
    },
    /// Exceeded maximum capacity for a layer during soma placement.
    LayerCapacityExceeded {
        /// Index of the layer in configuration.
        layer_index: usize,
        /// Maximum computed capacity of the layer in voxels.
        max_capacity: usize,
    },
    /// Geometrical layer bounds or alignment error.
    LayerGeometryError {
        /// Index of the layer in configuration.
        layer_index: usize,
        /// Description of the geometrical inconsistency.
        msg: String,
    },
    /// Soma composition count does not match expected target count in a layer.
    CompositionMismatch {
        /// Index of the layer in configuration.
        layer_index: usize,
        /// Expected total soma count for the layer.
        expected: usize,
        /// Actual total count of distributed somas.
        actual: usize,
    },
    /// Arithmetic overflow during checked calculations of capacity or count.
    CapacityOverflow,
    /// Reference to an undefined neuron type variant.
    UnknownNeuronType {
        /// The variant identifier that was not found in config.
        variant_id: u8,
    },
    /// Growth parameter value is non-finite or out of safe integer boundaries.
    InvalidGrowthParameter {
        /// The variant identifier of the neuron type.
        variant_id: u8,
        /// The name of the parameter field.
        field: &'static str,
    },
}
