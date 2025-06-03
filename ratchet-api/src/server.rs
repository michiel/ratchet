//! Unified API server implementation

use axum::Router;
use std::net::SocketAddr;

use crate::{
    config::ApiConfig,
    errors::{ApiError, ApiResult},
};

#[cfg(feature = "rest")]
use crate::rest::create_rest_router;

#[cfg(feature = "graphql")]
use crate::graphql::{create_graphql_router, create_schema, GraphQLContext};

/// Unified API server
pub struct ApiServer {
    config: ApiConfig,
    router: Router,
}

impl ApiServer {
    /// Create a new API server with the given configuration
    pub fn new(config: ApiConfig) -> ApiResult<Self> {
        let router = create_router(&config)?;
        
        Ok(Self {
            config,
            router,
        })
    }
    
    /// Run the server
    pub async fn run(self) -> ApiResult<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.server.port));
        
        tracing::info!("Starting API server on {}", addr);
        
        axum::Server::bind(&addr)
            .serve(self.router.into_make_service())
            .await
            .map_err(|e| ApiError::internal(format!("Server error: {}", e)))?;
        
        Ok(())
    }
}

/// Create the main router with all enabled features
fn create_router(config: &ApiConfig) -> ApiResult<Router> {
    let mut router = Router::new();
    
    // Add REST API routes if enabled
    #[cfg(feature = "rest")]
    {
        let rest_router = create_rest_router(config)?;
        router = router.merge(rest_router);
    }
    
    // Add GraphQL routes if enabled
    #[cfg(feature = "graphql")]
    {
        let context = GraphQLContext::new();
        let schema = create_schema(context);
        let graphql_router = create_graphql_router(config, schema)?;
        
        router = router.merge(graphql_router);
    }
    
    // Add a fallback route
    router = router.fallback(fallback_handler);
    
    Ok(router)
}

/// Fallback handler for unmatched routes
async fn fallback_handler() -> ApiResult<&'static str> {
    Err(ApiError::not_found("Route not found"))
}

/// Create and run the API server with the given configuration
pub async fn create_api_server(config: ApiConfig) -> ApiResult<()> {
    let server = ApiServer::new(config)?;
    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_server_creation() {
        let config = ApiConfig::development();
        let result = ApiServer::new(config);
        assert!(result.is_ok());
    }
}