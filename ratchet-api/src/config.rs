//! API configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Server configuration
    pub server: ServerConfig,
    
    /// REST API configuration
    pub rest: RestApiConfig,
    
    /// GraphQL API configuration
    pub graphql: GraphqlApiConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable graceful shutdown
    pub graceful_shutdown: bool,
    
    /// Shutdown timeout
    pub shutdown_timeout: Duration,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Keep-alive timeout
    pub keep_alive: Duration,
}

/// REST API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    /// Enable REST API
    pub enabled: bool,
    
    /// API base path
    pub base_path: String,
    
    /// API version
    pub version: String,
    
    /// Enable API documentation
    pub docs_enabled: bool,
    
    /// Maximum request body size
    pub max_body_size: usize,
    
    /// Enable legacy Refine.dev compatibility
    pub refine_compatibility: bool,
}

/// GraphQL API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphqlApiConfig {
    /// Enable GraphQL API
    pub enabled: bool,
    
    /// GraphQL endpoint path
    pub endpoint: String,
    
    /// Enable GraphQL Playground
    pub playground_enabled: bool,
    
    /// Playground endpoint path
    pub playground_path: String,
    
    /// Enable introspection
    pub introspection_enabled: bool,
    
    /// Query complexity limit
    pub complexity_limit: Option<usize>,
    
    /// Query depth limit
    pub depth_limit: Option<usize>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security headers
    pub security_headers: bool,
    
    /// Allowed hosts
    pub allowed_hosts: Vec<String>,
    
    /// Enable HTTPS only
    pub https_only: bool,
    
    /// Enable content type validation
    pub content_type_validation: bool,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: Option<usize>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Requests per minute
    pub requests_per_minute: u32,
    
    /// Burst size
    pub burst_size: u32,
    
    /// Rate limit by IP
    pub by_ip: bool,
    
    /// Rate limit by API key
    pub by_api_key: bool,
    
    /// Custom headers to include in rate limit key
    pub custom_headers: Vec<String>,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,
    
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    
    /// Exposed headers
    pub exposed_headers: Vec<String>,
    
    /// Allow credentials
    pub allow_credentials: bool,
    
    /// Max age for preflight requests
    pub max_age: Duration,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,
    
    /// JWT configuration
    pub jwt: Option<JwtConfig>,
    
    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,
    
    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,
}

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AuthType {
    /// No authentication
    #[default]
    None,
    
    /// JWT token authentication
    Jwt,
    
    /// API key authentication
    ApiKey,
    
    /// OAuth 2.0 authentication
    OAuth,
    
    /// Multiple authentication methods
    Multiple(Vec<AuthType>),
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret or public key
    pub secret: String,
    
    /// JWT algorithm
    pub algorithm: String,
    
    /// Token expiration time
    pub expiration: Duration,
    
    /// Token issuer
    pub issuer: Option<String>,
    
    /// Token audience
    pub audience: Option<String>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Header name for API key
    pub header_name: String,
    
    /// Query parameter name for API key
    pub query_param: Option<String>,
    
    /// Valid API keys
    pub keys: Vec<String>,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth provider
    pub provider: String,
    
    /// Client ID
    pub client_id: String,
    
    /// Client secret
    pub client_secret: String,
    
    /// Authorization URL
    pub auth_url: String,
    
    /// Token URL
    pub token_url: String,
    
    /// Scopes
    pub scopes: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            rest: RestApiConfig::default(),
            graphql: GraphqlApiConfig::default(),
            security: SecurityConfig::default(),
            rate_limit: Some(RateLimitConfig::default()),
            cors: CorsConfig::default(),
            auth: None,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            graceful_shutdown: true,
            shutdown_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            keep_alive: Duration::from_secs(75),
        }
    }
}

impl Default for RestApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_path: "/api/v1".to_string(),
            version: "1.0.0".to_string(),
            docs_enabled: true,
            max_body_size: 16 * 1024 * 1024, // 16MB
            refine_compatibility: true,
        }
    }
}

impl Default for GraphqlApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/graphql".to_string(),
            playground_enabled: cfg!(debug_assertions),
            playground_path: "/playground".to_string(),
            introspection_enabled: cfg!(debug_assertions),
            complexity_limit: Some(1000),
            depth_limit: Some(20),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            security_headers: true,
            allowed_hosts: vec!["*".to_string()],
            https_only: false,
            content_type_validation: true,
            max_concurrent_requests: Some(1000),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 60,
            burst_size: 10,
            by_ip: true,
            by_api_key: false,
            custom_headers: Vec::new(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
            ],
            exposed_headers: vec![
                "x-total-count".to_string(),
                "x-request-id".to_string(),
            ],
            allow_credentials: false,
            max_age: Duration::from_secs(3600),
        }
    }
}


impl ApiConfig {
    /// Create a development configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.graphql.playground_enabled = true;
        config.graphql.introspection_enabled = true;
        config.security.https_only = false;
        config.rest.docs_enabled = true;
        config
    }
    
    /// Create a production configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.graphql.playground_enabled = false;
        config.graphql.introspection_enabled = false;
        config.security.https_only = true;
        config.rest.docs_enabled = false;
        config.cors.allowed_origins = Vec::new(); // Must be configured
        config
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate server configuration
        if self.server.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }
        
        // Validate at least one API is enabled
        if !self.rest.enabled && !self.graphql.enabled {
            return Err("At least one API (REST or GraphQL) must be enabled".to_string());
        }
        
        // Validate CORS configuration
        if self.cors.enabled && self.cors.allowed_origins.is_empty() {
            return Err("CORS allowed origins cannot be empty when CORS is enabled".to_string());
        }
        
        // Validate rate limiting
        if let Some(rate_limit) = &self.rate_limit {
            if rate_limit.enabled && rate_limit.requests_per_minute == 0 {
                return Err("Rate limit requests per minute cannot be 0".to_string());
            }
        }
        
        // Validate auth configuration
        if let Some(auth) = &self.auth {
            match &auth.auth_type {
                AuthType::Jwt => {
                    if auth.jwt.is_none() {
                        return Err("JWT configuration required when JWT auth is enabled".to_string());
                    }
                }
                AuthType::ApiKey => {
                    if auth.api_key.is_none() {
                        return Err("API key configuration required when API key auth is enabled".to_string());
                    }
                }
                AuthType::OAuth => {
                    if auth.oauth.is_none() {
                        return Err("OAuth configuration required when OAuth auth is enabled".to_string());
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert!(config.validate().is_ok());
        assert!(config.rest.enabled);
        assert!(config.graphql.enabled);
        assert_eq!(config.server.port, 3000);
    }
    
    #[test]
    fn test_development_config() {
        let config = ApiConfig::development();
        assert!(config.validate().is_ok());
        assert!(config.graphql.playground_enabled);
        assert!(config.graphql.introspection_enabled);
        assert!(!config.security.https_only);
    }
    
    #[test]
    fn test_production_config() {
        let mut config = ApiConfig::production();
        config.cors.allowed_origins = vec!["https://example.com".to_string()];
        assert!(config.validate().is_ok());
        assert!(!config.graphql.playground_enabled);
        assert!(!config.graphql.introspection_enabled);
        assert!(config.security.https_only);
    }
    
    #[test]
    fn test_validation_errors() {
        let mut config = ApiConfig::default();
        
        // Test no APIs enabled
        config.rest.enabled = false;
        config.graphql.enabled = false;
        assert!(config.validate().is_err());
        
        // Test invalid port
        config.rest.enabled = true;
        config.server.port = 0;
        assert!(config.validate().is_err());
    }
}