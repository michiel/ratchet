//! Core plugin trait and context definitions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{PluginError, PluginResult};
use crate::types::{PluginCapabilities, PluginDependency, PluginStatus, PluginType, PluginVersion};

/// Plugin metadata containing information about the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin unique identifier
    pub id: String,
    /// Plugin display name
    pub name: String,
    /// Plugin version
    pub version: PluginVersion,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Plugin API version this plugin was built for
    pub api_version: String,
    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// Plugin capabilities
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    /// Plugin homepage URL
    pub homepage: Option<String>,
    /// Plugin repository URL
    pub repository: Option<String>,
    /// Plugin license
    pub license: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PluginMetadata {
    /// Create a new plugin metadata
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: PluginVersion,
        description: impl Into<String>,
        author: impl Into<String>,
        plugin_type: PluginType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version,
            description: description.into(),
            author: author.into(),
            plugin_type,
            api_version: crate::PLUGIN_SYSTEM_VERSION.to_string(),
            dependencies: Vec::new(),
            capabilities: PluginCapabilities::default(),
            homepage: None,
            repository: None,
            license: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a dependency to this plugin
    pub fn with_dependency(mut self, dependency: PluginDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Set plugin capabilities
    pub fn with_capabilities(mut self, capabilities: PluginCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Plugin execution context providing access to system resources
#[derive(Debug)]
pub struct PluginContext {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// Plugin configuration
    pub config: serde_json::Value,
    /// Shared data between plugins
    pub shared_data: HashMap<String, Box<dyn Any + Send + Sync>>,
    /// System configuration
    pub system_config: ratchet_config::RatchetConfig,
    /// Current plugin status
    pub status: PluginStatus,
    /// Plugin-specific logger
    pub logger: tracing::Span,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(execution_id: Uuid, config: serde_json::Value, system_config: ratchet_config::RatchetConfig) -> Self {
        Self {
            execution_id,
            config,
            shared_data: HashMap::new(),
            system_config,
            status: PluginStatus::Loading,
            logger: tracing::span!(tracing::Level::INFO, "plugin", id = execution_id.to_string()),
        }
    }

    /// Get plugin configuration as a specific type
    pub fn config_as<T>(&self) -> PluginResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.config.clone()).map_err(PluginError::from)
    }

    /// Store shared data
    pub fn set_shared_data<T>(&mut self, key: impl Into<String>, value: T)
    where
        T: Any + Send + Sync,
    {
        self.shared_data.insert(key.into(), Box::new(value));
    }

    /// Get shared data
    pub fn get_shared_data<T>(&self, key: &str) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.shared_data.get(key).and_then(|data| data.downcast_ref::<T>())
    }

    /// Update plugin status
    pub fn set_status(&mut self, status: PluginStatus) {
        tracing::info!(target: "plugin", status = %status, "Plugin status changed");
        self.status = status;
    }
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        tracing::info!(
            target: "plugin",
            plugin = %self.metadata().name,
            "Plugin initialized"
        );
        context.set_status(PluginStatus::Active);
        Ok(())
    }

    /// Execute the plugin's main functionality
    async fn execute(&mut self, context: &mut PluginContext) -> PluginResult<serde_json::Value>;

    /// Shutdown the plugin
    async fn shutdown(&mut self, context: &mut PluginContext) -> PluginResult<()> {
        tracing::info!(
            target: "plugin",
            plugin = %self.metadata().name,
            "Plugin shutdown"
        );
        context.set_status(PluginStatus::Unloaded);
        Ok(())
    }

    /// Validate plugin configuration
    fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        // Default implementation does no validation
        let _ = config;
        Ok(())
    }

    /// Get plugin configuration schema (JSON Schema)
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Handle plugin errors
    async fn handle_error(&mut self, error: &PluginError, context: &mut PluginContext) -> PluginResult<()> {
        tracing::error!(
            target: "plugin",
            plugin = %self.metadata().name,
            error = %error,
            "Plugin error occurred"
        );
        context.set_status(PluginStatus::Failed);
        Ok(())
    }

    /// Plugin health check
    async fn health_check(&self, context: &PluginContext) -> PluginResult<bool> {
        let _ = context;
        Ok(matches!(context.status, PluginStatus::Active))
    }

    /// Get plugin metrics
    async fn metrics(&self, context: &PluginContext) -> PluginResult<HashMap<String, f64>> {
        let _ = context;
        Ok(HashMap::new())
    }

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin factory function type for dynamic loading
pub type PluginFactory = fn() -> Box<dyn Plugin>;

