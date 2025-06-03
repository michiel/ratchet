//! API Key authentication middleware

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::errors::ApiError;

/// API key configuration loaded from environment
static API_KEYS: Lazy<HashMap<String, ApiKeyInfo>> = Lazy::new(|| {
    // In production, load from database or configuration
    let mut keys = HashMap::new();
    
    // Example API keys - replace with proper configuration
    if let Ok(key) = std::env::var("RATCHET_API_KEY") {
        keys.insert(
            key.clone(),
            ApiKeyInfo {
                name: "default".to_string(),
                permissions: vec!["read".to_string(), "write".to_string()],
                rate_limit: Some(1000),
            },
        );
    }
    
    keys
});

/// API Key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// Name/description of the API key
    pub name: String,
    /// Permissions granted to this API key
    pub permissions: Vec<String>,
    /// Rate limit for this API key (requests per minute)
    pub rate_limit: Option<u32>,
}

/// Authenticated API key user
#[derive(Debug, Clone)]
pub struct ApiKeyUser {
    pub api_key: String,
    pub info: ApiKeyInfo,
}

impl ApiKeyUser {
    /// Check if API key has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.info.permissions.iter().any(|p| p == permission)
    }
    
    /// Check if API key has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        self.info.permissions.iter().any(|p| permissions.contains(&p.as_str()))
    }
    
    /// Check if API key has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|perm| self.has_permission(perm))
    }
}

/// API Key authentication extractor
pub struct ApiKeyAuth(pub ApiKeyUser);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for ApiKeyAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract API key from header
        let api_key_str = if let Some(header_value) = parts.headers.get("X-API-Key") {
            header_value.to_str().map(|s| s.to_string()).ok()
        } else if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
            // Fallback to Authorization header with ApiKey scheme
            auth_header.to_str().ok().and_then(|value| {
                if value.starts_with("ApiKey ") {
                    Some(value[7..].to_string())
                } else {
                    None
                }
            })
        } else if let Some(query) = parts.uri.query() {
            // Fallback to query parameter
            form_urlencoded::parse(query.as_bytes())
                .find(|(key, _)| key == "api_key")
                .map(|(_, value)| value.into_owned())
        } else {
            None
        }.ok_or_else(|| ApiError::unauthorized("Missing API key"))?;
        
        let api_key = api_key_str.as_str();

        // Validate API key
        let info = API_KEYS
            .get(api_key)
            .ok_or_else(|| ApiError::unauthorized("Invalid API key"))?
            .clone();

        Ok(ApiKeyAuth(ApiKeyUser {
            api_key: api_key_str,
            info,
        }))
    }
}

/// Optional API Key authentication extractor
pub struct OptionalApiKeyAuth(pub Option<ApiKeyUser>);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for OptionalApiKeyAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match ApiKeyAuth::from_request_parts(parts, state).await {
            Ok(ApiKeyAuth(user)) => Ok(OptionalApiKeyAuth(Some(user))),
            Err(_) => Ok(OptionalApiKeyAuth(None)),
        }
    }
}

/// Permission-based access control guard for API keys
pub struct RequirePermission(pub String);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for RequirePermission
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ApiKeyAuth(user) = ApiKeyAuth::from_request_parts(parts, state).await?;
        
        // Check if API key has the required permission
        let required_permission = parts
            .extensions
            .get::<String>()
            .ok_or_else(|| ApiError::internal("Required permission not specified"))?;
            
        if !user.has_permission(required_permission) {
            return Err(ApiError::forbidden(format!(
                "API key does not have required permission: {}",
                required_permission
            )));
        }

        Ok(RequirePermission(required_permission.clone()))
    }
}

/// Combined authentication extractor that accepts either JWT or API key
pub enum Auth {
    Jwt(super::auth::AuthUser),
    ApiKey(ApiKeyUser),
}

impl Auth {
    /// Get the user ID regardless of auth type
    pub fn user_id(&self) -> &str {
        match self {
            Auth::Jwt(user) => &user.user_id,
            Auth::ApiKey(user) => &user.info.name,
        }
    }
    
    /// Check if the authenticated entity has a specific permission/role
    pub fn has_permission(&self, permission: &str) -> bool {
        match self {
            Auth::Jwt(user) => user.has_role(permission),
            Auth::ApiKey(user) => user.has_permission(permission),
        }
    }
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Try JWT first
        if let Ok(super::auth::JwtAuth(user)) = super::auth::JwtAuth::from_request_parts(parts, state).await {
            return Ok(Auth::Jwt(user));
        }
        
        // Fall back to API key
        if let Ok(ApiKeyAuth(user)) = ApiKeyAuth::from_request_parts(parts, state).await {
            return Ok(Auth::ApiKey(user));
        }
        
        Err(ApiError::unauthorized("No valid authentication provided"))
    }
}

/// API key management functions
pub mod management {
    use super::*;
    
    /// Generate a new API key
    pub fn generate_api_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        
        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Validate API key format
    pub fn validate_api_key_format(key: &str) -> bool {
        key.len() >= 32 && key.chars().all(|c| c.is_alphanumeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let key = management::generate_api_key();
        assert_eq!(key.len(), 32);
        assert!(management::validate_api_key_format(&key));
    }

    #[test]
    fn test_api_key_validation() {
        assert!(management::validate_api_key_format("abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!management::validate_api_key_format("short"));
        assert!(!management::validate_api_key_format("invalid-characters-@#$"));
    }

    #[test]
    fn test_api_key_permissions() {
        let user = ApiKeyUser {
            api_key: "test-key".to_string(),
            info: ApiKeyInfo {
                name: "test".to_string(),
                permissions: vec!["read".to_string(), "write".to_string()],
                rate_limit: Some(100),
            },
        };
        
        assert!(user.has_permission("read"));
        assert!(user.has_permission("write"));
        assert!(!user.has_permission("delete"));
        
        assert!(user.has_any_permission(&["read", "delete"]));
        assert!(user.has_all_permissions(&["read", "write"]));
        assert!(!user.has_all_permissions(&["read", "write", "delete"]));
    }
}