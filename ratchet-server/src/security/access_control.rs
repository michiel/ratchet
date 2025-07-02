//! Access control and authorization for repository operations
//!
//! This module provides role-based access control (RBAC) and permission
//! management for repository operations and user actions.

use anyhow::Result;
use chrono::{DateTime, Utc, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{AccessControlConfig, Permission, UserRole};
use super::SecurityContext;

/// Access control service for authorization
pub struct AccessControlService {
    /// User permissions by user ID
    user_permissions: Arc<RwLock<HashMap<String, UserPermissions>>>,
    /// Role definitions
    role_definitions: Arc<RwLock<HashMap<UserRole, RoleDefinition>>>,
    /// Repository access rules
    repository_rules: Arc<RwLock<HashMap<i32, RepositoryAccessRules>>>,
    /// Access control configuration
    config: Arc<RwLock<AccessControlConfig>>,
}

/// User permissions and roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// User ID
    pub user_id: String,
    /// User roles
    pub roles: Vec<UserRole>,
    /// Direct permissions (in addition to role permissions)
    pub permissions: Vec<Permission>,
    /// Repository-specific permissions
    pub repository_permissions: HashMap<i32, Vec<Permission>>,
    /// Permission metadata
    pub metadata: PermissionMetadata,
}

/// Permission metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionMetadata {
    /// Granted timestamp
    pub granted_at: DateTime<Utc>,
    /// Granted by user
    pub granted_by: String,
    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Permission source
    pub source: PermissionSource,
}

/// Permission source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionSource {
    Role,
    Direct,
    Repository,
    Inherited,
    System,
}

impl Default for PermissionMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            granted_at: now,
            granted_by: "system".to_string(),
            expires_at: None,
            updated_at: now,
            source: PermissionSource::System,
        }
    }
}

/// Role definition with permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// Role name
    pub role: UserRole,
    /// Role description
    pub description: String,
    /// Permissions granted by this role
    pub permissions: Vec<Permission>,
    /// Inherits from other roles
    pub inherits_from: Vec<UserRole>,
    /// Role metadata
    pub metadata: RoleMetadata,
}

/// Role metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMetadata {
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Role priority (higher number = higher priority)
    pub priority: u32,
    /// Role status
    pub status: RoleStatus,
}

/// Role status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoleStatus {
    Active,
    Deprecated,
    Disabled,
}

impl Default for RoleMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            priority: 100,
            status: RoleStatus::Active,
        }
    }
}

/// Repository-specific access rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAccessRules {
    /// Repository ID
    pub repository_id: i32,
    /// Required permissions for operations
    pub operation_permissions: HashMap<String, Vec<Permission>>,
    /// IP whitelist for this repository
    pub ip_whitelist: Vec<String>,
    /// Time-based access restrictions
    pub time_restrictions: Option<TimeRestrictions>,
    /// Maximum concurrent sessions
    pub max_concurrent_sessions: u32,
    /// Access rule metadata
    pub metadata: AccessRuleMetadata,
}

/// Time-based access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    /// Allowed hours (0-23)
    pub allowed_hours: Vec<u8>,
    /// Allowed days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Vec<u8>,
    /// Timezone for time restrictions
    pub timezone: String,
}

/// Access rule metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRuleMetadata {
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Rule status
    pub status: RuleStatus,
}

/// Access rule status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleStatus {
    Active,
    Disabled,
    Testing,
}

impl Default for AccessRuleMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            status: RuleStatus::Active,
        }
    }
}

/// Authorization result
#[derive(Debug, Clone)]
pub struct AuthorizationResult {
    /// Authorization granted
    pub granted: bool,
    /// Reason for decision
    pub reason: String,
    /// Required permissions
    pub required_permissions: Vec<Permission>,
    /// User permissions
    pub user_permissions: Vec<Permission>,
    /// Missing permissions
    pub missing_permissions: Vec<Permission>,
}

impl AccessControlService {
    /// Create a new access control service
    pub fn new(config: AccessControlConfig) -> Self {
        let service = Self {
            user_permissions: Arc::new(RwLock::new(HashMap::new())),
            role_definitions: Arc::new(RwLock::new(HashMap::new())),
            repository_rules: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
        };

        // Initialize with default roles
        tokio::spawn({
            let service = service.clone();
            async move {
                if let Err(e) = service.initialize_default_roles().await {
                    tracing::error!("Failed to initialize default roles: {}", e);
                }
            }
        });

        service
    }

