//! Error message sanitization to prevent internal data leakage

use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

/// Configuration for error sanitization
#[derive(Debug, Clone)]
pub struct ErrorSanitizationConfig {
    /// Whether to include generic error codes in sanitized messages
    pub include_error_codes: bool,
    /// Whether to include safe context information
    pub include_safe_context: bool,
    /// Maximum length for sanitized error messages
    pub max_message_length: usize,
    /// Custom safe error mappings
    pub custom_mappings: HashMap<String, String>,
}

impl Default for ErrorSanitizationConfig {
    fn default() -> Self {
        Self {
            include_error_codes: true,
            include_safe_context: true,
            max_message_length: 200,
            custom_mappings: HashMap::new(),
        }
    }
}

/// Sanitized error that's safe to return to users
#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct SanitizedError {
    /// User-safe error message
    pub message: String,
    /// Generic error code for client handling
    pub error_code: Option<String>,
    /// Safe context information
    pub context: Option<HashMap<String, String>>,
}

impl SanitizedError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_code: None,
            context: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.context.is_none() {
            self.context = Some(HashMap::new());
        }
        self.context.as_mut().unwrap().insert(key.into(), value.into());
        self
    }
}

/// Error sanitizer that converts internal errors to user-safe messages
pub struct ErrorSanitizer {
    config: ErrorSanitizationConfig,
    sensitive_patterns: Vec<Regex>,
    path_patterns: Vec<Regex>,
}

impl Default for ErrorSanitizer {
    fn default() -> Self {
        Self::new(ErrorSanitizationConfig::default())
    }
}

impl ErrorSanitizer {
    pub fn new(config: ErrorSanitizationConfig) -> Self {
        let sensitive_patterns = vec![
            // Database connection strings
            Regex::new(r"(?i)(postgresql|mysql|sqlite)://[^\s]+").unwrap(),
            // JWT tokens and API keys
            Regex::new(r"(?i)(jwt|token|key|secret|password)[=:\s]+[a-zA-Z0-9+/=]{20,}").unwrap(),
            // File paths (Unix and Windows)
            Regex::new(r"(/[a-zA-Z0-9_\-./]+){2,}|([A-Z]:\\[a-zA-Z0-9_\-\\./]+)").unwrap(),
            // IP addresses
            Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap(),
            // Email addresses
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
            // Environment variables
            Regex::new(r"\$\{[^}]+\}|\$[A-Z_][A-Z0-9_]*").unwrap(),
            // SQL error patterns
            Regex::new(r"(?i)(table|column|constraint|foreign key|primary key)\s+[a-zA-Z0-9_]+").unwrap(),
            // Stack traces
            Regex::new(r"(?m)^\s*at\s+.*$").unwrap(),
            // Function names with line numbers
            Regex::new(r"(in\s+function\s+)?[a-zA-Z_][a-zA-Z0-9_]*::\w+\(\)\s+(at\s+line\s+\d+)?").unwrap(),
        ];

        let path_patterns = vec![
            // Unix paths
            Regex::new(r"/(?:home|root|var|etc|usr|opt)/[a-zA-Z0-9_\-./]*").unwrap(),
            // Windows paths
            Regex::new(r"[A-Z]:\\(?:Users|Windows|Program Files)[a-zA-Z0-9_\-\\./]*").unwrap(),
            // Workspace paths
            Regex::new(r"/workspace/[a-zA-Z0-9_\-./]*").unwrap(),
        ];

        Self {
            config,
            sensitive_patterns,
            path_patterns,
        }
    }

    /// Sanitize an error message for safe external consumption
    pub fn sanitize_error<E: std::error::Error>(&self, error: &E) -> SanitizedError {
        let error_message = format!("{}", error);
        let error_source = error.source().map(|s| format!("{}", s));
        
        // Check for custom mappings first
        if let Some(custom_message) = self.check_custom_mappings(&error_message) {
            return SanitizedError::new(custom_message)
                .with_code("CUSTOM_ERROR");
        }

        // Categorize the error and provide appropriate sanitized message
        let sanitized = self.categorize_and_sanitize(&error_message, error_source.as_deref());
        
        self.apply_final_sanitization(sanitized)
    }

    /// Sanitize a string message directly
    pub fn sanitize_message(&self, message: &str) -> SanitizedError {
        // Check for custom mappings first
        if let Some(custom_message) = self.check_custom_mappings(message) {
            return SanitizedError::new(custom_message)
                .with_code("CUSTOM_ERROR");
        }

        // Categorize the error and provide appropriate sanitized message
        let sanitized = self.categorize_and_sanitize(message, None);
        
        self.apply_final_sanitization(sanitized)
    }

    /// Check for custom error mappings
    fn check_custom_mappings(&self, message: &str) -> Option<String> {
        for (pattern, replacement) in &self.config.custom_mappings {
            if message.contains(pattern) {
                return Some(replacement.clone());
            }
        }
        None
    }

