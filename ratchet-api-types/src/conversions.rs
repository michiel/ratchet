//! Type conversion utilities for ratchet-api-types
//!
//! This module provides placeholder conversion utilities that will be
//! implemented when integrating with specific database and service types.
//! 
//! Note: The actual conversion implementations will depend on the specific
//! database entities and service types available when this crate is integrated.

use crate::{enums::*, ApiId};

// Example conversion patterns that can be implemented when integrating:

impl ApiId {
    /// Convert from a database integer ID (example pattern)
    pub fn from_database_id(id: i32) -> Self {
        Self::from_i32(id)
    }

    /// Convert from an entity UUID (example pattern)  
    pub fn from_entity_uuid(uuid: uuid::Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

// TODO: Implement conversions when integrating with specific database entities
// Example patterns:
//
// impl From<DatabaseTask> for UnifiedTask {
//     fn from(task: DatabaseTask) -> Self {
//         Self {
//             id: ApiId::from_database_id(task.id),
//             uuid: task.uuid,
//             name: task.name,
//             // ... other fields
//         }
//     }
// }
//
// impl From<DatabaseExecution> for UnifiedExecution {
//     fn from(execution: DatabaseExecution) -> Self {
//         Self {
//             id: ApiId::from_database_id(execution.id),
//             uuid: execution.uuid,
//             task_id: ApiId::from_database_id(execution.task_id),
//             // ... other fields
//         }
//     }
// }

/// Helper functions for common conversions

/// Convert a computed execution status to our unified enum
pub fn compute_execution_status(
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    error_message: Option<&str>,
) -> ExecutionStatus {
    if completed_at.is_some() {
        if error_message.is_some() {
            ExecutionStatus::Failed
        } else {
            ExecutionStatus::Completed
        }
    } else if started_at.is_some() {
        ExecutionStatus::Running
    } else {
        ExecutionStatus::Pending
    }
}

/// Compute retry and cancel capabilities for executions
pub fn compute_execution_capabilities(status: ExecutionStatus) -> (bool, bool) {
    let can_retry = matches!(status, ExecutionStatus::Failed | ExecutionStatus::Cancelled);
    let can_cancel = matches!(status, ExecutionStatus::Pending | ExecutionStatus::Running);
    (can_retry, can_cancel)
}