    /// Initialize default role definitions
    async fn initialize_default_roles(&self) -> Result<()> {
        let mut roles = self.role_definitions.write().await;

        // Guest role
        roles.insert(UserRole::Guest, RoleDefinition {
            role: UserRole::Guest,
            description: "Guest user with minimal permissions".to_string(),
            permissions: vec![Permission::Read],
            inherits_from: vec![],
            metadata: RoleMetadata {
                priority: 10,
                ..Default::default()
            },
        });

        // User role
        roles.insert(UserRole::User, RoleDefinition {
            role: UserRole::User,
            description: "Standard user permissions".to_string(),
            permissions: vec![Permission::Read, Permission::Write],
            inherits_from: vec![UserRole::Guest],
            metadata: RoleMetadata {
                priority: 50,
                ..Default::default()
            },
        });

        // Moderator role
        roles.insert(UserRole::Moderator, RoleDefinition {
            role: UserRole::Moderator,
            description: "Moderator with sync and monitoring permissions".to_string(),
            permissions: vec![Permission::Read, Permission::Write, Permission::Sync, Permission::Monitor],
            inherits_from: vec![UserRole::User],
            metadata: RoleMetadata {
                priority: 75,
                ..Default::default()
            },
        });

        // Admin role
        roles.insert(UserRole::Admin, RoleDefinition {
            role: UserRole::Admin,
            description: "Administrator with full repository permissions".to_string(),
            permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Delete,
                Permission::Sync,
                Permission::Monitor,
            ],
            inherits_from: vec![UserRole::Moderator],
            metadata: RoleMetadata {
                priority: 90,
                ..Default::default()
            },
        });

