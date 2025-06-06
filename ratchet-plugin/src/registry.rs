//! Plugin registry for managing plugin metadata and lifecycle

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::{Plugin, PluginMetadata};
use crate::error::{PluginError, PluginResult};
use crate::types::{PluginStatus, PluginType};

/// Plugin information stored in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Current plugin status
    pub status: PluginStatus,
    /// Plugin execution context ID
    pub execution_id: Option<Uuid>,
    /// Plugin load time
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Plugin configuration
    pub config: serde_json::Value,
    /// Plugin metrics
    pub metrics: HashMap<String, f64>,
    /// Error information (if status is Failed)
    pub error: Option<String>,
}

impl PluginInfo {
    /// Create new plugin info
    pub fn new(metadata: PluginMetadata, config: serde_json::Value) -> Self {
        Self {
            metadata,
            status: PluginStatus::Unloaded,
            execution_id: None,
            loaded_at: None,
            config,
            metrics: HashMap::new(),
            error: None,
        }
    }

    /// Update plugin status
    pub fn set_status(&mut self, status: PluginStatus) {
        if matches!(status, PluginStatus::Active) {
            self.loaded_at = Some(chrono::Utc::now());
            self.error = None;
        } else if matches!(status, PluginStatus::Failed) && self.error.is_none() {
            self.error = Some("Unknown error".to_string());
        }
        self.status = status;
    }

    /// Set error information
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.status = PluginStatus::Failed;
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: HashMap<String, f64>) {
        self.metrics = metrics;
    }

    /// Get plugin uptime in seconds
    pub fn uptime_seconds(&self) -> Option<f64> {
        self.loaded_at.map(|loaded_at| {
            chrono::Utc::now()
                .signed_duration_since(loaded_at)
                .num_milliseconds() as f64
                / 1000.0
        })
    }
}

/// Plugin registry for managing plugin lifecycle and metadata
pub struct PluginRegistry {
    /// Registered plugins
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
    /// Plugin instances
    instances: Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn Plugin>>>>>>,
    /// Plugin dependency graph
    dependencies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Registry statistics
    stats: Arc<RwLock<RegistryStats>>,
}

