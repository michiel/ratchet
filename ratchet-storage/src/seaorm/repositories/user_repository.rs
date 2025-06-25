//! User repository implementation using SeaORM

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set};

use ratchet_api_types::{ApiId, ListResponse, PaginationInput, UnifiedUser};
use ratchet_interfaces::{
    database::{UserFilters, UserRepository},
    CrudRepository, DatabaseError, FilteredRepository,
};

use crate::seaorm::{
    connection::DatabaseConnection,
    entities::{users, Users},
};

/// SeaORM implementation of the UserRepository
#[derive(Clone)]
pub struct SeaOrmUserRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Convert SeaORM user model to unified domain type
    fn to_unified_user(model: users::Model) -> UnifiedUser {
        UnifiedUser {
            id: ApiId::from_i32(model.id),
            username: model.username,
            email: model.email,
            display_name: model.display_name,
            role: match model.role {
                users::UserRole::Admin => ratchet_api_types::UserRole::Admin,
                users::UserRole::User => ratchet_api_types::UserRole::User,
                users::UserRole::ReadOnly => ratchet_api_types::UserRole::ReadOnly,
                users::UserRole::Service => ratchet_api_types::UserRole::Service,
            },
            is_active: model.is_active,
            email_verified: model.email_verified,
            created_at: model.created_at,
            updated_at: model.updated_at,
            last_login_at: model.last_login_at,
        }
    }

    /// Apply filters to user query
    fn apply_filters(
        &self,
        query: sea_orm::Select<users::Entity>,
        filters: &UserFilters,
    ) -> sea_orm::Select<users::Entity> {
        let mut query = query;

        if let Some(username) = &filters.username {
            query = query.filter(users::Column::Username.eq(username));
        }

        if let Some(email) = &filters.email {
            query = query.filter(users::Column::Email.eq(email));
        }

        if let Some(role) = &filters.role {
            query = query.filter(users::Column::Role.eq(role));
        }

        if let Some(is_active) = filters.is_active {
            query = query.filter(users::Column::IsActive.eq(is_active));
        }

        if let Some(email_verified) = filters.email_verified {
            query = query.filter(users::Column::EmailVerified.eq(email_verified));
        }

        if let Some(created_after) = filters.created_after {
            query = query.filter(users::Column::CreatedAt.gte(created_after));
        }

        if let Some(created_before) = filters.created_before {
            query = query.filter(users::Column::CreatedAt.lte(created_before));
        }

        query
    }
}

#[async_trait]
impl CrudRepository<UnifiedUser> for SeaOrmUserRepository {
    async fn create(&self, user: UnifiedUser) -> Result<UnifiedUser, DatabaseError> {
        let active_model = users::ActiveModel {
            username: Set(user.username),
            email: Set(user.email),
            password_hash: Set("".to_string()), // Will be set separately
            display_name: Set(user.display_name),
            role: Set(match user.role {
                ratchet_api_types::UserRole::Admin => users::UserRole::Admin,
                ratchet_api_types::UserRole::User => users::UserRole::User,
                ratchet_api_types::UserRole::ReadOnly => users::UserRole::ReadOnly,
                ratchet_api_types::UserRole::Service => users::UserRole::Service,
            }),
            is_active: Set(user.is_active),
            email_verified: Set(user.email_verified),
            created_at: Set(user.created_at),
            updated_at: Set(user.updated_at),
            last_login_at: Set(user.last_login_at),
            ..Default::default()
        };

        let result = active_model
            .insert(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to create user: {}", e),
            })?;

        Ok(Self::to_unified_user(result))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedUser>, DatabaseError> {
        let user = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user by id: {}", e),
            })?;

        Ok(user.map(Self::to_unified_user))
    }

    async fn find_by_uuid(&self, _uuid: uuid::Uuid) -> Result<Option<UnifiedUser>, DatabaseError> {
        // Note: Users table doesn't have UUID field, using ID instead
        // This is a placeholder implementation
        Err(DatabaseError::Internal {
            message: "UUID lookup not supported for users".to_string(),
        })
    }

    async fn update(&self, user: UnifiedUser) -> Result<UnifiedUser, DatabaseError> {
        let id = user.id.as_i32().ok_or_else(|| DatabaseError::Validation {
            message: "Invalid user ID".to_string(),
        })?;

        let existing = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user for update: {}", e),
            })?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            })?;

        let active_model = users::ActiveModel {
            id: Set(existing.id),
            username: Set(user.username),
            email: Set(user.email),
            password_hash: Set(existing.password_hash), // Keep existing password
            display_name: Set(user.display_name),
            role: Set(match user.role {
                ratchet_api_types::UserRole::Admin => users::UserRole::Admin,
                ratchet_api_types::UserRole::User => users::UserRole::User,
                ratchet_api_types::UserRole::ReadOnly => users::UserRole::ReadOnly,
                ratchet_api_types::UserRole::Service => users::UserRole::Service,
            }),
            is_active: Set(user.is_active),
            email_verified: Set(user.email_verified),
            created_at: Set(existing.created_at), // Keep original
            updated_at: Set(Utc::now()),
            last_login_at: Set(user.last_login_at),
            reset_token: Set(existing.reset_token),
            reset_token_expires: Set(existing.reset_token_expires),
        };

        let updated = active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to update user: {}", e),
            })?;

        Ok(Self::to_unified_user(updated))
    }

    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        let result = Users::delete_by_id(id)
            .exec(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to delete user: {}", e),
            })?;

        if result.rows_affected == 0 {
            return Err(DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    async fn count(&self) -> Result<u64, DatabaseError> {
        Users::find()
            .count(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to count users: {}", e),
            })
    }
}

