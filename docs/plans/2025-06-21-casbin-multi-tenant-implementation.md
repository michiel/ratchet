# Casbin Multi-Tenant RBAC Implementation Plan

## Executive Summary

This document provides a detailed implementation plan for integrating Casbin-RS to transform Ratchet into a multi-tenant platform with flexible role-based access control. The implementation will support platform operators, tenant administrators, and configurable custom roles while maintaining complete tenant isolation and backward compatibility.

**Timeline**: 5 sprints (10 weeks)  
**Risk Level**: Medium  
**Dependencies**: Database migration, Casbin-RS integration, API redesign

## Phase 1: Foundation and Casbin Integration

### Sprint 1.1: Database Schema Design and Migration

#### New Database Tables

```sql
-- Tenant management
CREATE TABLE tenants (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(255),
    description TEXT,
    settings JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by INTEGER REFERENCES users(id)
);

-- Casbin policy storage
CREATE TABLE casbin_rules (
    id SERIAL PRIMARY KEY,
    ptype VARCHAR(100) NOT NULL,
    v0 VARCHAR(100),
    v1 VARCHAR(100),
    v2 VARCHAR(100),
    v3 VARCHAR(100),
    v4 VARCHAR(100),
    v5 VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User-tenant associations
CREATE TABLE user_tenants (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    joined_by INTEGER REFERENCES users(id),
    UNIQUE(user_id, tenant_id)
);

-- Add tenant_id to existing resource tables
ALTER TABLE tasks ADD COLUMN tenant_id INTEGER REFERENCES tenants(id);
ALTER TABLE executions ADD COLUMN tenant_id INTEGER REFERENCES executions(id);
ALTER TABLE jobs ADD COLUMN tenant_id INTEGER REFERENCES tenants(id);
ALTER TABLE schedules ADD COLUMN tenant_id INTEGER REFERENCES tenants(id);

-- Indexes for performance
CREATE INDEX idx_tenants_name ON tenants(name);
CREATE INDEX idx_tenants_active ON tenants(is_active);
CREATE INDEX idx_casbin_rules_ptype ON casbin_rules(ptype);
CREATE INDEX idx_casbin_rules_subject ON casbin_rules(v0);
CREATE INDEX idx_user_tenants_user ON user_tenants(user_id);
CREATE INDEX idx_user_tenants_tenant ON user_tenants(tenant_id);
CREATE INDEX idx_tasks_tenant ON tasks(tenant_id);
CREATE INDEX idx_executions_tenant ON executions(tenant_id);
CREATE INDEX idx_jobs_tenant ON jobs(tenant_id);
CREATE INDEX idx_schedules_tenant ON schedules(tenant_id);
```

#### Migration Strategy

**Backward Compatibility Migration**:
```rust
// Migration script: 20250621_add_multi_tenant_support.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create tenants table
        manager.create_table(/* tenant table definition */).await?;
        
        // 2. Create default tenant for existing data
        manager.exec_stmt(
            Query::insert()
                .into_table(Tenants::Table)
                .columns([Tenants::Name, Tenants::DisplayName, Tenants::IsActive])
                .values_panic(["default", "Default Tenant", true])
                .to_owned()
        ).await?;
        
        // 3. Add tenant_id columns (nullable initially)
        manager.alter_table(
            Table::alter()
                .table(Tasks::Table)
                .add_column(ColumnDef::new(Tasks::TenantId).integer())
                .to_owned()
        ).await?;
        
        // 4. Set default tenant for existing resources
        manager.exec_stmt(
            Query::update()
                .table(Tasks::Table)
                .value(Tasks::TenantId, 1) // Default tenant ID
                .to_owned()
        ).await?;
        
        // 5. Make tenant_id NOT NULL after data migration
        manager.alter_table(
            Table::alter()
                .table(Tasks::Table)
                .modify_column(ColumnDef::new(Tasks::TenantId).integer().not_null())
                .to_owned()
        ).await?;
        
        // 6. Create casbin_rules table
        manager.create_table(/* casbin rules table */).await?;
        
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rollback migration
        manager.drop_table(Table::drop().table(CasbinRules::Table).to_owned()).await?;
        manager.alter_table(
            Table::alter()
                .table(Tasks::Table)
                .drop_column(Tasks::TenantId)
                .to_owned()
        ).await?;
        manager.drop_table(Table::drop().table(Tenants::Table).to_owned()).await?;
        Ok(())
    }
}
```

### Sprint 1.2: Casbin Integration and Configuration

#### Add Casbin Dependencies

```toml
# Cargo.toml additions
[dependencies]
casbin = { version = "2.1", features = ["runtime-async-std", "cached"] }
ratchet-rbac = { path = "../ratchet-rbac" } # New crate
```

#### Create ratchet-rbac Crate

```rust
// ratchet-rbac/src/lib.rs
use casbin::{Enforcer, Result as CasbinResult};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub mod adapter;
pub mod models;
pub mod permissions;
pub mod roles;

pub use adapter::SeaOrmAdapter;
pub use models::*;
pub use permissions::*;

/// Main RBAC manager
pub struct RbacManager {
    enforcer: Arc<Enforcer>,
    adapter: Arc<SeaOrmAdapter>,
}

impl RbacManager {
    pub async fn new(db: DatabaseConnection) -> CasbinResult<Self> {
        let adapter = Arc::new(SeaOrmAdapter::new(db));
        let enforcer = Arc::new(
            Enforcer::new("config/rbac_model.conf", adapter.clone()).await?
        );
        
        Ok(Self { enforcer, adapter })
    }
    
    /// Check if subject has permission for resource in tenant context
    pub async fn enforce(
        &self,
        subject: &str,
        resource: &str,
        action: &str,
        tenant: &str,
    ) -> CasbinResult<bool> {
        self.enforcer.enforce((subject, resource, action, tenant))
    }
    
    /// Add role for user in tenant context
    pub async fn add_role_for_user_in_tenant(
        &self,
        user: &str,
        role: &str,
        tenant: &str,
    ) -> CasbinResult<bool> {
        self.enforcer.add_grouping_policy(vec![user, role, tenant]).await
    }
    
    /// Remove role for user in tenant context
    pub async fn remove_role_for_user_in_tenant(
        &self,
        user: &str,
        role: &str,
        tenant: &str,
    ) -> CasbinResult<bool> {
        self.enforcer.remove_grouping_policy(vec![user, role, tenant]).await
    }
    
    /// Get all permissions for user in tenant
    pub async fn get_permissions_for_user_in_tenant(
        &self,
        user: &str,
        tenant: &str,
    ) -> Vec<Vec<String>> {
        self.enforcer.get_permissions_for_user_in_domain(user, tenant)
    }
}
```

#### Casbin Model Configuration

```ini
# config/rbac_model.conf
[request_definition]
r = sub, obj, act, dom

[policy_definition]
p = sub, obj, act, dom

[role_definition]
g = _, _, _
g2 = _, _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub, r.dom) && r.obj == p.obj && r.act == p.act && (r.dom == p.dom || p.dom == "*")
```

#### SeaORM Adapter Implementation

