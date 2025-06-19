//! Server startup and shutdown logic

use anyhow::Result;
use axum::{
    Router,
    routing::get,
    response::Html,
};
// use tower_http::{
//     trace::TraceLayer,
// };
use std::fs;
use std::sync::Arc;

// use ratchet_rest_api::context::TasksContext;

use ratchet_rest_api::app::{create_rest_app, AppConfig as RestAppConfig, AppContext as RestAppContext};
use ratchet_graphql_api::{
    schema::{create_schema, configure_schema, graphql_handler, graphql_playground},
    context::{GraphQLContext, GraphQLConfig},
};
// use ratchet_web::middleware::{cors_layer, request_id_layer, error_handler_layer};

#[cfg(feature = "mcp")]
use ratchet_mcp::{
    server::{McpServer, tools::{RatchetToolRegistry}},
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
            enable_security_headers: true,
            enable_audit_logging: true,
            enable_rate_limiting: true,
            enable_session_management: true,
            security_config: ratchet_web::middleware::SecurityConfig::development(),
            audit_config: ratchet_web::middleware::AuditConfig::development(),
            rate_limit_config: ratchet_web::middleware::RateLimitConfig::permissive(),
            session_config: ratchet_web::middleware::SessionConfig::development(),
        };

        // Always create the REST app (even if disabled, we use its context)
        let mut app = create_rest_app(rest_context, rest_config);
        
        // Add root handler
        app = app.route("/", get(root_handler));
        
        // Add admin UI handler
        app = app.route("/admin", get(admin_handler));

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
            
            // Create a separate router for GraphQL with the required extensions
            let graphql_router = Router::new()
                .route(
                    &self.config.graphql_api.endpoint,
                    axum::routing::get(graphql_handler).post(graphql_handler)
                )
                .layer(axum::extract::Extension(graphql_context.clone()))
                .layer(axum::extract::Extension(schema));
            
            // Add GraphQL Playground if enabled
            let graphql_router = if self.config.graphql_api.enable_playground {
                graphql_router.route("/playground", axum::routing::get(graphql_playground))
            } else {
                graphql_router
            };
            
            // Merge the GraphQL router into the main app
            app = app.merge(graphql_router);
        }

        // Add MCP SSE API if enabled
        if self.config.mcp_api.enabled && self.config.mcp_api.sse_enabled {
            tracing::info!("MCP SSE API enabled, creating MCP server and routes");
            
            #[cfg(feature = "mcp")]
            {
                // Create MCP server configuration
                let mcp_server_config = McpServerConfig::sse_with_host(
                    self.config.mcp_api.port,
                    &self.config.mcp_api.host
                );
                
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

    /// Initialize default schedules from embedded registry
    async fn initialize_default_schedules(&self) -> Result<()> {
        use ratchet_interfaces::ScheduleFilters;
        use ratchet_api_types::{PaginationInput, UnifiedSchedule, ApiId};
        use chrono::Utc;
        
        tracing::info!("Initializing default schedules from registry");
        
        // Only create heartbeat schedule if heartbeat task is available in repository
        let task_repo = self.services.repositories.task_repository();
        let heartbeat_task = match task_repo.find_by_name("heartbeat").await {
            Ok(Some(task)) => {
                tracing::debug!("Found heartbeat task in repository: {}", task.id);
                task
            }
            Ok(None) => {
                tracing::info!("Heartbeat task not found in repository, skipping default schedule creation");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Failed to check repository for heartbeat task: {}", e);
                return Ok(());
            }
        };
        
        // Check if heartbeat schedule already exists
        let schedule_repo = self.services.repositories.schedule_repository();
        let filters = ScheduleFilters {
            task_id: Some(heartbeat_task.id.clone()),
            name_exact: Some("system_heartbeat".to_string()),
            enabled: Some(true),
            next_run_before: None,
            task_id_in: None,
            id_in: None,
            name_contains: None,
            name_starts_with: None,
            name_ends_with: None,
            cron_expression_contains: None,
            cron_expression_exact: None,
            next_run_after: None,
            last_run_after: None,
            last_run_before: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            has_next_run: None,
            has_last_run: None,
            is_due: None,
            overdue: None,
        };
        let pagination = PaginationInput { page: Some(1), limit: Some(1), offset: None };
        
        match schedule_repo.find_with_filters(filters, pagination).await {
            Ok(response) if !response.items.is_empty() => {
                tracing::debug!("Heartbeat schedule already exists, skipping creation");
                return Ok(());
            }
            Ok(_) => {
                // No existing schedule, create one
                tracing::info!("Creating default heartbeat schedule");
            }
            Err(e) => {
                tracing::warn!("Failed to check for existing heartbeat schedule: {}", e);
                return Ok(());
            }
        }
        
        // Create default heartbeat schedule (every 5 minutes)
        let heartbeat_schedule = UnifiedSchedule {
            id: ApiId::from_i32(0), // Will be set by database
            task_id: heartbeat_task.id,
            name: "system_heartbeat".to_string(),
            description: Some("System health monitoring heartbeat - managed by scheduler".to_string()),
            cron_expression: "0 */5 * * * *".to_string(), // Every 5 minutes
            enabled: true,
            next_run: None, // Will be calculated by scheduler
            last_run: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Create the schedule in the repository
        match schedule_repo.create(heartbeat_schedule.clone()).await {
            Ok(created_schedule) => {
                tracing::info!("Created default heartbeat schedule with ID: {}", created_schedule.id);
                
                // Add to running scheduler if available
                if let Some(scheduler) = &self.services.scheduler_service {
                    if let Err(e) = scheduler.add_schedule(created_schedule).await {
                        tracing::warn!("Failed to add heartbeat schedule to running scheduler: {}", e);
                        // Schedule will be picked up on next scheduler restart
                    } else {
                        tracing::info!("Successfully added heartbeat schedule to running scheduler");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create default heartbeat schedule: {}", e);
            }
        }
        
        Ok(())
    }

    /// Start the server
    pub async fn start(self) -> Result<()> {
        let app = self.build_app();
        let addr = self.config.server.bind_address;

        tracing::info!("Starting Ratchet server on {}", addr);
        
        // Initialize heartbeat system
        if let Err(e) = self.services.heartbeat_service.initialize().await {
            tracing::warn!("Failed to initialize heartbeat system: {}", e);
            // Don't fail server startup for this
        }
        
        // Initialize default schedules from registry BEFORE starting scheduler
        if let Err(e) = self.initialize_default_schedules().await {
            tracing::warn!("Failed to initialize default schedules: {}", e);
            // Don't fail server startup for this
        }
        
        // Start scheduler service as background task AFTER schedules are initialized
        if let Some(scheduler_service) = &self.services.scheduler_service {
            let scheduler_clone = scheduler_service.clone();
            tokio::spawn(async move {
                if let Err(e) = scheduler_clone.start().await {
                    tracing::error!("Scheduler service failed: {}", e);
                }
            });
            tracing::info!("Started background scheduler service");
        }
        
        // Print configuration summary
        self.log_config_summary();

        // Start the server with TLS if configured
        if let Some(tls_config) = &self.config.server.tls {
            tracing::info!("Starting HTTPS server with TLS on {}", addr);
            self.start_tls_server(app, addr, tls_config).await?;
        } else {
            tracing::info!("Starting HTTP server on {}", addr);
            self.start_http_server(app, addr).await?;
        }

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Start HTTP server
    async fn start_http_server(&self, app: Router<()>, addr: std::net::SocketAddr) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }

    /// Start HTTPS server with TLS
    async fn start_tls_server(&self, app: Router<()>, addr: std::net::SocketAddr, tls_config: &crate::config::TlsConfig) -> Result<()> {
        // Load TLS certificates
        let cert_pem = fs::read(&tls_config.cert_path)
            .map_err(|e| anyhow::anyhow!("Failed to read certificate file '{}': {}", tls_config.cert_path, e))?;
        let key_pem = fs::read(&tls_config.key_path)
            .map_err(|e| anyhow::anyhow!("Failed to read private key file '{}': {}", tls_config.key_path, e))?;

        // Parse certificates
        let cert_chain = rustls_pemfile::certs(&mut cert_pem.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to parse certificate: {}", e))?;

        // Parse private key
        let private_key = rustls_pemfile::private_key(&mut key_pem.as_slice())
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("No private key found in key file"))?;

        // Build TLS configuration
        let rustls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| anyhow::anyhow!("Failed to build TLS configuration: {}", e))?;

        // Create axum-server RustlsConfig
        let axum_tls_config = axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(rustls_config));

        // Start HTTPS server using axum-server
        axum_server::bind_rustls(addr, axum_tls_config)
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("HTTPS server error: {}", e))?;

        Ok(())
    }

    /// Get list of available MCP tools
    #[cfg(feature = "mcp")]
    fn get_mcp_tools_list(&self) -> Vec<String> {
        // For startup logging, return a static list of known tools to avoid runtime issues
        // The actual tool registry initialization happens during MCP server setup
        vec![
            // Core execution tools
            "ratchet.execute_task".to_string(),
            "ratchet.get_execution_status".to_string(),
            "ratchet.get_execution_logs".to_string(),
            "ratchet.get_execution_trace".to_string(),
            "ratchet.list_available_tasks".to_string(),
            "ratchet.analyze_execution_error".to_string(),
            "ratchet.batch_execute".to_string(),
            
            // Task development tools
            "ratchet.create_task".to_string(),
            "ratchet.validate_task".to_string(),
            "ratchet.debug_task_execution".to_string(),
            "ratchet.run_task_tests".to_string(),
            "ratchet.create_task_version".to_string(),
            "ratchet.edit_task".to_string(),
            "ratchet.delete_task".to_string(),
            "ratchet.import_tasks".to_string(),
            "ratchet.export_tasks".to_string(),
            "ratchet.generate_from_template".to_string(),
            "ratchet.list_templates".to_string(),
            "ratchet.store_result".to_string(),
            "ratchet.get_results".to_string(),
        ]
    }

    /// Log configuration summary
    fn log_config_summary(&self) {
        tracing::info!("🚀 === Ratchet Server Configuration ===");
        tracing::info!("📍 Bind Address: {}", self.config.server.bind_address);
        
        // TLS Configuration
        if let Some(tls_config) = &self.config.server.tls {
            tracing::info!("🔒 TLS: ✅ Enabled (HTTPS)");
            tracing::info!("   📄 Certificate: {}", tls_config.cert_path);
            tracing::info!("   🔑 Private Key: {}", tls_config.key_path);
            tracing::info!("   ↩️  HTTP Redirect: {}", if tls_config.enable_http_redirect { "✅ Enabled" } else { "❌ Disabled" });
        } else {
            tracing::info!("🔒 TLS: ❌ Disabled (HTTP only)");
            tracing::warn!("⚠️  Production Warning: TLS is disabled. Enable TLS for production deployment.");
        }
        
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
        let protocol = if self.config.server.tls.is_some() { "https" } else { "http" };
        tracing::info!("📋 Available endpoints:");
        tracing::info!("   🏠 Root: {}://{}/", protocol, self.config.server.bind_address);
        tracing::info!("   🎛️  Admin UI: {}://{}/admin", protocol, self.config.server.bind_address);
        
        // Health endpoints
        tracing::info!("   ❤️  Health Endpoints:");
        tracing::info!("      • Basic Health:     {}://{}/health", protocol, self.config.server.bind_address);
        tracing::info!("      • Detailed Health:  {}://{}/health/detailed", protocol, self.config.server.bind_address);
        tracing::info!("      • Readiness:        {}://{}/ready", protocol, self.config.server.bind_address);
        tracing::info!("      • Liveness:         {}://{}/live", protocol, self.config.server.bind_address);
        
        // Monitoring endpoints
        tracing::info!("   📊 Monitoring Endpoints:");
        tracing::info!("      • System Metrics:   {}://{}/metrics", protocol, self.config.server.bind_address);
        tracing::info!("      • Prometheus:       {}://{}/metrics/prometheus", protocol, self.config.server.bind_address);
        
        // API Documentation endpoints
        tracing::info!("   📚 API Documentation:");
        tracing::info!("      • OpenAPI Spec:     {}://{}/api-docs/openapi.json", protocol, self.config.server.bind_address);
        tracing::info!("      • Swagger UI:       {}://{}/docs", protocol, self.config.server.bind_address);
        
        if self.config.rest_api.enabled {
            let base_url = format!("{}://{}", protocol, self.config.server.bind_address);
            let api_prefix = &self.config.rest_api.prefix;
            tracing::info!("   🔗 REST API Base: {}{}/", base_url, api_prefix);
            tracing::info!("      📝 Task Management:");
            tracing::info!("      • List Tasks:       GET    {}{}/tasks", base_url, api_prefix);
            tracing::info!("      • Create Task:      POST   {}{}/tasks", base_url, api_prefix);
            tracing::info!("      • Get Task:         GET    {}{}/tasks/:id", base_url, api_prefix);
            tracing::info!("      • Update Task:      PATCH  {}{}/tasks/:id", base_url, api_prefix);
            tracing::info!("      • Delete Task:      DELETE {}{}/tasks/:id", base_url, api_prefix);
            tracing::info!("      • Enable Task:      POST   {}{}/tasks/:id/enable", base_url, api_prefix);
            tracing::info!("      • Disable Task:     POST   {}{}/tasks/:id/disable", base_url, api_prefix);
            tracing::info!("      • Task Stats:       GET    {}{}/tasks/stats", base_url, api_prefix);
            tracing::info!("      • Sync Tasks:       POST   {}{}/tasks/sync", base_url, api_prefix);
            tracing::info!("      🔄 Execution Management:");
            tracing::info!("      • List Executions:  GET    {}{}/executions", base_url, api_prefix);
            tracing::info!("      • Get Execution:    GET    {}{}/executions/:id", base_url, api_prefix);
            tracing::info!("      ⚙️  Job Management:");
            tracing::info!("      • List Jobs:        GET    {}{}/jobs", base_url, api_prefix);
            tracing::info!("      • Get Job:          GET    {}{}/jobs/:id", base_url, api_prefix);
            tracing::info!("      📅 Schedule Management:");
            tracing::info!("      • List Schedules:   GET    {}{}/schedules", base_url, api_prefix);
            tracing::info!("      • Get Schedule:     GET    {}{}/schedules/:id", base_url, api_prefix);
            tracing::info!("      👷 Worker Management:");
            tracing::info!("      • List Workers:     GET    {}{}/workers", base_url, api_prefix);
            tracing::info!("      • Worker Stats:     GET    {}{}/workers/stats", base_url, api_prefix);
        }
        
        if self.config.graphql_api.enabled {
            tracing::info!("   🔍 GraphQL API:");
            tracing::info!("      • Endpoint:         {}://{}{}", protocol, self.config.server.bind_address, self.config.graphql_api.endpoint);
            tracing::info!("      • Queries:          tasks, executions, jobs, schedules, workers");
            tracing::info!("      • Mutations:        createTask, updateTask, deleteTask, etc.");
            if self.config.graphql_api.enable_playground {
                tracing::info!("      • Playground:       {}://{}/playground", protocol, self.config.server.bind_address);
            }
            if self.config.graphql_api.enable_introspection {
                tracing::info!("      • Introspection:    ✅ Enabled");
            }
        }
        
        if self.config.mcp_api.enabled {
            tracing::info!("   🤖 MCP Server-Sent Events API:");
            tracing::info!("      • Base Endpoint:    http://{}:{}{}", self.config.mcp_api.host, self.config.mcp_api.port, self.config.mcp_api.endpoint);
            
            // Dynamically list available MCP tools
            #[cfg(feature = "mcp")]
            {
                let tools = self.get_mcp_tools_list();
                if !tools.is_empty() {
                    tracing::info!("      • Tools Available:  {}", tools.join(", "));
                } else {
                    tracing::info!("      • Tools Available:  None");
                }
            }
            
            #[cfg(not(feature = "mcp"))]
            {
                tracing::info!("      • Tools Available:  MCP feature not compiled");
            }
            
            tracing::info!("      • Protocol:         Model Context Protocol v2024-11-05");
            tracing::info!("      • Transport:        Server-Sent Events (SSE)");
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
            "health": "/health",
            "admin": "/admin"
        }
    }))
}

/// Admin UI handler - serves the frontend application with CDN assets
async fn admin_handler() -> Html<String> {
    let html = r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" href="/favicon.ico" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="theme-color" content="#000000" />
    <meta
      name="description"
      content="Ratchet - Task execution and automation system"
    />
    <title>Ratchet Admin Dashboard</title>
    
    <!-- CDN Assets -->
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/michiel/ratchet-ui@main-build/style.css">
  </head>
  <body>
    <noscript>You need to enable JavaScript to run this app.</noscript>
    <div id="root"></div>
    
    <!-- CDN JavaScript -->
    <script src="https://cdn.jsdelivr.net/gh/michiel/ratchet-ui@main-build/script.js"></script>
  </body>
</html>"##;

    Html(html.to_string())
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