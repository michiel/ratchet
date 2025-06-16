//! JWT Authentication middleware

use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::errors::WebError;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// User role
    pub role: String,
    /// Session ID for revocation
    pub jti: String,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT secret for signing/verification
    pub jwt_secret: String,
    /// JWT issuer
    pub jwt_issuer: String,
    /// JWT audience
    pub jwt_audience: String,
    /// Token expiration duration (in hours)
    pub token_expiry_hours: i64,
    /// Whether to require authentication
    pub require_auth: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-secret-key".to_string(),
            jwt_issuer: "ratchet-api".to_string(),
            jwt_audience: "ratchet-clients".to_string(),
            token_expiry_hours: 24,
            require_auth: false, // Default to disabled for development
        }
    }
}

/// Authentication context for the current request
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User ID
    pub user_id: String,
    /// User role
    pub role: String,
    /// Session ID
    pub session_id: String,
    /// Whether this is an authenticated request
    pub is_authenticated: bool,
}

impl Default for AuthContext {
    fn default() -> Self {
        Self {
            user_id: "anonymous".to_string(),
            role: "guest".to_string(),
            session_id: "none".to_string(),
            is_authenticated: false,
        }
    }
}

impl AuthContext {
    /// Create an authenticated context
    pub fn authenticated(user_id: String, role: String, session_id: String) -> Self {
        Self {
            user_id,
            role,
            session_id,
            is_authenticated: true,
        }
    }

    /// Check if user can perform admin operations
    pub fn can_admin(&self) -> bool {
        self.is_authenticated && self.role == "admin"
    }

    /// Check if user can write/modify resources
    pub fn can_write(&self) -> bool {
        self.is_authenticated && matches!(self.role.as_str(), "admin" | "user" | "service")
    }

    /// Check if user can read resources
    pub fn can_read(&self) -> bool {
        // Unauthenticated requests can read if auth is not required
        true
    }

    /// Check if user can execute tasks
    pub fn can_execute_tasks(&self) -> bool {
        self.is_authenticated && matches!(self.role.as_str(), "admin" | "user" | "service")
    }
}

/// JWT token manager
pub struct JwtManager {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(config: AuthConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_ref());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_ref());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user_id: &str, role: &str, session_id: &str) -> Result<String, WebError> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.config.token_expiry_hours);

        let claims = JwtClaims {
            sub: user_id.to_string(),
            role: role.to_string(),
            jti: session_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.config.jwt_issuer.clone(),
            aud: self.config.jwt_audience.clone(),
        };

        let header = Header::new(Algorithm::HS256);
        
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| WebError::internal(format!("Failed to generate JWT token: {}", e)))
    }

    /// Verify and decode a JWT token
    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, WebError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.config.jwt_issuer]);
        validation.set_audience(&[&self.config.jwt_audience]);

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| {
                warn!("JWT verification failed: {}", e);
                WebError::unauthorized("Invalid or expired token")
            })?;

        // Check if token is expired
        let now = Utc::now().timestamp();
        if token_data.claims.exp < now {
            warn!("JWT token expired");
            return Err(WebError::unauthorized("Token has expired"));
        }

        Ok(token_data.claims)
    }

    /// Extract token from Authorization header
    fn extract_token(&self, headers: &HeaderMap) -> Option<String> {
        let auth_header = headers.get("Authorization")?.to_str().ok()?;
        
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            Some(token.to_string())
        } else {
            None
        }
    }

    /// Extract API key from headers
    fn extract_api_key(&self, headers: &HeaderMap) -> Option<String> {
        // Try X-API-Key header first
        if let Some(api_key) = headers.get("X-API-Key").and_then(|h| h.to_str().ok()) {
            return Some(api_key.to_string());
        }

        // Try Authorization header with ApiKey scheme
        if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
            if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
                return Some(api_key.to_string());
            }
        }

        None
    }

    /// Authenticate a request
    pub async fn authenticate(&self, headers: &HeaderMap) -> Result<AuthContext, WebError> {
        // If authentication is not required, allow anonymous access
        if !self.config.require_auth {
            return Ok(AuthContext::default());
        }

        // Try JWT authentication first
        if let Some(token) = self.extract_token(headers) {
            match self.verify_token(&token) {
                Ok(claims) => {
                    debug!("JWT authentication successful for user: {}", claims.sub);
                    return Ok(AuthContext::authenticated(
                        claims.sub,
                        claims.role,
                        claims.jti,
                    ));
                }
                Err(e) => {
                    warn!("JWT authentication failed: {}", e);
                }
            }
        }

        // Try API key authentication
        if let Some(api_key) = self.extract_api_key(headers) {
            // TODO: Implement API key validation against database
            // For now, just check for a demo API key
            if api_key == "demo-api-key" {
                debug!("API key authentication successful");
                return Ok(AuthContext::authenticated(
                    "api-user".to_string(),
                    "service".to_string(),
                    uuid::Uuid::new_v4().to_string(),
                ));
            }
        }

        // No valid authentication found
        Err(WebError::unauthorized("Authentication required"))
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, WebError> {
    // Extract JWT manager from request extensions
    let jwt_manager = request
        .extensions()
        .get::<Arc<JwtManager>>()
        .ok_or_else(|| WebError::internal("JWT manager not configured"))?;

    // Authenticate the request
    let auth_context = jwt_manager.authenticate(&headers).await?;

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context);

    // Continue with the request
    Ok(next.run(request).await)
}

