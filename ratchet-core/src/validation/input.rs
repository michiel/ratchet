//! Comprehensive input validation and sanitization

use regex::Regex;
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use url::Url;

/// Maximum input sizes to prevent resource exhaustion
pub const MAX_JSON_SIZE: usize = 10 * 1024 * 1024; // 10MB
pub const MAX_STRING_LENGTH: usize = 10000; // 10K characters
pub const MAX_ARRAY_LENGTH: usize = 1000; // 1K array elements
pub const MAX_OBJECT_DEPTH: usize = 20; // 20 levels deep
pub const MAX_KEYS_PER_OBJECT: usize = 100; // 100 keys per object

/// Input validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Input too large: {actual} bytes exceeds maximum {max} bytes")]
    SizeTooLarge { actual: usize, max: usize },

    #[error("String too long: {actual} characters exceeds maximum {max} characters")]
    StringTooLong { actual: usize, max: usize },

    #[error("Array too large: {actual} elements exceeds maximum {max} elements")]
    ArrayTooLarge { actual: usize, max: usize },

    #[error("Object nesting too deep: {actual} levels exceeds maximum {max} levels")]
    NestingTooDeep { actual: usize, max: usize },

    #[error("Too many object keys: {actual} keys exceeds maximum {max} keys")]
    TooManyKeys { actual: usize, max: usize },

    #[error("Required field missing: {field}")]
    RequiredField { field: String },

    #[error("Invalid format for field {field}: {reason}")]
    InvalidFormat { field: String, reason: String },

    #[error("Invalid value for field {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Unsafe path detected: {path}")]
    UnsafePath { path: String },

    #[error("Invalid URL: {reason}")]
    InvalidUrl { reason: String },

    #[error("Blocked URL: {url} (reason: {reason})")]
    BlockedUrl { url: String, reason: String },

    #[error("Invalid JSON: {reason}")]
    InvalidJson { reason: String },

    #[error("Potential injection attack detected")]
    PotentialInjection,

    #[error("Invalid character encoding")]
    InvalidEncoding,
}

/// Comprehensive input validator
#[derive(Debug, Clone)]
pub struct InputValidator {
    max_json_size: usize,
    max_string_length: usize,
    max_array_length: usize,
    max_object_depth: usize,
    max_keys_per_object: usize,
    allow_unicode: bool,
    blocked_domains: Vec<String>,
    blocked_ips: Vec<IpAddr>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self {
            max_json_size: MAX_JSON_SIZE,
            max_string_length: MAX_STRING_LENGTH,
            max_array_length: MAX_ARRAY_LENGTH,
            max_object_depth: MAX_OBJECT_DEPTH,
            max_keys_per_object: MAX_KEYS_PER_OBJECT,
            allow_unicode: true,
            blocked_domains: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "0.0.0.0".to_string(),
                "169.254.169.254".to_string(),          // AWS metadata
                "metadata.google.internal".to_string(), // GCP metadata
                "169.254.0.1".to_string(),              // Azure metadata
            ],
            blocked_ips: vec![
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            ],
        }
    }
}

