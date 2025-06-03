//! Authentication management for MCP connections

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::McpError;
use super::permissions::ClientPermissions;

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
        McpError::AuthenticationFailed { reason: err.to_string() }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Default)]
pub enum McpAuth {
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
    /// Unique client identifier
    pub id: String,
    
    /// Human-readable client name
    pub name: String,
    
    /// Client permissions
    pub permissions: ClientPermissions,
    
    /// When the client was authenticated
    pub authenticated_at: DateTime<Utc>,
    
    /// Session identifier
    pub session_id: String,
}

/// MCP authentication manager
pub struct McpAuthManager {
    /// Authentication configuration
    config: McpAuth,
    
    /// Active sessions
    sessions: tokio::sync::RwLock<HashMap<String, ClientContext>>,
}

impl McpAuthManager {
    /// Create a new authentication manager
    pub fn new(config: McpAuth) -> Self {
        Self {
            config,
            sessions: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
    
    /// Authenticate a client request
    pub async fn authenticate(&self, auth_header: Option<&str>) -> AuthResult<ClientContext> {
        match &self.config {
            McpAuth::ApiKey { keys } => {
                let api_key = self.extract_api_key(auth_header)?;
                self.authenticate_api_key(&api_key, keys).await
            }
            McpAuth::OAuth2 { .. } => {
                // OAuth2 implementation would go here
                Err(AuthError::UnsupportedMethod {
                    method: "oauth2".to_string(),
                })
            }
            McpAuth::Certificate { .. } => {
                // Certificate authentication would go here
                Err(AuthError::UnsupportedMethod {
                    method: "certificate".to_string(),
                })
            }
            McpAuth::None => {
                // No authentication - create anonymous client
                Ok(ClientContext {
                    id: "anonymous".to_string(),
                    name: "Anonymous Client".to_string(),
                    permissions: ClientPermissions::default(),
                    authenticated_at: Utc::now(),
                    session_id: uuid::Uuid::new_v4().to_string(),
                })
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
    ) -> AuthResult<ClientContext> {
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
        let client_context = ClientContext {
            id: format!("api_key:{}", &api_key[..8]), // Use first 8 chars as ID
            name: key_info.name.clone(),
            permissions: key_info.permissions.clone(),
            authenticated_at: Utc::now(),
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        
        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(client_context.session_id.clone(), client_context.clone());
        
        Ok(client_context)
    }
    
    /// Get client context for a session
    pub async fn get_session(&self, session_id: &str) -> Option<ClientContext> {
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
        sessions.values().cloned().collect()
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_sessions(&self, max_age: chrono::Duration) {
        let mut sessions = self.sessions.write().await;
        let cutoff = Utc::now() - max_age;
        
        sessions.retain(|_, context| context.authenticated_at > cutoff);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::permissions::{RateLimits, ResourceQuotas};

    fn create_test_api_key_config() -> McpAuth {
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
        
        McpAuth::ApiKey { keys }
    }

    #[tokio::test]
    async fn test_api_key_authentication() {
        let config = create_test_api_key_config();
        let auth_manager = McpAuthManager::new(config);
        
        // Test valid API key
        let result = auth_manager
            .authenticate(Some("Bearer test-key-123"))
            .await;
        assert!(result.is_ok());
        
        let client = result.unwrap();
        assert_eq!(client.name, "Test Client");
        assert!(client.permissions.can_execute_tasks);
        
        // Test invalid API key
        let result = auth_manager
            .authenticate(Some("Bearer invalid-key"))
            .await;
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
        let client = auth_manager
            .authenticate(Some("Bearer test-key-123"))
            .await
            .unwrap();
        
        // Get session
        let session = auth_manager.get_session(&client.session_id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, client.id);
        
        // Remove session
        auth_manager.remove_session(&client.session_id).await;
        let session = auth_manager.get_session(&client.session_id).await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_no_auth() {
        let auth_manager = McpAuthManager::new(McpAuth::None);
        
        let result = auth_manager.authenticate(None).await;
        assert!(result.is_ok());
        
        let client = result.unwrap();
        assert_eq!(client.id, "anonymous");
    }
}