```rust
// ratchet-rbac/src/adapter.rs
use casbin::{Adapter, Filter, Model, Result as CasbinResult};
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait};
use async_trait::async_trait;

use crate::entities::casbin_rules;

pub struct SeaOrmAdapter {
    db: DatabaseConnection,
}

impl SeaOrmAdapter {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Adapter for SeaOrmAdapter {
    async fn load_policy(&self, m: &mut dyn Model) -> CasbinResult<()> {
        let rules = casbin_rules::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| casbin::Error::from(format!("Database error: {}", e)))?;

        for rule in rules {
            let mut tokens = vec![rule.ptype];
            if let Some(v) = rule.v0 { tokens.push(v); }
            if let Some(v) = rule.v1 { tokens.push(v); }
            if let Some(v) = rule.v2 { tokens.push(v); }
            if let Some(v) = rule.v3 { tokens.push(v); }
            if let Some(v) = rule.v4 { tokens.push(v); }
            if let Some(v) = rule.v5 { tokens.push(v); }
            
            m.add_def(&tokens[0], &tokens[0], tokens[1..].to_vec());
        }
        
        Ok(())
    }

    async fn save_policy(&self, m: &mut dyn Model) -> CasbinResult<()> {
        // Clear existing policies
        casbin_rules::Entity::delete_many()
            .exec(&self.db)
            .await
            .map_err(|e| casbin::Error::from(format!("Database error: {}", e)))?;

        // Save all policies
        for sec in vec!["p", "g"] {
            if let Some(policy) = m.get_model().get(sec) {
                for (key, ast) in policy {
                    for rule in &ast.policy {
                        let active_rule = casbin_rules::ActiveModel {
                            ptype: Set(key.clone()),
                            v0: Set(rule.get(0).cloned()),
                            v1: Set(rule.get(1).cloned()),
                            v2: Set(rule.get(2).cloned()),
                            v3: Set(rule.get(3).cloned()),
                            v4: Set(rule.get(4).cloned()),
                            v5: Set(rule.get(5).cloned()),
                            ..Default::default()
                        };
                        
                        active_rule.insert(&self.db).await
                            .map_err(|e| casbin::Error::from(format!("Database error: {}", e)))?;
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn add_policy(&self, _sec: &str, ptype: &str, rule: Vec<String>) -> CasbinResult<bool> {
        let active_rule = casbin_rules::ActiveModel {
            ptype: Set(ptype.to_string()),
            v0: Set(rule.get(0).cloned()),
            v1: Set(rule.get(1).cloned()),
            v2: Set(rule.get(2).cloned()),
            v3: Set(rule.get(3).cloned()),
            v4: Set(rule.get(4).cloned()),
            v5: Set(rule.get(5).cloned()),
            ..Default::default()
        };
        
        active_rule.insert(&self.db).await
            .map_err(|e| casbin::Error::from(format!("Database error: {}", e)))?;
        
        Ok(true)
    }

    async fn remove_policy(&self, _sec: &str, ptype: &str, rule: Vec<String>) -> CasbinResult<bool> {
        let mut query = casbin_rules::Entity::delete_many()
            .filter(casbin_rules::Column::Ptype.eq(ptype));

        for (i, value) in rule.iter().enumerate() {
            match i {
                0 => query = query.filter(casbin_rules::Column::V0.eq(value)),
                1 => query = query.filter(casbin_rules::Column::V1.eq(value)),
                2 => query = query.filter(casbin_rules::Column::V2.eq(value)),
                3 => query = query.filter(casbin_rules::Column::V3.eq(value)),
                4 => query = query.filter(casbin_rules::Column::V4.eq(value)),
                5 => query = query.filter(casbin_rules::Column::V5.eq(value)),
                _ => break,
            }
        }

        let result = query.exec(&self.db).await
            .map_err(|e| casbin::Error::from(format!("Database error: {}", e)))?;
        
        Ok(result.rows_affected > 0)
    }

    async fn remove_filtered_policy(
        &self,
        _sec: &str,
        ptype: &str,
        field_index: usize,
        field_values: Vec<String>,
    ) -> CasbinResult<bool> {
        // Implementation for filtered policy removal
        // Used for bulk operations like removing all policies for a user
        unimplemented!("Implement filtered policy removal")
    }
}
```

## Phase 2: Permission System and Role Management

### Sprint 2.1: Permission Framework

#### Define Permission System

```rust
// ratchet-rbac/src/permissions.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource types in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    Tasks,
    Executions,
    Jobs,
    Schedules,
    Users,
    Roles,
    Metrics,
    Tenants,
    Platform,
}

/// Action types that can be performed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Manage,
    Cancel,
    Retry,
}

/// Permission scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    Platform,     // Platform-wide access
    Tenant,       // Tenant-scoped access
    Own,         // User's own resources only
}

/// Complete permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub resource: Resource,
    pub action: Action,
    pub scope: Scope,
    pub conditions: Option<HashMap<String, String>>,
}

impl Permission {
    /// Convert to Casbin policy string
    pub fn to_policy_string(&self) -> String {
        format!("{}:{}:{}", 
            self.resource.to_string().to_lowercase(),
            self.action.to_string().to_lowercase(),
            self.scope.to_string().to_lowercase()
        )
    }
}

/// Permission checker for complex authorization logic
pub struct PermissionChecker {
    rbac: Arc<RbacManager>,
}

impl PermissionChecker {
    pub fn new(rbac: Arc<RbacManager>) -> Self {
        Self { rbac }
    }
    
    /// Check if user has permission for specific resource
    pub async fn check_permission(
        &self,
        user_id: i32,
        permission: &Permission,
        tenant_id: Option<i32>,
        resource_id: Option<i32>,
    ) -> Result<bool, RbacError> {
        let subject = format!("user:{}", user_id);
        let object = permission.to_policy_string();
        let action = permission.action.to_string().to_lowercase();
        let domain = match (permission.scope, tenant_id) {
            (Scope::Platform, _) => "platform".to_string(),
            (Scope::Tenant, Some(tid)) => format!("tenant:{}", tid),
            (Scope::Own, Some(tid)) => format!("tenant:{}:user:{}", tid, user_id),
            _ => return Err(RbacError::InvalidContext),
        };
        
        let allowed = self.rbac.enforce(&subject, &object, &action, &domain).await?;
        
        // Additional context-specific checks
        if allowed && permission.scope == Scope::Own && resource_id.is_some() {
            // Check if user owns the resource
            return self.check_resource_ownership(user_id, &permission.resource, resource_id.unwrap()).await;
        }
        
        Ok(allowed)
    }
    
    async fn check_resource_ownership(
        &self,
        user_id: i32,
        resource: &Resource,
        resource_id: i32,
    ) -> Result<bool, RbacError> {
        // Implementation depends on resource type
        match resource {
            Resource::Tasks => {
                // Check if user created the task or is assigned to it
                unimplemented!("Task ownership check")
            }
            Resource::Executions => {
                // Check if user initiated the execution
                unimplemented!("Execution ownership check")
            }
            _ => Ok(true), // Default allow for other resources
        }
    }
}
```

