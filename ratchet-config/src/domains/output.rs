//! Output destinations configuration

use crate::error::ConfigResult;
use crate::validation::{
    validate_enum_choice, validate_positive, validate_required_string, validate_url, Validatable,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Output destinations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Maximum number of concurrent deliveries
    #[serde(default = "default_max_concurrent_deliveries")]
    pub max_concurrent_deliveries: usize,

    /// Default timeout for deliveries
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_delivery_timeout"
    )]
    pub default_timeout: Duration,

    /// Whether to validate destination configurations on startup
    #[serde(default = "crate::domains::utils::default_true")]
    pub validate_on_startup: bool,

    /// Global output destination templates
    #[serde(default)]
    pub global_destinations: Vec<OutputDestinationTemplate>,

    /// Default retry policy for failed deliveries
    #[serde(default)]
    pub default_retry_policy: RetryPolicyConfig,

    /// Output formatting configuration
    #[serde(default)]
    pub formatting: OutputFormattingConfig,
}

/// Output destination template for reuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDestinationTemplate {
    /// Template name for reference
    pub name: String,

    /// Template description
    pub description: Option<String>,

    /// Destination configuration
    pub destination: OutputDestinationConfigTemplate,
}

/// Output destination configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutputDestinationConfigTemplate {
    Filesystem {
        /// Path template with variables
        path: String,
        /// Output format
        #[serde(default = "default_output_format")]
        format: String,
        /// File permissions (octal as string)
        #[serde(default = "default_file_permissions")]
        permissions: String,
        /// Whether to create directories
        #[serde(default = "crate::domains::utils::default_true")]
        create_dirs: bool,
        /// Whether to overwrite existing files
        #[serde(default = "crate::domains::utils::default_true")]
        overwrite: bool,
        /// Whether to backup existing files
        #[serde(default = "crate::domains::utils::default_false")]
        backup_existing: bool,
    },
    Webhook {
        /// Webhook URL template
        url: String,
        /// HTTP method
        #[serde(default = "default_http_method")]
        method: String,
        /// HTTP headers
        #[serde(default)]
        headers: HashMap<String, String>,
        /// Request timeout in seconds
        #[serde(default = "default_webhook_timeout")]
        timeout_seconds: u64,
        /// Content type header
        content_type: Option<String>,
        /// Authentication configuration
        auth: Option<WebhookAuthConfig>,
    },
    Database {
        /// Database connection string
        connection_string: String,
        /// Target table name
        table_name: String,
        /// Column mappings
        column_mappings: HashMap<String, String>,
        /// Connection pool configuration
        #[serde(default)]
        pool_config: DatabasePoolConfig,
    },
    S3 {
        /// S3 bucket name
        bucket: String,
        /// Object key template
        key_template: String,
        /// AWS region
        region: String,
        /// AWS access key ID (optional, can use environment)
        access_key_id: Option<String>,
        /// AWS secret access key (optional, can use environment)
        secret_access_key: Option<String>,
        /// Storage class
        #[serde(default = "default_s3_storage_class")]
        storage_class: String,
    },
}

/// Webhook authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WebhookAuthConfig {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { header: String, value: String },
}

/// Database pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections
    #[serde(default = "default_db_max_connections")]
    pub max_connections: u32,

    /// Connection timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_db_connection_timeout"
    )]
    pub connection_timeout: Duration,

    /// Idle timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_db_idle_timeout"
    )]
    pub idle_timeout: Duration,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryPolicyConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: i32,

    /// Initial delay between retries in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Backoff multiplier for exponential backoff
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

/// Output formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputFormattingConfig {
    /// Default timestamp format
    #[serde(default = "default_timestamp_format")]
    pub timestamp_format: String,

    /// Whether to include metadata in output
    #[serde(default = "crate::domains::utils::default_true")]
    pub include_metadata: bool,

    /// Whether to pretty-print JSON
    #[serde(default = "crate::domains::utils::default_false")]
    pub pretty_json: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_concurrent_deliveries: default_max_concurrent_deliveries(),
            default_timeout: default_delivery_timeout(),
            validate_on_startup: true,
            global_destinations: Vec::new(),
            default_retry_policy: RetryPolicyConfig::default(),
            formatting: OutputFormattingConfig::default(),
        }
    }
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_retries(),
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

impl Default for OutputFormattingConfig {
    fn default() -> Self {
        Self {
            timestamp_format: default_timestamp_format(),
            include_metadata: true,
            pretty_json: false,
        }
    }
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_db_max_connections(),
            connection_timeout: default_db_connection_timeout(),
            idle_timeout: default_db_idle_timeout(),
        }
    }
}

impl Validatable for OutputConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(
            self.max_concurrent_deliveries,
            "max_concurrent_deliveries",
            self.domain_name(),
        )?;

        validate_positive(
            self.default_timeout.as_secs(),
            "default_timeout",
            self.domain_name(),
        )?;

        self.default_retry_policy.validate()?;
        self.formatting.validate()?;

        // Validate global destination templates
        for (index, template) in self.global_destinations.iter().enumerate() {
            template.validate_with_context(&format!("global_destinations[{}]", index))?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "output"
    }
}

impl Validatable for RetryPolicyConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(self.max_attempts, "max_attempts", self.domain_name())?;
        validate_positive(
            self.initial_delay_ms,
            "initial_delay_ms",
            self.domain_name(),
        )?;
        validate_positive(self.max_delay_ms, "max_delay_ms", self.domain_name())?;

        if self.max_delay_ms < self.initial_delay_ms {
            return Err(self.validation_error(
                "max_delay_ms must be greater than or equal to initial_delay_ms",
            ));
        }

        if self.backoff_multiplier <= 1.0 {
            return Err(self.validation_error("backoff_multiplier must be greater than 1.0"));
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "output.retry_policy"
    }
}

