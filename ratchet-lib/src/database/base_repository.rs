use async_trait::async_trait;
use sea_orm::*;
use crate::database::{DatabaseConnection, SafeDatabaseError, SafeDatabaseResult};

/// Base repository trait with common CRUD operations
#[async_trait]
pub trait BaseRepository<E, M, AM>
where
    E: EntityTrait,
    M: FromQueryResult + Sized + Send + Sync,
    AM: ActiveModelTrait<Entity = E> + Send + Sync,
{
    /// Get reference to the database connection
    fn db(&self) -> &DatabaseConnection;

    /// Create a new record
    async fn create(&self, active_model: AM) -> SafeDatabaseResult<M> {
        let result = active_model
            .insert(self.db().get_connection())
            .await?;
        Ok(result.try_into_model()?)
    }

    /// Find record by primary key
    async fn find_by_id(&self, id: impl Into<Value> + Send) -> SafeDatabaseResult<Option<M>> {
        let result = E::find_by_id(id)
            .one(self.db().get_connection())
            .await?;
        Ok(result)
    }

    /// Update an existing record
    async fn update(&self, active_model: AM) -> SafeDatabaseResult<M> {
        let result = active_model
            .update(self.db().get_connection())
            .await?;
        Ok(result.try_into_model()?)
    }

    /// Delete record by primary key
    async fn delete_by_id(&self, id: impl Into<Value> + Send) -> SafeDatabaseResult<DeleteResult> {
        let result = E::delete_by_id(id)
            .exec(self.db().get_connection())
            .await?;
        Ok(result)
    }

    /// Count all records
    async fn count_all(&self) -> SafeDatabaseResult<u64> {
        let count = E::find()
            .count(self.db().get_connection())
            .await?;
        Ok(count)
    }

    /// Find all records with pagination
    async fn find_all_paginated(
        &self, 
        limit: Option<u64>, 
        offset: Option<u64>
    ) -> SafeDatabaseResult<Vec<M>> {
        let mut query = E::find();

        if let Some(limit) = limit {
            query = query.limit(limit);
        }
        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let results = query
            .all(self.db().get_connection())
            .await?;
        Ok(results)
    }

    /// Health check - verify database connectivity
    async fn health_check(&self) -> SafeDatabaseResult<()> {
        self.db().ping().await?;
        Ok(())
    }
}

/// Transaction manager for database operations
pub struct TransactionManager {
    db: DatabaseConnection,
}

impl TransactionManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Execute multiple operations in a transaction
    pub async fn execute<F, R, Fut>(&self, f: F) -> SafeDatabaseResult<R>
    where
        F: FnOnce(&DatabaseTransaction) -> Fut + Send,
        Fut: std::future::Future<Output = SafeDatabaseResult<R>> + Send,
        R: Send,
    {
        let txn = self.db.get_connection().begin().await?;

        let result = f(&txn).await;
        
        match result {
            Ok(value) => {
                txn.commit().await?;
                Ok(value)
            }
            Err(e) => {
                let _ = txn.rollback().await;
                Err(e)
            }
        }
    }

    /// Execute with retry logic for deadlock/serialization failures
    pub async fn execute_with_retry<F, R, Fut>(
        &self,
        f: F,
        max_retries: u32,
    ) -> SafeDatabaseResult<R>
    where
        F: Fn(&DatabaseTransaction) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = SafeDatabaseResult<R>> + Send,
        R: Send,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < max_retries {
            let txn = self.db.get_connection().begin().await?;

            match f(&txn).await {
                Ok(result) => {
                    txn.commit().await?;
                    return Ok(result);
                }
                Err(e) if Self::is_retryable_error(&e) => {
                    let _ = txn.rollback().await;
                    attempts += 1;
                    last_error = Some(e);
                    
                    // Exponential backoff
                    let delay = std::time::Duration::from_millis(100 * 2_u64.pow(attempts));
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    let _ = txn.rollback().await;
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            SafeDatabaseError::new(
                crate::database::ErrorCode::InternalError,
                "Transaction failed after maximum retries"
            )
        }))
    }

    fn is_retryable_error(error: &SafeDatabaseError) -> bool {
        // Check if error indicates deadlock, serialization failure, or temporary unavailability
        match &error.code {
            crate::database::ErrorCode::Timeout => true,
            crate::database::ErrorCode::ServiceUnavailable => true,
            _ => {
                // Check error message for specific database error patterns
                let msg = error.message.to_lowercase();
                msg.contains("deadlock") || 
                msg.contains("serialization") || 
                msg.contains("lock timeout") ||
                msg.contains("connection")
            }
        }
    }
}

/// Macro to implement BaseRepository for a repository struct
#[macro_export]
macro_rules! impl_base_repository {
    ($repo:ty, $entity:ty, $model:ty, $active_model:ty) => {
        #[async_trait::async_trait]
        impl $crate::database::base_repository::BaseRepository<$entity, $model, $active_model> for $repo {
            fn db(&self) -> &$crate::database::DatabaseConnection {
                &self.db
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_retryable_error_detection() {
        let deadlock_error = SafeDatabaseError::new(
            crate::database::ErrorCode::InternalError,
            "deadlock detected"
        );
        assert!(TransactionManager::is_retryable_error(&deadlock_error));

        let timeout_error = SafeDatabaseError::new(
            crate::database::ErrorCode::Timeout,
            "operation timed out"
        );
        assert!(TransactionManager::is_retryable_error(&timeout_error));

        let validation_error = SafeDatabaseError::new(
            crate::database::ErrorCode::ValidationError,
            "invalid input"
        );
        assert!(!TransactionManager::is_retryable_error(&validation_error));
    }
}