/// Optional authentication middleware (doesn't fail on missing auth)
pub async fn optional_auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract JWT manager from request extensions
    if let Some(jwt_manager) = request.extensions().get::<Arc<JwtManager>>() {
        // Try to authenticate, but don't fail if it doesn't work
        let auth_context = jwt_manager.authenticate(&headers).await
            .unwrap_or_default();

        // Add auth context to request extensions
        request.extensions_mut().insert(auth_context);
    }

    // Continue with the request regardless
    next.run(request).await
}

/// Create authentication layer with configuration
pub fn auth_layer(_config: AuthConfig) -> tower::layer::util::Identity {
    // For now, return identity layer - full implementation requires proper state management
    // Real implementation would need to add JwtManager to request extensions
    tower::layer::util::Identity::new()
}

/// Require authentication (to be used as a guard/extractor)
pub fn require_auth() -> impl Fn(&AuthContext) -> Result<(), WebError> {
    |auth_context: &AuthContext| {
        if auth_context.is_authenticated {
            Ok(())
        } else {
            Err(WebError::unauthorized("Authentication required"))
        }
    }
}

/// Require admin permissions
pub fn require_admin() -> impl Fn(&AuthContext) -> Result<(), WebError> {
    |auth_context: &AuthContext| {
        if auth_context.can_admin() {
            Ok(())
        } else {
            Err(WebError::forbidden("Admin privileges required"))
        }
    }
}

/// Require write permissions
pub fn require_write() -> impl Fn(&AuthContext) -> Result<(), WebError> {
    |auth_context: &AuthContext| {
        if auth_context.can_write() {
            Ok(())
        } else {
            Err(WebError::forbidden("Write privileges required"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn create_test_config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "test-secret-key".to_string(),
            jwt_issuer: "test-issuer".to_string(),
            jwt_audience: "test-audience".to_string(),
            token_expiry_hours: 1,
            require_auth: true,
        }
    }

    #[test]
    fn test_jwt_token_generation_and_verification() {
        let config = create_test_config();
        let jwt_manager = JwtManager::new(config);

        // Generate a token
        let token = jwt_manager
            .generate_token("user123", "admin", "session456")
            .unwrap();

        // Verify the token
        let claims = jwt_manager.verify_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.jti, "session456");
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
    }

    #[test]
    fn test_auth_context_permissions() {
        let admin_context = AuthContext::authenticated(
            "admin123".to_string(),
            "admin".to_string(),
            "session123".to_string(),
        );

        assert!(admin_context.can_admin());
        assert!(admin_context.can_write());
        assert!(admin_context.can_read());
        assert!(admin_context.can_execute_tasks());

        let user_context = AuthContext::authenticated(
            "user123".to_string(),
            "user".to_string(),
            "session456".to_string(),
        );

        assert!(!user_context.can_admin());
        assert!(user_context.can_write());
        assert!(user_context.can_read());
        assert!(user_context.can_execute_tasks());

        let anonymous_context = AuthContext::default();

        assert!(!anonymous_context.can_admin());
        assert!(!anonymous_context.can_write());
        assert!(anonymous_context.can_read());
        assert!(!anonymous_context.can_execute_tasks());
    }

    #[tokio::test]
    async fn test_jwt_authentication() {
        let config = create_test_config();
        let jwt_manager = JwtManager::new(config);

        // Create headers with valid JWT
        let token = jwt_manager
            .generate_token("user123", "admin", "session456")
            .unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );

        // Authenticate should succeed
        let auth_context = jwt_manager.authenticate(&headers).await.unwrap();
        assert!(auth_context.is_authenticated);
        assert_eq!(auth_context.user_id, "user123");
        assert_eq!(auth_context.role, "admin");
    }

    #[tokio::test]
    async fn test_api_key_authentication() {
        let config = create_test_config();
        let jwt_manager = JwtManager::new(config);

        // Create headers with demo API key
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", HeaderValue::from_str("demo-api-key").unwrap());

        // Authenticate should succeed
        let auth_context = jwt_manager.authenticate(&headers).await.unwrap();
        assert!(auth_context.is_authenticated);
        assert_eq!(auth_context.user_id, "api-user");
        assert_eq!(auth_context.role, "service");
    }

    #[tokio::test]
    async fn test_no_authentication() {
        let mut config = create_test_config();
        config.require_auth = false;
        let jwt_manager = JwtManager::new(config);

        // Empty headers
        let headers = HeaderMap::new();

        // Should succeed with anonymous context
        let auth_context = jwt_manager.authenticate(&headers).await.unwrap();
        assert!(!auth_context.is_authenticated);
        assert_eq!(auth_context.user_id, "anonymous");
    }
}