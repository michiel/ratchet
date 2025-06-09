# Ratchet-lib Circular Dependency Migration Plan

## Problem Analysis

**Primary Circular Dependency Chain**: `ratchet-lib` ↔ `ratchet-mcp` ↔ `ratchet-cli`

### Current Issues:
1. **ratchet-mcp** depends on `ratchet-lib` for service interfaces (`Service`, `ServiceHealth`, `ServiceMetrics`)
2. **ratchet-lib** depends on new modular crates (`ratchet-logging`, `ratchet-core`, `ratchet-http`)
3. **ratchet-cli** maintains dual systems with conversion code between legacy and new formats

### Impact:
- Prevents clean modular builds
- Creates maintenance burden with duplicate abstraction layers
- Runtime overhead from constant type conversions
- Complex conditional compilation with feature flags

## Migration Strategy: 4-Phase Plan

### Phase 1: Interface Extraction (Week 1)
**Goal**: Break the tightest circular dependency by extracting shared interfaces

#### 1.1 Create `ratchet-interfaces` Crate
```bash
mkdir ratchet-interfaces
```

**File**: `ratchet-interfaces/Cargo.toml`
```toml
[package]
name = "ratchet-interfaces"
version = "0.3.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
```

**File**: `ratchet-interfaces/src/lib.rs`
```rust
//! Core interfaces and traits for Ratchet modular architecture
//! 
//! This crate provides the fundamental interfaces that are shared across
//! the entire Ratchet ecosystem, breaking circular dependencies.

pub mod service;
pub mod execution;
pub mod logging;

// Re-export commonly used types
pub use service::{Service, ServiceHealth, ServiceMetrics, HealthStatus};
pub use execution::{TaskExecutor, ExecutionResult, ExecutionContext};
pub use logging::{LogEvent, LogLevel, StructuredLogger};
```

**File**: `ratchet-interfaces/src/service.rs`
```rust
//! Service interface definitions
//! Extracted from ratchet-lib/src/services/base.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base service trait that all services should implement
#[async_trait]
pub trait Service: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type Config: Send + Sync;

    /// Initialize the service with configuration
    async fn initialize(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Get the service name for logging and monitoring
    fn name(&self) -> &'static str;

    /// Perform health check
    async fn health_check(&self) -> Result<ServiceHealth, Self::Error>;

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<(), Self::Error>;

    /// Get service metrics
    fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }

    /// Get service configuration (optional)
    fn config(&self) -> Option<&Self::Config> {
        None
    }
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

/// Service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_checked: DateTime<Utc>,
    pub latency_ms: Option<u64>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ServiceHealth {
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
            last_checked: Utc::now(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn unhealthy(reason: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy {
                reason: reason.into(),
            },
            message: None,
            last_checked: Utc::now(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }
}

/// Service metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub requests_total: u64,
    pub requests_failed: u64,
    pub average_latency_ms: f64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: Option<u64>,
    pub custom_metrics: HashMap<String, f64>,
}
```

**File**: `ratchet-interfaces/src/execution.rs`
```rust
//! Execution interface definitions

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::time::Duration;

/// Core task execution interface
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute a task with given input
    async fn execute_task(
        &self,
        task_id: &str,
        input: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, Self::Error>;

    /// Check if executor is healthy
    async fn health_check(&self) -> Result<(), Self::Error>;
}

/// Execution context for task runs
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub timeout: Option<Duration>,
    pub trace_enabled: bool,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Task execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: JsonValue,
    pub execution_time_ms: u64,
    pub logs: Vec<String>,
    pub trace: Option<JsonValue>,
}
```

**File**: `ratchet-interfaces/src/logging.rs`
```rust
//! Logging interface definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Structured log event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: Option<String>,
    pub context: JsonValue,
}

/// Structured logger trait
pub trait StructuredLogger: Send + Sync {
    /// Log an event
    fn log(&self, event: LogEvent);
    
    /// Log with level and message
    fn log_simple(&self, level: LogLevel, message: String) {
        self.log(LogEvent {
            level,
            message,
            timestamp: Utc::now(),
            source: None,
            context: JsonValue::Null,
        });
    }
}
```

