//! Authentication endpoints

use axum::{
    extract::{State, Extension},
    response::IntoResponse,
    Json,
};
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use ratchet_web::{
    middleware::{AuthContext, JwtManager},
    ApiResponse,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
};

/// Login request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// Username or email
    pub username: String,
    /// Password
    pub password: String,
    /// Remember me (extended session)
    pub remember_me: Option<bool>,
}

/// Login response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// JWT access token
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiry time (ISO 8601)
    pub expires_at: String,
    /// User information
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Display name
    pub display_name: Option<String>,
    /// Email address
    pub email: String,
    /// User role
    pub role: String,
    /// Whether email is verified
    pub email_verified: bool,
}

/// User registration request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    /// Username (must be unique)
    pub username: String,
    /// Email address (must be unique)
    pub email: String,
    /// Password (will be hashed)
    pub password: String,
    /// Display name
    pub display_name: Option<String>,
}

/// Password change request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// Current password
    pub current_password: String,
    /// New password
    pub new_password: String,
}

/// User login endpoint
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    operation_id = "login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn login(
    State(ctx): State<TasksContext>,
    Json(request): Json<LoginRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Login attempt for user: {}", request.username);

    // For demonstration purposes, implement a simple hardcoded user
    // In a real implementation, this would validate against the database
    if request.username == "admin" && request.password == "admin123" {
        // Create a JWT token
        let jwt_manager = JwtManager::new(Default::default()); // Use default config for now
        let session_id = Uuid::new_v4().to_string();
        
        let token = jwt_manager
            .generate_token("1", "admin", &session_id)
            .map_err(|e| RestError::InternalError(format!("Failed to generate token: {}", e)))?;

        let expires_at = Utc::now() + chrono::Duration::hours(24);

        let response = LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                id: "1".to_string(),
                username: "admin".to_string(),
                display_name: Some("Administrator".to_string()),
                email: "admin@example.com".to_string(),
                role: "admin".to_string(),
                email_verified: true,
            },
        };

        info!("Login successful for user: {}", request.username);
        Ok(Json(ApiResponse::new(response)))
    } else if request.username == "user" && request.password == "user123" {
        // Create a JWT token for regular user
        let jwt_manager = JwtManager::new(Default::default());
        let session_id = Uuid::new_v4().to_string();
        
        let token = jwt_manager
            .generate_token("2", "user", &session_id)
            .map_err(|e| RestError::InternalError(format!("Failed to generate token: {}", e)))?;

        let expires_at = Utc::now() + chrono::Duration::hours(24);

        let response = LoginResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_at: expires_at.to_rfc3339(),
            user: UserInfo {
                id: "2".to_string(),
                username: "user".to_string(),
                display_name: Some("Regular User".to_string()),
                email: "user@example.com".to_string(),
                role: "user".to_string(),
                email_verified: true,
            },
        };

        info!("Login successful for user: {}", request.username);
        Ok(Json(ApiResponse::new(response)))
    } else {
        warn!("Login failed for user: {}", request.username);
        Err(RestError::unauthorized("Invalid username or password"))
    }
}

/// User registration endpoint
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    operation_id = "register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully"),
        (status = 400, description = "Invalid request or user already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn register(
    State(_ctx): State<TasksContext>,
    Json(request): Json<RegisterRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Registration attempt for user: {}", request.username);

    // Basic validation
    if request.username.len() < 3 {
        return Err(RestError::BadRequest(
            "Username must be at least 3 characters".to_string(),
        ));
    }

    if request.password.len() < 6 {
        return Err(RestError::BadRequest(
            "Password must be at least 6 characters".to_string(),
        ));
    }

    if !request.email.contains('@') {
        return Err(RestError::BadRequest(
            "Invalid email address".to_string(),
        ));
    }

    // Hash the password
    let _password_hash = hash(request.password.as_bytes(), DEFAULT_COST)
        .map_err(|e| RestError::InternalError(format!("Failed to hash password: {}", e)))?;

    // For demonstration purposes, just return success
    // In a real implementation, this would:
    // 1. Check if username/email already exists
    // 2. Create user in database
    // 3. Send verification email
    // 4. Return user info (without password)

    let user_info = UserInfo {
        id: Uuid::new_v4().to_string(),
        username: request.username.clone(),
        display_name: request.display_name,
        email: request.email,
        role: "user".to_string(),
        email_verified: false,
    };

    info!("Registration successful for user: {}", request.username);
    Ok(Json(ApiResponse::new(serde_json::json!({
        "message": "User registered successfully",
        "user": user_info
    }))))
}

