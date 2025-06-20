//! API key repository implementation using SeaORM

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};

use ratchet_api_types::{ApiId, ApiKeyPermissions, ListResponse, PaginationInput, UnifiedApiKey};
use ratchet_interfaces::{database::ApiKeyRepository, CrudRepository, DatabaseError, FilteredRepository, Repository};

use crate::seaorm::{
    connection::DatabaseConnection,
    entities::{api_keys, ApiKeys},
};

/// SeaORM implementation of the ApiKeyRepository
#[derive(Clone)]
pub struct SeaOrmApiKeyRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmApiKeyRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    /// Convert SeaORM API key permissions to unified domain type
    fn to_unified_permissions(permissions: api_keys::ApiKeyPermissions) -> ApiKeyPermissions {
        match permissions {
            api_keys::ApiKeyPermissions::Full => ApiKeyPermissions::Full,
            api_keys::ApiKeyPermissions::ReadOnly => ApiKeyPermissions::ReadOnly,
            api_keys::ApiKeyPermissions::ExecuteOnly => ApiKeyPermissions::ExecuteOnly,
            api_keys::ApiKeyPermissions::Admin => ApiKeyPermissions::Admin,
        }
    }

    /// Convert unified domain permissions to SeaORM type
    fn to_seaorm_permissions(permissions: ApiKeyPermissions) -> api_keys::ApiKeyPermissions {
        match permissions {
            ApiKeyPermissions::Full => api_keys::ApiKeyPermissions::Full,
            ApiKeyPermissions::ReadOnly => api_keys::ApiKeyPermissions::ReadOnly,
            ApiKeyPermissions::ExecuteOnly => api_keys::ApiKeyPermissions::ExecuteOnly,
            ApiKeyPermissions::Admin => api_keys::ApiKeyPermissions::Admin,
        }
    }

    /// Convert SeaORM API key model to unified domain type
    fn to_unified_api_key(model: api_keys::Model) -> UnifiedApiKey {
        UnifiedApiKey {
            id: ApiId::from_i32(model.id),
            name: model.name,
            user_id: ApiId::from_i32(model.user_id),
            key_prefix: model.key_prefix,
            permissions: Self::to_unified_permissions(model.permissions),
            is_active: model.is_active,
            expires_at: model.expires_at,
            created_at: model.created_at,
            last_used_at: model.last_used_at,
            usage_count: model.usage_count,
        }
    }

    /// Convert unified domain type to SeaORM active model for creation
    fn to_active_model_for_create(api_key: &UnifiedApiKey, key_hash: &str) -> api_keys::ActiveModel {
        api_keys::ActiveModel {
            id: Set(api_key.id.as_i32().unwrap_or(0)),
            key_hash: Set(key_hash.to_string()),
            name: Set(api_key.name.clone()),
            user_id: Set(api_key.user_id.as_i32().unwrap_or(0)),
            key_prefix: Set(api_key.key_prefix.clone()),
            permissions: Set(Self::to_seaorm_permissions(api_key.permissions)),
            is_active: Set(api_key.is_active),
            expires_at: Set(api_key.expires_at),
            created_at: Set(api_key.created_at),
            last_used_at: Set(api_key.last_used_at),
            usage_count: Set(api_key.usage_count),
        }
    }

    /// Convert unified domain type to SeaORM active model for updates
    fn to_active_model_for_update(id: i32, api_key: &UnifiedApiKey) -> api_keys::ActiveModel {
        api_keys::ActiveModel {
            id: Set(id),
            key_hash: Default::default(), // Don't update key hash
            name: Set(api_key.name.clone()),
            user_id: Set(api_key.user_id.as_i32().unwrap_or(0)),
            key_prefix: Set(api_key.key_prefix.clone()),
            permissions: Set(Self::to_seaorm_permissions(api_key.permissions)),
            is_active: Set(api_key.is_active),
            expires_at: Set(api_key.expires_at),
            created_at: Set(api_key.created_at),
            last_used_at: Set(api_key.last_used_at),
            usage_count: Set(api_key.usage_count),
        }
    }
}

