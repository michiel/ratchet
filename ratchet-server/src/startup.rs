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
// GraphQL temporarily disabled due to field mismatches
// use ratchet_graphql_api::{
//     schema::{create_schema, configure_schema, graphql_handler, graphql_playground},
//     context::GraphQLConfig,
// };
use ratchet_web::middleware::{cors_layer, request_id_layer, error_handler_layer};

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
    pub fn build_app(&self) -> Router {
        let mut app = Router::new();

        // Add REST API if enabled
        if self.config.rest_api.enabled {
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

            let rest_app = create_rest_app(rest_context, rest_config);
            app = app.merge(rest_app);
        }

        // Add GraphQL API if enabled (temporarily disabled)
        if self.config.graphql_api.enabled {
            tracing::warn!("GraphQL API is temporarily disabled due to field mismatches during migration");
            // TODO: Re-enable once field mappings are fixed
        }

        // Add root health endpoint
        app = app.route("/", get(root_handler));

        // Add global middleware layers
        app = app.layer(error_handler_layer());
        
        if self.config.server.enable_tracing {
            app = app.layer(TraceLayer::new_for_http());
        }
        
        if self.config.server.enable_request_id {
            app = app.layer(request_id_layer());
        }
        
        if self.config.server.enable_cors {
            app = app.layer(cors_layer());
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

        // Use axum 0.6 Server API
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Log configuration summary
    fn log_config_summary(&self) {
        tracing::info!("=== Ratchet Server Configuration ===");
        tracing::info!("Bind Address: {}", self.config.server.bind_address);
        tracing::info!("REST API: {} ({})", 
                      if self.config.rest_api.enabled { "Enabled" } else { "Disabled" },
                      self.config.rest_api.prefix);
        tracing::info!("GraphQL API: {} ({})", 
                      if self.config.graphql_api.enabled { "Enabled" } else { "Disabled" },
                      self.config.graphql_api.endpoint);
        tracing::info!("CORS: {}", if self.config.server.enable_cors { "Enabled" } else { "Disabled" });
        tracing::info!("Request ID: {}", if self.config.server.enable_request_id { "Enabled" } else { "Disabled" });
        tracing::info!("Tracing: {}", if self.config.server.enable_tracing { "Enabled" } else { "Disabled" });
        
        if self.config.graphql_api.enabled && self.config.graphql_api.enable_playground {
            tracing::info!("GraphQL Playground: http://{}/playground", self.config.server.bind_address);
        }
        
        tracing::info!("=====================================");
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
            "health": "/health"
        }
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