    /// Categorize error and provide appropriate sanitized message
    fn categorize_and_sanitize(&self, message: &str, source: Option<&str>) -> SanitizedError {
        let lower_message = message.to_lowercase();
        let full_context = if let Some(src) = source {
            format!("{} {}", message, src)
        } else {
            message.to_string()
        };

        // Database errors
        if self.is_database_error(&lower_message) {
            return SanitizedError::new("Database operation failed")
                .with_code("DATABASE_ERROR");
        }

        // Authentication/Authorization errors
        if self.is_auth_error(&lower_message) {
            return SanitizedError::new("Authentication or authorization failed")
                .with_code("AUTH_ERROR");
        }

        // Validation errors
        if self.is_validation_error(&lower_message) {
            return SanitizedError::new("Input validation failed")
                .with_code("VALIDATION_ERROR")
                .with_context("hint", "Please check your input format");
        }

        // Database errors (check before filesystem errors to avoid misclassification)
        if self.is_database_error(&lower_message) {
            return SanitizedError::new("Database operation failed")
                .with_code("DATABASE_ERROR");
        }

        // File system errors
        if self.is_filesystem_error(&lower_message) {
            return SanitizedError::new("File operation failed")
                .with_code("FILESYSTEM_ERROR");
        }

        // Network errors
        if self.is_network_error(&lower_message) {
            return SanitizedError::new("Network operation failed")
                .with_code("NETWORK_ERROR");
        }

        // Configuration errors
        if self.is_config_error(&lower_message) {
            return SanitizedError::new("Configuration error")
                .with_code("CONFIG_ERROR");
        }

        // Task execution errors
        if self.is_task_error(&lower_message) {
            return SanitizedError::new("Task execution failed")
                .with_code("TASK_ERROR");
        }

        // Generic sanitization for unknown errors
        let sanitized_text = self.sanitize_text(&full_context);
        SanitizedError::new(if sanitized_text.is_empty() {
            "An error occurred"
        } else {
            &sanitized_text
        }).with_code("INTERNAL_ERROR")
    }

