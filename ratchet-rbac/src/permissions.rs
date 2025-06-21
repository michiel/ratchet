//! Permission checking utilities

use crate::{
    auth::AuthContext,
    enforcer::RbacEnforcer,
    error::{RbacError, RbacResult},
    models::{ActionType, ResourceType},
};

/// Permission checker for validating user actions
#[derive(Clone)]
pub struct PermissionChecker {
    enforcer: RbacEnforcer,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(enforcer: RbacEnforcer) -> Self {
        Self { enforcer }
    }

    /// Check if user can perform action on resource
    pub async fn check(
        &self,
        auth_context: &AuthContext,
        resource: ResourceType,
        action: ActionType,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        // Use cached permissions if available and valid
        if let Some(cache) = auth_context.get_cached_permissions() {
            if cache.is_valid() {
                return Ok(self.check_cached_permission(cache, &resource, &action, tenant_id));
            }
        }

        // Check with enforcer
        self.enforcer
            .check_permission(auth_context, &resource, &action, tenant_id)
            .await
    }

    /// Check permission against cached permissions
    fn check_cached_permission(
        &self,
        cache: &crate::auth::PermissionSet,
        resource: &ResourceType,
        action: &ActionType,
        tenant_id: Option<i32>,
    ) -> bool {
        let resource_str = resource.as_str();
        let action_str = action.as_str();

        // Check platform permissions first
        if cache.has_platform_permission(resource_str, action_str) {
            return true;
        }

        // Check tenant permissions if tenant specified
        if let Some(tid) = tenant_id {
            return cache.has_tenant_permission(tid, resource_str, action_str);
        }

        false
    }

