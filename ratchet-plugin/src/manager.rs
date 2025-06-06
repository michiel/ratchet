//! Plugin manager for orchestrating the complete plugin lifecycle
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::PluginContext;
use crate::discovery::{DiscoveryConfig, PluginCatalog, PluginDiscovery};
use crate::error::{PluginError, PluginResult};
use crate::hooks::{ExecutionHook, HookRegistry, TaskExecutionData, TaskHook};
use crate::loader::{ConfiguredPluginLoader, LoaderConfig, PluginLoader};
use crate::registry::{PluginInfo, PluginRegistry};
use crate::types::{PluginStatus, PluginType};

/// Plugin manager configuration
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    /// Plugin loading configuration
    pub loader_config: LoaderConfig,
    /// Plugin discovery configuration
    pub discovery_config: DiscoveryConfig,
    /// Whether to auto-discover plugins on startup
    pub auto_discover: bool,
    /// Whether to auto-load discovered plugins
    pub auto_load: bool,
    /// Maximum number of concurrent plugin operations
    pub max_concurrent_operations: usize,
    /// Plugin execution timeout
    pub execution_timeout: std::time::Duration,
    /// Whether to enable plugin hot reloading
    pub enable_hot_reload: bool,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            loader_config: LoaderConfig::default(),
            discovery_config: DiscoveryConfig::default(),
            auto_discover: true,
            auto_load: false, // Require explicit loading for security
            max_concurrent_operations: 10,
            execution_timeout: std::time::Duration::from_secs(300), // 5 minutes
            enable_hot_reload: false,
        }
    }
}

/// Plugin manager for orchestrating plugin lifecycle
pub struct PluginManager {
    /// Plugin registry
    registry: Arc<PluginRegistry>,
    /// Hook registry
    hooks: Arc<HookRegistry>,
    /// Plugin loader
    loader: Arc<dyn PluginLoader>,
    /// Plugin discovery service
    discovery: Arc<PluginDiscovery>,
    /// Plugin catalog
    catalog: Arc<RwLock<PluginCatalog>>,
    /// Manager configuration
    config: PluginManagerConfig,
    /// Manager state
    state: Arc<RwLock<ManagerState>>,
    /// System configuration
    system_config: Arc<RwLock<ratchet_config::RatchetConfig>>,
}

/// Internal manager state
#[derive(Debug, Default)]
struct ManagerState {
    /// Whether the manager is initialized
    initialized: bool,
    /// Whether the manager is shutting down
    shutting_down: bool,
    /// Active plugin contexts
    active_contexts: HashMap<String, Arc<RwLock<PluginContext>>>,
    /// Manager statistics
    stats: ManagerStats,
}

