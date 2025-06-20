//! API key entity for service authentication

use sea_orm::entity::prelude::*;
use sea_query::StringLen;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The API key hash (not the actual key)
    #[sea_orm(unique)]
    pub key_hash: String,
    /// Human-readable name for this API key
    pub name: String,
    /// User who owns this API key
    pub user_id: i32,
    /// Key prefix for identification (first 8 chars)
    pub key_prefix: String,
    /// Permissions for this API key
    pub permissions: ApiKeyPermissions,
    /// Whether this key is active
    pub is_active: bool,
    /// When this key expires (if any)
    pub expires_at: Option<DateTimeUtc>,
    /// When the key was created
    pub created_at: DateTimeUtc,
    /// When the key was last used
    pub last_used_at: Option<DateTimeUtc>,
    /// How many times this key has been used
    pub usage_count: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// API key permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[derive(Default)]
pub enum ApiKeyPermissions {
    #[sea_orm(string_value = "full")]
    Full,
    #[sea_orm(string_value = "read")]
    #[default]
    ReadOnly,
    #[sea_orm(string_value = "execute")]
    ExecuteOnly,
    #[sea_orm(string_value = "admin")]
    Admin,
}

impl ApiKeyPermissions {
    /// Check if this permission allows reading
    pub fn can_read(&self) -> bool {
        true // All API keys can read
    }

    /// Check if this permission allows writing
    pub fn can_write(&self) -> bool {
        matches!(self, ApiKeyPermissions::Full | ApiKeyPermissions::Admin)
    }

    /// Check if this permission allows task execution
    pub fn can_execute(&self) -> bool {
        matches!(
            self,
            ApiKeyPermissions::Full | ApiKeyPermissions::ExecuteOnly | ApiKeyPermissions::Admin
        )
    }

    /// Check if this permission allows admin operations
    pub fn can_admin(&self) -> bool {
        matches!(self, ApiKeyPermissions::Admin)
    }
}
