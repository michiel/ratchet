use std::collections::HashMap;
use chrono::{DateTime, Utc};
use ratchet_api_types::ApiId;
use ratchet_interfaces::{TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters};
use ratchet_api_types::{ExecutionStatus, JobStatus, JobPriority};

/// Helper function to parse ApiId from string
fn parse_api_id(s: &str) -> Option<ApiId> {
    Some(ApiId::from_string(s.to_string()))
}

/// Helper function to parse enum from JSON string (uppercase snake case)
fn parse_execution_status(s: &str) -> Option<ExecutionStatus> {
    let upper = s.to_uppercase();
    match upper.as_str() {
        "PENDING" => Some(ExecutionStatus::Pending),
        "RUNNING" => Some(ExecutionStatus::Running),
        "COMPLETED" => Some(ExecutionStatus::Completed),
        "FAILED" => Some(ExecutionStatus::Failed),
        "CANCELLED" => Some(ExecutionStatus::Cancelled),
        _ => None,
    }
}

/// Helper function to parse JobStatus from string
fn parse_job_status(s: &str) -> Option<JobStatus> {
    let upper = s.to_uppercase();
    match upper.as_str() {
        "QUEUED" => Some(JobStatus::Queued),
        "PROCESSING" => Some(JobStatus::Processing),
        "COMPLETED" => Some(JobStatus::Completed),
        "FAILED" => Some(JobStatus::Failed),
        "CANCELLED" => Some(JobStatus::Cancelled),
        "RETRYING" => Some(JobStatus::Retrying),
        _ => None,
    }
}

/// Helper function to parse JobPriority from string
fn parse_job_priority(s: &str) -> Option<JobPriority> {
    let upper = s.to_uppercase();
    match upper.as_str() {
        "LOW" => Some(JobPriority::Low),
        "NORMAL" => Some(JobPriority::Normal),
        "HIGH" => Some(JobPriority::High),
        "CRITICAL" => Some(JobPriority::Critical),
        _ => None,
    }
}