#[async_trait]
impl Repository for SeaOrmApiKeyRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simple connection check
        ApiKeys::find()
            .limit(1)
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("API key repository health check failed: {}", e),
            })?;
        Ok(())
    }
}

#[async_trait]
impl CrudRepository<UnifiedApiKey> for SeaOrmApiKeyRepository {
    async fn create(&self, api_key: UnifiedApiKey) -> Result<UnifiedApiKey, DatabaseError> {
        // Note: This method requires key_hash but UnifiedApiKey doesn't contain it
        // This is a limitation - the interface should probably be updated
        // For now, we'll generate a placeholder hash
        let placeholder_hash = format!("hash_{}", api_key.key_prefix);
        let active_model = Self::to_active_model_for_create(&api_key, &placeholder_hash);

        let result = active_model
            .insert(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to create API key: {}", e),
            })?;

        Ok(Self::to_unified_api_key(result))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedApiKey>, DatabaseError> {
        let model = ApiKeys::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API key by id: {}", e),
            })?;

        Ok(model.map(Self::to_unified_api_key))
    }

    async fn find_by_uuid(&self, _uuid: uuid::Uuid) -> Result<Option<UnifiedApiKey>, DatabaseError> {
        // API keys don't have UUIDs in the current schema, so return None
        Ok(None)
    }

    async fn update(&self, api_key: UnifiedApiKey) -> Result<UnifiedApiKey, DatabaseError> {
        let active_model = Self::to_active_model_for_update(api_key.id.as_i32().unwrap_or(0), &api_key);

        let result = active_model
            .update(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to update API key: {}", e),
            })?;

        Ok(Self::to_unified_api_key(result))
    }

    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        ApiKeys::delete_by_id(id)
            .exec(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to delete API key: {}", e),
            })?;

        Ok(())
    }

    async fn count(&self) -> Result<u64, DatabaseError> {
        ApiKeys::find()
            .count(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to count API keys: {}", e),
            })
    }
}

// For now, implement an empty filtered repository - API keys don't need complex filtering
#[async_trait]
impl FilteredRepository<UnifiedApiKey, ()> for SeaOrmApiKeyRepository {
    async fn find_with_filters(
        &self,
        _filters: (),
        pagination: PaginationInput,
    ) -> Result<ListResponse<UnifiedApiKey>, DatabaseError> {
        // Apply pagination
        let offset = pagination.get_offset() as u64;
        let limit = pagination.limit.unwrap_or(50) as u64;

        let paginator = ApiKeys::find()
            .order_by_desc(api_keys::Column::CreatedAt)
            .paginate(self.db.get_connection(), limit);
        let page_number = offset / limit;

        let api_keys = paginator
            .fetch_page(page_number)
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to fetch API keys: {}", e),
            })?;

        let total = paginator.num_items().await.map_err(|e| DatabaseError::Internal {
            message: format!("Failed to count API keys: {}", e),
        })?;

        let items: Vec<UnifiedApiKey> = api_keys.into_iter().map(Self::to_unified_api_key).collect();

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
        _filters: (),
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedApiKey>, DatabaseError> {
        let pagination = list_input.pagination.unwrap_or_default();
        self.find_with_filters(_filters, pagination).await
    }

    async fn count_with_filters(&self, _filters: ()) -> Result<u64, DatabaseError> {
        self.count().await
    }
}

