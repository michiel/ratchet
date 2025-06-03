//! JWT authentication middleware

use axum::{
    extract::{FromRequestParts, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::ApiError;

/// JWT secret key (should be loaded from environment in production)
static JWT_SECRET: Lazy<String> = Lazy::new(|| {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-change-in-production".to_string())
});

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (as UTC timestamp)
    pub exp: usize,
    /// Issued at (as UTC timestamp)
    pub iat: usize,
    /// Not before (as UTC timestamp)
    pub nbf: usize,
    /// User email
    pub email: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// Additional custom claims
    #[serde(flatten)]
    pub custom: serde_json::Map<String, serde_json::Value>,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: impl Into<String>, email: Option<String>, roles: Vec<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        
        Self {
            sub: user_id.into(),
            exp: now + 86400, // 24 hours
            iat: now,
            nbf: now,
            email,
            roles,
            custom: serde_json::Map::new(),
        }
    }
    
    /// Add a custom claim
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
    
    /// Set expiration time in seconds from now
    pub fn with_expiry(mut self, seconds: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        self.exp = now + seconds;
        self
    }
}

/// Authenticated user information
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub claims: Claims,
}

impl AuthUser {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
    
    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        self.roles.iter().any(|r| roles.contains(&r.as_str()))
    }
    
    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// JWT authentication extractor
pub struct JwtAuth(pub AuthUser);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for JwtAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, _state)
            .await
            .map_err(|_| ApiError::unauthorized("Missing authorization header"))?;

        // Decode and validate the token
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| ApiError::unauthorized(format!("Invalid token: {}", e)))?;

        let claims = token_data.claims;
        
        // Create AuthUser from claims
        let auth_user = AuthUser {
            user_id: claims.sub.clone(),
            email: claims.email.clone(),
            roles: claims.roles.clone(),
            claims,
        };

        Ok(JwtAuth(auth_user))
    }
}

/// Optional JWT authentication extractor
pub struct OptionalJwtAuth(pub Option<AuthUser>);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for OptionalJwtAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match JwtAuth::from_request_parts(parts, state).await {
            Ok(JwtAuth(user)) => Ok(OptionalJwtAuth(Some(user))),
            Err(_) => Ok(OptionalJwtAuth(None)),
        }
    }
}

/// Generate a JWT token for the given claims
pub fn generate_token(claims: &Claims) -> Result<String, ApiError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .map_err(|e| ApiError::internal(format!("Failed to generate token: {}", e)))
}

/// Verify a JWT token and return the claims
pub fn verify_token(token: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| ApiError::unauthorized(format!("Invalid token: {}", e)))
}

/// Role-based access control guard
pub struct RequireRole(pub String);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for RequireRole
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let JwtAuth(user) = JwtAuth::from_request_parts(parts, state).await?;
        
        // Check if user has the required role
        let required_role = parts
            .extensions
            .get::<String>()
            .ok_or_else(|| ApiError::internal("Required role not specified"))?;
            
        if !user.has_role(required_role) {
            return Err(ApiError::forbidden(format!(
                "User does not have required role: {}",
                required_role
            )));
        }

        Ok(RequireRole(required_role.clone()))
    }
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: usize,
    pub token_type: String,
}

impl LoginResponse {
    /// Create a new login response
    pub fn new(token: String, expires_in: usize) -> Self {
        Self {
            token,
            expires_in,
            token_type: "Bearer".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("user123", Some("user@example.com".to_string()), vec!["user".to_string()]);
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.roles, vec!["user"]);
    }

    #[test]
    fn test_token_generation_and_verification() {
        let claims = Claims::new("user123", None, vec!["admin".to_string()])
            .with_expiry(3600);
        
        let token = generate_token(&claims).unwrap();
        let verified_claims = verify_token(&token).unwrap();
        
        assert_eq!(verified_claims.sub, "user123");
        assert_eq!(verified_claims.roles, vec!["admin"]);
    }

    #[test]
    fn test_auth_user_roles() {
        let user = AuthUser {
            user_id: "user123".to_string(),
            email: None,
            roles: vec!["user".to_string(), "admin".to_string()],
            claims: Claims::new("user123", None, vec!["user".to_string(), "admin".to_string()]),
        };
        
        assert!(user.has_role("admin"));
        assert!(user.has_role("user"));
        assert!(!user.has_role("super_admin"));
        
        assert!(user.has_any_role(&["admin", "super_admin"]));
        assert!(user.has_all_roles(&["user", "admin"]));
        assert!(!user.has_all_roles(&["user", "admin", "super_admin"]));
    }
}