#### Standard Role Definitions

```yaml
# config/default_roles.yaml
platform_roles:
  platform_admin:
    display_name: "Platform Administrator"
    description: "Full platform administration access"
    permissions:
      - resource: "platform"
        action: "manage"
        scope: "platform"
      - resource: "tenants"
        action: "create"
        scope: "platform"
      - resource: "tenants"
        action: "read"
        scope: "platform"
      - resource: "tenants"
        action: "update"
        scope: "platform"
      - resource: "tenants"
        action: "delete"
        scope: "platform"
      - resource: "users"
        action: "manage"
        scope: "platform"
      - resource: "metrics"
        action: "read"
        scope: "platform"

  platform_monitor:
    display_name: "Platform Monitor"
    description: "Read-only platform monitoring access"
    permissions:
      - resource: "metrics"
        action: "read"
        scope: "platform"
      - resource: "tenants"
        action: "read"
        scope: "platform"

tenant_roles:
  tenant_admin:
    display_name: "Tenant Administrator"
    description: "Full administrative access within tenant"
    permissions:
      - resource: "tasks"
        action: "create"
        scope: "tenant"
      - resource: "tasks"
        action: "read"
        scope: "tenant"
      - resource: "tasks"
        action: "update"
        scope: "tenant"
      - resource: "tasks"
        action: "delete"
        scope: "tenant"
      - resource: "executions"
        action: "read"
        scope: "tenant"
      - resource: "executions"
        action: "cancel"
        scope: "tenant"
      - resource: "executions"
        action: "retry"
        scope: "tenant"
      - resource: "jobs"
        action: "create"
        scope: "tenant"
      - resource: "jobs"
        action: "read"
        scope: "tenant"
      - resource: "jobs"
        action: "update"
        scope: "tenant"
      - resource: "jobs"
        action: "cancel"
        scope: "tenant"
      - resource: "schedules"
        action: "create"
        scope: "tenant"
      - resource: "schedules"
        action: "read"
        scope: "tenant"
      - resource: "schedules"
        action: "update"
        scope: "tenant"
      - resource: "schedules"
        action: "delete"
        scope: "tenant"
      - resource: "users"
        action: "manage"
        scope: "tenant"
      - resource: "roles"
        action: "manage"
        scope: "tenant"
      - resource: "metrics"
        action: "read"
        scope: "tenant"

  tenant_user:
    display_name: "Tenant User"
    description: "Standard user access within tenant"
    permissions:
      - resource: "tasks"
        action: "create"
        scope: "tenant"
      - resource: "tasks"
        action: "read"
        scope: "tenant"
      - resource: "tasks"
        action: "update"
        scope: "own"
      - resource: "executions"
        action: "read"
        scope: "own"
      - resource: "executions"
        action: "cancel"
        scope: "own"
      - resource: "jobs"
        action: "create"
        scope: "tenant"
      - resource: "jobs"
        action: "read"
        scope: "own"
      - resource: "schedules"
        action: "read"
        scope: "tenant"

  tenant_viewer:
    display_name: "Tenant Viewer"
    description: "Read-only access within tenant"
    permissions:
      - resource: "tasks"
        action: "read"
        scope: "tenant"
      - resource: "executions"
        action: "read"
        scope: "tenant"
      - resource: "jobs"
        action: "read"
        scope: "tenant"
      - resource: "schedules"
        action: "read"
        scope: "tenant"
      - resource: "metrics"
        action: "read"
        scope: "tenant"
```

### Sprint 2.2: Role Management Implementation

#### Role Management Service

```rust
// ratchet-rbac/src/roles.rs
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub is_system_role: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub user_id: i32,
    pub role_name: String,
    pub tenant_id: Option<i32>,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub assigned_by: Option<i32>,
}

pub struct RoleManager {
    rbac: Arc<RbacManager>,
    db: DatabaseConnection,
}

impl RoleManager {
    pub fn new(rbac: Arc<RbacManager>, db: DatabaseConnection) -> Self {
        Self { rbac, db }
    }
    
    /// Load default roles from configuration
    pub async fn load_default_roles(&self) -> Result<(), RbacError> {
        let config = std::fs::read_to_string("config/default_roles.yaml")?;
        let roles: DefaultRolesConfig = serde_yaml::from_str(&config)?;
        
        // Load platform roles
        for (role_name, role_def) in roles.platform_roles {
            self.create_platform_role(&role_name, &role_def).await?;
        }
        
        // Load tenant role templates
        for (role_name, role_def) in roles.tenant_roles {
            self.create_tenant_role_template(&role_name, &role_def).await?;
        }
        
        Ok(())
    }
    
    /// Create platform-level role
    pub async fn create_platform_role(
        &self,
        role_name: &str,
        role_def: &RoleDefinition,
    ) -> Result<(), RbacError> {
        // Add role policies to Casbin
        for permission in &role_def.permissions {
            let policy = vec![
                role_name.to_string(),
                permission.to_policy_string(),
                permission.action.to_string().to_lowercase(),
                "platform".to_string(),
            ];
            
            self.rbac.add_policy("p", &policy).await?;
        }
        
        Ok(())
    }
    
    /// Assign role to user in tenant context
    pub async fn assign_role_to_user(
        &self,
        user_id: i32,
        role_name: &str,
        tenant_id: Option<i32>,
        assigned_by: Option<i32>,
    ) -> Result<(), RbacError> {
        let subject = format!("user:{}", user_id);
        let domain = match tenant_id {
            Some(tid) => format!("tenant:{}", tid),
            None => "platform".to_string(),
        };
        
        // Add role assignment in Casbin
        self.rbac.add_role_for_user_in_tenant(&subject, role_name, &domain).await?;
        
        // Store assignment in database for auditing
        let assignment = user_tenant_roles::ActiveModel {
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            role_name: Set(role_name.to_string()),
            assigned_by: Set(assigned_by),
            ..Default::default()
        };
        
        assignment.insert(&self.db).await?;
        
        Ok(())
    }
    
    /// Remove role from user
    pub async fn remove_role_from_user(
        &self,
        user_id: i32,
        role_name: &str,
        tenant_id: Option<i32>,
    ) -> Result<(), RbacError> {
        let subject = format!("user:{}", user_id);
        let domain = match tenant_id {
            Some(tid) => format!("tenant:{}", tid),
            None => "platform".to_string(),
        };
        
        // Remove from Casbin
        self.rbac.remove_role_for_user_in_tenant(&subject, role_name, &domain).await?;
        
        // Remove from database
        user_tenant_roles::Entity::delete_many()
            .filter(user_tenant_roles::Column::UserId.eq(user_id))
            .filter(user_tenant_roles::Column::RoleName.eq(role_name))
            .filter(user_tenant_roles::Column::TenantId.eq(tenant_id))
            .exec(&self.db)
            .await?;
        
        Ok(())
    }
    
    /// Get all roles for user
    pub async fn get_user_roles(
        &self,
        user_id: i32,
    ) -> Result<Vec<UserRole>, RbacError> {
        let roles = user_tenant_roles::Entity::find()
            .filter(user_tenant_roles::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;
        
        Ok(roles.into_iter().map(|r| UserRole {
            user_id: r.user_id,
            role_name: r.role_name,
            tenant_id: r.tenant_id,
            assigned_at: r.assigned_at,
            assigned_by: r.assigned_by,
        }).collect())
    }
    
    /// Create custom role for tenant
    pub async fn create_custom_tenant_role(
        &self,
        tenant_id: i32,
        role_def: &RoleDefinition,
        created_by: i32,
    ) -> Result<(), RbacError> {
        let domain = format!("tenant:{}", tenant_id);
        
        // Validate permissions are within tenant scope
        for permission in &role_def.permissions {
            if permission.scope == Scope::Platform {
                return Err(RbacError::InvalidPermission("Tenant roles cannot have platform scope".to_string()));
            }
        }
        
        // Add role policies to Casbin
        for permission in &role_def.permissions {
            let policy = vec![
                role_def.name.clone(),
                permission.to_policy_string(),
                permission.action.to_string().to_lowercase(),
                domain.clone(),
            ];
            
            self.rbac.add_policy("p", &policy).await?;
        }
        
        // Store custom role definition in database
        let custom_role = tenant_custom_roles::ActiveModel {
            tenant_id: Set(tenant_id),
            role_name: Set(role_def.name.clone()),
            display_name: Set(role_def.display_name.clone()),
            description: Set(role_def.description.clone()),
            permissions: Set(serde_json::to_value(&role_def.permissions)?),
            created_by: Set(created_by),
            ..Default::default()
        };
        
        custom_role.insert(&self.db).await?;
        
        Ok(())
    }
}
```

