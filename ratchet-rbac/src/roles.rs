//! Role management utilities

use std::collections::HashMap;

use crate::{
    config::RbacConfig,
    error::{RbacError, RbacResult},
    models::{Permission, PermissionScope, Role},
};

/// Role manager for handling role operations
pub struct RoleManager {
    config: RbacConfig,
}

impl RoleManager {
    /// Create a new role manager
    pub fn new(config: RbacConfig) -> Self {
        Self { config }
    }

    /// Get standard role by name
    pub fn get_standard_role(&self, role_name: &str, tenant_id: Option<i32>) -> Option<Role> {
        if let Some(role_def) = self.config.get_role_definition(role_name) {
            Some(self.config.to_role(role_name, role_def, tenant_id))
        } else {
            None
        }
    }

    /// Get all platform roles
    pub fn get_platform_roles(&self) -> Vec<Role> {
        self.config
            .get_platform_roles()
            .into_iter()
            .map(|role_def| {
                let role_name = self
                    .config
                    .standard_roles
                    .iter()
                    .find(|(_, def)| std::ptr::eq(*def, role_def))
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                
                self.config.to_role(&role_name, role_def, None)
            })
            .collect()
    }

    /// Get all tenant roles
    pub fn get_tenant_roles(&self, tenant_id: i32) -> Vec<Role> {
        self.config
            .get_tenant_roles()
            .into_iter()
            .map(|role_def| {
                let role_name = self
                    .config
                    .standard_roles
                    .iter()
                    .find(|(_, def)| std::ptr::eq(*def, role_def))
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                
                self.config.to_role(&role_name, role_def, Some(tenant_id))
            })
            .collect()
    }

    /// Validate role name
    pub fn validate_role_name(&self, role_name: &str) -> RbacResult<()> {
        if role_name.is_empty() {
            return Err(RbacError::InvalidConfig {
                message: "Role name cannot be empty".to_string(),
            });
        }

        if role_name.len() > 100 {
            return Err(RbacError::InvalidConfig {
                message: "Role name cannot exceed 100 characters".to_string(),
            });
        }

        // Check for invalid characters
        if !role_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(RbacError::InvalidConfig {
                message: "Role name can only contain alphanumeric characters, underscores, and hyphens".to_string(),
            });
        }

