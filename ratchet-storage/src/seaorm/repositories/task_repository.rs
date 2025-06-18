use crate::seaorm::{
    connection::{DatabaseConnection, DatabaseError},
    entities::{tasks, Task, TaskActiveModel, Tasks},
    filters::{validation, SafeFilterBuilder},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Filter criteria for task queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilters {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub has_validation: Option<bool>,
    pub version: Option<String>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
}

/// Repository for task-related database operations
#[derive(Clone)]
pub struct TaskRepository {
    db: DatabaseConnection,
}

impl TaskRepository {
    /// Create a new task repository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new task
    pub async fn create(&self, task: Task) -> Result<Task, DatabaseError> {
        let active_model = TaskActiveModel {
            uuid: Set(task.uuid),
            name: Set(task.name),
            description: Set(task.description),
            version: Set(task.version),
            path: Set(task.path),
            metadata: Set(task.metadata),
            input_schema: Set(task.input_schema),
            output_schema: Set(task.output_schema),
            enabled: Set(task.enabled),
            created_at: Set(task.created_at),
            updated_at: Set(task.updated_at),
            validated_at: Set(task.validated_at),
            ..Default::default()
        };

        let result = active_model.insert(self.db.get_connection()).await?;
        Ok(result)
    }

    /// Find task by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Task>, DatabaseError> {
        let task = Tasks::find_by_id(id).one(self.db.get_connection()).await?;
        Ok(task)
    }

    /// Find task by UUID
    pub async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<Task>, DatabaseError> {
        let task = Tasks::find()
            .filter(tasks::Column::Uuid.eq(uuid))
            .one(self.db.get_connection())
            .await?;
        Ok(task)
    }

    /// Find task by name
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Task>, DatabaseError> {
        let task = Tasks::find()
            .filter(tasks::Column::Name.eq(name))
            .one(self.db.get_connection())
            .await?;
        Ok(task)
    }

    /// Find all tasks
    pub async fn find_all(&self) -> Result<Vec<Task>, DatabaseError> {
        let tasks = Tasks::find().all(self.db.get_connection()).await?;
        Ok(tasks)
    }

    /// Find all enabled tasks
    pub async fn find_enabled(&self) -> Result<Vec<Task>, DatabaseError> {
        let tasks = Tasks::find()
            .filter(tasks::Column::Enabled.eq(true))
            .all(self.db.get_connection())
            .await?;
        Ok(tasks)
    }

    /// Update a task
    pub async fn update(&self, task: Task) -> Result<Task, DatabaseError> {
        let active_model = TaskActiveModel {
            id: Set(task.id),
            uuid: Set(task.uuid),
            name: Set(task.name),
            description: Set(task.description),
            version: Set(task.version),
            path: Set(task.path),
            metadata: Set(task.metadata),
            input_schema: Set(task.input_schema),
            output_schema: Set(task.output_schema),
            enabled: Set(task.enabled),
            created_at: Set(task.created_at), // Keep original creation time
            updated_at: Set(chrono::Utc::now()), // Update the timestamp
            validated_at: Set(task.validated_at),
        };

        let updated_task = active_model.update(self.db.get_connection()).await?;
        Ok(updated_task)
    }

    /// Update task validation timestamp
    pub async fn mark_validated(&self, id: i32) -> Result<(), DatabaseError> {
        let active_model = TaskActiveModel {
            id: Set(id),
            validated_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Enable or disable a task
    pub async fn set_enabled(&self, id: i32, enabled: bool) -> Result<(), DatabaseError> {
        let active_model = TaskActiveModel {
            id: Set(id),
            enabled: Set(enabled),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Set task synchronization status (for registry sync tracking)
    pub async fn set_in_sync(&self, id: i32, in_sync: bool) -> Result<(), DatabaseError> {
        // Note: This would ideally require an 'in_sync' column in the tasks table
        // For now, we'll store the sync status in the metadata JSON field
        if let Ok(Some(task)) = self.find_by_id(id).await {
            let mut metadata = task.metadata;
            if let Some(metadata_obj) = metadata.as_object_mut() {
                metadata_obj.insert("in_sync".to_string(), serde_json::Value::Bool(in_sync));
                metadata_obj.insert("last_sync_check".to_string(), 
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            }

            let active_model = TaskActiveModel {
                id: Set(id),
                metadata: Set(metadata),
                updated_at: Set(chrono::Utc::now()),
                ..Default::default()
            };

            active_model.update(self.db.get_connection()).await?;
        }
        Ok(())
    }

    /// Delete a task by ID
    pub async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        Tasks::delete_by_id(id)
            .exec(self.db.get_connection())
            .await?;
        Ok(())
    }

    /// Delete a task by UUID
    pub async fn delete_by_uuid(&self, uuid: Uuid) -> Result<(), DatabaseError> {
        Tasks::delete_many()
            .filter(tasks::Column::Uuid.eq(uuid))
            .exec(self.db.get_connection())
            .await?;
        Ok(())
    }

    /// Count total tasks
    pub async fn count(&self) -> Result<u64, DatabaseError> {
        let count = Tasks::find().count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Count enabled tasks
    pub async fn count_enabled(&self) -> Result<u64, DatabaseError> {
        let count = Tasks::find()
            .filter(tasks::Column::Enabled.eq(true))
            .count(self.db.get_connection())
            .await?;
        Ok(count)
    }

    /// Check if task name exists
    pub async fn name_exists(&self, name: &str) -> Result<bool, DatabaseError> {
        let count = Tasks::find()
            .filter(tasks::Column::Name.eq(name))
            .count(self.db.get_connection())
            .await?;
        Ok(count > 0)
    }

    /// Check if task UUID exists
    pub async fn uuid_exists(&self, uuid: Uuid) -> Result<bool, DatabaseError> {
        let count = Tasks::find()
            .filter(tasks::Column::Uuid.eq(uuid))
            .count(self.db.get_connection())
            .await?;
        Ok(count > 0)
    }

    /// Find tasks with safe filtering and pagination
    pub async fn find_with_filters(
        &self,
        filters: TaskFilters,
        pagination: Pagination,
    ) -> Result<Vec<Task>, DatabaseError> {
        // Validate filter inputs
        if let Some(ref name) = filters.name {
            validation::validate_query_input(name)?;
        }
        if let Some(ref version) = filters.version {
            validation::validate_query_input(version)?;
        }

        let mut query = Tasks::find();
        let mut filter_builder = SafeFilterBuilder::<tasks::Entity>::new();

        // Apply filters safely
        if let Some(name) = filters.name {
            filter_builder.add_like_filter(tasks::Column::Name, &name);
        }

        filter_builder.add_optional_filter(tasks::Column::Enabled, filters.enabled);

        if let Some(version) = filters.version {
            filter_builder.add_exact_filter(tasks::Column::Version, version);
        }

        if let Some(has_validation) = filters.has_validation {
            if has_validation {
                filter_builder.add_condition(tasks::Column::ValidatedAt.is_not_null());
            } else {
                filter_builder.add_condition(tasks::Column::ValidatedAt.is_null());
            }
        }

        query = query.filter(filter_builder.build());

        // Apply ordering
        if let Some(order_by) = pagination.order_by {
            match order_by.as_str() {
                "name" => {
                    query = if pagination.order_desc.unwrap_or(false) {
                        query.order_by_desc(tasks::Column::Name)
                    } else {
                        query.order_by_asc(tasks::Column::Name)
                    };
                }
                "created_at" => {
                    query = if pagination.order_desc.unwrap_or(false) {
                        query.order_by_desc(tasks::Column::CreatedAt)
                    } else {
                        query.order_by_asc(tasks::Column::CreatedAt)
                    };
                }
                "updated_at" => {
                    query = if pagination.order_desc.unwrap_or(false) {
                        query.order_by_desc(tasks::Column::UpdatedAt)
                    } else {
                        query.order_by_asc(tasks::Column::UpdatedAt)
                    };
                }
                _ => {
                    // Default to ID ordering for unknown fields
                    query = query.order_by_asc(tasks::Column::Id);
                }
            }
        } else {
            query = query.order_by_asc(tasks::Column::Id);
        }

        // Apply pagination
        if let Some(limit) = pagination.limit {
            query = query.limit(limit);
        }
        if let Some(offset) = pagination.offset {
            query = query.offset(offset);
        }

        let tasks = query.all(self.db.get_connection()).await?;
        Ok(tasks)
    }

    /// Count tasks with safe filtering
    pub async fn count_with_filters(&self, filters: TaskFilters) -> Result<u64, DatabaseError> {
        // Validate filter inputs
        if let Some(ref name) = filters.name {
            validation::validate_query_input(name)?;
        }
        if let Some(ref version) = filters.version {
            validation::validate_query_input(version)?;
        }

        let mut query = Tasks::find();
        let mut filter_builder = SafeFilterBuilder::<tasks::Entity>::new();

        // Apply same filters as find_with_filters
        if let Some(name) = filters.name {
            filter_builder.add_like_filter(tasks::Column::Name, &name);
        }

        filter_builder.add_optional_filter(tasks::Column::Enabled, filters.enabled);

        if let Some(version) = filters.version {
            filter_builder.add_exact_filter(tasks::Column::Version, version);
        }

        if let Some(has_validation) = filters.has_validation {
            if has_validation {
                filter_builder.add_condition(tasks::Column::ValidatedAt.is_not_null());
            } else {
                filter_builder.add_condition(tasks::Column::ValidatedAt.is_null());
            }
        }

        query = query.filter(filter_builder.build());
        let count = query.count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Send-compatible health check method for GraphQL resolvers
    pub async fn health_check_send(&self) -> Result<(), DatabaseError> {
        // Direct implementation to avoid ?Send trait issues
        // Simple health check - try to count tasks
        self.count().await?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl super::Repository for TaskRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simple health check - try to count tasks
        self.count().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::entities::Task;
    use crate::database::repositories::Repository;
    use crate::seaorm::config::DatabaseConfig;

    use serde_json::json;
    use std::time::Duration;

    async fn create_test_db() -> DatabaseConnection {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        };

        let db = DatabaseConnection::new(config).await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    fn create_sample_task() -> Task {
        Task {
            id: 0,
            uuid: Uuid::new_v4(),
            name: "test-task".to_string(),
            description: Some("Test task description".to_string()),
            version: "1.0.0".to_string(),
            path: "/path/to/task".to_string(),
            metadata: json!({"test": "metadata"}),
            input_schema: json!({"type": "object"}),
            output_schema: json!({"type": "object"}),
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_find_task() {
        let db = create_test_db().await;
        let repo = TaskRepository::new(db);

        let task = create_sample_task();
        let task_uuid = task.uuid;

        // Create task
        let created_task = repo.create(task).await.unwrap();
        assert!(created_task.id > 0);
        assert_eq!(created_task.uuid, task_uuid);

        // Find by ID
        let found_task = repo.find_by_id(created_task.id).await.unwrap();
        assert!(found_task.is_some());
        assert_eq!(found_task.unwrap().uuid, task_uuid);

        // Find by UUID
        let found_task = repo.find_by_uuid(task_uuid).await.unwrap();
        assert!(found_task.is_some());
        assert_eq!(found_task.unwrap().name, "test-task");
    }

    #[tokio::test]
    async fn test_update_task() {
        let db = create_test_db().await;
        let repo = TaskRepository::new(db);

        let task = create_sample_task();
        let created_task = repo.create(task).await.unwrap();

        // Update task
        let mut updated_task = created_task.clone();
        updated_task.name = "updated-task".to_string();
        updated_task.description = Some("Updated description".to_string());

        let result = repo.update(updated_task).await.unwrap();
        assert_eq!(result.name, "updated-task");
        assert_eq!(result.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_enable_disable_task() {
        let db = create_test_db().await;
        let repo = TaskRepository::new(db);

        let task = create_sample_task();
        let created_task = repo.create(task).await.unwrap();

        // Disable task
        repo.set_enabled(created_task.id, false).await.unwrap();
        let found_task = repo.find_by_id(created_task.id).await.unwrap().unwrap();
        assert!(!found_task.enabled);

        // Enable task
        repo.set_enabled(created_task.id, true).await.unwrap();
        let found_task = repo.find_by_id(created_task.id).await.unwrap().unwrap();
        assert!(found_task.enabled);
    }

    #[tokio::test]
    async fn test_count_and_exists() {
        let db = create_test_db().await;
        let repo = TaskRepository::new(db);

        assert_eq!(repo.count().await.unwrap(), 0);
        assert_eq!(repo.count_enabled().await.unwrap(), 0);

        let task = create_sample_task();
        let task_uuid = task.uuid;
        let task_name = task.name.clone();

        repo.create(task).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 1);
        assert_eq!(repo.count_enabled().await.unwrap(), 1);
        assert!(repo.uuid_exists(task_uuid).await.unwrap());
        assert!(repo.name_exists(&task_name).await.unwrap());
    }

    #[tokio::test]
    async fn test_health_check() {
        let db = create_test_db().await;
        let repo = TaskRepository::new(db);

        assert!(repo.health_check().await.is_ok());
    }
}
