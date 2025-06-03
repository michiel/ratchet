//! Unified API types for both REST and GraphQL

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Flexible API ID that supports both integers and UUIDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiId {
    /// Integer ID
    Int(i32),
    /// UUID ID
    Uuid(Uuid),
}

impl std::fmt::Display for ApiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiId::Int(id) => write!(f, "{}", id),
            ApiId::Uuid(id) => write!(f, "{}", id),
        }
    }
}

impl FromStr for ApiId {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try to parse as integer first
        if let Ok(id) = s.parse::<i32>() {
            return Ok(ApiId::Int(id));
        }
        
        // Try to parse as UUID
        if let Ok(uuid) = Uuid::parse_str(s) {
            return Ok(ApiId::Uuid(uuid));
        }
        
        Err(format!("Invalid ID format: {}", s))
    }
}

impl From<i32> for ApiId {
    fn from(id: i32) -> Self {
        ApiId::Int(id)
    }
}

impl From<Uuid> for ApiId {
    fn from(uuid: Uuid) -> Self {
        ApiId::Uuid(uuid)
    }
}

impl ApiId {
    /// Convert to integer if possible
    pub fn as_int(&self) -> Option<i32> {
        match self {
            ApiId::Int(id) => Some(*id),
            ApiId::Uuid(_) => None,
        }
    }
    
    /// Convert to UUID if possible
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            ApiId::Int(_) => None,
            ApiId::Uuid(uuid) => Some(*uuid),
        }
    }
}

/// Unified task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTask {
    pub id: ApiId,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub path: String,
    pub metadata: serde_json::Value,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub enabled: bool,
    pub status: TaskStatus,
    pub tags: Vec<String>,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub validated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub registry_source: Option<String>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Active,
    Inactive,
    Invalid,
    Deprecated,
    Archived,
}

/// Unified execution representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedExecution {
    pub id: ApiId,
    pub uuid: Uuid,
    pub task_id: ApiId,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<i32>,
    pub worker_id: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub metadata: serde_json::Value,
    pub http_requests: Option<serde_json::Value>,
    pub recording_path: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Retrying,
}

/// Unified job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedJob {
    pub id: ApiId,
    pub uuid: Uuid,
    pub task_id: ApiId,
    pub execution_id: Option<ApiId>,
    pub schedule_id: Option<ApiId>,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub input_data: serde_json::Value,
    pub retry_count: i32,
    pub max_retries: i32,
    pub retry_delay_seconds: i32,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub process_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: serde_json::Value,
    pub output_destinations: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Job priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Urgent = 4,
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Retrying,
    Scheduled,
}

/// Unified schedule representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSchedule {
    pub id: ApiId,
    pub uuid: Uuid,
    pub task_id: ApiId,
    pub name: String,
    pub cron_expression: String,
    pub input_data: serde_json::Value,
    pub enabled: bool,
    pub status: ScheduleStatus,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub execution_count: i32,
    pub max_executions: Option<i32>,
    pub metadata: serde_json::Value,
    pub output_destinations: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Schedule status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleStatus {
    Active,
    Inactive,
    Completed,
    Failed,
}

/// Unified worker representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedWorker {
    pub id: String,
    pub status: WorkerStatus,
    pub pid: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub tasks_executed: u64,
    pub tasks_failed: u64,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
    pub pool: Option<String>,
    pub capabilities: Vec<String>,
    pub current_task_id: Option<ApiId>,
    pub metadata: serde_json::Value,
}

/// Worker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub status: WorkerStatus,
    pub pid: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub tasks_executed: u64,
    pub tasks_failed: u64,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
}

/// Worker status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    Starting,
    Ready,
    Busy,
    Idle,
    Stopping,
    Stopped,
    Failed,
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub tasks: TaskStats,
    pub executions: ExecutionStats,
    pub jobs: JobStats,
    pub workers: WorkerStats,
    pub system: SystemResourceStats,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: u64,
    pub active: u64,
    pub inactive: u64,
    pub deprecated: u64,
    pub archived: u64,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub running: u64,
    pub average_duration_ms: f64,
}

/// Job statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStats {
    pub queued: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub scheduled: u64,
}

/// Worker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub total: u32,
    pub active: u32,
    pub idle: u32,
    pub failed: u32,
}

/// System resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_total_mb: f64,
    pub disk_usage_percent: f64,
    pub uptime_seconds: u64,
}