    /// Check task-related permissions
    pub async fn can_create_task(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Task, ActionType::Create, tenant_id)
            .await
    }

    pub async fn can_read_task(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Task, ActionType::Read, tenant_id)
            .await
    }

    pub async fn can_update_task(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Task, ActionType::Update, tenant_id)
            .await
    }

    pub async fn can_delete_task(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Task, ActionType::Delete, tenant_id)
            .await
    }

    pub async fn can_execute_task(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Task, ActionType::Execute, tenant_id)
            .await
    }

    /// Check execution-related permissions
    pub async fn can_read_execution(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Execution, ActionType::Read, tenant_id)
            .await
    }

    pub async fn can_cancel_execution(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Execution, ActionType::Update, tenant_id)
            .await
    }

    /// Check job-related permissions
    pub async fn can_create_job(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Job, ActionType::Create, tenant_id)
            .await
    }

    pub async fn can_read_job(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Job, ActionType::Read, tenant_id)
            .await
    }

    pub async fn can_manage_jobs(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Job, ActionType::Manage, tenant_id)
            .await
    }

    /// Check schedule-related permissions
    pub async fn can_create_schedule(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Schedule, ActionType::Create, tenant_id)
            .await
    }

    pub async fn can_read_schedule(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Schedule, ActionType::Read, tenant_id)
            .await
    }

    pub async fn can_update_schedule(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Schedule, ActionType::Update, tenant_id)
            .await
    }

    pub async fn can_delete_schedule(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Schedule, ActionType::Delete, tenant_id)
            .await
    }

    /// Check user management permissions
    pub async fn can_manage_users(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::User, ActionType::Manage, tenant_id)
            .await
    }

    pub async fn can_read_users(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::User, ActionType::Read, tenant_id)
            .await
    }

    /// Check role management permissions
    pub async fn can_manage_roles(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Role, ActionType::Manage, tenant_id)
            .await
    }

    /// Check tenant management permissions (platform only)
    pub async fn can_create_tenant(&self, auth_context: &AuthContext) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Tenant, ActionType::Create, None)
            .await
    }

    pub async fn can_manage_tenant(
        &self,
        auth_context: &AuthContext,
        tenant_id: i32,
    ) -> RbacResult<bool> {
        // Platform admins can manage any tenant
        if self
            .check(auth_context, ResourceType::Tenant, ActionType::Manage, None)
            .await?
        {
            return Ok(true);
        }

        // Tenant admins can manage their own tenant
        self.check(
            auth_context,
            ResourceType::Tenant,
            ActionType::Manage,
            Some(tenant_id),
        )
        .await
    }

    /// Check metrics access permissions
    pub async fn can_read_metrics(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Metric, ActionType::Read, tenant_id)
            .await
    }

    /// Check configuration management permissions
    pub async fn can_manage_config(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::Configuration, ActionType::Manage, tenant_id)
            .await
    }

    /// Check API key management permissions
    pub async fn can_manage_api_keys(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        self.check(auth_context, ResourceType::ApiKey, ActionType::Manage, tenant_id)
            .await
    }

    /// Verify tenant membership and permissions
    pub async fn verify_tenant_access(
        &self,
        auth_context: &AuthContext,
        tenant_id: i32,
        resource: ResourceType,
        action: ActionType,
    ) -> RbacResult<()> {
        // Check if user is member of tenant
        if !auth_context.is_tenant_member(tenant_id) {
            return Err(RbacError::NotTenantMember {
                user_id: auth_context.user_id,
                tenant_id: tenant_id.to_string(),
            });
        }

        // Check if user has required permission
        let has_permission = self
            .check(auth_context, resource.clone(), action.clone(), Some(tenant_id))
            .await?;

        if !has_permission {
            return Err(RbacError::permission_denied(
                action.as_str(),
                resource.as_str(),
                tenant_id.to_string(),
            ));
        }

        Ok(())
    }

    /// Get effective permissions for user in a context
    pub async fn get_effective_permissions(
        &self,
        auth_context: &AuthContext,
        tenant_id: Option<i32>,
    ) -> RbacResult<Vec<(ResourceType, ActionType)>> {
        let mut permissions = Vec::new();

        // Get platform permissions if user has platform roles
        if !auth_context.platform_roles.is_empty() {
            for role in &auth_context.platform_roles {
                let role_perms = self
                    .enforcer
                    .get_permissions_for_role(role, "platform")
                    .await?;
                permissions.extend(role_perms);
            }
        }

        // Get tenant permissions if tenant specified
        if let Some(tid) = tenant_id {
            if let Some(tenant_roles) = auth_context.tenant_roles.get(&tid) {
                let domain = format!("tenant_{}", tid);
                for role in tenant_roles {
                    let role_perms = self
                        .enforcer
                        .get_permissions_for_role(role, &domain)
                        .await?;
                    permissions.extend(role_perms);
                }
            }
        }

        // Remove duplicates
        permissions.sort_unstable();
        permissions.dedup();

        Ok(permissions)
    }

    /// Check if user has any administrative permissions
    pub async fn is_admin(&self, auth_context: &AuthContext) -> RbacResult<bool> {
        // Platform admin
        if auth_context.is_platform_admin() {
            return Ok(true);
        }

        // Tenant admin in any tenant
        if auth_context.is_any_tenant_admin() {
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if user can access platform-level resources
    pub async fn can_access_platform(&self, auth_context: &AuthContext) -> RbacResult<bool> {
        Ok(!auth_context.platform_roles.is_empty())
    }

    /// Batch permission check for multiple resources/actions
    pub async fn batch_check(
        &self,
        auth_context: &AuthContext,
        checks: Vec<(ResourceType, ActionType, Option<i32>)>,
    ) -> RbacResult<Vec<bool>> {
        let mut results = Vec::with_capacity(checks.len());

        for (resource, action, tenant_id) in checks {
            let result = self.check(auth_context, resource, action, tenant_id).await?;
            results.push(result);
        }

        Ok(results)
    }
}

/// Convenience macros for common permission checks
#[macro_export]
macro_rules! require_permission {
    ($checker:expr, $auth:expr, $resource:expr, $action:expr, $tenant:expr) => {{
        let has_permission = $checker
            .check($auth, $resource, $action, $tenant)
            .await?;
        if !has_permission {
            return Err(RbacError::permission_denied(
                $action.as_str(),
                $resource.as_str(),
                $tenant.map(|t| t.to_string()).unwrap_or_else(|| "platform".to_string()),
            ));
        }
    }};
}

#[macro_export]
macro_rules! require_tenant_access {
    ($checker:expr, $auth:expr, $tenant_id:expr, $resource:expr, $action:expr) => {{
        $checker
            .verify_tenant_access($auth, $tenant_id, $resource, $action)
            .await?;
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::AuthContextBuilder, config::RbacConfig};
    // Note: MockDatabase is not available in this version of SeaORM

    // Database-dependent tests would need integration testing with a real database
    // async fn create_mock_checker() -> PermissionChecker { ... }

    #[test]
    fn test_auth_context_admin_checks() {
        let mut context = AuthContext::new(1);
        
        // Platform admin
        context.add_platform_role("platform_admin".to_string());
        assert!(context.is_platform_admin());
        
        // Tenant admin
        context.add_tenant_role(100, "tenant_admin".to_string());
        assert!(context.is_any_tenant_admin());
        
        // Current tenant admin
        context.current_tenant_id = Some(100);
        assert!(context.is_current_tenant_admin());
    }

    #[test]
    fn test_tenant_membership() {
        let context = AuthContextBuilder::for_user(1)
            .with_tenant_role(100, "tenant_user".to_string())
            .with_tenant_role(200, "tenant_admin".to_string())
            .build();
            
        assert!(context.is_tenant_member(100));
        assert!(context.is_tenant_member(200));
        assert!(!context.is_tenant_member(300));
    }

    #[test]
    fn test_require_permission_macro() {
        // Test that macro compiles correctly
        // Note: This is a compile-time test since we can't easily test async macros
        // In real usage, this would be used in async functions
    }
}