#[async_trait]
impl FilteredRepository<UnifiedUser, UserFilters> for SeaOrmUserRepository {
    async fn find_with_filters(
        &self,
        filters: UserFilters,
        pagination: PaginationInput,
    ) -> Result<ListResponse<UnifiedUser>, DatabaseError> {
        let query = Users::find();
        let query = self.apply_filters(query, &filters);

        // Apply pagination
        let offset = pagination.get_offset() as u64;
        let limit = pagination.limit.unwrap_or(50) as u64;

        let paginator = query.paginate(self.db.get_connection(), limit);
        let page_number = offset / limit;

        let users = paginator
            .fetch_page(page_number)
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to fetch users: {}", e),
            })?;

        let total = paginator.num_items().await.map_err(|e| DatabaseError::Internal {
            message: format!("Failed to count users: {}", e),
        })?;

        let items: Vec<UnifiedUser> = users.into_iter().map(Self::to_unified_user).collect();

        Ok(ListResponse {
            items,
            meta: ratchet_api_types::pagination::PaginationMeta {
                page: (page_number + 1) as u32,
                limit: limit as u32,
                total,
                total_pages: total.div_ceil(limit) as u32,
                has_previous: page_number > 0,
                has_next: (page_number + 1) * limit < total,
                offset: offset as u32,
            },
        })
    }

    async fn find_with_list_input(
        &self,
        filters: UserFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedUser>, DatabaseError> {
        let pagination = list_input.pagination.unwrap_or_default();
        self.find_with_filters(filters, pagination).await
    }

    async fn count_with_filters(&self, filters: UserFilters) -> Result<u64, DatabaseError> {
        let query = Users::find();
        let query = self.apply_filters(query, &filters);

        query
            .count(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to count users with filters: {}", e),
            })
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn find_by_username(&self, username: &str) -> Result<Option<UnifiedUser>, DatabaseError> {
        let user = Users::find()
            .filter(users::Column::Username.eq(username))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user by username: {}", e),
            })?;

        Ok(user.map(Self::to_unified_user))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UnifiedUser>, DatabaseError> {
        let user = Users::find()
            .filter(users::Column::Email.eq(email))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user by email: {}", e),
            })?;

        Ok(user.map(Self::to_unified_user))
    }

    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<UnifiedUser, DatabaseError> {
        let user_role = match role {
            "admin" => users::UserRole::Admin,
            "user" => users::UserRole::User,
            "readonly" => users::UserRole::ReadOnly,
            "service" => users::UserRole::Service,
            _ => users::UserRole::User,
        };

        let now = Utc::now();
        let active_model = users::ActiveModel {
            username: Set(username.to_string()),
            email: Set(email.to_string()),
            password_hash: Set(password_hash.to_string()),
            display_name: Set(None),
            role: Set(user_role),
            is_active: Set(true),
            email_verified: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(None),
            reset_token: Set(None),
            reset_token_expires: Set(None),
            ..Default::default()
        };

        let result = active_model
            .insert(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to create user: {}", e),
            })?;

        Ok(Self::to_unified_user(result))
    }

    async fn update_password(&self, user_id: ApiId, password_hash: &str) -> Result<(), DatabaseError> {
        let id = user_id.as_i32().ok_or_else(|| DatabaseError::Validation {
            message: "Invalid user ID".to_string(),
        })?;

        let user = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user for password update: {}", e),
            })?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            })?;

        let active_model = users::ActiveModel {
            id: Set(user.id),
            password_hash: Set(password_hash.to_string()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to update password: {}", e),
            })?;

        Ok(())
    }

    async fn update_last_login(&self, user_id: ApiId) -> Result<(), DatabaseError> {
        let id = user_id.as_i32().ok_or_else(|| DatabaseError::Validation {
            message: "Invalid user ID".to_string(),
        })?;

        let user = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user for login update: {}", e),
            })?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            })?;

        let active_model = users::ActiveModel {
            id: Set(user.id),
            last_login_at: Set(Some(Utc::now())),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to update last login: {}", e),
            })?;

        Ok(())
    }

    async fn set_active(&self, user_id: ApiId, is_active: bool) -> Result<(), DatabaseError> {
        let id = user_id.as_i32().ok_or_else(|| DatabaseError::Validation {
            message: "Invalid user ID".to_string(),
        })?;

        let user = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user for active status update: {}", e),
            })?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            })?;

        let active_model = users::ActiveModel {
            id: Set(user.id),
            is_active: Set(is_active),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to update active status: {}", e),
            })?;

        Ok(())
    }

    async fn verify_email(&self, user_id: ApiId) -> Result<(), DatabaseError> {
        let id = user_id.as_i32().ok_or_else(|| DatabaseError::Validation {
            message: "Invalid user ID".to_string(),
        })?;

        let user = Users::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find user for email verification: {}", e),
            })?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "User".to_string(),
                id: id.to_string(),
            })?;

        let active_model = users::ActiveModel {
            id: Set(user.id),
            email_verified: Set(true),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to verify email: {}", e),
            })?;

        Ok(())
    }
}

#[async_trait]
impl ratchet_interfaces::Repository for SeaOrmUserRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        Users::find()
            .limit(1)
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Connection {
                message: format!("User repository health check failed: {}", e),
            })?;

        Ok(())
    }
}