// Conversion functions from storage entities
impl From<ratchet_storage::entities::task::Task> for UnifiedTask {
    fn from(task: ratchet_storage::entities::task::Task) -> Self {
        Self {
            id: ApiId::Int(task.id),
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            path: task.path,
            metadata: task.metadata,
            input_schema: task.input_schema,
            output_schema: task.output_schema,
            enabled: task.enabled,
            status: TaskStatus::from(task.status),
            tags: task.tags,
            deprecated: task.deprecated,
            deprecation_message: task.deprecation_message,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
            registry_source: task.registry_source,
        }
    }
}

impl From<ratchet_storage::entities::task::TaskStatus> for TaskStatus {
    fn from(status: ratchet_storage::entities::task::TaskStatus) -> Self {
        match status {
            ratchet_storage::entities::task::TaskStatus::Pending => TaskStatus::Pending,
            ratchet_storage::entities::task::TaskStatus::Active => TaskStatus::Active,
            ratchet_storage::entities::task::TaskStatus::Inactive => TaskStatus::Inactive,
            ratchet_storage::entities::task::TaskStatus::Invalid => TaskStatus::Invalid,
            ratchet_storage::entities::task::TaskStatus::Deprecated => TaskStatus::Deprecated,
            ratchet_storage::entities::task::TaskStatus::Archived => TaskStatus::Archived,
        }
    }
}

impl From<ratchet_storage::entities::execution::Execution> for UnifiedExecution {
    fn from(execution: ratchet_storage::entities::execution::Execution) -> Self {
        Self {
            id: ApiId::Int(execution.id),
            uuid: execution.uuid,
            task_id: ApiId::Int(execution.task_id),
            input: execution.input,
            output: execution.output,
            status: ExecutionStatus::from(execution.status),
            error_message: execution.error_message,
            error_details: execution.error_details,
            queued_at: execution.queued_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
            worker_id: execution.worker_id,
            retry_count: execution.retry_count,
            max_retries: execution.max_retries,
            metadata: execution.metadata,
            http_requests: execution.http_requests,
            recording_path: execution.recording_path,
            created_at: execution.created_at,
            updated_at: execution.updated_at,
        }
    }
}

impl From<ratchet_storage::entities::execution::ExecutionStatus> for ExecutionStatus {
    fn from(status: ratchet_storage::entities::execution::ExecutionStatus) -> Self {
        match status {
            ratchet_storage::entities::execution::ExecutionStatus::Pending => ExecutionStatus::Pending,
            ratchet_storage::entities::execution::ExecutionStatus::Running => ExecutionStatus::Running,
            ratchet_storage::entities::execution::ExecutionStatus::Completed => ExecutionStatus::Completed,
            ratchet_storage::entities::execution::ExecutionStatus::Failed => ExecutionStatus::Failed,
            ratchet_storage::entities::execution::ExecutionStatus::Cancelled => ExecutionStatus::Cancelled,
            ratchet_storage::entities::execution::ExecutionStatus::TimedOut => ExecutionStatus::TimedOut,
            ratchet_storage::entities::execution::ExecutionStatus::Retrying => ExecutionStatus::Retrying,
        }
    }
}

