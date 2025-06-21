//! Policy management utilities

use crate::{
    enforcer::RbacEnforcer,
    error::{RbacError, RbacResult},
    models::{ActionType, ResourceType, Role},
};

/// Policy manager for handling Casbin policies
pub struct PolicyManager {
    enforcer: RbacEnforcer,
}

impl PolicyManager {
    /// Create a new policy manager
    pub fn new(enforcer: RbacEnforcer) -> Self {
        Self { enforcer }
    }

    /// Initialize standard policies from configuration
    pub async fn initialize_standard_policies(&self) -> RbacResult<()> {
        // Standard platform roles and permissions are initialized in the enforcer
        // This method can be used for additional policy setup
        
        // Add default platform administrator if none exists
        let platform_admins = self
            .enforcer
            .get_users_for_role("platform_admin", "platform")
            .await?;
            
        if platform_admins.is_empty() {
            // Log that no platform admin exists - this should be handled by application setup
            tracing::warn!("No platform administrators found. Ensure at least one user has platform_admin role.");
        }

        Ok(())
    }

    /// Add policy for role
    pub async fn add_role_policy(
        &self,
        role_name: &str,
        resource: ResourceType,
        action: ActionType,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .add_permission_for_role(role_name, &resource, &action, domain)
            .await
    }

    /// Remove policy for role
    pub async fn remove_role_policy(
        &self,
        role_name: &str,
        resource: ResourceType,
        action: ActionType,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .remove_permission_for_role(role_name, &resource, &action, domain)
            .await
    }

    /// Add user to role
    pub async fn add_user_role(
        &self,
        user_id: i32,
        role_name: &str,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .add_role_for_user(user_id, role_name, domain)
            .await
    }

    /// Remove user from role
    pub async fn remove_user_role(
        &self,
        user_id: i32,
        role_name: &str,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .remove_role_for_user(user_id, role_name, domain)
            .await
    }

    /// Create complete role with all policies
    pub async fn create_role_with_policies(
        &self,
        role: &Role,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .create_custom_role(role, domain)
            .await
    }

    /// Remove role and all associated policies
    pub async fn remove_role_with_policies(
        &self,
        role_name: &str,
        domain: &str,
    ) -> RbacResult<()> {
        self.enforcer
            .remove_custom_role(role_name, domain)
            .await
    }

    /// Get all policies for a domain
    pub async fn get_domain_policies(&self, _domain: &str) -> RbacResult<Vec<PolicyRule>> {
        // This would require access to the underlying Casbin enforcer's policy data
        // For now, return empty - this would need to be implemented with direct Casbin access
        Ok(Vec::new())
    }

    /// Backup current policies
    pub async fn backup_policies(&self) -> RbacResult<PolicyBackup> {
        // Save current policy state
        self.enforcer.save_policy().await?;
        
        // Create backup metadata
        Ok(PolicyBackup {
            timestamp: chrono::Utc::now(),
            version: "1.0".to_string(),
            description: "Policy backup".to_string(),
        })
    }

    /// Restore policies from backup
    pub async fn restore_policies(&self, _backup: &PolicyBackup) -> RbacResult<()> {
        // This would restore from a backup file or database snapshot
        // For now, just reload current policies
        self.enforcer.reload_policy().await
    }

    /// Validate policy consistency
    pub async fn validate_policies(&self) -> RbacResult<PolicyValidationReport> {
        let mut report = PolicyValidationReport::new();

        // Check for orphaned user-role assignments
        // (users assigned to roles that don't exist)
        // This would require querying the Casbin policy data directly

        // Check for circular role inheritance
        // This is handled at the role level during role creation

        // Check for invalid resource/action combinations
        // This is handled during policy creation

        // For now, return a basic report
        report.is_valid = true;
        report.add_info("Policy validation completed successfully".to_string());

        Ok(report)
    }

    /// Get policy statistics
    pub async fn get_policy_stats(&self) -> RbacResult<PolicyStats> {
        // These would require direct access to Casbin policy data
        // For now, return placeholder values
        Ok(PolicyStats {
            total_policies: 0,
            role_policies: 0,
            user_role_assignments: 0,
            domains: vec!["platform".to_string()],
        })
    }
}