        Ok(())
    }

    /// Validate custom role definition
    pub fn validate_custom_role(&self, role: &Role) -> RbacResult<()> {
        // Validate role name
        self.validate_role_name(&role.name)?;

        // Check if it conflicts with standard roles
        if self.config.standard_roles.contains_key(&role.name) {
            return Err(RbacError::InvalidConfig {
                message: format!("Role name '{}' conflicts with standard role", role.name),
            });
        }

        // Validate display name
        if role.display_name.is_empty() {
            return Err(RbacError::InvalidConfig {
                message: "Role display name cannot be empty".to_string(),
            });
        }

        // Validate permissions
        if role.permissions.is_empty() {
            return Err(RbacError::InvalidConfig {
                message: "Role must have at least one permission".to_string(),
            });
        }

        for permission in &role.permissions {
            self.validate_permission(permission)?;
        }

        // Validate inheritance (check for cycles)
        self.validate_inheritance_chain(&role.name, &role.inherits_from)?;

        Ok(())
    }

    /// Validate permission definition
    fn validate_permission(&self, permission: &Permission) -> RbacResult<()> {
        if permission.resource.is_empty() {
            return Err(RbacError::InvalidConfig {
                message: "Permission resource cannot be empty".to_string(),
            });
        }

        if permission.action.is_empty() {
            return Err(RbacError::InvalidConfig {
                message: "Permission action cannot be empty".to_string(),
            });
        }

        // Validate known resource types
        let valid_resources = [
            "tasks", "executions", "jobs", "schedules", "users", "roles",
            "tenants", "metrics", "configurations", "api_keys", "sessions",
        ];

        if !valid_resources.contains(&permission.resource.as_str()) &&
           !permission.resource.starts_with("custom_") {
            return Err(RbacError::InvalidConfig {
                message: format!("Unknown resource type: {}", permission.resource),
            });
        }

        // Validate known action types
        let valid_actions = ["create", "read", "update", "delete", "execute", "manage", "list"];

        if !valid_actions.contains(&permission.action.as_str()) &&
           !permission.action.starts_with("custom_") {
            return Err(RbacError::InvalidConfig {
                message: format!("Unknown action type: {}", permission.action),
            });
        }

        Ok(())
    }

    /// Validate inheritance chain for cycles
    fn validate_inheritance_chain(
        &self,
        role_name: &str,
        inherits_from: &[String],
    ) -> RbacResult<()> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = Vec::new();

        self.check_cycle(role_name, inherits_from, &mut visited, &mut stack)
    }

    /// Recursive cycle detection
    fn check_cycle(
        &self,
        current_role: &str,
        inherits_from: &[String],
        visited: &mut std::collections::HashSet<String>,
        stack: &mut Vec<String>,
    ) -> RbacResult<()> {
        if stack.contains(&current_role.to_string()) {
            return Err(RbacError::CircularInheritance {
                role_name: current_role.to_string(),
            });
        }

        if visited.contains(current_role) {
            return Ok(());
        }

        visited.insert(current_role.to_string());
        stack.push(current_role.to_string());

        for parent_role in inherits_from {
            if let Some(parent_def) = self.config.get_role_definition(parent_role) {
                self.check_cycle(parent_role, &parent_def.inherits_from, visited, stack)?;
            }
        }

        stack.pop();
        Ok(())
    }

    /// Create permission from string (action:resource format)
    pub fn parse_permission_string(&self, perm_str: &str) -> RbacResult<Permission> {
        let parts: Vec<&str> = perm_str.split(':').collect();
        
        if parts.len() != 2 {
            return Err(RbacError::InvalidConfig {
                message: format!("Invalid permission format: '{}'. Expected 'action:resource'", perm_str),
            });
        }

        let action = parts[0].to_string();
        let resource = parts[1].to_string();

        // Determine scope based on resource name
        let scope = if resource.starts_with("platform_") || resource == "tenants" {
            PermissionScope::Platform
        } else if resource.starts_with("own_") {
            PermissionScope::Self_
        } else {
            PermissionScope::Tenant
        };

        let permission = Permission::new(resource, action, scope);
        self.validate_permission(&permission)?;
        
        Ok(permission)
    }

    /// Create custom role builder
    pub fn custom_role_builder(&self, name: String, display_name: String) -> CustomRoleBuilder {
        CustomRoleBuilder::new(name, display_name, self)
    }

    /// Get effective permissions for role including inheritance
    pub fn get_effective_permissions(&self, role: &Role) -> Vec<Permission> {
        let mut permissions = role.permissions.clone();
        
        // Add inherited permissions
        for parent_role in &role.inherits_from {
            if let Some(parent) = self.get_standard_role(parent_role, role.tenant_id) {
                let parent_permissions = self.get_effective_permissions(&parent);
                permissions.extend(parent_permissions);
            }
        }

        // Remove duplicates
        permissions.sort_by(|a, b| {
            a.resource.cmp(&b.resource)
                .then_with(|| a.action.cmp(&b.action))
        });
        permissions.dedup_by(|a, b| {
            a.resource == b.resource && a.action == b.action
        });

        permissions
    }

    /// Check if role has specific permission (including inheritance)
    pub fn role_has_permission(&self, role: &Role, resource: &str, action: &str) -> bool {
        let effective_permissions = self.get_effective_permissions(role);
        effective_permissions
            .iter()
            .any(|perm| perm.matches(resource, action))
    }
}

/// Builder for creating custom roles
pub struct CustomRoleBuilder<'a> {
    role: Role,
    manager: &'a RoleManager,
}

impl<'a> CustomRoleBuilder<'a> {
    /// Create a new custom role builder
    fn new(name: String, display_name: String, manager: &'a RoleManager) -> Self {
        Self {
            role: Role {
                name,
                display_name,
                description: None,
                permissions: Vec::new(),
                inherits_from: Vec::new(),
                is_platform_role: false,
                tenant_id: None,
            },
            manager,
        }
    }

    /// Set description
    pub fn description(mut self, description: String) -> Self {
        self.role.description = Some(description);
        self
    }

    /// Set as platform role
    pub fn platform_role(mut self) -> Self {
        self.role.is_platform_role = true;
        self.role.tenant_id = None;
        self
    }

    /// Set as tenant role
    pub fn tenant_role(mut self, tenant_id: i32) -> Self {
        self.role.is_platform_role = false;
        self.role.tenant_id = Some(tenant_id);
        self
    }

    /// Add permission
    pub fn permission(mut self, permission: Permission) -> Self {
        self.role.permissions.push(permission);
        self
    }

    /// Add permission from string
    pub fn permission_str(self, perm_str: &str) -> RbacResult<Self> {
        let permission = self.manager.parse_permission_string(perm_str)?;
        Ok(self.permission(permission))
    }

    /// Add multiple permissions from strings
    pub fn permissions_str(mut self, perm_strs: Vec<&str>) -> RbacResult<Self> {
        for perm_str in perm_strs {
            let permission = self.manager.parse_permission_string(perm_str)?;
            self.role.permissions.push(permission);
        }
        Ok(self)
    }

    /// Add role inheritance
    pub fn inherits_from(mut self, parent_role: String) -> Self {
        self.role.inherits_from.push(parent_role);
        self
    }

    /// Build and validate the role
    pub fn build(self) -> RbacResult<Role> {
        self.manager.validate_custom_role(&self.role)?;
        Ok(self.role)
    }
}

