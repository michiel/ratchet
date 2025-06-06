//! Plugin hook system for extending execution at specific points

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::PluginContext;
use crate::error::{PluginError, PluginResult};

/// Hook priority for determining execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u16)]
pub enum HookPriority {
    /// Highest priority (executes first)
    Highest = 0,
    /// High priority
    High = 100,
    /// Normal priority (default)
    Normal = 500,
    /// Low priority
    Low = 900,
    /// Lowest priority (executes last)
    Lowest = 1000,
    /// Custom priority with specific value
    Custom(u16),
}

impl Default for HookPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<u16> for HookPriority {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Highest,
            1..=99 => Self::High,
            100..=899 => Self::Normal,
            900..=999 => Self::Low,
            1000 => Self::Lowest,
            _ => Self::Custom(value),
        }
    }
}

impl From<HookPriority> for u16 {
    fn from(priority: HookPriority) -> Self {
        match priority {
            HookPriority::Highest => 0,
            HookPriority::High => 100,
            HookPriority::Normal => 500,
            HookPriority::Low => 900,
            HookPriority::Lowest => 1000,
            HookPriority::Custom(value) => value,
        }
    }
}

/// Task execution data passed to hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionData {
    /// Task ID
    pub task_id: String,
    /// Task input data
    pub input: serde_json::Value,
    /// Task output (available in post-execution hooks)
    pub output: Option<serde_json::Value>,
    /// Execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Execution duration in milliseconds (for post-execution hooks)
    pub duration_ms: Option<u64>,
    /// Whether the execution was successful
    pub success: Option<bool>,
    /// Error information (if execution failed)
    pub error: Option<String>,
}