## Phase 3: API Integration and Middleware

### Sprint 3.1: Authorization Middleware

#### Request Context and Middleware

```rust
// ratchet-web/src/middleware/rbac.rs
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: i32,
    pub username: String,
    pub tenant_id: Option<i32>,
    pub platform_roles: Vec<String>,
    pub tenant_roles: HashMap<i32, Vec<String>>,
    pub is_platform_operator: bool,
}

/// Permission requirement for endpoint
#[derive(Debug, Clone)]
pub struct RequiredPermission {
    pub resource: Resource,
    pub action: Action,
    pub scope: Scope,
    pub allow_platform_override: bool,
}

impl RequiredPermission {
    pub fn new(resource: Resource, action: Action, scope: Scope) -> Self {
        Self {
            resource,
            action,
            scope,
            allow_platform_override: true,
        }
    }
    
    pub fn platform_only(resource: Resource, action: Action) -> Self {
        Self {
            resource,
            action,
            scope: Scope::Platform,
            allow_platform_override: false,
        }
    }
    
    pub fn tenant_only(resource: Resource, action: Action) -> Self {
        Self {
            resource,
            action,
            scope: Scope::Tenant,
            allow_platform_override: false,
        }
    }
}

/// RBAC middleware for request authorization
pub async fn rbac_middleware(
    State(rbac): State<Arc<RbacManager>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract auth context from request
    let auth_context = match extract_auth_context(&req).await {
        Ok(ctx) => ctx,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Extract required permission from request
    let required_permission = match extract_required_permission(&req) {
        Some(perm) => perm,
        None => {
            // No permission required, continue
            req.extensions_mut().insert(auth_context);
            return Ok(next.run(req).await);
        }
    };
    
    // Check permission
    let permission_checker = PermissionChecker::new(rbac);
    let tenant_id = extract_tenant_from_request(&req);
    
    let allowed = permission_checker
        .check_permission(
            auth_context.user_id,
            &Permission {
                resource: required_permission.resource,
                action: required_permission.action,
                scope: required_permission.scope,
                conditions: None,
            },
            tenant_id,
            None, // resource_id extracted separately if needed
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Add auth context to request for handlers
    req.extensions_mut().insert(auth_context);
    Ok(next.run(req).await)
}

/// Extract authentication context from request
async fn extract_auth_context(req: &Request) -> Result<AuthContext, AuthError> {
    // Extract from JWT token, session, or API key
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(AuthError::MissingToken)?;
    
    // Decode and validate token
    let claims = decode_jwt_token(token)?;
    
    Ok(AuthContext {
        user_id: claims.user_id,
        username: claims.username,
        tenant_id: claims.tenant_id,
        platform_roles: claims.platform_roles,
        tenant_roles: claims.tenant_roles,
        is_platform_operator: claims.platform_roles.contains(&"platform_admin".to_string()),
    })
}

/// Extract required permission from request path and method
fn extract_required_permission(req: &Request) -> Option<RequiredPermission> {
    let path = req.uri().path();
    let method = req.method();
    
    // Match request patterns to required permissions
    match (method.as_str(), path) {
        // Platform tenant management
        ("POST", "/api/v1/platform/tenants") => Some(RequiredPermission::platform_only(Resource::Tenants, Action::Create)),
        ("GET", "/api/v1/platform/tenants") => Some(RequiredPermission::platform_only(Resource::Tenants, Action::Read)),
        ("PUT", path) if path.starts_with("/api/v1/platform/tenants/") => Some(RequiredPermission::platform_only(Resource::Tenants, Action::Update)),
        ("DELETE", path) if path.starts_with("/api/v1/platform/tenants/") => Some(RequiredPermission::platform_only(Resource::Tenants, Action::Delete)),
        
        // Tenant task management
        ("POST", path) if path.contains("/tenants/") && path.ends_with("/tasks") => Some(RequiredPermission::tenant_only(Resource::Tasks, Action::Create)),
        ("GET", path) if path.contains("/tenants/") && path.contains("/tasks") => Some(RequiredPermission::tenant_only(Resource::Tasks, Action::Read)),
        ("PUT", path) if path.contains("/tenants/") && path.contains("/tasks/") => Some(RequiredPermission::tenant_only(Resource::Tasks, Action::Update)),
        ("DELETE", path) if path.contains("/tenants/") && path.contains("/tasks/") => Some(RequiredPermission::tenant_only(Resource::Tasks, Action::Delete)),
        
        // Tenant execution management
        ("POST", path) if path.contains("/tenants/") && path.ends_with("/executions") => Some(RequiredPermission::tenant_only(Resource::Executions, Action::Create)),
        ("GET", path) if path.contains("/tenants/") && path.contains("/executions") => Some(RequiredPermission::tenant_only(Resource::Executions, Action::Read)),
        ("POST", path) if path.contains("/executions/") && path.ends_with("/cancel") => Some(RequiredPermission::tenant_only(Resource::Executions, Action::Cancel)),
        
        // Health and metrics endpoints (no auth required)
        ("GET", "/health") | ("GET", "/metrics") => None,
        
        // Default deny for unmatched patterns
        _ => Some(RequiredPermission::platform_only(Resource::Platform, Action::Manage)),
    }
}

/// Extract tenant ID from request path
fn extract_tenant_from_request(req: &Request) -> Option<i32> {
    let path = req.uri().path();
    
    // Extract tenant ID from URL pattern like /api/v1/tenants/{tenant_id}/...
    if let Some(captures) = regex::Regex::new(r"/api/v1/tenants/(\d+)/")
        .unwrap()
        .captures(path) 
    {
        captures.get(1)?.as_str().parse().ok()
    } else {
        None
    }
}
```