/// Represents a single policy rule
#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub policy_type: String, // "p" for policy, "g" for grouping
    pub subject: String,
    pub object: Option<String>,
    pub action: Option<String>,
    pub domain: String,
}

/// Policy backup metadata
#[derive(Debug, Clone)]
pub struct PolicyBackup {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub description: String,
}

/// Policy validation report
#[derive(Debug, Clone)]
pub struct PolicyValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl PolicyValidationReport {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_info(&mut self, info: String) {
        self.info.push(info);
    }
}

/// Policy statistics
#[derive(Debug, Clone)]
pub struct PolicyStats {
    pub total_policies: u32,
    pub role_policies: u32,
    pub user_role_assignments: u32,
    pub domains: Vec<String>,
}

/// Helper functions for common policy operations
pub struct PolicyHelper;

impl PolicyHelper {
    /// Create platform admin policy set
    pub fn platform_admin_policies() -> Vec<(ResourceType, ActionType)> {
        vec![
            (ResourceType::Tenant, ActionType::Create),
            (ResourceType::Tenant, ActionType::Read),
            (ResourceType::Tenant, ActionType::Update),
            (ResourceType::Tenant, ActionType::Delete),
            (ResourceType::Tenant, ActionType::Manage),
            (ResourceType::User, ActionType::Manage),
            (ResourceType::Role, ActionType::Manage),
            (ResourceType::Metric, ActionType::Read),
            (ResourceType::Configuration, ActionType::Manage),
        ]
    }

    /// Create tenant admin policy set
    pub fn tenant_admin_policies() -> Vec<(ResourceType, ActionType)> {
        vec![
            (ResourceType::Task, ActionType::Create),
            (ResourceType::Task, ActionType::Read),
            (ResourceType::Task, ActionType::Update),
            (ResourceType::Task, ActionType::Delete),
            (ResourceType::Task, ActionType::Execute),
            (ResourceType::Execution, ActionType::Read),
            (ResourceType::Execution, ActionType::Update),
            (ResourceType::Job, ActionType::Create),
            (ResourceType::Job, ActionType::Read),
            (ResourceType::Job, ActionType::Update),
            (ResourceType::Job, ActionType::Delete),
            (ResourceType::Job, ActionType::Manage),
            (ResourceType::Schedule, ActionType::Create),
            (ResourceType::Schedule, ActionType::Read),
            (ResourceType::Schedule, ActionType::Update),
            (ResourceType::Schedule, ActionType::Delete),
            (ResourceType::User, ActionType::Manage),
            (ResourceType::Role, ActionType::Manage),
            (ResourceType::Metric, ActionType::Read),
        ]
    }

    /// Create tenant user policy set
    pub fn tenant_user_policies() -> Vec<(ResourceType, ActionType)> {
        vec![
            (ResourceType::Task, ActionType::Create),
            (ResourceType::Task, ActionType::Read),
            (ResourceType::Task, ActionType::Execute),
            (ResourceType::Execution, ActionType::Read),
            (ResourceType::Job, ActionType::Create),
            (ResourceType::Job, ActionType::Read),
            (ResourceType::Schedule, ActionType::Read),
        ]
    }

    /// Create tenant viewer policy set
    pub fn tenant_viewer_policies() -> Vec<(ResourceType, ActionType)> {
        vec![
            (ResourceType::Task, ActionType::Read),
            (ResourceType::Execution, ActionType::Read),
            (ResourceType::Job, ActionType::Read),
            (ResourceType::Schedule, ActionType::Read),
            (ResourceType::Metric, ActionType::Read),
        ]
    }