/// Registry statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total number of registered plugins
    pub total_plugins: usize,
    /// Number of active plugins
    pub active_plugins: usize,
    /// Number of failed plugins
    pub failed_plugins: usize,
    /// Total number of plugin loads
    pub total_loads: u64,
    /// Total number of plugin unloads
    pub total_unloads: u64,
    /// Number of dependency resolution failures
    pub dependency_failures: u64,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RegistryStats::default())),
        }
    }

    /// Register a plugin
    pub async fn register_plugin(
        &self,
        plugin: Box<dyn Plugin>,
        config: serde_json::Value,
    ) -> PluginResult<()> {
        let metadata = plugin.metadata().clone();
        let plugin_id = metadata.id.clone();

        // Validate plugin metadata
        if plugin_id.is_empty() {
            return Err(PluginError::generic("Plugin ID cannot be empty"));
        }

        // Check if plugin already exists
        {
            let plugins = self.plugins.read().await;
            if plugins.contains_key(&plugin_id) {
                return Err(PluginError::PluginAlreadyExists { name: plugin_id });
            }
        }

        // Validate dependencies
        self.validate_dependencies(&metadata.dependencies).await?;

        // Create plugin info
        let plugin_info = PluginInfo::new(metadata, config);

        // Store plugin
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(plugin_id.clone(), plugin_info);
        }

        {
            let mut instances = self.instances.write().await;
            instances.insert(plugin_id.clone(), Arc::new(RwLock::new(plugin)));
        }

        // Update dependency graph
        {
            let mut dependencies = self.dependencies.write().await;
            let plugin_deps: Vec<String> = self
                .plugins
                .read()
                .await
                .get(&plugin_id)
                .unwrap()
                .metadata
                .dependencies
                .iter()
                .map(|dep| dep.name.clone())
                .collect();
            dependencies.insert(plugin_id.clone(), plugin_deps);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_plugins += 1;
        }

        tracing::info!(
            target: "plugin_registry",
            plugin_id = %plugin_id,
            "Plugin registered"
        );

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, plugin_id: &str) -> PluginResult<bool> {
        // Check if plugin exists
        let exists = {
            let plugins = self.plugins.read().await;
            plugins.contains_key(plugin_id)
        };

        if !exists {
            return Ok(false);
        }

        // Check for dependents
        let dependents = self.get_dependents(plugin_id).await;
        if !dependents.is_empty() {
            return Err(PluginError::DependencyError {
                name: plugin_id.to_string(),
                reason: format!("Plugin has dependents: {}", dependents.join(", ")),
            });
        }

        // Remove plugin
        {
            let mut plugins = self.plugins.write().await;
            plugins.remove(plugin_id);
        }

        {
            let mut instances = self.instances.write().await;
            instances.remove(plugin_id);
        }

        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.remove(plugin_id);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_plugins = stats.total_plugins.saturating_sub(1);
        }

        tracing::info!(
            target: "plugin_registry",
            plugin_id = %plugin_id,
            "Plugin unregistered"
        );

        Ok(true)
    }

    /// Get plugin information
    pub async fn get_plugin_info(&self, plugin_id: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).cloned()
    }

    /// Get plugin instance
    pub async fn get_plugin_instance(
        &self,
        plugin_id: &str,
    ) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        let instances = self.instances.read().await;
        instances.get(plugin_id).cloned()
    }

    /// List all plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().cloned().collect()
    }

    /// List plugins by type
    pub async fn list_plugins_by_type(&self, plugin_type: &PluginType) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|info| &info.metadata.plugin_type == plugin_type)
            .cloned()
            .collect()
    }

    /// List plugins by status
    pub async fn list_plugins_by_status(&self, status: &PluginStatus) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|info| &info.status == status)
            .cloned()
            .collect()
    }

    /// Update plugin status
    pub async fn update_plugin_status(
        &self,
        plugin_id: &str,
        status: PluginStatus,
    ) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin_info) = plugins.get_mut(plugin_id) {
            let old_status = plugin_info.status.clone();
            plugin_info.set_status(status.clone());

            // Update stats
            if old_status != status {
                let mut stats = self.stats.write().await;
                match (old_status, &status) {
                    (PluginStatus::Loading, PluginStatus::Active) => {
                        stats.active_plugins += 1;
                        stats.total_loads += 1;
                    }
                    (PluginStatus::Active, PluginStatus::Failed) => {
                        stats.active_plugins = stats.active_plugins.saturating_sub(1);
                        stats.failed_plugins += 1;
                    }
                    (PluginStatus::Active, PluginStatus::Unloaded) => {
                        stats.active_plugins = stats.active_plugins.saturating_sub(1);
                        stats.total_unloads += 1;
                    }
                    (PluginStatus::Failed, PluginStatus::Active) => {
                        stats.failed_plugins = stats.failed_plugins.saturating_sub(1);
                        stats.active_plugins += 1;
                    }
                    _ => {}
                }
            }

            tracing::debug!(
                target: "plugin_registry",
                plugin_id = %plugin_id,
                status = %status,
                "Plugin status updated"
            );

            Ok(())
        } else {
            Err(PluginError::PluginNotFound {
                name: plugin_id.to_string(),
            })
        }
    }

    /// Update plugin error
    pub async fn update_plugin_error(
        &self,
        plugin_id: &str,
        error: impl Into<String>,
    ) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin_info) = plugins.get_mut(plugin_id) {
            plugin_info.set_error(error);

            tracing::error!(
                target: "plugin_registry",
                plugin_id = %plugin_id,
                error = %plugin_info.error.as_ref().unwrap(),
                "Plugin error recorded"
            );

            Ok(())
        } else {
            Err(PluginError::PluginNotFound {
                name: plugin_id.to_string(),
            })
        }
    }

    /// Update plugin metrics
    pub async fn update_plugin_metrics(
        &self,
        plugin_id: &str,
        metrics: HashMap<String, f64>,
    ) -> PluginResult<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin_info) = plugins.get_mut(plugin_id) {
            plugin_info.update_metrics(metrics);
            Ok(())
        } else {
            Err(PluginError::PluginNotFound {
                name: plugin_id.to_string(),
            })
        }
    }

    /// Get plugin dependencies
    pub async fn get_dependencies(&self, plugin_id: &str) -> Vec<String> {
        let dependencies = self.dependencies.read().await;
        dependencies.get(plugin_id).cloned().unwrap_or_default()
    }

    /// Get plugins that depend on the given plugin
    pub async fn get_dependents(&self, plugin_id: &str) -> Vec<String> {
        let dependencies = self.dependencies.read().await;
        dependencies
            .iter()
            .filter_map(|(id, deps)| {
                if deps.contains(&plugin_id.to_string()) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Resolve plugin load order based on dependencies
    pub async fn resolve_load_order(&self, plugin_ids: &[String]) -> PluginResult<Vec<String>> {
        let dependencies = self.dependencies.read().await;
        let mut resolved = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();

        for plugin_id in plugin_ids {
            if !visited.contains(plugin_id) {
                self.visit_plugin(
                    plugin_id,
                    &dependencies,
                    &mut resolved,
                    &mut visiting,
                    &mut visited,
                )?;
            }
        }

        Ok(resolved)
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> RegistryStats {
        self.stats.read().await.clone()
    }

    /// Validate plugin dependencies
    async fn validate_dependencies(
        &self,
        dependencies: &[crate::types::PluginDependency],
    ) -> PluginResult<()> {
        let plugins = self.plugins.read().await;

        for dep in dependencies {
            if dep.optional {
                continue;
            }

            if let Some(plugin_info) = plugins.get(&dep.name) {
                // Check version compatibility
                if !dep.version.matches(&plugin_info.metadata.version.version) {
                    return Err(PluginError::VersionIncompatible {
                        name: dep.name.clone(),
                        version: plugin_info.metadata.version.to_string(),
                        required: dep.version.to_string(),
                    });
                }
            } else {
                return Err(PluginError::DependencyError {
                    name: dep.name.clone(),
                    reason: "Required dependency not found".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Depth-first search for dependency resolution
    fn visit_plugin(
        &self,
        plugin_id: &str,
        dependencies: &HashMap<String, Vec<String>>,
        resolved: &mut Vec<String>,
        visiting: &mut std::collections::HashSet<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> PluginResult<()> {
        if visiting.contains(plugin_id) {
            return Err(PluginError::DependencyError {
                name: plugin_id.to_string(),
                reason: "Circular dependency detected".to_string(),
            });
        }

        if visited.contains(plugin_id) {
            return Ok(());
        }

        visiting.insert(plugin_id.to_string());

        if let Some(deps) = dependencies.get(plugin_id) {
            for dep in deps {
                self.visit_plugin(dep, dependencies, resolved, visiting, visited)?;
            }
        }

        visiting.remove(plugin_id);
        visited.insert(plugin_id.to_string());
        resolved.push(plugin_id.to_string());

        Ok(())
    }

    /// Check if a plugin is compatible with the system
    pub async fn is_compatible(&self, metadata: &PluginMetadata) -> PluginResult<bool> {
        // Check API version compatibility
        let api_version = semver::Version::parse(&metadata.api_version)
            .map_err(|e| PluginError::generic(format!("Invalid API version: {}", e)))?;

        let min_version = semver::Version::parse(crate::MIN_PLUGIN_API_VERSION)
            .map_err(|e| PluginError::generic(format!("Invalid minimum API version: {}", e)))?;

        if api_version < min_version {
            return Ok(false);
        }

        // Check if all dependencies can be satisfied
        match self.validate_dependencies(&metadata.dependencies).await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false), // Dependencies not satisfied, but plugin could be compatible later
        }
    }

    /// Find plugins by criteria
    pub async fn find_plugins<F>(&self, predicate: F) -> Vec<PluginInfo>
    where
        F: Fn(&PluginInfo) -> bool,
    {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .filter(|info| predicate(info))
            .cloned()
            .collect()
    }

    /// Get plugin count by status
    pub async fn count_by_status(&self) -> HashMap<PluginStatus, usize> {
        let plugins = self.plugins.read().await;
        let mut counts = HashMap::new();

        for plugin_info in plugins.values() {
            *counts.entry(plugin_info.status.clone()).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Plugin, PluginContext, PluginMetadata};
    use crate::types::{PluginDependency, PluginType, PluginVersion};
    use async_trait::async_trait;
    use std::any::Any;

    struct TestPlugin {
        metadata: PluginMetadata,
    }

    impl TestPlugin {
        fn new(id: &str, name: &str) -> Self {
            let metadata = PluginMetadata::new(
                id,
                name,
                PluginVersion::new(1, 0, 0),
                "Test plugin",
                "Test Author",
                PluginType::Task,
            );

            Self { metadata }
        }

        fn with_dependency(mut self, dep_name: &str, version: &str) -> Self {
            let dependency = PluginDependency::new(dep_name, version).unwrap();
            self.metadata = self.metadata.with_dependency(dependency);
            self
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
            Ok(serde_json::json!({"status": "success"}))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));
        let config = serde_json::json!({"enabled": true});

        // Register plugin
        assert!(registry.register_plugin(plugin, config).await.is_ok());

        // Check plugin info
        let plugin_info = registry.get_plugin_info("test-plugin").await.unwrap();
        assert_eq!(plugin_info.metadata.id, "test-plugin");
        assert_eq!(plugin_info.status, PluginStatus::Unloaded);

        // Check stats
        let stats = registry.get_stats().await;
        assert_eq!(stats.total_plugins, 1);
    }

    #[tokio::test]
    async fn test_plugin_duplicate_registration() {
        let registry = PluginRegistry::new();
        let plugin1 = Box::new(TestPlugin::new("test-plugin", "Test Plugin 1"));
        let plugin2 = Box::new(TestPlugin::new("test-plugin", "Test Plugin 2"));
        let config = serde_json::json!({});

        // Register first plugin
        assert!(registry
            .register_plugin(plugin1, config.clone())
            .await
            .is_ok());

        // Try to register duplicate - should fail
        assert!(registry.register_plugin(plugin2, config).await.is_err());
    }

    #[tokio::test]
    async fn test_plugin_dependencies() {
        let registry = PluginRegistry::new();

        // Register dependency first
        let dep_plugin = Box::new(TestPlugin::new("dep-plugin", "Dependency Plugin"));
        assert!(registry
            .register_plugin(dep_plugin, serde_json::json!({}))
            .await
            .is_ok());

        // Register plugin with dependency
        let main_plugin = Box::new(
            TestPlugin::new("main-plugin", "Main Plugin").with_dependency("dep-plugin", "^1.0.0"),
        );
        assert!(registry
            .register_plugin(main_plugin, serde_json::json!({}))
            .await
            .is_ok());

        // Check dependencies
        let deps = registry.get_dependencies("main-plugin").await;
        assert_eq!(deps, vec!["dep-plugin"]);

        let dependents = registry.get_dependents("dep-plugin").await;
        assert_eq!(dependents, vec!["main-plugin"]);
    }

    #[tokio::test]
    async fn test_dependency_load_order() {
        let registry = PluginRegistry::new();

        // Register plugins with dependencies: C -> B -> A
        let plugin_a = Box::new(TestPlugin::new("plugin-a", "Plugin A"));
        let plugin_b =
            Box::new(TestPlugin::new("plugin-b", "Plugin B").with_dependency("plugin-a", "^1.0.0"));
        let plugin_c =
            Box::new(TestPlugin::new("plugin-c", "Plugin C").with_dependency("plugin-b", "^1.0.0"));

        assert!(registry
            .register_plugin(plugin_a, serde_json::json!({}))
            .await
            .is_ok());
        assert!(registry
            .register_plugin(plugin_b, serde_json::json!({}))
            .await
            .is_ok());
        assert!(registry
            .register_plugin(plugin_c, serde_json::json!({}))
            .await
            .is_ok());

        // Resolve load order
        let load_order = registry
            .resolve_load_order(&[
                "plugin-c".to_string(),
                "plugin-a".to_string(),
                "plugin-b".to_string(),
            ])
            .await
            .unwrap();

        // Should be ordered by dependencies: A, B, C
        assert_eq!(load_order, vec!["plugin-a", "plugin-b", "plugin-c"]);
    }

    #[tokio::test]
    async fn test_plugin_status_updates() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("test-plugin", "Test Plugin"));

        assert!(registry
            .register_plugin(plugin, serde_json::json!({}))
            .await
            .is_ok());

        // Update status
        assert!(registry
            .update_plugin_status("test-plugin", PluginStatus::Loading)
            .await
            .is_ok());
        assert!(registry
            .update_plugin_status("test-plugin", PluginStatus::Active)
            .await
            .is_ok());

        let plugin_info = registry.get_plugin_info("test-plugin").await.unwrap();
        assert_eq!(plugin_info.status, PluginStatus::Active);
        assert!(plugin_info.loaded_at.is_some());

        // Check stats
        let stats = registry.get_stats().await;
        assert_eq!(stats.active_plugins, 1);
        assert_eq!(stats.total_loads, 1);
    }

    #[tokio::test]
    async fn test_plugin_filtering() {
        let registry = PluginRegistry::new();

        let plugin1 = Box::new(TestPlugin::new("plugin-1", "Plugin 1"));
        let plugin2_meta = PluginMetadata::new(
            "plugin-2",
            "Plugin 2",
            PluginVersion::new(1, 0, 0),
            "Test plugin",
            "Test Author",
            PluginType::Output,
        );
        let plugin2 = TestPlugin {
            metadata: plugin2_meta,
        };

        assert!(registry
            .register_plugin(plugin1, serde_json::json!({}))
            .await
            .is_ok());
        assert!(registry
            .register_plugin(Box::new(plugin2), serde_json::json!({}))
            .await
            .is_ok());

        // Update statuses
        assert!(registry
            .update_plugin_status("plugin-1", PluginStatus::Active)
            .await
            .is_ok());
        assert!(registry
            .update_plugin_status("plugin-2", PluginStatus::Failed)
            .await
            .is_ok());

        // Filter by type
        let task_plugins = registry.list_plugins_by_type(&PluginType::Task).await;
        assert_eq!(task_plugins.len(), 1);
        assert_eq!(task_plugins[0].metadata.id, "plugin-1");

        // Filter by status
        let active_plugins = registry.list_plugins_by_status(&PluginStatus::Active).await;
        assert_eq!(active_plugins.len(), 1);
        assert_eq!(active_plugins[0].metadata.id, "plugin-1");

        let failed_plugins = registry.list_plugins_by_status(&PluginStatus::Failed).await;
        assert_eq!(failed_plugins.len(), 1);
        assert_eq!(failed_plugins[0].metadata.id, "plugin-2");
    }

    #[tokio::test]
    async fn test_plugin_unregistration_with_dependents() {
        let registry = PluginRegistry::new();

        let dep_plugin = Box::new(TestPlugin::new("dep-plugin", "Dependency Plugin"));
        let main_plugin = Box::new(
            TestPlugin::new("main-plugin", "Main Plugin").with_dependency("dep-plugin", "^1.0.0"),
        );

        assert!(registry
            .register_plugin(dep_plugin, serde_json::json!({}))
            .await
            .is_ok());
        assert!(registry
            .register_plugin(main_plugin, serde_json::json!({}))
            .await
            .is_ok());

        // Try to unregister dependency - should fail because of dependent
        assert!(registry.unregister_plugin("dep-plugin").await.is_err());

        // Unregister dependent first
        assert!(registry.unregister_plugin("main-plugin").await.unwrap());

        // Now should be able to unregister dependency
        assert!(registry.unregister_plugin("dep-plugin").await.unwrap());
    }
}
