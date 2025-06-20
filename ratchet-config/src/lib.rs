//! Domain-driven configuration management for Ratchet
//!
//! This crate provides modular configuration split by functional domains,
//! with validation, defaults, and environment variable support.

pub mod error;
pub mod loader;
pub mod validation;

// Legacy compatibility layer removed - migration complete

// Domain-specific configuration modules
pub mod domains;

// Re-export main types
pub use error::{ConfigError, ConfigResult};
pub use loader::ConfigLoader;

// Re-export domain configurations
pub use domains::{
    cache::CacheConfig, database::DatabaseConfig, execution::ExecutionConfig, http::HttpConfig, logging::LoggingConfig,
    mcp::McpConfig, output::OutputConfig, registry::RegistryConfig, server::ServerConfig, RatchetConfig,
};

// Re-export utilities
pub use domains::utils::serde_duration;
