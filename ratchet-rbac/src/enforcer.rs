//! RBAC Enforcer using Casbin

use casbin::{Enforcer, MgmtApi, RbacApi, CoreApi};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    adapter::SeaOrmAdapter,
    auth::AuthContext,
    config::RbacConfig,
    error::{RbacError, RbacResult},
    models::{ActionType, ResourceType, Role, Tenant},
};

/// RBAC Enforcer for authorization decisions
#[derive(Clone)]
pub struct RbacEnforcer {
    enforcer: Arc<RwLock<Enforcer>>,
    config: RbacConfig,
}

impl RbacEnforcer {
    /// Create a new RBAC enforcer
    pub async fn new(db: DatabaseConnection, config: RbacConfig) -> RbacResult<Self> {
        let adapter = SeaOrmAdapter::new(db);
        
        // Create Casbin enforcer with model and adapter
        let model_text = config.get_model_config();
        let model = casbin::DefaultModel::from_str(&model_text).await.map_err(|e| RbacError::Casbin(e))?;
        let mut enforcer = Enforcer::new(model, adapter).await.map_err(|e| RbacError::Casbin(e))?;

        // Load policy from database
        enforcer.load_policy().await.map_err(|e| RbacError::Casbin(e))?;

        // Initialize with standard roles if database is empty
        let policy_count = enforcer.get_policy().len();
        if policy_count == 0 {
            Self::initialize_standard_policies(&mut enforcer, &config).await?;
        }

        Ok(Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
            config,
        })
    }

    /// Check if user has permission for a specific action on a resource
    pub async fn check_permission(
        &self,
        auth_context: &AuthContext,
        resource: &ResourceType,
        action: &ActionType,
        tenant_id: Option<i32>,
    ) -> RbacResult<bool> {
        let enforcer = self.enforcer.read().await;
        
        let subject = format!("user_{}", auth_context.user_id);
        let object = resource.as_str();
        let action_str = action.as_str();
        
        // Check platform permissions first
        let platform_result = enforcer
            .enforce((subject.clone(), object, action_str, "platform"))
            .map_err(|e| RbacError::Casbin(e))?;
            
        if platform_result {
            return Ok(true);
        }
        
        // Check tenant permissions if tenant_id provided
        if let Some(tid) = tenant_id {
            let domain = format!("tenant_{}", tid);
            let tenant_result = enforcer
                .enforce((subject, object, action_str, domain))
                .map_err(|e| RbacError::Casbin(e))?;
                
            return Ok(tenant_result);
        }
        
        Ok(false)
    }

    /// Check if user has any permission in a tenant
    pub async fn check_tenant_access(
        &self,
        user_id: i32,
        tenant_id: i32,
    ) -> RbacResult<bool> {
        let enforcer = self.enforcer.read().await;
        let subject = format!("user_{}", user_id);
        let domain = format!("tenant_{}", tenant_id);
        
        // Check if user has any roles in this tenant
        let roles = enforcer.get_roles_for_user(&subject, Some(&domain));
            
        Ok(!roles.is_empty())
    }

    /// Add role to user for a specific domain (platform or tenant)
    pub async fn add_role_for_user(
        &self,
        user_id: i32,
        role: &str,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        let subject = format!("user_{}", user_id);
        
        enforcer
            .add_role_for_user(&subject, role, Some(domain))
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        Ok(())
    }

    /// Remove role from user for a specific domain
    pub async fn remove_role_for_user(
        &self,
        user_id: i32,
        role: &str,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        let subject = format!("user_{}", user_id);
        
        enforcer
            .delete_role_for_user(&subject, role, Some(domain))
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        Ok(())
    }

    /// Get all roles for a user in a domain
    pub async fn get_roles_for_user(
        &self,
        user_id: i32,
        domain: &str,
    ) -> RbacResult<Vec<String>> {
        let enforcer = self.enforcer.read().await;
        let subject = format!("user_{}", user_id);
        
        let roles = enforcer.get_roles_for_user(&subject, Some(domain));
            
        Ok(roles)
    }

    /// Get all users with a specific role in a domain
    pub async fn get_users_for_role(
        &self,
        role: &str,
        domain: &str,
    ) -> RbacResult<Vec<i32>> {
        let enforcer = self.enforcer.read().await;
        
        let users = enforcer.get_users_for_role(role, Some(domain));
            
        // Convert user strings back to IDs
        let user_ids: Vec<i32> = users
            .into_iter()
            .filter_map(|user| {
                user.strip_prefix("user_")
                    .and_then(|id_str| id_str.parse().ok())
            })
            .collect();
            
        Ok(user_ids)
    }

    /// Add permission for a role in a domain
    pub async fn add_permission_for_role(
        &self,
        role: &str,
        resource: &ResourceType,
        action: &ActionType,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        
        let policy = vec![
            role.to_string(),
            resource.as_str().to_string(),
            action.as_str().to_string(),
            domain.to_string(),
        ];
        
        enforcer
            .add_named_policy("p", policy)
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        Ok(())
    }

    /// Remove permission for a role in a domain
    pub async fn remove_permission_for_role(
        &self,
        role: &str,
        resource: &ResourceType,
        action: &ActionType,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        
        let policy = vec![
            role.to_string(),
            resource.as_str().to_string(),
            action.as_str().to_string(),
            domain.to_string(),
        ];
        
        enforcer
            .remove_named_policy("p", policy)
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        Ok(())
    }

    /// Create a custom role with permissions
    pub async fn create_custom_role(
        &self,
        role: &Role,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        
        // Add permissions for the role
        for permission in &role.permissions {
            let policy = vec![
                role.name.clone(),
                permission.resource.clone(),
                permission.action.clone(),
                domain.to_string(),
            ];
            
            enforcer
                .add_named_policy("p", policy)
                .await
                .map_err(|e| RbacError::Casbin(e))?;
        }
        
        // Add role inheritance if specified
        for parent_role in &role.inherits_from {
            enforcer
                .add_role_for_user(&role.name, parent_role, Some(domain))
                .await
                .map_err(|e| RbacError::Casbin(e))?;
        }
        
        Ok(())
    }

    /// Remove a custom role and all its policies
    pub async fn remove_custom_role(
        &self,
        role_name: &str,
        domain: &str,
    ) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        
        // Remove all policies for this role
        enforcer
            .remove_filtered_named_policy("p", 0, vec![role_name.to_string(), domain.to_string()])
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        // Remove role inheritance
        enforcer
            .remove_filtered_named_grouping_policy("g", 0, vec![role_name.to_string(), domain.to_string()])
            .await
            .map_err(|e| RbacError::Casbin(e))?;
            
        Ok(())
    }

    /// Get all permissions for a role in a domain
    pub async fn get_permissions_for_role(
        &self,
        role: &str,
        domain: &str,
    ) -> RbacResult<Vec<(ResourceType, ActionType)>> {
        let enforcer = self.enforcer.read().await;
        
        let policies = enforcer
            .get_named_policy("p")
            .into_iter()
            .filter(|policy| {
                policy.len() >= 4 
                    && policy[0] == role 
                    && policy[3] == domain
            })
            .filter_map(|policy| {
                if policy.len() >= 3 {
                    let resource = Self::parse_resource_type(&policy[1])?;
                    let action = Self::parse_action_type(&policy[2])?;
                    Some((resource, action))
                } else {
                    None
                }
            })
            .collect();
            
        Ok(policies)
    }

    /// Initialize standard policies from configuration
    async fn initialize_standard_policies(
        enforcer: &mut Enforcer,
        config: &RbacConfig,
    ) -> RbacResult<()> {
        // Add platform roles
        for (role_name, role_def) in &config.standard_roles {
            if role_def.is_platform_role {
                for permission_str in &role_def.permissions {
                    if let Some((action, resource)) = permission_str.split_once(':') {
                        let policy = vec![
                            role_name.clone(),
                            resource.to_string(),
                            action.to_string(),
                            "platform".to_string(),
                        ];
                        
                        enforcer
                            .add_named_policy("p", policy)
                            .await
                            .map_err(|e| RbacError::Casbin(e))?;
                    }
                }
            }
        }
        
        // Save policies to database
        enforcer.save_policy().await.map_err(|e| RbacError::Casbin(e))?;
        
        Ok(())
    }

    /// Parse resource type from string
    fn parse_resource_type(resource_str: &str) -> Option<ResourceType> {
        match resource_str {
            "tasks" => Some(ResourceType::Task),
            "executions" => Some(ResourceType::Execution),
            "jobs" => Some(ResourceType::Job),
            "schedules" => Some(ResourceType::Schedule),
            "users" => Some(ResourceType::User),
            "roles" => Some(ResourceType::Role),
            "tenants" => Some(ResourceType::Tenant),
            "metrics" => Some(ResourceType::Metric),
            "configurations" => Some(ResourceType::Configuration),
            "api_keys" => Some(ResourceType::ApiKey),
            "sessions" => Some(ResourceType::Session),
            _ => None,
        }
    }

    /// Parse action type from string
    fn parse_action_type(action_str: &str) -> Option<ActionType> {
        match action_str {
            "create" => Some(ActionType::Create),
            "read" => Some(ActionType::Read),
            "update" => Some(ActionType::Update),
            "delete" => Some(ActionType::Delete),
            "execute" => Some(ActionType::Execute),
            "manage" => Some(ActionType::Manage),
            "list" => Some(ActionType::List),
            _ => None,
        }
    }

    /// Save current policy state to database
    pub async fn save_policy(&self) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        enforcer.save_policy().await.map_err(|e| RbacError::Casbin(e))?;
        Ok(())
    }

    /// Reload policy from database
    pub async fn reload_policy(&self) -> RbacResult<()> {
        let mut enforcer = self.enforcer.write().await;
        enforcer.load_policy().await.map_err(|e| RbacError::Casbin(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: MockDatabase is not available in this version of SeaORM
    use crate::models::{ActionType, ResourceType};

    #[test]
    fn test_config_creation() {
        // Test basic configuration without database dependency
        let config = RbacConfig::default();
        assert!(!config.get_model_config().is_empty());
    }

    #[test]
    fn test_resource_type_parsing() {
        assert!(matches!(
            RbacEnforcer::parse_resource_type("tasks"),
            Some(ResourceType::Task)
        ));
        assert!(matches!(
            RbacEnforcer::parse_resource_type("executions"),
            Some(ResourceType::Execution)
        ));
        assert!(RbacEnforcer::parse_resource_type("invalid").is_none());
    }

    #[test]
    fn test_action_type_parsing() {
        assert!(matches!(
            RbacEnforcer::parse_action_type("create"),
            Some(ActionType::Create)
        ));
        assert!(matches!(
            RbacEnforcer::parse_action_type("read"),
            Some(ActionType::Read)
        ));
        assert!(RbacEnforcer::parse_action_type("invalid").is_none());
    }
}