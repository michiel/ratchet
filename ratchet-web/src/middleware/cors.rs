use axum::http::{HeaderName, HeaderValue, Method};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

/// CORS configuration for different environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (use ["*"] for any origin in development only)
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers
    pub expose_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Maximum age for preflight cache
    pub max_age: Option<Duration>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            // Secure defaults - only allow localhost for development
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "https://localhost:3000".to_string(),
                "https://127.0.0.1:3000".to_string(),
            ],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "accept".to_string(),
                "x-requested-with".to_string(),
            ],
            expose_headers: vec![
                "x-total-count".to_string(),
                "content-range".to_string(),
            ],
            allow_credentials: false,
            max_age: Some(Duration::from_secs(3600)), // 1 hour
        }
    }
}

impl CorsConfig {
    /// Create development configuration with permissive settings
    pub fn development() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()], // Only for development!
            ..Default::default()
        }
    }

    /// Create production configuration with strict settings
    pub fn production(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
            allow_credentials: true, // Enable for production with specific origins
            ..Default::default()
        }
    }

    /// Validate CORS configuration for security
    pub fn validate(&self) -> Result<(), String> {
        // Check for security issues
        if self.allowed_origins.contains(&"*".to_string()) && self.allow_credentials {
            return Err("Cannot use wildcard origin '*' with allow_credentials: true".to_string());
        }

        // Warn about wildcard in production
        if self.allowed_origins.contains(&"*".to_string()) {
            tracing::warn!("CORS configured with wildcard origin '*' - this should only be used in development");
        }

        Ok(())
    }
}

/// Create CORS layer with default secure configuration
pub fn cors_layer() -> CorsLayer {
    cors_layer_with_config(CorsConfig::default())
}

/// Create CORS layer with custom configuration
pub fn cors_layer_with_config(config: CorsConfig) -> CorsLayer {
    // Validate configuration
    if let Err(e) = config.validate() {
        tracing::error!("Invalid CORS configuration: {}, falling back to secure defaults", e);
        // Fall back to secure defaults instead of panicking
        return cors_layer_with_config(CorsConfig::default());
    }

    let mut cors = CorsLayer::new();

    // Configure origins
    if config.allowed_origins.contains(&"*".to_string()) {
        cors = cors.allow_origin(Any);
        tracing::warn!("CORS configured to allow any origin - use only in development");
    } else {
        let origins: Result<Vec<HeaderValue>, _> = config
            .allowed_origins
            .iter()
            .map(|origin| origin.parse::<HeaderValue>())
            .collect();

        match origins {
            Ok(origins) => {
                cors = cors.allow_origin(origins);
            }
            Err(e) => {
                tracing::error!("Invalid origin in CORS configuration: {}", e);
                // Fall back to localhost only
                cors = cors.allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());
            }
        }
    }

    // Configure methods
    let methods: Vec<Method> = config
        .allowed_methods
        .iter()
        .filter_map(|method| method.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);

    // Configure headers
    let headers: Vec<HeaderName> = config
        .allowed_headers
        .iter()
        .filter_map(|header| header.parse().ok())
        .collect();
    cors = cors.allow_headers(headers);

    // Configure exposed headers
    let expose_headers: Vec<HeaderName> = config
        .expose_headers
        .iter()
        .filter_map(|header| header.parse().ok())
        .collect();
    cors = cors.expose_headers(expose_headers);

    // Configure credentials
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    // Configure max age
    if let Some(max_age) = config.max_age {
        cors = cors.max_age(max_age);
    }

    cors
}

/// Create development CORS layer (permissive - use only in development)
pub fn development_cors_layer() -> CorsLayer {
    cors_layer_with_config(CorsConfig::development())
}

/// Create production CORS layer with specific allowed origins
pub fn production_cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    cors_layer_with_config(CorsConfig::production(allowed_origins))
}
