use chrono::{DateTime, Utc};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Safe database error that doesn't expose internal details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeDatabaseError {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    #[cfg(debug_assertions)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_info: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    #[serde(rename = "NOT_FOUND")]
    NotFound,
    #[serde(rename = "CONFLICT")]
    Conflict,
    #[serde(rename = "SERVICE_UNAVAILABLE")]
    ServiceUnavailable,
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError,
    #[serde(rename = "VALIDATION_ERROR")]
    ValidationError,
    #[serde(rename = "TIMEOUT")]
    Timeout,
}

impl ErrorCode {
    pub fn to_http_status(&self) -> u16 {
        match self {
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::ServiceUnavailable => 503,
            ErrorCode::InternalError => 500,
            ErrorCode::ValidationError => 400,
            ErrorCode::Timeout => 408,
        }
    }
}

impl SafeDatabaseError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            #[cfg(debug_assertions)]
            debug_info: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = request_id;
        self
    }

    #[cfg(debug_assertions)]
    pub fn with_debug_info(mut self, debug_info: String) -> Self {
        self.debug_info = Some(debug_info);
        self
    }
}

impl fmt::Display for SafeDatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.to_http_status(), self.message)
    }
}

impl std::error::Error for SafeDatabaseError {}

impl From<DbErr> for SafeDatabaseError {
    fn from(err: DbErr) -> Self {
        let (code, safe_message) = match &err {
            DbErr::ConnectionAcquire(_) => (
                ErrorCode::ServiceUnavailable,
                "Database connection unavailable",
            ),
            DbErr::RecordNotFound(_) => (ErrorCode::NotFound, "Requested resource not found"),
            DbErr::RecordNotInserted => (ErrorCode::InternalError, "Failed to create resource"),
            DbErr::RecordNotUpdated => {
                (ErrorCode::NotFound, "Resource not found or no changes made")
            }
            DbErr::Custom(msg) if msg.contains("UNIQUE constraint") => {
                (ErrorCode::Conflict, "Resource already exists")
            }
            DbErr::Custom(msg) if msg.contains("FOREIGN KEY constraint") => (
                ErrorCode::ValidationError,
                "Invalid reference to related resource",
            ),
            DbErr::Custom(msg) if msg.contains("timeout") => {
                (ErrorCode::Timeout, "Database operation timed out")
            }
            DbErr::Exec(_) => (ErrorCode::InternalError, "Database operation failed"),
            DbErr::Query(_) => (ErrorCode::ValidationError, "Invalid query parameters"),
            _ => (
                ErrorCode::InternalError,
                "An unexpected database error occurred",
            ),
        };

        // Log the full error internally with structured logging
        tracing::error!(
            error = %err,
            error_type = ?std::mem::discriminant(&err),
            safe_message = safe_message,
            "Database error occurred"
        );

        let mut safe_error = SafeDatabaseError::new(code, safe_message);

        #[cfg(debug_assertions)]
        {
            safe_error = safe_error.with_debug_info(err.to_string());
        }

        safe_error
    }
}

impl From<crate::database::DatabaseError> for SafeDatabaseError {
    fn from(err: crate::database::DatabaseError) -> Self {
        match err {
            crate::database::DatabaseError::DbError(db_err) => SafeDatabaseError::from(db_err),
            crate::database::DatabaseError::SerializationError(ser_err) => {
                tracing::error!(error = %ser_err, "Serialization error");
                SafeDatabaseError::new(ErrorCode::InternalError, "Data processing error")
            }
            crate::database::DatabaseError::MigrationError(msg) => {
                tracing::error!(error = %msg, "Migration error");
                SafeDatabaseError::new(
                    ErrorCode::ServiceUnavailable,
                    "Database migration in progress",
                )
            }
            crate::database::DatabaseError::ConfigError(msg) => {
                tracing::error!(error = %msg, "Database configuration error");
                SafeDatabaseError::new(
                    ErrorCode::ServiceUnavailable,
                    "Database configuration error",
                )
            }
            crate::database::DatabaseError::ValidationError(validation_err) => {
                SafeDatabaseError::from(validation_err)
            }
        }
    }
}

impl From<crate::database::filters::validation::ValidationError> for SafeDatabaseError {
    fn from(err: crate::database::filters::validation::ValidationError) -> Self {
        // Log security-related validation errors
        tracing::warn!(
            error = %err,
            "Input validation failed"
        );

        SafeDatabaseError::new(ErrorCode::ValidationError, "Invalid input parameters")
    }
}

/// Result type using SafeDatabaseError
pub type SafeDatabaseResult<T> = Result<T, SafeDatabaseError>;

/// Extension trait for converting database results to safe results
pub trait ToSafeResult<T> {
    fn to_safe_result(self) -> SafeDatabaseResult<T>;
    fn to_safe_result_with_request_id(self, request_id: String) -> SafeDatabaseResult<T>;
}

impl<T> ToSafeResult<T> for Result<T, crate::database::DatabaseError> {
    fn to_safe_result(self) -> SafeDatabaseResult<T> {
        self.map_err(|e| match e {
            crate::database::DatabaseError::DbError(db_err) => SafeDatabaseError::from(db_err),
            crate::database::DatabaseError::SerializationError(ser_err) => {
                tracing::error!(error = %ser_err, "Serialization error");
                SafeDatabaseError::new(ErrorCode::InternalError, "Data processing error")
            }
            crate::database::DatabaseError::MigrationError(msg) => {
                tracing::error!(error = %msg, "Migration error");
                SafeDatabaseError::new(
                    ErrorCode::ServiceUnavailable,
                    "Database migration in progress",
                )
            }
            crate::database::DatabaseError::ConfigError(msg) => {
                tracing::error!(error = %msg, "Database configuration error");
                SafeDatabaseError::new(
                    ErrorCode::ServiceUnavailable,
                    "Database configuration error",
                )
            }
            crate::database::DatabaseError::ValidationError(validation_err) => {
                SafeDatabaseError::from(validation_err)
            }
        })
    }

    fn to_safe_result_with_request_id(self, request_id: String) -> SafeDatabaseResult<T> {
        self.to_safe_result()
            .map_err(|e| e.with_request_id(request_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_http_status() {
        assert_eq!(ErrorCode::NotFound.to_http_status(), 404);
        assert_eq!(ErrorCode::Conflict.to_http_status(), 409);
        assert_eq!(ErrorCode::ServiceUnavailable.to_http_status(), 503);
        assert_eq!(ErrorCode::InternalError.to_http_status(), 500);
        assert_eq!(ErrorCode::ValidationError.to_http_status(), 400);
        assert_eq!(ErrorCode::Timeout.to_http_status(), 408);
    }

    #[test]
    fn test_safe_database_error_creation() {
        let error = SafeDatabaseError::new(ErrorCode::NotFound, "Test message");
        assert_eq!(error.code.to_http_status(), 404);
        assert_eq!(error.message, "Test message");
        assert!(!error.request_id.is_empty());
    }

    #[test]
    fn test_db_err_conversion() {
        let db_err = DbErr::RecordNotFound("test".to_string());
        let safe_err = SafeDatabaseError::from(db_err);

        assert_eq!(safe_err.code.to_http_status(), 404);
        assert_eq!(safe_err.message, "Requested resource not found");
    }
}