impl InputValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(max_json_size: usize, max_string_length: usize, max_array_length: usize) -> Self {
        Self {
            max_json_size,
            max_string_length,
            max_array_length,
            ..Default::default()
        }
    }

    /// Validate and sanitize JSON input
    pub fn validate_json(&self, input: &str) -> Result<Value, ValidationError> {
        // Check size limits
        if input.len() > self.max_json_size {
            return Err(ValidationError::SizeTooLarge {
                actual: input.len(),
                max: self.max_json_size,
            });
        }

        // Check for potential injection attacks
        self.check_injection_patterns(input)?;

        // Parse JSON
        let value: Value = serde_json::from_str(input).map_err(|_e| ValidationError::InvalidJson {
            reason: "Invalid JSON syntax".to_string(),
        })?;

        // Validate structure
        self.validate_json_value(&value, 0)?;

        Ok(value)
    }

    /// Validate JSON value recursively
    fn validate_json_value(&self, value: &Value, depth: usize) -> Result<(), ValidationError> {
        // Check nesting depth
        if depth > self.max_object_depth {
            return Err(ValidationError::NestingTooDeep {
                actual: depth,
                max: self.max_object_depth,
            });
        }

        match value {
            Value::String(s) => self.validate_string(s, "value")?,
            Value::Array(arr) => {
                if arr.len() > self.max_array_length {
                    return Err(ValidationError::ArrayTooLarge {
                        actual: arr.len(),
                        max: self.max_array_length,
                    });
                }
                for item in arr {
                    self.validate_json_value(item, depth + 1)?;
                }
            }
            Value::Object(obj) => {
                if obj.len() > self.max_keys_per_object {
                    return Err(ValidationError::TooManyKeys {
                        actual: obj.len(),
                        max: self.max_keys_per_object,
                    });
                }
                for (key, val) in obj {
                    self.validate_string(key, "key")?;
                    self.validate_json_value(val, depth + 1)?;
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }

        Ok(())
    }

    /// Validate string input with field-specific rules
    pub fn validate_string(&self, input: &str, field_name: &str) -> Result<(), ValidationError> {
        // Check length
        if input.len() > self.max_string_length {
            return Err(ValidationError::StringTooLong {
                actual: input.len(),
                max: self.max_string_length,
            });
        }

        // Check for null bytes
        if input.contains('\0') {
            return Err(ValidationError::InvalidFormat {
                field: field_name.to_string(),
                reason: "Null bytes not allowed".to_string(),
            });
        }

        // Apply field-specific validation
        match field_name {
            "cron_expression" | "cronExpression" => {
                // Validate cron expression using proper cron parsing
                if input.trim().is_empty() {
                    return Err(ValidationError::InvalidFormat {
                        field: "cron_expression".to_string(),
                        reason: "Cron expression cannot be empty".to_string(),
                    });
                }

                // Use the cron library for proper validation
                use std::str::FromStr;

                // Try to parse with cron library (same one used in storage)
                match cron::Schedule::from_str(input) {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        return Err(ValidationError::InvalidFormat {
                            field: "cron_expression".to_string(),
                            reason: format!("Invalid cron expression: {}", e),
                        })
                    }
                }
            }
            "email" | "email_address" | "emailAddress" => {
                // Use existing email validation but with field-specific error
                return self.validate_email(input).map_err(|e| match e {
                    ValidationError::InvalidFormat { reason, .. } => ValidationError::InvalidFormat {
                        field: "email".to_string(),
                        reason,
                    },
                    other => other,
                });
            }
            _ => {
                // Apply standard validation for other fields
            }
        }

        // Check for control characters (except common whitespace)
        for ch in input.chars() {
            if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t' | ' ') {
                return Err(ValidationError::InvalidFormat {
                    field: field_name.to_string(),
                    reason: "Control characters not allowed".to_string(),
                });
            }
        }

        // Check for potential injection patterns
        self.check_injection_patterns(input)?;

        Ok(())
    }

    /// Validate required string field
    pub fn validate_required_string(&self, input: Option<&str>, field_name: &str) -> Result<String, ValidationError> {
        let value = input.ok_or_else(|| ValidationError::RequiredField {
            field: field_name.to_string(),
        })?;

        if value.trim().is_empty() {
            return Err(ValidationError::RequiredField {
                field: field_name.to_string(),
            });
        }

        self.validate_string(value, field_name)?;
        Ok(value.trim().to_string())
    }

    /// Validate task name
    pub fn validate_task_name(&self, name: &str) -> Result<(), ValidationError> {
        let name_regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();

        if name.is_empty() {
            return Err(ValidationError::RequiredField {
                field: "name".to_string(),
            });
        }

        if name.len() > 100 {
            return Err(ValidationError::StringTooLong {
                actual: name.len(),
                max: 100,
            });
        }

        if !name_regex.is_match(name) {
            return Err(ValidationError::InvalidFormat {
                field: "name".to_string(),
                reason: "Name must contain only alphanumeric characters, hyphens, and underscores".to_string(),
            });
        }

        Ok(())
    }

    /// Validate semantic version
    pub fn validate_semver(&self, version: &str) -> Result<(), ValidationError> {
        let semver_regex = Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*|[0-9a-zA-Z-]*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*|[0-9a-zA-Z-]*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();

        if !semver_regex.is_match(version) {
            return Err(ValidationError::InvalidFormat {
                field: "version".to_string(),
                reason: "Must be a valid semantic version (e.g., 1.0.0)".to_string(),
            });
        }

        Ok(())
    }

    /// Validate file path for safety
    pub fn validate_safe_path(&self, path: &str) -> Result<(), ValidationError> {
        // Check for directory traversal attempts
        if path.contains("..") {
            return Err(ValidationError::UnsafePath { path: path.to_string() });
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err(ValidationError::UnsafePath { path: path.to_string() });
        }

        // Check for absolute paths (may be dangerous depending on context)
        if Path::new(path).is_absolute() && !path.starts_with("/workspace") {
            return Err(ValidationError::UnsafePath { path: path.to_string() });
        }

        // Check for Windows reserved names
        let path_lower = path.to_lowercase();
        let reserved_names = [
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1",
            "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];

        for name in &reserved_names {
            if path_lower == *name || path_lower.starts_with(&format!("{}.", name)) {
                return Err(ValidationError::UnsafePath { path: path.to_string() });
            }
        }

        Ok(())
    }

    /// Validate URL and check for SSRF risks
    pub fn validate_url(&self, url: &str) -> Result<Url, ValidationError> {
        // Parse URL
        let parsed_url = Url::parse(url).map_err(|e| ValidationError::InvalidUrl { reason: e.to_string() })?;

        // Only allow HTTP/HTTPS
        match parsed_url.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(ValidationError::BlockedUrl {
                    url: url.to_string(),
                    reason: format!("Scheme '{}' not allowed", scheme),
                });
            }
        }

        // Check for blocked domains
        if let Some(host) = parsed_url.host_str() {
            for blocked_domain in &self.blocked_domains {
                if host == blocked_domain || host.ends_with(&format!(".{}", blocked_domain)) {
                    return Err(ValidationError::BlockedUrl {
                        url: url.to_string(),
                        reason: format!("Domain '{}' is blocked", host),
                    });
                }
            }

            // Check for IP addresses
            if let Ok(ip) = host.parse::<IpAddr>() {
                // Block private/local IPs
                if self.is_blocked_ip(&ip) {
                    return Err(ValidationError::BlockedUrl {
                        url: url.to_string(),
                        reason: "Private/local IP addresses are blocked".to_string(),
                    });
                }
            }
        }

        Ok(parsed_url)
    }

    /// Check for injection attack patterns
    fn check_injection_patterns(&self, input: &str) -> Result<(), ValidationError> {
        let suspicious_patterns = [
            // SQL injection patterns
            r"(?i)\b(union|select|insert|update|delete|drop|create|alter|exec|execute)\b",
            r"(?i)(\-\-|\#|\/\*|\*\/)",
            r"(?i)\b(or|and)\s+\w+\s*=\s*\w+",
            // XSS patterns
            r"(?i)<script[^>]*>",
            r"(?i)javascript:",
            r"(?i)on\w+\s*=",
            r"(?i)<iframe[^>]*>",
            // Command injection patterns
            r"(?i)(\||;|&|`|\$\(|\$\{)",
            r"(?i)\b(rm|cat|ls|ps|kill|sudo|su)\b",
            // Path traversal patterns
            r"\.\./",
            r"\.\.\\",
        ];

        for pattern in &suspicious_patterns {
            let regex = Regex::new(pattern).unwrap();
            if regex.is_match(input) {
                return Err(ValidationError::PotentialInjection);
            }
        }

        Ok(())
    }

    /// Check if input looks like a cron expression
    fn is_likely_cron_expression(&self, input: &str) -> bool {
        let trimmed = input.trim();

        // Basic cron expression patterns - 5 or 6 space-separated fields
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return false;
        }

        // Check if all parts match cron field patterns
        for part in &parts {
            if !self.is_valid_cron_field(part) {
                return false;
            }
        }

        true
    }

    /// Check if a single field matches cron syntax
    fn is_valid_cron_field(&self, field: &str) -> bool {
        // Cron field can contain: numbers, *, /, -, ,
        // Examples: *, */5, 1-10, 1,3,5, 15, */2
        let cron_field_regex = regex::Regex::new(r"^[0-9*,/\-]+$").unwrap();
        cron_field_regex.is_match(field)
    }

    /// Check if IP address is blocked
    fn is_blocked_ip(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                // Check localhost
                if ipv4.is_loopback() {
                    return true;
                }

                // Check private networks
                if ipv4.is_private() {
                    return true;
                }

                // Check link-local
                if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
                    return true;
                }

                // Check multicast
                if ipv4.is_multicast() {
                    return true;
                }
            }
            IpAddr::V6(ipv6) => {
                // Check localhost
                if ipv6.is_loopback() {
                    return true;
                }

                // Check link-local
                if (ipv6.segments()[0] & 0xffc0) == 0xfe80 {
                    return true;
                }

                // Check multicast
                if ipv6.is_multicast() {
                    return true;
                }
            }
        }

        // Check explicit blocked IPs
        self.blocked_ips.contains(ip)
    }

    /// Validate email format
    pub fn validate_email(&self, email: &str) -> Result<(), ValidationError> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

        if !email_regex.is_match(email) {
            return Err(ValidationError::InvalidFormat {
                field: "email".to_string(),
                reason: "Invalid email format".to_string(),
            });
        }

        Ok(())
    }

    /// Sanitize HTML/text content
    pub fn sanitize_text(&self, input: &str) -> String {
        // Remove or escape dangerous characters
        // Note: Replace & first to avoid double-encoding
        input
            .chars()
            .filter(|&c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
            .collect::<String>()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Validate and sanitize task input for JavaScript execution
    pub fn validate_task_input(&self, input: &Value) -> Result<Value, ValidationError> {
        // First validate the JSON structure
        self.validate_json_value(input, 0)?;

        // Additional validation for task input
        match input {
            Value::Object(obj) => {
                let mut sanitized = serde_json::Map::new();
                for (key, value) in obj {
                    // Validate key
                    self.validate_string(key, "input_key")?;

                    // Recursively validate and sanitize value
                    let sanitized_value = self.validate_task_input(value)?;
                    sanitized.insert(key.clone(), sanitized_value);
                }
                Ok(Value::Object(sanitized))
            }
            Value::Array(arr) => {
                let mut sanitized = Vec::new();
                for item in arr {
                    sanitized.push(self.validate_task_input(item)?);
                }
                Ok(Value::Array(sanitized))
            }
            Value::String(s) => {
                self.validate_string(s, "input_value")?;
                Ok(Value::String(self.sanitize_text(s)))
            }
            _ => Ok(input.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_json_size_limit() {
        let validator = InputValidator::with_limits(100, 50, 10);
        let large_input = "x".repeat(101);
        let result = validator.validate_json(&large_input);
        assert!(matches!(result, Err(ValidationError::SizeTooLarge { .. })));
    }

    #[test]
    fn test_validate_string_injection() {
        let validator = InputValidator::new();
        let injection_attempt = "'; DROP TABLE users; --";
        let result = validator.validate_string(injection_attempt, "test");
        assert!(matches!(result, Err(ValidationError::PotentialInjection)));
    }

    #[test]
    fn test_validate_task_name() {
        let validator = InputValidator::new();

        // Valid names
        assert!(validator.validate_task_name("my-task").is_ok());
        assert!(validator.validate_task_name("task_123").is_ok());

        // Invalid names
        assert!(validator.validate_task_name("").is_err());
        assert!(validator.validate_task_name("task with spaces").is_err());
        assert!(validator.validate_task_name("task!@#").is_err());
    }

    #[test]
    fn test_validate_semver() {
        let validator = InputValidator::new();

        // Valid versions
        assert!(validator.validate_semver("1.0.0").is_ok());
        assert!(validator.validate_semver("2.1.3-alpha.1").is_ok());

        // Invalid versions
        assert!(validator.validate_semver("1.0").is_err());
        assert!(validator.validate_semver("v1.0.0").is_err());
        assert!(validator.validate_semver("1.0.0.0").is_err());
    }

    #[test]
    fn test_validate_safe_path() {
        let validator = InputValidator::new();

        // Safe paths
        assert!(validator.validate_safe_path("tasks/my-task.js").is_ok());
        assert!(validator.validate_safe_path("/workspace/tasks/task.js").is_ok());

        // Unsafe paths
        assert!(validator.validate_safe_path("../../../etc/passwd").is_err());
        assert!(validator.validate_safe_path("tasks/file\0.js").is_err());
        assert!(validator.validate_safe_path("CON").is_err());
    }

    #[test]
    fn test_validate_url_ssrf() {
        let validator = InputValidator::new();

        // Safe URLs
        assert!(validator.validate_url("https://api.example.com/data").is_ok());

        // Unsafe URLs
        assert!(validator.validate_url("http://localhost:8080/admin").is_err());
        assert!(validator.validate_url("http://169.254.169.254/metadata").is_err());
        assert!(validator.validate_url("file:///etc/passwd").is_err());
        assert!(validator.validate_url("ftp://example.com/file").is_err());
    }

    #[test]
    fn test_validate_json_nesting() {
        let validator = InputValidator::with_limits(1000, 100, 10);

        // Create deeply nested JSON
        let mut deep_json = json!("value");
        for _ in 0..25 {
            deep_json = json!({ "nested": deep_json });
        }

        let result = validator.validate_json_value(&deep_json, 0);
        assert!(matches!(result, Err(ValidationError::NestingTooDeep { .. })));
    }

    #[test]
    fn test_sanitize_text() {
        let validator = InputValidator::new();

        let input = "<script>alert('xss')</script>";
        let sanitized = validator.sanitize_text(input);
        assert_eq!(sanitized, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
    }
}
