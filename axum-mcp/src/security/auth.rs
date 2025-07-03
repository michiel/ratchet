//! Authentication management for MCP connections

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::permissions::ClientPermissions;
use crate::error::{McpError, McpResult};

/// Authentication result
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication error
#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Authentication method not supported: {method}")]
    UnsupportedMethod { method: String },

    #[error("Token expired")]
    TokenExpired,

    #[error("Client not found: {client_id}")]
    ClientNotFound { client_id: String },

    #[error("Authentication required")]
    AuthenticationRequired,
}

impl From<AuthError> for McpError {
    fn from(err: AuthError) -> Self {
        McpError::Authentication {
            message: err.to_string(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Default)]
pub enum McpAuthConfig {
    /// API key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        /// List of valid API keys with their associated client info
        keys: HashMap<String, ApiKeyInfo>,
    },

    /// OAuth2 token authentication
    #[serde(rename = "oauth2")]
    OAuth2 {
        /// OAuth2 configuration
        issuer: String,
        audience: String,
        jwks_uri: String,
    },

    /// Certificate-based authentication
    #[serde(rename = "certificate")]
    Certificate {
        /// CA certificate for validation
        ca_cert: String,
        /// Whether to require client certificates
        require_client_cert: bool,
    },

    /// No authentication (insecure - for development only)
    #[serde(rename = "none")]
    #[default]
    None,
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// Human-readable name for this key
    pub name: String,

    /// Client permissions
    pub permissions: ClientPermissions,

    /// When this key was created
    pub created_at: DateTime<Utc>,

    /// When this key expires (if any)
    pub expires_at: Option<DateTime<Utc>>,

    /// Whether this key is currently active
    pub active: bool,
}

/// Client authentication context
#[derive(Debug, Clone)]
pub struct ClientContext {
    /// User agent string
    pub user_agent: String,
    
    /// Unique client identifier
    pub client_id: Option<String>,

    /// Session identifier
    pub session_id: Option<String>,
    