/// Plugin manifest for describing plugin packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMetadata,
    /// Entry point for the plugin (for dynamic loading)
    pub entry_point: Option<String>,
    /// Plugin files
    #[serde(default)]
    pub files: Vec<String>,
    /// Build information
    pub build: Option<PluginBuildInfo>,
    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
}

/// Plugin build information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBuildInfo {
    /// Rust version used to build the plugin
    pub rust_version: String,
    /// Target architecture
    pub target: String,
    /// Build timestamp
    pub timestamp: String,
    /// Git commit hash (if available)
    pub git_hash: Option<String>,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(plugin: PluginMetadata) -> Self {
        Self {
            plugin,
            entry_point: None,
            files: Vec::new(),
            build: None,
            config_schema: None,
        }
    }

    /// Set entry point for dynamic loading
    pub fn with_entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.entry_point = Some(entry_point.into());
        self
    }

    /// Add file to the plugin package
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.files.push(file.into());
        self
    }

    /// Set build information
    pub fn with_build_info(mut self, build: PluginBuildInfo) -> Self {
        self.build = Some(build);
        self
    }

    /// Validate the manifest
    pub fn validate(&self) -> PluginResult<()> {
        if self.plugin.id.is_empty() {
            return Err(PluginError::generic("Plugin ID cannot be empty"));
        }

        if self.plugin.name.is_empty() {
            return Err(PluginError::generic("Plugin name cannot be empty"));
        }

        // Validate API version compatibility
        let api_version = semver::Version::parse(&self.plugin.api_version)
            .map_err(|e| PluginError::generic(format!("Invalid API version: {}", e)))?;

        let min_version = semver::Version::parse(crate::MIN_PLUGIN_API_VERSION)
            .map_err(|e| PluginError::generic(format!("Invalid minimum API version: {}", e)))?;

        if api_version < min_version {
            return Err(PluginError::ApiVersionIncompatible {
                name: self.plugin.name.clone(),
                api_version: self.plugin.api_version.clone(),
                system_version: crate::PLUGIN_SYSTEM_VERSION.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginVersion;

    struct TestPlugin {
        metadata: PluginMetadata,
    }

    impl TestPlugin {
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

        async fn execute(&mut self, _context: &mut PluginContext) -> PluginResult<serde_json::Value> {
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
    async fn test_plugin_basic_lifecycle() {
        let mut plugin = TestPlugin::new();
        let mut context = PluginContext::new(
            Uuid::new_v4(),
            serde_json::json!({}),
            ratchet_config::RatchetConfig::default(),
        );

        // Initialize
        assert!(plugin.initialize(&mut context).await.is_ok());
        assert_eq!(context.status, PluginStatus::Active);

        // Execute
        let result = plugin.execute(&mut context).await.unwrap();
        assert_eq!(result["status"], "success");

        // Health check
        assert!(plugin.health_check(&context).await.unwrap());

        // Shutdown
        assert!(plugin.shutdown(&mut context).await.is_ok());
        assert_eq!(context.status, PluginStatus::Unloaded);
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata::new(
            "test-id",
            "Test Plugin",
            PluginVersion::new(1, 2, 3),
            "Test description",
            "Test Author",
            PluginType::Custom("test".to_string()),
        );

        assert_eq!(metadata.id, "test-id");
        assert_eq!(metadata.name, "Test Plugin");
        assert_eq!(metadata.version.version.major, 1);
        assert_eq!(metadata.author, "Test Author");
    }

    #[test]
    fn test_plugin_context() {
        let execution_id = Uuid::new_v4();
        let config = serde_json::json!({"test": "value"});
        let system_config = ratchet_config::RatchetConfig::default();

        let mut context = PluginContext::new(execution_id, config, system_config);

        // Test shared data
        context.set_shared_data("test_key", "test_value".to_string());
        let value: Option<&String> = context.get_shared_data("test_key");
        assert_eq!(value, Some(&"test_value".to_string()));

        // Test status
        context.set_status(PluginStatus::Active);
        assert_eq!(context.status, PluginStatus::Active);
    }

    #[test]
    fn test_plugin_manifest_validation() {
        let metadata = PluginMetadata::new(
            "test-plugin",
            "Test Plugin",
            PluginVersion::new(1, 0, 0),
            "Test description",
            "Test Author",
            PluginType::Task,
        );

        let manifest = PluginManifest::new(metadata);
        assert!(manifest.validate().is_ok());

        // Test invalid manifest
        let mut invalid_metadata = PluginMetadata::new(
            "",
            "Test Plugin",
            PluginVersion::new(1, 0, 0),
            "Test description",
            "Test Author",
            PluginType::Task,
        );
        invalid_metadata.id = String::new();

        let invalid_manifest = PluginManifest::new(invalid_metadata);
        assert!(invalid_manifest.validate().is_err());
    }
}