### Sprint 3.2: Tenant-Aware API Endpoints

#### Tenant Management API

```rust
// ratchet-rest-api/src/handlers/tenants.rs
use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantResponse {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub member_count: u32,
}

/// Create new tenant (platform admin only)
#[utoipa::path(
    post,
    path = "/api/v1/platform/tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created successfully", body = TenantResponse),
        (status = 403, description = "Insufficient permissions"),
        (status = 409, description = "Tenant name already exists")
    ),
    tag = "tenants"
)]
pub async fn create_tenant(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<CreateTenantRequest>,
) -> RestResult<Json<TenantResponse>> {
    // Verify platform admin permission (handled by middleware)
    
    let tenant = ctx
        .repositories
        .tenant_repository()
        .create_tenant(&request.name, &request.display_name, request.description.as_deref())
        .await?;
    
    // Assign creator as tenant admin
    ctx.rbac
        .assign_role_to_user(auth.user_id, "tenant_admin", Some(tenant.id), Some(auth.user_id))
        .await?;
    
    Ok(Json(TenantResponse {
        id: tenant.id,
        name: tenant.name,
        display_name: tenant.display_name,
        description: tenant.description,
        is_active: tenant.is_active,
        created_at: tenant.created_at,
        member_count: 1,
    }))
}

/// List all tenants (platform admin) or user's tenants
#[utoipa::path(
    get,
    path = "/api/v1/platform/tenants",
    responses(
        (status = 200, description = "List of tenants", body = Vec<TenantResponse>),
        (status = 403, description = "Insufficient permissions")
    ),
    tag = "tenants"
)]
pub async fn list_tenants(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
) -> RestResult<Json<Vec<TenantResponse>>> {
    let tenants = if auth.is_platform_operator {
        // Platform operators see all tenants
        ctx.repositories.tenant_repository().list_all().await?
    } else {
        // Regular users see only their tenants
        ctx.repositories.tenant_repository().list_for_user(auth.user_id).await?
    };
    
    let response: Vec<TenantResponse> = tenants
        .into_iter()
        .map(|t| TenantResponse {
            id: t.id,
            name: t.name,
            display_name: t.display_name,
            description: t.description,
            is_active: t.is_active,
            created_at: t.created_at,
            member_count: 0, // TODO: Count members
        })
        .collect();
    
    Ok(Json(response))
}

/// Get tenant details
#[utoipa::path(
    get,
    path = "/api/v1/platform/tenants/{tenant_id}",
    responses(
        (status = 200, description = "Tenant details", body = TenantResponse),
        (status = 404, description = "Tenant not found"),
        (status = 403, description = "Insufficient permissions")
    ),
    tag = "tenants"
)]
pub async fn get_tenant(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
    Path(tenant_id): Path<i32>,
) -> RestResult<Json<TenantResponse>> {
    // Check if user has access to this tenant
    if !auth.is_platform_operator && !auth.tenant_roles.contains_key(&tenant_id) {
        return Err(RestError::Forbidden);
    }
    
    let tenant = ctx
        .repositories
        .tenant_repository()
        .find_by_id(tenant_id)
        .await?
        .ok_or(RestError::NotFound)?;
    
    Ok(Json(TenantResponse {
        id: tenant.id,
        name: tenant.name,
        display_name: tenant.display_name,
        description: tenant.description,
        is_active: tenant.is_active,
        created_at: tenant.created_at,
        member_count: 0, // TODO: Count members
    }))
}
```

#### Tenant User Management API

```rust
// ratchet-rest-api/src/handlers/tenant_users.rs
use axum::{
    extract::{Path, State},
    response::Json,
    Extension,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub role: String,
    pub send_email: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantUserResponse {
    pub user_id: i32,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Invite user to tenant
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/users/invite",
    request_body = InviteUserRequest,
    responses(
        (status = 201, description = "User invited successfully"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "User not found")
    ),
    tag = "tenant-users"
)]
pub async fn invite_user_to_tenant(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
    Path(tenant_id): Path<i32>,
    Json(request): Json<InviteUserRequest>,
) -> RestResult<Json<serde_json::Value>> {
    // Check if user can manage users in this tenant
    let permission = Permission {
        resource: Resource::Users,
        action: Action::Manage,
        scope: Scope::Tenant,
        conditions: None,
    };
    
    let allowed = ctx
        .permission_checker
        .check_permission(auth.user_id, &permission, Some(tenant_id), None)
        .await?;
    
    if !allowed {
        return Err(RestError::Forbidden);
    }
    
    // Find user by email
    let user = ctx
        .repositories
        .user_repository()
        .find_by_email(&request.email)
        .await?
        .ok_or(RestError::NotFound)?;
    
    // Add user to tenant with specified role
    ctx.rbac
        .assign_role_to_user(user.id, &request.role, Some(tenant_id), Some(auth.user_id))
        .await?;
    
    // TODO: Send invitation email if requested
    
    Ok(Json(serde_json::json!({ "status": "invited" })))
}

/// List tenant users
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/users",
    responses(
        (status = 200, description = "List of tenant users", body = Vec<TenantUserResponse>),
        (status = 403, description = "Insufficient permissions")
    ),
    tag = "tenant-users"
)]
pub async fn list_tenant_users(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
    Path(tenant_id): Path<i32>,
) -> RestResult<Json<Vec<TenantUserResponse>>> {
    // Check if user can read users in this tenant
    let permission = Permission {
        resource: Resource::Users,
        action: Action::Read,
        scope: Scope::Tenant,
        conditions: None,
    };
    
    let allowed = ctx
        .permission_checker
        .check_permission(auth.user_id, &permission, Some(tenant_id), None)
        .await?;
    
    if !allowed {
        return Err(RestError::Forbidden);
    }
    
    let users = ctx
        .repositories
        .tenant_repository()
        .list_tenant_users(tenant_id)
        .await?;
    
    let response: Vec<TenantUserResponse> = users
        .into_iter()
        .map(|u| TenantUserResponse {
            user_id: u.user_id,
            username: u.username,
            email: u.email,
            roles: u.roles,
            joined_at: u.joined_at,
        })
        .collect();
    
    Ok(Json(response))
}

/// Update user roles in tenant
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/users/{user_id}/roles",
    request_body = Vec<String>,
    responses(
        (status = 200, description = "User roles updated successfully"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "User not found")
    ),
    tag = "tenant-users"
)]
pub async fn update_user_roles(
    State(ctx): State<AppContext>,
    Extension(auth): Extension<AuthContext>,
    Path((tenant_id, user_id)): Path<(i32, i32)>,
    Json(roles): Json<Vec<String>>,
) -> RestResult<Json<serde_json::Value>> {
    // Check if user can manage users in this tenant
    let permission = Permission {
        resource: Resource::Users,
        action: Action::Manage,
        scope: Scope::Tenant,
        conditions: None,
    };
    
    let allowed = ctx
        .permission_checker
        .check_permission(auth.user_id, &permission, Some(tenant_id), None)
        .await?;
    
    if !allowed {
        return Err(RestError::Forbidden);
    }
    
    // Get current roles
    let current_roles = ctx.rbac.get_user_roles(user_id).await?;
    let current_tenant_roles: Vec<String> = current_roles
        .into_iter()
        .filter(|r| r.tenant_id == Some(tenant_id))
        .map(|r| r.role_name)
        .collect();
    
    // Remove old roles
    for role in &current_tenant_roles {
        ctx.rbac
            .remove_role_from_user(user_id, role, Some(tenant_id))
            .await?;
    }
    
    // Add new roles
    for role in &roles {
        ctx.rbac
            .assign_role_to_user(user_id, role, Some(tenant_id), Some(auth.user_id))
            .await?;
    }
    
    Ok(Json(serde_json::json!({ "status": "updated" })))
}
```

