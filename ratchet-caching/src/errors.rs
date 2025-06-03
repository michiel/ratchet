//! Cache error types

use thiserror::Error;

/// Result type for cache operations
pub type CacheResult<T> = std::result::Result<T, CacheError>;

/// Cache-related errors
#[derive(Debug, Error)]
pub enum CacheError {
    /// Cache capacity exceeded
    #[error("Cache capacity exceeded: {0}")]
    CapacityExceeded(String),
    
    /// Key not found in cache
    #[error("Key not found in cache")]
    KeyNotFound,
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    /// Lock acquisition failed
    #[error("Failed to acquire cache lock: {0}")]
    LockError(String),
    
    /// TTL expired
    #[error("Cache entry expired")]
    Expired,
    
    /// Invalid configuration
    #[error("Invalid cache configuration: {0}")]
    InvalidConfiguration(String),
    
    /// Backend-specific error
    #[error("Cache backend error: {0}")]
    BackendError(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            CacheError::DeserializationError(err.to_string())
        } else {
            CacheError::SerializationError(err.to_string())
        }
    }
}