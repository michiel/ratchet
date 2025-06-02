/// Conversion utilities between internal types and unified API types
use crate::{
    api::types::*,
    database::entities::{Task, Execution, Job, Schedule},
    services::UnifiedTask as ServiceUnifiedTask,
};
use uuid::Uuid;

/// Convert database Task to unified API type
impl From<Task> for UnifiedTask {
    fn from(task: Task) -> Self {
        Self {
            id: ApiId::from_i32(task.id),
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            enabled: true, // Tasks are enabled by default in the database
            registry_source: false, // TODO: Add this field to database
            available_versions: vec![task.version.clone()], // TODO: Implement version tracking
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: None, // TODO: Add validation tracking
            in_sync: true, // TODO: Add sync tracking
            input_schema: task.input_schema.map(|s| s.0),
            output_schema: task.output_schema.map(|s| s.0),
            metadata: task.metadata.map(|m| m.0),
        }
    }
}

/// Convert service UnifiedTask to API type
impl From<ServiceUnifiedTask> for UnifiedTask {
    fn from(task: ServiceUnifiedTask) -> Self {
        Self {
            id: ApiId::from_uuid(task.uuid),
            uuid: task.uuid,
            name: task.label, // Service uses 'label' field
            description: Some(task.description),
            version: task.version,
            enabled: task.enabled,
            registry_source: task.registry_source,
            available_versions: task.available_versions,
            created_at: task.created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: task.updated_at.unwrap_or_else(chrono::Utc::now),
            validated_at: task.validated_at,
            in_sync: task.in_sync,
            input_schema: None, // Not included in service type
            output_schema: None, // Not included in service type
            metadata: None, // Not included in service type
        }
    }
}

/// Convert database Execution to unified API type
impl From<Execution> for UnifiedExecution {
    fn from(execution: Execution) -> Self {
        // Calculate computed fields
        let can_retry = matches!(
            execution.status,
            crate::database::entities::executions::ExecutionStatus::Failed |
            crate::database::entities::executions::ExecutionStatus::Cancelled
        );
        let can_cancel = matches!(
            execution.status,
            crate::database::entities::executions::ExecutionStatus::Pending |
            crate::database::entities::executions::ExecutionStatus::Running
        );
        
        Self {
            id: ApiId::from_i32(execution.id),
            uuid: execution.uuid,
            task_id: ApiId::from_i32(execution.task_id),
            input: execution.input.0,
            output: execution.output.map(|o| o.0),
            status: execution.status.into(),
            error_message: execution.error_message,
            error_details: execution.error_details.map(|d| d.0),
            queued_at: execution.queued_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
            http_requests: execution.http_requests.map(|r| r.0),
            recording_path: execution.recording_path,
            can_retry,
            can_cancel,
            progress: None, // TODO: Implement progress tracking
        }
    }
}

/// Convert database Job to unified API type
impl From<Job> for UnifiedJob {
    fn from(job: Job) -> Self {
        Self {
            id: ApiId::from_i32(job.id),
            task_id: ApiId::from_i32(job.task_id),
            priority: job.priority.into(),
            status: job.status.into(),
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            queued_at: job.queued_at,
            scheduled_for: job.process_at,
            error_message: job.error_message,
            output_destinations: job.output_destinations
                .map(|dest| dest.0)
                .and_then(|value| serde_json::from_value(value).ok()),
        }
    }
}

