//! AxiEngine biological DSL configuration parser and validator.
//!
//! Provides DTOs, parsing and "Shift-Left" local validations for model, department,
//! and shard configuration files (`model.toml`, `department.toml`, `shard.toml`).

pub mod dto;
pub mod error;
pub mod validation;

pub use dto::*;
pub use error::ConfigError;
pub use validation::{validate_department, validate_model, validate_shard};

/// Parses a raw TOML string into a [`ModelConfig`].
///
/// # Errors
/// Returns [`ConfigError::ParseError`] if deserialization fails.
pub fn parse_model_str(toml_content: &str) -> Result<ModelConfig, ConfigError> {
    toml::from_str(toml_content).map_err(|e| ConfigError::ParseError(e.to_string()))
}

/// Parses a raw TOML string into a [`DepartmentConfig`].
///
/// # Errors
/// Returns [`ConfigError::ParseError`] if deserialization fails.
pub fn parse_department_str(toml_content: &str) -> Result<DepartmentConfig, ConfigError> {
    toml::from_str(toml_content).map_err(|e| ConfigError::ParseError(e.to_string()))
}

/// Parses a raw TOML string into a [`ShardConfig`].
///
/// # Errors
/// Returns [`ConfigError::ParseError`] if deserialization fails.
pub fn parse_shard_str(toml_content: &str) -> Result<ShardConfig, ConfigError> {
    toml::from_str(toml_content).map_err(|e| ConfigError::ParseError(e.to_string()))
}

/// Loads and parses a global model configuration from a file.
///
/// # Errors
/// Returns [`ConfigError::IoError`] if reading fails, or [`ConfigError::ParseError`] if parsing fails.
pub fn load_model_from_file<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<ModelConfig, ConfigError> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| ConfigError::IoError(e.to_string()))?;
    parse_model_str(&content)
}

/// Loads and parses a department configuration from a file.
///
/// # Errors
/// Returns [`ConfigError::IoError`] if reading fails, or [`ConfigError::ParseError`] if parsing fails.
pub fn load_department_from_file<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<DepartmentConfig, ConfigError> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| ConfigError::IoError(e.to_string()))?;
    parse_department_str(&content)
}

/// Loads and parses a shard configuration from a file.
///
/// # Errors
/// Returns [`ConfigError::IoError`] if reading fails, or [`ConfigError::ParseError`] if parsing fails.
pub fn load_shard_from_file<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<ShardConfig, ConfigError> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| ConfigError::IoError(e.to_string()))?;
    parse_shard_str(&content)
}
