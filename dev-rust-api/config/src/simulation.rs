use crate::error::ConfigError;
use serde::Deserialize;

fn default_segment_length_voxels() -> u32 {
    2
}

fn default_axon_growth_max_steps() -> u32 {
    255
}

/// System metadata block for configuration files.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SystemMeta {
    pub id: String,
    pub version: String,
    pub created_at: String,
}

/// Root configuration for the simulation model (model.toml).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    pub meta: Option<SystemMeta>,
    pub world: WorldConfig,
    pub simulation: SimulationParams,
    pub departments: Vec<DepartmentEntry>,
    pub connections: Vec<ModelConnectionConfig>,
}

/// Physical world dimensions.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldConfig {
    pub width_um: f64,
    pub depth_um: f64,
    pub height_um: f64,
}

/// Simulation run parameters and physical limits.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimulationParams {
    pub tick_duration_us: u32,
    pub total_ticks: u64,
    pub master_seed: String,
    pub voxel_size_um: f32,
    #[serde(default = "default_segment_length_voxels")]
    pub segment_length_voxels: u32,
    pub signal_speed_m_s: f32,
    pub sync_batch_ticks: u32,
    #[serde(default = "default_axon_growth_max_steps")]
    pub axon_growth_max_steps: u32,
    pub max_dendrites: u8,
}

/// Sub-brain department registration.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentEntry {
    pub meta: Option<SystemMeta>,
    pub name: String,
    pub config: String,
}

/// Inter-department connection config.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelConnectionConfig {
    pub from: String,
    pub to: String,
}

/// Parse TOML content into ModelConfig.
pub fn parse_model_config(content: &str) -> Result<ModelConfig, ConfigError> {
    let config: ModelConfig = toml::from_str(content)?;
    Ok(config)
}

/// Validate model parameters against specification invariants.
pub fn validate_model(config: &ModelConfig) -> Result<(), ConfigError> {
    let params = &config.simulation;

    if params.tick_duration_us == 0 {
        return Err(ConfigError::ValidationError(
            "tick_duration_us must be greater than zero".to_string(),
        ));
    }
    if params.voxel_size_um <= 0.0 {
        return Err(ConfigError::ValidationError(
            "voxel_size_um must be greater than zero".to_string(),
        ));
    }
    if params.signal_speed_m_s <= 0.0 {
        return Err(ConfigError::ValidationError(
            "signal_speed_m_s must be greater than zero".to_string(),
        ));
    }
    if params.segment_length_voxels == 0 {
        return Err(ConfigError::ValidationError(
            "segment_length_voxels must be greater than zero".to_string(),
        ));
    }

    // INV-CONFIG-003: Discrete step speed (v_seg) must evaluate to an exact integer
    let speed_um_tick = params.signal_speed_m_s * params.tick_duration_us as f32;
    let segment_length_um = params.voxel_size_um * params.segment_length_voxels as f32;
    let v_seg_f32 = speed_um_tick / segment_length_um;
    let rounded = v_seg_f32.round();
    if (v_seg_f32 - rounded).abs() > 1e-5 {
        return Err(ConfigError::ValidationError(format!(
            "INV-CONFIG-003: v_seg must be an integer, got f32 speed steps {} (speed_um_tick={}, segment_length_um={})",
            v_seg_f32, speed_um_tick, segment_length_um
        )));
    }

    // INV-CONFIG-005: Axon growth steps cannot exceed 255
    if params.axon_growth_max_steps > 255 {
        return Err(ConfigError::ValidationError(format!(
            "INV-CONFIG-005: axon_growth_max_steps must be <= 255, got {}",
            params.axon_growth_max_steps
        )));
    }

    // Max dendrites constraint
    if params.max_dendrites != 128 {
        return Err(ConfigError::ValidationError(format!(
            "max_dendrites must be exactly 128, got {}",
            params.max_dendrites
        )));
    }

    // Seed cannot be empty
    if params.master_seed.is_empty() {
        return Err(ConfigError::ValidationError(
            "master_seed must not be empty".to_string(),
        ));
    }

    // World bounds validation
    if config.world.width_um <= 0.0 || config.world.depth_um <= 0.0 || config.world.height_um <= 0.0 {
        return Err(ConfigError::ValidationError(
            "world dimensions must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
        [world]
        width_um = 1000.0
        depth_um = 1000.0
        height_um = 500.0

        [simulation]
        tick_duration_us = 1000
        total_ticks = 0
        master_seed = "test-seed"
        voxel_size_um = 10.0
        segment_length_voxels = 2
        signal_speed_m_s = 2.0  # speed_um_tick = 2000.0, segment_length_um = 20.0, v_seg = 100.0
        sync_batch_ticks = 10
        axon_growth_max_steps = 200
        max_dendrites = 128

        [[departments]]
        name = "cortex"
        config = "brain_cortex.toml"

        [[connections]]
        from = "thalamus.output"
        to = "cortex.input"
    "#;

    #[test]
    fn test_parse_valid_model() {
        let config = parse_model_config(VALID_TOML).unwrap();
        assert_eq!(config.world.width_um, 1000.0);
        assert_eq!(config.simulation.tick_duration_us, 1000);
        assert_eq!(config.simulation.segment_length_voxels, 2);
        assert_eq!(config.simulation.axon_growth_max_steps, 200);
        assert_eq!(config.simulation.max_dendrites, 128);
        assert!(validate_model(&config).is_ok());
    }

    #[test]
    fn test_default_values() {
        let toml_defaults = r#"
            departments = []
            connections = []

            [world]
            width_um = 1000.0
            depth_um = 1000.0
            height_um = 500.0

            [simulation]
            tick_duration_us = 1000
            total_ticks = 0
            master_seed = "test-seed"
            voxel_size_um = 10.0
            signal_speed_m_s = 2.0
            sync_batch_ticks = 10
            max_dendrites = 128
        "#;
        let config = parse_model_config(toml_defaults).unwrap();
        assert_eq!(config.simulation.segment_length_voxels, 2);
        assert_eq!(config.simulation.axon_growth_max_steps, 255);
        assert!(validate_model(&config).is_ok());
    }

    #[test]
    fn test_validation_err_v_seg_not_integer() {
        let invalid_toml = VALID_TOML.replace("signal_speed_m_s = 2.0", "signal_speed_m_s = 1.23");
        let config = parse_model_config(&invalid_toml).unwrap();
        let res = validate_model(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-003"));
    }

    #[test]
    fn test_validation_err_axon_growth_overflow() {
        let invalid_toml = VALID_TOML.replace("axon_growth_max_steps = 200", "axon_growth_max_steps = 300");
        let config = parse_model_config(&invalid_toml).unwrap();
        let res = validate_model(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-005"));
    }

    #[test]
    fn test_validation_err_dendrites_mismatch() {
        let invalid_toml = VALID_TOML.replace("max_dendrites = 128", "max_dendrites = 64");
        let config = parse_model_config(&invalid_toml).unwrap();
        let res = validate_model(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("max_dendrites"));
    }
}
