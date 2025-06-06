//! Compatibility layer for migration from ratchet-lib config
//!
//! This module provides conversion functions and compatibility types
//! to ease the transition from the old ratchet-lib configuration system
//! to the new domain-driven configuration in ratchet-config.

use crate::domains::RatchetConfig as NewRatchetConfig;

/// Convert new RatchetConfig to ratchet-lib's config format
///
/// This function is used during the migration period to maintain compatibility
/// with components that still expect the old configuration format.
pub fn to_legacy_config(new_config: &NewRatchetConfig) -> LegacyRatchetConfig {
    // Convert database config
    let database_config = LegacyDatabaseConfig {
        url: "sqlite::memory:".to_string(), // Default for now
        max_connections: 10,
        connection_timeout: std::time::Duration::from_secs(30),
    };

    // Convert server config
    let server_config = LegacyServerConfig {
        bind_address: new_config
            .server
            .as_ref()
            .map(|s| s.bind_address.clone())
            .unwrap_or_else(|| "0.0.0.0".to_string()),
        port: new_config.server.as_ref().map(|s| s.port).unwrap_or(8080),
        database: database_config,
    };

    // Convert execution config
    let execution_config = LegacyExecutionConfig {
        max_execution_duration: new_config.execution.max_execution_duration.as_secs(),
        validate_schemas: new_config.execution.validate_schemas,
        max_concurrent_tasks: 4,  // Default
        timeout_grace_period: 30, // Default
    };

    // Convert HTTP config
    let http_config = LegacyHttpConfig {
        timeout: new_config.http.timeout,
        user_agent: new_config.http.user_agent.clone(),
        verify_ssl: new_config.http.verify_ssl,
        max_redirects: 10, // Default
    };

    // Convert logging config
    let logging_config = LegacyLoggingConfig {
        level: "info".to_string(),     // Default
        format: "text".to_string(),    // Default
        output: "console".to_string(), // Default
    };

    // Convert MCP config
    let mcp_config = new_config.mcp.as_ref().map(|mcp| LegacyMcpServerConfig {
        enabled: mcp.enabled,
        transport: mcp.transport.clone(),
        host: mcp.host.clone(),
        port: mcp.port,
    });

    LegacyRatchetConfig {
        server: Some(server_config),
        execution: execution_config,
        http: http_config,
        logging: logging_config,
        mcp: mcp_config,
    }
}

// Legacy configuration structures that match ratchet-lib's format
// These will be removed once the migration is complete

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyRatchetConfig {
    pub server: Option<LegacyServerConfig>,
    pub execution: LegacyExecutionConfig,
    pub http: LegacyHttpConfig,
    pub logging: LegacyLoggingConfig,
    pub mcp: Option<LegacyMcpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub database: LegacyDatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyDatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyExecutionConfig {
    pub max_execution_duration: u64,
    pub validate_schemas: bool,
    pub max_concurrent_tasks: usize,
    pub timeout_grace_period: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyHttpConfig {
    pub timeout: Duration,
    pub user_agent: String,
    pub verify_ssl: bool,
    pub max_redirects: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyLoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMcpServerConfig {
    pub enabled: bool,
    pub transport: String,
    pub host: String,
    pub port: u16,
}

impl Default for LegacyRatchetConfig {
    fn default() -> Self {
        Self {
            server: Some(LegacyServerConfig::default()),
            execution: LegacyExecutionConfig::default(),
            http: LegacyHttpConfig::default(),
            logging: LegacyLoggingConfig::default(),
            mcp: None,
        }
    }
}

impl Default for LegacyServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            database: LegacyDatabaseConfig::default(),
        }
    }
}

impl Default for LegacyDatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for LegacyExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_duration: 300, // 5 minutes
            validate_schemas: true,
            max_concurrent_tasks: 4,
            timeout_grace_period: 30,
        }
    }
}

impl Default for LegacyHttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            user_agent: "Ratchet/1.0".to_string(),
            verify_ssl: true,
            max_redirects: 10,
        }
    }
}

impl Default for LegacyLoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            output: "console".to_string(),
        }
    }
}
