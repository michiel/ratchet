//! Plugin loading mechanisms for static and dynamic plugins

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

use crate::core::{Plugin, PluginFactory, PluginManifest};
use crate::error::{PluginError, PluginResult};

/// Plugin loader trait for different loading strategies
#[async_trait]
pub trait PluginLoader: Send + Sync {
    /// Load a plugin from the given source
    async fn load_plugin(&self, source: &str) -> PluginResult<Box<dyn Plugin>>;

    /// Load a plugin manifest from the given source
    async fn load_manifest(&self, source: &str) -> PluginResult<PluginManifest>;

    /// Check if the loader can handle the given source
    fn can_load(&self, source: &str) -> bool;

    /// Get loader name/type
    fn loader_type(&self) -> &'static str;
}

/// Static plugin loader for plugins compiled into the binary
pub struct StaticPluginLoader {
    /// Static plugin factories
    factories: Vec<PluginFactory>,
}

impl StaticPluginLoader {
    /// Create a new static plugin loader
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Add a plugin factory
    pub fn add_factory(&mut self, factory: PluginFactory) {
        self.factories.push(factory);
    }

    /// Load all static plugins
    pub fn load_all_plugins(&self) -> Vec<Box<dyn Plugin>> {
        self.factories.iter().map(|factory| factory()).collect()
    }

    /// Discover static plugins using inventory
    pub fn discover_static_plugins() -> Self {
        let loader = Self::new();
        
        // TODO: Use inventory to collect static plugins when needed
        // The inventory crate pattern would be:
        // for plugin_factory in inventory::iter::<PluginFactory> {
        //     loader.add_factory(plugin_factory.create);
        // }
        // This requires setting up proper inventory submission patterns.
        
        loader
    }
}

impl Default for StaticPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginLoader for StaticPluginLoader {
    async fn load_plugin(&self, source: &str) -> PluginResult<Box<dyn Plugin>> {
        // For static plugins, source is the plugin index or name
        if let Ok(index) = source.parse::<usize>() {
            if index < self.factories.len() {
                Ok(self.factories[index]())
            } else {
                Err(PluginError::PluginNotFound {
                    name: source.to_string(),
                })
            }
        } else {
            // Try to find by plugin name
            for factory in &self.factories {
                let plugin = factory();
                if plugin.metadata().name == source || plugin.metadata().id == source {
                    return Ok(plugin);
                }
            }
            
            Err(PluginError::PluginNotFound {
                name: source.to_string(),
            })
        }
    }

    async fn load_manifest(&self, source: &str) -> PluginResult<PluginManifest> {
        let plugin = self.load_plugin(source).await?;
        let manifest = PluginManifest::new(plugin.metadata().clone());
        Ok(manifest)
    }

    fn can_load(&self, source: &str) -> bool {
        // Can load if source is a valid index or matches a plugin name
        if source.parse::<usize>().is_ok() {
            return true;
        }

        // Check if any plugin matches the name
        for factory in &self.factories {
            let plugin = factory();
            if plugin.metadata().name == source || plugin.metadata().id == source {
                return true;
            }
        }

        false
    }

    fn loader_type(&self) -> &'static str {
        "static"
    }
}

/// Dynamic plugin loader for plugins loaded from shared libraries
pub struct DynamicPluginLoader {
    /// Loaded libraries
    libraries: Arc<tokio::sync::RwLock<Vec<libloading::Library>>>,
}

impl DynamicPluginLoader {
    /// Create a new dynamic plugin loader
    pub fn new() -> Self {
        Self {
            libraries: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Load plugin from shared library file
    async fn load_from_library(&self, path: &Path) -> PluginResult<Box<dyn Plugin>> {
        if !path.exists() {
            return Err(PluginError::PluginFileNotFound {
                path: path.to_string_lossy().to_string(),
            });
        }

        unsafe {
            // Load the library
            let lib = libloading::Library::new(path)?;

            // Get the plugin factory function
            let factory: libloading::Symbol<PluginFactory> = lib
                .get(b"create_plugin")
                .map_err(|e| PluginError::DynamicLoadingError(e.into()))?;

            // Create the plugin
            let plugin = factory();

            // Store the library to keep it loaded
            {
                let mut libraries = self.libraries.write().await;
                libraries.push(lib);
            }

            Ok(plugin)
        }
    }

    /// Load plugin manifest from JSON file
    async fn load_manifest_from_file(&self, path: &Path) -> PluginResult<PluginManifest> {
        let content = tokio::fs::read_to_string(path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }
}

impl Default for DynamicPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginLoader for DynamicPluginLoader {
    async fn load_plugin(&self, source: &str) -> PluginResult<Box<dyn Plugin>> {
        let path = Path::new(source);

        // Check if it's a manifest file
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let manifest = self.load_manifest_from_file(path).await?;
            
            if let Some(entry_point) = manifest.entry_point {
                let lib_path = path.parent().unwrap_or(Path::new(".")).join(entry_point);
                self.load_from_library(&lib_path).await
            } else {
                Err(PluginError::InvalidManifest {
                    reason: "No entry point specified in manifest".to_string(),
                })
            }
        } else {
            // Assume it's a library file
            self.load_from_library(path).await
        }
    }

    async fn load_manifest(&self, source: &str) -> PluginResult<PluginManifest> {
        let path = Path::new(source);

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            self.load_manifest_from_file(path).await
        } else {
            // Load plugin and generate manifest
            let plugin = self.load_plugin(source).await?;
            let manifest = PluginManifest::new(plugin.metadata().clone());
            Ok(manifest)
        }
    }

    fn can_load(&self, source: &str) -> bool {
        let path = Path::new(source);
        
        // Check for supported file extensions
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(ext, "so" | "dll" | "dylib" | "json")
        } else {
            false
        }
    }