        // SuperAdmin role
        roles.insert(UserRole::SuperAdmin, RoleDefinition {
            role: UserRole::SuperAdmin,
            description: "Super administrator with system-wide permissions".to_string(),
            permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Delete,
                Permission::Admin,
                Permission::Sync,
                Permission::Monitor,
            ],
            inherits_from: vec![UserRole::Admin],
            metadata: RoleMetadata {
                priority: 100,
                ..Default::default()
            },
        });

        Ok(())
    }

    /// Check if user has permission for repository operation
    pub async fn check_permission(
        &self,
        context: &SecurityContext,
        repository_id: i32,
        operation: &str,
    ) -> Result<bool> {
        let result = self.authorize_operation(context, repository_id, operation).await?;
        Ok(result.granted)
    }

    /// Authorize a repository operation
    pub async fn authorize_operation(
        &self,
        context: &SecurityContext,
        repository_id: i32,
        operation: &str,
    ) -> Result<AuthorizationResult> {
        let config = self.config.read().await;

        // If RBAC is disabled, allow all operations
        if !config.enable_rbac {
            return Ok(AuthorizationResult {
                granted: true,
                reason: "RBAC disabled".to_string(),
                required_permissions: vec![],
                user_permissions: vec![],
                missing_permissions: vec![],
            });
        }

        // Get user ID from context
        let user_id = match &context.user_id {
            Some(id) => id,
            None => {
                return Ok(AuthorizationResult {
                    granted: false,
                    reason: "No user ID in security context".to_string(),
                    required_permissions: vec![],
                    user_permissions: vec![],
                    missing_permissions: vec![],
                });
            }
        };

        // Check IP whitelist
        if !config.ip_whitelist.is_empty() {
            if let Some(ip) = &context.ip_address {
                if !config.ip_whitelist.contains(ip) {
                    return Ok(AuthorizationResult {
                        granted: false,
                        reason: format!("IP address {} not in whitelist", ip),
                        required_permissions: vec![],
                        user_permissions: vec![],
                        missing_permissions: vec![],
                    });
                }
            } else {
                return Ok(AuthorizationResult {
                    granted: false,
                    reason: "No IP address in context for whitelist check".to_string(),
                    required_permissions: vec![],
                    user_permissions: vec![],
                    missing_permissions: vec![],
                });
            }
        }

        // Check time restrictions
        if config.time_restrictions {
            if let Some((start_hour, end_hour)) = config.allowed_hours {
                let current_hour = Utc::now().hour() as u8;
                if current_hour < start_hour || current_hour > end_hour {
                    return Ok(AuthorizationResult {
                        granted: false,
                        reason: format!("Access not allowed at current time ({})", current_hour),
                        required_permissions: vec![],
                        user_permissions: vec![],
                        missing_permissions: vec![],
                    });
                }
            }
        }

        // Get required permissions for operation
        let required_permissions = self.get_required_permissions(repository_id, operation).await?;

        // Get user permissions
        let user_permissions = self.get_user_permissions(user_id).await?;

        // Check if user has all required permissions
        let missing_permissions: Vec<Permission> = required_permissions
            .iter()
            .filter(|&perm| !user_permissions.contains(perm))
            .cloned()
            .collect();

        let granted = missing_permissions.is_empty();
        let reason = if granted {
            format!("User has all required permissions for operation '{}'", operation)
        } else {
            format!("User missing permissions: {:?}", missing_permissions)
        };

        Ok(AuthorizationResult {
            granted,
            reason,
            required_permissions,
            user_permissions,
            missing_permissions,
        })
    }

    /// Get required permissions for a repository operation
    async fn get_required_permissions(&self, repository_id: i32, operation: &str) -> Result<Vec<Permission>> {
        let repository_rules = self.repository_rules.read().await;
        
        if let Some(rules) = repository_rules.get(&repository_id) {
            if let Some(permissions) = rules.operation_permissions.get(operation) {
                return Ok(permissions.clone());
            }
        }

        // Default operation permissions
        let default_permissions = match operation {
            "read" | "list" | "get" => vec![Permission::Read],
            "write" | "create" | "update" => vec![Permission::Write],
            "delete" | "remove" => vec![Permission::Delete],
            "sync" | "synchronize" => vec![Permission::Sync],
            "monitor" | "health" | "status" => vec![Permission::Monitor],
            "admin" | "configure" | "manage" => vec![Permission::Admin],
            _ => vec![Permission::Read], // Default to read permission
        };

        Ok(default_permissions)
    }

    /// Get all effective permissions for a user
    async fn get_user_permissions(&self, user_id: &str) -> Result<Vec<Permission>> {
        let user_permissions = self.user_permissions.read().await;
        
        if let Some(user_perms) = user_permissions.get(user_id) {
            let mut all_permissions = HashSet::new();

            // Add direct permissions
            for perm in &user_perms.permissions {
                all_permissions.insert(perm.clone());
            }

            // Add permissions from roles
            let role_definitions = self.role_definitions.read().await;
            for role in &user_perms.roles {
                if let Some(role_def) = role_definitions.get(role) {
                    // Add role permissions
                    for perm in &role_def.permissions {
                        all_permissions.insert(perm.clone());
                    }

                    // Add inherited permissions
                    for inherited_role in &role_def.inherits_from {
                        if let Some(inherited_def) = role_definitions.get(inherited_role) {
                            for perm in &inherited_def.permissions {
                                all_permissions.insert(perm.clone());
                            }
                        }
                    }
                }
            }

            Ok(all_permissions.into_iter().collect())
        } else {
            // User not found, return default permissions
            let config = self.config.read().await;
            Ok(config.default_permissions.clone())
        }
    }

    /// Grant permissions to a user
    pub async fn grant_user_permissions(
        &self,
        user_id: String,
        permissions: Vec<Permission>,
        granted_by: String,
    ) -> Result<()> {
        let mut user_permissions = self.user_permissions.write().await;
        
        let user_perms = user_permissions.entry(user_id.clone()).or_insert_with(|| {
            UserPermissions {
                user_id: user_id.clone(),
                roles: vec![],
                permissions: vec![],
                repository_permissions: HashMap::new(),
                metadata: PermissionMetadata {
                    granted_by: granted_by.clone(),
                    source: PermissionSource::Direct,
                    ..Default::default()
                },
            }
        });

        // Add new permissions (avoid duplicates)
        for perm in permissions {
            if !user_perms.permissions.contains(&perm) {
                user_perms.permissions.push(perm);
            }
        }

        user_perms.metadata.updated_at = Utc::now();
        user_perms.metadata.granted_by = granted_by;

        Ok(())
    }

    /// Assign role to a user
    pub async fn assign_user_role(
        &self,
        user_id: String,
        role: UserRole,
        granted_by: String,
    ) -> Result<()> {
        let mut user_permissions = self.user_permissions.write().await;
        
        let user_perms = user_permissions.entry(user_id.clone()).or_insert_with(|| {
            UserPermissions {
                user_id: user_id.clone(),
                roles: vec![],
                permissions: vec![],
                repository_permissions: HashMap::new(),
                metadata: PermissionMetadata {
                    granted_by: granted_by.clone(),
                    source: PermissionSource::Role,
                    ..Default::default()
                },
            }
        });

        // Add role if not already assigned
        if !user_perms.roles.contains(&role) {
            user_perms.roles.push(role);
        }

        user_perms.metadata.updated_at = Utc::now();
        user_perms.metadata.granted_by = granted_by;

        Ok(())
    }

    /// Revoke permissions from a user
    pub async fn revoke_user_permissions(
        &self,
        user_id: &str,
        permissions: Vec<Permission>,
    ) -> Result<()> {
        let mut user_permissions = self.user_permissions.write().await;
        
        if let Some(user_perms) = user_permissions.get_mut(user_id) {
            user_perms.permissions.retain(|perm| !permissions.contains(perm));
            user_perms.metadata.updated_at = Utc::now();
        }

        Ok(())
    }

    /// Remove role from a user
    pub async fn remove_user_role(&self, user_id: &str, role: UserRole) -> Result<()> {
        let mut user_permissions = self.user_permissions.write().await;
        
        if let Some(user_perms) = user_permissions.get_mut(user_id) {
            user_perms.roles.retain(|r| r != &role);
            user_perms.metadata.updated_at = Utc::now();
        }

        Ok(())
    }

    /// Set repository access rules
    pub async fn set_repository_rules(
        &self,
        repository_id: i32,
        rules: RepositoryAccessRules,
    ) -> Result<()> {
        let mut repository_rules = self.repository_rules.write().await;
        repository_rules.insert(repository_id, rules);
        Ok(())
    }

    /// Get user permissions
    pub async fn get_user_permissions_info(&self, user_id: &str) -> Option<UserPermissions> {
        let user_permissions = self.user_permissions.read().await;
        user_permissions.get(user_id).cloned()
    }

    /// List all users with permissions
    pub async fn list_users(&self) -> Vec<String> {
        let user_permissions = self.user_permissions.read().await;
        user_permissions.keys().cloned().collect()
    }

    /// Update access control configuration
    pub async fn update_config(&self, config: AccessControlConfig) -> Result<()> {
        let mut current_config = self.config.write().await;
        *current_config = config;
        Ok(())
    }
}