## Phase 4: Testing and Migration

### Sprint 4.1: Comprehensive Testing

#### Unit Tests for RBAC System

```rust
// ratchet-rbac/tests/rbac_tests.rs
use ratchet_rbac::*;
use tokio_test;

#[tokio::test]
async fn test_platform_admin_permissions() {
    let rbac = setup_test_rbac().await;
    
    // Platform admin should have access to tenant management
    let allowed = rbac
        .enforce("user:1", "platform:manage", "manage", "platform")
        .await
        .unwrap();
    assert!(allowed);
    
    // Platform admin should have access to any tenant
    let allowed = rbac
        .enforce("user:1", "tasks:create", "create", "tenant:1")
        .await
        .unwrap();
    assert!(allowed);
}

#[tokio::test]
async fn test_tenant_isolation() {
    let rbac = setup_test_rbac().await;
    
    // Tenant user should have access to their tenant
    let allowed = rbac
        .enforce("user:2", "tasks:create", "create", "tenant:1")
        .await
        .unwrap();
    assert!(allowed);
    
    // Tenant user should NOT have access to other tenant
    let allowed = rbac
        .enforce("user:2", "tasks:create", "create", "tenant:2")
        .await
        .unwrap();
    assert!(!allowed);
}

#[tokio::test]
async fn test_role_assignment() {
    let rbac = setup_test_rbac().await;
    let role_manager = RoleManager::new(rbac.clone(), setup_test_db().await);
    
    // Assign tenant admin role
    role_manager
        .assign_role_to_user(3, "tenant_admin", Some(1), Some(1))
        .await
        .unwrap();
    
    // User should now have tenant admin permissions
    let allowed = rbac
        .enforce("user:3", "users:manage", "manage", "tenant:1")
        .await
        .unwrap();
    assert!(allowed);
}

#[tokio::test]
async fn test_custom_role_creation() {
    let rbac = setup_test_rbac().await;
    let role_manager = RoleManager::new(rbac.clone(), setup_test_db().await);
    
    let custom_role = RoleDefinition {
        name: "task_executor".to_string(),
        display_name: "Task Executor".to_string(),
        description: "Can only execute tasks".to_string(),
        permissions: vec![
            Permission {
                resource: Resource::Tasks,
                action: Action::Execute,
                scope: Scope::Tenant,
                conditions: None,
            }
        ],
        is_system_role: false,
    };
    
    role_manager
        .create_custom_tenant_role(1, &custom_role, 1)
        .await
        .unwrap();
    
    // Assign custom role to user
    role_manager
        .assign_role_to_user(4, "task_executor", Some(1), Some(1))
        .await
        .unwrap();
    
    // User should be able to execute tasks
    let allowed = rbac
        .enforce("user:4", "tasks:execute", "execute", "tenant:1")
        .await
        .unwrap();
    assert!(allowed);
    
    // User should NOT be able to create tasks
    let allowed = rbac
        .enforce("user:4", "tasks:create", "create", "tenant:1")
        .await
        .unwrap();
    assert!(!allowed);
}

async fn setup_test_rbac() -> Arc<RbacManager> {
    // Setup test RBAC with default roles
    let db = setup_test_db().await;
    let rbac = RbacManager::new(db).await.unwrap();
    
    // Add platform admin role
    rbac.add_policy("p", &vec![
        "platform_admin".to_string(),
        "platform:manage".to_string(),
        "manage".to_string(),
        "platform".to_string(),
    ]).await.unwrap();
    
    // Add tenant admin role
    rbac.add_policy("p", &vec![
        "tenant_admin".to_string(),
        "users:manage".to_string(),
        "manage".to_string(),
        "tenant:1".to_string(),
    ]).await.unwrap();
    
    // Assign roles to test users
    rbac.add_role_for_user_in_tenant("user:1", "platform_admin", "platform").await.unwrap();
    rbac.add_role_for_user_in_tenant("user:2", "tenant_user", "tenant:1").await.unwrap();
    
    Arc::new(rbac)
}
```

#### Integration Tests for API Endpoints

```rust
// ratchet-rest-api/tests/tenant_api_tests.rs
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_platform_admin_can_create_tenant() {
    let server = setup_test_server().await;
    
    let response = server
        .post("/api/v1/platform/tenants")
        .add_header("Authorization", "Bearer platform_admin_token")
        .json(&json!({
            "name": "test_tenant",
            "display_name": "Test Tenant",
            "description": "A test tenant"
        }))
        .await;
    
    response.assert_status_created();
    response.assert_json_contains(&json!({
        "name": "test_tenant",
        "display_name": "Test Tenant"
    }));
}

#[tokio::test]
async fn test_tenant_user_cannot_create_tenant() {
    let server = setup_test_server().await;
    
    let response = server
        .post("/api/v1/platform/tenants")
        .add_header("Authorization", "Bearer tenant_user_token")
        .json(&json!({
            "name": "unauthorized_tenant",
            "display_name": "Unauthorized Tenant"
        }))
        .await;
    
    response.assert_status_forbidden();
}

#[tokio::test]
async fn test_tenant_isolation_in_task_listing() {
    let server = setup_test_server().await;
    
    // Create tasks in different tenants
    create_test_task(&server, 1, "tenant1_task").await;
    create_test_task(&server, 2, "tenant2_task").await;
    
    // User in tenant 1 should only see tenant 1 tasks
    let response = server
        .get("/api/v1/tenants/1/tasks")
        .add_header("Authorization", "Bearer tenant1_user_token")
        .await;
    
    response.assert_status_ok();
    let tasks: serde_json::Value = response.json();
    assert_eq!(tasks["items"].as_array().unwrap().len(), 1);
    assert_eq!(tasks["items"][0]["name"], "tenant1_task");
}

#[tokio::test]
async fn test_role_based_access_to_user_management() {
    let server = setup_test_server().await;
    
    // Tenant admin should be able to invite users
    let response = server
        .post("/api/v1/tenants/1/users/invite")
        .add_header("Authorization", "Bearer tenant_admin_token")
        .json(&json!({
            "email": "newuser@example.com",
            "role": "tenant_user",
            "send_email": false
        }))
        .await;
    
    response.assert_status_created();
    
    // Regular tenant user should NOT be able to invite users
    let response = server
        .post("/api/v1/tenants/1/users/invite")
        .add_header("Authorization", "Bearer tenant_user_token")
        .json(&json!({
            "email": "unauthorized@example.com",
            "role": "tenant_user",
            "send_email": false
        }))
        .await;
    
    response.assert_status_forbidden();
}
```