/// Extract filters from query parameters for TaskFilters
pub fn extract_task_filters(filters: &HashMap<String, String>) -> TaskFilters {
    TaskFilters {
        // Basic filters
        name: filters.get("name").cloned(),
        enabled: filters.get("enabled").and_then(|v| v.parse().ok()),
        registry_source: filters.get("registry_source").and_then(|v| v.parse().ok()),
        validated_after: filters.get("validated_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Advanced string filtering (Refine.dev style)
        name_exact: filters.get("name_exact").cloned(),
        name_contains: filters.get("name_like").cloned(), // Refine.dev uses _like suffix
        name_starts_with: filters.get("name_starts_with").cloned(),
        name_ends_with: filters.get("name_ends_with").cloned(),
        
        // Version filtering
        version: filters.get("version").cloned(),
        version_in: filters.get("version_in").map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
        
        // Extended date filtering
        created_after: filters.get("created_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        created_before: filters.get("created_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        updated_after: filters.get("updated_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        updated_before: filters.get("updated_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        validated_before: filters.get("validated_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // ID filtering
        uuid: filters.get("uuid").cloned(),
        uuid_in: filters.get("uuid_in").map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
        id_in: filters.get("id_in").map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect()),
        
        // Advanced boolean filtering
        has_validation: filters.get("has_validation").and_then(|v| v.parse().ok()),
        in_sync: filters.get("in_sync").and_then(|v| v.parse().ok()),
    }
}

/// Extract filters from query parameters for ExecutionFilters
pub fn extract_execution_filters(filters: &HashMap<String, String>) -> ExecutionFilters {
    ExecutionFilters {
        // Basic filters
        task_id: filters.get("task_id").and_then(|v| parse_api_id(v)),
        status: filters.get("status").and_then(|v| parse_execution_status(v)),
        queued_after: filters.get("queued_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        completed_after: filters.get("completed_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Advanced ID filtering
        task_id_in: filters.get("task_id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        id_in: filters.get("id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        
        // Advanced status filtering
        status_in: filters.get("status_in").map(|v| v.split(',').filter_map(|s| parse_execution_status(s.trim())).collect()),
        status_not: filters.get("status_ne").and_then(|v| parse_execution_status(v)),
        
        // Extended date filtering
        queued_before: filters.get("queued_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        started_after: filters.get("started_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        started_before: filters.get("started_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        completed_before: filters.get("completed_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Duration filtering
        duration_min_ms: filters.get("duration_gte").and_then(|v| v.parse().ok()),
        duration_max_ms: filters.get("duration_lte").and_then(|v| v.parse().ok()),
        
        // Progress filtering
        progress_min: filters.get("progress_gte").and_then(|v| v.parse().ok()),
        progress_max: filters.get("progress_lte").and_then(|v| v.parse().ok()),
        has_progress: filters.get("has_progress").and_then(|v| v.parse().ok()),
        
        // Error filtering
        has_error: filters.get("has_error").and_then(|v| v.parse().ok()),
        error_message_contains: filters.get("error_message_like").cloned(),
        
        // Advanced boolean filtering
        can_retry: filters.get("can_retry").and_then(|v| v.parse().ok()),
        can_cancel: filters.get("can_cancel").and_then(|v| v.parse().ok()),
    }
}

/// Extract filters from query parameters for JobFilters
pub fn extract_job_filters(filters: &HashMap<String, String>) -> JobFilters {
    JobFilters {
        // Basic filters
        task_id: filters.get("task_id").and_then(|v| parse_api_id(v)),
        status: filters.get("status").and_then(|v| parse_job_status(v)),
        priority: filters.get("priority").and_then(|v| parse_job_priority(v)),
        queued_after: filters.get("queued_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        scheduled_before: filters.get("scheduled_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Advanced ID filtering
        task_id_in: filters.get("task_id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        id_in: filters.get("id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        
        // Advanced status filtering
        status_in: filters.get("status_in").map(|v| v.split(',').filter_map(|s| parse_job_status(s.trim())).collect()),
        status_not: filters.get("status_ne").and_then(|v| parse_job_status(v)),
        
        // Advanced priority filtering
        priority_in: filters.get("priority_in").map(|v| v.split(',').filter_map(|s| parse_job_priority(s.trim())).collect()),
        priority_min: filters.get("priority_gte").and_then(|v| parse_job_priority(v)),
        
        // Extended date filtering
        queued_before: filters.get("queued_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        scheduled_after: filters.get("scheduled_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Retry filtering
        retry_count_min: filters.get("retry_count_gte").and_then(|v| v.parse().ok()),
        retry_count_max: filters.get("retry_count_lte").and_then(|v| v.parse().ok()),
        max_retries_min: filters.get("max_retries_gte").and_then(|v| v.parse().ok()),
        max_retries_max: filters.get("max_retries_lte").and_then(|v| v.parse().ok()),
        has_retries_remaining: filters.get("has_retries_remaining").and_then(|v| v.parse().ok()),
        
        // Error filtering
        has_error: filters.get("has_error").and_then(|v| v.parse().ok()),
        error_message_contains: filters.get("error_message_like").cloned(),
        
        // Scheduling filtering
        is_scheduled: filters.get("is_scheduled").and_then(|v| v.parse().ok()),
        due_now: filters.get("due_now").and_then(|v| v.parse().ok()),
    }
}

/// Extract filters from query parameters for ScheduleFilters
pub fn extract_schedule_filters(filters: &HashMap<String, String>) -> ScheduleFilters {
    ScheduleFilters {
        // Basic filters
        task_id: filters.get("task_id").and_then(|v| parse_api_id(v)),
        enabled: filters.get("enabled").and_then(|v| v.parse().ok()),
        next_run_before: filters.get("next_run_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Advanced ID filtering
        task_id_in: filters.get("task_id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        id_in: filters.get("id_in").map(|v| v.split(',').filter_map(|s| parse_api_id(s.trim())).collect()),
        
        // Name filtering
        name_contains: filters.get("name_like").cloned(),
        name_exact: filters.get("name").cloned(),
        name_starts_with: filters.get("name_starts_with").cloned(),
        name_ends_with: filters.get("name_ends_with").cloned(),
        
        // Cron expression filtering
        cron_expression_contains: filters.get("cron_expression_like").cloned(),
        cron_expression_exact: filters.get("cron_expression").cloned(),
        
        // Schedule timing filtering
        next_run_after: filters.get("next_run_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        last_run_after: filters.get("last_run_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        last_run_before: filters.get("last_run_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Date range filtering
        created_after: filters.get("created_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        created_before: filters.get("created_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        updated_after: filters.get("updated_after").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        updated_before: filters.get("updated_before").and_then(|v| DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.with_timezone(&Utc))),
        
        // Advanced filtering
        has_next_run: filters.get("has_next_run").and_then(|v| v.parse().ok()),
        has_last_run: filters.get("has_last_run").and_then(|v| v.parse().ok()),
        is_due: filters.get("is_due").and_then(|v| v.parse().ok()),
        overdue: filters.get("overdue").and_then(|v| v.parse().ok()),
    }
}