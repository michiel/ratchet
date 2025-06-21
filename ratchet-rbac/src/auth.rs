//! Authentication context and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Permission, Tenant};

/// Authentication context for a user request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// User ID
    pub user_id: i32,
    
    /// Current tenant ID (if operating in tenant context)
    pub current_tenant_id: Option<i32>,
    
    /// Platform roles assigned to user
    pub platform_roles: Vec<String>,
    
    /// Tenant roles per tenant
    pub tenant_roles: HashMap<i32, Vec<String>>,
    
    /// User's tenant memberships
    pub tenant_memberships: Vec<i32>,
    
    /// Cached permissions (optional for performance)
    pub cached_permissions: Option<PermissionSet>,
    
    /// Session information
    pub session_id: Option<String>,
    
    /// API key information (if authenticated via API key)
    pub api_key_id: Option<i32>,
}

impl AuthContext {
    /// Create a new authentication context
    pub fn new(user_id: i32) -> Self {
        Self {
            user_id,
            current_tenant_id: None,
            platform_roles: Vec::new(),
            tenant_roles: HashMap::new(),
            tenant_memberships: Vec::new(),
            cached_permissions: None,
            session_id: None,
            api_key_id: None,
        }
    }

    /// Create context with tenant scope
    pub fn with_tenant(user_id: i32, tenant_id: i32) -> Self {
        let mut context = Self::new(user_id);
        context.current_tenant_id = Some(tenant_id);
        context.tenant_memberships.push(tenant_id);
        context
    }

    /// Add platform role
    pub fn add_platform_role(&mut self, role: String) {
        if !self.platform_roles.contains(&role) {
            self.platform_roles.push(role);
        }
    }

    /// Add tenant role for specific tenant
    pub fn add_tenant_role(&mut self, tenant_id: i32, role: String) {
        self.tenant_roles
            .entry(tenant_id)
            .or_insert_with(Vec::new)
            .push(role);
            
        // Ensure tenant membership
        if !self.tenant_memberships.contains(&tenant_id) {
            self.tenant_memberships.push(tenant_id);
        }
    }

    /// Check if user has platform role
    pub fn has_platform_role(&self, role: &str) -> bool {
        self.platform_roles.contains(&role.to_string())
    }

    /// Check if user has tenant role
    pub fn has_tenant_role(&self, tenant_id: i32, role: &str) -> bool {
        self.tenant_roles
            .get(&tenant_id)
            .map(|roles| roles.contains(&role.to_string()))
            .unwrap_or(false)
    }

    /// Check if user is member of tenant
    pub fn is_tenant_member(&self, tenant_id: i32) -> bool {
        self.tenant_memberships.contains(&tenant_id)
    }

