//! Domain-driven configuration management for Ratchet
//!
//! This crate provides modular configuration split by functional domains,
//! with validation, defaults, and environment variable support.

pub mod error;
pub mod loader;
pub mod validation;

// Domain-specific configuration modules
pub mod domains;

// Re-export main types
pub use error::{ConfigError, ConfigResult};
pub use loader::ConfigLoader;

// Re-export domain configurations
pub use domains::{
    RatchetConfig,
    execution::ExecutionConfig,
    http::HttpConfig,
    cache::CacheConfig,
    logging::LoggingConfig,
    output::OutputConfig,
    server::ServerConfig,
    database::DatabaseConfig,
    registry::RegistryConfig,
    mcp::McpConfig,
};

// Re-export utilities
pub use domains::utils::serde_duration;