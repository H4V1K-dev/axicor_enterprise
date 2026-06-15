/// Local in-memory representation of a growing axon on the CPU.
/// Active only during the Night Phase.
#[derive(Debug, Clone, PartialEq)]
pub struct LivingAxon {
    pub axon_id: usize,
    pub soma_idx: usize,
    pub tip_uvw: u32,
    pub forward_dir: glam::Vec3,
    pub remaining_steps: u32,
    pub last_night_active: bool,
}

/// Abstract geometric description of an axon crossing the shard boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostPacket {
    pub origin_shard_id: u32,
    pub soma_idx: usize,
    pub type_idx: usize,
    pub entry_x: u32,
    pub entry_y: u32,
    pub entry_z: u32,
    pub entry_dir: glam::Vec3,
    pub remaining_steps: u32,
}

/// Finite state machine representing the outcome of a single axon growth step.
#[derive(Debug, Clone, PartialEq)]
pub enum GrowthEvent {
    Advanced(u32),
    TargetReached,
    Stagnated,
    OutOfBounds(GhostPacket),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxonSegment {
    pub axon_id: u32,
    pub type_idx: usize,
    pub pos: u32,
}

#[derive(Debug, Clone)]
pub struct SpatialGrid {
    pub segments: Vec<AxonSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewSynapse {
    pub soma_idx: usize,
    pub slot_idx: usize,
    pub target_packed: u32,
    pub weight: i32,
}