/// Plugin manager statistics
#[derive(Debug, Clone, Default)]
pub struct ManagerStats {
    /// Total number of plugin load attempts
    pub total_loads: u64,
    /// Total number of successful loads
    pub successful_loads: u64,
    /// Total number of plugin unloads
    pub total_unloads: u64,
    /// Total number of plugin executions
    pub total_executions: u64,
    /// Total number of failed executions
    pub failed_executions: u64,
    /// Average plugin execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Manager uptime in seconds
    pub uptime_seconds: f64,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginManagerConfig, system_config: ratchet_config::RatchetConfig) -> Self {
        let registry = Arc::new(PluginRegistry::new());
        let hooks = Arc::new(HookRegistry::new());
        let loader = Arc::new(ConfiguredPluginLoader::new(config.loader_config.clone()));
        let discovery = Arc::new(PluginDiscovery::new(
            config.discovery_config.clone(),
            config.loader_config.clone(),
        ));
        let catalog = Arc::new(RwLock::new(PluginCatalog::new()));
        let state = Arc::new(RwLock::new(ManagerState::default()));
        let system_config = Arc::new(RwLock::new(system_config));

        Self {
            registry,
            hooks,
            loader,
            discovery,
            catalog,
            config,
            state,
            system_config,
        }
    }

    /// Initialize the plugin manager
    pub async fn initialize(&self) -> PluginResult<()> {
        let mut state = self.state.write().await;
        if state.initialized {
            return Ok(());
        }

        tracing::info!(target: "plugin_manager", "Initializing plugin manager");

        // Auto-discover plugins if enabled
        if self.config.auto_discover {
            drop(state); // Release lock before async operation
            self.discover_plugins().await?;
            state = self.state.write().await;
        }

        // Auto-load plugins if enabled
        if self.config.auto_load {
            drop(state);
            self.auto_load_plugins().await?;
            state = self.state.write().await;
        }

        state.initialized = true;
        state.stats.uptime_seconds = 0.0;

        tracing::info!(target: "plugin_manager", "Plugin manager initialized");
        Ok(())
    }

    /// Shutdown the plugin manager
    pub async fn shutdown(&self) -> PluginResult<()> {
        let mut state = self.state.write().await;
        if state.shutting_down {
            return Ok(());
        }

        state.shutting_down = true;
        tracing::info!(target: "plugin_manager", "Shutting down plugin manager");

        // Unload all active plugins
        let plugin_ids: Vec<String> = state.active_contexts.keys().cloned().collect();
        drop(state);

        for plugin_id in plugin_ids {
            if let Err(e) = self.unload_plugin(&plugin_id).await {
                tracing::error!(
                    target: "plugin_manager",
                    plugin_id = %plugin_id,
                    error = %e,
                    "Failed to unload plugin during shutdown"
                );
            }
        }

        tracing::info!(target: "plugin_manager", "Plugin manager shutdown complete");
        Ok(())
    }

    /// Discover plugins in configured search paths
    pub async fn discover_plugins(&self) -> PluginResult<()> {
        tracing::info!(target: "plugin_manager", "Starting plugin discovery");

        let discovered = self.discovery.discover_plugins().await?;

        {
            let mut catalog = self.catalog.write().await;
            catalog.add_plugins(discovered.clone());
        }

        tracing::info!(
            target: "plugin_manager",
            count = discovered.len(),
            "Plugin discovery completed"
        );

        Ok(())
    }

    /// Auto-load compatible plugins from catalog
    async fn auto_load_plugins(&self) -> PluginResult<()> {
        let catalog = self.catalog.read().await;
        let plugins = catalog.list_plugins();

        for discovered in plugins {
            // Check if plugin is compatible
            if self
                .registry
                .is_compatible(&discovered.manifest.plugin)
                .await?
            {
                let source = discovered.source_path.to_string_lossy();
                if let Err(e) = self
                    .load_plugin_from_source(&source, serde_json::json!({}))
                    .await
                {
                    tracing::warn!(
                        target: "plugin_manager",
                        plugin_id = %discovered.manifest.plugin.id,
                        error = %e,
                        "Failed to auto-load plugin"
                    );
                }
            }
        }

        Ok(())
    }

    /// Load a plugin from source
    pub async fn load_plugin_from_source(
        &self,
        source: &str,
        config: serde_json::Value,
    ) -> PluginResult<String> {
        let mut state = self.state.write().await;
        state.stats.total_loads += 1;
        drop(state);

        tracing::info!(
            target: "plugin_manager",
            source = source,
            "Loading plugin from source"
        );

        // Load plugin using loader
        let plugin = self.loader.load_plugin(source).await?;
        let plugin_id = plugin.metadata().id.clone();

        // Check if already loaded
        if self.registry.get_plugin_info(&plugin_id).await.is_some() {
            return Err(PluginError::PluginAlreadyExists { name: plugin_id });
        }

        // Validate plugin configuration
        plugin.validate_config(&config)?;

        // Register plugin
        self.registry
            .register_plugin(plugin, config.clone())
            .await?;

        // Create plugin context
        let system_config = self.system_config.read().await.clone();
        let execution_id = Uuid::new_v4();
        let context = Arc::new(RwLock::new(PluginContext::new(
            execution_id,
            config,
            system_config,
        )));

        // Store context
        {
            let mut state = self.state.write().await;
            state
                .active_contexts
                .insert(plugin_id.clone(), context.clone());
            state.stats.successful_loads += 1;
        }

        // Update plugin status
        self.registry
            .update_plugin_status(&plugin_id, PluginStatus::Loading)
            .await?;

        // Initialize plugin
        if let Some(instance) = self.registry.get_plugin_instance(&plugin_id).await {
            let mut plugin = instance.write().await;
            let mut context = context.write().await;

            match plugin.initialize(&mut context).await {
                Ok(()) => {
                    self.registry
                        .update_plugin_status(&plugin_id, PluginStatus::Active)
                        .await?;

                    // Execute startup hooks
                    self.hooks
                        .execute_plugin_loaded_hooks(&mut context, &plugin_id)
                        .await?;

                    tracing::info!(
                        target: "plugin_manager",
                        plugin_id = %plugin_id,
                        "Plugin loaded and initialized successfully"
                    );
                }
                Err(e) => {
                    self.registry
                        .update_plugin_error(&plugin_id, e.to_string())
                        .await?;
                    self.registry
                        .update_plugin_status(&plugin_id, PluginStatus::Failed)
                        .await?;
                    return Err(PluginError::initialization_failed(
                        &plugin_id,
                        e.to_string(),
                    ));
                }
            }
        }

        Ok(plugin_id)
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        tracing::info!(
            target: "plugin_manager",
            plugin_id = %plugin_id,
            "Unloading plugin"
        );

        // Get plugin context
        let context = {
            let mut state = self.state.write().await;
            state.active_contexts.remove(plugin_id)
        };

        if let Some(context) = context {
            // Update plugin status
            self.registry
                .update_plugin_status(plugin_id, PluginStatus::Unloading)
                .await?;

            // Shutdown plugin
            if let Some(instance) = self.registry.get_plugin_instance(plugin_id).await {
                let mut plugin = instance.write().await;
                let mut context = context.write().await;

                if let Err(e) = plugin.shutdown(&mut context).await {
                    tracing::error!(
                        target: "plugin_manager",
                        plugin_id = %plugin_id,
                        error = %e,
                        "Plugin shutdown failed"
                    );
                }

                // Execute shutdown hooks
                self.hooks
                    .execute_plugin_unloaded_hooks(&mut context, plugin_id)
                    .await?;
            }

            // Unregister plugin
            self.registry.unregister_plugin(plugin_id).await?;

            // Update stats
            {
                let mut state = self.state.write().await;
                state.stats.total_unloads += 1;
            }

            tracing::info!(
                target: "plugin_manager",
                plugin_id = %plugin_id,
                "Plugin unloaded successfully"
            );
        }

        Ok(())
    }

    /// Execute a plugin
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        input: serde_json::Value,
    ) -> PluginResult<serde_json::Value> {
        let start_time = std::time::Instant::now();

        // Get plugin context
        let context = {
            let state = self.state.read().await;
            state.active_contexts.get(plugin_id).cloned()
        };

        let context = context.ok_or_else(|| PluginError::PluginNotFound {
            name: plugin_id.to_string(),
        })?;

        // Get plugin instance
        let instance = self
            .registry
            .get_plugin_instance(plugin_id)
            .await
            .ok_or_else(|| PluginError::PluginNotFound {
                name: plugin_id.to_string(),
            })?;

        let mut execution_data = TaskExecutionData::new(plugin_id, input);

        // Execute pre-execution hooks
        {
            let mut context = context.write().await;
            self.hooks
                .execute_pre_execution_hooks(&mut context, &mut execution_data)
                .await?;
        }

        // Execute plugin
        let result = {
            let mut plugin = instance.write().await;
            let mut context = context.write().await;

            tokio::time::timeout(self.config.execution_timeout, plugin.execute(&mut context))
                .await
                .map_err(|_| PluginError::execution_error(plugin_id, "Execution timeout"))??
        };

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        // Update execution data
        execution_data = execution_data
            .with_output(result.clone())
            .with_result(true, duration_ms);

        // Execute post-execution hooks
        {
            let mut context = context.write().await;
            self.hooks
                .execute_post_execution_hooks(&mut context, &mut execution_data)
                .await?;
            self.hooks
                .execute_success_hooks(&mut context, &mut execution_data)
                .await?;
        }

        // Update statistics
        {
            let mut state = self.state.write().await;
            state.stats.total_executions += 1;

            // Update average execution time
            let total_time =
                state.stats.avg_execution_time_ms * (state.stats.total_executions - 1) as f64;
            state.stats.avg_execution_time_ms =
                (total_time + duration_ms as f64) / state.stats.total_executions as f64;
        }

        tracing::debug!(
            target: "plugin_manager",
            plugin_id = %plugin_id,
            duration_ms = duration_ms,
            "Plugin execution completed"
        );

        Ok(result)
    }

    /// Get plugin information
    pub async fn get_plugin_info(&self, plugin_id: &str) -> Option<PluginInfo> {
        self.registry.get_plugin_info(plugin_id).await
    }

    /// List all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        self.registry.list_plugins().await
    }

    /// List plugins by type
    pub async fn list_plugins_by_type(&self, plugin_type: &PluginType) -> Vec<PluginInfo> {
        self.registry.list_plugins_by_type(plugin_type).await
    }

    /// List plugins by status
    pub async fn list_plugins_by_status(&self, status: &PluginStatus) -> Vec<PluginInfo> {
        self.registry.list_plugins_by_status(status).await
    }

    /// Register a task hook
    pub async fn register_task_hook(
        &self,
        hook: Arc<dyn TaskHook>,
        plugin_id: &str,
    ) -> PluginResult<Uuid> {
        self.hooks.register_task_hook(hook, plugin_id).await
    }

    /// Register an execution hook
    pub async fn register_execution_hook(
        &self,
        hook: Arc<dyn ExecutionHook>,
        plugin_id: &str,
    ) -> PluginResult<Uuid> {
        self.hooks.register_execution_hook(hook, plugin_id).await
    }

    /// Get manager statistics
    pub async fn get_stats(&self) -> ManagerStats {
        let state = self.state.read().await;
        state.stats.clone()
    }

    /// Get plugin catalog
    pub async fn get_catalog(&self) -> PluginCatalog {
        let catalog = self.catalog.read().await;
        catalog.clone()
    }

    /// Health check for all plugins
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let mut health_status = HashMap::new();
        let plugins = self.list_plugins().await;

        for plugin_info in plugins {
            if let Some(context) = {
                let state = self.state.read().await;
                state.active_contexts.get(&plugin_info.metadata.id).cloned()
            } {
                if let Some(instance) = self
                    .registry
                    .get_plugin_instance(&plugin_info.metadata.id)
                    .await
                {
                    let plugin = instance.read().await;
                    let context = context.read().await;

                    match plugin.health_check(&context).await {
                        Ok(healthy) => {
                            health_status.insert(plugin_info.metadata.id.clone(), healthy);
                        }
                        Err(_) => {
                            health_status.insert(plugin_info.metadata.id.clone(), false);
                        }
                    }
                } else {
                    health_status.insert(plugin_info.metadata.id.clone(), false);
                }
            } else {
                health_status.insert(plugin_info.metadata.id.clone(), false);
            }
        }

        health_status
    }

    /// Update system configuration
    pub async fn update_system_config(
        &self,
        config: ratchet_config::RatchetConfig,
    ) -> PluginResult<()> {
        {
            let mut system_config = self.system_config.write().await;
            *system_config = config.clone();
        }

        // Notify plugins of configuration change
        let state = self.state.read().await;
        for context in state.active_contexts.values() {
            let mut context = context.write().await;
            context.system_config = config.clone();

            // Execute config change hooks
            let config_value = serde_json::to_value(&config).unwrap_or_default();
            self.hooks
                .execute_config_change_hooks(&mut context, &config_value)
                .await?;
        }

        Ok(())
    }
}

