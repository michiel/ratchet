//! Session repository implementation using SeaORM

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    PaginatorTrait, Condition,
};

use ratchet_api_types::{ApiId, PaginationInput, ListResponse, UnifiedSession};
use ratchet_interfaces::{
    DatabaseError, CrudRepository, FilteredRepository, Repository,
    database::SessionRepository,
};

use crate::seaorm::{entities::{sessions, Sessions}, connection::DatabaseConnection};

/// SeaORM implementation of the SessionRepository
#[derive(Clone)]
pub struct SeaOrmSessionRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmSessionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    /// Convert SeaORM session model to unified domain type
    fn to_unified_session(model: sessions::Model) -> UnifiedSession {
        UnifiedSession {
            id: ApiId::from_i32(model.id),
            session_id: model.session_id,
            user_id: ApiId::from_i32(model.user_id),
            expires_at: model.expires_at,
            created_at: model.created_at,
            last_used_at: model.last_used_at,
            client_ip: model.client_ip,
            user_agent: model.user_agent,
            is_active: model.is_active,
        }
    }

    /// Convert unified domain type to SeaORM active model for creation
    fn to_active_model_for_create(session: &UnifiedSession) -> sessions::ActiveModel {
        sessions::ActiveModel {
            id: Set(session.id.as_i32().unwrap_or(0)),
            session_id: Set(session.session_id.clone()),
            user_id: Set(session.user_id.as_i32().unwrap_or(0)),
            jwt_id: Set(format!("jwt_{}", session.session_id)), // Derive JWT ID from session ID
            expires_at: Set(session.expires_at),
            created_at: Set(session.created_at),
            last_used_at: Set(session.last_used_at),
            client_ip: Set(session.client_ip.clone()),
            user_agent: Set(session.user_agent.clone()),
            is_active: Set(session.is_active),
            metadata: Set(None), // No metadata for now
        }
    }

    /// Convert unified domain type to SeaORM active model for updates
    fn to_active_model_for_update(id: i32, session: &UnifiedSession) -> sessions::ActiveModel {
        sessions::ActiveModel {
            id: Set(id),
            session_id: Set(session.session_id.clone()),
            user_id: Set(session.user_id.as_i32().unwrap_or(0)),
            jwt_id: Set(format!("jwt_{}", session.session_id)),
            expires_at: Set(session.expires_at),
            created_at: Set(session.created_at),
            last_used_at: Set(session.last_used_at),
            client_ip: Set(session.client_ip.clone()),
            user_agent: Set(session.user_agent.clone()),
            is_active: Set(session.is_active),
            metadata: Set(None),
        }
    }
}

#[async_trait]
impl Repository for SeaOrmSessionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simple connection check
        Sessions::find()
            .limit(1)
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Session repository health check failed: {}", e) 
            })?;
        Ok(())
    }
}

#[async_trait]
impl CrudRepository<UnifiedSession> for SeaOrmSessionRepository {
    async fn create(&self, session: UnifiedSession) -> Result<UnifiedSession, DatabaseError> {
        let active_model = Self::to_active_model_for_create(&session);
        
        let result = active_model.insert(self.db.get_connection()).await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to create session: {}", e) 
            })?;
        
        Ok(Self::to_unified_session(result))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedSession>, DatabaseError> {
        let model = Sessions::find_by_id(id)
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find session by id: {}", e) 
            })?;

        Ok(model.map(Self::to_unified_session))
    }

    async fn find_by_uuid(&self, _uuid: uuid::Uuid) -> Result<Option<UnifiedSession>, DatabaseError> {
        // Sessions don't have UUIDs in the current schema, so return None
        Ok(None)
    }

    async fn update(&self, session: UnifiedSession) -> Result<UnifiedSession, DatabaseError> {
        let active_model = Self::to_active_model_for_update(session.id.to_i32(), &session);
        
        let result = active_model.update(self.db.get_connection()).await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to update session: {}", e) 
            })?;
        
        Ok(Self::to_unified_session(result))
    }

    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        Sessions::delete_by_id(id)
            .exec(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to delete session: {}", e) 
            })?;

        Ok(())
    }


    async fn count(&self) -> Result<u64, DatabaseError> {
        Sessions::find()
            .count(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to count sessions: {}", e) 
            })
    }
}

