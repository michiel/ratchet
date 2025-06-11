//! Mock implementations for testing
//!
//! This module provides mock implementations of repositories and services
//! for testing purposes using the mockall framework.

use async_trait::async_trait;
use mockall::mock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{
    seaorm::{
        entities::{
            tasks::Model as Task,
            executions::Model as Execution,
            jobs::Model as Job,
            schedules::Model as Schedule,
            delivery_results::Model as DeliveryResult,
        },
        safe_errors::{SafeDatabaseResult, SafeDatabaseError},
    },
    repositories::{Repository, BaseRepository, DeliveryResultRepository},
    StorageResult, StorageError,
};
use ratchet_interfaces::database::{
    TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository
};

// Mock repository implementations using mockall

mock! {
    pub TaskRepo {}
    
    #[async_trait]
    impl TaskRepository for TaskRepo {
        async fn create(&self, task: Task) -> SafeDatabaseResult<Task>;
        async fn find_by_id(&self, id: i32) -> SafeDatabaseResult<Option<Task>>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<Option<Task>>;
        async fn find_by_name(&self, name: &str) -> SafeDatabaseResult<Option<Task>>;
        async fn find_all(&self) -> SafeDatabaseResult<Vec<Task>>;
        async fn update(&self, task: Task) -> SafeDatabaseResult<Task>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn delete_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn name_exists(&self, name: &str) -> SafeDatabaseResult<bool>;
        async fn uuid_exists(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<bool>;
        async fn health_check(&self) -> SafeDatabaseResult<bool>;
    }
}

mock! {
    pub ExecutionRepo {}
    
    #[async_trait]
    impl ExecutionRepository for ExecutionRepo {
        async fn create(&self, execution: Execution) -> SafeDatabaseResult<Execution>;
        async fn find_by_id(&self, id: i32) -> SafeDatabaseResult<Option<Execution>>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<Option<Execution>>;
        async fn find_all(&self) -> SafeDatabaseResult<Vec<Execution>>;
        async fn find_by_task_id(&self, task_id: i32) -> SafeDatabaseResult<Vec<Execution>>;
        async fn find_by_status(&self, status: &str) -> SafeDatabaseResult<Vec<Execution>>;
        async fn update(&self, execution: Execution) -> SafeDatabaseResult<Execution>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn count_by_status(&self, status: &str) -> SafeDatabaseResult<u64>;
        async fn health_check(&self) -> SafeDatabaseResult<bool>;
    }
}

mock! {
    pub JobRepo {}
    
    #[async_trait]
    impl JobRepository for JobRepo {
        async fn create(&self, job: Job) -> SafeDatabaseResult<Job>;
        async fn find_by_id(&self, id: i32) -> SafeDatabaseResult<Option<Job>>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<Option<Job>>;
        async fn find_all(&self) -> SafeDatabaseResult<Vec<Job>>;
        async fn find_by_status(&self, status: &str) -> SafeDatabaseResult<Vec<Job>>;
        async fn find_ready_for_processing(&self, limit: u64) -> SafeDatabaseResult<Vec<Job>>;
        async fn update(&self, job: Job) -> SafeDatabaseResult<Job>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn count_by_status(&self, status: &str) -> SafeDatabaseResult<u64>;
        async fn health_check(&self) -> SafeDatabaseResult<bool>;
    }
}

mock! {
    pub ScheduleRepo {}
    
    #[async_trait]
    impl ScheduleRepository for ScheduleRepo {
        async fn create(&self, schedule: Schedule) -> SafeDatabaseResult<Schedule>;
        async fn find_by_id(&self, id: i32) -> SafeDatabaseResult<Option<Schedule>>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<Option<Schedule>>;
        async fn find_all(&self) -> SafeDatabaseResult<Vec<Schedule>>;
        async fn find_enabled(&self) -> SafeDatabaseResult<Vec<Schedule>>;
        async fn find_due(&self) -> SafeDatabaseResult<Vec<Schedule>>;
        async fn update(&self, schedule: Schedule) -> SafeDatabaseResult<Schedule>;
        async fn set_enabled(&self, id: i32, enabled: bool) -> SafeDatabaseResult<()>;
        async fn update_next_run(&self, id: i32, next_run: chrono::DateTime<chrono::Utc>) -> SafeDatabaseResult<()>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn count_enabled(&self) -> SafeDatabaseResult<u64>;
        async fn health_check(&self) -> SafeDatabaseResult<bool>;
    }
}

mock! {
    pub DeliveryResultRepo {}
    
    #[async_trait]
    impl DeliveryResultRepository for DeliveryResultRepo {
        async fn create(&self, delivery_result: DeliveryResult) -> SafeDatabaseResult<DeliveryResult>;
        async fn find_by_id(&self, id: i32) -> SafeDatabaseResult<Option<DeliveryResult>>;
        async fn find_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<Option<DeliveryResult>>;
        async fn find_all(&self) -> SafeDatabaseResult<Vec<DeliveryResult>>;
        async fn find_by_execution_id(&self, execution_id: i32) -> SafeDatabaseResult<Vec<DeliveryResult>>;
        async fn find_by_status(&self, status: &str) -> SafeDatabaseResult<Vec<DeliveryResult>>;
        async fn update(&self, delivery_result: DeliveryResult) -> SafeDatabaseResult<DeliveryResult>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn count_by_status(&self, status: &str) -> SafeDatabaseResult<u64>;
        async fn health_check(&self) -> SafeDatabaseResult<bool>;
    }
}

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
pub struct MockFactory {
    task_repo_calls: Arc<Mutex<Vec<String>>>,
    execution_repo_calls: Arc<Mutex<Vec<String>>>,
    job_repo_calls: Arc<Mutex<Vec<String>>>,
    schedule_repo_calls: Arc<Mutex<Vec<String>>>,
    delivery_result_repo_calls: Arc<Mutex<Vec<String>>>,
}

impl MockFactory {
    pub fn new() -> Self {
        Self {
            task_repo_calls: Arc::new(Mutex::new(Vec::new())),
            execution_repo_calls: Arc::new(Mutex::new(Vec::new())),
            job_repo_calls: Arc::new(Mutex::new(Vec::new())),
            schedule_repo_calls: Arc::new(Mutex::new(Vec::new())),
            delivery_result_repo_calls: Arc::new(Mutex::new(Vec::new())),
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
                Ok(true)
            });

        mock
    }

    /// Create a mock task repository that always returns empty results
    pub fn empty_task_repository(&self) -> MockTaskRepo {
        let mut mock = self.task_repository();

        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
        mock.expect_count()
            .returning(|| Ok(0));
            
        mock.expect_find_by_id()
            .returning(|_| Ok(None));

        mock.expect_name_exists()
            .returning(|_| Ok(false));

        mock.expect_uuid_exists()
            .returning(|_| Ok(false));

        mock
    }

    /// Create a mock task repository with pre-populated data
    pub fn seeded_task_repository(&self, tasks: Vec<Task>) -> MockTaskRepo {
        let mut mock = self.task_repository();
        let tasks_clone = tasks.clone();
        let tasks_for_count = tasks.clone();

        mock.expect_find_all()
            .returning(move || Ok(tasks_clone.clone()));
            
        mock.expect_count()
            .returning(move || Ok(tasks_for_count.len() as u64));
            
        // Set up find_by_id expectations
        for task in tasks {
            let task_id = task.id;
            let task_clone = task.clone();
            mock.expect_find_by_id()
                .with(mockall::predicate::eq(task_id))
                .returning(move |_| Ok(Some(task_clone.clone())));
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
                Ok(true)
            });

        // Default empty implementations
        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
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
                Ok(true)
            });

        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
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
                Ok(true)
            });

        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
        mock.expect_count()
            .returning(|| Ok(0));

        mock
    }

    /// Create a mock delivery result repository with default expectations
    pub fn delivery_result_repository(&self) -> MockDeliveryResultRepo {
        let mut mock = MockDeliveryResultRepo::new();
        let calls = self.delivery_result_repo_calls.clone();

        mock.expect_health_check()
            .returning(move || {
                calls.lock().unwrap().push("health_check".to_string());
                Ok(true)
            });

        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
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

    /// Get the calls made to delivery result repository
    pub fn get_delivery_result_repo_calls(&self) -> Vec<String> {
        self.delivery_result_repo_calls.lock().unwrap().clone()
    }
}

impl Default for MockFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating mock errors
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::builders::*;

    #[tokio::test]
    async fn test_mock_factory_empty_repository() {
        let factory = MockFactory::new();
        let mock_repo = factory.empty_task_repository();

        let tasks = mock_repo.find_all().await.unwrap();
        assert!(tasks.is_empty());

        let count = mock_repo.count().await.unwrap();
        assert_eq!(count, 0);

        let health = mock_repo.health_check().await.unwrap();
        assert!(health);
    }

    #[tokio::test]
    async fn test_mock_factory_seeded_repository() {
        let factory = MockFactory::new();
        let test_tasks = vec![
            TaskBuilder::new().with_id(1).with_name("task1").build(),
            TaskBuilder::new().with_id(2).with_name("task2").build(),
        ];

        let mock_repo = factory.seeded_task_repository(test_tasks.clone());

        let tasks = mock_repo.find_all().await.unwrap();
        assert_eq!(tasks.len(), 2);

        let count = mock_repo.count().await.unwrap();
        assert_eq!(count, 2);

        let task = mock_repo.find_by_id(1).await.unwrap();
        assert!(task.is_some());
        assert_eq!(task.unwrap().name, "task1");
    }

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