### Sprint 4.2: Data Migration and Backward Compatibility

#### Migration Script for Existing Data

```rust
// ratchet-server/src/migration/rbac_migration.rs
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

pub struct RbacMigration {
    db: DatabaseConnection,
    rbac: Arc<RbacManager>,
}

impl RbacMigration {
    pub fn new(db: DatabaseConnection, rbac: Arc<RbacManager>) -> Self {
        Self { db, rbac }
    }
    
    /// Migrate existing users to multi-tenant system
    pub async fn migrate_existing_users(&self) -> Result<(), MigrationError> {
        println!("Starting RBAC migration...");
        
        // 1. Create default tenant
        let default_tenant = self.create_default_tenant().await?;
        println!("Created default tenant: {}", default_tenant.id);
        
        // 2. Migrate existing users to default tenant
        let users = self.get_existing_users().await?;
        println!("Found {} existing users to migrate", users.len());
        
        for user in users {
            self.migrate_user_to_tenant(&user, default_tenant.id).await?;
            println!("Migrated user {} to default tenant", user.username);
        }
        
        // 3. Update existing resources to belong to default tenant
        self.migrate_resources_to_tenant(default_tenant.id).await?;
        println!("Migrated all resources to default tenant");
        
        // 4. Load default roles and assign to users
        self.setup_default_roles().await?;
        self.assign_migrated_user_roles(default_tenant.id).await?;
        println!("Assigned roles to migrated users");
        
        println!("RBAC migration completed successfully");
        Ok(())
    }
    
    async fn create_default_tenant(&self) -> Result<Tenant, MigrationError> {
        let tenant = tenants::ActiveModel {
            name: Set("default".to_string()),
            display_name: Set("Default Tenant".to_string()),
            description: Set(Some("Default tenant for migrated data".to_string())),
            is_active: Set(true),
            ..Default::default()
        };
        
        Ok(tenant.insert(&self.db).await?)
    }
    
    async fn migrate_user_to_tenant(&self, user: &User, tenant_id: i32) -> Result<(), MigrationError> {
        // Add user to default tenant
        let user_tenant = user_tenants::ActiveModel {
            user_id: Set(user.id),
            tenant_id: Set(tenant_id),
            joined_by: Set(None), // System migration
            ..Default::default()
        };
        
        user_tenant.insert(&self.db).await?;
        Ok(())
    }
    
    async fn migrate_resources_to_tenant(&self, tenant_id: i32) -> Result<(), MigrationError> {
        // Update tasks
        tasks::Entity::update_many()
            .col_expr(tasks::Column::TenantId, Expr::value(tenant_id))
            .exec(&self.db)
            .await?;
        
        // Update executions
        executions::Entity::update_many()
            .col_expr(executions::Column::TenantId, Expr::value(tenant_id))
            .exec(&self.db)
            .await?;
        
        // Update jobs
        jobs::Entity::update_many()
            .col_expr(jobs::Column::TenantId, Expr::value(tenant_id))
            .exec(&self.db)
            .await?;
        
        // Update schedules
        schedules::Entity::update_many()
            .col_expr(schedules::Column::TenantId, Expr::value(tenant_id))
            .exec(&self.db)
            .await?;
        
        Ok(())
    }
    
    async fn assign_migrated_user_roles(&self, tenant_id: i32) -> Result<(), MigrationError> {
        let users = users::Entity::find().all(&self.db).await?;
        
        for user in users {
            // Map old roles to new multi-tenant roles
            let new_role = match user.role {
                users::UserRole::Admin => "tenant_admin",
                users::UserRole::User => "tenant_user",
                users::UserRole::ReadOnly => "tenant_viewer",
                users::UserRole::Service => "tenant_user", // Map service to user
            };
            
            // Assign role in tenant context
            self.rbac
                .assign_role_to_user(user.id, new_role, Some(tenant_id), None)
                .await?;
            
            // First admin user gets platform admin role
            if user.id == 1 && user.role == users::UserRole::Admin {
                self.rbac
                    .assign_role_to_user(user.id, "platform_admin", None, None)
                    .await?;
            }
        }
        
        Ok(())
    }
}
```

#### Backward Compatibility Layer

```rust
// ratchet-rest-api/src/compat/legacy_auth.rs
/// Backward compatibility middleware for single-tenant mode
pub async fn legacy_auth_compatibility(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check if multi-tenant mode is enabled
    if !is_multi_tenant_enabled() {
        // Legacy single-tenant mode - use original auth logic
        return legacy_auth_middleware(req, next).await;
    }
    
    // Multi-tenant mode - use new RBAC middleware
    rbac_middleware(req, next).await
}

/// Legacy authentication for backward compatibility
async fn legacy_auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract user from legacy session/token
    let user = extract_legacy_user(&req).await?;
    
    // Create compatible auth context
    let auth_context = AuthContext {
        user_id: user.id,
        username: user.username.clone(),
        tenant_id: Some(1), // Default tenant
        platform_roles: if user.role == users::UserRole::Admin {
            vec!["platform_admin".to_string()]
        } else {
            vec![]
        },
        tenant_roles: {
            let mut roles = HashMap::new();
            let tenant_role = match user.role {
                users::UserRole::Admin => "tenant_admin",
                users::UserRole::User => "tenant_user",
                users::UserRole::ReadOnly => "tenant_viewer",
                users::UserRole::Service => "tenant_user",
            };
            roles.insert(1, vec![tenant_role.to_string()]);
            roles
        },
        is_platform_operator: user.role == users::UserRole::Admin,
    };
    
    req.extensions_mut().insert(auth_context);
    Ok(next.run(req).await)
}
```

## Phase 5: Performance Optimization and Documentation

### Sprint 5.1: Performance Optimization

#### Casbin Policy Caching

```rust
// ratchet-rbac/src/cache.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct PolicyCache {
    cache: Arc<RwLock<HashMap<String, CachedPolicy>>>,
    ttl: Duration,
}

#[derive(Clone)]
struct CachedPolicy {
    result: bool,
    timestamp: Instant,
}

impl PolicyCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<bool> {
        let cache = self.cache.read().await;
        if let Some(policy) = cache.get(key) {
            if policy.timestamp.elapsed() < self.ttl {
                return Some(policy.result);
            }
        }
        None
    }
    
    pub async fn set(&self, key: String, result: bool) {
        let mut cache = self.cache.write().await;
        cache.insert(key, CachedPolicy {
            result,
            timestamp: Instant::now(),
        });
    }
    
    pub async fn invalidate_user(&self, user_id: i32) {
        let mut cache = self.cache.write().await;
        let prefix = format!("user:{}", user_id);
        cache.retain(|k, _| !k.starts_with(&prefix));
    }
    
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, policy| policy.timestamp.elapsed() < self.ttl);
    }
}

// Enhanced RbacManager with caching
impl RbacManager {
    pub async fn enforce_cached(
        &self,
        subject: &str,
        object: &str,
        action: &str,
        domain: &str,
    ) -> CasbinResult<bool> {
        let cache_key = format!("{}:{}:{}:{}", subject, object, action, domain);
        
        // Check cache first
        if let Some(result) = self.cache.get(&cache_key).await {
            return Ok(result);
        }
        
        // Call Casbin enforcer
        let result = self.enforcer.enforce((subject, object, action, domain)).await?;
        
        // Cache the result
        self.cache.set(cache_key, result).await;
        
        Ok(result)
    }
}
```