    fn loader_type(&self) -> &'static str {
        "dynamic"
    }
}

/// Composite plugin loader that tries multiple loading strategies
pub struct CompositePluginLoader {
    loaders: Vec<Box<dyn PluginLoader>>,
}

impl CompositePluginLoader {
    /// Create a new composite plugin loader
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
        }
    }

    /// Add a plugin loader
    pub fn add_loader(&mut self, loader: Box<dyn PluginLoader>) {
        self.loaders.push(loader);
    }

    /// Create with default loaders
    pub fn with_defaults() -> Self {
        let mut loader = Self::new();
        loader.add_loader(Box::new(StaticPluginLoader::discover_static_plugins()));
        loader.add_loader(Box::new(DynamicPluginLoader::new()));
        loader
    }
}

impl Default for CompositePluginLoader {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[async_trait]
impl PluginLoader for CompositePluginLoader {
    async fn load_plugin(&self, source: &str) -> PluginResult<Box<dyn Plugin>> {
        for loader in &self.loaders {
            if loader.can_load(source) {
                match loader.load_plugin(source).await {
                    Ok(plugin) => return Ok(plugin),
                    Err(e) => {
                        tracing::debug!(
                            target: "plugin_loader",
                            loader_type = loader.loader_type(),
                            source = source,
                            error = %e,
                            "Plugin loading failed with loader, trying next"
                        );
                        continue;
                    }
                }
            }
        }

        Err(PluginError::PluginNotFound {
            name: source.to_string(),
        })
    }

    async fn load_manifest(&self, source: &str) -> PluginResult<PluginManifest> {
        for loader in &self.loaders {
            if loader.can_load(source) {
                match loader.load_manifest(source).await {
                    Ok(manifest) => return Ok(manifest),
                    Err(e) => {
                        tracing::debug!(
                            target: "plugin_loader",
                            loader_type = loader.loader_type(),
                            source = source,
                            error = %e,
                            "Manifest loading failed with loader, trying next"
                        );
                        continue;
                    }
                }
            }
        }

        Err(PluginError::PluginNotFound {
            name: source.to_string(),
        })
    }

    fn can_load(&self, source: &str) -> bool {
        self.loaders.iter().any(|loader| loader.can_load(source))
    }

    fn loader_type(&self) -> &'static str {
        "composite"
    }
}

/// Plugin loading configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Whether to enable static plugin loading
    pub enable_static: bool,
    /// Whether to enable dynamic plugin loading
    pub enable_dynamic: bool,
    /// Plugin search paths for dynamic loading
    pub search_paths: Vec<String>,
    /// Plugin file extensions to search for
    pub extensions: Vec<String>,
    /// Whether to validate plugin signatures
    pub validate_signatures: bool,
    /// Maximum plugin size in bytes
    pub max_plugin_size: usize,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            enable_static: true,
            enable_dynamic: true,
            search_paths: vec![
                "./plugins".to_string(),
                "/usr/local/lib/ratchet/plugins".to_string(),
                "~/.ratchet/plugins".to_string(),
            ],
            extensions: vec![
                "so".to_string(),
                "dll".to_string(),
                "dylib".to_string(),
                "json".to_string(),
            ],
            validate_signatures: false,
            max_plugin_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Configured plugin loader that respects configuration settings
pub struct ConfiguredPluginLoader {
    composite: CompositePluginLoader,
    config: LoaderConfig,
}

impl ConfiguredPluginLoader {
    /// Create a new configured plugin loader
    pub fn new(config: LoaderConfig) -> Self {
        let mut composite = CompositePluginLoader::new();

        if config.enable_static {
            composite.add_loader(Box::new(StaticPluginLoader::discover_static_plugins()));
        }

        if config.enable_dynamic {
            composite.add_loader(Box::new(DynamicPluginLoader::new()));
        }

        Self { composite, config }
    }

    /// Get the loader configuration
    pub fn config(&self) -> &LoaderConfig {
        &self.config
    }

