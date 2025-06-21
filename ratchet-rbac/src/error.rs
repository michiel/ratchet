//! Error types for RBAC operations

use thiserror::Error;

/// Result type for RBAC operations
pub type RbacResult<T> = Result<T, RbacError>;

/// RBAC-specific errors
#[derive(Error, Debug)]
pub enum RbacError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// Casbin operation failed
    #[error("Casbin error: {0}")]
    Casbin(#[from] casbin::Error),

    /// Tenant not found
    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },

    /// User not found
    #[error("User not found: {user_id}")]
    UserNotFound { user_id: i32 },

    /// Role not found
    #[error("Role not found: {role_name}")]
    RoleNotFound { role_name: String },

    /// Permission denied
    #[error("Permission denied: {action} on {resource} for tenant {tenant_id}")]
    PermissionDenied {
        action: String,
        resource: String,
        tenant_id: String,
    },

    /// Invalid configuration
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    /// User not member of tenant
    #[error("User {user_id} is not a member of tenant {tenant_id}")]
    NotTenantMember { user_id: i32, tenant_id: String },

    /// Circular role inheritance detected
    #[error("Circular role inheritance detected involving role: {role_name}")]
    CircularInheritance { role_name: String },

    /// Invalid policy format
    #[error("Invalid policy format: {message}")]
    InvalidPolicy { message: String },

    /// Resource not found
    #[error("Resource not found: {resource_type}:{resource_id}")]
    ResourceNotFound {
        resource_type: String,
        resource_id: String,
    },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic internal error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl RbacError {
    /// Create a new permission denied error
    pub fn permission_denied(action: impl Into<String>, resource: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self::PermissionDenied {
            action: action.into(),
            resource: resource.into(),
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a new invalid config error
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Check if this is a permission denied error
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, Self::PermissionDenied { .. })
    }

    /// Check if this is a not found error  
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::TenantNotFound { .. }
                | Self::UserNotFound { .. }
                | Self::RoleNotFound { .. }
                | Self::ResourceNotFound { .. }
        )
    }
}