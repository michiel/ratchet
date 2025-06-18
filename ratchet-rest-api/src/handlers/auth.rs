//! Authentication endpoints

use axum::{
    extract::{State, Extension},
    response::IntoResponse,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Utc, Duration};
use ratchet_web::{
    middleware::{AuthContext, JwtManager},
    ApiResponse,
};
use ratchet_api_types::ApiId;
// Removed unused trait imports - using repositories via context
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
};

/// Login request
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Serialize, ToSchema)]
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
#[derive(Debug, Serialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
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

    // Get repositories
    let user_repo = ctx.repositories.user_repository();
    let session_repo = ctx.repositories.session_repository();

    // Find user by username or email
    let user = match user_repo.find_by_username(&request.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Try by email if username lookup failed
            match user_repo.find_by_email(&request.username).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    warn!("Login failed: user not found: {}", request.username);
                    return Err(RestError::unauthorized("Invalid username or password"));
                }
                Err(e) => {
                    error!("Database error during email lookup: {}", e);
                    return Err(RestError::InternalError("Authentication service unavailable".to_string()));
                }
            }
        }
        Err(e) => {
            error!("Database error during username lookup: {}", e);
            return Err(RestError::InternalError("Authentication service unavailable".to_string()));
        }
    };

    // Check if user is active
    if !user.is_active {
        warn!("Login failed: user account disabled: {}", request.username);
        return Err(RestError::unauthorized("Account is disabled"));
    }

    // For now, fall back to hardcoded password check since we need to implement proper password storage
    // TODO: Replace with actual password verification once password field is available in UnifiedUser
    let password_valid = match user.username.as_str() {
        "admin" => request.password == "admin123",
        "user" => request.password == "user123",
        _ => {
            // For demonstration, accept any password for other users
            // In production, this should verify against stored password hash
            true
        }
    };

    if !password_valid {
        warn!("Login failed: invalid password for user: {}", request.username);
        return Err(RestError::unauthorized("Invalid username or password"));
    }

    // Create session
    let session_id = Uuid::new_v4().to_string();
    let jwt_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(if request.remember_me.unwrap_or(false) { 168 } else { 24 }); // 7 days or 24 hours

    match session_repo.create_session(user.id.clone(), &session_id, &jwt_id, expires_at).await {
        Ok(_session) => {
            // Update user's last login timestamp
            if let Err(e) = user_repo.update_last_login(user.id.clone()).await {
                warn!("Failed to update last login for user {}: {}", user.id, e);
            }
        }
        Err(e) => {
            error!("Failed to create session for user {}: {}", user.id, e);
            return Err(RestError::InternalError("Failed to create session".to_string()));
        }
    }

    // Create JWT token
    let jwt_manager = JwtManager::new(Default::default()); // Use default config for now
    let role_str = match user.role {
        ratchet_api_types::UserRole::Admin => "admin",
        ratchet_api_types::UserRole::User => "user",
        ratchet_api_types::UserRole::ReadOnly => "readonly",
        ratchet_api_types::UserRole::Service => "service",
    };

    let token = jwt_manager
        .generate_token(&user.id.to_string(), role_str, &jwt_id)
        .map_err(|e| RestError::InternalError(format!("Failed to generate token: {}", e)))?;

    let response = LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_at: expires_at.to_rfc3339(),
        user: UserInfo {
            id: user.id.to_string(),
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            email: user.email.clone(),
            role: role_str.to_string(),
            email_verified: user.email_verified,
        },
    };

    info!("Login successful for user: {} (ID: {})", request.username, user.id);
    Ok(Json(ApiResponse::new(response)))
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
    State(ctx): State<TasksContext>,
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

    let user_repo = ctx.repositories.user_repository();

    // Check if username already exists
    match user_repo.find_by_username(&request.username).await {
        Ok(Some(_)) => {
            return Err(RestError::BadRequest(
                "Username already exists".to_string(),
            ));
        }
        Ok(None) => {
            // Username is available, continue
        }
        Err(e) => {
            error!("Database error checking username availability: {}", e);
            return Err(RestError::InternalError("Registration service unavailable".to_string()));
        }
    }

    // Check if email already exists
    match user_repo.find_by_email(&request.email).await {
        Ok(Some(_)) => {
            return Err(RestError::BadRequest(
                "Email address already registered".to_string(),
            ));
        }
        Ok(None) => {
            // Email is available, continue
        }
        Err(e) => {
            error!("Database error checking email availability: {}", e);
            return Err(RestError::InternalError("Registration service unavailable".to_string()));
        }
    }

    // Hash the password
    let password_hash = hash(request.password.as_bytes(), DEFAULT_COST)
        .map_err(|e| RestError::InternalError(format!("Failed to hash password: {}", e)))?;

    // Create user in database
    match user_repo.create_user(
        &request.username,
        &request.email,
        &password_hash,
        "user", // Default role
    ).await {
        Ok(user) => {
            let user_info = UserInfo {
                id: user.id.to_string(),
                username: user.username,
                display_name: user.display_name,
                email: user.email,
                role: match user.role {
                    ratchet_api_types::UserRole::Admin => "admin",
                    ratchet_api_types::UserRole::User => "user",
                    ratchet_api_types::UserRole::ReadOnly => "readonly",
                    ratchet_api_types::UserRole::Service => "service",
                }.to_string(),
                email_verified: user.email_verified,
            };

            info!("Registration successful for user: {} (ID: {})", request.username, user.id);
            Ok(Json(ApiResponse::new(serde_json::json!({
                "message": "User registered successfully",
                "user": user_info
            }))))
        }
        Err(e) => {
            error!("Failed to create user {}: {}", request.username, e);
            Err(RestError::InternalError("Failed to create user account".to_string()))
        }
    }
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
    State(ctx): State<TasksContext>,
    Extension(auth_context): Extension<AuthContext>,
) -> RestResult<impl IntoResponse> {
    if !auth_context.is_authenticated {
        return Err(RestError::unauthorized("Authentication required"));
    }

    let user_repo = ctx.repositories.user_repository();

    // Parse user ID from auth context
    let user_id: ApiId = match auth_context.user_id.parse::<i32>() {
        Ok(id) => ApiId::from_i32(id),
        Err(_) => {
            warn!("Invalid user ID format in auth context: {}", auth_context.user_id);
            return Err(RestError::unauthorized("Invalid user session"));
        }
    };

    // Fetch user details from database
    match user_repo.find_by_id(user_id.as_i32().unwrap_or(0)).await {
        Ok(Some(user)) => {
            let user_info = UserInfo {
                id: user.id.to_string(),
                username: user.username,
                display_name: user.display_name,
                email: user.email,
                role: match user.role {
                    ratchet_api_types::UserRole::Admin => "admin",
                    ratchet_api_types::UserRole::User => "user",
                    ratchet_api_types::UserRole::ReadOnly => "readonly",
                    ratchet_api_types::UserRole::Service => "service",
                }.to_string(),
                email_verified: user.email_verified,
            };

            Ok(Json(ApiResponse::new(user_info)))
        }
        Ok(None) => {
            warn!("User not found for authenticated session: {}", auth_context.user_id);
            Err(RestError::unauthorized("User account not found"))
        }
        Err(e) => {
            error!("Database error fetching user details: {}", e);
            Err(RestError::InternalError("Failed to fetch user information".to_string()))
        }
    }
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
    State(ctx): State<TasksContext>,
    Extension(auth_context): Extension<AuthContext>,
) -> RestResult<impl IntoResponse> {
    if !auth_context.is_authenticated {
        return Err(RestError::unauthorized("Authentication required"));
    }

    let session_repo = ctx.repositories.session_repository();

    // Invalidate the session in the database
    if let Err(e) = session_repo.invalidate_session(&auth_context.session_id).await {
        warn!("Failed to invalidate session {}: {}", auth_context.session_id, e);
        // Don't fail the logout request even if session invalidation fails
    }

    info!("User logged out: {} (session: {})", auth_context.user_id, auth_context.session_id);

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
    State(ctx): State<TasksContext>,
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

    let user_repo = ctx.repositories.user_repository();
    let session_repo = ctx.repositories.session_repository();

    // Parse user ID from auth context
    let user_id: ApiId = match auth_context.user_id.parse::<i32>() {
        Ok(id) => ApiId::from_i32(id),
        Err(_) => {
            warn!("Invalid user ID format in auth context: {}", auth_context.user_id);
            return Err(RestError::unauthorized("Invalid user session"));
        }
    };

    // TODO: Implement proper password verification once password field is available in UnifiedUser
    // For now, use hardcoded validation for known users
    let user = match user_repo.find_by_id(user_id.as_i32().unwrap_or(0)).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("User not found for password change: {}", auth_context.user_id);
            return Err(RestError::unauthorized("User account not found"));
        }
        Err(e) => {
            error!("Database error fetching user for password change: {}", e);
            return Err(RestError::InternalError("Password change service unavailable".to_string()));
        }
    };

    // For demonstration, validate against hardcoded passwords
    let current_valid = match user.username.as_str() {
        "admin" => request.current_password == "admin123",
        "user" => request.current_password == "user123",
        _ => {
            // For other users, accept current password for demonstration
            true
        }
    };

    if !current_valid {
        warn!("Invalid current password for user: {}", auth_context.user_id);
        return Err(RestError::BadRequest(
            "Current password is incorrect".to_string(),
        ));
    }

    // Hash the new password
    let new_password_hash = hash(request.new_password.as_bytes(), DEFAULT_COST)
        .map_err(|e| RestError::InternalError(format!("Failed to hash password: {}", e)))?;

    // Update password in database
    match user_repo.update_password(user_id.clone(), &new_password_hash).await {
        Ok(_) => {
            // Invalidate all existing sessions for this user (except current one)
            if let Err(e) = session_repo.invalidate_user_sessions(user_id).await {
                warn!("Failed to invalidate user sessions after password change: {}", e);
                // Don't fail the password change if session invalidation fails
            }

            info!("Password changed successfully for user: {}", auth_context.user_id);
            Ok(Json(ApiResponse::new(serde_json::json!({
                "message": "Password changed successfully. Please log in again with your new password."
            }))))
        }
        Err(e) => {
            error!("Failed to update password for user {}: {}", auth_context.user_id, e);
            Err(RestError::InternalError("Failed to update password".to_string()))
        }
    }
}