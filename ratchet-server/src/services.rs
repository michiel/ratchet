//! Service implementations and dependency injection setup

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;

use ratchet_interfaces::{
    RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator,
    TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository,
    Repository, CrudRepository, FilteredRepository,
    TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
    DatabaseError, TaskMetadata, RegistryError, ValidationResult, SyncResult
};
// Import storage repository trait for health checks
use ratchet_storage::seaorm::repositories::Repository as StorageRepositoryTrait;
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule
};
use uuid::Uuid;
use ratchet_rest_api::context::TasksContext;
use ratchet_graphql_api::context::GraphQLContext;

use crate::config::ServerConfig;
use crate::bridges::{BridgeTaskRegistry, BridgeRegistryManager, BridgeTaskValidator};

/// Service container holding all application services
#[derive(Clone)]
pub struct ServiceContainer {
    pub repositories: Arc<dyn RepositoryFactory>,
    pub registry: Arc<dyn TaskRegistry>,
    pub registry_manager: Arc<dyn RegistryManager>,
    pub validator: Arc<dyn TaskValidator>,
}

impl ServiceContainer {
    /// Create a new service container with real implementations
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        // For now, we'll use the legacy ratchet-lib implementations
        // In the future, these would be replaced with the new modular implementations
        
        // This is a bridge implementation during the migration
        let repositories = create_repository_factory(config).await?;
        let registry = create_task_registry(config, repositories.clone()).await?;
        let registry_manager = create_registry_manager(config).await?;
        let validator = create_task_validator(config).await?;

        Ok(Self {
            repositories,
            registry,
            registry_manager,
            validator,
        })
    }

    /// Create a test service container with mock implementations
    #[cfg(test)]
    pub fn new_test() -> Self {
        use std::sync::Arc;
        
        // Create mock implementations for testing
        // These would be defined in the testing modules of each interface crate
        todo!("Implement mock service container for tests")
    }

    /// Create REST API context from service container
    pub fn rest_context(&self) -> TasksContext {
        TasksContext {
            repositories: self.repositories.clone(),
            registry: self.registry.clone(),
            registry_manager: self.registry_manager.clone(),
            validator: self.validator.clone(),
            mcp_task_service: None,
        }
    }

    /// Create GraphQL context from service container
    pub fn graphql_context(&self) -> GraphQLContext {
        GraphQLContext::new(
            self.repositories.clone(),
            self.registry.clone(),
            self.registry_manager.clone(),
            self.validator.clone(),
        )
    }
}

/// Direct repository factory that bypasses bridge pattern
/// Uses ratchet-storage directly with interface adapters
pub struct DirectRepositoryFactory {
    storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>,
    task_repository: DirectTaskRepository,
    execution_repository: DirectExecutionRepository,
    job_repository: DirectJobRepository,
    schedule_repository: DirectScheduleRepository,
}

impl DirectRepositoryFactory {
    pub fn new(storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>) -> Self {
        let task_repository = DirectTaskRepository::new(Arc::new(storage_factory.task_repository()));
        let execution_repository = DirectExecutionRepository::new(Arc::new(storage_factory.execution_repository()));
        let job_repository = DirectJobRepository::new(Arc::new(storage_factory.job_repository()));
        let schedule_repository = DirectScheduleRepository::new(Arc::new(storage_factory.schedule_repository()));
        
        Self {
            storage_factory,
            task_repository,
            execution_repository,
            job_repository,
            schedule_repository,
        }
    }
}

#[async_trait]
impl RepositoryFactory for DirectRepositoryFactory {
    fn task_repository(&self) -> &dyn TaskRepository {
        &self.task_repository
    }
    
    fn execution_repository(&self) -> &dyn ExecutionRepository {
        &self.execution_repository
    }
    
    fn job_repository(&self) -> &dyn JobRepository {
        &self.job_repository
    }
    