/// Get current user info
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    operation_id = "getCurrentUser",
    responses(
        (status = 200, description = "Current user information"),
        (status = 401, description = "Not authenticated"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_current_user(
    State(_ctx): State<TasksContext>,
    Extension(auth_context): Extension<AuthContext>,
) -> RestResult<impl IntoResponse> {
    if !auth_context.is_authenticated {
        return Err(RestError::unauthorized("Authentication required"));
    }

    // In a real implementation, this would fetch user details from database
    let user_info = match auth_context.user_id.as_str() {
        "1" => UserInfo {
            id: "1".to_string(),
            username: "admin".to_string(),
            display_name: Some("Administrator".to_string()),
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
            email_verified: true,
        },
        "2" => UserInfo {
            id: "2".to_string(),
            username: "user".to_string(),
            display_name: Some("Regular User".to_string()),
            email: "user@example.com".to_string(),
            role: "user".to_string(),
            email_verified: true,
        },
        _ => UserInfo {
            id: auth_context.user_id.clone(),
            username: "unknown".to_string(),
            display_name: None,
            email: "unknown@example.com".to_string(),
            role: auth_context.role.clone(),
            email_verified: false,
        },
    };

    Ok(Json(ApiResponse::new(user_info)))
}

/// Logout endpoint
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    operation_id = "logout",
    responses(
        (status = 200, description = "Logout successful"),
        (status = 401, description = "Not authenticated"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn logout(
    State(_ctx): State<TasksContext>,
    Extension(auth_context): Extension<AuthContext>,
) -> RestResult<impl IntoResponse> {
    if !auth_context.is_authenticated {
        return Err(RestError::unauthorized("Authentication required"));
    }

    // In a real implementation, this would:
    // 1. Invalidate the session in the database
    // 2. Add JWT to a blacklist
    // 3. Clear any server-side session data

    info!("User logged out: {}", auth_context.user_id);

    Ok(Json(ApiResponse::new(serde_json::json!({
        "message": "Logged out successfully"
    }))))
}

/// Change password endpoint
#[utoipa::path(
    post,
    path = "/auth/change-password",
    tag = "auth",
    operation_id = "changePassword",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully"),
        (status = 400, description = "Invalid current password"),
        (status = 401, description = "Not authenticated"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn change_password(
    State(_ctx): State<TasksContext>,
    Extension(auth_context): Extension<AuthContext>,
    Json(request): Json<ChangePasswordRequest>,
) -> RestResult<impl IntoResponse> {
    if !auth_context.is_authenticated {
        return Err(RestError::unauthorized("Authentication required"));
    }

    // Basic validation
    if request.new_password.len() < 6 {
        return Err(RestError::BadRequest(
            "New password must be at least 6 characters".to_string(),
        ));
    }

    // In a real implementation, this would:
    // 1. Fetch current password hash from database
    // 2. Verify current password
    // 3. Hash new password
    // 4. Update password in database
    // 5. Invalidate existing sessions

    // For demonstration, just check against hardcoded passwords
    let current_valid = match auth_context.user_id.as_str() {
        "1" => request.current_password == "admin123",
        "2" => request.current_password == "user123",
        _ => false,
    };

    if !current_valid {
        warn!("Invalid current password for user: {}", auth_context.user_id);
        return Err(RestError::BadRequest(
            "Current password is incorrect".to_string(),
        ));
    }

    // Hash the new password
    let _new_password_hash = hash(request.new_password.as_bytes(), DEFAULT_COST)
        .map_err(|e| RestError::InternalError(format!("Failed to hash password: {}", e)))?;

    info!("Password changed for user: {}", auth_context.user_id);

    Ok(Json(ApiResponse::new(serde_json::json!({
        "message": "Password changed successfully"
    }))))
}