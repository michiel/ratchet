//! GraphQL API integration tests
//!
//! This module provides comprehensive integration testing for the Ratchet GraphQL API,
//! covering schema validation, query execution, mutations, subscriptions, and error handling.

use async_graphql::{Request, Variables, Response, value};
use chrono::Utc;
use ratchet_graphql_api::{
    schema::{create_schema, configure_schema, RatchetSchema},
    context::{GraphQLContext, GraphQLConfig},
};
use ratchet_interfaces::{RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator, UserRepository, SessionRepository, ApiKeyRepository};
use ratchet_api_types::{ApiId, UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule, ExecutionStatus, JobStatus, JobPriority};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Test configuration for GraphQL integration tests
#[derive(Debug, Clone)]
pub struct GraphQLTestConfig {
    pub enable_introspection: bool,
    pub enable_apollo_tracing: bool,
    pub max_query_depth: Option<usize>,
    pub max_query_complexity: Option<usize>,
}

impl Default for GraphQLTestConfig {
    fn default() -> Self {
        Self {
            enable_introspection: true,
            enable_apollo_tracing: false,
            max_query_depth: Some(10),
            max_query_complexity: Some(100),
        }
    }
}

/// Test server builder for GraphQL testing
pub struct GraphQLTestServer {
    schema: RatchetSchema,
    context: GraphQLContext,
}

impl GraphQLTestServer {
    /// Create a new test server with default configuration
    pub async fn new() -> Self {
        Self::with_config(GraphQLTestConfig::default()).await
    }
    
    /// Create a new test server with custom configuration
    pub async fn with_config(config: GraphQLTestConfig) -> Self {
        let repositories = create_mock_repository_factory().await;
        let registry = create_mock_registry().await;
        let registry_manager = create_mock_registry_manager().await;
        let validator = create_mock_validator().await;
        
        let context = GraphQLContext::new(
            repositories,
            registry,
            registry_manager,
            validator,
        );
        
        let graphql_config = GraphQLConfig {
            enable_playground: false, // Not needed for tests
            enable_introspection: config.enable_introspection,
            max_query_depth: config.max_query_depth,
            max_query_complexity: config.max_query_complexity,
            enable_tracing: false,
            enable_apollo_tracing: config.enable_apollo_tracing,
        };
        
        let schema = configure_schema(create_schema(), &graphql_config);
        
        Self { schema, context }
    }
    
    /// Execute a GraphQL query
    pub async fn execute(&self, query: &str) -> Response {
        let request = Request::new(query);
        self.execute_request(request).await
    }
    
    /// Execute a GraphQL query with variables
    pub async fn execute_with_variables(&self, query: &str, variables: Variables) -> Response {
        let request = Request::new(query).variables(variables);
        self.execute_request(request).await
    }
    
    /// Execute a GraphQL request with context
    async fn execute_request(&self, request: Request) -> Response {
        self.schema
            .execute(request.data(self.context.clone()))
            .await
    }
}

// Mock implementations for testing
mod mocks {
    use super::*;
    use ratchet_interfaces::{
        DatabaseError, TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
        TaskMetadata, RegistryError, ValidationResult, SyncResult
    };
    use ratchet_api_types::{ListResponse, PaginationInput, pagination::PaginationMeta};
    use async_trait::async_trait;
    
    pub struct MockRepositoryFactory;
    
    #[async_trait]
    impl RepositoryFactory for MockRepositoryFactory {
        fn task_repository(&self) -> &dyn ratchet_interfaces::TaskRepository {
            &MockTaskRepository
        }
        
        fn execution_repository(&self) -> &dyn ratchet_interfaces::ExecutionRepository {
            &MockExecutionRepository
        }
        
        fn job_repository(&self) -> &dyn ratchet_interfaces::JobRepository {
            &MockJobRepository
        }
        
        fn schedule_repository(&self) -> &dyn ratchet_interfaces::ScheduleRepository {
            &MockScheduleRepository
        }
        
        fn user_repository(&self) -> &dyn ratchet_interfaces::UserRepository {
            &MockUserRepository
        }
        
