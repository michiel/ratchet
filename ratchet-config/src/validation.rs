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

/// Validate a webhook URL with security checks
pub fn validate_webhook_url(
    url: &str, 
    field_name: &str, 
    domain: &str,
    allow_localhost: bool,
    allow_private_networks: bool,
    allowed_domains: &[String],
) -> ConfigResult<()> {
    use std::net::IpAddr;
    
    // First do basic URL validation
    validate_url(url, field_name, domain)?;
    
    let parsed_url = url::Url::parse(url).unwrap(); // Already validated above
    
    // Only allow HTTP/HTTPS schemes
    match parsed_url.scheme() {
        "http" | "https" => {}
        scheme => return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} scheme '{}' not allowed for webhooks (only http/https)", field_name, scheme),
        }),
    }
    
    let Some(host) = parsed_url.host_str() else {
        return Err(ConfigError::DomainError {
            domain: domain.to_string(),
            message: format!("{} must have a valid host", field_name),
        });
    };
    
    // Check if domain is explicitly allowed
    for allowed_domain in allowed_domains {
        if host == allowed_domain || host.ends_with(&format!(".{}", allowed_domain)) {
            return Ok(());
        }
    }
    
    // Check for localhost/loopback
    if !allow_localhost {
        let localhost_domains = ["localhost", "127.0.0.1", "::1", "0.0.0.0"];
        for blocked_domain in &localhost_domains {
            if host == *blocked_domain || host.ends_with(&format!(".{}", blocked_domain)) {
                return Err(ConfigError::DomainError {
                    domain: domain.to_string(),
                    message: format!(
                        "{} cannot target localhost/loopback addresses ({}). Set allow_localhost_webhooks=true to override", 
                        field_name, host
                    ),
                });
            }
        }
    }
    
    // Check for private network ranges
    if !allow_private_networks {
        if let Ok(ip) = host.parse::<IpAddr>() {
            let is_private = match ip {
                IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    // 10.0.0.0/8
                    octets[0] == 10 ||
                    // 172.16.0.0/12
                    (octets[0] == 172 && (16..=31).contains(&octets[1])) ||
                    // 192.168.0.0/16
                    (octets[0] == 192 && octets[1] == 168) ||
                    // Link-local 169.254.0.0/16
                    (octets[0] == 169 && octets[1] == 254)
                }
                IpAddr::V6(ipv6) => {
                    // Private/local IPv6 ranges
                    ipv6.is_loopback() || 
                    ipv6.segments()[0] == 0xfc00 || // Unique local fc00::/7
                    ipv6.segments()[0] == 0xfe80    // Link-local fe80::/10
                }
            };
            
            if is_private {
                return Err(ConfigError::DomainError {
                    domain: domain.to_string(),
                    message: format!(
                        "{} cannot target private network addresses ({}). Set allow_private_network_webhooks=true to override", 
                        field_name, host
                    ),
                });
            }
        }
    }
    
    // Check for cloud metadata endpoints (always blocked for security)
    let metadata_endpoints = [
        "169.254.169.254",           // AWS metadata
        "metadata.google.internal",  // GCP metadata
        "169.254.0.1",              // Azure metadata
    ];
    
    for blocked_endpoint in &metadata_endpoints {
        if host == *blocked_endpoint {
            return Err(ConfigError::DomainError {
                domain: domain.to_string(),
                message: format!(
                    "{} cannot target cloud metadata endpoints ({})", 
                    field_name, host
                ),
            });
        }
    }
    
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
