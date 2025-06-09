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