#### 1.2 Update Dependencies

**Add to workspace**: `Cargo.toml`
```toml
[workspace]
members = [
    "ratchet-cli",
    "ratchet-interfaces",  # New
    "ratchet-core",
    # ... existing members
]
```

**Update**: `ratchet-lib/Cargo.toml`
```toml
[dependencies]
# Add new interface dependency
ratchet-interfaces = { path = "../ratchet-interfaces" }
# Keep existing modular dependencies for now
ratchet-core = { path = "../ratchet-core" }
ratchet-http = { path = "../ratchet-http" }
ratchet-logging = { path = "../ratchet-logging" }
```

**Update**: `ratchet-mcp/Cargo.toml`
```toml
[dependencies]
# Replace ratchet-lib dependency with interfaces only
ratchet-interfaces = { path = "../ratchet-interfaces" }
# Remove: ratchet_lib = { path = "../ratchet-lib" }

# Add direct dependencies to modular crates
ratchet-execution = { path = "../ratchet-execution" }
ratchet-storage = { path = "../ratchet-storage" }
```

#### 1.3 Update Import Statements

**Update**: `ratchet-mcp/src/service.rs`
```rust
// Change from:
// use ratchet_lib::services::base::{Service, ServiceHealth, ServiceMetrics};

// To:
use ratchet_interfaces::{Service, ServiceHealth, ServiceMetrics, HealthStatus};
```

**Update**: `ratchet-mcp/src/server/adapter.rs`
```rust
// Change from:
// use ratchet_lib::logging::event::{LogEvent, LogLevel};

// To:
use ratchet_interfaces::logging::{LogEvent, LogLevel};
```

**Update**: `ratchet-lib/src/services/base.rs`
```rust
// Re-export from interfaces to maintain backward compatibility
pub use ratchet_interfaces::service::*;
```

### Phase 2: Execution Layer Migration (Week 2)
**Goal**: Move remaining execution functionality to modular crates

#### 2.1 Create Execution Bridge in `ratchet-execution`

**File**: `ratchet-execution/src/bridge.rs`
```rust
//! Bridge for converting between legacy and new execution systems

use ratchet_interfaces::{TaskExecutor, ExecutionResult, ExecutionContext};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Bridge that adapts ProcessTaskExecutor to legacy interfaces
pub struct ExecutionBridge {
    inner: crate::ProcessTaskExecutor,
}

impl ExecutionBridge {
    pub fn new(config: crate::ProcessExecutorConfig) -> Self {
        Self {
            inner: crate::ProcessTaskExecutor::new(config),
        }
    }

    /// Create from legacy ratchet-lib configuration
    pub fn from_legacy_config(
        legacy_config: &ratchet_lib::config::ExecutionConfig,
    ) -> Self {
        let config = crate::ProcessExecutorConfig {
            worker_count: legacy_config.max_workers,
            task_timeout_seconds: legacy_config.timeout_seconds,
            restart_on_crash: true,
            max_restart_attempts: 3,
        };
        Self::new(config)
    }
}

#[async_trait]
impl TaskExecutor for ExecutionBridge {
    type Error = crate::ExecutionError;

    async fn execute_task(
        &self,
        task_id: &str,
        input: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, Self::Error> {
        // Convert context and delegate to ProcessTaskExecutor
        let timeout = context.as_ref().and_then(|c| c.timeout);
        
        let result = self.inner.execute_json_task(task_id, input, timeout).await?;
        
        Ok(ExecutionResult {
            output: result.output,
            execution_time_ms: result.execution_time_ms,
            logs: result.logs,
            trace: result.trace,
        })
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // Delegate to ProcessTaskExecutor health check
        self.inner.health_check().await
    }
}
```

