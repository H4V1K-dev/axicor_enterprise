use crate::error::ConfigError;
use crate::simulation::SystemMeta;
use serde::Deserialize;
use std::collections::HashSet;

/// Root configuration describing an individual simulation unit (shard.toml).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShardConfig {
    pub meta: Option<SystemMeta>,
    pub dimensions: ShardDimensions,
    pub layers: Vec<LayerConfig>,
    pub neuron_types: Vec<NeuronType>,
    pub sockets: Option<Vec<SocketConfig>>,
    pub ports: Option<Vec<PortConfig>>,
    pub settings: ShardSettings,
}

/// Shard dimensions in voxels.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShardDimensions {
    pub w: u32,
    pub d: u32,
    pub h: u32,
}

/// Parameters configuring a single cortical layer.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
    pub name: String,
    pub height_pct: f32,
    pub density: f32,
    pub composition: Vec<NeuronTypeDistribution>,
}

/// Composition representation of neuron types in a cortical layer.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuronTypeDistribution {
    pub type_name: String,
    pub share: f32,
}

/// Full physical parameters profile representing a neuron type.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuronType {
    pub name: String,
    pub membrane: MembraneParams,
    pub timings: TimingParams,
    pub signal: SignalParams,
    pub homeostasis: HomeostasisParams,
    pub adaptive_leak: AdaptiveLeakParams,
    pub dopamine: DopamineParams,
    pub gsop: GsopParams,
    pub growth: GrowthParams,
    pub spontaneous: SpontaneousParams,
}

/// Membrane voltage parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MembraneParams {
    pub threshold: i32,
    pub rest_potential: i32,
    pub leak_shift: u32,
    pub ahp_amplitude: u16,
}

/// Timing parameters (simulation ticks).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimingParams {
    pub refractory_period: u8,
    pub synapse_refractory_period: u8,
}

/// Signal propagation parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignalParams {
    pub signal_propagation_length: u8,
}

/// Threshold offset dynamics (homeostasis).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HomeostasisParams {
    pub homeostasis_penalty: i32,
    pub homeostasis_decay: u16,
}

/// Active adaptive leak parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveLeakParams {
    pub adaptive_leak_min_shift: i32,
    pub adaptive_leak_gain: u16,
    pub adaptive_mode: u8,
}

/// Dopamine STDP affinity parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DopamineParams {
    pub d1_affinity: u8,
    pub d2_affinity: u8,
}

/// Synaptic plasticity and type parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GsopParams {
    pub gsop_potentiation: u16,
    pub gsop_depression: u16,
    pub is_inhibitory: bool,
    pub inertia_curve: Vec<u8>,
}

/// Axon and dendrite tree growth parameters.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GrowthParams {
    pub steering_fov_deg: f32,
    pub steering_radius_um: f32,
    pub steering_weight_inertia: f32,
    pub steering_weight_sensor: f32,
    pub steering_weight_jitter: f32,
    pub dendrite_radius_um: f32,
    pub growth_vertical_bias: f32,
    pub type_affinity: f32,
    pub dendrite_whitelist: Vec<String>,
    pub sprouting_weight_distance: f32,
    pub sprouting_weight_power: f32,
    pub sprouting_weight_explore: f32,
    pub sprouting_weight_type: f32,
}

/// Background noise parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpontaneousParams {
    pub spontaneous_firing_period_ticks: u32,
}

/// Interface direction (In/Out).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum SocketDirection {
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
}

/// Axon entry direction along the Z axis.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum EntryZ {
    #[serde(rename = "Top")]
    Top,
    #[serde(rename = "Mid")]
    Mid,
    #[serde(rename = "Bottom")]
    Bottom,
}

/// Local socket configuration for inter-shard connections.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SocketConfig {
    pub name: String,
    pub direction: SocketDirection,
    pub width: u32,
    pub height: u32,
    pub entry_z: Option<EntryZ>,
    pub target_type: Option<String>,
    pub growth_steps: Option<u32>,
}

/// External IO interface port configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PortConfig {
    pub name: String,
    pub direction: SocketDirection,
    pub entry_z: Option<EntryZ>,
    pub pins: Vec<PinConfig>,
}

