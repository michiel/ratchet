//! IPC error types

use thiserror::Error;
use crate::protocol::WorkerError;

/// IPC error types
#[derive(Debug, Error)]
pub enum IpcError {
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
    
    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,
    
    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch { expected: u32, actual: u32 },
    
    /// Timeout waiting for response
    #[error("Timeout waiting for response")]
    Timeout,
    
    /// Worker error
    #[error("Worker error: {0}")]
    WorkerError(WorkerError),
    
    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
    
    /// Transport not connected
    #[error("Transport not connected")]
    NotConnected,
}

impl IpcError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            IpcError::IoError(_) | IpcError::Timeout | IpcError::ConnectionClosed
        )
    }
    
    /// Check if this error indicates a fatal condition
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            IpcError::ProtocolVersionMismatch { .. } | IpcError::InvalidMessage(_)
        )
    }
}

impl From<std::io::Error> for IpcError {
    fn from(err: std::io::Error) -> Self {
        IpcError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_io() {
            IpcError::IoError(err.to_string())
        } else if err.is_data() {
            IpcError::DeserializationError(err.to_string())
        } else {
            IpcError::SerializationError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_retryable() {
        assert!(IpcError::IoError("network error".to_string()).is_retryable());
        assert!(IpcError::Timeout.is_retryable());
        assert!(IpcError::ConnectionClosed.is_retryable());
        assert!(!IpcError::ProtocolVersionMismatch { expected: 1, actual: 2 }.is_retryable());
        assert!(!IpcError::InvalidMessage("bad format".to_string()).is_retryable());
    }
    
    #[test]
    fn test_error_fatal() {
        assert!(IpcError::ProtocolVersionMismatch { expected: 1, actual: 2 }.is_fatal());
        assert!(IpcError::InvalidMessage("bad format".to_string()).is_fatal());
        assert!(!IpcError::IoError("network error".to_string()).is_fatal());
        assert!(!IpcError::Timeout.is_fatal());
    }
}