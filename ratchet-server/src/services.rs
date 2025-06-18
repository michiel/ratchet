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
// Import storage repository trait for health checks (unused for now)
// use ratchet_storage::seaorm::repositories::Repository as StorageRepositoryTrait;
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule
};
use uuid::Uuid;
use ratchet_rest_api::context::TasksContext;
use ratchet_graphql_api::context::GraphQLContext;
use ratchet_mcp::server::task_dev_tools::TaskDevelopmentService;
use ratchet_http::HttpManager;

use crate::config::ServerConfig;
use crate::bridges::{BridgeTaskRegistry, BridgeRegistryManager, BridgeTaskValidator};
use crate::scheduler::{SchedulerService, TokioCronSchedulerService, TokioCronSchedulerConfig};
use crate::heartbeat::HeartbeatService;
// use crate::scheduler_legacy::{SchedulerService as LegacySchedulerService, SchedulerConfig as LegacySchedulerConfig};
use ratchet_output::OutputDeliveryManager;

/// Service container holding all application services
#[derive(Clone)]
pub struct ServiceContainer {
    pub repositories: Arc<dyn RepositoryFactory>,
    pub registry: Arc<dyn TaskRegistry>,
    pub registry_manager: Arc<dyn RegistryManager>,
    pub validator: Arc<dyn TaskValidator>,
    pub mcp_task_service: Option<Arc<TaskDevelopmentService>>,
    pub output_manager: Arc<OutputDeliveryManager>,
    pub scheduler_service: Option<Arc<dyn SchedulerService>>,
    pub heartbeat_service: Arc<HeartbeatService>,
}

impl ServiceContainer {
    /// Create a new service container with real implementations
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        // For now, we'll use the legacy ratchet-lib implementations
        // In the future, these would be replaced with the new modular implementations
        
        // This is a bridge implementation during the migration
        let (repositories, mcp_task_service, _seaorm_factory) = create_repository_factory_with_mcp(config).await?;
        let registry = create_task_registry(config, repositories.clone()).await?;
        let registry_manager = create_registry_manager(config).await?;
        let validator = create_task_validator(config).await?;

        // Create output delivery manager
        let output_manager = Arc::new(OutputDeliveryManager::new());

        // Create scheduler service (using new tokio-cron-scheduler implementation)
        let scheduler_config = TokioCronSchedulerConfig::default();
        let scheduler_service: Option<Arc<dyn SchedulerService>> = Some(Arc::new(
            TokioCronSchedulerService::new(repositories.clone(), scheduler_config).await?
        ));

        // Create heartbeat service
        let heartbeat_service = Arc::new(HeartbeatService::new(
            config.heartbeat.clone(),
            repositories.clone(),
            output_manager.clone(),
        ));