#### Database Query Optimization

```sql
-- Optimized indexes for RBAC queries
CREATE INDEX CONCURRENTLY idx_casbin_rules_domain_lookup 
ON casbin_rules(ptype, v0, v3) 
WHERE ptype IN ('p', 'g');

CREATE INDEX CONCURRENTLY idx_user_tenants_user_lookup 
ON user_tenants(user_id) 
INCLUDE (tenant_id, role_name);

CREATE INDEX CONCURRENTLY idx_resources_tenant_user 
ON tasks(tenant_id, created_by);

-- Materialized view for user permissions
CREATE MATERIALIZED VIEW user_permissions_cache AS
SELECT 
    ur.user_id,
    ur.tenant_id,
    ur.role_name,
    cr.v1 as resource,
    cr.v2 as action,
    cr.v3 as domain
FROM user_tenant_roles ur
JOIN casbin_rules cr ON cr.v0 = ur.role_name
WHERE cr.ptype = 'p';

CREATE UNIQUE INDEX idx_user_permissions_cache_lookup 
ON user_permissions_cache(user_id, tenant_id, resource, action);

-- Refresh materialized view on policy changes
CREATE OR REPLACE FUNCTION refresh_user_permissions_cache()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY user_permissions_cache;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_refresh_permissions
AFTER INSERT OR UPDATE OR DELETE ON casbin_rules
FOR EACH STATEMENT
EXECUTE FUNCTION refresh_user_permissions_cache();
```

### Sprint 5.2: Documentation and Examples

#### Configuration Examples

```yaml
# config/rbac_config.yaml
rbac:
  # Enable multi-tenant mode
  multi_tenant_enabled: true
  
  # Default tenant for legacy compatibility
  default_tenant: "default"
  
  # Policy cache TTL in seconds
  cache_ttl: 300
  
  # Default roles to create on startup
  default_platform_roles:
    - platform_admin
    - platform_monitor
  
  default_tenant_roles:
    - tenant_admin
    - tenant_user
    - tenant_viewer
  
  # Custom role definitions
  custom_roles:
    task_developer:
      display_name: "Task Developer"
      description: "Can create and test tasks"
      scope: "tenant"
      permissions:
        - resource: "tasks"
          action: "create"
        - resource: "tasks"
          action: "read"
        - resource: "tasks"
          action: "update"
          conditions:
            owner: "self"
        - resource: "executions"
          action: "read"
          conditions:
            owner: "self"
    
    metrics_viewer:
      display_name: "Metrics Viewer"
      description: "Read-only access to metrics"
      scope: "tenant"
      permissions:
        - resource: "metrics"
          action: "read"
        - resource: "tasks"
          action: "read"
        - resource: "executions"
          action: "read"
```

#### API Usage Examples

```bash
#!/bin/bash
# examples/rbac_api_examples.sh

# Platform admin creates a new tenant
curl -X POST http://localhost:8080/api/v1/platform/tenants \
  -H "Authorization: Bearer $PLATFORM_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "acme_corp",
    "display_name": "ACME Corporation",
    "description": "ACME Corp tenant for task automation"
  }'

# Tenant admin invites a user
curl -X POST http://localhost:8080/api/v1/tenants/2/users/invite \
  -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "developer@acme.com",
    "role": "tenant_user",
    "send_email": true
  }'

# Create custom role for tenant
curl -X POST http://localhost:8080/api/v1/tenants/2/roles \
  -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "task_reviewer",
    "display_name": "Task Reviewer",
    "description": "Can review and approve tasks",
    "permissions": [
      {"resource": "tasks", "action": "read", "scope": "tenant"},
      {"resource": "executions", "action": "read", "scope": "tenant"},
      {"resource": "tasks", "action": "update", "scope": "tenant", "conditions": {"status": "pending_review"}}
    ]
  }'

# User creates task in their tenant
curl -X POST http://localhost:8080/api/v1/tenants/2/tasks \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "data_processing",
    "description": "Process daily data files",
    "version": "1.0.0",
    "input_schema": {"type": "object", "properties": {"file_path": {"type": "string"}}}
  }'
```

## Success Metrics and Rollout Plan

### Success Metrics

#### Functional Metrics
- ✅ Complete tenant isolation (0 cross-tenant data leaks in security testing)
- ✅ Authorization decision latency < 50ms for 95th percentile
- ✅ Support for 1000+ tenants and 10,000+ users
- ✅ API compatibility maintained for single-tenant deployments

#### Security Metrics
- ✅ Zero privilege escalation vulnerabilities in penetration testing
- ✅ All sensitive operations require explicit permission grants
- ✅ Complete audit trail for all role assignments and permission changes
- ✅ Tenant data completely isolated at database and application levels

#### Operational Metrics
- ✅ Zero-downtime migration of existing single-tenant deployments
- ✅ Platform operators can manage 100+ tenants efficiently
- ✅ Tenant administrators can manage their users without platform admin involvement
- ✅ Custom roles can be created and managed via both API and YAML

### Rollout Strategy

#### Phase 1: Internal Testing (Week 9)
- Deploy to internal staging environment
- Migrate test data from single-tenant to multi-tenant
- Comprehensive security testing
- Performance benchmarking

#### Phase 2: Beta Testing (Week 10)
- Deploy to select customer environments
- Gradual rollout with feature flags
- Monitor metrics and gather feedback
- Refine based on real-world usage

#### Phase 3: Production Release (Week 11+)
- Full rollout to all environments
- Documentation and training materials
- Support for both single-tenant (legacy) and multi-tenant modes
- Migration assistance for existing customers

### Risk Mitigation

#### Technical Risks
- **Performance degradation**: Extensive caching and query optimization
- **Security vulnerabilities**: Multiple security reviews and penetration testing
- **Data migration failures**: Comprehensive testing and rollback procedures

#### Operational Risks
- **Complex configuration**: Clear documentation and validation tooling
- **User training needs**: Comprehensive guides and examples
- **Support complexity**: Backward compatibility and gradual migration path

## Conclusion

This comprehensive Casbin-based multi-tenant RBAC implementation provides Ratchet with the foundation for secure, scalable multi-tenancy while maintaining backward compatibility. The phased approach ensures gradual migration with minimal risk, while the flexible permission system supports both standard and custom authorization patterns.

The implementation leverages proven technology (Casbin) combined with custom tenant management to deliver a robust authorization system suitable for platform and tenant-level administration, with performance optimizations and comprehensive testing to ensure production readiness.