impl From<ratchet_storage::entities::job::Job> for UnifiedJob {
    fn from(job: ratchet_storage::entities::job::Job) -> Self {
        Self {
            id: ApiId::Int(job.id),
            uuid: job.uuid,
            task_id: ApiId::Int(job.task_id),
            execution_id: job.execution_id.map(ApiId::Int),
            schedule_id: job.schedule_id.map(ApiId::Int),
            priority: JobPriority::from(job.priority),
            status: JobStatus::from(job.status),
            input_data: job.input_data,
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            retry_delay_seconds: job.retry_delay_seconds,
            error_message: job.error_message,
            error_details: job.error_details,
            queued_at: job.queued_at,
            process_at: job.process_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            metadata: job.metadata,
            output_destinations: job.output_destinations,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

impl From<ratchet_storage::entities::job::JobPriority> for JobPriority {
    fn from(priority: ratchet_storage::entities::job::JobPriority) -> Self {
        match priority {
            ratchet_storage::entities::job::JobPriority::Low => JobPriority::Low,
            ratchet_storage::entities::job::JobPriority::Normal => JobPriority::Normal,
            ratchet_storage::entities::job::JobPriority::High => JobPriority::High,
            ratchet_storage::entities::job::JobPriority::Urgent => JobPriority::Urgent,
        }
    }
}

impl From<ratchet_storage::entities::job::JobStatus> for JobStatus {
    fn from(status: ratchet_storage::entities::job::JobStatus) -> Self {
        match status {
            ratchet_storage::entities::job::JobStatus::Queued => JobStatus::Queued,
            ratchet_storage::entities::job::JobStatus::Processing => JobStatus::Processing,
            ratchet_storage::entities::job::JobStatus::Completed => JobStatus::Completed,
            ratchet_storage::entities::job::JobStatus::Failed => JobStatus::Failed,
            ratchet_storage::entities::job::JobStatus::Cancelled => JobStatus::Cancelled,
            ratchet_storage::entities::job::JobStatus::Retrying => JobStatus::Retrying,
            ratchet_storage::entities::job::JobStatus::Scheduled => JobStatus::Scheduled,
        }
    }
}

impl From<ratchet_storage::entities::schedule::Schedule> for UnifiedSchedule {
    fn from(schedule: ratchet_storage::entities::schedule::Schedule) -> Self {
        Self {
            id: ApiId::Int(schedule.id),
            uuid: schedule.uuid,
            task_id: ApiId::Int(schedule.task_id),
            name: schedule.name,
            cron_expression: schedule.cron_expression,
            input_data: schedule.input_data,
            enabled: schedule.enabled,
            status: ScheduleStatus::from(schedule.status),
            next_run_at: schedule.next_run_at,
            last_run_at: schedule.last_run_at,
            execution_count: schedule.execution_count,
            max_executions: schedule.max_executions,
            metadata: schedule.metadata,
            output_destinations: schedule.output_destinations,
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}

impl From<ratchet_storage::entities::schedule::ScheduleStatus> for ScheduleStatus {
    fn from(status: ratchet_storage::entities::schedule::ScheduleStatus) -> Self {
        match status {
            ratchet_storage::entities::schedule::ScheduleStatus::Active => ScheduleStatus::Active,
            ratchet_storage::entities::schedule::ScheduleStatus::Inactive => ScheduleStatus::Inactive,
            ratchet_storage::entities::schedule::ScheduleStatus::Completed => ScheduleStatus::Completed,
            ratchet_storage::entities::schedule::ScheduleStatus::Failed => ScheduleStatus::Failed,
        }
    }
}

// GraphQL scalar types (disabled for now)
// #[cfg(feature = "graphql")]
// mod graphql_scalars {
//     use super::*;
//     use async_graphql::*;
//     
//     #[Scalar]
//     impl ScalarType for ApiId {
//         fn parse(value: Value) -> InputValueResult<Self> {
//             match value {
//                 Value::String(s) => ApiId::from_str(&s).map_err(|e| InputValueError::custom(e)),
//                 Value::Number(n) => {
//                     if let Some(i) = n.as_i64() {
//                         Ok(ApiId::Int(i as i32))
//                     } else {
//                         Err(InputValueError::expected_type(value))
//                     }
//                 }
//                 _ => Err(InputValueError::expected_type(value)),
//             }
//         }
//         
//         fn to_value(&self) -> Value {
//             match self {
//                 ApiId::Int(id) => Value::Number((*id).into()),
//                 ApiId::Uuid(uuid) => Value::String(uuid.to_string()),
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_id_parsing() {
        // Test integer parsing
        let id = ApiId::from_str("123").unwrap();
        assert_eq!(id, ApiId::Int(123));
        assert_eq!(id.as_int(), Some(123));
        assert_eq!(id.as_uuid(), None);
        
        // Test UUID parsing
        let uuid = Uuid::new_v4();
        let id = ApiId::from_str(&uuid.to_string()).unwrap();
        assert_eq!(id, ApiId::Uuid(uuid));
        assert_eq!(id.as_int(), None);
        assert_eq!(id.as_uuid(), Some(uuid));
        
        // Test invalid format
        assert!(ApiId::from_str("invalid").is_err());
    }
    
    #[test]
    fn test_api_id_conversion() {
        let int_id = ApiId::from(42);
        assert_eq!(int_id.to_string(), "42");
        
        let uuid = Uuid::new_v4();
        let uuid_id = ApiId::from(uuid);
        assert_eq!(uuid_id.to_string(), uuid.to_string());
    }
    
    #[test]
    fn test_status_enums() {
        // Test serialization instead of Display since we use serde rename_all
        assert_eq!(serde_json::to_string(&TaskStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&ExecutionStatus::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&JobStatus::Queued).unwrap(), "\"queued\"");
        assert_eq!(serde_json::to_string(&ScheduleStatus::Active).unwrap(), "\"active\"");
    }
    
    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Urgent > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);
    }
    
    #[test]
    fn test_serialization() {
        let task = UnifiedTask {
            id: ApiId::Int(1),
            uuid: Uuid::new_v4(),
            name: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            version: "1.0.0".to_string(),
            path: "/test/task".to_string(),
            metadata: serde_json::json!({"test": true}),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            enabled: true,
            status: TaskStatus::Active,
            tags: vec!["test".to_string()],
            deprecated: false,
            deprecation_message: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None,
            registry_source: None,
        };
        
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("Test Task"));
        
        let deserialized: UnifiedTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Task");
    }
}