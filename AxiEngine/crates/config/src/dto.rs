//! Declarative biological configuration Data Transfer Objects (DTOs).

use serde::{Deserialize, Serialize};

/// Direction of connection sockets/ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Inbound connection
    In,
    /// Outbound connection
    Out,
}

/// Behavior when signal pixel has zero weight/activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmptyPixelMode {
    /// Skip empty pixels completely.
    Skip,
    /// Output zeros for empty pixels.
    Zero,
}

/// Vertical alignment of axonal entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EntryZ {
    /// Align to the top of the shard.
    Top,
    /// Align to the middle of the shard.
    Mid,
    /// Align to the bottom of the shard.
    Bottom,
}

/// System metadata block for configuration files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SystemMeta {
    /// Configuration identifier.
    pub id: String,
    /// Version of the configuration schema.
    pub version: String,
    /// Creation timestamp or ISO date.
    pub created_at: String,
}

/// Global model configuration (`model.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    /// Optional metadata about this system.
    pub meta: Option<SystemMeta>,
    /// Dimensions and parameters of the physical world.
    pub world: WorldConfig,
    /// Physical simulation constraints.
    pub simulation: SimulationParams,
    /// List of departments inside this model.
    pub departments: Vec<DepartmentEntry>,
    /// Global inter-departmental connections list.
    pub connections: Vec<ModelConnectionConfig>,
}

/// Dimensions of the simulated 3D world in micrometers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldConfig {
    /// Width in micrometers.
    pub width_um: f64,
    /// Depth in micrometers.
    pub depth_um: f64,
    /// Height in micrometers.
    pub height_um: f64,
}

/// Simulation run and physical constants parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimulationParams {
    /// Duration of a single tick in microseconds.
    pub tick_duration_us: u32,
    /// Total simulation run length (0 for infinite execution).
    pub total_ticks: u64,
    /// Seed string for master PRNG.
    pub master_seed: String,
    /// Grid voxel size in micrometers.
    pub voxel_size_um: f32,
    /// Axial segment length in units of voxels.
    pub segment_length_voxels: u32,
    /// Axonal signal propagation speed in meters per second.
    pub signal_speed_m_s: f32,
    /// Number of simulation ticks in a synchronization batch.
    pub sync_batch_ticks: u32,
    /// Max steps allowed for axonal growth.
    pub axon_growth_max_steps: u32,
}

/// Entry representing a department in the global model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentEntry {
    /// Name of the department.
    pub name: String,
    /// Path or key of the department configuration file.
    pub config: String,
    /// Optional system metadata.
    pub meta: Option<SystemMeta>,
}

/// Specification of a global model connection between departments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelConnectionConfig {
    /// Connection identifier.
    pub id: String,
    /// Source endpoint path (e.g. `DeptA.ShardB.SocketC`).
    pub from: String,
    /// Target endpoint path (e.g. `DeptX.ShardY.SocketZ`).
    pub to: String,
}

/// Department level configuration (`department.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentConfig {
    /// Optional system metadata block.
    pub meta: Option<SystemMeta>,
    /// List of shards inside the department.
    pub shards: Vec<ShardEntry>,
    /// List of connections within the department.
    pub connections: Vec<DepartmentConnection>,
}

/// Entry representing a shard in the department configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardEntry {
    /// Name of the shard.
    pub name: String,
    /// Path or key of the shard configuration file.
    pub config: String,
}

/// Connection within a department between two shards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentConnection {
    /// Connection identifier.
    pub id: String,
    /// Source endpoint path (e.g. `ShardA.SocketX`).
    pub from: String,
    /// Target endpoint path (e.g. `ShardB.SocketY`).
    pub to: String,
}

