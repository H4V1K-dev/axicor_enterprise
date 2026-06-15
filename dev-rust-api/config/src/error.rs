use std::fmt;

/// Errors that can occur during configuration loading, parsing, or semantic validation.
#[derive(Debug)]
pub enum ConfigError {
    /// Failure during file system reading or other I/O operations.
    IoError(std::io::Error),
    /// TOML parsing or deserialization syntax failures.
    ParseError(String),
    /// Semantic constraint violations (invariants check failures).
    ValidationError(String),
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(err) => write!(f, "I/O Error: {}", err),
            ConfigError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::ParseError(err.to_string())
    }
}
