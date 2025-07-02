//! Configuration types for core functionality
//!
//! This module provides configuration types and re-exports from the centralized
//! ratchet-config crate for consistency across the codebase.

// Re-export domain-specific configurations from centralized config
pub use ratchet_config::domains::{
    execution::ExecutionConfig,
    http::HttpConfig,
    logging::LoggingConfig,
    output::OutputConfig,
    server::ServerConfig,
    RatchetConfig,
};

use serde::{Deserialize, Serialize};

/// Storage configuration (extended from base)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StorageConfig {
    // Extended storage configuration specific to core
}

/// Plugin configuration 
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PluginConfig {
    // Plugin-specific configuration
}