#### 2.2 Update MCP Service to Use Execution Bridge

**Update**: `ratchet-mcp/src/server/adapter.rs`
```rust
use ratchet_execution::ExecutionBridge;
use ratchet_interfaces::{TaskExecutor, ExecutionResult};

pub struct RatchetMcpAdapter {
    task_executor: Arc<ExecutionBridge>,  // Changed from ProcessTaskExecutor
    // ... other fields
}

impl RatchetMcpAdapter {
    pub fn new(
        executor_config: ratchet_execution::ProcessExecutorConfig,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        let task_executor = Arc::new(ExecutionBridge::new(executor_config));
        
        Self {
            task_executor,
            task_repository,
            execution_repository,
            // ... initialize other fields
        }
    }
}
```

#### 2.3 Remove Legacy Execution Dependencies

**Update**: `ratchet-cli/src/main.rs`
```rust
// Remove convert_to_legacy_repository_factory function
// Remove convert_to_legacy_config function

// Use only modular crates directly:
use ratchet_config::RatchetConfig;
use ratchet_execution::{ProcessTaskExecutor, ProcessExecutorConfig};
use ratchet_storage::seaorm::{DatabaseConnection, repositories::RepositoryFactory};

async fn serve_command(config_path: Option<&PathBuf>) -> Result<()> {
    // Load config using only modular system
    let config = RatchetConfig::load_from_file_or_default(config_path)?;
    
    // Create database connection directly
    let db = DatabaseConnection::new(config.storage.database.clone()).await?;
    let repos = RepositoryFactory::new(db);
    
    // Create executor directly
    let executor_config = ProcessExecutorConfig {
        worker_count: config.execution.max_workers,
        task_timeout_seconds: config.execution.timeout_seconds,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let executor = Arc::new(ProcessTaskExecutor::new(executor_config));
    
    // No more legacy conversion needed
    // ... rest of serve logic
}
```

### Phase 3: Configuration Unification (Week 3)
**Goal**: Eliminate dual configuration systems

#### 3.1 Create Configuration Adapter in `ratchet-config`

**File**: `ratchet-config/src/legacy.rs`
```rust
//! Legacy configuration conversion utilities
//! One-way conversion from old to new format only

use crate::RatchetConfig;
use crate::domains::*;

/// Convert legacy configuration to new modular format
pub fn from_legacy_config(legacy: ratchet_lib::config::RatchetConfig) -> RatchetConfig {
    RatchetConfig {
        execution: execution::ExecutionConfig {
            max_workers: legacy.execution.max_workers,
            timeout_seconds: legacy.execution.timeout_seconds,
            validate_schemas: legacy.execution.validate_schemas,
        },
        http: http::HttpConfig {
            timeout_seconds: legacy.http.timeout_seconds,
            user_agent: legacy.http.user_agent,
            max_retries: legacy.http.max_retries.unwrap_or(3),
            retry_delay_ms: legacy.http.retry_delay_ms.unwrap_or(1000),
        },
        server: server::ServerConfig {
            host: legacy.server.host,
            port: legacy.server.port,
            database: database::DatabaseConfig {
                url: legacy.server.database.url,
                max_connections: legacy.server.database.max_connections,
                connection_timeout: legacy.server.database.connection_timeout,
            },
        },
        logging: logging::LoggingConfig {
            level: match legacy.logging.level.as_str() {
                "trace" => logging::LogLevel::Trace,
                "debug" => logging::LogLevel::Debug,
                "info" => logging::LogLevel::Info,
                "warn" => logging::LogLevel::Warn,
                "error" => logging::LogLevel::Error,
                _ => logging::LogLevel::Info,
            },
            targets: legacy.logging.targets,
            structured: legacy.logging.structured,
            file_rotation: legacy.logging.file_rotation,
        },
        mcp: legacy.mcp.map(|mcp_config| mcp::McpConfig {
            enabled: mcp_config.enabled,
            transport: match mcp_config.transport.as_str() {
                "stdio" => mcp::Transport::Stdio,
                "sse" => mcp::Transport::Sse {
                    host: mcp_config.host,
                    port: mcp_config.port,
                },
                _ => mcp::Transport::Stdio,
            },
            auth: mcp::AuthConfig {
                enabled: false, // Default disabled
                method: mcp::AuthMethod::None,
            },
            security: mcp::SecurityConfig {
                max_execution_time_seconds: 300,
                max_log_entries: 1000,
                allow_dangerous_tasks: false,
            },
        }),
        registry: legacy.registry.map(|reg_config| registry::RegistryConfig {
            sources: reg_config.sources.into_iter().map(|source| {
                registry::SourceConfig {
                    name: source.name,
                    uri: source.uri,
                    source_type: match source.source_type.as_str() {
                        "filesystem" => registry::SourceType::Filesystem,
                        "http" => registry::SourceType::Http,
                        _ => registry::SourceType::Filesystem,
                    },
                    enabled: source.enabled,
                }
            }).collect(),
        }),
        output: output::OutputConfig::default(),
    }
}

/// Check if a legacy config file exists
pub fn legacy_config_exists(path: &std::path::Path) -> bool {
    path.exists() && path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml")
}
```

