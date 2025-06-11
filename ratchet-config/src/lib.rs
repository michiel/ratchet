//! Domain-driven configuration management for Ratchet
//!
//! This crate provides modular configuration split by functional domains,
//! with validation, defaults, and environment variable support.

pub mod error;
pub mod loader;
pub mod validation;

// Compatibility layer for migration from ratchet-lib
pub mod compat;

// Configuration auto-migration functionality
pub mod migration;

// CLI utilities (feature-gated)
#[cfg(feature = "cli")]
pub mod cli;

// Domain-specific configuration modules
pub mod domains;

// Re-export main types
pub use error::{ConfigError, ConfigResult};
pub use loader::ConfigLoader;

// Re-export migration types
pub use migration::{
    ConfigMigrator, ConfigCompatibilityService, ConfigFormat, MigrationReport
};

// Re-export CLI types (feature-gated)
#[cfg(feature = "cli")]
pub use cli::{ConfigCli, ConfigCommand, ConfigCliRunner};

// Re-export domain configurations
pub use domains::{
    cache::CacheConfig, database::DatabaseConfig, execution::ExecutionConfig, http::HttpConfig,
    logging::LoggingConfig, mcp::McpConfig, output::OutputConfig, registry::RegistryConfig,
    server::ServerConfig, RatchetConfig,
};

// Re-export utilities
pub use domains::utils::serde_duration;