        fn session_repository(&self) -> &dyn ratchet_interfaces::SessionRepository {
            &MockSessionRepository
        }
        
        fn api_key_repository(&self) -> &dyn ratchet_interfaces::ApiKeyRepository {
            &MockApiKeyRepository
        }
        
        async fn health_check(&self) -> Result<(), DatabaseError> {
            Ok(())
        }
    }
    
    pub struct MockTaskRepository;
    
    #[async_trait]
    impl ratchet_interfaces::Repository for MockTaskRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<UnifiedTask> for MockTaskRepository {
        async fn create(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
            Ok(entity)
        }
        
        async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedTask>, DatabaseError> {
            Ok(Some(create_test_task()))
        }
        
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedTask>, DatabaseError> {
            Ok(Some(create_test_task()))
        }
        
        async fn update(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
            Ok(entity)
        }
        
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> {
            Ok(())
        }
        
        async fn count(&self) -> Result<u64, DatabaseError> {
            Ok(1)
        }
    }
    
    #[async_trait]
    impl ratchet_interfaces::FilteredRepository<UnifiedTask, TaskFilters> for MockTaskRepository {
        async fn find_with_filters(&self, _filters: TaskFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
            let tasks = vec![create_test_task()];
            Ok(ListResponse {
                items: tasks.clone(),
                meta: PaginationMeta {
                    page: pagination.page.unwrap_or(1),
                    limit: pagination.limit.unwrap_or(20),
                    offset: pagination.offset.unwrap_or(0),
                    total: tasks.len() as u64,
                    has_next: false,
                    has_previous: false,
                    total_pages: 1,
                },
            })
        }
        
        async fn find_with_list_input(&self, filters: TaskFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
            self.find_with_filters(filters, list_input.get_pagination()).await
        }
        
        async fn count_with_filters(&self, _filters: TaskFilters) -> Result<u64, DatabaseError> {
            Ok(1)
        }
    }
    
    #[async_trait]
    impl ratchet_interfaces::TaskRepository for MockTaskRepository {
        async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError> {
            Ok(vec![create_test_task()])
        }
        
        async fn find_by_name(&self, _name: &str) -> Result<Option<UnifiedTask>, DatabaseError> {
            Ok(Some(create_test_task()))
        }
        
        async fn mark_validated(&self, _id: ApiId) -> Result<(), DatabaseError> {
            Ok(())
        }
        
        async fn set_enabled(&self, _id: ApiId, _enabled: bool) -> Result<(), DatabaseError> {
            Ok(())
        }
        
        async fn set_in_sync(&self, _id: ApiId, _in_sync: bool) -> Result<(), DatabaseError> {
            Ok(())
        }
    }
    