    /// Get all roles for current tenant
    pub fn get_current_tenant_roles(&self) -> Vec<String> {
        if let Some(tenant_id) = self.current_tenant_id {
            self.tenant_roles
                .get(&tenant_id)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Get all roles across all tenants
    pub fn get_all_tenant_roles(&self) -> Vec<(i32, String)> {
        let mut roles = Vec::new();
        for (tenant_id, tenant_roles) in &self.tenant_roles {
            for role in tenant_roles {
                roles.push((*tenant_id, role.clone()));
            }
        }
        roles
    }

    /// Check if user is platform administrator
    pub fn is_platform_admin(&self) -> bool {
        self.has_platform_role("platform_admin")
    }

    /// Check if user is tenant administrator for current tenant
    pub fn is_current_tenant_admin(&self) -> bool {
        if let Some(tenant_id) = self.current_tenant_id {
            self.has_tenant_role(tenant_id, "tenant_admin")
        } else {
            false
        }
    }

    /// Check if user is tenant administrator for any tenant
    pub fn is_any_tenant_admin(&self) -> bool {
        self.tenant_roles
            .values()
            .any(|roles| roles.contains(&"tenant_admin".to_string()))
    }

    /// Set session information
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set API key information
    pub fn with_api_key(mut self, api_key_id: i32) -> Self {
        self.api_key_id = Some(api_key_id);
        self
    }

    /// Cache permissions for performance
    pub fn cache_permissions(&mut self, permissions: PermissionSet) {
        self.cached_permissions = Some(permissions);
    }

    /// Clear cached permissions
    pub fn clear_permission_cache(&mut self) {
        self.cached_permissions = None;
    }

    /// Get cached permissions
    pub fn get_cached_permissions(&self) -> Option<&PermissionSet> {
        self.cached_permissions.as_ref()
    }

    /// Convert to user subject string for Casbin
    pub fn to_casbin_subject(&self) -> String {
        format!("user_{}", self.user_id)
    }

    /// Get domain for current context (platform or tenant)
    pub fn get_domain(&self) -> String {
        if let Some(tenant_id) = self.current_tenant_id {
            format!("tenant_{}", tenant_id)
        } else {
            "platform".to_string()
        }
    }

    /// Switch context to different tenant
    pub fn switch_tenant(&mut self, tenant_id: Option<i32>) -> Result<(), String> {
        if let Some(tid) = tenant_id {
            if !self.is_tenant_member(tid) {
                return Err(format!("User is not a member of tenant {}", tid));
            }
        }
        
        self.current_tenant_id = tenant_id;
        self.clear_permission_cache(); // Invalidate cache when switching context
        Ok(())
    }
}

/// Set of permissions for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    /// Platform permissions
    pub platform_permissions: Vec<Permission>,
    
    /// Tenant permissions per tenant
    pub tenant_permissions: HashMap<i32, Vec<Permission>>,
    
    /// Timestamp when permissions were cached
    pub cached_at: chrono::DateTime<chrono::Utc>,
    
    /// TTL for cached permissions in seconds
    pub ttl_seconds: u64,
}

impl PermissionSet {
    /// Create a new permission set
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            platform_permissions: Vec::new(),
            tenant_permissions: HashMap::new(),
            cached_at: chrono::Utc::now(),
            ttl_seconds,
        }
    }

    /// Check if cached permissions are still valid
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now();
        let expiry = self.cached_at + chrono::Duration::seconds(self.ttl_seconds as i64);
        now < expiry
    }

    /// Add platform permission
    pub fn add_platform_permission(&mut self, permission: Permission) {
        self.platform_permissions.push(permission);
    }

    /// Add tenant permission
    pub fn add_tenant_permission(&mut self, tenant_id: i32, permission: Permission) {
        self.tenant_permissions
            .entry(tenant_id)
            .or_insert_with(Vec::new)
            .push(permission);
    }

    /// Check if user has specific permission in platform context
    pub fn has_platform_permission(&self, resource: &str, action: &str) -> bool {
        self.platform_permissions
            .iter()
            .any(|perm| perm.matches(resource, action))
    }

    /// Check if user has specific permission in tenant context
    pub fn has_tenant_permission(&self, tenant_id: i32, resource: &str, action: &str) -> bool {
        self.tenant_permissions
            .get(&tenant_id)
            .map(|perms| perms.iter().any(|perm| perm.matches(resource, action)))
            .unwrap_or(false)
    }

    /// Get all platform permissions
    pub fn get_platform_permissions(&self) -> &Vec<Permission> {
        &self.platform_permissions
    }

    /// Get tenant permissions
    pub fn get_tenant_permissions(&self, tenant_id: i32) -> Option<&Vec<Permission>> {
        self.tenant_permissions.get(&tenant_id)
    }
}

/// Helper to build AuthContext from database data
pub struct AuthContextBuilder {
    context: AuthContext,
}

impl AuthContextBuilder {
    /// Start building context for user
    pub fn for_user(user_id: i32) -> Self {
        Self {
            context: AuthContext::new(user_id),
        }
    }

