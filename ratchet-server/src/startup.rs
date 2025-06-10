//! Server startup and shutdown logic

use anyhow::Result;
use axum::{
    Router,
    routing::get,
};
use tower_http::{
    trace::TraceLayer,
};

use ratchet_rest_api::context::TasksContext;

use ratchet_rest_api::app::{create_rest_app, AppConfig as RestAppConfig, AppContext as RestAppContext};
use ratchet_graphql_api::{
    schema::{create_schema, configure_schema, graphql_handler, graphql_playground},
    context::{GraphQLContext, GraphQLConfig},
};
use ratchet_web::middleware::{cors_layer, request_id_layer, error_handler_layer};

#[cfg(feature = "mcp")]
use ratchet_mcp::{
    server::{McpServer, tools::RatchetToolRegistry, adapter::RatchetMcpAdapter},
    security::{McpAuth, McpAuthManager, AuditLogger},
    server::config::McpServerConfig,
};

use crate::{config::ServerConfig, services::ServiceContainer};

/// Server application struct
pub struct Server {
    config: ServerConfig,
    services: ServiceContainer,
}

impl Server {
    /// Create a new server instance
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Initialize logging first
        crate::services::init_logging(&config).await?;
        
        // Create service container
        let services = ServiceContainer::new(&config).await?;
        
