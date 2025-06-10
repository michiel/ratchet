//! Server configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Complete server configuration combining all subsystems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: HttpServerConfig,
    pub rest_api: RestApiConfig,
    pub graphql_api: GraphQLApiConfig,
    pub mcp_api: McpApiConfig,
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

/// MCP API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApiConfig {
    pub enabled: bool,
    pub sse_enabled: bool,
    pub host: String,
    pub port: u16,
    pub endpoint: String,
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
            mcp_api: McpApiConfig::default(),
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

impl Default for McpApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sse_enabled: true,
            host: "127.0.0.1".to_string(),
            port: 8090,
            endpoint: "/mcp".to_string(),
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
                enable_cors: server_config.cors.allowed_origins.contains(&"*".to_string()),
                enable_request_id: true, // Default enabled
                enable_tracing: true, // Default enabled  
                shutdown_timeout_seconds: 30, // Default value
            },
            rest_api: RestApiConfig {
                enabled: true, // Default enabled
                prefix: "/api/v1".to_string(), // Default prefix
                enable_health_checks: true,
                enable_detailed_health: true,
                enable_openapi_docs: true,
            },
            graphql_api: GraphQLApiConfig {
                enabled: true, // Default enabled
                endpoint: "/graphql".to_string(), // Default endpoint
                enable_playground: true, // Default enabled
                enable_introspection: true,
                max_query_depth: Some(15),
                max_query_complexity: Some(1000),
                enable_apollo_tracing: false,
            },
            mcp_api: McpApiConfig {
                enabled: config.mcp.as_ref().map_or(true, |mcp| mcp.enabled), // Default enabled unless explicitly disabled
                sse_enabled: config.mcp.as_ref().map_or(true, |mcp| mcp.transport == "sse"), // Default SSE enabled
                host: config.mcp.as_ref().map_or("127.0.0.1".to_string(), |mcp| mcp.host.clone()),
                port: config.mcp.as_ref().map_or(8090, |mcp| mcp.port),
                endpoint: "/mcp".to_string(), // Default endpoint
            },
            logging: LoggingConfig {
                level: format!("{:?}", config.logging.level).to_lowercase(),
                format: "json".to_string(),
                enable_structured: true,
                enable_file_logging: false,
                file_path: None,
            },
            database: DatabaseConfig {
                url: server_config.database.url,
                max_connections: server_config.database.max_connections as u32,
                min_connections: 1,
                connection_timeout_seconds: server_config.database.connection_timeout.as_secs(),
                enable_migrations: true,
            },
            registry: RegistryConfig {
                filesystem_paths: vec!["./tasks".to_string()], // Default value
                http_endpoints: Vec::new(), // Default empty
                sync_interval_seconds: 300, // Default 5 minutes
                enable_auto_sync: true, // Default enabled
                enable_validation: true, // Default enabled
            },
        })
    }
}