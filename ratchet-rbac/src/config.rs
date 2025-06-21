//! Configuration for RBAC system

use crate::models::{Permission, PermissionScope, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacConfig {
    /// Casbin model file path (optional, defaults to embedded model)
    pub model_path: Option<String>,
    
    /// Whether to enable policy caching
    pub enable_cache: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Default platform roles
    pub default_platform_roles: Vec<String>,
    
    /// Default tenant roles  
    pub default_tenant_roles: Vec<String>,
    
    /// Standard role definitions
    pub standard_roles: HashMap<String, RoleDefinition>,
    
    /// Custom permission definitions
    pub custom_permissions: HashMap<String, RoleDefinition>,
    
    /// Policy auto-save interval in seconds
    pub policy_save_interval_seconds: Option<u64>,
    
    /// Enable audit logging for authorization decisions
    pub enable_audit_logging: bool,
}

impl Default for RbacConfig {
    fn default() -> Self {
        let mut standard_roles = HashMap::new();
        
        // Platform roles
        standard_roles.insert(
            "platform_admin".to_string(),
            RoleDefinition {
                display_name: "Platform Administrator".to_string(),
                description: Some("Full platform administration access".to_string()),
                permissions: vec![
                    "create:tenants".to_string(),
                    "read:all_metrics".to_string(),
                    "manage:platform_config".to_string(),
                    "manage:platform_users".to_string(),
                ],
                inherits_from: vec![],
                is_platform_role: true,
            },
        );
        
        standard_roles.insert(
            "platform_monitor".to_string(),
            RoleDefinition {
                display_name: "Platform Monitor".to_string(),
                description: Some("Read-only platform monitoring access".to_string()),
                permissions: vec![
                    "read:all_metrics".to_string(),
                    "read:platform_logs".to_string(),
                ],
                inherits_from: vec![],
                is_platform_role: true,
            },
        );
        
        // Tenant roles
        standard_roles.insert(
            "tenant_admin".to_string(),
            RoleDefinition {
                display_name: "Tenant Administrator".to_string(),
                description: Some("Full tenant administration access".to_string()),
                permissions: vec![
                    "create:tasks".to_string(),
                    "read:tasks".to_string(),
                    "update:tasks".to_string(),
                    "delete:tasks".to_string(),
                    "execute:tasks".to_string(),
                    "manage:users".to_string(),
                    "manage:roles".to_string(),
                    "read:metrics".to_string(),
                ],
                inherits_from: vec![],
                is_platform_role: false,
            },
        );
        
        standard_roles.insert(
            "tenant_user".to_string(),
            RoleDefinition {
                display_name: "Tenant User".to_string(),
                description: Some("Standard tenant user access".to_string()),
                permissions: vec![
                    "create:tasks".to_string(),
                    "read:tasks".to_string(),
                    "update:own_tasks".to_string(),
                    "execute:tasks".to_string(),
                    "read:own_executions".to_string(),
                ],
                inherits_from: vec![],
                is_platform_role: false,
            },
        );
        
        standard_roles.insert(
            "tenant_viewer".to_string(),
            RoleDefinition {
                display_name: "Tenant Viewer".to_string(),
                description: Some("Read-only tenant access".to_string()),
                permissions: vec![
                    "read:tasks".to_string(),
                    "read:executions".to_string(),
                    "read:metrics".to_string(),
                ],
                inherits_from: vec![],
                is_platform_role: false,
            },
        );
        
        Self {
            model_path: None,
            enable_cache: true,
            cache_ttl_seconds: 300, // 5 minutes
            default_platform_roles: vec!["platform_monitor".to_string()],
            default_tenant_roles: vec!["tenant_user".to_string()],
            standard_roles,
            custom_permissions: HashMap::new(),
            policy_save_interval_seconds: Some(60), // 1 minute
            enable_audit_logging: true,
        }
    }
}

impl RbacConfig {
    /// Get embedded Casbin model configuration
    pub fn get_model_config(&self) -> String {
        // Embedded Casbin model for multi-tenant RBAC
        r#"[request_definition]
r = sub, obj, act, dom

[policy_definition]
p = sub, obj, act, dom

[role_definition]
g = _, _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub, r.dom) && r.obj == p.obj && r.act == p.act && r.dom == p.dom
"#.to_string()
    }
    
    /// Get role definition by name
    pub fn get_role_definition(&self, role_name: &str) -> Option<&RoleDefinition> {
        self.standard_roles.get(role_name)
            .or_else(|| self.custom_permissions.get(role_name))
    }
    
    /// Get all platform roles
    pub fn get_platform_roles(&self) -> Vec<&RoleDefinition> {
        self.standard_roles
            .values()
            .chain(self.custom_permissions.values())
            .filter(|role| role.is_platform_role)
            .collect()
    }
    
    /// Get all tenant roles
    pub fn get_tenant_roles(&self) -> Vec<&RoleDefinition> {
        self.standard_roles
            .values()
            .chain(self.custom_permissions.values())
            .filter(|role| !role.is_platform_role)
            .collect()
    }
    
    /// Convert role definition to internal Role model
    pub fn to_role(&self, name: &str, definition: &RoleDefinition, tenant_id: Option<i32>) -> Role {
        let permissions = definition.permissions
            .iter()
            .map(|perm| self.parse_permission(perm))
            .collect();
            
        if definition.is_platform_role {
            Role::new_platform_role(name.to_string(), definition.display_name.clone(), permissions)
        } else if let Some(tid) = tenant_id {
            Role::new_tenant_role(name.to_string(), definition.display_name.clone(), tid, permissions)
        } else {
            // Fallback to platform role if no tenant specified
            Role::new_platform_role(name.to_string(), definition.display_name.clone(), permissions)
        }
    }
    
    /// Parse permission string into Permission struct
    fn parse_permission(&self, permission_str: &str) -> Permission {
        let parts: Vec<&str> = permission_str.split(':').collect();
        if parts.len() >= 2 {
            let action = parts[0].to_string();
            let resource = parts[1].to_string();
            
            // Determine scope based on resource and action
            let scope = if resource.starts_with("all_") || resource.starts_with("platform_") {
                PermissionScope::Platform
            } else if resource.starts_with("own_") {
                PermissionScope::Self_
            } else {
                PermissionScope::Tenant
            };
            
            Permission::new(resource, action, scope)
        } else {
            // Fallback for malformed permission strings
            Permission::new("unknown".to_string(), permission_str.to_string(), PermissionScope::Tenant)
        }
    }
}