        Ok(Self { config, services })
    }

    /// Build the complete application router
    pub fn build_app(&self) -> Router<()> {
        // Create REST API context
        let rest_context = RestAppContext {
            tasks: self.services.rest_context(),
            executions: ratchet_rest_api::context::ExecutionsContext::new(self.services.repositories.clone()),
            jobs: ratchet_rest_api::context::JobsContext::new(self.services.repositories.clone()),
            schedules: ratchet_rest_api::context::SchedulesContext::new(self.services.repositories.clone()),
            workers: ratchet_rest_api::context::WorkersContext::new(),
        };

        let rest_config = RestAppConfig {
            api_prefix: self.config.rest_api.prefix.clone(),
            enable_cors: self.config.server.enable_cors,
            enable_request_id: self.config.server.enable_request_id,
            enable_tracing: self.config.server.enable_tracing,
        };

        // Always create the REST app (even if disabled, we use its context)
        let mut app = create_rest_app(rest_context, rest_config);
        
        // Add root handler
        app = app.route("/", get(root_handler));

        // Add GraphQL API if enabled
        if self.config.graphql_api.enabled {
            tracing::info!("GraphQL API enabled, creating schema and routes");
            
            // Create GraphQL context
            let graphql_context = GraphQLContext::new(
                self.services.repositories.clone(),
                self.services.registry.clone(),
                self.services.registry_manager.clone(),
                self.services.validator.clone(),
            );
            
            // Create GraphQL configuration
            let graphql_config = GraphQLConfig {
                enable_playground: self.config.graphql_api.enable_playground,
                enable_introspection: self.config.graphql_api.enable_introspection,
                max_query_depth: self.config.graphql_api.max_query_depth,
                max_query_complexity: self.config.graphql_api.max_query_complexity,
                enable_tracing: true, // Enable tracing for GraphQL operations
                enable_apollo_tracing: self.config.graphql_api.enable_apollo_tracing,
            };
            
            // Create and configure the GraphQL schema
            let schema = configure_schema(create_schema(), &graphql_config);
            
            // Add GraphQL endpoint (supporting both GET and POST)
            app = app.route(
                &self.config.graphql_api.endpoint,
                axum::routing::get(graphql_handler).post(graphql_handler)
                    .with_state(graphql_context.clone())
            );
            
            // Add GraphQL Playground if enabled
            if self.config.graphql_api.enable_playground {
                app = app.route("/playground", axum::routing::get(graphql_playground)
                    .with_state(graphql_context.clone()));
            }
            
            // Add GraphQL schema extension as shared state
            app = app.layer(axum::extract::Extension(schema));
        }

        // Add MCP SSE API if enabled
        if self.config.mcp_api.enabled && self.config.mcp_api.sse_enabled {
            tracing::info!("MCP SSE API enabled, creating MCP server and routes");
            
            #[cfg(feature = "mcp")]
            {
                // Create MCP server configuration
                let mcp_server_config = McpServerConfig {
                    host: self.config.mcp_api.host.clone(),
                    port: self.config.mcp_api.port,
                    transport: "sse".to_string(),
                    enabled: true,
                    max_concurrent_requests: 50,
                    request_timeout_secs: 30,
                    enable_cors: true,
                    cors_allow_origins: vec!["*".to_string()],
                };
                
                // Create MCP adapter (placeholder - would need actual task executor)
                // For now, create a minimal MCP server with basic tools
                let tool_registry = RatchetToolRegistry::new();
                let auth_manager = std::sync::Arc::new(McpAuthManager::new(McpAuth::default()));
                let audit_logger = std::sync::Arc::new(AuditLogger::new(false));
                
                let mcp_server = McpServer::new(
                    mcp_server_config,
                    std::sync::Arc::new(tool_registry),
                    auth_manager,
                    audit_logger,
                );
                
                // Create and nest MCP SSE routes
                let mcp_routes = mcp_server.create_sse_routes();
                app = app.nest(&self.config.mcp_api.endpoint, mcp_routes);
            }
            
            #[cfg(not(feature = "mcp"))]
            {
                tracing::warn!("MCP API enabled in config but mcp feature not available at compile time");
                // Add placeholder endpoints
                app = app.route(&self.config.mcp_api.endpoint, axum::routing::get(mcp_placeholder_handler));
                app = app.route(&format!("{}/health", self.config.mcp_api.endpoint), axum::routing::get(mcp_health_handler));
            }
        }

        app
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        let app = self.build_app();
        let addr = self.config.server.bind_address;

        tracing::info!("Starting Ratchet server on {}", addr);
        
        // Print configuration summary
        self.log_config_summary();

        // Start the server
        tracing::info!("Server listening on {}", addr);

        // Use axum 0.6 Server API with stateful router
        let make_service = app.into_make_service();
        axum::Server::bind(&addr)
            .serve(make_service)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Log configuration summary
    fn log_config_summary(&self) {
        tracing::info!("🚀 === Ratchet Server Configuration ===");
        tracing::info!("📍 Bind Address: {}", self.config.server.bind_address);
        
        // Core APIs
        tracing::info!("🔗 REST API: {} ({})", 
                      if self.config.rest_api.enabled { "✅ Enabled" } else { "❌ Disabled" },
                      self.config.rest_api.prefix);
        tracing::info!("🔍 GraphQL API: {} ({})", 
                      if self.config.graphql_api.enabled { "✅ Enabled" } else { "❌ Disabled" },
                      self.config.graphql_api.endpoint);
        tracing::info!("🤖 MCP SSE API: {} ({})", 
                      if self.config.mcp_api.enabled { "✅ Enabled" } else { "❌ Disabled" },
                      self.config.mcp_api.endpoint);
        
        // Features
        if self.config.graphql_api.enabled && self.config.graphql_api.enable_playground {
            tracing::info!("🎮 GraphQL Playground: ✅ Enabled");
        }
        
        // Middleware
        tracing::info!("🌐 CORS: {}", if self.config.server.enable_cors { "✅ Enabled" } else { "❌ Disabled" });
        tracing::info!("🆔 Request ID: {}", if self.config.server.enable_request_id { "✅ Enabled" } else { "❌ Disabled" });
        tracing::info!("📊 Tracing: {}", if self.config.server.enable_tracing { "✅ Enabled" } else { "❌ Disabled" });
        
        // Endpoints
        tracing::info!("📋 Available endpoints:");
        tracing::info!("   🏠 Root: http://{}/", self.config.server.bind_address);
        tracing::info!("   ❤️  Health: http://{}/health", self.config.server.bind_address);
        
        if self.config.rest_api.enabled {
            tracing::info!("   🔗 REST API: http://{}{}/", self.config.server.bind_address, self.config.rest_api.prefix);
        }
        
        if self.config.graphql_api.enabled {
            tracing::info!("   🔍 GraphQL: http://{}{}", self.config.server.bind_address, self.config.graphql_api.endpoint);
            if self.config.graphql_api.enable_playground {
                tracing::info!("   🎮 Playground: http://{}/playground", self.config.server.bind_address);
            }
        }
        
        if self.config.mcp_api.enabled {
            tracing::info!("   🤖 MCP SSE: http://{}:{}{}", self.config.mcp_api.host, self.config.mcp_api.port, self.config.mcp_api.endpoint);
        }
        
        tracing::info!("✅ =====================================");
    }
}

/// Root handler
async fn root_handler() -> axum::response::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "service": "Ratchet Task Execution System",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "endpoints": {
            "rest_api": "/api/v1",
            "graphql": "/graphql", 
            "playground": "/playground",
            "mcp_sse": "/mcp",
            "health": "/health"
        }
    }))
}


/// MCP SSE placeholder handler
async fn mcp_placeholder_handler() -> axum::response::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "message": "MCP SSE API is enabled and ready",
        "status": "placeholder",
        "protocol": "Model Context Protocol over Server-Sent Events",
        "note": "Full MCP SSE implementation will be added in future updates"
    }))
}

/// MCP health handler
async fn mcp_health_handler() -> axum::response::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "MCP SSE",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Graceful shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}