//! Security and authentication for MCP connections

pub mod auth;
pub mod permissions;
pub mod rate_limit;

pub use auth::{AuthResult, ClientContext, SecurityContext, McpAuth, McpAuthConfig, McpAuthManager};
pub use permissions::{ClientPermissions, PermissionChecker, RateLimits, ResourceQuotas};
pub use rate_limit::{RateLimitConfig, RateLimiter};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Security configuration for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum execution time for any single operation
    pub max_execution_time: Duration,

    /// Maximum number of log entries that can be retrieved in one request
    pub max_log_entries: usize,

    /// Whether to allow execution of potentially dangerous tasks
    pub allow_dangerous_tasks: bool,

    /// Whether audit logging is enabled
    pub audit_log_enabled: bool,

    /// Whether to sanitize inputs
    pub input_sanitization: bool,

    /// Maximum request size in bytes
    pub max_request_size: usize,

    /// Maximum response size in bytes
    pub max_response_size: usize,

    /// Session timeout duration
    pub session_timeout: Duration,

    /// Whether to require encrypted connections
    pub require_encryption: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(300), // 5 minutes
            max_log_entries: 1000,
            allow_dangerous_tasks: false,
            audit_log_enabled: true,
            input_sanitization: true,
            max_request_size: 1024 * 1024,              // 1MB
            max_response_size: 10 * 1024 * 1024,        // 10MB
            session_timeout: Duration::from_secs(3600), // 1 hour
            require_encryption: true,
        }
    }
}


/// Input sanitization utilities
pub struct InputSanitizer;

impl InputSanitizer {
    /// Sanitize a string input by removing potentially dangerous content
    pub fn sanitize_string(input: &str, max_length: usize) -> String {
        // Remove null bytes and control characters (except newlines and tabs)
        let cleaned: String = input
            .chars()
            .filter(|&c| c == '\n' || c == '\t' || (c >= ' ' && c != '\u{7f}'))
            .take(max_length)
            .collect();

        // Basic script injection prevention
        cleaned
            .replace("<script", "&lt;script")
            .replace("</script", "&lt;/script")
            .replace("javascript:", "")
            .replace("data:text/html", "")
    }

    /// Validate that a task name is safe
    pub fn validate_task_name(name: &str) -> bool {
        // Task names should only contain alphanumeric, dash, underscore
        name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && !name.is_empty() && name.len() <= 100
    }

    /// Validate that a resource URI is safe
    pub fn validate_resource_uri(uri: &str) -> bool {
        // Basic URI validation - no dangerous schemes
        if uri.starts_with("javascript:") || uri.starts_with("data:text/html") || uri.starts_with("file://") {
            return false;
        }

        // Check for path traversal
        !uri.contains("../") && !uri.contains("..\\")
    }
}

/// Audit logging for security events
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Client identifier
    pub client_id: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Event details
    pub details: serde_json::Value,

    /// Request ID for correlation
    pub request_id: Option<String>,

    /// Whether the event represents a security violation
    pub is_security_violation: bool,
}

/// Types of audit events
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication attempt
    Authentication,

    /// Authorization check
    Authorization,

    /// Tool execution
    ToolExecution,

    /// Resource access
    ResourceAccess,

    /// Rate limit hit
    RateLimitExceeded,

    /// Security violation
    SecurityViolation,

    /// Configuration change
    ConfigurationChange,

    /// Connection event
    Connection,
}

/// Audit logger
pub struct AuditLogger {
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Log an audit event
    pub async fn log_event(&self, event: AuditEvent) {
        if !self.enabled {
            return;
        }

        // In a real implementation, this would write to a secure audit log
        if event.is_security_violation {
            tracing::warn!(
                target: "ratchet_mcp_security",
                client_id = %event.client_id,
                event_type = ?event.event_type,
                details = %event.details,
                "Security violation detected"
            );
        } else {
            tracing::info!(
                target: "ratchet_mcp_audit",
                client_id = %event.client_id,
                event_type = ?event.event_type,
                "Audit event"
            );
        }
    }

    /// Log authentication event
    pub async fn log_authentication(&self, client_id: &str, success: bool, method: &str, request_id: Option<String>) {
        let event = AuditEvent {
            timestamp: chrono::Utc::now(),
            client_id: client_id.to_string(),
            event_type: AuditEventType::Authentication,
            details: serde_json::json!({
                "success": success,
                "method": method,
            }),
            request_id,
            is_security_violation: !success,
        };

        self.log_event(event).await;
    }

    /// Log authorization event
    pub async fn log_authorization(
        &self,
        client_id: &str,
        resource: &str,
        action: &str,
        allowed: bool,
        request_id: Option<String>,
    ) {
        let event = AuditEvent {
            timestamp: chrono::Utc::now(),
            client_id: client_id.to_string(),
            event_type: AuditEventType::Authorization,
            details: serde_json::json!({
                "resource": resource,
                "action": action,
                "allowed": allowed,
            }),
            request_id,
            is_security_violation: !allowed,
        };

        self.log_event(event).await;
    }

    /// Log tool execution
    pub async fn log_tool_execution(
        &self,
        client_id: &str,
        tool_name: &str,
        success: bool,
        duration_ms: u64,
        request_id: Option<String>,
    ) {
        let event = AuditEvent {
            timestamp: chrono::Utc::now(),
            client_id: client_id.to_string(),
            event_type: AuditEventType::ToolExecution,
            details: serde_json::json!({
                "tool_name": tool_name,
                "success": success,
                "duration_ms": duration_ms,
            }),
            request_id,
            is_security_violation: false,
        };

        self.log_event(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_sanitizer() {
        // Test string sanitization
        let dangerous = "<script>alert('xss')</script>";
        let safe = InputSanitizer::sanitize_string(dangerous, 1000);
        assert!(!safe.contains("<script>"));

        // Test task name validation
        assert!(InputSanitizer::validate_task_name("valid-task_name"));
        assert!(!InputSanitizer::validate_task_name("invalid task name"));
        assert!(!InputSanitizer::validate_task_name(""));

        // Test resource URI validation
        assert!(InputSanitizer::validate_resource_uri("https://example.com/resource"));
        assert!(!InputSanitizer::validate_resource_uri("javascript:alert(1)"));
        assert!(!InputSanitizer::validate_resource_uri("../../../etc/passwd"));
    }

    #[test]
    fn test_security_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_execution_time, Duration::from_secs(300));
        assert!(!config.allow_dangerous_tasks);
        assert!(config.audit_log_enabled);
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new(true);

        logger
            .log_authentication("test-client", true, "api_key", Some("req-123".to_string()))
            .await;

        logger
            .log_tool_execution("test-client", "test-tool", true, 100, Some("req-124".to_string()))
            .await;
    }
}
