//! User session entity for JWT token management

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Unique session identifier (JWT jti claim)
    #[sea_orm(unique)]
    pub session_id: String,
    /// User this session belongs to
    pub user_id: i32,
    /// JWT token ID for revocation
    pub jwt_id: String,
    /// Session expiry time
    pub expires_at: DateTimeUtc,
    /// When the session was created
    pub created_at: DateTimeUtc,
    /// When the session was last used
    pub last_used_at: DateTimeUtc,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Whether this session is active
    pub is_active: bool,
    /// Session metadata (JSON)
    pub metadata: Option<String>,
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

impl ActiveModelBehavior for ActiveModel {
    fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        // Update last_used_at on save
        self.last_used_at = Set(chrono::Utc::now());
        Ok(self)
    }
}