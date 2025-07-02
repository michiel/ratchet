//! Builder pattern utilities for creating test entities
//!
//! This module provides convenient builder patterns for creating test data
//! across all entity types in the ratchet-storage system.

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

#[cfg(all(feature = "testing", feature = "seaorm"))]
use crate::seaorm::entities::{
    delivery_results::{ActiveModel as DeliveryResultActiveModel, Model as DeliveryResult},
    executions::{ActiveModel as ExecutionActiveModel, ExecutionStatus, Model as Execution},
    jobs::{ActiveModel as JobActiveModel, JobPriority, JobStatus, Model as Job},
    schedules::{ActiveModel as ScheduleActiveModel, Model as Schedule},
    tasks::{ActiveModel as TaskActiveModel, Model as Task},
};
#[cfg(all(feature = "testing", feature = "seaorm"))]
use sea_orm::Set;

/// Builder pattern for creating test tasks
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub struct TaskBuilder {
    task: Task,
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl TaskBuilder {
    pub fn new() -> Self {
        Self {
            task: Task {
                id: 1,
                uuid: Uuid::new_v4(),
                name: "test-task".to_string(),
                description: Some("A test task".to_string()),
                version: "1.0.0".to_string(),
                path: Some("test/path".to_string()),
                metadata: json!({}),
                input_schema: json!({"type": "object"}),
                output_schema: json!({"type": "object"}),
                enabled: true,
                // New required fields
                source_code: "console.log('test');".to_string(),
                source_type: "javascript".to_string(),
                storage_type: "database".to_string(),
                file_path: Some("test/task.js".to_string()),
                checksum: "test-checksum".to_string(),
                repository_id: 1,
                repository_path: "test/task.js".to_string(),
                last_synced_at: Some(Utc::now()),
                sync_status: "synced".to_string(),
                is_editable: true,
                created_from: "api".to_string(),
                needs_push: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                validated_at: Some(Utc::now()),
                source_modified_at: Some(Utc::now()),
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.task.id = id;
        self
    }

    pub fn with_uuid(mut self, uuid: Uuid) -> Self {
        self.task.uuid = uuid;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.task.name = name.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.task.version = version.into();
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.task.path = Some(path.into());
        self
    }

    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.task.description = description;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.task.enabled = enabled;
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.task.input_schema = schema;
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.task.output_schema = schema;
        self
    }

    pub fn build(self) -> Task {
        self.task
    }

    pub fn build_active_model(self) -> TaskActiveModel {
        let task = self.task;
        TaskActiveModel {
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
            // New fields
            source_code: Set(task.source_code),
            source_type: Set(task.source_type),
            storage_type: Set(task.storage_type),
            file_path: Set(task.file_path),
            checksum: Set(task.checksum),
            repository_id: Set(task.repository_id),
            repository_path: Set(task.repository_path),
            last_synced_at: Set(task.last_synced_at),
            sync_status: Set(task.sync_status),
            is_editable: Set(task.is_editable),
            created_from: Set(task.created_from),
            needs_push: Set(task.needs_push),
            created_at: Set(task.created_at),
            updated_at: Set(task.updated_at),
            validated_at: Set(task.validated_at),
            source_modified_at: Set(task.source_modified_at),
        }
    }
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test executions
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub struct ExecutionBuilder {
    execution: Execution,
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl ExecutionBuilder {
    pub fn new() -> Self {
        Self {
            execution: Execution {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                input: json!({}),
                output: None,
                status: ExecutionStatus::Pending,
                error_message: None,
                error_details: None,
                queued_at: Utc::now(),
                started_at: None,
                completed_at: None,
                duration_ms: None,
                http_requests: None,
                recording_path: None,
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.execution.id = id;
        self
    }

    pub fn with_uuid(mut self, uuid: Uuid) -> Self {
        self.execution.uuid = uuid;
        self
    }

    pub fn with_task_id(mut self, task_id: i32) -> Self {
        self.execution.task_id = task_id;
        self
    }

    // Note: job_id is not a field in the Execution entity
    // Jobs reference executions, not the other way around

    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.execution.status = status;
        self
    }

    pub fn pending(self) -> Self {
        self.with_status(ExecutionStatus::Pending)
    }

    pub fn running(mut self) -> Self {
        self.execution.status = ExecutionStatus::Running;
        self.execution.started_at = Some(Utc::now());
        self
    }

    pub fn completed(mut self) -> Self {
        let now = Utc::now();
        self.execution.status = ExecutionStatus::Completed;
        self.execution.started_at = Some(now - chrono::Duration::seconds(5));
        self.execution.completed_at = Some(now);
        self.execution.duration_ms = Some(5000);
        self
    }

    pub fn failed(mut self, error_message: impl Into<String>) -> Self {
        let now = Utc::now();
        self.execution.status = ExecutionStatus::Failed;
        self.execution.started_at = Some(now - chrono::Duration::seconds(2));
        self.execution.completed_at = Some(now);
        self.execution.duration_ms = Some(2000);
        self.execution.error_message = Some(error_message.into());
        self
    }

    pub fn with_input(mut self, data: serde_json::Value) -> Self {
        self.execution.input = data;
        self
    }

    pub fn with_output(mut self, data: serde_json::Value) -> Self {
        self.execution.output = Some(data);
        self
    }

    pub fn with_duration(mut self, duration_ms: i32) -> Self {
        self.execution.duration_ms = Some(duration_ms);
        self
    }

    pub fn build(self) -> Execution {
        self.execution
    }

    pub fn build_active_model(self) -> ExecutionActiveModel {
        let execution = self.execution;
        ExecutionActiveModel {
            id: Set(execution.id),
            uuid: Set(execution.uuid),
            task_id: Set(execution.task_id),
            input: Set(execution.input),
            output: Set(execution.output),
            status: Set(execution.status),
            error_message: Set(execution.error_message),
            error_details: Set(execution.error_details),
            queued_at: Set(execution.queued_at),
            started_at: Set(execution.started_at),
            completed_at: Set(execution.completed_at),
            duration_ms: Set(execution.duration_ms),
            http_requests: Set(execution.http_requests),
            recording_path: Set(execution.recording_path),
        }
    }
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl Default for ExecutionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test jobs
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub struct JobBuilder {
    job: Job,
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl JobBuilder {
    pub fn new() -> Self {
        Self {
            job: Job {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                execution_id: None,
                schedule_id: None,
                priority: JobPriority::Normal,
                status: JobStatus::Queued,
                input_data: json!({}),
                retry_count: 0,
                max_retries: 3,
                retry_delay_seconds: 60,
                error_message: None,
                error_details: None,
                queued_at: Utc::now(),
                process_at: None,
                started_at: None,
                completed_at: None,
                metadata: None,
                output_destinations: None,
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.job.id = id;
        self
    }

    pub fn with_uuid(mut self, uuid: Uuid) -> Self {
        self.job.uuid = uuid;
        self
    }

    pub fn with_task_id(mut self, task_id: i32) -> Self {
        self.job.task_id = task_id;
        self
    }

    pub fn with_status(mut self, status: JobStatus) -> Self {
        self.job.status = status;
        self
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.job.priority = priority;
        self
    }

    pub fn high_priority(self) -> Self {
        self.with_priority(JobPriority::High)
    }

    pub fn low_priority(self) -> Self {
        self.with_priority(JobPriority::Low)
    }

    pub fn with_input_data(mut self, data: serde_json::Value) -> Self {
        self.job.input_data = data;
        self
    }

    pub fn with_execution_id(mut self, execution_id: i32) -> Self {
        self.job.execution_id = Some(execution_id);
        self
    }

    pub fn with_schedule_id(mut self, schedule_id: i32) -> Self {
        self.job.schedule_id = Some(schedule_id);
        self
    }

    pub fn process_at(mut self, process_at: chrono::DateTime<Utc>) -> Self {
        self.job.process_at = Some(process_at);
        self
    }

    pub fn immediate(self) -> Self {
        self.process_at(Utc::now())
    }

    pub fn delayed(self, delay: chrono::Duration) -> Self {
        self.process_at(Utc::now() + delay)
    }

    pub fn build(self) -> Job {
        self.job
    }

    pub fn build_active_model(self) -> JobActiveModel {
        let job = self.job;
        JobActiveModel {
            id: Set(job.id),
            uuid: Set(job.uuid),
            task_id: Set(job.task_id),
            execution_id: Set(job.execution_id),
            schedule_id: Set(job.schedule_id),
            priority: Set(job.priority),
            status: Set(job.status),
            input_data: Set(job.input_data),
            retry_count: Set(job.retry_count),
            max_retries: Set(job.max_retries),
            retry_delay_seconds: Set(job.retry_delay_seconds),
            error_message: Set(job.error_message),
            error_details: Set(job.error_details),
            queued_at: Set(job.queued_at),
            process_at: Set(job.process_at),
            started_at: Set(job.started_at),
            completed_at: Set(job.completed_at),
            metadata: Set(job.metadata),
            output_destinations: Set(job.output_destinations),
        }
    }
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl Default for JobBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test schedules
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub struct ScheduleBuilder {
    schedule: Schedule,
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl ScheduleBuilder {
    pub fn new() -> Self {
        Self {
            schedule: Schedule {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                name: "test-schedule".to_string(),
                cron_expression: "0 0 * * *".to_string(),
                input_data: json!({}),
                enabled: true,
                next_run_at: Some(Utc::now() + chrono::Duration::hours(24)),
                last_run_at: None,
                execution_count: 0,
                max_executions: None,
                metadata: None,
                output_destinations: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.schedule.id = id;
        self
    }

    pub fn with_uuid(mut self, uuid: Uuid) -> Self {
        self.schedule.uuid = uuid;
        self
    }

    pub fn with_task_id(mut self, task_id: i32) -> Self {
        self.schedule.task_id = task_id;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.schedule.name = name.into();
        self
    }

    pub fn with_execution_count(mut self, count: i32) -> Self {
        self.schedule.execution_count = count;
        self
    }

    pub fn with_max_executions(mut self, max: Option<i32>) -> Self {
        self.schedule.max_executions = max;
        self
    }

    pub fn with_cron(mut self, cron: impl Into<String>) -> Self {
        self.schedule.cron_expression = cron.into();
        self
    }

    pub fn daily(self) -> Self {
        self.with_cron("0 0 * * *")
    }

    pub fn hourly(self) -> Self {
        self.with_cron("0 * * * *")
    }

    pub fn every_minute(self) -> Self {
        self.with_cron("* * * * *")
    }

    pub fn with_input_data(mut self, data: serde_json::Value) -> Self {
        self.schedule.input_data = data;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.schedule.enabled = enabled;
        self
    }

    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    pub fn with_next_run_at(mut self, next_run: chrono::DateTime<Utc>) -> Self {
        self.schedule.next_run_at = Some(next_run);
        self
    }

    pub fn with_last_run_at(mut self, last_run: chrono::DateTime<Utc>) -> Self {
        self.schedule.last_run_at = Some(last_run);
        self
    }

    pub fn build(self) -> Schedule {
        self.schedule
    }

    pub fn build_active_model(self) -> ScheduleActiveModel {
        let schedule = self.schedule;
        ScheduleActiveModel {
            id: Set(schedule.id),
            uuid: Set(schedule.uuid),
            task_id: Set(schedule.task_id),
            name: Set(schedule.name),
            cron_expression: Set(schedule.cron_expression),
            input_data: Set(schedule.input_data),
            enabled: Set(schedule.enabled),
            next_run_at: Set(schedule.next_run_at),
            last_run_at: Set(schedule.last_run_at),
            execution_count: Set(schedule.execution_count),
            max_executions: Set(schedule.max_executions),
            metadata: Set(schedule.metadata),
            output_destinations: Set(schedule.output_destinations),
            created_at: Set(schedule.created_at),
            updated_at: Set(schedule.updated_at),
        }
    }
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl Default for ScheduleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test delivery results
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub struct DeliveryResultBuilder {
    delivery_result: DeliveryResult,
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl DeliveryResultBuilder {
    pub fn new() -> Self {
        Self {
            delivery_result: DeliveryResult {
                id: 1,
                job_id: 1,
                execution_id: 1,
                destination_type: "webhook".to_string(),
                destination_id: "test-destination".to_string(),
                success: false,
                delivery_time_ms: 0,
                size_bytes: 0,
                response_info: None,
                error_message: None,
                created_at: Utc::now(),
            },
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.delivery_result.id = id;
        self
    }

    pub fn with_job_id(mut self, job_id: i32) -> Self {
        self.delivery_result.job_id = job_id;
        self
    }

    pub fn with_execution_id(mut self, execution_id: i32) -> Self {
        self.delivery_result.execution_id = execution_id;
        self
    }

    pub fn with_destination_type(mut self, destination_type: impl Into<String>) -> Self {
        self.delivery_result.destination_type = destination_type.into();
        self
    }

    pub fn with_destination_id(mut self, destination_id: impl Into<String>) -> Self {
        self.delivery_result.destination_id = destination_id.into();
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.delivery_result.success = success;
        self
    }

    pub fn successful(mut self) -> Self {
        self.delivery_result.success = true;
        self.delivery_result.delivery_time_ms = 1000;
        self.delivery_result.size_bytes = 256;
        self
    }

    pub fn failed(mut self, error_message: impl Into<String>) -> Self {
        self.delivery_result.success = false;
        self.delivery_result.error_message = Some(error_message.into());
        self
    }

    pub fn with_delivery_time(mut self, delivery_time_ms: i32) -> Self {
        self.delivery_result.delivery_time_ms = delivery_time_ms;
        self
    }

    pub fn with_size_bytes(mut self, size_bytes: i32) -> Self {
        self.delivery_result.size_bytes = size_bytes;
        self
    }

    pub fn with_response_info(mut self, response_info: impl Into<String>) -> Self {
        self.delivery_result.response_info = Some(response_info.into());
        self
    }

    pub fn with_error_message(mut self, error_message: impl Into<String>) -> Self {
        self.delivery_result.error_message = Some(error_message.into());
        self
    }

    pub fn build(self) -> DeliveryResult {
        self.delivery_result
    }

    pub fn build_active_model(self) -> DeliveryResultActiveModel {
        let delivery_result = self.delivery_result;
        DeliveryResultActiveModel {
            id: Set(delivery_result.id),
            job_id: Set(delivery_result.job_id),
            execution_id: Set(delivery_result.execution_id),
            destination_type: Set(delivery_result.destination_type),
            destination_id: Set(delivery_result.destination_id),
            success: Set(delivery_result.success),
            delivery_time_ms: Set(delivery_result.delivery_time_ms),
            size_bytes: Set(delivery_result.size_bytes),
            response_info: Set(delivery_result.response_info),
            error_message: Set(delivery_result.error_message),
            created_at: Set(delivery_result.created_at),
        }
    }
}

#[cfg(all(feature = "testing", feature = "seaorm"))]
impl Default for DeliveryResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient factory functions for common test scenarios
#[cfg(all(feature = "testing", feature = "seaorm"))]
pub mod factories {
    use super::*;

    /// Create a simple task for testing
    pub fn simple_task() -> Task {
        TaskBuilder::new()
            .with_name("simple-task")
            .with_path("test-path")
            .build()
    }

    /// Create a task with complex schemas
    pub fn complex_task() -> Task {
        TaskBuilder::new()
            .with_name("complex-task")
            .with_input_schema(json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"},
                    "options": {"type": "object"}
                },
                "required": ["input"]
            }))
            .with_output_schema(json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"},
                    "metadata": {"type": "object"}
                },
                "required": ["result"]
            }))
            .build()
    }

    /// Create a completed execution
    pub fn completed_execution() -> Execution {
        ExecutionBuilder::new()
            .completed()
            .with_output(json!({"result": "success"}))
            .build()
    }

    /// Create a failed execution
    pub fn failed_execution() -> Execution {
        ExecutionBuilder::new().failed("Task execution failed").build()
    }

    /// Create a high priority job
    pub fn urgent_job() -> Job {
        JobBuilder::new().high_priority().immediate().build()
    }

    /// Create a scheduled job
    pub fn scheduled_job() -> Job {
        JobBuilder::new().delayed(chrono::Duration::hours(1)).build()
    }

    /// Create a daily schedule
    pub fn daily_schedule() -> Schedule {
        ScheduleBuilder::new().with_name("daily-backup").daily().build()
    }

    /// Create a disabled schedule
    pub fn disabled_schedule() -> Schedule {
        ScheduleBuilder::new().with_name("disabled-schedule").disabled().build()
    }

    /// Create a successful delivery result
    pub fn successful_delivery() -> DeliveryResult {
        DeliveryResultBuilder::new()
            .with_destination_type("webhook")
            .with_destination_id("example.com/hook")
            .successful()
            .build()
    }

    /// Create a failed delivery result
    pub fn failed_delivery() -> DeliveryResult {
        DeliveryResultBuilder::new()
            .with_destination_type("webhook")
            .with_destination_id("unreachable.com/hook")
            .failed("Connection timeout")
            .build()
    }
}

#[cfg(all(test, feature = "testing", feature = "seaorm"))]
mod tests {
    use super::*;

    #[test]
    fn test_task_builder() {
        let task = TaskBuilder::new()
            .with_name("test-task")
            .with_version("2.0.0")
            .with_path("test-path")
            .build();

        assert_eq!(task.name, "test-task");
        assert_eq!(task.version, "2.0.0");
        assert_eq!(task.path, Some("test-path".to_string()));
    }

    #[test]
    fn test_execution_builder() {
        let execution = ExecutionBuilder::new()
            .with_task_id(123)
            .completed()
            .with_output(json!({"success": true}))
            .build();

        assert_eq!(execution.task_id, 123);
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert!(execution.started_at.is_some());
        assert!(execution.completed_at.is_some());
        assert!(execution.duration_ms.is_some());
        assert_eq!(execution.output, Some(json!({"success": true})));
    }

    #[test]
    fn test_job_builder() {
        let job = JobBuilder::new().with_task_id(456).high_priority().immediate().build();

        assert_eq!(job.task_id, 456);
        assert_eq!(job.priority, JobPriority::High);
    }

    #[test]
    fn test_schedule_builder() {
        let schedule = ScheduleBuilder::new()
            .with_name("test-schedule")
            .hourly()
            .disabled()
            .build();

        assert_eq!(schedule.name, "test-schedule");
        assert_eq!(schedule.cron_expression, "0 * * * *");
        assert!(!schedule.enabled);
    }

    #[test]
    fn test_delivery_result_builder() {
        let delivery = DeliveryResultBuilder::new()
            .with_execution_id(789)
            .with_destination_type("webhook")
            .with_destination_id("test.com")
            .successful()
            .build();

        assert_eq!(delivery.execution_id, 789);
        assert_eq!(delivery.destination_type, "webhook");
        assert_eq!(delivery.destination_id, "test.com");
        assert!(delivery.success);
    }

    #[test]
    fn test_factories() {
        let task = factories::simple_task();
        assert_eq!(task.name, "simple-task");

        let execution = factories::completed_execution();
        assert_eq!(execution.status, ExecutionStatus::Completed);

        let job = factories::urgent_job();
        assert_eq!(job.priority, JobPriority::High);

        let schedule = factories::daily_schedule();
        assert_eq!(schedule.cron_expression, "0 0 * * *");

        let delivery = factories::successful_delivery();
        assert!(delivery.success);
    }
}
