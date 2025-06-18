//! User entity for authentication and authorization

use sea_orm::entity::prelude::*;
use sea_query::StringLen;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Unique username for login
    #[sea_orm(unique)]
    pub username: String,
    /// Email address (unique)
    #[sea_orm(unique)]
    pub email: String,
    /// Password hash (bcrypt)
    pub password_hash: String,
    /// Display name
    pub display_name: Option<String>,
    /// User role for RBAC
    pub role: UserRole,
    /// Whether the user account is active
    pub is_active: bool,
    /// Whether email is verified
    pub email_verified: bool,
    /// Password reset token (if any)
    pub reset_token: Option<String>,
    /// When reset token expires
    pub reset_token_expires: Option<DateTimeUtc>,
    /// When the user was created
    pub created_at: DateTimeUtc,
    /// When the user was last updated
    pub updated_at: DateTimeUtc,
    /// When the user last logged in
    pub last_login_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::api_keys::Entity")]
    ApiKeys,
    #[sea_orm(has_many = "super::sessions::Entity")]
    Sessions,
}

impl Related<super::api_keys::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiKeys.def()
    }
}

impl Related<super::sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// User roles for role-based access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[derive(Default)]
pub enum UserRole {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "user")]
    #[default]
    User,
    #[sea_orm(string_value = "readonly")]
    ReadOnly,
    #[sea_orm(string_value = "service")]
    Service,
}


impl UserRole {
    /// Check if this role can perform admin operations
    pub fn can_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Check if this role can write/modify resources
    pub fn can_write(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User | UserRole::Service)
    }

    /// Check if this role can read resources
    pub fn can_read(&self) -> bool {
        // All roles can read
        true
    }

    /// Check if this role can manage users
    pub fn can_manage_users(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Check if this role can execute tasks
    pub fn can_execute_tasks(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User | UserRole::Service)
    }
}