    /// Validate plugin file before loading
    async fn validate_plugin_file(&self, path: &Path) -> PluginResult<()> {
        // Check file size
        let metadata = tokio::fs::metadata(path).await?;
        if metadata.len() > self.config.max_plugin_size as u64 {
            return Err(PluginError::generic(format!(
                "Plugin file too large: {} bytes (max: {})",
                metadata.len(),
                self.config.max_plugin_size
            )));
        }

        // TODO: Add signature validation if enabled
        if self.config.validate_signatures {
            tracing::warn!("Plugin signature validation not yet implemented");
        }

        Ok(())
    }
}

#[async_trait]
impl PluginLoader for ConfiguredPluginLoader {
    async fn load_plugin(&self, source: &str) -> PluginResult<Box<dyn Plugin>> {
        let path = Path::new(source);

        // Validate file if it exists
        if path.exists() {
            self.validate_plugin_file(path).await?;
        }

        self.composite.load_plugin(source).await
    }

    async fn load_manifest(&self, source: &str) -> PluginResult<PluginManifest> {
        self.composite.load_manifest(source).await
    }

    fn can_load(&self, source: &str) -> bool {
        self.composite.can_load(source)
    }

    fn loader_type(&self) -> &'static str {
        "configured"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{PluginContext, PluginMetadata};
    use crate::types::{PluginType, PluginVersion};
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

    fn test_plugin_factory() -> Box<dyn Plugin> {
        Box::new(TestPlugin::new("test-plugin", "Test Plugin"))
    }

    #[tokio::test]
    async fn test_static_plugin_loader() {
        let mut loader = StaticPluginLoader::new();
        loader.add_factory(test_plugin_factory);

        // Test loading by index
        let plugin = loader.load_plugin("0").await.unwrap();
        assert_eq!(plugin.metadata().id, "test-plugin");

        // Test loading by name
        let plugin = loader.load_plugin("Test Plugin").await.unwrap();
        assert_eq!(plugin.metadata().id, "test-plugin");

        // Test loading by ID
        let plugin = loader.load_plugin("test-plugin").await.unwrap();
        assert_eq!(plugin.metadata().id, "test-plugin");

        // Test can_load
        assert!(loader.can_load("0"));
        assert!(loader.can_load("Test Plugin"));
        assert!(loader.can_load("test-plugin"));
        assert!(!loader.can_load("nonexistent"));

        // Test manifest loading
        let manifest = loader.load_manifest("0").await.unwrap();
        assert_eq!(manifest.plugin.id, "test-plugin");
    }

    #[tokio::test]
    async fn test_static_plugin_loader_not_found() {
        let loader = StaticPluginLoader::new();

        // Test loading non-existent plugin
        assert!(loader.load_plugin("0").await.is_err());
        assert!(loader.load_plugin("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_dynamic_plugin_loader_can_load() {
        let loader = DynamicPluginLoader::new();

        // Test can_load with different extensions
        assert!(loader.can_load("plugin.so"));
        assert!(loader.can_load("plugin.dll"));
        assert!(loader.can_load("plugin.dylib"));
        assert!(loader.can_load("plugin.json"));
        assert!(!loader.can_load("plugin.txt"));
        assert!(!loader.can_load("plugin"));
    }

    #[tokio::test]
    async fn test_composite_plugin_loader() {
        let mut composite = CompositePluginLoader::new();
        
        let mut static_loader = StaticPluginLoader::new();
        static_loader.add_factory(test_plugin_factory);
        
        composite.add_loader(Box::new(static_loader));
        composite.add_loader(Box::new(DynamicPluginLoader::new()));

        // Test loading from static loader
        let plugin = composite.load_plugin("0").await.unwrap();
        assert_eq!(plugin.metadata().id, "test-plugin");

        // Test can_load
        assert!(composite.can_load("0")); // Static
        assert!(composite.can_load("plugin.so")); // Dynamic
        assert!(!composite.can_load("invalid"));
    }

    #[tokio::test]
    async fn test_loader_config() {
        let config = LoaderConfig::default();
        
        assert!(config.enable_static);
        assert!(config.enable_dynamic);
        assert!(!config.search_paths.is_empty());
        assert!(!config.extensions.is_empty());
        assert!(!config.validate_signatures);
        assert!(config.max_plugin_size > 0);
    }

    #[tokio::test]
    async fn test_configured_plugin_loader() {
        let config = LoaderConfig::default();
        let loader = ConfiguredPluginLoader::new(config);

        assert_eq!(loader.loader_type(), "configured");
        assert_eq!(loader.config().enable_static, true);
        assert_eq!(loader.config().enable_dynamic, true);
    }

    #[test]
    fn test_loader_types() {
        let static_loader = StaticPluginLoader::new();
        let dynamic_loader = DynamicPluginLoader::new();
        let composite_loader = CompositePluginLoader::new();

        assert_eq!(static_loader.loader_type(), "static");
        assert_eq!(dynamic_loader.loader_type(), "dynamic");
        assert_eq!(composite_loader.loader_type(), "composite");
    }
}