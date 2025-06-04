//! Unified API layer (REST and GraphQL) for Ratchet
//!
//! This crate provides a unified API layer that supports both REST and GraphQL
//! interfaces, with shared types, error handling, and middleware.
//!
//! **Note**: This is a minimal implementation focused on core types and error handling.
//! Full REST and GraphQL implementations will be completed in a future phase.

pub mod config;
pub mod errors;
pub mod pagination;
pub mod types;

#[cfg(feature = "auth")]
pub mod middleware;

// REST and GraphQL modules
#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "graphql")]
pub mod graphql;

#[cfg(any(feature = "rest", feature = "graphql"))]
pub mod server;

// Re-export core types for convenience
pub use config::ApiConfig;
pub use errors::{ApiError, ApiResult};
pub use pagination::{PaginationInput, PaginationMeta, ListResponse};
pub use types::{ApiId, UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule, UnifiedWorker};

#[cfg(any(feature = "rest", feature = "graphql"))]
pub use server::create_api_server;

/// API version information
pub const API_VERSION: &str = "1.0.0";
pub const API_NAME: &str = "Ratchet API";

/// Health check response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub service: String,
}

impl HealthResponse {
    /// Create a healthy response
    pub fn healthy(uptime_seconds: u64) -> Self {
        Self {
            status: "healthy".to_string(),
            version: API_VERSION.to_string(),
            timestamp: chrono::Utc::now(),
            uptime_seconds,
            service: API_NAME.to_string(),
        }
    }
    
    /// Create an unhealthy response
    pub fn unhealthy(reason: impl Into<String>, uptime_seconds: u64) -> Self {
        Self {
            status: format!("unhealthy: {}", reason.into()),
            version: API_VERSION.to_string(),
            timestamp: chrono::Utc::now(),
            uptime_seconds,
            service: API_NAME.to_string(),
        }
    }
}

/// Create a basic API configuration for development
pub fn create_development_config() -> ApiConfig {
    ApiConfig::development()
}

/// Create a basic API configuration for production
pub fn create_production_config() -> ApiConfig {
    ApiConfig::production()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let health = HealthResponse::healthy(3600);
        assert_eq!(health.status, "healthy");
        assert_eq!(health.version, API_VERSION);
        assert_eq!(health.uptime_seconds, 3600);
    }

    #[test]
    fn test_unhealthy_response() {
        let health = HealthResponse::unhealthy("database connection failed", 1800);
        assert!(health.status.contains("unhealthy"));
        assert!(health.status.contains("database connection failed"));
        assert_eq!(health.uptime_seconds, 1800);
    }

    #[test]
    fn test_config_creation() {
        let dev_config = create_development_config();
        let prod_config = create_production_config();
        
        assert!(dev_config.validate().is_ok());
        
        // Production config needs CORS origins to be set for validation to pass
        let mut prod_config = prod_config;
        prod_config.cors.allowed_origins = vec!["https://example.com".to_string()];
        assert!(prod_config.validate().is_ok());
    }
}