#### 3.2 Update CLI to Use Only Modular Configuration

**Update**: `ratchet-cli/src/main.rs`
```rust
use ratchet_config::{RatchetConfig, legacy};

async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration using unified system
    let config = if let Some(config_path) = &cli.config {
        // Check if it's a legacy config file
        if legacy::legacy_config_exists(config_path) {
            eprintln!("Warning: Legacy configuration format detected. Consider migrating to new format.");
            let legacy_config = ratchet_lib::config::RatchetConfig::load_from_file(config_path)?;
            legacy::from_legacy_config(legacy_config)
        } else {
            RatchetConfig::load_from_file(config_path)?
        }
    } else {
        RatchetConfig::default()
    };
    
    // Initialize logging using modular system
    ratchet_logging::init_with_config(&config.logging)?;
    
    // All commands now use only modular configuration
    match cli.command {
        Command::Serve => serve_command(&config).await,
        Command::McpServe(mcp_args) => mcp_serve_command(&config, mcp_args).await,
        // ... other commands
    }
}

// Updated serve command signature
async fn serve_command(config: &RatchetConfig) -> Result<()> {
    // Use only modular crates - no legacy conversion
    let db = ratchet_storage::seaorm::DatabaseConnection::new(
        config.server.database.clone()
    ).await?;
    
    let repos = ratchet_storage::seaorm::repositories::RepositoryFactory::new(db);
    
    let executor_config = ratchet_execution::ProcessExecutorConfig {
        worker_count: config.execution.max_workers,
        task_timeout_seconds: config.execution.timeout_seconds,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let executor = Arc::new(ratchet_execution::ProcessTaskExecutor::new(executor_config));
    
    // ... rest of serve logic using modular components
}

// Updated MCP serve command  
async fn mcp_serve_command(config: &RatchetConfig, args: McpServeArgs) -> Result<()> {
    let mcp_config = config.mcp.as_ref()
        .ok_or_else(|| anyhow!("MCP configuration not found"))?;
    
    // Create MCP service using only modular components
    let db = ratchet_storage::seaorm::DatabaseConnection::new(
        config.server.database.clone()
    ).await?;
    let repos = ratchet_storage::seaorm::repositories::RepositoryFactory::new(db);
    
    let executor_config = ratchet_execution::ProcessExecutorConfig::from_mcp_config(mcp_config);
    let executor = Arc::new(ratchet_execution::ProcessTaskExecutor::new(executor_config));
    
    let service = ratchet_mcp::server::McpService::from_config(
        mcp_config,
        executor,
        Arc::new(repos.task_repository()),
        Arc::new(repos.execution_repository()),
    ).await?;
    
    service.start().await?;
    Ok(())
}
```

