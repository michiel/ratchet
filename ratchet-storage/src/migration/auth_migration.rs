//! Authentication database migration
//!
//! This migration adds the necessary tables for authentication:
//! - users: User accounts with roles and authentication info
//! - api_keys: API keys for service authentication
//! - sessions: User sessions for JWT token management

use sea_orm::{ConnectionTrait, DbErr, Statement};
use sea_orm_migration::prelude::*;

/// Authentication migration - adds user, api_keys, and sessions tables
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordHash)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::DisplayName).string())
                    .col(
                        ColumnDef::new(Users::Role)
                            .string()
                            .not_null()
                            .default("user"),
                    )
                    .col(
                        ColumnDef::new(Users::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Users::EmailVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Users::ResetToken).string())
                    .col(ColumnDef::new(Users::ResetTokenExpires).timestamp())
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(ColumnDef::new(Users::LastLoginAt).timestamp())
                    .to_owned(),
            )
            .await?;

        // Create api_keys table
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApiKeys::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::KeyHash)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(ApiKeys::Name).string().not_null())
                    .col(ColumnDef::new(ApiKeys::UserId).integer().not_null())
                    .col(ColumnDef::new(ApiKeys::KeyPrefix).string().not_null())
                    .col(
                        ColumnDef::new(ApiKeys::Permissions)
                            .string()
                            .not_null()
                            .default("read"),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(ApiKeys::ExpiresAt).timestamp())
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(ColumnDef::new(ApiKeys::LastUsedAt).timestamp())
                    .col(
                        ColumnDef::new(ApiKeys::UsageCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_keys_user_id")
                            .from(ApiKeys::Table, ApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create sessions table
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sessions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Sessions::SessionId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Sessions::UserId).integer().not_null())
                    .col(ColumnDef::new(Sessions::JwtId).string().not_null())
                    .col(ColumnDef::new(Sessions::ExpiresAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(Sessions::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Sessions::LastUsedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(ColumnDef::new(Sessions::ClientIp).string())
                    .col(ColumnDef::new(Sessions::UserAgent).string())
                    .col(
                        ColumnDef::new(Sessions::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Sessions::Metadata).json())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sessions_user_id")
                            .from(Sessions::Table, Sessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indices for better performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_users_email")
                    .table(Users::Table)
                    .col(Users::Email)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_users_username")
                    .table(Users::Table)
                    .col(Users::Username)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_api_keys_user_id")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sessions_user_id")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sessions_expires_at")
                    .table(Sessions::Table)
                    .col(Sessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order due to foreign key constraints
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Users table definition
#[derive(Iden)]
pub enum Users {
    Table,
    Id,
    Username,
    Email,
    PasswordHash,
    DisplayName,
    Role,
    IsActive,
    EmailVerified,
    ResetToken,
    ResetTokenExpires,
    CreatedAt,
    UpdatedAt,
    LastLoginAt,
}

/// API Keys table definition
#[derive(Iden)]
pub enum ApiKeys {
    Table,
    Id,
    KeyHash,
    Name,
    UserId,
    KeyPrefix,
    Permissions,
    IsActive,
    ExpiresAt,
    CreatedAt,
    LastUsedAt,
    UsageCount,
}

/// Sessions table definition
#[derive(Iden)]
pub enum Sessions {
    Table,
    Id,
    SessionId,
    UserId,
    JwtId,
    ExpiresAt,
    CreatedAt,
    LastUsedAt,
    ClientIp,
    UserAgent,
    IsActive,
    Metadata,
}