/// Complete configuration file for a single simulation shard (`shard.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardConfig {
    /// Optional system metadata block.
    pub meta: Option<SystemMeta>,
    /// Grid dimensions of the shard.
    pub dimensions: ShardDimensions,
    /// Biological and VRAM settings.
    pub settings: ShardSettings,
    /// Physical anatomy layers.
    pub layers: Vec<LayerConfig>,
    /// Array of up to 16 biological neuron type profiles.
    pub neuron_types: Vec<NeuronType>,
    /// Optional external input/output connection sockets.
    pub sockets: Option<Vec<SocketConfig>>,
    /// Optional external input/output parallel ports.
    pub ports: Option<Vec<PortConfig>>,
}

/// Voxel dimensions of a simulation shard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardDimensions {
    /// Width in voxels.
    pub w: u32,
    /// Depth in voxels.
    pub d: u32,
    /// Height in voxels.
    pub h: u32,
}

/// Internal thresholds, night cycle rates and checkpoint interval settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardSettings {
    /// External inbound connections queue VRAM size.
    pub ghost_capacity: u32,
    /// Synapse pruning structural threshold.
    pub prune_threshold: i32,
    /// Maximum sprouts a neuron can grow in one growth step.
    pub max_sprouts: u32,
    /// Interval in ticks for night cycle processing.
    pub night_interval_ticks: u32,
    /// Checkpoint state serialization interval in ticks.
    pub save_checkpoints_interval_ticks: u32,
}

/// Anatomical layer boundary layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
    /// Unique name of the layer.
    pub name: String,
    /// Percentage height (relative to shard height) in `0.0..=1.0`.
    pub height_pct: f32,
    /// Voxel filling ratio with somas.
    pub density: f32,
    /// Distributed shares of different neuron type populations.
    pub composition: Vec<NeuronTypeDistribution>,
}

/// Share of a specific neuron type within a layer's composition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuronTypeDistribution {
    /// Name of the neuron type.
    pub type_name: String,
    /// Share ratio in `0.0..=1.0`.
    pub share: f32,
}

/// Integrated biological profile for a single neuron type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuronType {
    /// Unique name of the neuron type.
    pub name: String,
    /// Membrane electrical properties.
    pub membrane: MembraneParams,
    /// Temporal firing refractory settings.
    pub timing: TimingParams,
    /// Sentinels and signal range properties.
    pub signal: SignalParams,
    /// Soma homeostatic voltage adjustment.
    pub homeostasis: HomeostasisParams,
    /// Leaky threshold decay shift settings.
    pub adaptive_leak: AdaptiveLeakParams,
    /// Synaptic receptors affinity parameters.
    pub dopamine: DopamineParams,
    /// Synaptic plasticity constants and learning curves.
    pub gsop: GsopParams,
    /// Axonal and dendritic growth constraints.
    pub growth: GrowthParams,
    /// Spontaneous digital phase excitation.
    pub spontaneous: SpontaneousParams,
}

/// GLIF model membrane electric potentials and threshold shifts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MembraneParams {
    /// Firing threshold voltage.
    pub threshold: i32,
    /// Baseline rest potential voltage.
    pub rest_potential: i32,
    /// Bit-shift divisor parameter for leaks.
    pub leak_shift: u32,
    /// Amplitude of after-hyperpolarization.
    pub ahp_amplitude: u16,
}

/// Action potential temporal refractory periods.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TimingParams {
    /// Duration of neuron firing refractory state in ticks.
    pub refractory_period: u8,
    /// Maximum fatigue capacity for synaptic gradient fatigue in ticks.
    pub fatigue_capacity: u8,
}

/// Signal propagation range constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SignalParams {
    /// Maximum length of signal propagation in synapses/units.
    pub signal_propagation_length: u8,
}

/// Intracellular homeostasis voltage penalty parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HomeostasisParams {
    /// Penalty value subtracted on firing activity.
    pub homeostasis_penalty: i32,
    /// Exponential decay coefficient.
    pub homeostasis_decay: u16,
}

/// Adaptive threshold leak parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveLeakParams {
    /// Minimum threshold adaptation offset.
    pub adaptive_leak_min_shift: i32,
    /// Adaptation gain multiplier.
    pub adaptive_leak_gain: u16,
    /// Selection of leak adjustment formula (0, 1 or 2).
    pub adaptive_mode: u8,
}

