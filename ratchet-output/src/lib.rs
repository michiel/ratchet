//! # Ratchet Output System
//!
//! This crate provides a flexible system for delivering task outputs to various
//! destinations including filesystem, webhooks, databases, and message queues.
//!
//! ## Features
//!
//! - **Multiple Destinations**: Support for filesystem, webhooks, databases, and cloud storage
//! - **Template Engine**: Dynamic configuration using Handlebars templates
//! - **Retry Logic**: Configurable retry policies with exponential backoff
//! - **Authentication**: Multiple auth methods for webhooks (Bearer, Basic, API Key, HMAC)
//! - **Format Support**: JSON, YAML, CSV, and custom templates
//! - **Async/Await**: Full async support for non-blocking operations
//!
//! ## Example
//!
//! ```rust
//! use ratchet_output::{OutputDeliveryManager, OutputDestinationConfig, OutputFormat, TaskOutput, DeliveryContext};
//! use serde_json::json;
//! use chrono::Utc;
//! use std::collections::HashMap;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OutputDestinationConfig::Filesystem {
//!     path: "/results/{{job_id}}.json".to_string(),
//!     format: OutputFormat::Json,
//!     permissions: 0o644,
//!     create_dirs: true,
//!     overwrite: false,
//!     backup_existing: false,
//! };
//!
//! let manager = OutputDeliveryManager::new();
//! manager.add_destination("results".to_string(), config).await?;
//!
//! let output = TaskOutput {
//!     job_id: 1,
//!     task_id: 1,
//!     execution_id: 1,
//!     output_data: json!({"status": "success", "result": 42}),
//!     metadata: HashMap::new(),
//!     completed_at: Utc::now(),
//!     execution_duration: Duration::from_secs(1),
//! };
//! let context = DeliveryContext::default();
//! manager.deliver_output("results", &output, &context).await?;
//! # Ok(())
//! # }
//! ```

pub mod destination;
pub mod destinations;
pub mod errors;
pub mod manager;
pub mod metrics;
pub mod template;

pub use destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput};
pub use destinations::{FilesystemDestination, WebhookDestination};
pub use errors::{ConfigError, DeliveryError, ValidationError};
pub use manager::{OutputDeliveryManager, TestResult};
pub use template::TemplateEngine;


// Re-export HttpMethod from ratchet-http for consistency
pub use ratchet_http::HttpMethod;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for output destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputDestinationConfig {
    #[serde(rename = "filesystem")]
    Filesystem {
        path: String,         // Template: /results/{{job_id}}/{{timestamp}}.json
        format: OutputFormat, // json, yaml, csv, raw
        #[serde(default = "default_permissions")]
        permissions: u32, // File permissions (default: 644)
        #[serde(default = "default_true")]
        create_dirs: bool, // Create parent directories (default: true)
        #[serde(default)]
        overwrite: bool, // Overwrite existing files (default: false)
        #[serde(default)]
        backup_existing: bool, // Backup existing files (default: false)
    },
    #[serde(rename = "webhook")]
    Webhook {
        url: String, // Template: https://api.{{env}}.com/webhook/{{job_id}}
        #[serde(default = "default_post_method")]
        method: HttpMethod, // POST, PUT, PATCH
        #[serde(default)]
        headers: HashMap<String, String>, // Template values in headers
        #[serde(
            default = "default_webhook_timeout",
            alias = "timeout_seconds",
            with = "duration_serde"
        )]
        timeout: Duration, // Request timeout (default: 30s)
        #[serde(default)]
        retry_policy: RetryPolicy, // Retry configuration
        auth: Option<WebhookAuth>, // Authentication configuration
        content_type: Option<String>, // Override content-type header
    },
    #[serde(rename = "database")]
    Database {
        connection: String,               // Database connection string
        table: String,                    // Table name
        columns: HashMap<String, String>, // Column mapping
        #[serde(default)]
        upsert: bool, // Insert or update
    },
    #[serde(rename = "s3")]
    S3 {
        bucket: String,                // S3 bucket name
        key: String,                   // Object key template
        region: String,                // AWS region
        storage_class: Option<String>, // Storage class
        #[serde(default)]
        metadata: HashMap<String, String>, // Object metadata
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[serde(rename = "json")]
    #[default]
    Json, // Pretty JSON
    #[serde(rename = "json_compact")]
    JsonCompact, // Minified JSON
    #[serde(rename = "yaml")]
    Yaml, // YAML format
    #[serde(rename = "csv")]
    Csv, // CSV (for array outputs)
    #[serde(rename = "raw")]
    Raw, // Raw output as-is
    #[serde(rename = "template")]
    Template(String), // Custom template
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebhookAuth {
    #[serde(rename = "bearer")]
    Bearer { token: String }, // Bearer token
    #[serde(rename = "basic")]
    Basic { username: String, password: String }, // Basic auth
    #[serde(rename = "api_key")]
    ApiKey { header: String, key: String }, // API key in header
    #[serde(rename = "signature")]
    Signature { secret: String, algorithm: String }, // HMAC signature
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32, // Maximum retry attempts (default: 3)
    #[serde(default = "default_initial_delay", with = "duration_serde")]
    pub initial_delay: Duration, // Initial delay (default: 1s)
    #[serde(default = "default_max_delay", with = "duration_serde")]
    pub max_delay: Duration, // Maximum delay (default: 60s)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64, // Backoff multiplier (default: 2.0)
    #[serde(default = "default_true")]
    pub jitter: bool, // Add random jitter (default: true)
    #[serde(default = "default_retry_status_codes")]
    pub retry_on_status: Vec<u16>, // HTTP status codes to retry on
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_delay: default_initial_delay(),
            max_delay: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            jitter: default_true(),
            retry_on_status: default_retry_status_codes(),
        }
    }
}

/// Context for job execution
#[derive(Debug, Clone, Default)]
pub struct JobContext {
    pub job_uuid: String,
    pub task_name: String,
    pub task_version: String,
    pub schedule_id: Option<i32>,
    pub priority: String,
    pub environment: String,
}

// Helper functions for serde defaults
fn default_permissions() -> u32 {
    0o644
}
fn default_true() -> bool {
    true
}
fn default_post_method() -> HttpMethod {
    HttpMethod::Post
}
fn default_webhook_timeout() -> Duration {
    Duration::from_secs(30)
}
fn default_max_attempts() -> u32 {
    3
}
fn default_initial_delay() -> Duration {
    Duration::from_secs(1)
}
fn default_max_delay() -> Duration {
    Duration::from_secs(60)
}
fn default_backoff_multiplier() -> f64 {
    2.0
}
fn default_retry_status_codes() -> Vec<u16> {
    vec![429, 500, 502, 503, 504]
}

// Duration serialization helper
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}