/// Plugin manager builder for fluent construction
pub struct PluginManagerBuilder {
    config: PluginManagerConfig,
    system_config: Option<ratchet_config::RatchetConfig>,
}

impl PluginManagerBuilder {
    /// Create a new plugin manager builder
    pub fn new() -> Self {
        Self {
            config: PluginManagerConfig::default(),
            system_config: None,
        }
    }

    /// Set loader configuration
    pub fn with_loader_config(mut self, config: LoaderConfig) -> Self {
        self.config.loader_config = config;
        self
    }

    /// Set discovery configuration
    pub fn with_discovery_config(mut self, config: DiscoveryConfig) -> Self {
        self.config.discovery_config = config;
        self
    }

    /// Set system configuration
    pub fn with_system_config(mut self, config: ratchet_config::RatchetConfig) -> Self {
        self.system_config = Some(config);
        self
    }

    /// Enable auto-discovery
    pub fn with_auto_discover(mut self, enable: bool) -> Self {
        self.config.auto_discover = enable;
        self
    }

    /// Enable auto-loading
    pub fn with_auto_load(mut self, enable: bool) -> Self {
        self.config.auto_load = enable;
        self
    }

    /// Set execution timeout
    pub fn with_execution_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config.execution_timeout = timeout;
        self
    }

    /// Enable hot reload
    pub fn with_hot_reload(mut self, enable: bool) -> Self {
        self.config.enable_hot_reload = enable;
        self
    }

    /// Build the plugin manager
    pub fn build(self) -> PluginManager {
        let system_config = self.system_config.unwrap_or_default();
        PluginManager::new(self.config, system_config)
    }
}

