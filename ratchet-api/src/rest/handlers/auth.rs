//! Authentication-related REST API handlers

use axum::{
    extract::Query,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ApiError, ApiResult},
    middleware::{AuthUser, JwtAuth, OptionalJwtAuth, ApiKeyAuth, Auth},
};

#[cfg(feature = "auth")]
use crate::middleware::auth::{Claims, LoginRequest, LoginResponse, generate_token};

/// Login endpoint for JWT authentication
#[cfg(feature = "auth")]
pub async fn login(
    Json(request): Json<LoginRequest>,
) -> ApiResult<(StatusCode, Json<LoginResponse>)> {
    // TODO: Implement actual user authentication with database
    // For now, this is a placeholder implementation
    
    // Hardcoded demo credentials (replace with real authentication)
    if request.username == "admin" && request.password == "password" {
        let claims = Claims::new(
            "admin_user_id",
            Some("admin@example.com".to_string()),
            vec!["admin".to_string(), "user".to_string()],
        ).with_expiry(86400); // 24 hours
        
        let token = generate_token(&claims)?;
        
        Ok((
            StatusCode::OK,
            Json(LoginResponse::new(token, 86400)),
        ))
    } else if request.username == "user" && request.password == "password" {
        let claims = Claims::new(
            "regular_user_id",
            Some("user@example.com".to_string()),
            vec!["user".to_string()],
        ).with_expiry(86400); // 24 hours
        
        let token = generate_token(&claims)?;
        
        Ok((
            StatusCode::OK,
            Json(LoginResponse::new(token, 86400)),
        ))
    } else {
        Err(ApiError::unauthorized("Invalid username or password"))
    }
}

/// Get current user information (requires JWT authentication)
#[cfg(feature = "auth")]
pub async fn me(
    JwtAuth(user): JwtAuth,
) -> ApiResult<Json<UserInfo>> {
    Ok(Json(UserInfo {
        user_id: user.user_id,
        email: user.email,
        roles: user.roles,
    }))
}

/// Get current user information (optional JWT authentication)
#[cfg(feature = "auth")]
pub async fn profile(
    OptionalJwtAuth(user): OptionalJwtAuth,
) -> ApiResult<Json<ProfileResponse>> {
    match user {
        Some(user) => Ok(Json(ProfileResponse {
            authenticated: true,
            user: Some(UserInfo {
                user_id: user.user_id,
                email: user.email,
                roles: user.roles,
            }),
        })),
        None => Ok(Json(ProfileResponse {
            authenticated: false,
            user: None,
        })),
    }
}

/// Admin-only endpoint (requires JWT with admin role)
#[cfg(feature = "auth")]
pub async fn admin_only(
    JwtAuth(user): JwtAuth,
) -> ApiResult<Json<serde_json::Value>> {
    if !user.has_role("admin") {
        return Err(ApiError::forbidden("Admin role required"));
    }
    
    Ok(Json(serde_json::json!({
        "message": "Welcome admin!",
        "user": user.user_id,
        "timestamp": chrono::Utc::now()
    })))
}

/// API key protected endpoint
#[cfg(feature = "auth")]
pub async fn api_key_protected(
    ApiKeyAuth(api_user): ApiKeyAuth,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "API key authentication successful",
        "api_key_name": api_user.info.name,
        "permissions": api_user.info.permissions,
        "timestamp": chrono::Utc::now()
    })))
}

/// Endpoint that accepts either JWT or API key authentication
#[cfg(feature = "auth")]
pub async fn flexible_auth(
    auth: Auth,
) -> ApiResult<Json<serde_json::Value>> {
    match auth {
        Auth::Jwt(user) => Ok(Json(serde_json::json!({
            "auth_type": "jwt",
            "user_id": user.user_id,
            "roles": user.roles,
            "timestamp": chrono::Utc::now()
        }))),
        Auth::ApiKey(api_user) => Ok(Json(serde_json::json!({
            "auth_type": "api_key",
            "api_key_name": api_user.info.name,
            "permissions": api_user.info.permissions,
            "timestamp": chrono::Utc::now()
        }))),
    }
}

/// User information response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
}

/// Profile response
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub authenticated: bool,
    pub user: Option<UserInfo>,
}

/// Public endpoint (no authentication required)
pub async fn public_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "This is a public endpoint",
        "timestamp": chrono::Utc::now(),
        "api_version": crate::API_VERSION
    }))
}

/// Health check with optional authentication info
pub async fn health_with_auth(
    OptionalJwtAuth(user): OptionalJwtAuth,
) -> Json<serde_json::Value> {
    let mut response = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "api_version": crate::API_VERSION
    });
    
    if let Some(user) = user {
        response["authenticated_user"] = serde_json::json!({
            "user_id": user.user_id,
            "roles": user.roles
        });
    }
    
    Json(response)
}

/// Fallback handlers for when auth feature is disabled
#[cfg(not(feature = "auth"))]
pub async fn login() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}

#[cfg(not(feature = "auth"))]
pub async fn me() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}

#[cfg(not(feature = "auth"))]
pub async fn profile() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}

#[cfg(not(feature = "auth"))]
pub async fn admin_only() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}

#[cfg(not(feature = "auth"))]
pub async fn api_key_protected() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}

#[cfg(not(feature = "auth"))]
pub async fn flexible_auth() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::not_implemented("Authentication feature is disabled"))
}