/// IO Pin projection mapping sensor/motor channels.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PinConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub local_u: f32,
    pub local_v: f32,
    pub u_width: f32,
    pub v_height: f32,
    pub target_type: String,
    pub stride: u32,
    pub growth_steps: Option<u32>,
    pub empty_pixel: Option<String>,
}

/// Local shard settings and plasticity boundaries.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShardSettings {
    pub ghost_capacity: u32,
    pub prune_threshold: i32,
    pub max_sprouts: u32,
    pub night_interval_ticks: u32,
    pub save_checkpoints_interval_ticks: u32,
}

pub fn parse_shard_config(content: &str) -> Result<ShardConfig, ConfigError> {
    let config: ShardConfig = toml::from_str(content)?;
    Ok(config)
}

pub fn validate_shard(config: &ShardConfig) -> Result<(), ConfigError> {
    // 1. Validate dimensions (w <= 1023, d <= 1023, h <= 255)
    let d = &config.dimensions;
    if d.w == 0 || d.w > 1023 || d.d == 0 || d.d > 1023 || d.h == 0 || d.h > 255 {
        return Err(ConfigError::ValidationError(format!(
            "Invalid shard dimensions (max: 1023x1023x255): w={}, d={}, h={}",
            d.w, d.d, d.h
        )));
    }

    // 2. Build type set of defined neuron types
    let mut neuron_types = HashSet::new();
    if config.neuron_types.len() > 16 {
        return Err(ConfigError::ValidationError(format!(
            "INV-CONFIG-001: Maximum number of neuron types is 16, got {}",
            config.neuron_types.len()
        )));
    }

    for nt in &config.neuron_types {
        if nt.name.is_empty() {
            return Err(ConfigError::ValidationError(
                "Neuron type name must not be empty".to_string(),
            ));
        }
        if !neuron_types.insert(&nt.name) {
            return Err(ConfigError::ValidationError(format!(
                "Duplicate neuron type definition '{}'",
                nt.name
            )));
        }

        if nt.timings.refractory_period == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' refractory_period must be > 0",
                nt.name
            )));
        }
        if nt.signal.signal_propagation_length == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' signal_propagation_length must be > 0",
                nt.name
            )));
        }

        // INV-CONFIG-004: signal_propagation_length >= refractory_period
        if nt.signal.signal_propagation_length < nt.timings.refractory_period {
            return Err(ConfigError::ValidationError(format!(
                "INV-CONFIG-004: Neuron type '{}' signal_propagation_length ({}) must be >= refractory_period ({})",
                nt.name, nt.signal.signal_propagation_length, nt.timings.refractory_period
            )));
        }

        if nt.gsop.inertia_curve.len() != 8 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' gsop.inertia_curve must have exactly 8 elements, got {}",
                nt.name, nt.gsop.inertia_curve.len()
            )));
        }
    }

    // 3. Validate anatomy layers
    let mut total_height = 0.0;
    for layer in &config.layers {
        if layer.height_pct <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "Layer height_pct must be > 0, got {}",
                layer.height_pct
            )));
        }
        total_height += layer.height_pct;

        // INV-CONFIG-002: density must be strictly non-negative
        if layer.density < 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "INV-CONFIG-002: Layer density must be >= 0.0, got {}",
                layer.density
            )));
        }

        let mut total_share = 0.0;
        for comp in &layer.composition {
            if comp.share < 0.0 {
                return Err(ConfigError::ValidationError(format!(
                    "Composition share must be >= 0, got {}",
                    comp.share
                )));
            }
            total_share += comp.share;

            // Verify type_name references an actual neuron type
            if !neuron_types.contains(&comp.type_name) {
                return Err(ConfigError::ValidationError(format!(
                    "Layer composition references undefined neuron type '{}'",
                    comp.type_name
                )));
            }
        }

        if (total_share - 1.0).abs() > 1e-4 {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' composition share sum must be 1.0 (got {})",
                layer.name, total_share
            )));
        }
    }

    if (total_height - 1.0).abs() > 1e-4 {
        return Err(ConfigError::ValidationError(format!(
            "Total height_pct of all layers must be 1.0 (got {})",
            total_height
        )));
    }

    // 4. Validate sockets and settings ghost capacity
    let mut has_incoming_sockets = false;
    if let Some(ref sockets) = config.sockets {
        for sock in sockets {
            if sock.name.is_empty() {
                return Err(ConfigError::ValidationError(
                    "Socket name must not be empty".to_string(),
                ));
            }
            if sock.direction == SocketDirection::In {
                has_incoming_sockets = true;
            }
        }
    }

    if has_incoming_sockets && config.settings.ghost_capacity == 0 {
        return Err(ConfigError::ValidationError(
            "Shard has incoming sockets but ghost_capacity is configured to zero".to_string(),
        ));
    }

    // 5. Validate ports and pins boundaries
    if let Some(ref ports) = config.ports {
        for port in ports {
            if port.name.is_empty() {
                return Err(ConfigError::ValidationError(
                    "Port name must not be empty".to_string(),
                ));
            }
            for pin in &port.pins {
                if pin.name.is_empty() {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin inside port '{}' must have a name",
                        port.name
                    )));
                }
                if pin.local_u < 0.0 || pin.local_u > 1.0 || pin.local_v < 0.0 || pin.local_v > 1.0 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' coordinates must be in 0.0..1.0: local_u={}, local_v={}",
                        pin.name, pin.local_u, pin.local_v
                    )));
                }
                if pin.local_u + pin.u_width > 1.0 + 1e-5 || pin.local_v + pin.v_height > 1.0 + 1e-5 {
                    return Err(ConfigError::ValidationError(format!(
                        "Pin '{}' bounds exceed maximum limit of 1.0: local_u+u_width={}, local_v+v_height={}",
                        pin.name, pin.local_u + pin.u_width, pin.local_v + pin.v_height
                    )));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SHARD: &str = r#"
        [dimensions]
        w = 256
        d = 256
        h = 63

        [[layers]]
        name = "L4_Sensory"
        height_pct = 0.6
        density = 0.8
        composition = [
            { type_name = "Stellate_Exc", share = 1.0 }
        ]

        [[layers]]
        name = "L5_Output"
        height_pct = 0.4
        density = 0.5
        composition = [
            { type_name = "Stellate_Exc", share = 1.0 }
        ]

        [[neuron_types]]
        name = "Stellate_Exc"
        
          [neuron_types.membrane]
          threshold = 20000
          rest_potential = -70000
          leak_shift = 4
          ahp_amplitude = 0
          
          [neuron_types.timings]
          refractory_period = 5
          synapse_refractory_period = 10
          
          [neuron_types.signal]
          signal_propagation_length = 8
          
          [neuron_types.homeostasis]
          homeostasis_penalty = 1500
          homeostasis_decay = 990
          
          [neuron_types.adaptive_leak]
          adaptive_leak_min_shift = -5
          adaptive_leak_gain = 2
          adaptive_mode = 1
          
          [neuron_types.dopamine]
          d1_affinity = 80
          d2_affinity = 20
          
          [neuron_types.gsop]
          gsop_potentiation = 15
          gsop_depression = 5
          is_inhibitory = false
          inertia_curve = [10, 20, 30, 40, 50, 60, 70, 80]

          [neuron_types.growth]
          steering_fov_deg = 60.0
          steering_radius_um = 100.0
          steering_weight_inertia = 0.6
          steering_weight_sensor = 0.3
          steering_weight_jitter = 0.1
          dendrite_radius_um = 150.0
          growth_vertical_bias = 0.7
          type_affinity = 0.5
          dendrite_whitelist = []
          sprouting_weight_distance = 0.4
          sprouting_weight_power = 0.4
          sprouting_weight_explore = 0.1
          sprouting_weight_type = 0.1

          [neuron_types.spontaneous]
          spontaneous_firing_period_ticks = 10000

        [[sockets]]
        name = "motor_commands"
        direction = "in"
        width = 16
        height = 16

        [[ports]]
        name = "retina_feed"
        direction = "in"
        entry_z = "Top"

          [[ports.pins]]
          name = "retina_left"
          width = 28
          height = 16
          local_u = 0.0
          local_v = 0.0
          u_width = 0.5
          v_height = 1.0
          target_type = "Stellate_Exc"
          stride = 1
          growth_steps = 255
          empty_pixel = "skip"

        [settings]
        ghost_capacity = 1024
        prune_threshold = 15
        max_sprouts = 4
        night_interval_ticks = 10000
        save_checkpoints_interval_ticks = 100000
    "#;

    #[test]
    fn test_parse_valid_shard() {
        let config = parse_shard_config(VALID_SHARD).unwrap();
        assert_eq!(config.dimensions.w, 256);
        assert_eq!(config.layers.len(), 2);
        assert_eq!(config.neuron_types.len(), 1);
        assert!(validate_shard(&config).is_ok());
    }

    #[test]
    fn test_validation_err_dimensions_overflow() {
        let invalid = VALID_SHARD.replace("w = 256", "w = 2000");
        let config = parse_shard_config(&invalid).unwrap();
        let res = validate_shard(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Invalid shard dimensions"));
    }

    #[test]
    fn test_validation_err_lut_limit() {
        let mut toml_lots_of_types = VALID_SHARD.to_string();
        for i in 0..16 {
            let type_toml = format!(r#"
                [[neuron_types]]
                name = "Type_{}"
                
                  [neuron_types.membrane]
                  threshold = 20000
                  rest_potential = -70000
                  leak_shift = 4
                  ahp_amplitude = 0
                  
                  [neuron_types.timings]
                  refractory_period = 5
                  synapse_refractory_period = 10
                  
                  [neuron_types.signal]
                  signal_propagation_length = 8
                  
                  [neuron_types.homeostasis]
                  homeostasis_penalty = 1500
                  homeostasis_decay = 990
                  
                  [neuron_types.adaptive_leak]
                  adaptive_leak_min_shift = -5
                  adaptive_leak_gain = 2
                  adaptive_mode = 1
                  
                  [neuron_types.dopamine]
                  d1_affinity = 80
                  d2_affinity = 20
                  
                  [neuron_types.gsop]
                  gsop_potentiation = 15
                  gsop_depression = 5
                  is_inhibitory = false
                  inertia_curve = [10, 20, 30, 40, 50, 60, 70, 80]

                  [neuron_types.growth]
                  steering_fov_deg = 60.0
                  steering_radius_um = 100.0
                  steering_weight_inertia = 0.6
                  steering_weight_sensor = 0.3
                  steering_weight_jitter = 0.1
                  dendrite_radius_um = 150.0
                  growth_vertical_bias = 0.7
                  type_affinity = 0.5
                  dendrite_whitelist = []
                  sprouting_weight_distance = 0.4
                  sprouting_weight_power = 0.4
                  sprouting_weight_explore = 0.1
                  sprouting_weight_type = 0.1

                  [neuron_types.spontaneous]
                  spontaneous_firing_period_ticks = 10000
            "#, i);
            toml_lots_of_types.push_str(&type_toml);
        }
        let config = parse_shard_config(&toml_lots_of_types).unwrap();
        let res = validate_shard(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-001"));
    }

    #[test]
    fn test_validation_err_overlapping_spikes() {
        let invalid = VALID_SHARD.replace("signal_propagation_length = 8", "signal_propagation_length = 3");
        let config = parse_shard_config(&invalid).unwrap();
        let res = validate_shard(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-004"));
    }

    #[test]
    fn test_validation_err_height_mismatch() {
        let invalid = VALID_SHARD.replace("height_pct = 0.6", "height_pct = 0.5");
        let config = parse_shard_config(&invalid).unwrap();
        let res = validate_shard(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Total height_pct"));
    }

    #[test]
    fn test_validation_err_pins_boundary() {
        let invalid = VALID_SHARD.replace("u_width = 0.5", "u_width = 1.5");
        let config = parse_shard_config(&invalid).unwrap();
        let res = validate_shard(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("bounds exceed maximum limit"));
    }
}