        Ok(Self {
            repositories,
            registry,
            registry_manager,
            validator,
            mcp_task_service,
            output_manager,
            scheduler_service,
            heartbeat_service,
        })
    }

    /// Create a test service container with mock implementations
    #[cfg(test)]
    pub fn new_test() -> Self {
        
        
        // Create mock implementations for testing
        // These would be defined in the testing modules of each interface crate
        todo!("Implement mock service container for tests")
    }

    /// Create REST API context from service container
    pub fn rest_context(&self) -> TasksContext {
        if let (Some(mcp), Some(scheduler)) = (&self.mcp_task_service, &self.scheduler_service) {
            TasksContext::with_all_services(
                self.repositories.clone(),
                self.registry.clone(),
                self.registry_manager.clone(),
                self.validator.clone(),
                mcp.clone(),
                scheduler.clone(),
            )
        } else if let Some(mcp) = &self.mcp_task_service {
            TasksContext::with_mcp_service(
                self.repositories.clone(),
                self.registry.clone(),
                self.registry_manager.clone(),
                self.validator.clone(),
                mcp.clone(),
            )
        } else if let Some(scheduler) = &self.scheduler_service {
            TasksContext::with_scheduler(
                self.repositories.clone(),
                self.registry.clone(),
                self.registry_manager.clone(),
                self.validator.clone(),
                scheduler.clone(),
            )
        } else {
            TasksContext::new(
                self.repositories.clone(),
                self.registry.clone(),
                self.registry_manager.clone(),
                self.validator.clone(),
            )
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
    user_repository: ratchet_storage::seaorm::repositories::SeaOrmUserRepository,
    session_repository: ratchet_storage::seaorm::repositories::SeaOrmSessionRepository,
    api_key_repository: ratchet_storage::seaorm::repositories::SeaOrmApiKeyRepository,
}

impl DirectRepositoryFactory {
    pub fn new(storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>) -> Self {
        let task_repository = DirectTaskRepository::new(Arc::new(storage_factory.task_repository()));
        let execution_repository = DirectExecutionRepository::new(Arc::new(storage_factory.execution_repository()));
        let job_repository = DirectJobRepository::new(Arc::new(storage_factory.job_repository()));
        let schedule_repository = DirectScheduleRepository::new(Arc::new(storage_factory.schedule_repository()));
        let user_repository = storage_factory.user_repository();
        let session_repository = storage_factory.session_repository();
        let api_key_repository = storage_factory.api_key_repository();
        
        Self {
            storage_factory,
            task_repository,
            execution_repository,
            job_repository,
            schedule_repository,
            user_repository,
            session_repository,
            api_key_repository,
        }
    }
    
    /// Get access to the underlying storage factory (for MCP service creation)
    pub fn storage_factory(&self) -> &Arc<ratchet_storage::seaorm::repositories::RepositoryFactory> {
        &self.storage_factory
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
    
    fn user_repository(&self) -> &dyn ratchet_interfaces::database::UserRepository {
        &self.user_repository
    }
    
    fn session_repository(&self) -> &dyn ratchet_interfaces::database::SessionRepository {
        &self.session_repository
    }
    
    fn api_key_repository(&self) -> &dyn ratchet_interfaces::database::ApiKeyRepository {
        &self.api_key_repository
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
                            if limit > 0 { total.div_ceil(limit) as u32 } else { 1 }
                        },
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: TaskFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // For direct repositories, we can just delegate to the existing find_with_filters method
        self.find_with_filters(filters, list_input.get_pagination()).await
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
    
    async fn find_with_list_input(
        &self,
        filters: ExecutionFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        self.find_with_filters(filters, list_input.get_pagination()).await
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
    async fn create(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        match self.storage_repo.create(storage_job).await {
            Ok(created_job) => Ok(convert_storage_job_to_unified(created_job)),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedJob>, DatabaseError> {
        // TODO: Implement job lookup
        Ok(None)
    }
    
    async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedJob>, DatabaseError> {
        // TODO: Implement job lookup by UUID
        Ok(None)
    }
    
    async fn update(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        match self.storage_repo.update(storage_job).await {
            Ok(updated_job) => Ok(convert_storage_job_to_unified(updated_job)),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
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
    
    async fn find_with_list_input(
        &self,
        filters: JobFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        self.find_with_filters(filters, list_input.get_pagination()).await
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
    async fn create(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.storage_repo.create(storage_schedule).await {
            Ok(created_schedule) => Ok(convert_storage_schedule_to_unified(created_schedule)),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(schedule)) => Ok(Some(convert_storage_schedule_to_unified(schedule))),
            Ok(None) => Ok(None),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        // For now, we don't have a direct UUID lookup in storage, so we'll need to search
        // This is a temporary implementation - ideally storage would support UUID lookup
        match self.storage_repo.find_enabled().await {
            Ok(schedules) => {
                for schedule in schedules {
                    if schedule.uuid == uuid {
                        return Ok(Some(convert_storage_schedule_to_unified(schedule)));
                    }
                }
                Ok(None)
            },
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn update(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.storage_repo.update(storage_schedule).await {
            Ok(updated_schedule) => Ok(convert_storage_schedule_to_unified(updated_schedule)),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        self.storage_repo.delete(id).await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
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
        filters: ScheduleFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        // For now, get enabled schedules (which should include our heartbeat schedule)
        match self.storage_repo.find_enabled().await {
            Ok(schedules) => {
                let mut filtered_schedules = schedules;
                
                // Apply name filtering if provided
                if let Some(ref name_exact) = filters.name_exact {
                    filtered_schedules.retain(|s| s.name == *name_exact);
                }
                
                // Convert to unified schedules
                let unified_schedules: Vec<UnifiedSchedule> = filtered_schedules
                    .into_iter()
                    .map(convert_storage_schedule_to_unified)
                    .collect();
                
                Ok(ListResponse {
                    items: unified_schedules,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0, // Would need separate count query
                        has_next: false,
                        has_previous: false,
                        total_pages: 0,
                    },
                })
            },
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: ScheduleFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        self.find_with_filters(filters, list_input.get_pagination()).await
    }
    
    async fn count_with_filters(&self, _filters: ScheduleFilters) -> Result<u64, DatabaseError> {
        // TODO: Implement schedule counting with filters
        Ok(0)
    }
}

#[async_trait]
impl ScheduleRepository for DirectScheduleRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_enabled().await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_ready_to_run().await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(DatabaseError::Internal { message: e.to_string() })
        }
    }
    
    async fn record_execution(&self, id: ApiId, _execution_id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.record_execution(i32_id).await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
    
    async fn update_next_run(&self, id: ApiId, next_run: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.update_next_run(i32_id, Some(next_run)).await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        self.storage_repo.set_enabled(i32_id, enabled).await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
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

fn convert_unified_schedule_to_storage(schedule: UnifiedSchedule) -> ratchet_storage::seaorm::entities::Schedule {
    ratchet_storage::seaorm::entities::Schedule {
        id: schedule.id.as_i32().unwrap_or(0),
        uuid: schedule.id.as_uuid().unwrap_or_else(uuid::Uuid::new_v4), // Use schedule id as UUID or generate new one
        task_id: schedule.task_id.as_i32().unwrap_or(0),
        name: schedule.name,
        cron_expression: schedule.cron_expression,
        input_data: serde_json::Value::Null, // Default empty input
        enabled: schedule.enabled,
        next_run_at: schedule.next_run,
        last_run_at: schedule.last_run,
        execution_count: 0, // Default to 0
        max_executions: None, // No limit by default
        metadata: Some(serde_json::json!({
            "description": schedule.description
        })),
        output_destinations: Some(serde_json::Value::Null), // Default empty output destinations
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    }
}

fn convert_storage_schedule_to_unified(schedule: ratchet_storage::seaorm::entities::Schedule) -> UnifiedSchedule {
    UnifiedSchedule {
        id: ApiId::from_i32(schedule.id),
        task_id: ApiId::from_i32(schedule.task_id),
        name: schedule.name,
        description: schedule.metadata
            .as_ref()
            .and_then(|m| m.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        cron_expression: schedule.cron_expression,
        enabled: schedule.enabled,
        next_run: schedule.next_run_at,
        last_run: schedule.last_run_at,
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
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

fn convert_unified_job_to_storage(job: UnifiedJob) -> ratchet_storage::seaorm::entities::Job {
    ratchet_storage::seaorm::entities::Job {
        id: job.id.as_i32().unwrap_or(0),
        uuid: job.id.as_uuid().unwrap_or_else(uuid::Uuid::new_v4),
        task_id: job.task_id.as_i32().unwrap_or(0),
        execution_id: None, // Not set until execution starts
        schedule_id: None, // Would need to be provided if job is from a schedule
        priority: convert_api_job_priority_to_storage(job.priority),
        status: convert_api_job_status_to_storage(job.status),
        input_data: serde_json::Value::Null, // Default empty input
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        retry_delay_seconds: 60, // Default 60 seconds
        error_message: job.error_message,
        error_details: None,
        queued_at: job.queued_at,
        process_at: job.scheduled_for,
        started_at: None,
        completed_at: None,
        metadata: None,
        output_destinations: job.output_destinations.map(|destinations| {
            serde_json::to_value(destinations).unwrap_or(serde_json::Value::Null)
        }),
    }
}

fn convert_storage_job_to_unified(job: ratchet_storage::seaorm::entities::Job) -> UnifiedJob {
    UnifiedJob {
        id: ApiId::from_i32(job.id),
        task_id: ApiId::from_i32(job.task_id),
        priority: convert_storage_job_priority_to_api(job.priority),
        status: convert_storage_job_status_to_api(job.status),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        queued_at: job.queued_at,
        scheduled_for: job.process_at,
        error_message: job.error_message,
        output_destinations: job.output_destinations.and_then(|v| {
            serde_json::from_value(v).ok()
        }),
    }
}

fn convert_api_job_priority_to_storage(priority: ratchet_api_types::JobPriority) -> ratchet_storage::seaorm::entities::jobs::JobPriority {
    match priority {
        ratchet_api_types::JobPriority::Low => ratchet_storage::seaorm::entities::jobs::JobPriority::Low,
        ratchet_api_types::JobPriority::Normal => ratchet_storage::seaorm::entities::jobs::JobPriority::Normal,
        ratchet_api_types::JobPriority::High => ratchet_storage::seaorm::entities::jobs::JobPriority::High,
        ratchet_api_types::JobPriority::Critical => ratchet_storage::seaorm::entities::jobs::JobPriority::Urgent,
    }
}

fn convert_storage_job_priority_to_api(priority: ratchet_storage::seaorm::entities::jobs::JobPriority) -> ratchet_api_types::JobPriority {
    match priority {
        ratchet_storage::seaorm::entities::jobs::JobPriority::Low => ratchet_api_types::JobPriority::Low,
        ratchet_storage::seaorm::entities::jobs::JobPriority::Normal => ratchet_api_types::JobPriority::Normal,
        ratchet_storage::seaorm::entities::jobs::JobPriority::High => ratchet_api_types::JobPriority::High,
        ratchet_storage::seaorm::entities::jobs::JobPriority::Urgent => ratchet_api_types::JobPriority::Critical,
    }
}

fn convert_api_job_status_to_storage(status: ratchet_api_types::JobStatus) -> ratchet_storage::seaorm::entities::jobs::JobStatus {
    match status {
        ratchet_api_types::JobStatus::Queued => ratchet_storage::seaorm::entities::jobs::JobStatus::Queued,
        ratchet_api_types::JobStatus::Processing => ratchet_storage::seaorm::entities::jobs::JobStatus::Processing,
        ratchet_api_types::JobStatus::Completed => ratchet_storage::seaorm::entities::jobs::JobStatus::Completed,
        ratchet_api_types::JobStatus::Failed => ratchet_storage::seaorm::entities::jobs::JobStatus::Failed,
        ratchet_api_types::JobStatus::Cancelled => ratchet_storage::seaorm::entities::jobs::JobStatus::Cancelled,
        ratchet_api_types::JobStatus::Retrying => ratchet_storage::seaorm::entities::jobs::JobStatus::Retrying,
    }
}

fn convert_storage_job_status_to_api(status: ratchet_storage::seaorm::entities::jobs::JobStatus) -> ratchet_api_types::JobStatus {
    match status {
        ratchet_storage::seaorm::entities::jobs::JobStatus::Queued => ratchet_api_types::JobStatus::Queued,
        ratchet_storage::seaorm::entities::jobs::JobStatus::Processing => ratchet_api_types::JobStatus::Processing,
        ratchet_storage::seaorm::entities::jobs::JobStatus::Completed => ratchet_api_types::JobStatus::Completed,
        ratchet_storage::seaorm::entities::jobs::JobStatus::Failed => ratchet_api_types::JobStatus::Failed,
        ratchet_storage::seaorm::entities::jobs::JobStatus::Cancelled => ratchet_api_types::JobStatus::Cancelled,
        ratchet_storage::seaorm::entities::jobs::JobStatus::Retrying => ratchet_api_types::JobStatus::Retrying,
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
    let (repos, _, _) = create_repository_factory_with_mcp(config).await?;
    Ok(repos)
}

async fn create_repository_factory_with_mcp(config: &ServerConfig) -> Result<(Arc<dyn RepositoryFactory>, Option<Arc<TaskDevelopmentService>>, Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>)> {
    // Create storage database connection directly (no bridge pattern)
    let storage_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        connection_timeout: std::time::Duration::from_secs(config.database.connection_timeout_seconds),
    };
    
    let db_connection = ratchet_storage::seaorm::connection::DatabaseConnection::new(storage_config).await?;
    let storage_factory = Arc::new(ratchet_storage::seaorm::repositories::RepositoryFactory::new(db_connection));
    
    // Create the DirectRepositoryFactory
    let direct_factory = DirectRepositoryFactory::new(storage_factory.clone());
    
    // Create MCP task development service if MCP is enabled
    let mcp_task_service = if config.mcp_api.enabled {
        // Create HTTP manager for task development service
        let http_manager = HttpManager::new();
        
        // Get the database connection from storage factory
        let storage_db = storage_factory.database();
        
        // Create concrete repository instances for TaskDevelopmentService
        let task_repo_arc = Arc::new(ratchet_storage::seaorm::repositories::task_repository::TaskRepository::new(
            storage_db.clone()
        ));
        let execution_repo_arc = Arc::new(ratchet_storage::seaorm::repositories::execution_repository::ExecutionRepository::new(
            storage_db.clone()
        ));
        
        // Use a default task base path - this could be configurable
        let task_base_path = std::path::PathBuf::from("./tasks");
        
        let service = TaskDevelopmentService::new(
            task_repo_arc,
            execution_repo_arc,
            http_manager,
            task_base_path,
            true, // allow_fs_operations
        );
        
        Some(Arc::new(service))
    } else {
        None
    };
    
    // Use the adapter factory that directly implements the interface
    Ok((Arc::new(direct_factory), mcp_task_service, storage_factory))
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

impl Default for StubRepositoryFactory {
    fn default() -> Self {
        Self::new()
    }
}

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
    
    fn user_repository(&self) -> &dyn ratchet_interfaces::database::UserRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn session_repository(&self) -> &dyn ratchet_interfaces::database::SessionRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn api_key_repository(&self) -> &dyn ratchet_interfaces::database::ApiKeyRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        Ok(())
    }
}

/// Stub task registry
pub struct StubTaskRegistry;

impl Default for StubTaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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

impl Default for StubRegistryManager {
    fn default() -> Self {
        Self::new()
    }
}

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

impl Default for StubTaskValidator {
    fn default() -> Self {
        Self::new()
    }
}

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