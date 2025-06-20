//! REST API Testing Framework
//!
//! This module provides the foundation for comprehensive REST API testing.
//! It includes test utilities, mock implementations, and integration test scaffolding.

use serde_json::{json, Value};

/// Test configuration for different testing scenarios
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub use_in_memory_db: bool,
    pub enable_auth: bool,
    pub enable_rate_limiting: bool,
    pub enable_audit_logging: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_in_memory_db: true,
            enable_auth: false,
            enable_rate_limiting: false,
            enable_audit_logging: false,
        }
    }
}

/// Test utility functions for creating test data
pub mod test_utils {
    use super::*;

    pub fn create_test_task_json() -> Value {
        json!({
            "name": "test-task",
            "description": "A test task",
            "version": "1.0.0",
            "enabled": true,
            "input_schema": {"type": "object"},
            "output_schema": {"type": "object"}
        })
    }

    pub fn create_test_execution_json() -> Value {
        json!({
            "task_id": 1,
            "input": {},
            "status": "completed",
            "output": {"result": "test"}
        })
    }

    pub fn create_test_job_json() -> Value {
        json!({
            "task_id": 1,
            "priority": "medium",
            "status": "completed"
        })
    }
}

/// Mock implementations for testing
pub mod mocks {
    // TODO: Implement mock repository and service implementations
    // when needed for full integration testing
}

/// Integration test helpers
pub mod integration {
    use super::*;

    /// Test helper to verify JSON response structure
    pub fn assert_json_structure(json: &Value, expected_fields: &[&str]) {
        for field in expected_fields {
            assert!(json.get(field).is_some(), "Missing field: {}", field);
        }
    }

    /// Test helper to verify pagination metadata
    pub fn assert_pagination_meta(json: &Value) {
        let meta = json.get("meta").expect("Missing pagination meta");
        assert_json_structure(meta, &["page", "limit", "total", "has_next", "has_previous"]);
    }

    /// Test helper to verify error response structure
    pub fn assert_error_response(json: &Value) {
        assert_json_structure(json, &["error", "message"]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert!(config.use_in_memory_db);
        assert!(!config.enable_auth);
    }

    #[test]
    fn test_task_json_creation() {
        let task_json = test_utils::create_test_task_json();
        assert_eq!(task_json["name"], "test-task");
        assert_eq!(task_json["enabled"], true);
    }

    #[test]
    fn test_json_structure_assertion() {
        let test_json = json!({"name": "test", "value": 42});
        integration::assert_json_structure(&test_json, &["name", "value"]);
    }

    #[test]
    #[should_panic(expected = "Missing field")]
    fn test_json_structure_assertion_fails() {
        let test_json = json!({"name": "test"});
        integration::assert_json_structure(&test_json, &["name", "missing_field"]);
    }
}