    /// Add platform role
    pub fn with_platform_role(mut self, role: String) -> Self {
        self.context.add_platform_role(role);
        self
    }

    /// Add tenant role
    pub fn with_tenant_role(mut self, tenant_id: i32, role: String) -> Self {
        self.context.add_tenant_role(tenant_id, role);
        self
    }

    /// Set current tenant
    pub fn with_current_tenant(mut self, tenant_id: i32) -> Self {
        self.context.current_tenant_id = Some(tenant_id);
        self
    }

    /// Set session
    pub fn with_session(mut self, session_id: String) -> Self {
        self.context.session_id = Some(session_id);
        self
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key_id: i32) -> Self {
        self.context.api_key_id = Some(api_key_id);
        self
    }

    /// Build the final context
    pub fn build(self) -> AuthContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_creation() {
        let context = AuthContext::new(1);
        assert_eq!(context.user_id, 1);
        assert!(context.platform_roles.is_empty());
        assert!(context.tenant_roles.is_empty());
        assert!(context.current_tenant_id.is_none());
    }

    #[test]
    fn test_tenant_context() {
        let context = AuthContext::with_tenant(1, 100);
        assert_eq!(context.user_id, 1);
        assert_eq!(context.current_tenant_id, Some(100));
        assert!(context.is_tenant_member(100));
    }

    #[test]
    fn test_role_management() {
        let mut context = AuthContext::new(1);
        
        context.add_platform_role("platform_admin".to_string());
        assert!(context.has_platform_role("platform_admin"));
        assert!(context.is_platform_admin());
        
        context.add_tenant_role(100, "tenant_admin".to_string());
        assert!(context.has_tenant_role(100, "tenant_admin"));
        assert!(context.is_tenant_member(100));
    }

    #[test]
    fn test_tenant_switching() {
        let mut context = AuthContext::new(1);
        context.add_tenant_role(100, "tenant_user".to_string());
        context.add_tenant_role(200, "tenant_admin".to_string());
        
        // Switch to tenant 100
        assert!(context.switch_tenant(Some(100)).is_ok());
        assert_eq!(context.current_tenant_id, Some(100));
        
        // Try to switch to tenant user is not member of
        assert!(context.switch_tenant(Some(300)).is_err());
        
        // Switch back to platform
        assert!(context.switch_tenant(None).is_ok());
        assert!(context.current_tenant_id.is_none());
    }

    #[test]
    fn test_auth_context_builder() {
        let context = AuthContextBuilder::for_user(1)
            .with_platform_role("platform_monitor".to_string())
            .with_tenant_role(100, "tenant_admin".to_string())
            .with_current_tenant(100)
            .with_session("session_123".to_string())
            .build();
            
        assert_eq!(context.user_id, 1);
        assert!(context.has_platform_role("platform_monitor"));
        assert!(context.has_tenant_role(100, "tenant_admin"));
        assert_eq!(context.current_tenant_id, Some(100));
        assert_eq!(context.session_id, Some("session_123".to_string()));
    }

    #[test]
    fn test_permission_set() {
        let mut perm_set = PermissionSet::new(300);
        
        let platform_perm = crate::models::Permission::new(
            "tenants".to_string(),
            "create".to_string(),
            crate::models::PermissionScope::Platform,
        );
        
        perm_set.add_platform_permission(platform_perm);
        assert!(perm_set.has_platform_permission("tenants", "create"));
        assert!(!perm_set.has_platform_permission("tenants", "delete"));
        
        assert!(perm_set.is_valid()); // Should be valid immediately after creation
    }

    #[test]
    fn test_casbin_subject() {
        let context = AuthContext::new(42);
        assert_eq!(context.to_casbin_subject(), "user_42");
    }

    #[test]
    fn test_domain_resolution() {
        let mut context = AuthContext::new(1);
        assert_eq!(context.get_domain(), "platform");
        
        context.current_tenant_id = Some(100);
        assert_eq!(context.get_domain(), "tenant_100");
    }
}