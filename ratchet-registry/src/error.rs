use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Load error: {0}")]
    LoadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("Watcher error: {0}")]
    WatcherError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] ratchet_http::errors::HttpError),

    #[error("Storage error: {0}")]
    Storage(#[from] ratchet_storage::error::StorageError),

    #[error("Core error: {0}")]
    Core(#[from] ratchet_core::error::RatchetError),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RegistryError>;