#### 3.3 Remove Legacy Configuration Dependencies from ratchet-lib

**Update**: `ratchet-lib/Cargo.toml`
```toml
[dependencies]
# Remove these to break circular dependencies:
# ratchet-http = { path = "../ratchet-http" }
# ratchet-logging = { path = "../ratchet-logging" }
# ratchet-core = { path = "../ratchet-core" }

# Keep only interfaces for backward compatibility
ratchet-interfaces = { path = "../ratchet-interfaces" }

# Keep legacy-specific dependencies
anyhow = "1.0"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# ... other legacy-specific deps
```

### Phase 4: Legacy Isolation & Deprecation (Week 4)
**Goal**: Complete the migration and deprecate ratchet-lib

#### 4.1 Rename and Isolate Legacy Crate

```bash
# Rename to indicate legacy status
mv ratchet-lib ratchet-legacy
```

**Update**: `ratchet-legacy/Cargo.toml`
```toml
[package]
name = "ratchet-legacy"
version = "0.3.0"
edition = "2021"
description = "Legacy compatibility layer for Ratchet - DEPRECATED"

[dependencies]
# Only interfaces dependency - no circular deps
ratchet-interfaces = { path = "../ratchet-interfaces" }

# Legacy-specific dependencies only
anyhow = "1.0"
async-trait = "0.1"
boa_engine = { version = "0.20", optional = true }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
# ... other legacy deps
```

**Add**: `ratchet-legacy/src/lib.rs`
```rust
//! # Ratchet Legacy Compatibility Layer
//! 
//! **⚠️ DEPRECATED**: This crate is deprecated and will be removed in a future version.
//! Please migrate to the modular crates:
//! 
//! - `ratchet-core` for core types and validation
//! - `ratchet-execution` for task execution
//! - `ratchet-config` for configuration
//! - `ratchet-storage` for data persistence
//! - `ratchet-mcp` for MCP server functionality
//! 
//! This crate exists only for backward compatibility during the migration period.

#![deprecated(
    since = "0.3.0",
    note = "Use modular crates: ratchet-core, ratchet-execution, ratchet-config, etc."
)]

// Re-export interfaces for backward compatibility
pub use ratchet_interfaces as interfaces;

// Legacy modules - kept for compatibility but marked deprecated
#[deprecated]
pub mod config;
#[deprecated] 
pub mod task;
#[deprecated]
pub mod services;
// ... other legacy modules

// Provide migration helpers
pub mod migration {
    //! Migration utilities to help transition from legacy to modular crates
    
    /// Migration guide for common patterns
    pub const MIGRATION_GUIDE: &str = r#"
# Migration Guide from ratchet-lib to Modular Crates

## Task Loading and Execution
Before:
```rust
use ratchet_lib::{Task, RatchetEngine};
let engine = RatchetEngine::new(config)?;
let result = engine.execute_task_from_path("./task", input).await?;
```

After:
```rust
use ratchet_core::Task;
use ratchet_execution::{ProcessTaskExecutor, ProcessExecutorConfig};
let executor = ProcessTaskExecutor::new(ProcessExecutorConfig::default());
let task = Task::from_fs("./task")?;
let result = executor.execute_task(&task, input).await?;
```

## Configuration
Before:
```rust
use ratchet_lib::config::RatchetConfig;
let config = RatchetConfig::load_from_file("config.yaml")?;
```

After:
```rust
use ratchet_config::RatchetConfig;
let config = RatchetConfig::load_from_file("config.yaml")?;
```

## MCP Server
Before:
```rust
use ratchet_lib::mcp::McpServer;
let server = McpServer::new(config)?;
```

After:
```rust
use ratchet_mcp::server::McpService;
let service = McpService::from_config(&config.mcp, executor, repos).await?;
```
"#;
}
```

#### 4.2 Update Workspace Configuration

