//! Permission system for MCP clients

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Client permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientPermissions {
    /// Whether client can execute tasks
    pub can_execute_tasks: bool,

    /// Whether client can read logs
    pub can_read_logs: bool,

    /// Whether client can read execution traces
    pub can_read_traces: bool,

    /// Patterns of task names this client can execute
    pub allowed_task_patterns: Vec<String>,

    /// Rate limiting configuration
    pub rate_limits: RateLimits,

    /// Resource quota limits
    pub resource_quotas: ResourceQuotas,
}

impl ClientPermissions {
    /// Create permissions with full access
    pub fn full_access() -> Self {
        Self {
            can_execute_tasks: true,
            can_read_logs: true,
            can_read_traces: true,
            allowed_task_patterns: vec!["*".to_string()],
            rate_limits: RateLimits::unlimited(),
            resource_quotas: ResourceQuotas::unlimited(),
        }
    }
    
    /// Create admin permissions (alias for full_access)
    pub fn admin() -> Self {
        Self::full_access()
    }

    /// Create read-only permissions
    pub fn read_only() -> Self {
        Self {
            can_execute_tasks: false,
            can_read_logs: true,
            can_read_traces: true,
            allowed_task_patterns: vec![],
            rate_limits: RateLimits::default(),
            resource_quotas: ResourceQuotas::default(),
        }
    }

    /// Create task execution permissions for specific patterns
    pub fn task_execution(patterns: Vec<String>) -> Self {
        Self {
            can_execute_tasks: true,
            can_read_logs: true,
            can_read_traces: false,
            allowed_task_patterns: patterns,
            rate_limits: RateLimits::default(),
            resource_quotas: ResourceQuotas::default(),
        }
    }

    /// Check if client can execute a specific task
    pub fn can_execute_task(&self, task_name: &str) -> bool {
        if !self.can_execute_tasks {
            return false;
        }

        if self.allowed_task_patterns.is_empty() {
            return false;
        }

        self.allowed_task_patterns
            .iter()
            .any(|pattern| self.matches_pattern(task_name, pattern))
    }

    /// Check if a task name matches a pattern
    fn matches_pattern(&self, task_name: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return task_name.starts_with(prefix);
        }

        if let Some(suffix) = pattern.strip_prefix('*') {
            return task_name.ends_with(suffix);
        }

        task_name == pattern
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per minute for task execution
    pub execute_task_per_minute: Option<u32>,

    /// Requests per minute for reading logs
    pub get_logs_per_minute: Option<u32>,

    /// Requests per minute for reading traces
    pub get_traces_per_minute: Option<u32>,

    /// Overall requests per minute
    pub total_requests_per_minute: Option<u32>,

    /// Maximum concurrent executions
    pub max_concurrent_executions: Option<u32>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            execute_task_per_minute: Some(10),
            get_logs_per_minute: Some(100),
            get_traces_per_minute: Some(50),
            total_requests_per_minute: Some(1000),
            max_concurrent_executions: Some(5),
        }
    }
}

impl RateLimits {
    /// Create unlimited rate limits (use with caution)
    pub fn unlimited() -> Self {
        Self {
            execute_task_per_minute: None,
            get_logs_per_minute: None,
            get_traces_per_minute: None,
            total_requests_per_minute: None,
            max_concurrent_executions: None,
        }
    }

    /// Create strict rate limits for untrusted clients
    pub fn strict() -> Self {
        Self {
            execute_task_per_minute: Some(5),
            get_logs_per_minute: Some(20),
            get_traces_per_minute: Some(10),
            total_requests_per_minute: Some(100),
            max_concurrent_executions: Some(2),
        }
    }
}

/// Resource quota configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    /// Maximum execution time per task (in seconds)
    pub max_execution_time_secs: Option<u64>,

    /// Maximum memory usage per task (in MB)
    pub max_memory_mb: Option<u64>,

    /// Maximum number of log entries per request
    pub max_log_entries_per_request: Option<u32>,

    /// Maximum trace entries per request
    pub max_trace_entries_per_request: Option<u32>,

    /// Maximum request size (in bytes)
    pub max_request_size_bytes: Option<u64>,

    /// Maximum response size (in bytes)
    pub max_response_size_bytes: Option<u64>,
}

impl Default for ResourceQuotas {
    fn default() -> Self {
        Self {
            max_execution_time_secs: Some(300), // 5 minutes
            max_memory_mb: Some(1024),          // 1GB
            max_log_entries_per_request: Some(1000),
            max_trace_entries_per_request: Some(500),
            max_request_size_bytes: Some(1024 * 1024),       // 1MB
            max_response_size_bytes: Some(10 * 1024 * 1024), // 10MB
        }
    }
}

impl ResourceQuotas {
    /// Create unlimited quotas (use with caution)
    pub fn unlimited() -> Self {
        Self {
            max_execution_time_secs: None,
            max_memory_mb: None,
            max_log_entries_per_request: None,
            max_trace_entries_per_request: None,
            max_request_size_bytes: None,
            max_response_size_bytes: None,
        }
    }

    /// Create restrictive quotas for untrusted clients
    pub fn restrictive() -> Self {
        Self {
            max_execution_time_secs: Some(60), // 1 minute
            max_memory_mb: Some(256),          // 256MB
            max_log_entries_per_request: Some(100),
            max_trace_entries_per_request: Some(50),
            max_request_size_bytes: Some(100 * 1024),   // 100KB
            max_response_size_bytes: Some(1024 * 1024), // 1MB
        }
    }

