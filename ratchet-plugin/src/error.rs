//! Plugin system error types

use thiserror::Error;

/// Plugin system result type
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin system errors
#[derive(Error, Debug)]
pub enum PluginError {
    /// Plugin not found
    #[error("Plugin '{name}' not found")]
    PluginNotFound {
        name: String,
    },

    /// Plugin already exists
    #[error("Plugin '{name}' already exists")]
    PluginAlreadyExists {
        name: String,
    },

    /// Plugin initialization failed
    #[error("Plugin '{name}' initialization failed: {reason}")]
    InitializationFailed {
        name: String,
        reason: String,
    },

    /// Plugin dependency error
    #[error("Plugin '{name}' dependency error: {reason}")]
    DependencyError {
        name: String,
        reason: String,
    },

    /// Plugin version incompatible
    #[error("Plugin '{name}' version {version} is incompatible with required version {required}")]
    VersionIncompatible {
        name: String,
        version: String,
        required: String,
    },

    /// Plugin API version incompatible
    #[error("Plugin '{name}' API version {api_version} is incompatible with system version {system_version}")]
    ApiVersionIncompatible {
        name: String,
        api_version: String,
        system_version: String,
    },

    /// Hook execution failed
    #[error("Hook '{hook_name}' execution failed: {reason}")]
    HookExecutionFailed {
        hook_name: String,
        reason: String,
    },

    /// Dynamic loading error
    #[error("Dynamic loading error: {0}")]
    DynamicLoadingError(#[from] libloading::Error),

    /// Plugin file not found
    #[error("Plugin file not found: {path}")]
    PluginFileNotFound {
        path: String,
    },

    /// Invalid plugin manifest
    #[error("Invalid plugin manifest: {reason}")]
    InvalidManifest {
        reason: String,
    },

    /// Plugin execution error
    #[error("Plugin '{name}' execution error: {reason}")]
    ExecutionError {
        name: String,
        reason: String,
    },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ratchet_config::ConfigError),

    /// Generic error
    #[error("Plugin system error: {0}")]
    Generic(String),
}

impl PluginError {
    /// Create a new generic plugin error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic(message.into())
    }

    /// Create a new initialization failed error
    pub fn initialization_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InitializationFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Create a new execution error
    pub fn execution_error(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ExecutionError {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Create a new hook execution failed error
    pub fn hook_execution_failed(hook_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HookExecutionFailed {
            hook_name: hook_name.into(),
            reason: reason.into(),
        }
    }
}