**Update**: `Cargo.toml`
```toml
[workspace]
members = [
    "ratchet-cli",
    "ratchet-interfaces",  # New
    "ratchet-core",
    "ratchet-config",
    "ratchet-execution",
    "ratchet-storage",
    "ratchet-mcp",
    "ratchet-http",
    "ratchet-logging",
    "ratchet-runtime",
    "ratchet-caching",
    "ratchet-ipc",
    "ratchet-js",
    "ratchet-plugin",
    "ratchet-plugins",
    "ratchet-resilience",
    "ratchet-legacy",     # Renamed and marked legacy
]

[workspace.dependencies]
# Define common dependencies for workspace
ratchet-interfaces = { path = "ratchet-interfaces" }
ratchet-core = { path = "ratchet-core" }
ratchet-config = { path = "ratchet-config" }
ratchet-execution = { path = "ratchet-execution" }
# ... other modular crates
```

#### 4.3 Remove All ratchet-lib Dependencies

**Update**: `ratchet-mcp/Cargo.toml`
```toml
[dependencies]
# ✅ Only interfaces and modular crates
ratchet-interfaces = { workspace = true }
ratchet-execution = { workspace = true }
ratchet-storage = { workspace = true }
ratchet-config = { workspace = true }

# ❌ Remove completely:
# ratchet_lib = { path = "../ratchet-lib" }
```

**Update**: `ratchet-cli/Cargo.toml`
```toml
[dependencies]
# ✅ Only modular crates
ratchet-interfaces = { workspace = true }
ratchet-core = { workspace = true }
ratchet-config = { workspace = true }
ratchet-execution = { workspace = true }
ratchet-storage = { workspace = true }
ratchet-mcp = { workspace = true }

# ❌ Remove legacy dependency:
# ratchet_lib = { path = "../ratchet-lib" }

# Optional legacy support for backward compatibility
ratchet-legacy = { workspace = true, optional = true }

[features]
default = []
legacy-support = ["ratchet-legacy"]
```

## Implementation Timeline

### Week 1: Interface Extraction
- [ ] Create `ratchet-interfaces` crate with extracted traits
- [ ] Update `ratchet-mcp` to use interfaces instead of `ratchet-lib`
- [ ] Update `ratchet-lib` to re-export from interfaces
- [ ] Test that circular dependency is broken

### Week 2: Execution Migration  
- [ ] Create execution bridge in `ratchet-execution`
- [ ] Update MCP adapter to use execution bridge
- [ ] Remove legacy execution dependencies from CLI
- [ ] Test execution layer works with modular system

### Week 3: Configuration Unification
- [ ] Create legacy config adapter in `ratchet-config`
- [ ] Update CLI to use only modular configuration
- [ ] Remove config dependencies from `ratchet-lib`
- [ ] Test configuration loading and conversion

### Week 4: Legacy Deprecation
- [ ] Rename `ratchet-lib` to `ratchet-legacy`
- [ ] Add deprecation warnings and migration guide
- [ ] Update all crates to remove `ratchet-lib` dependencies
- [ ] Add optional legacy support feature to CLI
- [ ] Update documentation and examples

## Success Criteria

1. **No Circular Dependencies**: `cargo check --workspace` passes without circular dependency errors
2. **Modular Build**: Each crate can be built independently
3. **Backward Compatibility**: Existing code works with deprecation warnings
4. **Clean Separation**: Legacy code isolated in `ratchet-legacy` crate
5. **Migration Path**: Clear migration guide and tooling provided

## Risk Mitigation

1. **Incremental Migration**: Each phase can be tested independently
2. **Backward Compatibility**: Legacy crate maintains API compatibility
3. **Feature Flags**: Optional legacy support for gradual transition
4. **Comprehensive Testing**: Test each phase thoroughly before proceeding
5. **Rollback Plan**: Each phase can be reverted if issues arise

This plan will eliminate circular dependencies while maintaining backward compatibility and providing a clear migration path for users.