/// Role definition in configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub display_name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub inherits_from: Vec<String>,
    pub is_platform_role: bool,
}

impl RoleDefinition {
    /// Create a new role definition
    pub fn new(
        display_name: String,
        permissions: Vec<String>,
        is_platform_role: bool,
    ) -> Self {
        Self {
            display_name,
            description: None,
            permissions,
            inherits_from: Vec::new(),
            is_platform_role,
        }
    }
    
    /// Add permission to role definition
    pub fn add_permission(&mut self, permission: String) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }
    
    /// Add role inheritance
    pub fn add_inheritance(&mut self, parent_role: String) {
        if !self.inherits_from.contains(&parent_role) {
            self.inherits_from.push(parent_role);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = RbacConfig::default();
        
        // Check platform roles exist
        assert!(config.standard_roles.contains_key("platform_admin"));
        assert!(config.standard_roles.contains_key("platform_monitor"));
        
        // Check tenant roles exist  
        assert!(config.standard_roles.contains_key("tenant_admin"));
        assert!(config.standard_roles.contains_key("tenant_user"));
        assert!(config.standard_roles.contains_key("tenant_viewer"));
        
        // Check default role assignments
        assert!(config.default_platform_roles.contains(&"platform_monitor".to_string()));
        assert!(config.default_tenant_roles.contains(&"tenant_user".to_string()));
    }
    
    #[test]
    fn test_permission_parsing() {
        let config = RbacConfig::default();
        
        let perm = config.parse_permission("create:tasks");
        assert_eq!(perm.action, "create");
        assert_eq!(perm.resource, "tasks");
        assert!(matches!(perm.scope, PermissionScope::Tenant));
        
        let perm = config.parse_permission("read:all_metrics");
        assert_eq!(perm.action, "read");
        assert_eq!(perm.resource, "all_metrics");
        assert!(matches!(perm.scope, PermissionScope::Platform));
        
        let perm = config.parse_permission("update:own_tasks");
        assert_eq!(perm.action, "update");
        assert_eq!(perm.resource, "own_tasks");
        assert!(matches!(perm.scope, PermissionScope::Self_));
    }
    
    #[test]
    fn test_role_filtering() {
        let config = RbacConfig::default();
        
        let platform_roles = config.get_platform_roles();
        assert!(platform_roles.len() >= 2);
        assert!(platform_roles.iter().all(|r| r.is_platform_role));
        
        let tenant_roles = config.get_tenant_roles();
        assert!(tenant_roles.len() >= 3);
        assert!(tenant_roles.iter().all(|r| !r.is_platform_role));
    }
    
    #[test]
    fn test_model_config() {
        let config = RbacConfig::default();
        let model = config.get_model_config();
        
        assert!(model.contains("[request_definition]"));
        assert!(model.contains("[policy_definition]"));
        assert!(model.contains("[role_definition]"));
        assert!(model.contains("[policy_effect]"));
        assert!(model.contains("[matchers]"));
    }
}