//! Configuration validation traits and utilities

use crate::error::{ConfigError, ConfigResult};

/// Trait for validatable configuration
pub trait Validatable {
    /// Validate the configuration
    fn validate(&self) -> ConfigResult<()>;

    /// Get the domain name for error reporting
    fn domain_name(&self) -> &'static str;

    /// Helper to create a domain-specific validation error
    fn validation_error(&self, message: impl Into<String>) -> ConfigError {
        ConfigError::DomainError {
            domain: self.domain_name().to_string(),
            message: message.into(),
        }
    }
}

/// Validate a required string field
pub fn validate_required_string(value: &str, field_name: &str, domain: &str) -> ConfigResult<()> {
    if value.is_empty() {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} cannot be empty", field_name),
        });
    }
    Ok(())
}

/// Validate a positive number
pub fn validate_positive<T>(value: T, field_name: &str, domain: &str) -> ConfigResult<()>
where
    T: PartialOrd + Default + std::fmt::Display,
{
    if value <= T::default() {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} must be greater than 0, got {}", field_name, value),
        });
    }
    Ok(())
}

/// Validate a URL
pub fn validate_url(url: &str, field_name: &str, domain: &str) -> ConfigResult<()> {
    if url.is_empty() {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} cannot be empty", field_name),
        });
    }

    // Parse URL to validate format
    url::Url::parse(url).map_err(|e| ConfigError::DomainError {
        domain: domain.to_string(),
        message: format!("{} has invalid URL format: {}", field_name, e),
    })?;

    Ok(())
}

/// Validate an enum choice
pub fn validate_enum_choice<T>(value: &str, valid_choices: &[T], field_name: &str, domain: &str) -> ConfigResult<()>
where
    T: AsRef<str>,
{
    let valid: Vec<&str> = valid_choices.iter().map(|c| c.as_ref()).collect();

    if !valid.iter().any(|&v| v.eq_ignore_ascii_case(value)) {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!(
                "{} has invalid value '{}'. Valid choices: {}",
                field_name,
                value,
                valid.join(", ")
            ),
        });
    }

    Ok(())
}

/// Validate a port number
pub fn validate_port_range(port: u16, field_name: &str, domain: &str) -> ConfigResult<()> {
    if port == 0 {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} cannot be 0", field_name),
        });
    }

    // Port 1-1023 are typically reserved for system services
    if port <= 1023 {
        log::warn!("{} port {} is in the reserved range (1-1023)", field_name, port);
    }

    Ok(())
}

/// Validate a complete configuration object
pub fn validate_config(config: &crate::domains::RatchetConfig) -> ConfigResult<()> {
    // Validate all domains that implement the Validatable trait
    config.execution.validate()?;
    config.http.validate()?;
    config.logging.validate()?;
    config.cache.validate()?;
    config.output.validate()?;

    // Validate optional domains
    if let Some(server) = &config.server {
        server.validate()?;
    }

    if let Some(registry) = &config.registry {
        registry.validate()?;
    }

    if let Some(mcp) = &config.mcp {
        mcp.validate()?;
    }

    Ok(())
}
