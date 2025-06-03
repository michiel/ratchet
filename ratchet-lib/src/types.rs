use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// HTTP methods supported by the Ratchet HTTP client
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    /// Get the string representation of the HTTP method
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST", 
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    /// Get all supported HTTP methods
    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
            HttpMethod::Head,
            HttpMethod::Options,
        ]
    }
}


impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HttpMethod {
    type Err = HttpMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "PATCH" => Ok(HttpMethod::Patch),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            _ => Err(HttpMethodError::InvalidMethod(s.to_string())),
        }
    }
}

impl From<HttpMethod> for reqwest::Method {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        }
    }
}

/// Log levels supported by the Ratchet framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Get the string representation of the log level
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    /// Get all supported log levels
    pub fn all() -> &'static [LogLevel] {
        &[
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ]
    }
}


impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = LogLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(LogLevelError::InvalidLevel(s.to_string())),
        }
    }
}

/// Task execution states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TaskStatus {
    /// Task is ready to be executed
    #[default]
    Pending,
    /// Task is currently being executed
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled before completion
    Cancelled,
    /// Task is being validated
    Validating,
    /// Task is loading content/dependencies
    Loading,
}

impl TaskStatus {
    /// Get the string representation of the task status
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Validating => "validating",
            TaskStatus::Loading => "loading",
        }
    }

    /// Check if the task is in a terminal state (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled)
    }

    /// Check if the task is in an active state (running, validating, or loading)
    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::Running | TaskStatus::Validating | TaskStatus::Loading)
    }

    /// Get all supported task statuses
    pub fn all() -> &'static [TaskStatus] {
        &[
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
            TaskStatus::Validating,
            TaskStatus::Loading,
        ]
    }
}


impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = TaskStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            "validating" => Ok(TaskStatus::Validating),
            "loading" => Ok(TaskStatus::Loading),
            _ => Err(TaskStatusError::InvalidStatus(s.to_string())),
        }
    }
}

/// Errors that can occur when parsing HTTP methods
#[derive(Error, Debug, Clone)]
pub enum HttpMethodError {
    #[error("Invalid HTTP method: '{0}'. Supported methods are: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS")]
    InvalidMethod(String),
}

/// Errors that can occur when parsing log levels
#[derive(Error, Debug, Clone)]
pub enum LogLevelError {
    #[error("Invalid log level: '{0}'. Supported levels are: trace, debug, info, warn, error")]
    InvalidLevel(String),
}

/// Errors that can occur when parsing task statuses
#[derive(Error, Debug, Clone)]
pub enum TaskStatusError {
    #[error("Invalid task status: '{0}'. Supported statuses are: pending, running, completed, failed, cancelled, validating, loading")]
    InvalidStatus(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_from_str() {
        assert_eq!("GET".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
        assert_eq!("post".parse::<HttpMethod>().unwrap(), HttpMethod::Post);
        assert_eq!("PUT".parse::<HttpMethod>().unwrap(), HttpMethod::Put);
        assert_eq!("delete".parse::<HttpMethod>().unwrap(), HttpMethod::Delete);
        assert_eq!("PATCH".parse::<HttpMethod>().unwrap(), HttpMethod::Patch);
        assert_eq!("head".parse::<HttpMethod>().unwrap(), HttpMethod::Head);
        assert_eq!("OPTIONS".parse::<HttpMethod>().unwrap(), HttpMethod::Options);
        
        assert!("INVALID".parse::<HttpMethod>().is_err());
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
        assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
        assert_eq!(HttpMethod::Head.as_str(), "HEAD");
        assert_eq!(HttpMethod::Options.as_str(), "OPTIONS");
    }

    #[test]
    fn test_http_method_to_reqwest() {
        assert_eq!(reqwest::Method::from(HttpMethod::Get), reqwest::Method::GET);
        assert_eq!(reqwest::Method::from(HttpMethod::Post), reqwest::Method::POST);
        assert_eq!(reqwest::Method::from(HttpMethod::Put), reqwest::Method::PUT);
        assert_eq!(reqwest::Method::from(HttpMethod::Delete), reqwest::Method::DELETE);
        assert_eq!(reqwest::Method::from(HttpMethod::Patch), reqwest::Method::PATCH);
        assert_eq!(reqwest::Method::from(HttpMethod::Head), reqwest::Method::HEAD);
        assert_eq!(reqwest::Method::from(HttpMethod::Options), reqwest::Method::OPTIONS);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("WARN".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn test_task_status_from_str() {
        assert_eq!("pending".parse::<TaskStatus>().unwrap(), TaskStatus::Pending);
        assert_eq!("RUNNING".parse::<TaskStatus>().unwrap(), TaskStatus::Running);
        assert_eq!("Completed".parse::<TaskStatus>().unwrap(), TaskStatus::Completed);
        assert_eq!("FAILED".parse::<TaskStatus>().unwrap(), TaskStatus::Failed);
        assert_eq!("cancelled".parse::<TaskStatus>().unwrap(), TaskStatus::Cancelled);
        assert_eq!("validating".parse::<TaskStatus>().unwrap(), TaskStatus::Validating);
        assert_eq!("loading".parse::<TaskStatus>().unwrap(), TaskStatus::Loading);
        
        assert!("invalid".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_task_status_state_checks() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());

        assert!(TaskStatus::Running.is_active());
        assert!(TaskStatus::Validating.is_active());
        assert!(TaskStatus::Loading.is_active());
        assert!(!TaskStatus::Pending.is_active());
        assert!(!TaskStatus::Completed.is_active());
    }

    #[test]
    fn test_defaults() {
        assert_eq!(HttpMethod::default(), HttpMethod::Get);
        assert_eq!(LogLevel::default(), LogLevel::Info);
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn test_serialization() {
        // Test JSON serialization/deserialization
        let method = HttpMethod::Post;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"POST\"");
        let parsed: HttpMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, method);

        let level = LogLevel::Debug;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"debug\"");
        let parsed: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, level);

        let status = TaskStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");
        let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", HttpMethod::Get), "GET");
        assert_eq!(format!("{}", LogLevel::Info), "info");
        assert_eq!(format!("{}", TaskStatus::Running), "running");
    }

    #[test]
    fn test_error_messages() {
        let err = "INVALID".parse::<HttpMethod>().unwrap_err();
        assert!(err.to_string().contains("INVALID"));
        assert!(err.to_string().contains("GET, POST, PUT"));

        let err = "INVALID".parse::<LogLevel>().unwrap_err();
        assert!(err.to_string().contains("INVALID"));
        assert!(err.to_string().contains("trace, debug, info"));

        let err = "INVALID".parse::<TaskStatus>().unwrap_err();
        assert!(err.to_string().contains("INVALID"));
        assert!(err.to_string().contains("pending, running, completed"));
    }
}