impl Clone for AccessControlService {
    fn clone(&self) -> Self {
        Self {
            user_permissions: self.user_permissions.clone(),
            role_definitions: self.role_definitions.clone(),
            repository_rules: self.repository_rules.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_control_service_creation() {
        let config = AccessControlConfig::default();
        let service = AccessControlService::new(config);
        
        // Wait a moment for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check that default roles were created
        let roles = service.role_definitions.read().await;
        assert!(roles.contains_key(&UserRole::Guest));
        assert!(roles.contains_key(&UserRole::User));
        assert!(roles.contains_key(&UserRole::Admin));
    }

    #[tokio::test]
    async fn test_user_role_assignment() {
        let config = AccessControlConfig::default();
        let service = AccessControlService::new(config);
        
        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let user_id = "test_user".to_string();
        let granted_by = "admin".to_string();

        // Assign user role
        service.assign_user_role(user_id.clone(), UserRole::User, granted_by).await.unwrap();

        // Check user permissions
        let user_perms = service.get_user_permissions_info(&user_id).await;
        assert!(user_perms.is_some());
        assert!(user_perms.unwrap().roles.contains(&UserRole::User));
    }

    #[tokio::test]
    async fn test_permission_authorization() {
        let config = AccessControlConfig::default();
        let service = AccessControlService::new(config);
        
        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let user_id = "test_user".to_string();
        
        // Assign user role
        service.assign_user_role(user_id.clone(), UserRole::User, "admin".to_string()).await.unwrap();

        // Create security context
        let mut context = SecurityContext::new("test_correlation".to_string());
        context.user_id = Some(user_id);

        // Test read operation (should be allowed for User role)
        let result = service.authorize_operation(&context, 1, "read").await.unwrap();
        assert!(result.granted);

        // Test admin operation (should be denied for User role)
        let result = service.authorize_operation(&context, 1, "admin").await.unwrap();
        assert!(!result.granted);
    }
}