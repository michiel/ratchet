//! Tenant management utilities

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ActiveModelTrait, ColumnTrait};
use uuid::Uuid;

use crate::{
    error::{RbacError, RbacResult},
    models::Tenant,
};

/// Tenant manager for multi-tenant operations
pub struct TenantManager {
    db: DatabaseConnection,
}

impl TenantManager {
    /// Create a new tenant manager
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new tenant
    pub async fn create_tenant(
        &self,
        name: String,
        display_name: String,
        description: Option<String>,
        created_by: Option<i32>,
    ) -> RbacResult<Tenant> {
        use ratchet_storage::seaorm::entities::tenants;
        use sea_orm::Set;

        let tenant_model = tenants::ActiveModel {
            uuid: Set(Uuid::new_v4()),
            name: Set(name.clone()),
            display_name: Set(display_name.clone()),
            description: Set(description),
            is_active: Set(true),
            created_by: Set(created_by),
            ..Default::default()
        };

        let result = tenant_model.insert(&self.db).await?;

        Ok(Tenant {
            id: result.id,
            uuid: result.uuid,
            name: result.name,
            display_name: result.display_name,
            description: result.description,
            settings: result.settings.as_object().cloned().unwrap_or_default().into_iter().map(|(k, v)| (k, v)).collect(),
            is_active: result.is_active,
            created_at: result.created_at,
            updated_at: result.updated_at,
            created_by: result.created_by,
        })
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, tenant_id: i32) -> RbacResult<Option<Tenant>> {
        use ratchet_storage::seaorm::entities::Tenants;

        let tenant = Tenants::find_by_id(tenant_id)
            .one(&self.db)
            .await?;

        if let Some(t) = tenant {
            Ok(Some(Tenant {
                id: t.id,
                uuid: t.uuid,
                name: t.name,
                display_name: t.display_name,
                description: t.description,
                settings: t.settings.as_object().cloned().unwrap_or_default().into_iter().map(|(k, v)| (k, v)).collect(),
                is_active: t.is_active,
                created_at: t.created_at,
                updated_at: t.updated_at,
                created_by: t.created_by,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get tenant by name
    pub async fn get_tenant_by_name(&self, name: &str) -> RbacResult<Option<Tenant>> {
        use ratchet_storage::seaorm::entities::{Tenants, tenants};

        let tenant = Tenants::find()
            .filter(tenants::Column::Name.eq(name))
            .one(&self.db)
            .await?;

        if let Some(t) = tenant {
            Ok(Some(Tenant {
                id: t.id,
                uuid: t.uuid,
                name: t.name,
                display_name: t.display_name,
                description: t.description,
                settings: t.settings.as_object().cloned().unwrap_or_default().into_iter().map(|(k, v)| (k, v)).collect(),
                is_active: t.is_active,
                created_at: t.created_at,
                updated_at: t.updated_at,
                created_by: t.created_by,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all active tenants
    pub async fn list_tenants(&self, only_active: bool) -> RbacResult<Vec<Tenant>> {
        use ratchet_storage::seaorm::entities::{Tenants, tenants};

        let mut query = Tenants::find();
        
        if only_active {
            query = query.filter(tenants::Column::IsActive.eq(true));
        }

        let tenants = query.all(&self.db).await?;

        Ok(tenants
            .into_iter()
            .map(|t| Tenant {
                id: t.id,
                uuid: t.uuid,
                name: t.name,
                display_name: t.display_name,
                description: t.description,
                settings: t.settings.as_object().cloned().unwrap_or_default().into_iter().map(|(k, v)| (k, v)).collect(),
                is_active: t.is_active,
                created_at: t.created_at,
                updated_at: t.updated_at,
                created_by: t.created_by,
            })
            .collect())
    }

    /// Update tenant
    pub async fn update_tenant(
        &self,
        tenant_id: i32,
        display_name: Option<String>,
        description: Option<String>,
        is_active: Option<bool>,
    ) -> RbacResult<()> {
        use ratchet_storage::seaorm::entities::{Tenants, tenants};
        use sea_orm::{Set, ActiveModelTrait};

        let tenant = Tenants::find_by_id(tenant_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| RbacError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        let mut tenant: tenants::ActiveModel = tenant.into();

        if let Some(name) = display_name {
            tenant.display_name = Set(name);
        }
        if let Some(desc) = description {
            tenant.description = Set(Some(desc));
        }
        if let Some(active) = is_active {
            tenant.is_active = Set(active);
        }

        tenant.update(&self.db).await?;
        Ok(())
    }

    /// Delete tenant (soft delete by marking inactive)
    pub async fn deactivate_tenant(&self, tenant_id: i32) -> RbacResult<()> {
        self.update_tenant(tenant_id, None, None, Some(false)).await
    }

    /// Add user to tenant
    pub async fn add_user_to_tenant(
        &self,
        user_id: i32,
        tenant_id: i32,
        added_by: Option<i32>,
    ) -> RbacResult<()> {
        use ratchet_storage::seaorm::entities::{UserTenants, user_tenants};
        use sea_orm::{Set, ActiveModelTrait};

        // Check if tenant exists and is active
        let tenant = self.get_tenant(tenant_id).await?
            .ok_or_else(|| RbacError::TenantNotFound {
                tenant_id: tenant_id.to_string(),
            })?;

        if !tenant.is_active {
            return Err(RbacError::Internal {
                message: format!("Cannot add user to inactive tenant {}", tenant_id),
            });
        }

        // Check if user is already a member
        let existing = UserTenants::find()
            .filter(user_tenants::Column::UserId.eq(user_id))
            .filter(user_tenants::Column::TenantId.eq(tenant_id))
            .one(&self.db)
            .await?;

        if existing.is_some() {
            return Ok(()); // User already a member
        }

        let user_tenant = user_tenants::ActiveModel {
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            joined_by: Set(added_by),
            ..Default::default()
        };

        user_tenant.insert(&self.db).await?;
        Ok(())
    }

    /// Remove user from tenant
    pub async fn remove_user_from_tenant(
        &self,
        user_id: i32,
        tenant_id: i32,
    ) -> RbacResult<()> {
        use ratchet_storage::seaorm::entities::{UserTenants, user_tenants, UserRoles, user_roles};

        // Remove tenant membership
        UserTenants::delete_many()
            .filter(user_tenants::Column::UserId.eq(user_id))
            .filter(user_tenants::Column::TenantId.eq(tenant_id))
            .exec(&self.db)
            .await?;

        // Remove user roles in this tenant
        UserRoles::delete_many()
            .filter(user_roles::Column::UserId.eq(user_id))
            .filter(user_roles::Column::TenantId.eq(tenant_id))
            .exec(&self.db)
            .await?;

        Ok(())
    }

    /// Get users in tenant
    pub async fn get_tenant_users(&self, tenant_id: i32) -> RbacResult<Vec<i32>> {
        use ratchet_storage::seaorm::entities::{UserTenants, user_tenants};

        let users = UserTenants::find()
            .filter(user_tenants::Column::TenantId.eq(tenant_id))
            .all(&self.db)
            .await?;

        Ok(users.into_iter().map(|ut| ut.user_id).collect())
    }

    /// Get tenants for user
    pub async fn get_user_tenants(&self, user_id: i32) -> RbacResult<Vec<i32>> {
        use ratchet_storage::seaorm::entities::{UserTenants, user_tenants};

        let tenants = UserTenants::find()
            .filter(user_tenants::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;

        Ok(tenants.into_iter().map(|ut| ut.tenant_id).collect())
    }

    /// Check if user is member of tenant
    pub async fn is_user_tenant_member(
        &self,
        user_id: i32,
        tenant_id: i32,
    ) -> RbacResult<bool> {
        use ratchet_storage::seaorm::entities::{UserTenants, user_tenants};

        let exists = UserTenants::find()
            .filter(user_tenants::Column::UserId.eq(user_id))
            .filter(user_tenants::Column::TenantId.eq(tenant_id))
            .one(&self.db)
            .await?;

        Ok(exists.is_some())
    }

    /// Get default tenant (should always exist after migration)
    pub async fn get_default_tenant(&self) -> RbacResult<Tenant> {
        self.get_tenant_by_name("default")
            .await?
            .ok_or_else(|| RbacError::Internal {
                message: "Default tenant not found".to_string(),
            })
    }

    /// Ensure user is member of default tenant (for backward compatibility)
    pub async fn ensure_user_in_default_tenant(&self, user_id: i32) -> RbacResult<()> {
        let default_tenant = self.get_default_tenant().await?;
        
        let is_member = self
            .is_user_tenant_member(user_id, default_tenant.id)
            .await?;

        if !is_member {
            self.add_user_to_tenant(user_id, default_tenant.id, None)
                .await?;
        }

        Ok(())
    }

    /// Get tenant statistics
    pub async fn get_tenant_stats(&self, tenant_id: i32) -> RbacResult<TenantStats> {
        // Get user count
        let user_count = self.get_tenant_users(tenant_id).await?.len();

        // Get resource counts (would need to query actual resource tables)
        // For now, return placeholder values
        Ok(TenantStats {
            tenant_id,
            user_count: user_count as u32,
            task_count: 0,    // TODO: Query tasks table
            execution_count: 0, // TODO: Query executions table
            job_count: 0,     // TODO: Query jobs table
            schedule_count: 0, // TODO: Query schedules table
        })
    }
}

/// Tenant statistics
#[derive(Debug, Clone)]
pub struct TenantStats {
    pub tenant_id: i32,
    pub user_count: u32,
    pub task_count: u32,
    pub execution_count: u32,
    pub job_count: u32,
    pub schedule_count: u32,
}


#[cfg(test)]
mod tests {
    use super::*;
    // Note: MockDatabase is not available in this version of SeaORM

    // Database-dependent tests would need integration testing with a real database
    // #[tokio::test] 
    // async fn test_tenant_manager_creation() { ... }

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new(
            "test-tenant".to_string(),
            "Test Tenant".to_string(),
            Some(1),
        );
        
        assert_eq!(tenant.name, "test-tenant");
        assert_eq!(tenant.display_name, "Test Tenant");
        assert_eq!(tenant.created_by, Some(1));
        assert!(tenant.is_active());
    }

    #[test]
    fn test_tenant_domain() {
        let tenant = Tenant {
            id: 123,
            uuid: Uuid::new_v4(),
            name: "test".to_string(),
            display_name: "Test".to_string(),
            description: None,
            settings: std::collections::HashMap::new(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: None,
        };
        
        assert_eq!(tenant.domain(), "tenant_123");
    }

    #[test]
    fn test_tenant_user_creation() {
        use crate::models::TenantUser;
        let tenant_user = TenantUser::new(1, 100, Some(2));
        
        assert_eq!(tenant_user.user_id, 1);
        assert_eq!(tenant_user.tenant_id, 100);
        assert_eq!(tenant_user.joined_by, Some(2));
    }
}