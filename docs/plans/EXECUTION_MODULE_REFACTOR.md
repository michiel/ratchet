# Execution Module Refactoring Example

This document provides a concrete example of refactoring the monolithic `execution` module into smaller, focused modules.

## Current Structure Problems

The `execution` module currently contains:
- `executor.rs` - Task execution orchestration
- `graceful_shutdown.rs` - Shutdown handling
- `ipc.rs` - Inter-process communication
- `job_queue.rs` - Job queuing logic
- `load_balancer.rs` - Load balancing
- `process_executor.rs` - Process management (979 lines!)
- `retry.rs` - Retry logic
- `task_cache.rs` - Task caching
- `worker.rs` - Worker traits
- `worker_process.rs` - Worker implementation (723 lines!)

Total: ~3,000 lines in a single module mixing multiple concerns.

## Proposed Refactored Structure

```
ratchet-runtime/
├── src/
│   ├── lib.rs
│   ├── executor/
│   │   ├── mod.rs          # Core execution traits
│   │   ├── orchestrator.rs # Task orchestration
│   │   └── context.rs      # Execution context
│   │
│   ├── process/
│   │   ├── mod.rs          # Process management traits
│   │   ├── pool.rs         # Process pool management
│   │   ├── supervisor.rs   # Process supervision
│   │   └── lifecycle.rs    # Process lifecycle
│   │
│   ├── worker/
│   │   ├── mod.rs          # Worker traits
│   │   ├── implementation.rs # Worker implementation
│   │   ├── scheduler.rs    # Work scheduling
│   │   └── state.rs        # Worker state management
│   │
│   └── scheduling/
│       ├── mod.rs          # Scheduling traits
│       ├── queue.rs        # Job queue implementation
│       ├── priority.rs     # Priority scheduling
│       └── balancer.rs     # Load balancing

ratchet-ipc/
├── src/
│   ├── lib.rs
│   ├── protocol.rs         # IPC protocol definition
│   ├── messages.rs         # Message types
│   ├── transport.rs        # Transport abstraction
│   └── serialization.rs    # Message serialization

ratchet-resilience/
├── src/
│   ├── lib.rs
│   ├── retry/
│   │   ├── mod.rs          # Retry traits
│   │   ├── policy.rs       # Retry policies
│   │   └── backoff.rs      # Backoff strategies
│   │
│   ├── circuit_breaker/
│   │   ├── mod.rs          # Circuit breaker pattern
│   │   └── state.rs        # Circuit states
│   │
│   └── shutdown/
│       ├── mod.rs          # Graceful shutdown
│       └── coordinator.rs   # Shutdown coordination

ratchet-caching/
├── src/
│   ├── lib.rs
│   ├── cache.rs            # Cache trait
│   ├── memory.rs           # In-memory cache
│   ├── task_cache.rs       # Task-specific caching
│   └── invalidation.rs     # Cache invalidation
```

## Detailed Refactoring Example: Process Executor

### Before (Monolithic)

```rust
// execution/process_executor.rs (979 lines!)

pub struct ProcessExecutor {
    config: Arc<RatchetConfig>,
    processes: Arc<Mutex<HashMap<u32, ProcessInfo>>>,
    task_service: Arc<dyn TaskService + Send + Sync>,
    shutdown: Arc<AtomicBool>,
    // ... many more fields
}

impl ProcessExecutor {
    // Everything in one place:
    // - Process spawning
    // - Health checking
    // - Message routing
    // - Error handling
    // - Shutdown coordination
    // - Performance monitoring
    // ... 20+ methods
}
```

### After (Modular)

#### Core Process Management

```rust
// ratchet-runtime/src/process/mod.rs

/// Core process management trait
pub trait ProcessManager: Send + Sync {
    /// Spawn a new process
    async fn spawn_process(&self, config: ProcessConfig) -> Result<ProcessHandle>;
    
    /// Get process by ID
    async fn get_process(&self, id: ProcessId) -> Result<Option<ProcessHandle>>;
    
    /// List all processes
    async fn list_processes(&self) -> Result<Vec<ProcessInfo>>;
    
    /// Terminate a process
    async fn terminate_process(&self, id: ProcessId) -> Result<()>;
}

/// Process handle for interaction
pub struct ProcessHandle {
    id: ProcessId,
    stdin: ChildStdin,
    stdout: ChildStdout,
    child: Child,
}

/// Process configuration
pub struct ProcessConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
}
```

#### Process Pool Management

```rust
// ratchet-runtime/src/process/pool.rs

/// Manages a pool of worker processes
pub struct ProcessPool {
    manager: Arc<dyn ProcessManager>,
    config: PoolConfig,
    processes: Arc<RwLock<Vec<ProcessHandle>>>,
    health_checker: Arc<HealthChecker>,
}

impl ProcessPool {
    /// Create a new process pool
    pub fn new(
        manager: Arc<dyn ProcessManager>,
        config: PoolConfig,
    ) -> Self {
        Self {
            manager,
            config,
            processes: Arc::new(RwLock::new(Vec::new())),
            health_checker: Arc::new(HealthChecker::new()),
        }
    }
    
    /// Scale pool to desired size
    pub async fn scale_to(&self, size: usize) -> Result<()> {
        let current = self.processes.read().await.len();
        
        match size.cmp(&current) {
            Ordering::Greater => self.spawn_processes(size - current).await,
            Ordering::Less => self.remove_processes(current - size).await,
            Ordering::Equal => Ok(()),
        }
    }
    
    /// Get a healthy process for work
    pub async fn get_available_process(&self) -> Result<ProcessHandle> {
        // Implementation focused only on pool management
    }
}
```