    /// Get execution timeout as Duration
    pub fn execution_timeout(&self) -> Option<Duration> {
        self.max_execution_time_secs.map(Duration::from_secs)
    }
}

/// Permission checker for validating operations
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if client can execute a specific task
    pub fn can_execute_task(permissions: &ClientPermissions, task_name: &str) -> bool {
        permissions.can_execute_task(task_name)
    }

    /// Check if client can read logs
    pub fn can_read_logs(permissions: &ClientPermissions) -> bool {
        permissions.can_read_logs
    }

    /// Check if client can read traces
    pub fn can_read_traces(permissions: &ClientPermissions) -> bool {
        permissions.can_read_traces
    }

    /// Validate request size against quotas
    pub fn validate_request_size(permissions: &ClientPermissions, size_bytes: u64) -> Result<(), String> {
        if let Some(max_size) = permissions.resource_quotas.max_request_size_bytes {
            if size_bytes > max_size {
                return Err(format!(
                    "Request size {} bytes exceeds limit of {} bytes",
                    size_bytes, max_size
                ));
            }
        }
        Ok(())
    }

    /// Validate log request parameters
    pub fn validate_log_request(permissions: &ClientPermissions, max_entries: u32) -> Result<u32, String> {
        if !permissions.can_read_logs {
            return Err("Client does not have permission to read logs".to_string());
        }

        let limit = permissions
            .resource_quotas
            .max_log_entries_per_request
            .unwrap_or(u32::MAX);

        Ok(max_entries.min(limit))
    }

    /// Validate trace request parameters
    pub fn validate_trace_request(permissions: &ClientPermissions, max_entries: u32) -> Result<u32, String> {
        if !permissions.can_read_traces {
            return Err("Client does not have permission to read traces".to_string());
        }

        let limit = permissions
            .resource_quotas
            .max_trace_entries_per_request
            .unwrap_or(u32::MAX);

        Ok(max_entries.min(limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_pattern_matching() {
        let permissions = ClientPermissions::task_execution(vec![
            "test-*".to_string(),
            "safe-task".to_string(),
            "*-read-only".to_string(),
        ]);

        assert!(permissions.can_execute_task("test-anything"));
        assert!(permissions.can_execute_task("test-task"));
        assert!(permissions.can_execute_task("safe-task"));
        assert!(permissions.can_execute_task("data-read-only"));

        assert!(!permissions.can_execute_task("dangerous-task"));
        assert!(!permissions.can_execute_task("test"));
        assert!(!permissions.can_execute_task("read-only"));
    }

    #[test]
    fn test_wildcard_patterns() {
        let permissions = ClientPermissions::task_execution(vec!["*".to_string()]);

        assert!(permissions.can_execute_task("any-task"));
        assert!(permissions.can_execute_task("test"));
        assert!(permissions.can_execute_task("dangerous-task"));
    }

    #[test]
    fn test_permission_presets() {
        let full = ClientPermissions::full_access();
        assert!(full.can_execute_tasks);
        assert!(full.can_read_logs);
        assert!(full.can_read_traces);
        assert!(full.can_execute_task("any-task"));

        let read_only = ClientPermissions::read_only();
        assert!(!read_only.can_execute_tasks);
        assert!(read_only.can_read_logs);
        assert!(read_only.can_read_traces);
        assert!(!read_only.can_execute_task("any-task"));
    }

    #[test]
    fn test_permission_checker() {
        let permissions = ClientPermissions::read_only();

        assert!(PermissionChecker::can_read_logs(&permissions));
        assert!(PermissionChecker::can_read_traces(&permissions));
        assert!(!PermissionChecker::can_execute_task(&permissions, "test-task"));

        // Test request size validation
        let mut permissions = ClientPermissions::default();
        permissions.resource_quotas.max_request_size_bytes = Some(1000);

        assert!(PermissionChecker::validate_request_size(&permissions, 500).is_ok());
        assert!(PermissionChecker::validate_request_size(&permissions, 1500).is_err());
    }

    #[test]
    fn test_log_request_validation() {
        let mut permissions = ClientPermissions::default();
        permissions.can_read_logs = true;
        permissions.resource_quotas.max_log_entries_per_request = Some(100);

        // Should allow up to the limit
        let result = PermissionChecker::validate_log_request(&permissions, 50);
        assert_eq!(result.unwrap(), 50);

        // Should cap at the limit
        let result = PermissionChecker::validate_log_request(&permissions, 150);
        assert_eq!(result.unwrap(), 100);

        // Should deny if no permission
        permissions.can_read_logs = false;
        let result = PermissionChecker::validate_log_request(&permissions, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_presets() {
        let default = RateLimits::default();
        assert!(default.execute_task_per_minute.is_some());

        let unlimited = RateLimits::unlimited();
        assert!(unlimited.execute_task_per_minute.is_none());

        let strict = RateLimits::strict();
        assert!(strict.execute_task_per_minute.unwrap() < default.execute_task_per_minute.unwrap());
    }

    #[test]
    fn test_resource_quota_timeout() {
        let quotas = ResourceQuotas::default();
        let timeout = quotas.execution_timeout();
        assert!(timeout.is_some());
        assert_eq!(timeout.unwrap(), Duration::from_secs(300));

        let unlimited = ResourceQuotas::unlimited();
        assert!(unlimited.execution_timeout().is_none());
    }
}