    fn schedule_repository(&self) -> &dyn ScheduleRepository {
        &self.schedule_repository
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Delegate to storage health check
        self.storage_factory.task_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

/// Direct task repository adapter
pub struct DirectTaskRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::TaskRepository>,
}

impl DirectTaskRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::TaskRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for DirectTaskRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.storage_repo.health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl CrudRepository<UnifiedTask> for DirectTaskRepository {
    async fn create(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        // Convert unified task to storage task
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.storage_repo.create(storage_task).await {
            Ok(created_task) => Ok(convert_storage_task_to_unified(created_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_uuid(uuid).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.storage_repo.update(storage_task).await {
            Ok(updated_task) => Ok(convert_storage_task_to_unified(updated_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        self.storage_repo.delete(id).await
            .map_err(convert_storage_error)
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        self.storage_repo.count().await
            .map_err(convert_storage_error)
    }
}

#[async_trait]
impl FilteredRepository<UnifiedTask, TaskFilters> for DirectTaskRepository {
    async fn find_with_filters(
        &self, 
        filters: TaskFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // Convert interface filters to storage filters (clone to avoid move)
        let storage_filters = convert_interface_filters_to_storage(filters.clone());
        let storage_pagination = convert_interface_pagination_to_storage(pagination.clone());
        
        match self.storage_repo.find_with_filters(storage_filters, storage_pagination).await {
            Ok(tasks) => {
                let unified_tasks: Vec<UnifiedTask> = tasks.into_iter()
                    .map(convert_storage_task_to_unified)
                    .collect();
                    
                // Store items count before getting total count
                let items_count = unified_tasks.len() as u64;
                    
                // Get proper total count
                let total = self.count_with_filters(filters).await?;
                    
                Ok(ListResponse {
                    items: unified_tasks,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total,
                        has_next: {
                            let current_offset = pagination.offset.unwrap_or(0) as u64;
                            current_offset + items_count < total
                        },
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: {
                            let limit = pagination.limit.unwrap_or(20) as u64;
                            if limit > 0 { ((total + limit - 1) / limit) as u32 } else { 1 }
                        },
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count_with_filters(&self, filters: TaskFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_interface_filters_to_storage(filters);
        self.storage_repo.count_with_filters(storage_filters).await
            .map_err(convert_storage_error)
    }
}

#[async_trait]
impl TaskRepository for DirectTaskRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_enabled().await {
            Ok(tasks) => Ok(tasks.into_iter().map(convert_storage_task_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_name(name).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_validated(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.mark_validated(i32_id).await
            .map_err(convert_storage_error)
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.set_enabled(i32_id, enabled).await
            .map_err(convert_storage_error)
    }
    
    async fn set_in_sync(&self, id: ApiId, in_sync: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.set_in_sync(i32_id, in_sync).await
            .map_err(convert_storage_error)
    }
}

// Placeholder implementations for other repositories (will need to be completed)
pub struct DirectExecutionRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::ExecutionRepository>,
}

impl DirectExecutionRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::ExecutionRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for DirectExecutionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Use count as a simple health check since direct health_check is ?Send
        self.storage_repo.count().await
            .map(|_| ())
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl CrudRepository<UnifiedExecution> for DirectExecutionRepository {
    async fn create(&self, _entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        // TODO: Implement execution creation
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedExecution>, DatabaseError> {
        // TODO: Implement execution lookup
        Ok(None)
    }
    
    async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedExecution>, DatabaseError> {
        // TODO: Implement execution lookup by UUID
        Ok(None)
    }
    
    async fn update(&self, _entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        // TODO: Implement execution update
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn delete(&self, _id: i32) -> Result<(), DatabaseError> {
        // TODO: Implement execution deletion
        Ok(())
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        self.storage_repo.count().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl FilteredRepository<UnifiedExecution, ExecutionFilters> for DirectExecutionRepository {
    async fn find_with_filters(
        &self, 
        _filters: ExecutionFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        // TODO: Implement execution filtering
        Ok(ListResponse {
            items: Vec::new(),
            meta: ratchet_api_types::pagination::PaginationMeta {
                page: pagination.page.unwrap_or(1),
                limit: pagination.limit.unwrap_or(20),
                offset: pagination.offset.unwrap_or(0),
                total: 0,
                has_next: false,
                has_previous: false,
                total_pages: 0,
            },
        })
    }
    
    async fn count_with_filters(&self, _filters: ExecutionFilters) -> Result<u64, DatabaseError> {
        // TODO: Implement execution counting with filters
        Ok(0)
    }
}

#[async_trait]
impl ExecutionRepository for DirectExecutionRepository {
    async fn find_by_task_id(&self, _task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        // TODO: Implement find by task ID
        Ok(Vec::new())
    }
    
    async fn find_by_status(&self, _status: ratchet_api_types::ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        // TODO: Implement find by status
        Ok(Vec::new())
    }
    
    async fn update_status(&self, _id: ApiId, _status: ratchet_api_types::ExecutionStatus) -> Result<(), DatabaseError> {
        // TODO: Implement status update
        Ok(())
    }
    
    async fn mark_started(&self, _id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement mark started
        Ok(())
    }
    
    async fn mark_completed(&self, _id: ApiId, _output: serde_json::Value, _duration_ms: Option<i32>) -> Result<(), DatabaseError> {
        // TODO: Implement mark completed
        Ok(())
    }
    
    async fn mark_failed(&self, _id: ApiId, _error_message: String, _error_details: Option<serde_json::Value>) -> Result<(), DatabaseError> {
        // TODO: Implement mark failed
        Ok(())
    }
    
    async fn mark_cancelled(&self, _id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement mark cancelled
        Ok(())
    }
    
    async fn update_progress(&self, _id: ApiId, _progress: f32) -> Result<(), DatabaseError> {
        // TODO: Implement progress update
        Ok(())
    }
}

pub struct DirectJobRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::JobRepository>,
}

impl DirectJobRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::JobRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for DirectJobRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Use count as a simple health check since direct health_check is ?Send
        self.storage_repo.count().await
            .map(|_| ())
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl CrudRepository<UnifiedJob> for DirectJobRepository {
    async fn create(&self, _entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        // TODO: Implement job creation
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedJob>, DatabaseError> {
        // TODO: Implement job lookup
        Ok(None)
    }
    
    async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedJob>, DatabaseError> {
        // TODO: Implement job lookup by UUID
        Ok(None)
    }
    
    async fn update(&self, _entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        // TODO: Implement job update
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn delete(&self, _id: i32) -> Result<(), DatabaseError> {
        // TODO: Implement job deletion
        Ok(())
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        self.storage_repo.count().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl FilteredRepository<UnifiedJob, JobFilters> for DirectJobRepository {
    async fn find_with_filters(
        &self, 
        _filters: JobFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        // TODO: Implement job filtering
        Ok(ListResponse {
            items: Vec::new(),
            meta: ratchet_api_types::pagination::PaginationMeta {
                page: pagination.page.unwrap_or(1),
                limit: pagination.limit.unwrap_or(20),
                offset: pagination.offset.unwrap_or(0),
                total: 0,
                has_next: false,
                has_previous: false,
                total_pages: 0,
            },
        })
    }
    
    async fn count_with_filters(&self, _filters: JobFilters) -> Result<u64, DatabaseError> {
        // TODO: Implement job counting with filters
        Ok(0)
    }
}

#[async_trait]
impl JobRepository for DirectJobRepository {
    async fn find_ready_for_processing(&self, _limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError> {
        // TODO: Implement ready for processing
        Ok(Vec::new())
    }
    
    async fn find_by_status(&self, _status: ratchet_api_types::JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError> {
        // TODO: Implement find by status
        Ok(Vec::new())
    }
    
    async fn mark_processing(&self, _id: ApiId, _execution_id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement mark processing
        Ok(())
    }
    
    async fn mark_completed(&self, _id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement mark completed
        Ok(())
    }
    
    async fn mark_failed(&self, _id: ApiId, _error: String, _details: Option<serde_json::Value>) -> Result<bool, DatabaseError> {
        // TODO: Implement mark failed
        Ok(false)
    }
    
    async fn schedule_retry(&self, _id: ApiId, _retry_at: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError> {
        // TODO: Implement schedule retry
        Ok(())
    }
    
    async fn cancel(&self, _id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement cancel
        Ok(())
    }
}

pub struct DirectScheduleRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::ScheduleRepository>,
}

impl DirectScheduleRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::ScheduleRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for DirectScheduleRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Use count as a simple health check since direct health_check is ?Send
        self.storage_repo.count().await
            .map(|_| ())
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl CrudRepository<UnifiedSchedule> for DirectScheduleRepository {
    async fn create(&self, _entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        // TODO: Implement schedule creation
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        // TODO: Implement schedule lookup
        Ok(None)
    }
    
    async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        // TODO: Implement schedule lookup by UUID
        Ok(None)
    }
    
    async fn update(&self, _entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        // TODO: Implement schedule update
        Err(DatabaseError::Internal { message: "Not implemented yet".to_string() })
    }
    
    async fn delete(&self, _id: i32) -> Result<(), DatabaseError> {
        // TODO: Implement schedule deletion
        Ok(())
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        self.storage_repo.count().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl FilteredRepository<UnifiedSchedule, ScheduleFilters> for DirectScheduleRepository {
    async fn find_with_filters(
        &self, 
        _filters: ScheduleFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        // TODO: Implement schedule filtering
        Ok(ListResponse {
            items: Vec::new(),
            meta: ratchet_api_types::pagination::PaginationMeta {
                page: pagination.page.unwrap_or(1),
                limit: pagination.limit.unwrap_or(20),
                offset: pagination.offset.unwrap_or(0),
                total: 0,
                has_next: false,
                has_previous: false,
                total_pages: 0,
            },
        })
    }
    
    async fn count_with_filters(&self, _filters: ScheduleFilters) -> Result<u64, DatabaseError> {
        // TODO: Implement schedule counting with filters
        Ok(0)
    }
}

#[async_trait]
impl ScheduleRepository for DirectScheduleRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        // TODO: Implement find enabled
        Ok(Vec::new())
    }
    
    async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        // TODO: Implement find ready to run
        Ok(Vec::new())
    }
    
    async fn record_execution(&self, _id: ApiId, _execution_id: ApiId) -> Result<(), DatabaseError> {
        // TODO: Implement record execution
        Ok(())
    }
    
    async fn update_next_run(&self, _id: ApiId, _next_run: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError> {
        // TODO: Implement update next run
        Ok(())
    }
    
    async fn set_enabled(&self, _id: ApiId, _enabled: bool) -> Result<(), DatabaseError> {
        // TODO: Implement set enabled
        Ok(())
    }
}

// Conversion functions (simplified - reuse from bridges for now)
fn convert_unified_task_to_storage(task: UnifiedTask) -> ratchet_storage::seaorm::entities::Task {
    ratchet_storage::seaorm::entities::Task {
        id: task.id.as_i32().unwrap_or(0),
        uuid: task.uuid,
        name: task.name,
        description: task.description,
        version: task.version,
        path: String::new(), // Would need to be provided or inferred
        metadata: task.metadata.unwrap_or_default(),
        input_schema: task.input_schema.unwrap_or_default(),
        output_schema: task.output_schema.unwrap_or_default(),
        enabled: task.enabled,
        created_at: task.created_at,
        updated_at: task.updated_at,
        validated_at: task.validated_at,
    }
}

fn convert_storage_task_to_unified(task: ratchet_storage::seaorm::entities::Task) -> UnifiedTask {
    UnifiedTask {
        id: ApiId::from_i32(task.id),
        uuid: task.uuid,
        name: task.name,
        description: task.description,
        version: task.version.clone(),
        enabled: task.enabled,
        registry_source: false, // Default value, could be inferred from metadata
        available_versions: vec![task.version], // Default, could expand based on registry
        created_at: task.created_at,
        updated_at: task.updated_at,
        validated_at: task.validated_at,
        in_sync: task.metadata.get("in_sync").and_then(|v| v.as_bool()).unwrap_or(true),
        input_schema: Some(task.input_schema),
        output_schema: Some(task.output_schema),
        metadata: Some(task.metadata),
    }
}

fn convert_storage_error(err: ratchet_storage::seaorm::connection::DatabaseError) -> DatabaseError {
    use ratchet_storage::seaorm::connection::DatabaseError as StorageError;
    match err {
        StorageError::DbError(db_err) => {
            // Convert SeaORM database errors to appropriate interface errors
            match db_err {
                sea_orm::DbErr::RecordNotFound(_) => DatabaseError::NotFound { 
                    entity: "unknown".to_string(), 
                    id: "unknown".to_string() 
                },
                sea_orm::DbErr::ConnectionAcquire(_) => DatabaseError::Connection { 
                    message: db_err.to_string() 
                },
                sea_orm::DbErr::Exec(_) | sea_orm::DbErr::Query(_) => DatabaseError::Internal { 
                    message: db_err.to_string() 
                },
                _ => DatabaseError::Internal { message: db_err.to_string() }
            }
        },
        StorageError::MigrationError(msg) => DatabaseError::Internal { message: msg },
        StorageError::SerializationError(e) => DatabaseError::Internal { message: e.to_string() },
        StorageError::ConfigError(msg) => DatabaseError::Internal { message: msg },
        StorageError::ValidationError(e) => DatabaseError::Validation { message: e.to_string() },
    }
}

fn convert_interface_filters_to_storage(filters: TaskFilters) -> ratchet_storage::seaorm::repositories::task_repository::TaskFilters {
    ratchet_storage::seaorm::repositories::task_repository::TaskFilters {
        name: filters.name,
        enabled: filters.enabled,
        has_validation: filters.validated_after.map(|_| true), // Convert validated_after to has_validation
        version: None, // Not supported in current interface
    }
}

fn convert_interface_pagination_to_storage(pagination: PaginationInput) -> ratchet_storage::seaorm::repositories::task_repository::Pagination {
    ratchet_storage::seaorm::repositories::task_repository::Pagination {
        limit: Some(pagination.get_limit() as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by: None,
        order_desc: None,
    }
}

/// Create repository factory from configuration
async fn create_repository_factory(config: &ServerConfig) -> Result<Arc<dyn RepositoryFactory>> {
    // Create storage database connection directly (no bridge pattern)
    let storage_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        connection_timeout: std::time::Duration::from_secs(config.database.connection_timeout_seconds),
    };
    
    let db_connection = ratchet_storage::seaorm::connection::DatabaseConnection::new(storage_config).await?;
    let storage_factory = ratchet_storage::seaorm::repositories::RepositoryFactory::new(db_connection);
    
    // Use the adapter factory that directly implements the interface
    Ok(Arc::new(DirectRepositoryFactory::new(Arc::new(storage_factory))))
}

/// Create task registry from configuration
async fn create_task_registry(config: &ServerConfig, repositories: Arc<dyn RepositoryFactory>) -> Result<Arc<dyn TaskRegistry>> {
    // Create functional task registry using ratchet-registry
    let mut bridge_registry = BridgeTaskRegistry::new(config).await?;
    bridge_registry.set_repositories(repositories);
    
    // Sync discovered tasks to database
    bridge_registry.sync_tasks_to_database().await?;
    
    Ok(Arc::new(bridge_registry))
}

/// Create registry manager from configuration
async fn create_registry_manager(config: &ServerConfig) -> Result<Arc<dyn RegistryManager>> {
    // Create functional registry manager using ratchet-registry
    let bridge_manager = BridgeRegistryManager::new(config).await?;
    Ok(Arc::new(bridge_manager))
}

/// Create task validator from configuration
async fn create_task_validator(_config: &ServerConfig) -> Result<Arc<dyn TaskValidator>> {
    // Create functional task validator using ratchet-registry
    Ok(Arc::new(BridgeTaskValidator::new()))
}

/// Initialize logging system
pub async fn init_logging(config: &ServerConfig) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let subscriber = tracing_subscriber::registry();

    // Add console layer
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
    );

    // Add file layer if enabled
    if config.logging.enable_file_logging {
        if let Some(file_path) = &config.logging.file_path {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;
            
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(file)
                .with_ansi(false);
                
            let subscriber = subscriber.with(file_layer);
            // Use try_init to avoid panic if global subscriber already set
            if let Err(_) = subscriber.try_init() {
                tracing::debug!("Global tracing subscriber already initialized, skipping");
            }
        } else {
            // Use try_init to avoid panic if global subscriber already set
            if let Err(_) = subscriber.try_init() {
                tracing::debug!("Global tracing subscriber already initialized, skipping");
            }
        }
    } else {
        // Use try_init to avoid panic if global subscriber already set
        if let Err(_) = subscriber.try_init() {
            tracing::debug!("Global tracing subscriber already initialized, skipping");
        }
    }

    tracing::info!("Logging initialized");
    Ok(())
}

// =============================================================================
// Stub Implementations (Temporary for migration phase)
// =============================================================================

/// Stub repository factory that returns empty results
pub struct StubRepositoryFactory;

impl StubRepositoryFactory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RepositoryFactory for StubRepositoryFactory {
    fn task_repository(&self) -> &dyn TaskRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn execution_repository(&self) -> &dyn ExecutionRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn job_repository(&self) -> &dyn JobRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn schedule_repository(&self) -> &dyn ScheduleRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        Ok(())
    }
}

/// Stub task registry
pub struct StubTaskRegistry;

impl StubTaskRegistry {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskRegistry for StubTaskRegistry {
    async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>, RegistryError> {
        Ok(vec![])
    }
    
    async fn get_task_metadata(&self, _name: &str) -> Result<TaskMetadata, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn load_task_content(&self, _name: &str) -> Result<String, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn task_exists(&self, _name: &str) -> Result<bool, RegistryError> {
        Ok(false)
    }
    
    fn registry_id(&self) -> &str {
        "stub-registry"
    }
    
    async fn health_check(&self) -> Result<(), RegistryError> {
        Ok(())
    }
}

/// Stub registry manager
pub struct StubRegistryManager;

impl StubRegistryManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RegistryManager for StubRegistryManager {
    async fn add_registry(&self, _registry: Box<dyn TaskRegistry>) -> Result<(), RegistryError> {
        Ok(())
    }
    
    async fn remove_registry(&self, _registry_id: &str) -> Result<(), RegistryError> {
        Ok(())
    }
    
    async fn list_registries(&self) -> Vec<&str> {
        vec![]
    }
    
    async fn discover_all_tasks(&self) -> Result<Vec<(String, TaskMetadata)>, RegistryError> {
        Ok(vec![])
    }
    
    async fn find_task(&self, _name: &str) -> Result<(String, TaskMetadata), RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn load_task(&self, _name: &str) -> Result<String, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn sync_with_database(&self) -> Result<SyncResult, RegistryError> {
        Ok(SyncResult {
            added: vec![],
            updated: vec![],
            removed: vec![],
            errors: vec![],
        })
    }
}

/// Stub task validator
pub struct StubTaskValidator;

impl StubTaskValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskValidator for StubTaskValidator {
    async fn validate_metadata(&self, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_content(&self, _content: &str, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_input(&self, _input: &serde_json::Value, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
}