impl Default for PluginManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Plugin, PluginContext, PluginMetadata};
    use crate::types::{PluginType, PluginVersion};
    use async_trait::async_trait;
    use std::any::Any;

    #[allow(dead_code)]
    struct TestPlugin {
        metadata: PluginMetadata,
    }

    impl TestPlugin {
        #[allow(dead_code)]
        fn new() -> Self {
            let metadata = PluginMetadata::new(
                "test-plugin",
                "Test Plugin",
                PluginVersion::new(1, 0, 0),
                "A test plugin",
                "Test Author",
                PluginType::Task,
            );

            Self { metadata }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn execute(
            &mut self,
            _context: &mut PluginContext,
        ) -> PluginResult<serde_json::Value> {
            Ok(serde_json::json!({"status": "success", "message": "Test execution"}))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_builder() {
        let config = LoaderConfig::default();
        let system_config = ratchet_config::RatchetConfig::default();

        let manager = PluginManagerBuilder::new()
            .with_loader_config(config)
            .with_system_config(system_config)
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        assert!(!manager.config.auto_discover);
        assert!(!manager.config.auto_load);
    }

    #[tokio::test]
    async fn test_plugin_manager_initialization() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        // Initialize manager
        assert!(manager.initialize().await.is_ok());

        // Should not initialize twice
        assert!(manager.initialize().await.is_ok());

        // Shutdown
        assert!(manager.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_manager_stats() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_loads, 0);
        assert_eq!(stats.total_executions, 0);
    }

    #[tokio::test]
    async fn test_plugin_manager_health_check() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        let health = manager.health_check().await;
        assert!(health.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_manager_list_operations() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        // List all plugins (should be empty)
        let plugins = manager.list_plugins().await;
        assert!(plugins.is_empty());

        // List by type
        let task_plugins = manager.list_plugins_by_type(&PluginType::Task).await;
        assert!(task_plugins.is_empty());

        // List by status
        let active_plugins = manager.list_plugins_by_status(&PluginStatus::Active).await;
        assert!(active_plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_manager_config_update() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        let new_config = ratchet_config::RatchetConfig::default();
        assert!(manager.update_system_config(new_config).await.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_manager_catalog() {
        let manager = PluginManagerBuilder::new()
            .with_auto_discover(false)
            .with_auto_load(false)
            .build();

        let catalog = manager.get_catalog().await;
        assert!(catalog.list_plugins().is_empty());
    }
}
