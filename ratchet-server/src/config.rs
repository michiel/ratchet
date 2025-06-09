//! Server configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Complete server configuration combining all subsystems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: HttpServerConfig,
    pub rest_api: RestApiConfig,
    pub graphql_api: GraphQLApiConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub registry: RegistryConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    pub bind_address: SocketAddr,
    pub enable_cors: bool,
    pub enable_request_id: bool,
    pub enable_tracing: bool,
    pub shutdown_timeout_seconds: u64,
}

/// REST API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    pub enabled: bool,
    pub prefix: String,
    pub enable_health_checks: bool,
    pub enable_detailed_health: bool,
    pub enable_openapi_docs: bool,
}

/// GraphQL API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLApiConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub enable_playground: bool,
    pub enable_introspection: bool,
    pub max_query_depth: Option<usize>,
    pub max_query_complexity: Option<usize>,
    pub enable_apollo_tracing: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub enable_structured: bool,
    pub enable_file_logging: bool,
    pub file_path: Option<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub enable_migrations: bool,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub filesystem_paths: Vec<String>,
    pub http_endpoints: Vec<String>,
    pub sync_interval_seconds: u64,
    pub enable_auto_sync: bool,
    pub enable_validation: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: HttpServerConfig::default(),
            rest_api: RestApiConfig::default(),
            graphql_api: GraphQLApiConfig::default(),
            logging: LoggingConfig::default(),
            database: DatabaseConfig::default(),
            registry: RegistryConfig::default(),
        }
    }
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:3000".parse().unwrap(),
            enable_cors: true,
            enable_request_id: true,
            enable_tracing: true,
            shutdown_timeout_seconds: 30,
        }
    }
}

impl Default for RestApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: "/api/v1".to_string(),
            enable_health_checks: true,
            enable_detailed_health: true,
            enable_openapi_docs: true,
        }
    }
}

impl Default for GraphQLApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/graphql".to_string(),
            enable_playground: true,
            enable_introspection: true,
            max_query_depth: Some(15),
            max_query_complexity: Some(1000),
            enable_apollo_tracing: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            enable_structured: true,
            enable_file_logging: false,
            file_path: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://ratchet.db".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout_seconds: 30,
            enable_migrations: true,
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            filesystem_paths: vec!["./tasks".to_string()],
            http_endpoints: vec![],
            sync_interval_seconds: 300,
            enable_auto_sync: true,
            enable_validation: true,
        }
    }
}

impl ServerConfig {
    /// Convert from ratchet-config RatchetConfig to ServerConfig
    pub fn from_ratchet_config(config: ratchet_config::RatchetConfig) -> anyhow::Result<Self> {
        // Extract server configuration
        let server_config = config.server.ok_or_else(|| anyhow::anyhow!("Server configuration is required"))?;
        
        let bind_address = format!("{}:{}", server_config.bind_address, server_config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid bind address: {}", e))?;
        
        Ok(Self {
            server: HttpServerConfig {
                bind_address,
                enable_cors: server_config.enable_cors.unwrap_or(true),
                enable_request_id: server_config.enable_request_id.unwrap_or(true),
                enable_tracing: server_config.enable_tracing.unwrap_or(true),
                shutdown_timeout_seconds: server_config.shutdown_timeout.map(|t| t as u64).unwrap_or(30),
            },
            rest_api: RestApiConfig {
                enabled: config.rest_api.map(|r| r.enabled).unwrap_or(true),
                prefix: config.rest_api.map(|r| r.prefix).unwrap_or_else(|| "/api/v1".to_string()),
                enable_health_checks: true,
                enable_detailed_health: true,
                enable_openapi_docs: true,
            },
            graphql_api: GraphQLApiConfig {
                enabled: config.graphql_api.map(|g| g.enabled).unwrap_or(true),
                endpoint: config.graphql_api.map(|g| g.endpoint).unwrap_or_else(|| "/graphql".to_string()),
                enable_playground: config.graphql_api.and_then(|g| g.enable_playground).unwrap_or(true),
                enable_introspection: true,
                max_query_depth: Some(15),
                max_query_complexity: Some(1000),
                enable_apollo_tracing: false,
            },
            logging: LoggingConfig {
                level: config.logging.map(|l| l.level.to_string()).unwrap_or_else(|| "info".to_string()),
                format: "json".to_string(),
                enable_structured: true,
                enable_file_logging: false,
                file_path: None,
            },
            database: DatabaseConfig {
                url: server_config.database.url,
                max_connections: server_config.database.max_connections as u32,
                min_connections: 1,
                connection_timeout_seconds: server_config.database.connection_timeout.unwrap_or(30),
                enable_migrations: true,
            },
            registry: RegistryConfig {
                filesystem_paths: config.registry
                    .and_then(|r| r.filesystem_paths)
                    .unwrap_or_else(|| vec!["./tasks".to_string()]),
                http_endpoints: config.registry
                    .and_then(|r| r.http_endpoints)
                    .unwrap_or_else(Vec::new),
                sync_interval_seconds: config.registry
                    .and_then(|r| r.sync_interval_seconds.map(|s| s as u64))
                    .unwrap_or(300),
                enable_auto_sync: config.registry
                    .and_then(|r| r.enable_auto_sync)
                    .unwrap_or(true),
                enable_validation: config.registry
                    .and_then(|r| r.enable_validation)
                    .unwrap_or(true),
            },
        })
    }
}