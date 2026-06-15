use crate::error::ConfigError;
use serde::Deserialize;

/// Root configuration describing all neuron types blueprints.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlueprintsConfig {
    pub neuron_types: Vec<NeuronType>,
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
    pub spontaneous: SpontaneousParams,
}

/// Membrane voltage parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MembraneParams {
    pub threshold: i32,
    pub rest_potential: i32,
    pub leak_shift: u32,
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

/// Background noise parameters.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpontaneousParams {
    pub spontaneous_firing_period_ticks: u32,
}

/// Parse TOML content into BlueprintsConfig.
pub fn parse_blueprints_config(content: &str) -> Result<BlueprintsConfig, ConfigError> {
    let config: BlueprintsConfig = toml::from_str(content)?;
    Ok(config)
}

/// Validate blueprints parameters against specification invariants.
pub fn validate_blueprints(config: &BlueprintsConfig) -> Result<(), ConfigError> {
    // INV-CONFIG-001: Neuron types cannot exceed 16 variants (LUT constant memory limit)
    if config.neuron_types.len() > 16 {
        return Err(ConfigError::ValidationError(format!(
            "INV-CONFIG-001: Maximum number of neuron types is 16, got {}",
            config.neuron_types.len()
        )));
    }

    for nt in &config.neuron_types {
        if nt.timings.refractory_period == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' refractory_period must be greater than zero",
                nt.name
            )));
        }
        if nt.signal.signal_propagation_length == 0 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' signal_propagation_length must be greater than zero",
                nt.name
            )));
        }

        // INV-CONFIG-004: Signal propagation tail must exceed absolute refractory period to prevent overlapping spikes
        if nt.signal.signal_propagation_length < nt.timings.refractory_period {
            return Err(ConfigError::ValidationError(format!(
                "INV-CONFIG-004: Neuron type '{}' signal_propagation_length ({}) must be >= refractory_period ({})",
                nt.name, nt.signal.signal_propagation_length, nt.timings.refractory_period
            )));
        }

        // Gsop plasticity inertia curve must have exactly 8 elements
        if nt.gsop.inertia_curve.len() != 8 {
            return Err(ConfigError::ValidationError(format!(
                "Neuron type '{}' gsop.inertia_curve must have exactly 8 elements, got {}",
                nt.name, nt.gsop.inertia_curve.len()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_BLUEPRINTS: &str = r#"
        [[neuron_types]]
        name = "Excitatory_L4"
        
        [neuron_types.membrane]
        threshold = 20000
        rest_potential = -70000
        leak_shift = 4
        
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
        
        [neuron_types.spontaneous]
        spontaneous_firing_period_ticks = 10000
    "#;

    #[test]
    fn test_parse_valid_blueprints() {
        let config = parse_blueprints_config(VALID_BLUEPRINTS).unwrap();
        assert_eq!(config.neuron_types.len(), 1);
        assert_eq!(config.neuron_types[0].name, "Excitatory_L4");
        assert_eq!(config.neuron_types[0].membrane.threshold, 20000);
        assert_eq!(config.neuron_types[0].gsop.inertia_curve[1], 20);
        assert!(validate_blueprints(&config).is_ok());
    }

    #[test]
    fn test_validation_err_lut_overflow() {
        let mut toml_lots_of_types = String::new();
        for i in 0..17 {
            let t = VALID_BLUEPRINTS.replace("name = \"Excitatory_L4\"", &format!("name = \"Type_{}\"", i));
            toml_lots_of_types.push_str(&t);
        }
        let config = parse_blueprints_config(&toml_lots_of_types).unwrap();
        let res = validate_blueprints(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-001"));
    }

    #[test]
    fn test_validation_err_overlapping_spikes() {
        // Change signal_propagation_length to 3 (which is < refractory_period of 5)
        let invalid = VALID_BLUEPRINTS.replace("signal_propagation_length = 8", "signal_propagation_length = 3");
        let config = parse_blueprints_config(&invalid).unwrap();
        let res = validate_blueprints(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-004"));
    }

    #[test]
    fn test_validation_err_bad_inertia_curve_length() {
        let invalid = VALID_BLUEPRINTS.replace("[10, 20, 30, 40, 50, 60, 70, 80]", "[10, 20]");
        let config = parse_blueprints_config(&invalid).unwrap();
        let res = validate_blueprints(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("gsop.inertia_curve"));
    }

    #[test]
    fn test_deny_unknown_fields() {
        let bad = r#"
            [[neuron_types]]
            name = "Excitatory"
            unknown_garbage = 123
        "#;
        let res = parse_blueprints_config(bad);
        assert!(res.is_err());
    }
}
