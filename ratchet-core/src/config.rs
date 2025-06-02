//! Configuration types - simplified for now

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExecutionConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HttpConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StorageConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggingConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OutputConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ServerConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PluginConfig {}