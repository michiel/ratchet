//! Error types for the output destination system

use thiserror::Error;

/// Errors that can occur during output delivery
#[derive(Debug, Error, Clone)]
pub enum DeliveryError {
    #[error("Template rendering failed: {template} - {error}")]
    TemplateRender {
        template: String,
        error: String,
    },

    #[error("Serialization failed for format {format}: {error}")]
    Serialization {
        format: String,
        error: String,
    },

    #[error("Filesystem operation failed at {path} ({operation}): {error}")]
    Filesystem {
        path: String,
        operation: String,
        error: String,
    },

    #[error("File already exists: {path}")]
    FileExists { path: String },

    #[error("Webhook request failed to {url} (HTTP {status}): {response}")]
    WebhookFailed {
        url: String,
        status: u16,
        response: String,
    },

    #[error("Network error for {url}: {error}")]
    Network {
        url: String,
        error: String,
    },

    #[error("Failed to clone request for retry")]
    RequestClone,

    #[error("Maximum retry attempts exceeded for {destination} ({attempts} attempts)")]
    MaxRetriesExceeded {
        destination: String,
        attempts: u32,
    },

    #[error("Task join error: {error}")]
    TaskJoin { error: String },

    #[error("Invalid template variable: {variable}")]
    InvalidTemplateVariable { variable: String },

    #[error("Database error: {operation} - {error}")]
    Database {
        operation: String,
        error: String,
    },

    #[error("S3 error: {operation} - {error}")]
    S3 {
        operation: String,
        error: String,
    },
}

/// Configuration validation errors
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Path template cannot be empty")]
    EmptyPath,

    #[error("URL template cannot be empty")]
    EmptyUrl,

    #[error("Header name cannot be empty")]
    EmptyHeaderName,

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Invalid file permissions: {0:o}")]
    InvalidPermissions(u32),

    #[error("Invalid timeout: must be between 1s and 300s")]
    InvalidTimeout,

    #[error("Invalid retry policy: {reason}")]
    InvalidRetryPolicy { reason: String },
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid destination configuration for {destination_type}: {error}")]
    InvalidDestination {
        destination_type: String,
        #[source]
        error: ValidationError,
    },

    #[error("Unsupported destination type: {0}")]
    UnsupportedDestination(String),

    #[error("Failed to create HTTP client: {source}")]
    HttpClientCreate {
        #[source]
        source: reqwest::Error,
    },

    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    #[error("Invalid configuration value for {field}: {value}")]
    InvalidValue { field: String, value: String },
}