#[async_trait]
impl ApiKeyRepository for SeaOrmApiKeyRepository {
    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<UnifiedApiKey>, DatabaseError> {
        let model = ApiKeys::find()
            .filter(api_keys::Column::KeyHash.eq(key_hash))
            .filter(api_keys::Column::IsActive.eq(true))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API key by hash: {}", e),
            })?;

        // Check expiration if key exists
        if let Some(ref model) = model {
            if let Some(expires_at) = model.expires_at {
                if expires_at < Utc::now() {
                    return Ok(None); // Expired key
                }
            }
        }

        Ok(model.map(Self::to_unified_api_key))
    }

    async fn find_by_user_id(&self, user_id: ApiId) -> Result<Vec<UnifiedApiKey>, DatabaseError> {
        let models = ApiKeys::find()
            .filter(api_keys::Column::UserId.eq(user_id.as_i32().unwrap_or(0)))
            .order_by_desc(api_keys::Column::CreatedAt)
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API keys by user_id: {}", e),
            })?;

        Ok(models.into_iter().map(Self::to_unified_api_key).collect())
    }

    async fn create_api_key(
        &self,
        user_id: ApiId,
        name: &str,
        key_hash: &str,
        key_prefix: &str,
        permissions: &str,
    ) -> Result<UnifiedApiKey, DatabaseError> {
        let now = Utc::now();

        // Parse permissions string
        let permissions_enum = match permissions {
            "full" => api_keys::ApiKeyPermissions::Full,
            "read" => api_keys::ApiKeyPermissions::ReadOnly,
            "execute" => api_keys::ApiKeyPermissions::ExecuteOnly,
            "admin" => api_keys::ApiKeyPermissions::Admin,
            _ => api_keys::ApiKeyPermissions::ReadOnly, // Default to read-only
        };

        let active_model = api_keys::ActiveModel {
            id: Default::default(), // Auto-generated
            key_hash: Set(key_hash.to_string()),
            name: Set(name.to_string()),
            user_id: Set(user_id.as_i32().unwrap_or(0)),
            key_prefix: Set(key_prefix.to_string()),
            permissions: Set(permissions_enum),
            is_active: Set(true),
            expires_at: Set(None), // No expiration by default
            created_at: Set(now),
            last_used_at: Set(None),
            usage_count: Set(0),
        };

        let result = active_model
            .insert(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to create API key: {}", e),
            })?;

        Ok(Self::to_unified_api_key(result))
    }

    async fn update_last_used(&self, api_key_id: ApiId) -> Result<(), DatabaseError> {
        let api_key = ApiKeys::find_by_id(api_key_id.as_i32().unwrap_or(0))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API key for last_used update: {}", e),
            })?;

        if let Some(api_key) = api_key {
            let mut active_model: api_keys::ActiveModel = api_key.into();
            active_model.last_used_at = Set(Some(Utc::now()));

            active_model
                .update(self.db.get_connection())
                .await
                .map_err(|e| DatabaseError::Internal {
                    message: format!("Failed to update API key last_used: {}", e),
                })?;
        }

        Ok(())
    }

    async fn increment_usage(&self, api_key_id: ApiId) -> Result<(), DatabaseError> {
        let api_key = ApiKeys::find_by_id(api_key_id.as_i32().unwrap_or(0))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API key for usage increment: {}", e),
            })?;

        if let Some(api_key) = api_key {
            let mut active_model: api_keys::ActiveModel = api_key.into();
            active_model.usage_count = Set(active_model.usage_count.unwrap() + 1);
            active_model.last_used_at = Set(Some(Utc::now()));

            active_model
                .update(self.db.get_connection())
                .await
                .map_err(|e| DatabaseError::Internal {
                    message: format!("Failed to increment API key usage: {}", e),
                })?;
        }

        Ok(())
    }

    async fn set_active(&self, api_key_id: ApiId, is_active: bool) -> Result<(), DatabaseError> {
        let api_key = ApiKeys::find_by_id(api_key_id.as_i32().unwrap_or(0))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal {
                message: format!("Failed to find API key for active status update: {}", e),
            })?;

        if let Some(api_key) = api_key {
            let mut active_model: api_keys::ActiveModel = api_key.into();
            active_model.is_active = Set(is_active);

            active_model
                .update(self.db.get_connection())
                .await
                .map_err(|e| DatabaseError::Internal {
                    message: format!("Failed to update API key active status: {}", e),
                })?;
        }

        Ok(())
    }
}
