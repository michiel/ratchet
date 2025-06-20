//! Storage error types

use thiserror::Error;

/// Result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Storage-related errors
#[derive(Debug, Error)]
pub enum StorageError {
    /// Connection-related errors
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    /// Query execution errors
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Transaction errors
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Migration errors
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Entity not found
    #[error("Entity not found")]
    NotFound,

    /// Duplicate key constraint violation
    #[error("Duplicate key: {0}")]
    DuplicateKey(String),

    /// Validation errors
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Concurrency/locking errors
    #[error("Concurrency error: {0}")]
    ConcurrencyError(String),

    /// Generic storage errors
    #[error("Storage error: {0}")]
    Other(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Clone for StorageError {
    fn clone(&self) -> Self {
        match self {
            StorageError::ConnectionFailed(msg) => StorageError::ConnectionFailed(msg.clone()),
            StorageError::QueryFailed(msg) => StorageError::QueryFailed(msg.clone()),
            StorageError::TransactionFailed(msg) => StorageError::TransactionFailed(msg.clone()),
            StorageError::MigrationFailed(msg) => StorageError::MigrationFailed(msg.clone()),
            StorageError::NotFound => StorageError::NotFound,
            StorageError::DuplicateKey(msg) => StorageError::DuplicateKey(msg.clone()),
            StorageError::ValidationFailed(msg) => StorageError::ValidationFailed(msg.clone()),
            StorageError::ConfigError(msg) => StorageError::ConfigError(msg.clone()),
            StorageError::SerializationError(msg) => StorageError::SerializationError(msg.clone()),
            StorageError::ConcurrencyError(msg) => StorageError::ConcurrencyError(msg.clone()),
            StorageError::Other(msg) => StorageError::Other(msg.clone()),
            // For non-cloneable error types, convert to string representation
            StorageError::Io(err) => StorageError::Other(format!("I/O error: {}", err)),
            StorageError::Json(err) => StorageError::Other(format!("JSON error: {}", err)),
        }
    }
}

impl StorageError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            StorageError::ConnectionFailed(_) | StorageError::TransactionFailed(_) | StorageError::ConcurrencyError(_)
        )
    }

    /// Get a user-friendly error message that doesn't expose internal details
    pub fn user_message(&self) -> &'static str {
        match self {
            StorageError::ConnectionFailed(_) => "Database connection unavailable",
            StorageError::QueryFailed(_) => "Database operation failed",
            StorageError::TransactionFailed(_) => "Transaction could not be completed",
            StorageError::NotFound => "Requested item not found",
            StorageError::DuplicateKey(_) => "Item already exists",
            StorageError::ValidationFailed(_) => "Invalid data provided",
            StorageError::ConfigError(_) => "Configuration error",
            StorageError::ConcurrencyError(_) => "Operation conflict, please retry",
            _ => "An error occurred",
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            StorageError::NotFound => "NOT_FOUND",
            StorageError::DuplicateKey(_) => "DUPLICATE_KEY",
            StorageError::ValidationFailed(_) => "VALIDATION_ERROR",
            StorageError::ConfigError(_) => "CONFIG_ERROR",
            StorageError::ConnectionFailed(_) => "CONNECTION_ERROR",
            StorageError::QueryFailed(_) => "QUERY_ERROR",
            StorageError::TransactionFailed(_) => "TRANSACTION_ERROR",
            StorageError::ConcurrencyError(_) => "CONCURRENCY_ERROR",
            _ => "STORAGE_ERROR",
        }
    }
}

/// Convert from ratchet-core errors
impl From<ratchet_core::RatchetError> for StorageError {
    fn from(err: ratchet_core::RatchetError) -> Self {
        match err {
            ratchet_core::RatchetError::Storage(storage_err) => match storage_err {
                ratchet_core::error::StorageError::ConnectionFailed(msg) => StorageError::ConnectionFailed(msg),
                ratchet_core::error::StorageError::QueryFailed(msg) => StorageError::QueryFailed(msg),
                ratchet_core::error::StorageError::TransactionFailed(msg) => StorageError::TransactionFailed(msg),
                ratchet_core::error::StorageError::MigrationFailed(msg) => StorageError::MigrationFailed(msg),
                ratchet_core::error::StorageError::NotFound => StorageError::NotFound,
                ratchet_core::error::StorageError::DuplicateKey(msg) => StorageError::DuplicateKey(msg),
            },
            ratchet_core::RatchetError::Validation(validation_err) => {
                StorageError::ValidationFailed(validation_err.to_string())
            }
            ratchet_core::RatchetError::Config(config_err) => StorageError::ConfigError(config_err.to_string()),
            ratchet_core::RatchetError::Serialization(msg) => StorageError::SerializationError(msg),
            ratchet_core::RatchetError::Io(io_err) => StorageError::Io(io_err),
            _ => StorageError::Other(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        assert!(StorageError::ConnectionFailed("test".to_string()).is_retryable());
        assert!(StorageError::TransactionFailed("test".to_string()).is_retryable());
        assert!(!StorageError::NotFound.is_retryable());
        assert!(!StorageError::ValidationFailed("test".to_string()).is_retryable());
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(StorageError::NotFound.error_code(), "NOT_FOUND");
        assert_eq!(
            StorageError::DuplicateKey("key".to_string()).error_code(),
            "DUPLICATE_KEY"
        );
        assert_eq!(
            StorageError::ValidationFailed("msg".to_string()).error_code(),
            "VALIDATION_ERROR"
        );
    }

    #[test]
    fn test_user_messages() {
        assert_eq!(StorageError::NotFound.user_message(), "Requested item not found");
        assert_eq!(
            StorageError::ConnectionFailed("test".to_string()).user_message(),
            "Database connection unavailable"
        );
    }
}