/// Dopaminergic receptor affinity parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DopamineParams {
    /// D1 receptor family affinity.
    pub d1_affinity: u8,
    /// D2 receptor family affinity.
    pub d2_affinity: u8,
}

/// GSOP learning rule, inhibitory traits and synaptic weight tables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GsopParams {
    /// Weight potentiation step.
    pub gsop_potentiation: u16,
    /// Weight depression step.
    pub gsop_depression: u16,
    /// Initial weight for newly generated synapses.
    pub initial_synapse_weight: u16,
    /// Specifies if the synapses are inhibitory.
    pub is_inhibitory: bool,
    /// Array of exactly 8 points defining synaptic inertia curve.
    pub inertia_curve: Vec<u8>,
}

/// Geometrical and affinity parameters for axonal/dendritic growth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GrowthParams {
    /// Field of view in degrees for pathfinding.
    pub steering_fov_deg: f32,
    /// Sensing search radius in micrometers.
    pub steering_radius_um: f32,
    /// Inertial straight-line steering weight.
    pub steering_weight_inertia: f32,
    /// Target sensor field attraction weight.
    pub steering_weight_sensor: f32,
    /// Noise/jitter steering variance.
    pub steering_weight_jitter: f32,
    /// Dendritic connectivity target sphere radius in micrometers.
    pub dendrite_radius_um: f32,
    /// Vertical orientation bias factor.
    pub growth_vertical_bias: f32,
    /// Biological connection affinity factor.
    pub type_affinity: f32,
    /// Whitelist of neuron type names allowed for connection.
    pub dendrite_whitelist: Vec<String>,
    /// Sprouting weight distance factor.
    pub sprouting_weight_distance: f32,
    /// Sprouting weight power exponent.
    pub sprouting_weight_power: f32,
    /// Sprouting weight exploration factor.
    pub sprouting_weight_explore: f32,
    /// Sprouting weight biological type match factor.
    pub sprouting_weight_type: f32,
}

/// Spontaneous activity triggering parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpontaneousParams {
    /// Period of spontaneous firing in ticks (0 to disable, >= 2 to enable).
    pub spontaneous_firing_period_ticks: u32,
}

/// Single external connection socket configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SocketConfig {
    /// Socket unique name inside shard.
    pub name: String,
    /// Connection direction.
    pub direction: Direction,
    /// Grid width in neurons/pixels.
    pub width: u32,
    /// Grid height in neurons/pixels.
    pub height: u32,
    /// Optional axonal entry vertical alignment.
    pub entry_z: Option<EntryZ>,
    /// Target neuron type name filter.
    pub target_type: Option<String>,
    /// Optional limit of growth steps.
    pub growth_steps: Option<u32>,
}

/// Parallel connection port mapping config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PortConfig {
    /// Unique name of the port.
    pub name: String,
    /// Connection direction.
    pub direction: Direction,
    /// Optional axonal entry vertical alignment.
    pub entry_z: Option<EntryZ>,
    /// Structured list of sub-pin mappings.
    pub pins: Vec<PinConfig>,
}

/// Single pixel pin configuration within a port mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PinConfig {
    /// Unique pin identifier.
    pub name: String,
    /// Pin grid width.
    pub width: u32,
    /// Pin grid height.
    pub height: u32,
    /// Local normalized U coordinate.
    pub local_u: f32,
    /// Local normalized V coordinate.
    pub local_v: f32,
    /// U projection span width.
    pub u_width: f32,
    /// V projection span height.
    pub v_height: f32,
    /// Target neuron type filter.
    pub target_type: String,
    /// Target grid stride.
    pub stride: u32,
    /// Optional maximum axonal growth steps.
    pub growth_steps: Option<u32>,
    /// Optional mode for empty pixel outputs.
    pub empty_pixel: Option<EmptyPixelMode>,
}