impl Validatable for OutputFormattingConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(
            &self.timestamp_format,
            "timestamp_format",
            self.domain_name(),
        )?;
        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "output.formatting"
    }
}

impl OutputDestinationTemplate {
    pub fn validate_with_context(&self, context: &str) -> ConfigResult<()> {
        if self.name.is_empty() {
            return Err(crate::error::ConfigError::DomainError {
                domain: "output".to_string(),
                message: format!("{} has empty name", context),
            });
        }

        self.destination
            .validate_with_context(&format!("{}.{}", context, self.name))
    }
}

impl OutputDestinationConfigTemplate {
    pub fn validate_with_context(&self, context: &str) -> ConfigResult<()> {
        match self {
            Self::Filesystem {
                path,
                format,
                permissions,
                ..
            } => {
                if path.is_empty() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "output".to_string(),
                        message: format!("{} filesystem destination has empty path", context),
                    });
                }

                let valid_formats = ["json", "json_compact", "yaml", "csv", "raw", "template"];
                validate_enum_choice(format, &valid_formats, "format", "output")?;

                // Validate permissions format (octal)
                if !permissions.chars().all(|c| c.is_ascii_digit() && c <= '7') {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "output".to_string(),
                        message: format!("{} has invalid file permissions format", context),
                    });
                }
            }

            Self::Webhook {
                url,
                method,
                timeout_seconds,
                auth,
                ..
            } => {
                validate_url(url, "url", "output")?;

                let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
                validate_enum_choice(method, &valid_methods, "method", "output")?;

                validate_positive(*timeout_seconds, "timeout_seconds", "output")?;

                if let Some(ref auth_config) = auth {
                    auth_config.validate()?;
                }
            }

            Self::Database {
                connection_string,
                table_name,
                pool_config,
                ..
            } => {
                validate_required_string(connection_string, "connection_string", "output")?;
                validate_required_string(table_name, "table_name", "output")?;
                pool_config.validate()?;
            }

            Self::S3 {
                bucket,
                key_template,
                region,
                storage_class,
                ..
            } => {
                validate_required_string(bucket, "bucket", "output")?;
                validate_required_string(key_template, "key_template", "output")?;
                validate_required_string(region, "region", "output")?;

                let valid_storage_classes = [
                    "STANDARD",
                    "REDUCED_REDUNDANCY",
                    "STANDARD_IA",
                    "ONEZONE_IA",
                    "INTELLIGENT_TIERING",
                    "GLACIER",
                    "DEEP_ARCHIVE",
                ];
                validate_enum_choice(
                    storage_class,
                    &valid_storage_classes,
                    "storage_class",
                    "output",
                )?;
            }
        }

        Ok(())
    }
}

impl Validatable for WebhookAuthConfig {
    fn validate(&self) -> ConfigResult<()> {
        match self {
            Self::Bearer { token } => {
                validate_required_string(token, "token", self.domain_name())?;
            }
            Self::Basic { username, password } => {
                validate_required_string(username, "username", self.domain_name())?;
                validate_required_string(password, "password", self.domain_name())?;
            }
            Self::ApiKey { header, value } => {
                validate_required_string(header, "header", self.domain_name())?;
                validate_required_string(value, "value", self.domain_name())?;
            }
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "output.webhook.auth"
    }
}

impl Validatable for DatabasePoolConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(self.max_connections, "max_connections", self.domain_name())?;
        validate_positive(
            self.connection_timeout.as_secs(),
            "connection_timeout",
            self.domain_name(),
        )?;
        validate_positive(
            self.idle_timeout.as_secs(),
            "idle_timeout",
            self.domain_name(),
        )?;
        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "output.database.pool"
    }
}

// Default value functions
fn default_max_concurrent_deliveries() -> usize {
    10
}

fn default_delivery_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_output_format() -> String {
    "json".to_string()
}

fn default_file_permissions() -> String {
    "644".to_string()
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn default_webhook_timeout() -> u64 {
    30
}

fn default_s3_storage_class() -> String {
    "STANDARD".to_string()
}

fn default_max_retries() -> i32 {
    3
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_timestamp_format() -> String {
    "%Y-%m-%dT%H:%M:%S%.3fZ".to_string()
}

fn default_db_max_connections() -> u32 {
    10
}

fn default_db_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_db_idle_timeout() -> Duration {
    Duration::from_secs(600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_config_defaults() {
        let config = OutputConfig::default();
        assert_eq!(config.max_concurrent_deliveries, 10);
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert!(config.validate_on_startup);
    }

    #[test]
    fn test_retry_policy_validation() {
        let mut policy = RetryPolicyConfig::default();
        assert!(policy.validate().is_ok());

        // Test invalid backoff multiplier
        policy.backoff_multiplier = 0.5;
        assert!(policy.validate().is_err());

        // Test invalid delay relationship
        policy = RetryPolicyConfig::default();
        policy.max_delay_ms = 500;
        policy.initial_delay_ms = 1000;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_webhook_auth_validation() {
        let auth = WebhookAuthConfig::Bearer {
            token: "test-token".to_string(),
        };
        assert!(auth.validate().is_ok());

        let invalid_auth = WebhookAuthConfig::Bearer {
            token: String::new(),
        };
        assert!(invalid_auth.validate().is_err());
    }
}
