//! Mock implementations for testing
//!
//! This module provides mock implementations of repositories and services
//! for testing purposes using the mockall framework.

#[cfg(feature = "testing")]
use async_trait::async_trait;
#[cfg(feature = "testing")]
use mockall::mock;
#[cfg(feature = "testing")]
use std::sync::{Arc, Mutex};

#[cfg(all(feature = "testing", feature = "seaorm"))]
use crate::{
    seaorm::safe_errors::SafeDatabaseError,
    StorageError,
};
#[cfg(feature = "testing")]
use ratchet_interfaces::database::{
    TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository,
    TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
    FilteredRepository, CrudRepository, Repository, DatabaseError
};
#[cfg(feature = "testing")]
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus
};

// Mock repository implementations using mockall

#[cfg(feature = "testing")]
mock! {
    pub TaskRepo {}
    
    #[async_trait]
    impl Repository for TaskRepo {
        async fn health_check(&self) -> Result<(), DatabaseError>;
    }
    
    #[async_trait]
    impl CrudRepository<UnifiedTask> for TaskRepo {
        async fn create(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError>;
        async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedTask>, DatabaseError>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedTask>, DatabaseError>;
        async fn update(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError>;
        async fn delete(&self, id: i32) -> Result<(), DatabaseError>;
        async fn count(&self) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl FilteredRepository<UnifiedTask, TaskFilters> for TaskRepo {
        async fn find_with_filters(&self, filters: TaskFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedTask>, DatabaseError>;
        async fn find_with_list_input(&self, filters: TaskFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedTask>, DatabaseError>;
        async fn count_with_filters(&self, filters: TaskFilters) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl TaskRepository for TaskRepo {
        async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError>;
        async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError>;
        async fn mark_validated(&self, id: ApiId) -> Result<(), DatabaseError>;
        async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError>;
        async fn set_in_sync(&self, id: ApiId, in_sync: bool) -> Result<(), DatabaseError>;
    }
}

#[cfg(feature = "testing")]
mock! {
    pub ExecutionRepo {}
    
    #[async_trait]
    impl Repository for ExecutionRepo {
        async fn health_check(&self) -> Result<(), DatabaseError>;
    }
    
    #[async_trait]
    impl CrudRepository<UnifiedExecution> for ExecutionRepo {
        async fn create(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError>;
        async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedExecution>, DatabaseError>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedExecution>, DatabaseError>;
        async fn update(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError>;
        async fn delete(&self, id: i32) -> Result<(), DatabaseError>;
        async fn count(&self) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl FilteredRepository<UnifiedExecution, ExecutionFilters> for ExecutionRepo {
        async fn find_with_filters(&self, filters: ExecutionFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedExecution>, DatabaseError>;
        async fn find_with_list_input(&self, filters: ExecutionFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedExecution>, DatabaseError>;
        async fn count_with_filters(&self, filters: ExecutionFilters) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl ExecutionRepository for ExecutionRepo {
        async fn find_by_task_id(&self, task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError>;
        async fn find_by_status(&self, status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError>;
        async fn update_status(&self, id: ApiId, status: ExecutionStatus) -> Result<(), DatabaseError>;
        async fn mark_started(&self, id: ApiId) -> Result<(), DatabaseError>;
        async fn mark_completed(&self, id: ApiId, output: serde_json::Value, duration_ms: Option<i32>) -> Result<(), DatabaseError>;
        async fn mark_failed(&self, id: ApiId, error_message: String, error_details: Option<serde_json::Value>) -> Result<(), DatabaseError>;
        async fn mark_cancelled(&self, id: ApiId) -> Result<(), DatabaseError>;
        async fn update_progress(&self, id: ApiId, progress: f32) -> Result<(), DatabaseError>;
    }
}

#[cfg(feature = "testing")]
mock! {
    pub JobRepo {}
    
    #[async_trait]
    impl Repository for JobRepo {
        async fn health_check(&self) -> Result<(), DatabaseError>;
    }
    
    #[async_trait]
    impl CrudRepository<UnifiedJob> for JobRepo {
        async fn create(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError>;
        async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedJob>, DatabaseError>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedJob>, DatabaseError>;
        async fn update(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError>;
        async fn delete(&self, id: i32) -> Result<(), DatabaseError>;
        async fn count(&self) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl FilteredRepository<UnifiedJob, JobFilters> for JobRepo {
        async fn find_with_filters(&self, filters: JobFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedJob>, DatabaseError>;
        async fn find_with_list_input(&self, filters: JobFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedJob>, DatabaseError>;
        async fn count_with_filters(&self, filters: JobFilters) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl JobRepository for JobRepo {
        async fn find_ready_for_processing(&self, limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError>;
        async fn find_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError>;
        async fn mark_processing(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError>;
        async fn mark_completed(&self, id: ApiId) -> Result<(), DatabaseError>;
        async fn mark_failed(&self, id: ApiId, error: String, details: Option<serde_json::Value>) -> Result<bool, DatabaseError>;
        async fn schedule_retry(&self, id: ApiId, retry_at: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError>;
        async fn cancel(&self, id: ApiId) -> Result<(), DatabaseError>;
    }
}

#[cfg(feature = "testing")]
mock! {
    pub ScheduleRepo {}
    
    #[async_trait]
    impl Repository for ScheduleRepo {
        async fn health_check(&self) -> Result<(), DatabaseError>;
    }
    
    #[async_trait]
    impl CrudRepository<UnifiedSchedule> for ScheduleRepo {
        async fn create(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError>;
        async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError>;
        async fn update(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError>;
        async fn delete(&self, id: i32) -> Result<(), DatabaseError>;
        async fn count(&self) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl FilteredRepository<UnifiedSchedule, ScheduleFilters> for ScheduleRepo {
        async fn find_with_filters(&self, filters: ScheduleFilters, pagination: PaginationInput) -> Result<ListResponse<UnifiedSchedule>, DatabaseError>;
        async fn find_with_list_input(&self, filters: ScheduleFilters, list_input: ratchet_api_types::pagination::ListInput) -> Result<ListResponse<UnifiedSchedule>, DatabaseError>;
        async fn count_with_filters(&self, filters: ScheduleFilters) -> Result<u64, DatabaseError>;
    }
    
    #[async_trait]
    impl ScheduleRepository for ScheduleRepo {
        async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError>;
        async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError>;
        async fn record_execution(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError>;
        async fn update_next_run(&self, id: ApiId, next_run: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError>;
        async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError>;
    }
}

// DeliveryResultRepository mock removed - not using interface trait pattern

// Mock abstract repository implementations
// NOTE: Temporarily disabled due to trait interface mismatches
// TODO: Fix Repository trait interface and re-enable these mocks

/*
mock! {
    pub AbstractTaskRepo {}
    
    #[async_trait]
    impl Repository<crate::entities::task::Task> for AbstractTaskRepo {
        // ... implementation
    }
    
    #[async_trait]
    impl BaseRepository for AbstractTaskRepo {
        // ... implementation
    }
}
*/

/// Mock factory for creating consistent mock objects
#[cfg(feature = "testing")]
pub struct MockFactory {
    task_repo_calls: Arc<Mutex<Vec<String>>>,
    execution_repo_calls: Arc<Mutex<Vec<String>>>,
    job_repo_calls: Arc<Mutex<Vec<String>>>,
    schedule_repo_calls: Arc<Mutex<Vec<String>>>,
}

#[cfg(feature = "testing")]
impl MockFactory {
    pub fn new() -> Self {
        Self {
            task_repo_calls: Arc::new(Mutex::new(Vec::new())),
            execution_repo_calls: Arc::new(Mutex::new(Vec::new())),
            job_repo_calls: Arc::new(Mutex::new(Vec::new())),
            schedule_repo_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a mock task repository with default expectations
    pub fn task_repository(&self) -> MockTaskRepo {
        let mut mock = MockTaskRepo::new();
        let calls = self.task_repo_calls.clone();

        // Default health check expectation
        mock.expect_health_check()
            .returning(move || {
                calls.lock().unwrap().push("health_check".to_string());
                Ok(())
            });

        mock
    }

    /// Create a mock task repository that always returns empty results
    pub fn empty_task_repository(&self) -> MockTaskRepo {
        let mut mock = self.task_repository();

        mock.expect_find_enabled()
            .returning(|| Ok(vec![]));
            
        mock.expect_count()
            .returning(|| Ok(0));
            
        mock.expect_find_by_id()
            .returning(|_| Ok(None));

        mock.expect_find_by_name()
            .returning(|_| Ok(None));

        mock.expect_find_by_uuid()
            .returning(|_| Ok(None));

        mock
    }

    /// Create a mock task repository with pre-populated unified tasks
    pub fn seeded_unified_task_repository(&self, tasks: Vec<UnifiedTask>) -> MockTaskRepo {
        let mut mock = self.task_repository();
        let tasks_clone = tasks.clone();
        let tasks_for_count = tasks.clone();

        mock.expect_find_enabled()
            .returning(move || Ok(tasks_clone.clone()));
            
        mock.expect_count()
            .returning(move || Ok(tasks_for_count.len() as u64));
            
        // Set up find_by_id expectations for each task
        for task in tasks {
            if let Some(task_id) = task.id.as_i32() {
                let task_clone = task.clone();
                mock.expect_find_by_id()
                    .with(mockall::predicate::eq(task_id))
                    .returning(move |_| Ok(Some(task_clone.clone())));
            }
        }

        mock
    }

    /// Create a mock execution repository with default expectations
    pub fn execution_repository(&self) -> MockExecutionRepo {
        let mut mock = MockExecutionRepo::new();
        let calls = self.execution_repo_calls.clone();

        // Default health check expectation
        mock.expect_health_check()
            .returning(move || {
                calls.lock().unwrap().push("health_check".to_string());
                Ok(())
            });

        // Default empty implementations
        mock.expect_count()
            .returning(|| Ok(0));

        mock
    }

    /// Create a mock job repository with default expectations
    pub fn job_repository(&self) -> MockJobRepo {
        let mut mock = MockJobRepo::new();
        let calls = self.job_repo_calls.clone();

        mock.expect_health_check()
            .returning(move || {
                calls.lock().unwrap().push("health_check".to_string());
                Ok(())
            });

        mock.expect_count()
            .returning(|| Ok(0));

        mock
    }

    /// Create a mock schedule repository with default expectations
    pub fn schedule_repository(&self) -> MockScheduleRepo {
        let mut mock = MockScheduleRepo::new();
        let calls = self.schedule_repo_calls.clone();

        mock.expect_health_check()
            .returning(move || {
                calls.lock().unwrap().push("health_check".to_string());
                Ok(())
            });

        mock.expect_count()
            .returning(|| Ok(0));

        mock
    }


    /// Create a mock abstract task repository that always succeeds
    /// NOTE: Temporarily disabled - abstract repository mocks not working
    /*
    pub fn successful_abstract_task_repository(&self) -> MockAbstractTaskRepo {
        // ... implementation
    }

    /// Create a mock abstract task repository that always fails  
    pub fn failing_abstract_task_repository(&self) -> MockAbstractTaskRepo {
        // ... implementation
    }
    */

    /// Get the calls made to task repository
    pub fn get_task_repo_calls(&self) -> Vec<String> {
        self.task_repo_calls.lock().unwrap().clone()
    }

    /// Get the calls made to execution repository
    pub fn get_execution_repo_calls(&self) -> Vec<String> {
        self.execution_repo_calls.lock().unwrap().clone()
    }

    /// Get the calls made to job repository
    pub fn get_job_repo_calls(&self) -> Vec<String> {
        self.job_repo_calls.lock().unwrap().clone()
    }

    /// Get the calls made to schedule repository
    pub fn get_schedule_repo_calls(&self) -> Vec<String> {
        self.schedule_repo_calls.lock().unwrap().clone()
    }

}

#[cfg(feature = "testing")]
impl Default for MockFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating mock errors
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub mod mock_errors {
    use super::*;

    pub fn not_found_error() -> SafeDatabaseError {
        SafeDatabaseError::new(
            crate::seaorm::safe_errors::ErrorCode::NotFound, 
            "Resource not found"
        )
    }

    pub fn internal_error() -> SafeDatabaseError {
        SafeDatabaseError::new(
            crate::seaorm::safe_errors::ErrorCode::InternalError, 
            "Internal server error"
        )
    }

    pub fn validation_error() -> SafeDatabaseError {
        SafeDatabaseError::new(
            crate::seaorm::safe_errors::ErrorCode::ValidationError, 
            "Validation failed"
        )
    }

    pub fn service_unavailable_error() -> SafeDatabaseError {
        SafeDatabaseError::new(
            crate::seaorm::safe_errors::ErrorCode::ServiceUnavailable, 
            "Service unavailable"
        )
    }

    pub fn storage_connection_error() -> StorageError {
        StorageError::ConnectionFailed("Mock database connection error".to_string())
    }

    pub fn storage_validation_error() -> StorageError {
        StorageError::ValidationFailed("Mock validation error".to_string())
    }

    pub fn storage_not_found_error() -> StorageError {
        StorageError::NotFound
    }
}

#[cfg(all(test, feature = "testing", feature = "seaorm"))]
mod tests {
    use super::*;
    

    #[tokio::test]
    async fn test_mock_factory_empty_repository() {
        let factory = MockFactory::new();
        let mock_repo = factory.empty_task_repository();

        let tasks = mock_repo.find_enabled().await.unwrap();
        assert!(tasks.is_empty());

        let count = mock_repo.count().await.unwrap();
        assert_eq!(count, 0);

        let health = mock_repo.health_check().await;
        assert!(health.is_ok());
    }

    // NOTE: Temporarily disabled - need to create UnifiedTask builders
    /*
    #[tokio::test]
    async fn test_mock_factory_seeded_repository() {
        let factory = MockFactory::new();
        let test_tasks = vec![
            // Need UnifiedTask instances here
        ];

        let mock_repo = factory.seeded_unified_task_repository(test_tasks.clone());

        let tasks = mock_repo.find_enabled().await.unwrap();
        assert_eq!(tasks.len(), 2);

        let count = mock_repo.count().await.unwrap();
        assert_eq!(count, 2);

        let task = mock_repo.find_by_id(1).await.unwrap();
        assert!(task.is_some());
        assert_eq!(task.unwrap().name, "task1");
    }
    */

    // NOTE: Abstract repository tests temporarily disabled
    /*
    #[tokio::test]
    async fn test_successful_abstract_repository() {
        // ... implementation
    }

    #[tokio::test]
    async fn test_failing_abstract_repository() {
        // ... implementation
    }
    */

    #[test]
    fn test_mock_errors() {
        let not_found = mock_errors::not_found_error();
        assert_eq!(not_found.code, crate::seaorm::safe_errors::ErrorCode::NotFound);

        let internal = mock_errors::internal_error();
        assert_eq!(internal.code, crate::seaorm::safe_errors::ErrorCode::InternalError);

        let validation = mock_errors::validation_error();
        assert_eq!(validation.code, crate::seaorm::safe_errors::ErrorCode::ValidationError);

        let storage_connection = mock_errors::storage_connection_error();
        assert!(matches!(storage_connection, StorageError::ConnectionFailed(_)));

        let storage_validation = mock_errors::storage_validation_error();
        assert!(matches!(storage_validation, StorageError::ValidationFailed(_)));

        let storage_not_found = mock_errors::storage_not_found_error();
        assert!(matches!(storage_not_found, StorageError::NotFound));
    }

    #[tokio::test]
    async fn test_call_tracking() {
        let factory = MockFactory::new();
        let mock_repo = factory.task_repository();

        // Make some calls
        let _ = mock_repo.health_check().await;

        // Check that calls were tracked
        let calls = factory.get_task_repo_calls();
        assert!(calls.contains(&"health_check".to_string()));
    }
}