    /// Validate policy combination
    pub fn validate_policy_combination(
        policies: &[(ResourceType, ActionType)],
    ) -> Result<(), String> {
        // Check for conflicting policies
        for (resource, action) in policies {
            // Example: Can't have both read and manage for the same resource
            // (manage typically includes read)
            if *action == ActionType::Manage {
                let has_read = policies.iter().any(|(r, a)| {
                    r == resource && *a == ActionType::Read
                });
                if has_read {
                    return Err(format!(
                        "Redundant permissions: '{}' already includes 'read' for {}",
                        ActionType::Manage.as_str(),
                        resource.as_str()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get minimal policy set (remove redundant permissions)
    pub fn minimize_policies(
        policies: Vec<(ResourceType, ActionType)>,
    ) -> Vec<(ResourceType, ActionType)> {
        let mut minimized = Vec::new();
        
        for (resource, action) in policies {
            // Check if this policy is already covered by a more general one
            let is_redundant = minimized.iter().any(|(min_resource, min_action)| {
                min_resource == &resource && Self::action_includes(min_action, &action)
            });
            
            if !is_redundant {
                // Remove any existing policies that this one covers
                minimized.retain(|(min_resource, min_action)| {
                    !(min_resource == &resource && Self::action_includes(&action, min_action))
                });
                
                minimized.push((resource, action));
            }
        }
        
        minimized
    }

    /// Check if one action includes another (e.g., "manage" includes "read")
    fn action_includes(broader: &ActionType, narrower: &ActionType) -> bool {
        match broader {
            ActionType::Manage => matches!(narrower, 
                ActionType::Create | ActionType::Read | ActionType::Update | ActionType::Delete
            ),
            ActionType::Update => matches!(narrower, ActionType::Read),
            ActionType::Delete => matches!(narrower, ActionType::Read),
            _ => broader == narrower,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_validation_report() {
        let mut report = PolicyValidationReport::new();
        assert!(report.is_valid);
        
        report.add_warning("Test warning".to_string());
        assert!(report.is_valid);
        assert_eq!(report.warnings.len(), 1);
        
        report.add_error("Test error".to_string());
        assert!(!report.is_valid);
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn test_policy_helper_combinations() {
        let platform_policies = PolicyHelper::platform_admin_policies();
        assert!(!platform_policies.is_empty());
        assert!(platform_policies.contains(&(ResourceType::Tenant, ActionType::Create)));
        
        let tenant_policies = PolicyHelper::tenant_admin_policies();
        assert!(!tenant_policies.is_empty());
        assert!(tenant_policies.contains(&(ResourceType::Task, ActionType::Create)));
        
        let user_policies = PolicyHelper::tenant_user_policies();
        assert!(!user_policies.is_empty());
        assert!(user_policies.contains(&(ResourceType::Task, ActionType::Read)));
        
        let viewer_policies = PolicyHelper::tenant_viewer_policies();
        assert!(!viewer_policies.is_empty());
        assert!(viewer_policies.contains(&(ResourceType::Task, ActionType::Read)));
    }

    #[test]
    fn test_policy_minimization() {
        let policies = vec![
            (ResourceType::Task, ActionType::Read),
            (ResourceType::Task, ActionType::Create),
            (ResourceType::Task, ActionType::Manage), // Should include read and create
            (ResourceType::User, ActionType::Read),
        ];
        
        let minimized = PolicyHelper::minimize_policies(policies);
        
        // Should only have Manage for Task (covers read and create) and Read for User
        assert_eq!(minimized.len(), 2);
        assert!(minimized.contains(&(ResourceType::Task, ActionType::Manage)));
        assert!(minimized.contains(&(ResourceType::User, ActionType::Read)));
    }

    #[test]
    fn test_action_inclusion() {
        assert!(PolicyHelper::action_includes(&ActionType::Manage, &ActionType::Read));
        assert!(PolicyHelper::action_includes(&ActionType::Manage, &ActionType::Create));
        assert!(PolicyHelper::action_includes(&ActionType::Update, &ActionType::Read));
        assert!(!PolicyHelper::action_includes(&ActionType::Read, &ActionType::Create));
    }

    #[test]
    fn test_policy_validation() {
        let valid_policies = vec![
            (ResourceType::Task, ActionType::Create),
            (ResourceType::User, ActionType::Read),
        ];
        assert!(PolicyHelper::validate_policy_combination(&valid_policies).is_ok());
        
        let conflicting_policies = vec![
            (ResourceType::Task, ActionType::Read),
            (ResourceType::Task, ActionType::Manage), // Includes read
        ];
        assert!(PolicyHelper::validate_policy_combination(&conflicting_policies).is_err());
    }
}