use crate::error::ConfigError;
use serde::Deserialize;

/// Composition representation of neuron types in a cortical layer.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NeuronTypeDistribution {
    pub type_name: String,
    pub share: f32,
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

/// Root configuration describing brain/cortical anatomy.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnatomyConfig {
    pub layers: Vec<LayerConfig>,
}

/// Parse TOML content into AnatomyConfig.
pub fn parse_anatomy_config(content: &str) -> Result<AnatomyConfig, ConfigError> {
    let config: AnatomyConfig = toml::from_str(content)?;
    Ok(config)
}

/// Validate anatomy parameters against specification invariants.
pub fn validate_anatomy(config: &AnatomyConfig) -> Result<(), ConfigError> {
    let mut total_height = 0.0;

    for layer in &config.layers {
        if layer.height_pct <= 0.0 {
            return Err(ConfigError::ValidationError(format!(
                "Layer height_pct must be greater than zero, got {}",
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
                    "Composition share must be non-negative, got {}",
                    comp.share
                )));
            }
            total_share += comp.share;
        }

        // Sum of composition shares must be 1.0 within 1e-4 tolerance
        if (total_share - 1.0).abs() > 1e-4 {
            return Err(ConfigError::ValidationError(format!(
                "Layer '{}' composition share sum must be 1.0 (got {})",
                layer.name, total_share
            )));
        }
    }

    // Sum of layer height percentages must be 1.0 within 1e-4 tolerance
    if (total_height - 1.0).abs() > 1e-4 {
        return Err(ConfigError::ValidationError(format!(
            "Total height_pct of all layers must be 1.0 (got {})",
            total_height
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_ANATOMY: &str = r#"
        [[layers]]
        name = "L1"
        height_pct = 0.1
        density = 50000.0
        composition = [
            { type_name = "L1_Interneuron", share = 1.0 }
        ]

        [[layers]]
        name = "L2_3"
        height_pct = 0.3
        density = 80000.0
        composition = [
            { type_name = "L2_3_Pyramidal", share = 0.8 },
            { type_name = "L2_3_Interneuron", share = 0.2 }
        ]

        [[layers]]
        name = "L4"
        height_pct = 0.2
        density = 100000.0
        composition = [
            { type_name = "L4_Stellate", share = 0.7 },
            { type_name = "L4_Interneuron", share = 0.3 }
        ]

        [[layers]]
        name = "L5_6"
        height_pct = 0.4
        density = 70000.0
        composition = [
            { type_name = "L5_6_Pyramidal", share = 0.9 },
            { type_name = "L5_6_Interneuron", share = 0.1 }
        ]
    "#;

    #[test]
    fn test_parse_valid_anatomy() {
        let config = parse_anatomy_config(VALID_ANATOMY).unwrap();
        assert_eq!(config.layers.len(), 4);
        assert_eq!(config.layers[0].name, "L1");
        assert_eq!(config.layers[1].composition.len(), 2);
        assert!(validate_anatomy(&config).is_ok());
    }

    #[test]
    fn test_validation_err_height_mismatch() {
        // Adjust height_pct so they sum to 0.95 instead of 1.0
        let invalid = VALID_ANATOMY.replace("height_pct = 0.1", "height_pct = 0.05");
        let config = parse_anatomy_config(&invalid).unwrap();
        let res = validate_anatomy(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Total height_pct"));
    }

    #[test]
    fn test_validation_err_negative_density() {
        let invalid = VALID_ANATOMY.replace("density = 50000.0", "density = -5.0");
        let config = parse_anatomy_config(&invalid).unwrap();
        let res = validate_anatomy(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("INV-CONFIG-002"));
    }

    #[test]
    fn test_validation_err_share_mismatch() {
        let invalid = VALID_ANATOMY.replace("share = 1.0", "share = 0.85");
        let config = parse_anatomy_config(&invalid).unwrap();
        let res = validate_anatomy(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("composition share sum"));
    }

    #[test]
    fn test_deny_unknown_fields() {
        let bad = r#"
            [[layers]]
            name = "L1"
            height_pct = 1.0
            density = 1000.0
            unknown_garbage = "oops"
            composition = []
        "#;
        let res = parse_anatomy_config(bad);
        assert!(res.is_err());
    }
}