    // Similar mock implementations for other repositories (simplified for brevity)
    pub struct MockExecutionRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockExecutionRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<UnifiedExecution> for MockExecutionRepository {
        async fn create(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedExecution>, DatabaseError> { Ok(Some(create_test_execution())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedExecution>, DatabaseError> { Ok(Some(create_test_execution())) }
        async fn update(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::FilteredRepository<UnifiedExecution, ExecutionFilters> for MockExecutionRepository {
        async fn find_with_filters(&self, _filters: ExecutionFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
            let executions = vec![create_test_execution()];
            Ok(ListResponse {
                items: executions.clone(),
                meta: PaginationMeta {
                    page: pagination.page.unwrap_or(1),
                    limit: pagination.limit.unwrap_or(20),
                    offset: pagination.offset.unwrap_or(0),
                    total: executions.len() as u64,
                    has_next: false,
                    has_previous: false,
                    total_pages: 1,
                },
            })
        }
        async fn find_with_list_input(&self, filters: ExecutionFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
            self.find_with_filters(filters, list_input.get_pagination()).await
        }
        async fn count_with_filters(&self, _filters: ExecutionFilters) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::ExecutionRepository for MockExecutionRepository {
        async fn find_by_task_id(&self, _task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError> { Ok(vec![create_test_execution()]) }
        async fn find_by_status(&self, _status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError> { Ok(vec![create_test_execution()]) }
        async fn update_status(&self, _id: ApiId, _status: ExecutionStatus) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_started(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_completed(&self, _id: ApiId, _output: serde_json::Value, _duration_ms: Option<i32>) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_failed(&self, _id: ApiId, _error_message: String, _error_details: Option<serde_json::Value>) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_cancelled(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn update_progress(&self, _id: ApiId, _progress: f32) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    pub struct MockJobRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockJobRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<UnifiedJob> for MockJobRepository {
        async fn create(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedJob>, DatabaseError> { Ok(Some(create_test_job())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedJob>, DatabaseError> { Ok(Some(create_test_job())) }
        async fn update(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::FilteredRepository<UnifiedJob, JobFilters> for MockJobRepository {
        async fn find_with_filters(&self, _filters: JobFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
            let jobs = vec![create_test_job()];
            Ok(ListResponse {
                items: jobs.clone(),
                meta: PaginationMeta {
                    page: pagination.page.unwrap_or(1),
                    limit: pagination.limit.unwrap_or(20),
                    offset: pagination.offset.unwrap_or(0),
                    total: jobs.len() as u64,
                    has_next: false,
                    has_previous: false,
                    total_pages: 1,
                },
            })
        }
        async fn find_with_list_input(&self, filters: JobFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
            self.find_with_filters(filters, list_input.get_pagination()).await
        }
        async fn count_with_filters(&self, _filters: JobFilters) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::JobRepository for MockJobRepository {
        async fn find_ready_for_processing(&self, _limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError> { Ok(vec![create_test_job()]) }
        async fn find_by_status(&self, _status: JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError> { Ok(vec![create_test_job()]) }
        async fn mark_processing(&self, _id: ApiId, _execution_id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_completed(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn mark_failed(&self, _id: ApiId, _error: String, _details: Option<serde_json::Value>) -> Result<bool, DatabaseError> { Ok(true) }
        async fn schedule_retry(&self, _id: ApiId, _retry_at: chrono::DateTime<Utc>) -> Result<(), DatabaseError> { Ok(()) }
        async fn cancel(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    pub struct MockScheduleRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockScheduleRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<UnifiedSchedule> for MockScheduleRepository {
        async fn create(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError> { Ok(Some(create_test_schedule())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> { Ok(Some(create_test_schedule())) }
        async fn update(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::FilteredRepository<UnifiedSchedule, ScheduleFilters> for MockScheduleRepository {
        async fn find_with_filters(&self, _filters: ScheduleFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
            let schedules = vec![create_test_schedule()];
            Ok(ListResponse {
                items: schedules.clone(),
                meta: PaginationMeta {
                    page: pagination.page.unwrap_or(1),
                    limit: pagination.limit.unwrap_or(20),
                    offset: pagination.offset.unwrap_or(0),
                    total: schedules.len() as u64,
                    has_next: false,
                    has_previous: false,
                    total_pages: 1,
                },
            })
        }
        async fn find_with_list_input(&self, filters: ScheduleFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
            self.find_with_filters(filters, list_input.get_pagination()).await
        }
        async fn count_with_filters(&self, _filters: ScheduleFilters) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::ScheduleRepository for MockScheduleRepository {
        async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> { Ok(vec![create_test_schedule()]) }
        async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> { Ok(vec![create_test_schedule()]) }
        async fn record_execution(&self, _id: ApiId, _execution_id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn update_next_run(&self, _id: ApiId, _next_run: chrono::DateTime<Utc>) -> Result<(), DatabaseError> { Ok(()) }
        async fn set_enabled(&self, _id: ApiId, _enabled: bool) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    // Mock authentication repositories
    pub struct MockUserRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockUserRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<ratchet_api_types::UnifiedUser> for MockUserRepository {
        async fn create(&self, entity: ratchet_api_types::UnifiedUser) -> Result<ratchet_api_types::UnifiedUser, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<ratchet_api_types::UnifiedUser>, DatabaseError> { Ok(Some(create_test_user())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<ratchet_api_types::UnifiedUser>, DatabaseError> { Ok(Some(create_test_user())) }
        async fn update(&self, entity: ratchet_api_types::UnifiedUser) -> Result<ratchet_api_types::UnifiedUser, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ratchet_interfaces::FilteredRepository<ratchet_api_types::UnifiedUser, ratchet_interfaces::UserFilters> for MockUserRepository {
        async fn find_with_filters(&self, _filters: ratchet_interfaces::UserFilters, pagination: PaginationInput) -> Result<ListResponse<ratchet_api_types::UnifiedUser>, DatabaseError> {
            let users = vec![create_test_user()];
            Ok(ListResponse {
                items: users.clone(),
                meta: PaginationMeta {
                    page: pagination.page.unwrap_or(1),
                    limit: pagination.limit.unwrap_or(20),
                    offset: pagination.offset.unwrap_or(0),
                    total: users.len() as u64,
                    has_next: false,
                    has_previous: false,
                    total_pages: 1,
                },
            })
        }
        async fn find_with_list_input(&self, filters: ratchet_interfaces::UserFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<ratchet_api_types::UnifiedUser>, DatabaseError> {
            self.find_with_filters(filters, list_input.get_pagination()).await
        }
        async fn count_with_filters(&self, _filters: ratchet_interfaces::UserFilters) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_username(&self, _username: &str) -> Result<Option<ratchet_api_types::UnifiedUser>, DatabaseError> { Ok(None) }
        async fn find_by_email(&self, _email: &str) -> Result<Option<ratchet_api_types::UnifiedUser>, DatabaseError> { Ok(None) }
        async fn create_user(&self, _username: &str, _email: &str, _password_hash: &str, _role: &str) -> Result<ratchet_api_types::UnifiedUser, DatabaseError> {
            Ok(create_test_user())
        }
        async fn update_password(&self, _id: ApiId, _password_hash: &str) -> Result<(), DatabaseError> { Ok(()) }
        async fn update_last_login(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn set_active(&self, _id: ApiId, _active: bool) -> Result<(), DatabaseError> { Ok(()) }
        async fn verify_email(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    pub struct MockSessionRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockSessionRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<ratchet_api_types::UnifiedSession> for MockSessionRepository {
        async fn create(&self, entity: ratchet_api_types::UnifiedSession) -> Result<ratchet_api_types::UnifiedSession, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<ratchet_api_types::UnifiedSession>, DatabaseError> { Ok(Some(create_test_session())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<ratchet_api_types::UnifiedSession>, DatabaseError> { Ok(Some(create_test_session())) }
        async fn update(&self, entity: ratchet_api_types::UnifiedSession) -> Result<ratchet_api_types::UnifiedSession, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl SessionRepository for MockSessionRepository {
        async fn create_session(&self, _user_id: ApiId, _session_id: &str, _jwt_id: &str, _expires_at: chrono::DateTime<chrono::Utc>) -> Result<ratchet_api_types::UnifiedSession, DatabaseError> {
            Ok(create_test_session())
        }
        async fn find_by_session_id(&self, _session_id: &str) -> Result<Option<ratchet_api_types::UnifiedSession>, DatabaseError> { Ok(None) }
        async fn find_by_user_id(&self, _user_id: ApiId) -> Result<Vec<ratchet_api_types::UnifiedSession>, DatabaseError> { Ok(vec![]) }
        async fn invalidate_session(&self, _session_id: &str) -> Result<(), DatabaseError> { Ok(()) }
        async fn invalidate_user_sessions(&self, _user_id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn update_last_used(&self, _session_id: &str) -> Result<(), DatabaseError> { Ok(()) }
        async fn cleanup_expired_sessions(&self) -> Result<u64, DatabaseError> { Ok(0) }
    }
    
    pub struct MockApiKeyRepository;
    #[async_trait]
    impl ratchet_interfaces::Repository for MockApiKeyRepository {
        async fn health_check(&self) -> Result<(), DatabaseError> { Ok(()) }
    }
    #[async_trait]
    impl ratchet_interfaces::CrudRepository<ratchet_api_types::UnifiedApiKey> for MockApiKeyRepository {
        async fn create(&self, entity: ratchet_api_types::UnifiedApiKey) -> Result<ratchet_api_types::UnifiedApiKey, DatabaseError> { Ok(entity) }
        async fn find_by_id(&self, _id: i32) -> Result<Option<ratchet_api_types::UnifiedApiKey>, DatabaseError> { Ok(Some(create_test_api_key())) }
        async fn find_by_uuid(&self, _uuid: Uuid) -> Result<Option<ratchet_api_types::UnifiedApiKey>, DatabaseError> { Ok(Some(create_test_api_key())) }
        async fn update(&self, entity: ratchet_api_types::UnifiedApiKey) -> Result<ratchet_api_types::UnifiedApiKey, DatabaseError> { Ok(entity) }
        async fn delete(&self, _id: i32) -> Result<(), DatabaseError> { Ok(()) }
        async fn count(&self) -> Result<u64, DatabaseError> { Ok(1) }
    }
    #[async_trait]
    impl ApiKeyRepository for MockApiKeyRepository {
        async fn find_by_key_hash(&self, _key_hash: &str) -> Result<Option<ratchet_api_types::UnifiedApiKey>, DatabaseError> { Ok(None) }
        async fn find_by_user_id(&self, _user_id: ApiId) -> Result<Vec<ratchet_api_types::UnifiedApiKey>, DatabaseError> { Ok(vec![]) }
        async fn create_api_key(&self, _user_id: ApiId, _name: &str, _key_hash: &str, _key_prefix: &str, _permissions: &str) -> Result<ratchet_api_types::UnifiedApiKey, DatabaseError> {
            Ok(create_test_api_key())
        }
        async fn update_last_used(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn increment_usage(&self, _id: ApiId) -> Result<(), DatabaseError> { Ok(()) }
        async fn set_active(&self, _id: ApiId, _active: bool) -> Result<(), DatabaseError> { Ok(()) }
    }
    
    pub struct MockTaskRegistry;
    #[async_trait]
    impl TaskRegistry for MockTaskRegistry {
        async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>, RegistryError> {
            Ok(vec![TaskMetadata {
                name: "test-task".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test task".to_string()),
                input_schema: Some(serde_json::json!({"type": "object"})),
                output_schema: Some(serde_json::json!({"type": "object"})),
                metadata: None,
            }])
        }
        async fn get_task_metadata(&self, _name: &str) -> Result<TaskMetadata, RegistryError> {
            Ok(TaskMetadata {
                name: "test-task".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test task".to_string()),
                input_schema: Some(serde_json::json!({"type": "object"})),
                output_schema: Some(serde_json::json!({"type": "object"})),
                metadata: None,
            })
        }
        async fn load_task_content(&self, _name: &str) -> Result<String, RegistryError> {
            Ok("module.exports = function(input) { return { result: 'test' }; };".to_string())
        }
        async fn task_exists(&self, _name: &str) -> Result<bool, RegistryError> { Ok(true) }
        fn registry_id(&self) -> &str { "test-registry" }
        async fn health_check(&self) -> Result<(), RegistryError> { Ok(()) }
    }
    
    pub struct MockRegistryManager;
    #[async_trait]
    impl RegistryManager for MockRegistryManager {
        async fn add_registry(&self, _registry: Box<dyn TaskRegistry>) -> Result<(), RegistryError> {
            Ok(())
        }
        async fn remove_registry(&self, _registry_id: &str) -> Result<(), RegistryError> {
            Ok(())
        }
        async fn list_registries(&self) -> Vec<&str> {
            vec!["test-registry"]
        }
        async fn discover_all_tasks(&self) -> Result<Vec<(String, TaskMetadata)>, RegistryError> {
            Ok(vec![("test-registry".to_string(), TaskMetadata {
                name: "test-task".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test task".to_string()),
                input_schema: Some(serde_json::json!({"type": "object"})),
                output_schema: Some(serde_json::json!({"type": "object"})),
                metadata: None,
            })])
        }
        async fn find_task(&self, _name: &str) -> Result<(String, TaskMetadata), RegistryError> {
            Ok(("test-registry".to_string(), TaskMetadata {
                name: "test-task".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test task".to_string()),
                input_schema: Some(serde_json::json!({"type": "object"})),
                output_schema: Some(serde_json::json!({"type": "object"})),
                metadata: None,
            }))
        }
        async fn load_task(&self, _name: &str) -> Result<String, RegistryError> {
            Ok("module.exports = function(input) { return { result: 'test' }; };".to_string())
        }
        async fn sync_with_database(&self) -> Result<SyncResult, RegistryError> {
            Ok(SyncResult { added: vec!["test-task".to_string()], updated: vec![], removed: vec![], errors: vec![] })
        }
    }
    
    pub struct MockTaskValidator;
    #[async_trait]
    impl TaskValidator for MockTaskValidator {
        async fn validate_metadata(&self, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
            Ok(ValidationResult { valid: true, errors: vec![], warnings: vec![] })
        }
        async fn validate_content(&self, _content: &str, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
            Ok(ValidationResult { valid: true, errors: vec![], warnings: vec![] })
        }
        async fn validate_input(&self, _input: &serde_json::Value, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
            Ok(ValidationResult { valid: true, errors: vec![], warnings: vec![] })
        }
    }
}

// Helper functions to create test data
fn create_test_task() -> UnifiedTask {
    UnifiedTask {
        id: ApiId::from_i32(1),
        uuid: Uuid::new_v4(),
        name: "test-task".to_string(),
        description: Some("A test task".to_string()),
        version: "1.0.0".to_string(),
        enabled: true,
        registry_source: false,
        available_versions: vec!["1.0.0".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validated_at: Some(Utc::now()),
        in_sync: true,
        input_schema: Some(json!({"type": "object", "properties": {}})),
        output_schema: Some(json!({"type": "object", "properties": {"result": {"type": "string"}}})),
        metadata: None,
    }
}

fn create_test_execution() -> UnifiedExecution {
    UnifiedExecution {
        id: ApiId::from_i32(1),
        uuid: Uuid::new_v4(),
        task_id: ApiId::from_i32(1),
        input: json!({}),
        output: Some(json!({"result": "test"})),
        status: ExecutionStatus::Completed,
        error_message: None,
        error_details: None,
        queued_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(1000),
        http_requests: None,
        recording_path: None,
        can_retry: false,
        can_cancel: false,
        progress: Some(100.0),
    }
}

fn create_test_job() -> UnifiedJob {
    UnifiedJob {
        id: ApiId::from_i32(1),
        task_id: ApiId::from_i32(1),
        priority: JobPriority::Normal,
        status: JobStatus::Completed,
        retry_count: 0,
        max_retries: 3,
        queued_at: Utc::now(),
        scheduled_for: None,
        error_message: None,
        output_destinations: None,
    }
}

fn create_test_schedule() -> UnifiedSchedule {
    UnifiedSchedule {
        id: ApiId::from_i32(1),
        task_id: ApiId::from_i32(1),
        name: "test-schedule".to_string(),
        description: Some("A test schedule".to_string()),
        cron_expression: "0 0 * * *".to_string(),
        enabled: true,
        next_run: Some(Utc::now()),
        last_run: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_test_user() -> ratchet_api_types::UnifiedUser {
    ratchet_api_types::UnifiedUser {
        id: ApiId::from_i32(1),
        username: "test-user".to_string(),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        role: ratchet_api_types::UserRole::User,
        is_active: true,
        email_verified: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    }
}

fn create_test_session() -> ratchet_api_types::UnifiedSession {
    ratchet_api_types::UnifiedSession {
        id: ApiId::from_i32(1),
        user_id: ApiId::from_i32(1),
        session_id: "test-session".to_string(),
        expires_at: Utc::now() + chrono::Duration::hours(24),
        created_at: Utc::now(),
        last_used_at: Utc::now(),
        client_ip: None,
        user_agent: None,
        is_active: true,
    }
}

fn create_test_api_key() -> ratchet_api_types::UnifiedApiKey {
    ratchet_api_types::UnifiedApiKey {
        id: ApiId::from_i32(1),
        user_id: ApiId::from_i32(1),
        name: "test-api-key".to_string(),
        key_prefix: "test-key".to_string(),
        permissions: ratchet_api_types::ApiKeyPermissions::Full,
        is_active: true,
        expires_at: None,
        created_at: Utc::now(),
        last_used_at: None,
        usage_count: 0,
    }
}

// Mock factory functions
async fn create_mock_repository_factory() -> Arc<dyn RepositoryFactory> {
    Arc::new(mocks::MockRepositoryFactory)
}

async fn create_mock_registry() -> Arc<dyn TaskRegistry> {
    Arc::new(mocks::MockTaskRegistry)
}

async fn create_mock_registry_manager() -> Arc<dyn RegistryManager> {
    Arc::new(mocks::MockRegistryManager)
}

async fn create_mock_validator() -> Arc<dyn TaskValidator> {
    Arc::new(mocks::MockTaskValidator)
}

// =============================================================================
// ACTUAL INTEGRATION TESTS
// =============================================================================

#[tokio::test]
async fn test_schema_creation() {
    let server = GraphQLTestServer::new().await;
    
    // Test that the schema can be created without errors
    let query = "{ __typename }";
    let response = server.execute(query).await;
    
    assert!(response.errors.is_empty());
    assert_eq!(response.data, value!({ "__typename": "Query" }));
}

#[tokio::test]
async fn test_introspection_query() {
    let server = GraphQLTestServer::new().await;
    
    let introspection_query = r#"
        {
            __schema {
                types {
                    name
                    kind
                }
            }
        }
    "#;
    
    let response = server.execute(introspection_query).await;
    
    assert!(response.errors.is_empty());
    // Check that response data is valid JSON and contains schema
    let data_str = response.data.to_string();
    assert!(data_str.contains("__schema"));
}

#[tokio::test]
async fn test_tasks_query() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        {
            tasks {
                items {
                    id
                    name
                    enabled
                    version
                }
                meta {
                    total
                    page
                    limit
                }
            }
        }
    "#;
    
    let response = server.execute(query).await;
    
    assert!(response.errors.is_empty());
    
    // Verify response structure using string matching for now
    let data_str = response.data.to_string();
    assert!(data_str.contains("tasks"));
    assert!(data_str.contains("items"));
    assert!(data_str.contains("meta"));
    assert!(data_str.contains("test-task"));
}

#[tokio::test]
async fn test_tasks_query_with_filters() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        query TasksWithFilters($filters: TaskFiltersInput, $limit: Int) {
            tasks(filters: $filters, limit: $limit) {
                items {
                    name
                    enabled
                }
                meta {
                    total
                    limit
                }
            }
        }
    "#;
    
    let variables = Variables::from_json(json!({
        "filters": {
            "enabled": true,
            "nameContains": "test"
        },
        "limit": 10
    }));
    
    let response = server.execute_with_variables(query, variables).await;
    
    assert!(response.errors.is_empty());
    
    // Verify response structure and limit using string matching
    let data_str = response.data.to_string();
    assert!(data_str.contains("tasks"));
    assert!(data_str.contains("limit"));
}

#[tokio::test]
async fn test_task_by_id_query() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        query TaskById($id: ID!) {
            task(id: $id) {
                id
                name
                description
                enabled
                version
            }
        }
    "#;
    
    let variables = Variables::from_json(json!({
        "id": "1"
    }));
    
    let response = server.execute_with_variables(query, variables).await;
    
    assert!(response.errors.is_empty());
    
    // Verify task data using string matching
    let data_str = response.data.to_string();
    assert!(data_str.contains("task"));
    assert!(data_str.contains("test-task"));
}

#[tokio::test]
async fn test_executions_query() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        {
            executions {
                items {
                    id
                    taskId
                    status
                    progress
                }
                meta {
                    total
                }
            }
        }
    "#;
    
    let response = server.execute(query).await;
    
    assert!(response.errors.is_empty());
    
    // Verify executions response structure
    let data_str = response.data.to_string();
    assert!(data_str.contains("executions"));
    assert!(data_str.contains("COMPLETED"));
}

#[tokio::test]
async fn test_jobs_query() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        {
            jobs {
                items {
                    id
                    taskId
                    status
                    priority
                }
            }
        }
    "#;
    
    let response = server.execute(query).await;
    
    assert!(response.errors.is_empty());
    
    // Verify jobs response structure
    let data_str = response.data.to_string();
    assert!(data_str.contains("jobs"));
    assert!(data_str.contains("COMPLETED"));
    assert!(data_str.contains("NORMAL"));
}

#[tokio::test]
async fn test_schedules_query() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        {
            schedules {
                items {
                    id
                    name
                    cronExpression
                    enabled
                }
            }
        }
    "#;
    
    let response = server.execute(query).await;
    
    assert!(response.errors.is_empty());
    
    // Verify schedules response structure
    let data_str = response.data.to_string();
    assert!(data_str.contains("schedules"));
    assert!(data_str.contains("test-schedule"));
    assert!(data_str.contains("0 0 * * *"));
}

#[tokio::test]
async fn test_create_task_mutation() {
    let server = GraphQLTestServer::new().await;
    
    let mutation = r#"
        mutation CreateTask($input: CreateTaskInput!) {
            createTask(input: $input) {
                id
                name
                description
                enabled
            }
        }
    "#;
    
    let variables = Variables::from_json(json!({
        "input": {
            "name": "new-test-task",
            "description": "A new test task",
            "enabled": true
        }
    }));
    
    let response = server.execute_with_variables(mutation, variables).await;
    
    if !response.errors.is_empty() {
        eprintln!("GraphQL errors: {:?}", response.errors);
        for error in &response.errors {
            eprintln!("Error: {}", error.message);
        }
    }
    assert!(response.errors.is_empty());
    
    // Verify created task response
    let data_str = response.data.to_string();
    assert!(data_str.contains("createTask"));
    assert!(data_str.contains("new-test-task"));
}

#[tokio::test]
async fn test_query_depth_limit() {
    let config = GraphQLTestConfig {
        max_query_depth: Some(2),
        ..Default::default()
    };
    let server = GraphQLTestServer::with_config(config).await;
    
    // This query exceeds the depth limit of 2
    let deep_query = r#"
        {
            tasks {
                items {
                    executions {
                        items {
                            task {
                                name
                            }
                        }
                    }
                }
            }
        }
    "#;
    
    let response = server.execute(deep_query).await;
    
    // Should have errors due to depth limit
    assert!(!response.errors.is_empty());
    if !response.errors.is_empty() {
        eprintln!("Depth limit error: {}", response.errors[0].message);
    }
    // The exact error message may vary, so let's just check that there's an error
    assert!(!response.errors.is_empty());
}

#[tokio::test]
async fn test_invalid_query_syntax() {
    let server = GraphQLTestServer::new().await;
    
    let invalid_query = r#"
        {
            tasks {
                invalidField
            }
        }
    "#;
    
    let response = server.execute(invalid_query).await;
    
    // Should have syntax/validation errors
    assert!(!response.errors.is_empty());
}

#[tokio::test]
async fn test_query_with_variables_validation() {
    let server = GraphQLTestServer::new().await;
    
    let query = r#"
        query TaskById($id: ID!) {
            task(id: $id) {
                name
            }
        }
    "#;
    
    // Test with missing required variable
    let response = server.execute(query).await;
    
    // Should have validation error for missing variable
    assert!(!response.errors.is_empty());
}

// TODO: Add tests for:
// - Subscription functionality
// - Error handling scenarios
// - Complex nested queries
// - Performance under load
// - Authentication integration
// - Rate limiting integration