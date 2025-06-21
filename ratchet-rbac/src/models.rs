//! Data models for RBAC system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Tenant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub settings: HashMap<String, serde_json::Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i32>,
}

impl Tenant {
    /// Create a new tenant
    pub fn new(name: String, display_name: String, created_by: Option<i32>) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            name,
            display_name,
            description: None,
            settings: HashMap::new(),
            is_active: true,
            created_at: now,
            updated_at: now,
            created_by,
        }
    }

    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get tenant ID as string (for Casbin domain)
    pub fn domain(&self) -> String {
        format!("tenant_{}", self.id)
    }
}

/// User-tenant association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUser {
    pub id: i32,
    pub user_id: i32,
    pub tenant_id: i32,
    pub joined_at: DateTime<Utc>,
    pub joined_by: Option<i32>,
}

impl TenantUser {
    /// Create a new tenant user association
    pub fn new(user_id: i32, tenant_id: i32, joined_by: Option<i32>) -> Self {
        Self {
            id: 0, // Will be set by database
            user_id,
            tenant_id,
            joined_at: Utc::now(),
            joined_by,
        }
    }
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub inherits_from: Vec<String>,
    pub is_platform_role: bool,
    pub tenant_id: Option<i32>,
}

impl Role {
    /// Create a new tenant role
    pub fn new_tenant_role(
        name: String,
        display_name: String,
        tenant_id: i32,
        permissions: Vec<Permission>,
    ) -> Self {
        Self {
            name,
            display_name,
            description: None,
            permissions,
            inherits_from: Vec::new(),
            is_platform_role: false,
            tenant_id: Some(tenant_id),
        }
    }

    /// Create a new platform role
    pub fn new_platform_role(name: String, display_name: String, permissions: Vec<Permission>) -> Self {
        Self {
            name,
            display_name,
            description: None,
            permissions,
            inherits_from: Vec::new(),
            is_platform_role: true,
            tenant_id: None,
        }
    }

    /// Check if this is a platform role
    pub fn is_platform_role(&self) -> bool {
        self.is_platform_role
    }

    /// Get the scope for this role (platform or tenant domain)
    pub fn scope(&self) -> String {
        if self.is_platform_role {
            "platform".to_string()
        } else if let Some(tenant_id) = self.tenant_id {
            format!("tenant_{}", tenant_id)
        } else {
            "unknown".to_string()
        }
    }
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub scope: PermissionScope,
    pub conditions: Vec<String>,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: String, action: String, scope: PermissionScope) -> Self {
        Self {
            resource,
            action,
            scope,
            conditions: Vec::new(),
        }
    }

    /// Create permission with conditions
    pub fn with_conditions(
        resource: String,
        action: String,
        scope: PermissionScope,
        conditions: Vec<String>,
    ) -> Self {
        Self {
            resource,
            action,
            scope,
            conditions,
        }
    }

    /// Convert to Casbin policy string
    pub fn to_policy_string(&self, subject: &str, domain: &str) -> String {
        format!("{}, {}, {}, {}", subject, self.resource, self.action, domain)
    }

    /// Check if permission matches the given request
    pub fn matches(&self, resource: &str, action: &str) -> bool {
        self.resource == resource && self.action == action
    }
}

/// Permission scope types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionScope {
    /// Platform-wide operations
    Platform,
    /// Tenant-scoped operations
    Tenant,
    /// User's own resources only
    Self_,
    /// Custom scope with conditions
    Custom(String),
}

impl PermissionScope {
    /// Convert scope to domain string for Casbin
    pub fn to_domain(&self, tenant_id: Option<i32>) -> String {
        match self {
            PermissionScope::Platform => "platform".to_string(),
            PermissionScope::Tenant => {
                if let Some(tid) = tenant_id {
                    format!("tenant_{}", tid)
                } else {
                    "tenant_*".to_string()
                }
            }
            PermissionScope::Self_ => "self".to_string(),
            PermissionScope::Custom(domain) => domain.clone(),
        }
    }
}

/// User role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub id: i32,
    pub user_id: i32,
    pub tenant_id: Option<i32>,
    pub role_name: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<i32>,
}

impl UserRole {
    /// Create a new user role assignment
    pub fn new(
        user_id: i32,
        tenant_id: Option<i32>,
        role_name: String,
        assigned_by: Option<i32>,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            user_id,
            tenant_id,
            role_name,
            assigned_at: Utc::now(),
            assigned_by,
        }
    }

    /// Check if this is a platform role assignment
    pub fn is_platform_role(&self) -> bool {
        self.tenant_id.is_none()
    }

    /// Get the domain for this role assignment
    pub fn domain(&self) -> String {
        if let Some(tenant_id) = self.tenant_id {
            format!("tenant_{}", tenant_id)
        } else {
            "platform".to_string()
        }
    }
}

/// Custom role definition for tenants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCustomRole {
    pub id: i32,
    pub tenant_id: i32,
    pub role_name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub created_at: DateTime<Utc>,
    pub created_by: i32,
}

impl TenantCustomRole {
    /// Create a new custom role for a tenant
    pub fn new(
        tenant_id: i32,
        role_name: String,
        display_name: String,
        permissions: Vec<Permission>,
        created_by: i32,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            tenant_id,
            role_name,
            display_name,
            description: None,
            permissions,
            created_at: Utc::now(),
            created_by,
        }
    }

    /// Convert to Role for use with Casbin
    pub fn to_role(&self) -> Role {
        Role {
            name: self.role_name.clone(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            permissions: self.permissions.clone(),
            inherits_from: Vec::new(),
            is_platform_role: false,
            tenant_id: Some(self.tenant_id),
        }
    }
}

/// Resource types for permission checking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Task,
    Execution,
    Job,
    Schedule,
    User,
    Role,
    Tenant,
    Metric,
    Configuration,
    ApiKey,
    Session,
}

impl ResourceType {
    /// Convert to string for use in permissions
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Task => "tasks",
            ResourceType::Execution => "executions",
            ResourceType::Job => "jobs",
            ResourceType::Schedule => "schedules",
            ResourceType::User => "users",
            ResourceType::Role => "roles",
            ResourceType::Tenant => "tenants",
            ResourceType::Metric => "metrics",
            ResourceType::Configuration => "configurations",
            ResourceType::ApiKey => "api_keys",
            ResourceType::Session => "sessions",
        }
    }
}

/// Action types for permission checking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionType {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Manage,
    List,
}

impl ActionType {
    /// Convert to string for use in permissions
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Create => "create",
            ActionType::Read => "read",
            ActionType::Update => "update",
            ActionType::Delete => "delete",
            ActionType::Execute => "execute",
            ActionType::Manage => "manage",
            ActionType::List => "list",
        }
    }
}