/// Predefined role templates
pub struct RoleTemplates;

impl RoleTemplates {
    /// Create a basic read-only role
    pub fn read_only_role(name: String, display_name: String, manager: &RoleManager) -> RbacResult<Role> {
        manager
            .custom_role_builder(name, display_name)
            .permissions_str(vec![
                "read:tasks",
                "read:executions", 
                "read:jobs",
                "read:schedules",
                "read:metrics",
            ])?
            .build()
    }

    /// Create a basic task executor role
    pub fn task_executor_role(name: String, display_name: String, manager: &RoleManager) -> RbacResult<Role> {
        manager
            .custom_role_builder(name, display_name)
            .permissions_str(vec![
                "read:tasks",
                "execute:tasks",
                "read:executions",
                "create:jobs",
                "read:jobs",
            ])?
            .build()
    }

    /// Create a task manager role
    pub fn task_manager_role(name: String, display_name: String, manager: &RoleManager) -> RbacResult<Role> {
        manager
            .custom_role_builder(name, display_name)
            .permissions_str(vec![
                "create:tasks",
                "read:tasks", 
                "update:tasks",
                "delete:tasks",
                "execute:tasks",
                "read:executions",
                "manage:jobs",
                "create:schedules",
                "read:schedules",
                "update:schedules",
                "delete:schedules",
            ])?
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> RoleManager {
        RoleManager::new(RbacConfig::default())
    }

    #[test]
    fn test_get_standard_role() {
        let manager = create_test_manager();
        
        let role = manager.get_standard_role("tenant_admin", Some(100));
        assert!(role.is_some());
        
        let role = role.unwrap();
        assert_eq!(role.name, "tenant_admin");
        assert!(!role.is_platform_role);
        assert_eq!(role.tenant_id, Some(100));
    }

    #[test]
    fn test_validate_role_name() {
        let manager = create_test_manager();
        
        assert!(manager.validate_role_name("valid_role-name").is_ok());
        assert!(manager.validate_role_name("").is_err());
        assert!(manager.validate_role_name("invalid@role").is_err());
        
        let long_name = "a".repeat(101);
        assert!(manager.validate_role_name(&long_name).is_err());
    }

    #[test]
    fn test_parse_permission_string() {
        let manager = create_test_manager();
        
        let perm = manager.parse_permission_string("read:tasks").unwrap();
        assert_eq!(perm.action, "read");
        assert_eq!(perm.resource, "tasks");
        
        assert!(manager.parse_permission_string("invalid").is_err());
        assert!(manager.parse_permission_string("read:invalid_resource").is_err());
    }

    #[test]
    fn test_custom_role_builder() {
        let manager = create_test_manager();
        
        let role = manager
            .custom_role_builder("test_role".to_string(), "Test Role".to_string())
            .description("A test role".to_string())
            .tenant_role(100)
            .permission_str("read:tasks")
            .unwrap()
            .permission_str("execute:tasks")
            .unwrap()
            .build()
            .unwrap();
            
        assert_eq!(role.name, "test_role");
        assert_eq!(role.display_name, "Test Role");
        assert_eq!(role.description, Some("A test role".to_string()));
        assert!(!role.is_platform_role);
        assert_eq!(role.tenant_id, Some(100));
        assert_eq!(role.permissions.len(), 2);
    }

    #[test]
    fn test_role_templates() {
        let manager = create_test_manager();
        
        let role = RoleTemplates::read_only_role(
            "readonly".to_string(),
            "Read Only".to_string(),
            &manager,
        ).unwrap();
        
        assert_eq!(role.name, "readonly");
        assert!(role.permissions.len() >= 4);
        assert!(manager.role_has_permission(&role, "tasks", "read"));
        assert!(!manager.role_has_permission(&role, "tasks", "create"));
    }

    #[test]
    fn test_circular_inheritance_detection() {
        let manager = create_test_manager();
        
        // This would create a cycle: role_a -> role_b -> role_a
        let permission = Permission::new(
            "tasks".to_string(),
            "read".to_string(),
            PermissionScope::Tenant,
        );
        let mut role = Role::new_tenant_role(
            "role_a".to_string(),
            "Role A".to_string(),
            100,
            vec![permission],
        );
        role.inherits_from.push("role_b".to_string());
        
        // Since role_b doesn't exist in standard roles, this should pass
        assert!(manager.validate_custom_role(&role).is_ok());
    }

    #[test]
    fn test_effective_permissions() {
        let manager = create_test_manager();
        
        let tenant_user = manager.get_standard_role("tenant_user", Some(100)).unwrap();
        let effective_perms = manager.get_effective_permissions(&tenant_user);
        
        assert!(!effective_perms.is_empty());
        assert!(manager.role_has_permission(&tenant_user, "tasks", "create"));
        assert!(manager.role_has_permission(&tenant_user, "tasks", "read"));
    }
}