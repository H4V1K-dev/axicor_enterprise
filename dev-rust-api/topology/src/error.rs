use std::fmt;

/// Custom error types representing spatial and topological logic constraints.
#[derive(Debug, Clone, PartialEq)]
pub enum TopologyError {
    /// Exceeded reject-sampling attempt limit when placing somas.
    PlacementCollision { density: f32, layer: String },
    /// Dendrite slot limit (128) exceeded.
    DendriteSlotOverflow { soma_id: usize },
    /// Attempted to grow axon longer than 256 segments.
    AxonLengthOverflow { axon_id: usize },
    /// Provided an empty layer or zero layer height.
    EmptyZone { zone_name: String },
    /// Integrity violation in the voxel grid structure.
    InvalidVoxelGrid,
    /// Exceeded pre-allocated VRAM ghost_capacity limit.
    GhostCapacityExceeded { current: u32, limit: u32 },
}

impl std::error::Error for TopologyError {}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlacementCollision { density, layer } => {
                write!(f, "Placement collision in layer '{}' with density {}", layer, density)
            }
            Self::DendriteSlotOverflow { soma_id } => {
                write!(f, "Dendrite slot overflow for soma_id {}", soma_id)
            }
            Self::AxonLengthOverflow { axon_id } => {
                write!(f, "Axon length overflow for axon_id {}", axon_id)
            }
            Self::EmptyZone { zone_name } => {
                write!(f, "Empty zone or zero height: {}", zone_name)
            }
            Self::InvalidVoxelGrid => {
                write!(f, "Invalid voxel grid integrity")
            }
            Self::GhostCapacityExceeded { current, limit } => {
                write!(f, "Ghost capacity exceeded: current {} / limit {}", current, limit)
            }
        }
    }
}