impl TaskExecutionData {
    /// Create new task execution data
    pub fn new(task_id: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            task_id: task_id.into(),
            input,
            output: None,
            metadata: HashMap::new(),
            duration_ms: None,
            success: None,
            error: None,
        }
    }

    /// Set output data
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set execution result
    pub fn with_result(mut self, success: bool, duration_ms: u64) -> Self {
        self.success = Some(success);
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set error information
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.success = Some(false);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Base trait for all hooks
#[async_trait]
pub trait Hook: Send + Sync {
    /// Hook name/identifier
    fn name(&self) -> &str;

    /// Hook priority
    fn priority(&self) -> HookPriority {
        HookPriority::Normal
    }

    /// Whether this hook should run
    async fn should_run(&self, context: &PluginContext) -> bool {
        let _ = context;
        true
    }

    /// Execute the hook
    async fn execute(
        &self,
        context: &mut PluginContext,
        data: &mut serde_json::Value,
    ) -> PluginResult<()>;

    /// Handle hook execution errors
    async fn handle_error(&self, error: &PluginError, context: &PluginContext) -> PluginResult<()> {
        tracing::error!(
            target: "hook",
            hook = self.name(),
            error = %error,
            "Hook execution failed"
        );
        let _ = context;
        Ok(())
    }
}

/// Task-specific hooks for task execution lifecycle
#[async_trait]
pub trait TaskHook: Hook {
    /// Called before task validation
    async fn pre_validate(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let _ = (context, data);
        Ok(())
    }

    /// Called before task execution
    async fn pre_execute(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let _ = (context, data);
        Ok(())
    }

    /// Called after task execution (success or failure)
    async fn post_execute(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let _ = (context, data);
        Ok(())
    }

    /// Called when task execution succeeds
    async fn on_success(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let _ = (context, data);
        Ok(())
    }

    /// Called when task execution fails
    async fn on_failure(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let _ = (context, data);
        Ok(())
    }
}

/// Execution hooks for general execution lifecycle
#[async_trait]
pub trait ExecutionHook: Hook {
    /// Called at system startup
    async fn on_startup(&self, context: &mut PluginContext) -> PluginResult<()> {
        let _ = context;
        Ok(())
    }

    /// Called at system shutdown
    async fn on_shutdown(&self, context: &mut PluginContext) -> PluginResult<()> {
        let _ = context;
        Ok(())
    }

    /// Called on configuration changes
    async fn on_config_change(
        &self,
        context: &mut PluginContext,
        config: &serde_json::Value,
    ) -> PluginResult<()> {
        let _ = (context, config);
        Ok(())
    }

    /// Called on plugin loaded
    async fn on_plugin_loaded(
        &self,
        context: &mut PluginContext,
        plugin_id: &str,
    ) -> PluginResult<()> {
        let _ = (context, plugin_id);
        Ok(())
    }

    /// Called on plugin unloaded
    async fn on_plugin_unloaded(
        &self,
        context: &mut PluginContext,
        plugin_id: &str,
    ) -> PluginResult<()> {
        let _ = (context, plugin_id);
        Ok(())
    }
}

/// Hook registration information
#[derive(Debug, Clone)]
pub struct HookRegistration {
    /// Hook ID
    pub id: Uuid,
    /// Hook name
    pub name: String,
    /// Hook priority
    pub priority: HookPriority,
    /// Plugin ID that registered this hook
    pub plugin_id: String,
    /// Whether the hook is enabled
    pub enabled: bool,
}

/// Registry for managing hooks
pub struct HookRegistry {
    /// Registered task hooks
    task_hooks: Arc<RwLock<BTreeMap<u16, Vec<(HookRegistration, Arc<dyn TaskHook>)>>>>,
    /// Registered execution hooks
    execution_hooks: Arc<RwLock<BTreeMap<u16, Vec<(HookRegistration, Arc<dyn ExecutionHook>)>>>>,
    /// Hook statistics
    stats: Arc<RwLock<HashMap<String, HookStats>>>,
}

/// Hook execution statistics
#[derive(Debug, Clone, Default)]
pub struct HookStats {
    /// Total number of executions
    pub executions: u64,
    /// Number of successful executions
    pub successes: u64,
    /// Number of failed executions
    pub failures: u64,
    /// Total execution time in microseconds
    pub total_time_us: u64,
    /// Average execution time in microseconds
    pub avg_time_us: f64,
}

impl HookStats {
    /// Record a successful execution
    pub fn record_success(&mut self, duration_us: u64) {
        self.executions += 1;
        self.successes += 1;
        self.total_time_us += duration_us;
        self.avg_time_us = self.total_time_us as f64 / self.executions as f64;
    }

    /// Record a failed execution
    pub fn record_failure(&mut self, duration_us: u64) {
        self.executions += 1;
        self.failures += 1;
        self.total_time_us += duration_us;
        self.avg_time_us = self.total_time_us as f64 / self.executions as f64;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            self.successes as f64 / self.executions as f64
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistry {
    /// Create a new hook registry
    pub fn new() -> Self {
        Self {
            task_hooks: Arc::new(RwLock::new(BTreeMap::new())),
            execution_hooks: Arc::new(RwLock::new(BTreeMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a task hook
    pub async fn register_task_hook(
        &self,
        hook: Arc<dyn TaskHook>,
        plugin_id: impl Into<String>,
    ) -> PluginResult<Uuid> {
        let registration = HookRegistration {
            id: Uuid::new_v4(),
            name: hook.name().to_string(),
            priority: hook.priority(),
            plugin_id: plugin_id.into(),
            enabled: true,
        };

        let priority_value: u16 = registration.priority.into();
        let mut hooks = self.task_hooks.write().await;
        hooks
            .entry(priority_value)
            .or_insert_with(Vec::new)
            .push((registration.clone(), hook));

        tracing::info!(
            target: "hook_registry",
            hook_id = %registration.id,
            hook_name = %registration.name,
            priority = priority_value,
            "Task hook registered"
        );

        Ok(registration.id)
    }

    /// Register an execution hook
    pub async fn register_execution_hook(
        &self,
        hook: Arc<dyn ExecutionHook>,
        plugin_id: impl Into<String>,
    ) -> PluginResult<Uuid> {
        let registration = HookRegistration {
            id: Uuid::new_v4(),
            name: hook.name().to_string(),
            priority: hook.priority(),
            plugin_id: plugin_id.into(),
            enabled: true,
        };

        let priority_value: u16 = registration.priority.into();
        let mut hooks = self.execution_hooks.write().await;
        hooks
            .entry(priority_value)
            .or_insert_with(Vec::new)
            .push((registration.clone(), hook));

        tracing::info!(
            target: "hook_registry",
            hook_id = %registration.id,
            hook_name = %registration.name,
            priority = priority_value,
            "Execution hook registered"
        );

        Ok(registration.id)
    }

    /// Unregister a hook by ID
    pub async fn unregister_hook(&self, hook_id: Uuid) -> PluginResult<bool> {
        // Try task hooks first
        {
            let mut task_hooks = self.task_hooks.write().await;
            for (_, hooks) in task_hooks.iter_mut() {
                if let Some(pos) = hooks.iter().position(|(reg, _)| reg.id == hook_id) {
                    let (registration, _) = hooks.remove(pos);
                    tracing::info!(
                        target: "hook_registry",
                        hook_id = %hook_id,
                        hook_name = %registration.name,
                        "Task hook unregistered"
                    );
                    return Ok(true);
                }
            }
        }

        // Try execution hooks
        {
            let mut execution_hooks = self.execution_hooks.write().await;
            for (_, hooks) in execution_hooks.iter_mut() {
                if let Some(pos) = hooks.iter().position(|(reg, _)| reg.id == hook_id) {
                    let (registration, _) = hooks.remove(pos);
                    tracing::info!(
                        target: "hook_registry",
                        hook_id = %hook_id,
                        hook_name = %registration.name,
                        "Execution hook unregistered"
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Execute pre-validation hooks
    pub async fn execute_pre_validation_hooks(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let hooks = self.task_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.pre_validate(context, data).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute pre-execution hooks
    pub async fn execute_pre_execution_hooks(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let hooks = self.task_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.pre_execute(context, data).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute post-execution hooks
    pub async fn execute_post_execution_hooks(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let hooks = self.task_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.post_execute(context, data).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute success hooks
    pub async fn execute_success_hooks(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let hooks = self.task_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_success(context, data).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute failure hooks
    pub async fn execute_failure_hooks(
        &self,
        context: &mut PluginContext,
        data: &mut TaskExecutionData,
    ) -> PluginResult<()> {
        let hooks = self.task_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_failure(context, data).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute startup hooks
    pub async fn execute_startup_hooks(&self, context: &mut PluginContext) -> PluginResult<()> {
        let hooks = self.execution_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_startup(context).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute shutdown hooks
    pub async fn execute_shutdown_hooks(&self, context: &mut PluginContext) -> PluginResult<()> {
        let hooks = self.execution_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_shutdown(context).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute plugin loaded hooks
    pub async fn execute_plugin_loaded_hooks(
        &self,
        context: &mut PluginContext,
        plugin_id: &str,
    ) -> PluginResult<()> {
        let hooks = self.execution_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_plugin_loaded(context, plugin_id).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute plugin unloaded hooks
    pub async fn execute_plugin_unloaded_hooks(
        &self,
        context: &mut PluginContext,
        plugin_id: &str,
    ) -> PluginResult<()> {
        let hooks = self.execution_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_plugin_unloaded(context, plugin_id).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute config change hooks
    pub async fn execute_config_change_hooks(
        &self,
        context: &mut PluginContext,
        config: &serde_json::Value,
    ) -> PluginResult<()> {
        let hooks = self.execution_hooks.read().await;

        for (_, priority_hooks) in hooks.iter() {
            for (registration, hook) in priority_hooks {
                if !registration.enabled {
                    continue;
                }

                if !hook.should_run(context).await {
                    continue;
                }

                let start_time = std::time::Instant::now();
                let result = hook.on_config_change(context, config).await;
                let duration_us = start_time.elapsed().as_micros() as u64;

                // Update statistics
                let mut stats = self.stats.write().await;
                let hook_stats = stats.entry(registration.name.clone()).or_default();

                match result {
                    Ok(()) => hook_stats.record_success(duration_us),
                    Err(ref e) => {
                        hook_stats.record_failure(duration_us);
                        hook.handle_error(e, context).await?;
                        return Err(PluginError::hook_execution_failed(
                            &registration.name,
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get hook statistics
    pub async fn get_stats(&self) -> HashMap<String, HookStats> {
        self.stats.read().await.clone()
    }

    /// Get all registered hooks
    pub async fn list_hooks(&self) -> Vec<HookRegistration> {
        let mut registrations = Vec::new();

        let task_hooks = self.task_hooks.read().await;
        for (_, hooks) in task_hooks.iter() {
            for (registration, _) in hooks {
                registrations.push(registration.clone());
            }
        }

        let execution_hooks = self.execution_hooks.read().await;
        for (_, hooks) in execution_hooks.iter() {
            for (registration, _) in hooks {
                registrations.push(registration.clone());
            }
        }

        registrations
    }

    /// Enable or disable a hook
    pub async fn set_hook_enabled(&self, hook_id: Uuid, enabled: bool) -> PluginResult<bool> {
        // Try task hooks first
        {
            let mut task_hooks = self.task_hooks.write().await;
            for (_, hooks) in task_hooks.iter_mut() {
                if let Some((registration, _)) = hooks.iter_mut().find(|(reg, _)| reg.id == hook_id)
                {
                    registration.enabled = enabled;
                    return Ok(true);
                }
            }
        }

        // Try execution hooks
        {
            let mut execution_hooks = self.execution_hooks.write().await;
            for (_, hooks) in execution_hooks.iter_mut() {
                if let Some((registration, _)) = hooks.iter_mut().find(|(reg, _)| reg.id == hook_id)
                {
                    registration.enabled = enabled;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PluginContext;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestTaskHook {
        name: String,
        priority: HookPriority,
        call_count: AtomicUsize,
    }

    impl TestTaskHook {
        fn new(name: impl Into<String>, priority: HookPriority) -> Self {
            Self {
                name: name.into(),
                priority,
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Hook for TestTaskHook {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> HookPriority {
            self.priority
        }

        async fn execute(
            &self,
            _context: &mut PluginContext,
            _data: &mut serde_json::Value,
        ) -> PluginResult<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl TaskHook for TestTaskHook {
        async fn pre_execute(
            &self,
            _context: &mut PluginContext,
            _data: &mut TaskExecutionData,
        ) -> PluginResult<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_priority_ordering() {
        let registry = HookRegistry::new();

        let hook1 = Arc::new(TestTaskHook::new("hook1", HookPriority::Low));
        let hook2 = Arc::new(TestTaskHook::new("hook2", HookPriority::High));
        let hook3 = Arc::new(TestTaskHook::new("hook3", HookPriority::Normal));

        // Register hooks in different order
        registry
            .register_task_hook(hook1.clone(), "plugin1")
            .await
            .unwrap();
        registry
            .register_task_hook(hook2.clone(), "plugin2")
            .await
            .unwrap();
        registry
            .register_task_hook(hook3.clone(), "plugin3")
            .await
            .unwrap();

        let mut context = PluginContext::new(
            Uuid::new_v4(),
            serde_json::json!({}),
            ratchet_config::RatchetConfig::default(),
        );
        let mut data = TaskExecutionData::new("test-task", serde_json::json!({}));

        // Execute hooks - should execute in priority order (High, Normal, Low)
        registry
            .execute_pre_execution_hooks(&mut context, &mut data)
            .await
            .unwrap();

        // All hooks should have been called
        assert_eq!(hook1.call_count(), 1);
        assert_eq!(hook2.call_count(), 1);
        assert_eq!(hook3.call_count(), 1);
    }

    #[tokio::test]
    async fn test_hook_registration_and_unregistration() {
        let registry = HookRegistry::new();
        let hook = Arc::new(TestTaskHook::new("test-hook", HookPriority::Normal));

        // Register hook
        let hook_id = registry
            .register_task_hook(hook.clone(), "test-plugin")
            .await
            .unwrap();

        // List hooks
        let hooks = registry.list_hooks().await;
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].name, "test-hook");

        // Unregister hook
        let success = registry.unregister_hook(hook_id).await.unwrap();
        assert!(success);

        // List hooks should be empty now
        let hooks = registry.list_hooks().await;
        assert_eq!(hooks.len(), 0);
    }

    #[tokio::test]
    async fn test_hook_enable_disable() {
        let registry = HookRegistry::new();
        let hook = Arc::new(TestTaskHook::new("test-hook", HookPriority::Normal));

        let hook_id = registry
            .register_task_hook(hook.clone(), "test-plugin")
            .await
            .unwrap();

        // Disable hook
        let success = registry.set_hook_enabled(hook_id, false).await.unwrap();
        assert!(success);

        let mut context = PluginContext::new(
            Uuid::new_v4(),
            serde_json::json!({}),
            ratchet_config::RatchetConfig::default(),
        );
        let mut data = TaskExecutionData::new("test-task", serde_json::json!({}));

        // Execute hooks - disabled hook should not be called
        registry
            .execute_pre_execution_hooks(&mut context, &mut data)
            .await
            .unwrap();

        assert_eq!(hook.call_count(), 0);

        // Enable hook
        registry.set_hook_enabled(hook_id, true).await.unwrap();

        // Execute hooks - enabled hook should be called
        registry
            .execute_pre_execution_hooks(&mut context, &mut data)
            .await
            .unwrap();

        assert_eq!(hook.call_count(), 1);
    }

    #[test]
    fn test_hook_priority_conversion() {
        assert_eq!(u16::from(HookPriority::Highest), 0);
        assert_eq!(u16::from(HookPriority::High), 100);
        assert_eq!(u16::from(HookPriority::Normal), 500);
        assert_eq!(u16::from(HookPriority::Low), 900);
        assert_eq!(u16::from(HookPriority::Lowest), 1000);
        assert_eq!(u16::from(HookPriority::Custom(750)), 750);

        assert_eq!(HookPriority::from(0), HookPriority::Highest);
        assert_eq!(HookPriority::from(50), HookPriority::High);
        assert_eq!(HookPriority::from(500), HookPriority::Normal);
        assert_eq!(HookPriority::from(950), HookPriority::Low);
        assert_eq!(HookPriority::from(1000), HookPriority::Lowest);
    }

    #[test]
    fn test_task_execution_data() {
        let mut data = TaskExecutionData::new("test-task", serde_json::json!({"input": "value"}));

        data = data.with_metadata("key", serde_json::json!("value"));
        data = data.with_output(serde_json::json!({"output": "result"}));
        data = data.with_result(true, 1000);

        assert_eq!(data.task_id, "test-task");
        assert_eq!(data.input["input"], "value");
        assert_eq!(data.output.as_ref().unwrap()["output"], "result");
        assert_eq!(data.success, Some(true));
        assert_eq!(data.duration_ms, Some(1000));
        assert_eq!(data.metadata["key"], "value");
    }

    #[tokio::test]
    async fn test_hook_stats() {
        let registry = HookRegistry::new();
        let hook = Arc::new(TestTaskHook::new("test-hook", HookPriority::Normal));

        registry
            .register_task_hook(hook.clone(), "test-plugin")
            .await
            .unwrap();

        let mut context = PluginContext::new(
            Uuid::new_v4(),
            serde_json::json!({}),
            ratchet_config::RatchetConfig::default(),
        );
        let mut data = TaskExecutionData::new("test-task", serde_json::json!({}));

        // Execute hook multiple times
        for _ in 0..3 {
            registry
                .execute_pre_execution_hooks(&mut context, &mut data)
                .await
                .unwrap();
        }

        let stats = registry.get_stats().await;
        let hook_stats = stats.get("test-hook").unwrap();

        assert_eq!(hook_stats.executions, 3);
        assert_eq!(hook_stats.successes, 3);
        assert_eq!(hook_stats.failures, 0);
        assert_eq!(hook_stats.success_rate(), 1.0);
    }
}