/// Convert database Schedule to unified API type
impl From<Schedule> for UnifiedSchedule {
    fn from(schedule: Schedule) -> Self {
        Self {
            id: ApiId::from_i32(schedule.id),
            task_id: ApiId::from_i32(schedule.task_id),
            name: schedule.name,
            description: schedule.description,
            cron_expression: schedule.cron_expression,
            enabled: schedule.enabled,
            next_run: schedule.next_run,
            last_run: schedule.last_run,
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}

/// Convert between internal and API enum types

impl From<crate::database::entities::executions::ExecutionStatus> for ExecutionStatus {
    fn from(status: crate::database::entities::executions::ExecutionStatus) -> Self {
        match status {
            crate::database::entities::executions::ExecutionStatus::Pending => Self::Pending,
            crate::database::entities::executions::ExecutionStatus::Running => Self::Running,
            crate::database::entities::executions::ExecutionStatus::Completed => Self::Completed,
            crate::database::entities::executions::ExecutionStatus::Failed => Self::Failed,
            crate::database::entities::executions::ExecutionStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<ExecutionStatus> for crate::database::entities::executions::ExecutionStatus {
    fn from(status: ExecutionStatus) -> Self {
        match status {
            ExecutionStatus::Pending => Self::Pending,
            ExecutionStatus::Running => Self::Running,
            ExecutionStatus::Completed => Self::Completed,
            ExecutionStatus::Failed => Self::Failed,
            ExecutionStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<crate::database::entities::jobs::JobPriority> for JobPriority {
    fn from(priority: crate::database::entities::jobs::JobPriority) -> Self {
        match priority {
            crate::database::entities::jobs::JobPriority::Low => Self::Low,
            crate::database::entities::jobs::JobPriority::Normal => Self::Normal,
            crate::database::entities::jobs::JobPriority::High => Self::High,
            crate::database::entities::jobs::JobPriority::Critical => Self::Critical,
        }
    }
}

impl From<JobPriority> for crate::database::entities::jobs::JobPriority {
    fn from(priority: JobPriority) -> Self {
        match priority {
            JobPriority::Low => Self::Low,
            JobPriority::Normal => Self::Normal,
            JobPriority::High => Self::High,
            JobPriority::Critical => Self::Critical,
        }
    }
}

impl From<crate::database::entities::jobs::JobStatus> for JobStatus {
    fn from(status: crate::database::entities::jobs::JobStatus) -> Self {
        match status {
            crate::database::entities::jobs::JobStatus::Queued => Self::Queued,
            crate::database::entities::jobs::JobStatus::Processing => Self::Processing,
            crate::database::entities::jobs::JobStatus::Completed => Self::Completed,
            crate::database::entities::jobs::JobStatus::Failed => Self::Failed,
            crate::database::entities::jobs::JobStatus::Cancelled => Self::Cancelled,
            crate::database::entities::jobs::JobStatus::Retrying => Self::Retrying,
        }
    }
}

impl From<JobStatus> for crate::database::entities::jobs::JobStatus {
    fn from(status: JobStatus) -> Self {
        match status {
            JobStatus::Queued => Self::Queued,
            JobStatus::Processing => Self::Processing,
            JobStatus::Completed => Self::Completed,
            JobStatus::Failed => Self::Failed,
            JobStatus::Cancelled => Self::Cancelled,
            JobStatus::Retrying => Self::Retrying,
        }
    }
}

/// Convert output destination types
impl From<crate::output::OutputDestinationConfig> for UnifiedOutputDestination {
    fn from(config: crate::output::OutputDestinationConfig) -> Self {
        Self {
            destination_type: config.destination_type,
            template: config.template,
            filesystem: config.filesystem.map(Into::into),
            webhook: config.webhook.map(Into::into),
        }
    }
}

impl From<crate::output::FilesystemConfig> for UnifiedFilesystemConfig {
    fn from(config: crate::output::FilesystemConfig) -> Self {
        Self {
            path: config.path,
            format: config.format.into(),
            compression: config.compression.map(Into::into),
            permissions: config.permissions,
        }
    }
}

impl From<crate::output::WebhookConfig> for UnifiedWebhookConfig {
    fn from(config: crate::output::WebhookConfig) -> Self {
        Self {
            url: config.url,
            method: config.method.into(),
            timeout_seconds: config.timeout.as_secs() as i32,
            content_type: config.content_type,
            retry_policy: config.retry_policy.map(Into::into),
            authentication: config.authentication.map(Into::into),
        }
    }
}

impl From<crate::output::OutputFormat> for OutputFormat {
    fn from(format: crate::output::OutputFormat) -> Self {
        match format {
            crate::output::OutputFormat::Json => Self::Json,
            crate::output::OutputFormat::Yaml => Self::Yaml,
            crate::output::OutputFormat::Csv => Self::Csv,
            crate::output::OutputFormat::Xml => Self::Xml,
        }
    }
}

impl From<OutputFormat> for crate::output::OutputFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Json => Self::Json,
            OutputFormat::Yaml => Self::Yaml,
            OutputFormat::Csv => Self::Csv,
            OutputFormat::Xml => Self::Xml,
        }
    }
}

impl From<crate::output::HttpMethod> for HttpMethod {
    fn from(method: crate::output::HttpMethod) -> Self {
        match method {
            crate::output::HttpMethod::Get => Self::Get,
            crate::output::HttpMethod::Post => Self::Post,
            crate::output::HttpMethod::Put => Self::Put,
            crate::output::HttpMethod::Patch => Self::Patch,
            crate::output::HttpMethod::Delete => Self::Delete,
        }
    }
}

impl From<HttpMethod> for crate::output::HttpMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => Self::Get,
            HttpMethod::Post => Self::Post,
            HttpMethod::Put => Self::Put,
            HttpMethod::Patch => Self::Patch,
            HttpMethod::Delete => Self::Delete,
        }
    }
}

// Add more conversion implementations as needed for retry policies, authentication, etc.