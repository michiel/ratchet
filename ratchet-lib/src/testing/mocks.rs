use async_trait::async_trait;
use mockall::mock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{
    database::{
        repositories::{TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository},
        entities::{
            tasks::Model as Task,
            executions::Model as Execution,
            jobs::Model as Job,
            schedules::Model as Schedule,
        },
        SafeDatabaseResult, SafeDatabaseError, ErrorCode,
    },
    execution::{TaskExecutor, ExecutionError, ExecutionResult, ExecutionContext},
    http::{HttpManager, HttpError},
    services::{TaskSyncService, ServiceError},
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
        async fn find_enabled(&self) -> SafeDatabaseResult<Vec<Task>>;
        async fn update(&self, task: Task) -> SafeDatabaseResult<Task>;
        async fn mark_validated(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn set_enabled(&self, id: i32, enabled: bool) -> SafeDatabaseResult<()>;
        async fn delete(&self, id: i32) -> SafeDatabaseResult<()>;
        async fn delete_by_uuid(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<()>;
        async fn count(&self) -> SafeDatabaseResult<u64>;
        async fn count_enabled(&self) -> SafeDatabaseResult<u64>;
        async fn name_exists(&self, name: &str) -> SafeDatabaseResult<bool>;
        async fn uuid_exists(&self, uuid: uuid::Uuid) -> SafeDatabaseResult<bool>;
        async fn health_check_send(&self) -> SafeDatabaseResult<()>;
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
    }
}

// Mock execution and services

mock! {
    pub TaskExecutorImpl {}
    
    #[async_trait]
    impl TaskExecutor for TaskExecutorImpl {
        async fn execute_task(
            &self,
            task_id: i32,
            task_path: &str,
            input_data: &serde_json::Value,
            context: ExecutionContext,
        ) -> Result<ExecutionResult, ExecutionError>;
        
        async fn validate_task(&self, task_path: &str) -> Result<bool, ExecutionError>;
        async fn health_check(&self) -> Result<(), ExecutionError>;
    }
}

mock! {
    pub HttpManagerImpl {}
    
    #[async_trait]
    impl HttpManager for HttpManagerImpl {
        async fn fetch(
            &self,
            url: &str,
            method: &str,
            headers: Option<HashMap<String, String>>,
            body: Option<serde_json::Value>,
        ) -> Result<serde_json::Value, HttpError>;
        
        fn set_offline_mode(&self, offline: bool);
        fn is_offline(&self) -> bool;
    }
}

mock! {
    pub TaskSyncServiceImpl {}
    
    #[async_trait]
    impl TaskSyncService for TaskSyncServiceImpl {
        async fn sync_all_tasks(&self) -> Result<Vec<crate::services::UnifiedTask>, ServiceError>;
        async fn sync_task(&self, task_id: i32) -> Result<crate::services::UnifiedTask, ServiceError>;
    }
}

/// Mock factory for creating consistent mock objects
pub struct MockFactory {
    task_repo_calls: Arc<Mutex<Vec<String>>>,
    execution_repo_calls: Arc<Mutex<Vec<String>>>,
}

impl MockFactory {
    pub fn new() -> Self {
        Self {
            task_repo_calls: Arc::new(Mutex::new(Vec::new())),
            execution_repo_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a mock task repository with default expectations
    pub fn task_repository(&self) -> MockTaskRepo {
        let mut mock = MockTaskRepo::new();
        let calls = self.task_repo_calls.clone();

        // Default health check expectation
        mock.expect_health_check_send()
            .returning(move || {
                calls.lock().unwrap().push("health_check_send".to_string());
                Ok(())
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

        // Default empty implementations
        mock.expect_find_all()
            .returning(|| Ok(vec![]));
            
        mock.expect_count()
            .returning(|| Ok(0));

        mock
    }

    /// Create a mock task executor that always succeeds
    pub fn successful_task_executor(&self) -> MockTaskExecutorImpl {
        let mut mock = MockTaskExecutorImpl::new();

        mock.expect_execute_task()
            .returning(|_, _, _, _| {
                Ok(ExecutionResult {
                    success: true,
                    output: Some(serde_json::json!({"result": "success"})),
                    error_message: None,
                    duration_ms: 1000,
                })
            });

        mock.expect_validate_task()
            .returning(|_| Ok(true));

        mock.expect_health_check()
            .returning(|| Ok(()));

        mock
    }

    /// Create a mock task executor that always fails
    pub fn failing_task_executor(&self) -> MockTaskExecutorImpl {
        let mut mock = MockTaskExecutorImpl::new();

        mock.expect_execute_task()
            .returning(|_, _, _, _| {
                Err(ExecutionError::InternalError(
                    "Mock execution failure".to_string()
                ))
            });

        mock.expect_validate_task()
            .returning(|_| Ok(false));

        mock.expect_health_check()
            .returning(|| Err(ExecutionError::InternalError("Mock health check failure".to_string())));

        mock
    }

    /// Create a mock HTTP manager in offline mode
    pub fn offline_http_manager(&self) -> MockHttpManagerImpl {
        let mut mock = MockHttpManagerImpl::new();

        mock.expect_is_offline()
            .returning(|| true);

        mock.expect_set_offline_mode()
            .returning(|_| ());

        mock.expect_fetch()
            .returning(|_, _, _, _| {
                Err(HttpError::NetworkError("Offline mode".to_string()))
            });

        mock
    }

    /// Get the calls made to task repository
    pub fn get_task_repo_calls(&self) -> Vec<String> {
        self.task_repo_calls.lock().unwrap().clone()
    }

    /// Get the calls made to execution repository
    pub fn get_execution_repo_calls(&self) -> Vec<String> {
        self.execution_repo_calls.lock().unwrap().clone()
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
        SafeDatabaseError::new(ErrorCode::NotFound, "Resource not found")
    }

    pub fn internal_error() -> SafeDatabaseError {
        SafeDatabaseError::new(ErrorCode::InternalError, "Internal server error")
    }

    pub fn validation_error() -> SafeDatabaseError {
        SafeDatabaseError::new(ErrorCode::ValidationError, "Validation failed")
    }

    pub fn service_unavailable_error() -> SafeDatabaseError {
        SafeDatabaseError::new(ErrorCode::ServiceUnavailable, "Service unavailable")
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

    #[tokio::test]
    async fn test_successful_task_executor() {
        let factory = MockFactory::new();
        let executor = factory.successful_task_executor();

        let result = executor.execute_task(
            1,
            "/test/task",
            &serde_json::json!({}),
            ExecutionContext::default(),
        ).await.unwrap();

        assert!(result.success);
        assert!(result.output.is_some());
        assert_eq!(result.duration_ms, 1000);
    }

    #[tokio::test]
    async fn test_failing_task_executor() {
        let factory = MockFactory::new();
        let executor = factory.failing_task_executor();

        let result = executor.execute_task(
            1,
            "/test/task",
            &serde_json::json!({}),
            ExecutionContext::default(),
        ).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_offline_http_manager() {
        let factory = MockFactory::new();
        let http_manager = factory.offline_http_manager();

        assert!(http_manager.is_offline());

        let result = http_manager.fetch(
            "https://example.com",
            "GET",
            None,
            None,
        ).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_mock_errors() {
        let not_found = mock_errors::not_found_error();
        assert_eq!(not_found.code.to_http_status(), 404);

        let internal = mock_errors::internal_error();
        assert_eq!(internal.code.to_http_status(), 500);

        let validation = mock_errors::validation_error();
        assert_eq!(validation.code.to_http_status(), 400);
    }
}