// For now, implement an empty filtered repository - sessions don't need complex filtering
#[async_trait]
impl FilteredRepository<UnifiedSession, ()> for SeaOrmSessionRepository {
    async fn find_with_filters(
        &self,
        _filters: &(),
        pagination: PaginationInput,
    ) -> Result<ListResponse<UnifiedSession>, DatabaseError> {
        let offset = (pagination.page.saturating_sub(1)) * pagination.per_page;
        
        let paginator = Sessions::find()
            .order_by_desc(sessions::Column::CreatedAt)
            .paginate(self.db.get_connection(), pagination.per_page as u64);

        let total_items = paginator.num_items().await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to count sessions: {}", e) 
            })?;

        let items = paginator
            .fetch_page((pagination.page - 1) as u64)
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to fetch sessions: {}", e) 
            })?
            .into_iter()
            .map(Self::to_unified_session)
            .collect();

        Ok(ListResponse {
            items,
            total_items: total_items as u32,
            page: pagination.page,
            per_page: pagination.per_page,
            total_pages: ((total_items + pagination.per_page as u64 - 1) / pagination.per_page as u64) as u32,
        })
    }

    async fn count_with_filters(&self, _filters: &()) -> Result<u64, DatabaseError> {
        self.count().await
    }
}

#[async_trait]
impl SessionRepository for SeaOrmSessionRepository {
    async fn create_session(
        &self,
        user_id: ApiId,
        session_id: &str,
        jwt_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<UnifiedSession, DatabaseError> {
        let now = Utc::now();
        
        let active_model = sessions::ActiveModel {
            id: Default::default(), // Auto-generated
            session_id: Set(session_id.to_string()),
            user_id: Set(user_id.to_i32()),
            jwt_id: Set(jwt_id.to_string()),
            expires_at: Set(expires_at),
            created_at: Set(now),
            last_used_at: Set(now),
            client_ip: Set(None),
            user_agent: Set(None),
            is_active: Set(true),
            metadata: Set(None),
        };

        let result = active_model.insert(self.db.get_connection()).await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to create session: {}", e) 
            })?;

        Ok(Self::to_unified_session(result))
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Option<UnifiedSession>, DatabaseError> {
        let model = Sessions::find()
            .filter(sessions::Column::SessionId.eq(session_id))
            .filter(sessions::Column::IsActive.eq(true))
            .filter(sessions::Column::ExpiresAt.gt(Utc::now()))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find session by session_id: {}", e) 
            })?;

        Ok(model.map(Self::to_unified_session))
    }

    async fn find_by_user_id(&self, user_id: ApiId) -> Result<Vec<UnifiedSession>, DatabaseError> {
        let models = Sessions::find()
            .filter(sessions::Column::UserId.eq(user_id.to_i32()))
            .filter(sessions::Column::IsActive.eq(true))
            .order_by_desc(sessions::Column::LastUsedAt)
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find sessions by user_id: {}", e) 
            })?;

        Ok(models.into_iter().map(Self::to_unified_session).collect())
    }

    async fn invalidate_session(&self, session_id: &str) -> Result<(), DatabaseError> {
        let session = Sessions::find()
            .filter(sessions::Column::SessionId.eq(session_id))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find session for invalidation: {}", e) 
            })?;

        if let Some(session) = session {
            let mut active_model: sessions::ActiveModel = session.into();
            active_model.is_active = Set(false);
            
            active_model.update(&self.db).await
                .map_err(|e| DatabaseError::Database { 
                    message: format!("Failed to invalidate session: {}", e) 
                })?;
        }

        Ok(())
    }

    async fn invalidate_user_sessions(&self, user_id: ApiId) -> Result<(), DatabaseError> {
        // Find all active sessions for the user
        let sessions = Sessions::find()
            .filter(sessions::Column::UserId.eq(user_id.to_i32()))
            .filter(sessions::Column::IsActive.eq(true))
            .all(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find user sessions for invalidation: {}", e) 
            })?;

        // Invalidate each session
        for session in sessions {
            let mut active_model: sessions::ActiveModel = session.into();
            active_model.is_active = Set(false);
            
            active_model.update(&self.db).await
                .map_err(|e| DatabaseError::Database { 
                    message: format!("Failed to invalidate user session: {}", e) 
                })?;
        }

        Ok(())
    }

    async fn update_last_used(&self, session_id: &str) -> Result<(), DatabaseError> {
        let session = Sessions::find()
            .filter(sessions::Column::SessionId.eq(session_id))
            .filter(sessions::Column::IsActive.eq(true))
            .one(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to find session for last_used update: {}", e) 
            })?;

        if let Some(session) = session {
            let mut active_model: sessions::ActiveModel = session.into();
            active_model.last_used_at = Set(Utc::now());
            
            active_model.update(&self.db).await
                .map_err(|e| DatabaseError::Database { 
                    message: format!("Failed to update session last_used: {}", e) 
                })?;
        }

        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, DatabaseError> {
        let result = Sessions::delete_many()
            .filter(
                Condition::any()
                    .add(sessions::Column::ExpiresAt.lt(Utc::now()))
                    .add(sessions::Column::IsActive.eq(false))
            )
            .exec(self.db.get_connection())
            .await
            .map_err(|e| DatabaseError::Internal { 
                message: format!("Failed to cleanup expired sessions: {}", e) 
            })?;

        Ok(result.rows_affected)
    }
}