    /// Additional client metadata
    pub metadata: HashMap<String, String>,
}

impl Default for ClientContext {
    fn default() -> Self {
        Self {
            user_agent: "unknown".to_string(),
            client_id: None,
            session_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Security context for authenticated requests
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Client information
    pub client: ClientContext,
    
    /// Client permissions
    pub permissions: ClientPermissions,
    
    /// Authentication status
    pub authenticated: bool,
    
    /// When the client was authenticated
    pub authenticated_at: Option<DateTime<Utc>>,
    
    /// Additional capabilities
    pub capabilities: Vec<String>,
}

impl SecurityContext {
    /// Create a system security context (full permissions)
    pub fn system() -> Self {
        Self {
            client: ClientContext::default(),
            permissions: ClientPermissions::admin(),
            authenticated: true,
            authenticated_at: Some(Utc::now()),
            capabilities: vec!["system".to_string(), "initialized".to_string()],
        }
    }
    
    /// Create an anonymous security context (limited permissions)
    pub fn anonymous() -> Self {
        Self {
            client: ClientContext::default(),
            permissions: ClientPermissions::default(),
            authenticated: false,
            authenticated_at: None,
            capabilities: Vec::new(),
        }
    }
    
    /// Create an authenticated security context
    pub fn authenticated(client: ClientContext, capabilities: Vec<String>) -> Self {
        Self {
            client,
            permissions: ClientPermissions::default(),
            authenticated: true,
            authenticated_at: Some(Utc::now()),
            capabilities,
        }
    }
    
    /// Check if the context is anonymous
    pub fn is_anonymous(&self) -> bool {
        !self.authenticated
    }
    
    /// Check if the context is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
    
    /// Check if the context is a system context
    pub fn is_system(&self) -> bool {
        self.capabilities.contains(&"system".to_string())
    }
    
    /// Check if the context has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }
    
    /// Add a capability
    pub fn add_capability(&mut self, capability: impl Into<String>) {
        let cap = capability.into();
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }
}

/// MCP authentication trait
#[async_trait]
pub trait McpAuth: Send + Sync {
    /// Authenticate a client and return security context
    async fn authenticate(&self, client_info: &ClientContext) -> McpResult<SecurityContext>;
    
    /// Authorize an action for a security context
    async fn authorize(&self, context: &SecurityContext, resource: &str, action: &str) -> bool;
}

/// MCP authentication manager
pub struct McpAuthManager {
    /// Authentication configuration
    config: McpAuthConfig,

    /// Active sessions
    sessions: tokio::sync::RwLock<HashMap<String, SecurityContext>>,
}

impl McpAuthManager {
    /// Create a new authentication manager
    pub fn new(config: McpAuthConfig) -> Self {
        Self {
            config,
            sessions: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Authenticate a client request
    pub async fn authenticate(&self, auth_header: Option<&str>) -> AuthResult<SecurityContext> {
        match &self.config {
            McpAuthConfig::ApiKey { keys } => {
                let api_key = self.extract_api_key(auth_header)?;
                self.authenticate_api_key(&api_key, keys).await
            }
            McpAuthConfig::OAuth2 { .. } => {
                // OAuth2 implementation would go here
                Err(AuthError::UnsupportedMethod {
                    method: "oauth2".to_string(),
                })
            }
            McpAuthConfig::Certificate { .. } => {
                // Certificate authentication would go here
                Err(AuthError::UnsupportedMethod {
                    method: "certificate".to_string(),
                })
            }
            McpAuthConfig::None => {
                // No authentication - create anonymous context
                Ok(SecurityContext::anonymous())
            }
        }
    }

    /// Extract API key from authorization header
    fn extract_api_key(&self, auth_header: Option<&str>) -> AuthResult<String> {
        let header = auth_header.ok_or(AuthError::AuthenticationRequired)?;

        if let Some(key) = header.strip_prefix("Bearer ") {
            Ok(key.to_string())
        } else if let Some(key) = header.strip_prefix("ApiKey ") {
            Ok(key.to_string())
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Authenticate using API key
    async fn authenticate_api_key(
        &self,
        api_key: &str,
        keys: &HashMap<String, ApiKeyInfo>,
    ) -> AuthResult<SecurityContext> {
        let key_info = keys.get(api_key).ok_or(AuthError::InvalidCredentials)?;

        // Check if key is active
        if !key_info.active {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if key is expired
        if let Some(expires_at) = key_info.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::TokenExpired);
            }
        }

        // Create client context
        let client = ClientContext {
            user_agent: "api-key-client".to_string(),
            client_id: Some(key_info.name.clone()),
            session_id: Some(uuid::Uuid::new_v4().to_string()),
            metadata: HashMap::new(),
        };

        let security_context = SecurityContext {
            client,
            permissions: key_info.permissions.clone(),
            authenticated: true,
            authenticated_at: Some(Utc::now()),
            capabilities: vec!["authenticated".to_string(), "initialized".to_string()],
        };

        // Store session
        let mut sessions = self.sessions.write().await;
        if let Some(session_id) = &security_context.client.session_id {
            sessions.insert(session_id.clone(), security_context.clone());
        }

        Ok(security_context)
    }

    /// Get security context for a session
    pub async fn get_session(&self, session_id: &str) -> Option<SecurityContext> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }

    /// List active sessions
    pub async fn list_sessions(&self) -> Vec<ClientContext> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|ctx| ctx.client.clone()).collect()
    }

    /// Clean up expired sessions
    pub async fn cleanup_sessions(&self, max_age: chrono::Duration) {
        let mut sessions = self.sessions.write().await;
        let cutoff = Utc::now() - max_age;

        sessions.retain(|_, context| {
            context.authenticated_at.map_or(false, |auth_time| auth_time > cutoff)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::permissions::{RateLimits, ResourceQuotas};

    fn create_test_api_key_config() -> McpAuthConfig {
        let mut keys = HashMap::new();
        keys.insert(
            "test-key-123".to_string(),
            ApiKeyInfo {
                name: "Test Client".to_string(),
                permissions: ClientPermissions {
                    can_execute_tasks: true,
                    can_read_logs: true,
                    can_read_traces: false,
                    allowed_task_patterns: vec!["test-*".to_string()],
                    rate_limits: RateLimits::default(),
                    resource_quotas: ResourceQuotas::default(),
                },
                created_at: Utc::now(),
                expires_at: None,
                active: true,
            },
        );

        McpAuthConfig::ApiKey { keys }
    }

    #[tokio::test]
    async fn test_api_key_authentication() {
        let config = create_test_api_key_config();
        let auth_manager = McpAuthManager::new(config);

        // Test valid API key
        let result = auth_manager.authenticate(Some("Bearer test-key-123")).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.client.client_id.as_ref().unwrap(), "Test Client");
        assert!(context.permissions.can_execute_tasks);

        // Test invalid API key
        let result = auth_manager.authenticate(Some("Bearer invalid-key")).await;
        assert!(result.is_err());

        // Test missing auth header
        let result = auth_manager.authenticate(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_management() {
        let config = create_test_api_key_config();
        let auth_manager = McpAuthManager::new(config);

        // Authenticate and create session
        let context = auth_manager.authenticate(Some("Bearer test-key-123")).await.unwrap();
        let session_id = context.client.session_id.as_ref().unwrap();

        // Get session
        let session = auth_manager.get_session(session_id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().client.session_id.as_ref().unwrap(), session_id);

        // Remove session
        auth_manager.remove_session(session_id).await;
        let session = auth_manager.get_session(session_id).await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_no_auth() {
        let auth_manager = McpAuthManager::new(McpAuthConfig::None);

        let result = auth_manager.authenticate(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert!(!context.is_authenticated());
    }
}