#### Process Supervision

```rust
// ratchet-runtime/src/process/supervisor.rs

/// Supervises processes and handles failures
pub struct ProcessSupervisor {
    pool: Arc<ProcessPool>,
    policy: SupervisionPolicy,
    metrics: Arc<Metrics>,
}

impl ProcessSupervisor {
    /// Start supervision
    pub async fn start(&self) -> Result<()> {
        loop {
            self.check_health().await?;
            self.handle_failures().await?;
            self.report_metrics().await?;
            
            tokio::time::sleep(self.policy.check_interval).await;
        }
    }
    
    async fn check_health(&self) -> Result<()> {
        // Focused on health checking
    }
    
    async fn handle_failures(&self) -> Result<()> {
        // Focused on failure handling
    }
}
```

#### IPC Separation

```rust
// ratchet-ipc/src/protocol.rs

/// IPC protocol definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    Request(IpcRequest),
    Response(IpcResponse),
    Event(IpcEvent),
}

/// IPC transport abstraction
#[async_trait]
pub trait IpcTransport: Send + Sync {
    async fn send(&mut self, message: IpcMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<IpcMessage>;
    async fn close(&mut self) -> Result<()>;
}

/// Standard I/O transport implementation
pub struct StdioTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[async_trait]
impl IpcTransport for StdioTransport {
    async fn send(&mut self, message: IpcMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<IpcMessage> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line)?)
    }
    
    async fn close(&mut self) -> Result<()> {
        // Clean shutdown
        Ok(())
    }
}
```

#### Retry Logic Separation

```rust
// ratchet-resilience/src/retry/mod.rs

/// Retry policy trait
pub trait RetryPolicy: Send + Sync {
    /// Determine if operation should be retried
    fn should_retry(&self, attempt: u32, error: &RatchetError) -> bool;
    
    /// Calculate delay before next retry
    fn retry_delay(&self, attempt: u32) -> Duration;
}

/// Exponential backoff retry policy
pub struct ExponentialBackoff {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl RetryPolicy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32, error: &RatchetError) -> bool {
        attempt < self.max_attempts && error.is_retryable()
    }
    
    fn retry_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay.as_secs_f64() 
            * self.multiplier.powi(attempt as i32);
        
        Duration::from_secs_f64(delay.min(self.max_delay.as_secs_f64()))
    }
}

/// Retry executor
pub struct RetryExecutor<T> {
    policy: Box<dyn RetryPolicy>,
    operation: Box<dyn Fn() -> Future<Output = Result<T>> + Send + Sync>,
}

impl<T> RetryExecutor<T> {
    pub async fn execute(&self) -> Result<T> {
        let mut attempt = 0;
        
        loop {
            match (self.operation)().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempt += 1;
                    
                    if !self.policy.should_retry(attempt, &error) {
                        return Err(error);
                    }
                    
                    let delay = self.policy.retry_delay(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
```

## Benefits of This Refactoring

### 1. **Single Responsibility**
Each module now has a clear, focused purpose:
- `process/pool.rs` - Only manages process pools
- `ipc/transport.rs` - Only handles IPC communication
- `retry/policy.rs` - Only implements retry logic

### 2. **Testability**
Smaller, focused modules are easier to test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_process_pool_scaling() {
        let manager = MockProcessManager::new();
        let pool = ProcessPool::new(Arc::new(manager), PoolConfig::default());
        
        // Test only pool scaling logic
        pool.scale_to(5).await.unwrap();
        assert_eq!(pool.size().await, 5);
        
        pool.scale_to(3).await.unwrap();
        assert_eq!(pool.size().await, 3);
    }
}
```

### 3. **Reusability**
Separated modules can be reused:
- Retry logic can be used anywhere in the codebase
- IPC protocol can be used for other communication needs
- Process management can be used for non-worker processes

### 4. **Compilation Efficiency**
Smaller modules compile faster:
- Changes to retry logic don't recompile process management
- IPC changes don't affect execution logic
- Feature flags can exclude unused modules

### 5. **Maintainability**
Clear boundaries make maintenance easier:
- Bug in retry logic? Look only in `retry/` module
- IPC protocol change? Only affects `ipc/` module
- New load balancing algorithm? Add to `scheduling/` module

## Migration Strategy

1. **Create New Structure**: Build new modules alongside existing
2. **Implement Interfaces**: Define traits for each concern
3. **Gradual Migration**: Move functionality piece by piece
4. **Update Tests**: Ensure all tests pass after each step
5. **Remove Old Code**: Delete monolithic module once complete

This refactoring demonstrates how breaking down large modules into focused, single-purpose components improves the overall architecture while maintaining functionality.