use crate::error::ConfigError;
use crate::simulation::SystemMeta;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentConfig {
    pub meta: Option<SystemMeta>,
    pub shards: Vec<ShardEntry>,
    pub connections: Vec<DepartmentConnection>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShardEntry {
    pub name: String,
    pub config: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DepartmentConnection {
    pub from: String,
    pub to: String,
}

pub fn parse_department_config(content: &str) -> Result<DepartmentConfig, ConfigError> {
    let config: DepartmentConfig = toml::from_str(content)?;
    Ok(config)
}

pub fn validate_department(config: &DepartmentConfig) -> Result<(), ConfigError> {
    let mut shard_names = HashSet::new();
    for shard in &config.shards {
        if shard.name.is_empty() {
            return Err(ConfigError::ValidationError(
                "Shard name must not be empty".to_string(),
            ));
        }
        if shard.config.is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Shard '{}' config path must not be empty",
                shard.name
            )));
        }
        if !shard_names.insert(&shard.name) {
            return Err(ConfigError::ValidationError(format!(
                "Duplicate shard name '{}'",
                shard.name
            )));
        }
    }

    for conn in &config.connections {
        if conn.from.is_empty() || conn.to.is_empty() {
            return Err(ConfigError::ValidationError(
                "Connection 'from' and 'to' must not be empty".to_string(),
            ));
        }
        let parts_from: Vec<&str> = conn.from.split('.').collect();
        let parts_to: Vec<&str> = conn.to.split('.').collect();
        if parts_from.len() != 2 || parts_to.len() != 2 {
            return Err(ConfigError::ValidationError(format!(
                "Invalid connection format (expected Shard.Socket): from='{}', to='{}'",
                conn.from, conn.to
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_DEPARTMENT: &str = r#"
        [[shards]]
        name = "Retina"
        config = "Retina/Retina.toml"

        [[shards]]
        name = "Auditory"
        config = "Auditory/Auditory.toml"

        [[connections]]
        from = "Retina.cross_modal"
        to = "Auditory.cross_feed"
    "#;

    #[test]
    fn test_parse_valid_department() {
        let config = parse_department_config(VALID_DEPARTMENT).unwrap();
        assert_eq!(config.shards.len(), 2);
        assert_eq!(config.connections.len(), 1);
        assert!(validate_department(&config).is_ok());
    }

    #[test]
    fn test_validation_err_duplicate_shards() {
        let invalid = r#"
            connections = []

            [[shards]]
            name = "Retina"
            config = "Retina/Retina.toml"

            [[shards]]
            name = "Retina"
            config = "Retina2/Retina.toml"
        "#;
        let config = parse_department_config(invalid).unwrap();
        let res = validate_department(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Duplicate shard name"));
    }

    #[test]
    fn test_validation_err_invalid_connection_format() {
        let invalid = r#"
            shards = []
            [[connections]]
            from = "Retina"
            to = "Auditory.cross_feed"
        "#;
        let config = parse_department_config(invalid).unwrap();
        let res = validate_department(&config);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Invalid connection format"));
    }
}
