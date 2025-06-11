//! Builder pattern utilities for creating test entities
//!
//! This module provides convenient builder patterns for creating test data
//! across all entity types in the ratchet-storage system.

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::seaorm::entities::{
    tasks::{Model as Task, ActiveModel as TaskActiveModel},
    executions::{Model as Execution, ActiveModel as ExecutionActiveModel},
    jobs::{Model as Job, ActiveModel as JobActiveModel},
    schedules::{Model as Schedule, ActiveModel as ScheduleActiveModel},
    delivery_results::{Model as DeliveryResult, ActiveModel as DeliveryResultActiveModel},
};
use sea_orm::Set;

/// Builder pattern for creating test tasks
pub struct TaskBuilder {
    task: Task,
}

impl TaskBuilder {
    pub fn new() -> Self {
        Self {
            task: Task {
                id: 1,
                uuid: Uuid::new_v4(),
                name: "test-task".to_string(),
                version: "1.0.0".to_string(),
                source: "test".to_string(),
                input_schema: json!({"type": "object"}),
                output_schema: json!({"type": "object"}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
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

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.task.source = source.into();
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
            version: Set(task.version),
            source: Set(task.source),
            input_schema: Set(task.input_schema),
            output_schema: Set(task.output_schema),
            created_at: Set(task.created_at),
            updated_at: Set(task.updated_at),
        }
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test executions
pub struct ExecutionBuilder {
    execution: Execution,
}

impl ExecutionBuilder {
    pub fn new() -> Self {
        Self {
            execution: Execution {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                job_id: None,
                status: "pending".to_string(),
                input_data: None,
                output_data: None,
                error_message: None,
                error_details: None,
                queued_at: Utc::now(),
                started_at: None,
                completed_at: None,
                duration_ms: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
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

    pub fn with_job_id(mut self, job_id: i32) -> Self {
        self.execution.job_id = Some(job_id);
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.execution.status = status.into();
        self
    }

    pub fn pending(self) -> Self {
        self.with_status("pending")
    }

    pub fn running(mut self) -> Self {
        self.execution.status = "running".to_string();
        self.execution.started_at = Some(Utc::now());
        self
    }

    pub fn completed(mut self) -> Self {
        let now = Utc::now();
        self.execution.status = "completed".to_string();
        self.execution.started_at = Some(now - chrono::Duration::seconds(5));
        self.execution.completed_at = Some(now);
        self.execution.duration_ms = Some(5000);
        self
    }

    pub fn failed(mut self, error_message: impl Into<String>) -> Self {
        let now = Utc::now();
        self.execution.status = "failed".to_string();
        self.execution.started_at = Some(now - chrono::Duration::seconds(2));
        self.execution.completed_at = Some(now);
        self.execution.duration_ms = Some(2000);
        self.execution.error_message = Some(error_message.into());
        self
    }

    pub fn with_input_data(mut self, data: serde_json::Value) -> Self {
        self.execution.input_data = Some(data);
        self
    }

    pub fn with_output_data(mut self, data: serde_json::Value) -> Self {
        self.execution.output_data = Some(data);
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
            job_id: Set(execution.job_id),
            status: Set(execution.status),
            input_data: Set(execution.input_data),
            output_data: Set(execution.output_data),
            error_message: Set(execution.error_message),
            error_details: Set(execution.error_details),
            queued_at: Set(execution.queued_at),
            started_at: Set(execution.started_at),
            completed_at: Set(execution.completed_at),
            duration_ms: Set(execution.duration_ms),
            created_at: Set(execution.created_at),
            updated_at: Set(execution.updated_at),
        }
    }
}

impl Default for ExecutionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test jobs
pub struct JobBuilder {
    job: Job,
}

impl JobBuilder {
    pub fn new() -> Self {
        Self {
            job: Job {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                status: "pending".to_string(),
                priority: 5,
                input_data: None,
                description: None,
                scheduled_for: Utc::now(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
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

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.job.status = status.into();
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.job.priority = priority;
        self
    }

    pub fn high_priority(self) -> Self {
        self.with_priority(1)
    }

    pub fn low_priority(self) -> Self {
        self.with_priority(10)
    }

    pub fn with_input_data(mut self, data: serde_json::Value) -> Self {
        self.job.input_data = Some(data);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.job.description = Some(description.into());
        self
    }

    pub fn scheduled_for(mut self, scheduled_for: chrono::DateTime<Utc>) -> Self {
        self.job.scheduled_for = scheduled_for;
        self
    }

    pub fn immediate(self) -> Self {
        self.scheduled_for(Utc::now())
    }

    pub fn delayed(self, delay: chrono::Duration) -> Self {
        self.scheduled_for(Utc::now() + delay)
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
            status: Set(job.status),
            priority: Set(job.priority),
            input_data: Set(job.input_data),
            description: Set(job.description),
            scheduled_for: Set(job.scheduled_for),
            created_at: Set(job.created_at),
            updated_at: Set(job.updated_at),
        }
    }
}

impl Default for JobBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test schedules
pub struct ScheduleBuilder {
    schedule: Schedule,
}

impl ScheduleBuilder {
    pub fn new() -> Self {
        Self {
            schedule: Schedule {
                id: 1,
                uuid: Uuid::new_v4(),
                task_id: 1,
                name: "test-schedule".to_string(),
                description: None,
                cron_expression: "0 0 * * *".to_string(),
                input_data: None,
                enabled: true,
                next_run: Utc::now() + chrono::Duration::hours(24),
                last_run: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
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

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.schedule.description = Some(description.into());
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
        self.schedule.input_data = Some(data);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.schedule.enabled = enabled;
        self
    }

    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    pub fn with_next_run(mut self, next_run: chrono::DateTime<Utc>) -> Self {
        self.schedule.next_run = next_run;
        self
    }

    pub fn with_last_run(mut self, last_run: chrono::DateTime<Utc>) -> Self {
        self.schedule.last_run = Some(last_run);
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
            description: Set(schedule.description),
            cron_expression: Set(schedule.cron_expression),
            input_data: Set(schedule.input_data),
            enabled: Set(schedule.enabled),
            next_run: Set(schedule.next_run),
            last_run: Set(schedule.last_run),
            created_at: Set(schedule.created_at),
            updated_at: Set(schedule.updated_at),
        }
    }
}

impl Default for ScheduleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating test delivery results
pub struct DeliveryResultBuilder {
    delivery_result: DeliveryResult,
}

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
            }
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

impl Default for DeliveryResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient factory functions for common test scenarios
pub mod factories {
    use super::*;

    /// Create a simple task for testing
    pub fn simple_task() -> Task {
        TaskBuilder::new()
            .with_name("simple-task")
            .with_source("test-source")
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
            .with_output_data(json!({"result": "success"}))
            .build()
    }

    /// Create a failed execution
    pub fn failed_execution() -> Execution {
        ExecutionBuilder::new()
            .failed("Task execution failed")
            .build()
    }

    /// Create a high priority job
    pub fn urgent_job() -> Job {
        JobBuilder::new()
            .high_priority()
            .immediate()
            .with_description("Urgent job")
            .build()
    }

    /// Create a scheduled job
    pub fn scheduled_job() -> Job {
        JobBuilder::new()
            .delayed(chrono::Duration::hours(1))
            .with_description("Scheduled job")
            .build()
    }

    /// Create a daily schedule
    pub fn daily_schedule() -> Schedule {
        ScheduleBuilder::new()
            .with_name("daily-backup")
            .daily()
            .with_description("Daily backup schedule")
            .build()
    }

    /// Create a disabled schedule
    pub fn disabled_schedule() -> Schedule {
        ScheduleBuilder::new()
            .with_name("disabled-schedule")
            .disabled()
            .build()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_builder() {
        let task = TaskBuilder::new()
            .with_name("test-task")
            .with_version("2.0.0")
            .with_source("test-source")
            .build();

        assert_eq!(task.name, "test-task");
        assert_eq!(task.version, "2.0.0");
        assert_eq!(task.source, "test-source");
    }

    #[test]
    fn test_execution_builder() {
        let execution = ExecutionBuilder::new()
            .with_task_id(123)
            .completed()
            .with_output_data(json!({"success": true}))
            .build();

        assert_eq!(execution.task_id, 123);
        assert_eq!(execution.status, "completed");
        assert!(execution.started_at.is_some());
        assert!(execution.completed_at.is_some());
        assert!(execution.duration_ms.is_some());
        assert_eq!(execution.output_data, Some(json!({"success": true})));
    }

    #[test]
    fn test_job_builder() {
        let job = JobBuilder::new()
            .with_task_id(456)
            .high_priority()
            .immediate()
            .with_description("Test job")
            .build();

        assert_eq!(job.task_id, 456);
        assert_eq!(job.priority, 1);
        assert_eq!(job.description, Some("Test job".to_string()));
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
        assert_eq!(execution.status, "completed");

        let job = factories::urgent_job();
        assert_eq!(job.priority, 1);

        let schedule = factories::daily_schedule();
        assert_eq!(schedule.cron_expression, "0 0 * * *");

        let delivery = factories::successful_delivery();
        assert!(delivery.success);
    }
}