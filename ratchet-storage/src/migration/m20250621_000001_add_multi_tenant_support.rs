//! Multi-tenant support migration
//! 
//! This migration adds the foundational tables for multi-tenant RBAC support:
//! - tenants: Tenant management
//! - casbin_rules: Policy storage for Casbin
//! - user_tenants: User-tenant associations
//! - Adds tenant_id to existing resource tables

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create tenants table
        manager
            .create_table(
                Table::create()
                    .table(Tenants::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tenants::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Tenants::Uuid)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Tenants::Name)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Tenants::DisplayName)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tenants::Description).text())
                    .col(
                        ColumnDef::new(Tenants::Settings)
                            .json()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Tenants::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Tenants::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tenants::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Tenants::CreatedBy).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tenants_created_by")
                            .from(Tenants::Table, Tenants::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Create casbin_rules table for policy storage
        manager
            .create_table(
                Table::create()
                    .table(CasbinRules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CasbinRules::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CasbinRules::Ptype)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(CasbinRules::V0).string_len(100))
                    .col(ColumnDef::new(CasbinRules::V1).string_len(100))
                    .col(ColumnDef::new(CasbinRules::V2).string_len(100))
                    .col(ColumnDef::new(CasbinRules::V3).string_len(100))
                    .col(ColumnDef::new(CasbinRules::V4).string_len(100))
                    .col(ColumnDef::new(CasbinRules::V5).string_len(100))
                    .col(
                        ColumnDef::new(CasbinRules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Create user_tenants association table
        manager
            .create_table(
                Table::create()
                    .table(UserTenants::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserTenants::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserTenants::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserTenants::TenantId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserTenants::JoinedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(UserTenants::JoinedBy).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_tenants_user_id")
                            .from(UserTenants::Table, UserTenants::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_tenants_tenant_id")
                            .from(UserTenants::Table, UserTenants::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_tenants_joined_by")
                            .from(UserTenants::Table, UserTenants::JoinedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Create user_roles table for role assignments
        manager
            .create_table(
                Table::create()
                    .table(UserRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRoles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserRoles::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserRoles::TenantId).integer())
                    .col(
                        ColumnDef::new(UserRoles::RoleName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRoles::AssignedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(UserRoles::AssignedBy).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_user_id")
                            .from(UserRoles::Table, UserRoles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_tenant_id")
                            .from(UserRoles::Table, UserRoles::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_roles_assigned_by")
                            .from(UserRoles::Table, UserRoles::AssignedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // 5. Create tenant_custom_roles table for custom role definitions
        manager
            .create_table(
                Table::create()
                    .table(TenantCustomRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TenantCustomRoles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TenantCustomRoles::TenantId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TenantCustomRoles::RoleName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TenantCustomRoles::DisplayName)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(TenantCustomRoles::Description).text())
                    .col(
                        ColumnDef::new(TenantCustomRoles::Permissions)
                            .json()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TenantCustomRoles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TenantCustomRoles::CreatedBy)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tenant_custom_roles_tenant_id")
                            .from(TenantCustomRoles::Table, TenantCustomRoles::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tenant_custom_roles_created_by")
                            .from(TenantCustomRoles::Table, TenantCustomRoles::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 6. Add tenant_id columns to existing resource tables
        // Tasks table
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::TenantId).integer())
                    .to_owned(),
            )
            .await?;

        // Executions table
        manager
            .alter_table(
                Table::alter()
                    .table(Executions::Table)
                    .add_column(ColumnDef::new(Executions::TenantId).integer())
                    .to_owned(),
            )
            .await?;

        // Jobs table
        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column(ColumnDef::new(Jobs::TenantId).integer())
                    .to_owned(),
            )
            .await?;

        // Schedules table
        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .add_column(ColumnDef::new(Schedules::TenantId).integer())
                    .to_owned(),
            )
            .await?;

        // 7. Create default tenant and assign existing data
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Tenants::Table)
                    .columns([
                        Tenants::Uuid,
                        Tenants::Name,
                        Tenants::DisplayName,
                        Tenants::Description,
                        Tenants::IsActive,
                    ])
                    .values_panic([
                        "00000000-0000-0000-0000-000000000001".into(),
                        "default".into(),
                        "Default Tenant".into(),
                        "Default tenant for migrated data".into(),
                        true.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // 8. Set default tenant for existing resources
        manager
            .exec_stmt(
                Query::update()
                    .table(Tasks::Table)
                    .value(Tasks::TenantId, 1)
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::update()
                    .table(Executions::Table)
                    .value(Executions::TenantId, 1)
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::update()
                    .table(Jobs::Table)
                    .value(Jobs::TenantId, 1)
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::update()
                    .table(Schedules::Table)
                    .value(Schedules::TenantId, 1)
                    .to_owned(),
            )
            .await?;

        // 9. Add all existing users to default tenant
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(UserTenants::Table)
                    .columns([UserTenants::UserId, UserTenants::TenantId])
                    .select_from(
                        Query::select()
                            .column(Users::Id)
                            .expr(Expr::value(1))
                            .from(Users::Table)
                            .to_owned(),
                    )?
                    .to_owned(),
            )
            .await?;

        // 10. Make tenant_id NOT NULL after data migration
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .modify_column(ColumnDef::new(Tasks::TenantId).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Executions::Table)
                    .modify_column(ColumnDef::new(Executions::TenantId).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .modify_column(ColumnDef::new(Jobs::TenantId).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .modify_column(ColumnDef::new(Schedules::TenantId).integer().not_null())
                    .to_owned(),
            )
            .await?;

        // 11. Add foreign key constraints
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_tasks_tenant_id")
                            .from(Tasks::Table, Tasks::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Executions::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_executions_tenant_id")
                            .from(Executions::Table, Executions::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_jobs_tenant_id")
                            .from(Jobs::Table, Jobs::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_schedules_tenant_id")
                            .from(Schedules::Table, Schedules::TenantId)
                            .to(Tenants::Table, Tenants::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        // 12. Create indexes for performance
        manager
            .create_index(
                Index::create()
                    .name("idx_tenants_name")
                    .table(Tenants::Table)
                    .col(Tenants::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tenants_active")
                    .table(Tenants::Table)
                    .col(Tenants::IsActive)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_casbin_rules_ptype")
                    .table(CasbinRules::Table)
                    .col(CasbinRules::Ptype)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_casbin_rules_subject")
                    .table(CasbinRules::Table)
                    .col(CasbinRules::V0)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_tenants_user")
                    .table(UserTenants::Table)
                    .col(UserTenants::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_tenants_tenant")
                    .table(UserTenants::Table)
                    .col(UserTenants::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_roles_user_tenant")
                    .table(UserRoles::Table)
                    .col(UserRoles::UserId)
                    .col(UserRoles::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tasks_tenant")
                    .table(Tasks::Table)
                    .col(Tasks::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_executions_tenant")
                    .table(Executions::Table)
                    .col(Executions::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_tenant")
                    .table(Jobs::Table)
                    .col(Jobs::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_schedules_tenant")
                    .table(Schedules::Table)
                    .col(Schedules::TenantId)
                    .to_owned(),
            )
            .await?;

        // 13. Create unique constraints
        manager
            .create_index(
                Index::create()
                    .name("idx_user_tenants_unique")
                    .table(UserTenants::Table)
                    .col(UserTenants::UserId)
                    .col(UserTenants::TenantId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_roles_unique")
                    .table(UserRoles::Table)
                    .col(UserRoles::UserId)
                    .col(UserRoles::TenantId)
                    .col(UserRoles::RoleName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tenant_custom_roles_unique")
                    .table(TenantCustomRoles::Table)
                    .col(TenantCustomRoles::TenantId)
                    .col(TenantCustomRoles::RoleName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove foreign key constraints first
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_foreign_key(Alias::new("fk_tasks_tenant_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Executions::Table)
                    .drop_foreign_key(Alias::new("fk_executions_tenant_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .drop_foreign_key(Alias::new("fk_jobs_tenant_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .drop_foreign_key(Alias::new("fk_schedules_tenant_id"))
                    .to_owned(),
            )
            .await?;

        // Remove tenant_id columns
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Executions::Table)
                    .drop_column(Executions::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .drop_column(Jobs::TenantId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Schedules::Table)
                    .drop_column(Schedules::TenantId)
                    .to_owned(),
            )
            .await?;

        // Drop new tables
        manager
            .drop_table(Table::drop().table(TenantCustomRoles::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserRoles::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(UserTenants::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(CasbinRules::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Tenants::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Tenants {
    Table,
    Id,
    Uuid,
    Name,
    DisplayName,
    Description,
    Settings,
    IsActive,
    CreatedAt,
    UpdatedAt,
    CreatedBy,
}

#[derive(DeriveIden)]
enum CasbinRules {
    Table,
    Id,
    Ptype,
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserTenants {
    Table,
    Id,
    UserId,
    TenantId,
    JoinedAt,
    JoinedBy,
}

#[derive(DeriveIden)]
enum UserRoles {
    Table,
    Id,
    UserId,
    TenantId,
    RoleName,
    AssignedAt,
    AssignedBy,
}

#[derive(DeriveIden)]
enum TenantCustomRoles {
    Table,
    Id,
    TenantId,
    RoleName,
    DisplayName,
    Description,
    Permissions,
    CreatedAt,
    CreatedBy,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    TenantId,
}

#[derive(DeriveIden)]
enum Executions {
    Table,
    TenantId,
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    TenantId,
}

#[derive(DeriveIden)]
enum Schedules {
    Table,
    TenantId,
}