    /// Check if error is database-related
    fn is_database_error(&self, message: &str) -> bool {
        let db_keywords = [
            "database", "sql", "connection", "sqlite", "postgresql", "mysql",
            "table", "column", "constraint", "foreign key", "primary key",
            "deadlock", "timeout", "transaction", "rollback", "commit"
        ];

        db_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Check if error is authentication/authorization-related
    fn is_auth_error(&self, message: &str) -> bool {
        let auth_keywords = [
            "unauthorized", "forbidden", "access denied", "permission",
            "authentication", "authorization", "token", "credential",
            "login", "session", "expired"
        ];

        auth_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Check if error is validation-related
    fn is_validation_error(&self, message: &str) -> bool {
        let validation_keywords = [
            "validation", "invalid", "required", "format", "schema",
            "constraint", "length", "range", "pattern", "type"
        ];

        validation_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Check if error is filesystem-related
    fn is_filesystem_error(&self, message: &str) -> bool {
        let fs_keywords = [
            "file", "directory", "path", "permission", "not found",
            "exists", "read", "write", "create", "delete", "io error"
        ];

        fs_keywords.iter().any(|keyword| message.contains(keyword))
    }


    /// Check if error is network-related
    fn is_network_error(&self, message: &str) -> bool {
        let network_keywords = [
            "network", "connection", "timeout", "dns", "http", "https",
            "ssl", "tls", "certificate", "host", "unreachable", "refused"
        ];

        network_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Check if error is configuration-related
    fn is_config_error(&self, message: &str) -> bool {
        let config_keywords = [
            "config", "configuration", "setting", "option", "parameter",
            "property", "environment", "variable", "missing", "parse"
        ];

        config_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Check if error is task-related
    fn is_task_error(&self, message: &str) -> bool {
        let task_keywords = [
            "task", "execution", "runtime", "script", "javascript",
            "eval", "syntax", "reference", "undefined", "null"
        ];

        task_keywords.iter().any(|keyword| message.contains(keyword))
    }

    /// Sanitize text by removing sensitive information
    fn sanitize_text(&self, text: &str) -> String {
        let mut sanitized = text.to_string();

        // Remove sensitive patterns
        for pattern in &self.sensitive_patterns {
            sanitized = pattern.replace_all(&sanitized, "[REDACTED]").to_string();
        }

        // Remove file paths but keep relative context
        for pattern in &self.path_patterns {
            sanitized = pattern.replace_all(&sanitized, "[PATH]").to_string();
        }

        // Remove stack traces
        let lines: Vec<&str> = sanitized.lines().collect();
        let filtered_lines: Vec<&str> = lines.into_iter()
            .filter(|line| !line.trim().starts_with("at ") && !line.contains("Error:"))
            .take(3) // Limit to first 3 non-stack-trace lines
            .collect();

        sanitized = filtered_lines.join(" ");

        // Remove extra whitespace
        sanitized = sanitized.split_whitespace().collect::<Vec<&str>>().join(" ");

        // Remove common debug information
        sanitized = sanitized
            .replace("Error: ", "")
            .replace("panic: ", "")
            .replace("thread 'main' panicked at", "")
            .replace("note: run with `RUST_BACKTRACE=1`", "");

        sanitized.trim().to_string()
    }

    /// Apply final sanitization rules
    fn apply_final_sanitization(&self, mut error: SanitizedError) -> SanitizedError {
        // Truncate message if too long
        if error.message.len() > self.config.max_message_length {
            error.message = format!("{}...", &error.message[..self.config.max_message_length.saturating_sub(3)]);
        }

        // Ensure message is not empty
        if error.message.trim().is_empty() {
            error.message = "An error occurred".to_string();
        }

        // Remove error code if not configured
        if !self.config.include_error_codes {
            error.error_code = None;
        }

        // Remove context if not configured
        if !self.config.include_safe_context {
            error.context = None;
        }

        error
    }
}

/// Convenience functions for common error types
impl ErrorSanitizer {
    /// Create a sanitized validation error
    pub fn validation_error(message: &str) -> SanitizedError {
        SanitizedError::new("Input validation failed")
            .with_code("VALIDATION_ERROR")
            .with_context("field", message)
    }

    /// Create a sanitized not found error
    pub fn not_found_error(resource_type: &str) -> SanitizedError {
        SanitizedError::new(format!("{} not found", resource_type))
            .with_code("NOT_FOUND")
    }

    /// Create a sanitized permission error
    pub fn permission_error() -> SanitizedError {
        SanitizedError::new("Permission denied")
            .with_code("PERMISSION_DENIED")
    }

    /// Create a sanitized internal error
    pub fn internal_error() -> SanitizedError {
        SanitizedError::new("Internal server error")
            .with_code("INTERNAL_ERROR")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_database_error() {
        let sanitizer = ErrorSanitizer::default();
        let error_msg = "Database connection failed: postgresql://user:pass@localhost:5432/db";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert_eq!(sanitized.message, "Database operation failed");
        assert_eq!(sanitized.error_code, Some("DATABASE_ERROR".to_string()));
        assert!(!sanitized.message.contains("postgresql://"));
    }

    #[test]
    fn test_sanitize_file_path() {
        let sanitizer = ErrorSanitizer::default();
        let error_msg = "Failed to read file: /home/user/secret/config.json";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert_eq!(sanitized.message, "File operation failed");
        assert!(!sanitized.message.contains("/home/user"));
    }

    #[test]
    fn test_sanitize_validation_error() {
        let sanitizer = ErrorSanitizer::default();
        let error_msg = "Validation failed: invalid email format";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert_eq!(sanitized.message, "Input validation failed");
        assert_eq!(sanitized.error_code, Some("VALIDATION_ERROR".to_string()));
    }

    #[test]
    fn test_sanitize_generic_error() {
        let sanitizer = ErrorSanitizer::default();
        let error_msg = "Something went wrong in function xyz::process() at line 123";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert!(!sanitized.message.contains("xyz::process"));
        assert!(!sanitized.message.contains("line 123"));
    }

    #[test]
    fn test_sanitize_stack_trace() {
        let sanitizer = ErrorSanitizer::default();
        let error_msg = r#"Error: Something failed
    at process (/path/to/file.js:123:45)
    at main (/path/to/main.js:567:89)"#;
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert!(!sanitized.message.contains("at process"));
        assert!(!sanitized.message.contains("/path/to/"));
    }

    #[test]
    fn test_custom_mappings() {
        let mut config = ErrorSanitizationConfig::default();
        config.custom_mappings.insert(
            "special error".to_string(),
            "User-friendly message".to_string(),
        );
        
        let sanitizer = ErrorSanitizer::new(config);
        let error_msg = "This is a special error that occurred";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert_eq!(sanitized.message, "User-friendly message");
        assert_eq!(sanitized.error_code, Some("CUSTOM_ERROR".to_string()));
    }

    #[test]
    fn test_message_length_limit() {
        let mut config = ErrorSanitizationConfig::default();
        config.max_message_length = 20;
        
        let sanitizer = ErrorSanitizer::new(config);
        let error_msg = "This is a very long error message that should be truncated";
        
        let sanitized = sanitizer.sanitize_message(error_msg);
        assert!(sanitized.message.len() <= 20);
        assert!(sanitized.message.ends_with("..."));
    }

    #[test]
    fn test_convenience_functions() {
        let validation_error = ErrorSanitizer::validation_error("email");
        assert_eq!(validation_error.message, "Input validation failed");
        assert_eq!(validation_error.error_code, Some("VALIDATION_ERROR".to_string()));

        let not_found = ErrorSanitizer::not_found_error("task");
        assert_eq!(not_found.message, "task not found");
        assert_eq!(not_found.error_code, Some("NOT_FOUND".to_string()));

        let permission = ErrorSanitizer::permission_error();
        assert_eq!(permission.message, "Permission denied");
        assert_eq!(permission.error_code, Some("PERMISSION_DENIED".to_string()));

        let internal = ErrorSanitizer::internal_error();
        assert_eq!(internal.message, "Internal server error");
        assert_eq!(internal.error_code, Some("INTERNAL_ERROR".to_string()));
    }
}