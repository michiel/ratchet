//! Plugin discovery mechanisms for finding and cataloging plugins

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::core::PluginManifest;
use crate::error::{PluginError, PluginResult};
use crate::loader::LoaderConfig;

/// Discovered plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPlugin {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Plugin source path
    pub source_path: PathBuf,
    /// Discovery method used
    pub discovery_method: String,
    /// Discovery timestamp
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    /// Plugin size in bytes
    pub size_bytes: u64,
    /// File checksums for integrity verification
    pub checksums: HashMap<String, String>,
}

impl DiscoveredPlugin {
    /// Create a new discovered plugin
    pub fn new(manifest: PluginManifest, source_path: PathBuf, discovery_method: impl Into<String>) -> Self {
        Self {
            manifest,
            source_path,
            discovery_method: discovery_method.into(),
            discovered_at: chrono::Utc::now(),
            size_bytes: 0,
            checksums: HashMap::new(),
        }
    }

    /// Set plugin size
    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = size_bytes;
        self
    }

    /// Add checksum
    pub fn with_checksum(mut self, algorithm: impl Into<String>, checksum: impl Into<String>) -> Self {
        self.checksums.insert(algorithm.into(), checksum.into());
        self
    }

    /// Get plugin unique identifier for discovery
    pub fn discovery_id(&self) -> String {
        format!("{}:{}", self.manifest.plugin.id, self.manifest.plugin.version)
    }
}

/// Plugin discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Paths to search for plugins
    pub search_paths: Vec<PathBuf>,
    /// File patterns to match
    pub file_patterns: Vec<String>,
    /// Whether to search recursively
    pub recursive: bool,
    /// Maximum search depth
    pub max_depth: Option<usize>,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Whether to validate plugin files during discovery
    pub validate_during_discovery: bool,
    /// Whether to calculate checksums
    pub calculate_checksums: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("./plugins"),
                PathBuf::from("/usr/local/lib/ratchet/plugins"),
                PathBuf::from("/opt/ratchet/plugins"),
            ],
            file_patterns: vec![
                "*.so".to_string(),
                "*.dll".to_string(),
                "*.dylib".to_string(),
                "*.json".to_string(),
                "plugin.toml".to_string(),
                "manifest.json".to_string(),
            ],
            recursive: true,
            max_depth: Some(3),
            follow_symlinks: false,
            validate_during_discovery: true,
            calculate_checksums: false,
        }
    }
}

/// Plugin discovery service
pub struct PluginDiscovery {
    config: DiscoveryConfig,
    loader_config: LoaderConfig,
}

impl PluginDiscovery {
    /// Create a new plugin discovery service
    pub fn new(config: DiscoveryConfig, loader_config: LoaderConfig) -> Self {
        Self { config, loader_config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(DiscoveryConfig::default(), LoaderConfig::default())
    }

    /// Discover plugins in configured search paths
    pub async fn discover_plugins(&self) -> PluginResult<Vec<DiscoveredPlugin>> {
        let mut discovered = Vec::new();

        for search_path in &self.config.search_paths {
            if !search_path.exists() {
                tracing::debug!(
                    target: "plugin_discovery",
                    path = ?search_path,
                    "Search path does not exist, skipping"
                );
                continue;
            }

            let plugins = self.discover_in_path(search_path).await?;
            discovered.extend(plugins);
        }

        // Remove duplicates based on plugin ID and version
        self.deduplicate_plugins(discovered)
    }

    /// Discover plugins in a specific path
    pub async fn discover_in_path(&self, path: &Path) -> PluginResult<Vec<DiscoveredPlugin>> {
        let mut discovered = Vec::new();

        tracing::info!(
            target: "plugin_discovery",
            path = ?path,
            "Starting plugin discovery"
        );

        let walker = if self.config.recursive {
            let mut walker = WalkDir::new(path);

            if let Some(max_depth) = self.config.max_depth {
                walker = walker.max_depth(max_depth);
            }

            if !self.config.follow_symlinks {
                walker = walker.follow_links(false);
            }

            walker
        } else {
            WalkDir::new(path).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if !entry.file_type().is_file() {
                continue;
            }

            if self.should_process_file(path) {
                match self.discover_plugin_from_file(path).await {
                    Ok(Some(plugin)) => discovered.push(plugin),
                    Ok(None) => {
                        tracing::debug!(
                            target: "plugin_discovery",
                            file = ?path,
                            "File did not contain a valid plugin"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "plugin_discovery",
                            file = ?path,
                            error = %e,
                            "Failed to discover plugin from file"
                        );
                    }
                }
            }
        }

        tracing::info!(
            target: "plugin_discovery",
            path = ?path,
            count = discovered.len(),
            "Plugin discovery completed"
        );

        Ok(discovered)
    }

    /// Discover a plugin from a specific file
    async fn discover_plugin_from_file(&self, path: &Path) -> PluginResult<Option<DiscoveredPlugin>> {
        // Get file metadata
        let metadata = tokio::fs::metadata(path).await?;
        let size_bytes = metadata.len();

        // Check file size against limits
        if size_bytes > self.loader_config.max_plugin_size as u64 {
            return Err(PluginError::generic(format!(
                "Plugin file too large: {} bytes (max: {})",
                size_bytes, self.loader_config.max_plugin_size
            )));
        }

        // Try to load manifest
        let manifest = match self.load_manifest_from_file(path).await {
            Ok(manifest) => manifest,
            Err(_) => {
                // If direct manifest loading fails, check if it's a library file
                if self.is_library_file(path) {
                    // For library files, we can't get metadata without loading
                    return Ok(None);
                } else {
                    return Ok(None);
                }
            }
        };

        // Validate manifest if required
        if self.config.validate_during_discovery {
            manifest.validate()?;
        }

        let mut discovered = DiscoveredPlugin::new(manifest, path.to_path_buf(), "filesystem").with_size(size_bytes);

        // Calculate checksums if required
        if self.config.calculate_checksums {
            let checksum = self.calculate_file_checksum(path).await?;
            discovered = discovered.with_checksum("sha256", checksum);
        }

        Ok(Some(discovered))
    }

    /// Load manifest from file
    async fn load_manifest_from_file(&self, path: &Path) -> PluginResult<PluginManifest> {
        let content = tokio::fs::read_to_string(path).await?;

        // Try different manifest formats
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let manifest: PluginManifest = serde_json::from_str(&content)?;
            Ok(manifest)
        } else if path.file_name().and_then(|s| s.to_str()) == Some("plugin.toml") {
            // TODO: Add TOML support
            Err(PluginError::generic("TOML manifests not yet supported"))
        } else {
            Err(PluginError::InvalidManifest {
                reason: "Unknown manifest format".to_string(),
            })
        }
    }

    /// Check if file should be processed
    fn should_process_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Check against file patterns
        for pattern in &self.config.file_patterns {
            if self.matches_pattern(file_name, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, file_name: &str, pattern: &str) -> bool {
        if pattern == file_name {
            return true;
        }

        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return file_name.starts_with(prefix) && file_name.ends_with(suffix);
            }
        }

        false
    }

    /// Check if file is a library file
    fn is_library_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(ext, "so" | "dll" | "dylib")
        } else {
            false
        }
    }

    /// Calculate file checksum
    async fn calculate_file_checksum(&self, path: &Path) -> PluginResult<String> {
        use sha2::{Digest, Sha256};

        let content = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Remove duplicate plugins
    fn deduplicate_plugins(&self, plugins: Vec<DiscoveredPlugin>) -> PluginResult<Vec<DiscoveredPlugin>> {
        let mut unique_plugins: HashMap<String, DiscoveredPlugin> = HashMap::new();
        let mut duplicates = Vec::new();

        for plugin in plugins {
            let key = plugin.discovery_id();

            if let Some(existing) = unique_plugins.get(&key) {
                duplicates.push((plugin.clone(), existing.clone()));
            }

            unique_plugins.insert(key, plugin);
        }

        // Log duplicates
        for (duplicate, original) in duplicates {
            tracing::warn!(
                target: "plugin_discovery",
                plugin_id = %duplicate.manifest.plugin.id,
                version = %duplicate.manifest.plugin.version,
                duplicate_path = ?duplicate.source_path,
                original_path = ?original.source_path,
                "Duplicate plugin discovered, keeping first found"
            );
        }

        Ok(unique_plugins.into_values().collect())
    }

    /// Watch for plugin changes (requires file system watching)
    pub async fn watch_for_changes<F>(&self, callback: F) -> PluginResult<()>
    where
        F: Fn(PluginDiscoveryEvent) + Send + Sync + 'static,
    {
        // TODO: Implement file system watching
        // This would use a crate like `notify` to watch for file system changes
        let _ = callback;
        Err(PluginError::generic("Plugin watching not yet implemented"))
    }

    /// Get discovery configuration
    pub fn config(&self) -> &DiscoveryConfig {
        &self.config
    }
}

/// Plugin discovery events
#[derive(Debug, Clone)]
pub enum PluginDiscoveryEvent {
    /// Plugin discovered
    PluginDiscovered(DiscoveredPlugin),
    /// Plugin removed
    PluginRemoved(PathBuf),
    /// Plugin modified
    PluginModified(DiscoveredPlugin),
    /// Discovery error
    DiscoveryError { path: PathBuf, error: String },
}

/// Plugin catalog for managing discovered plugins
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCatalog {
    /// Cataloged plugins
    plugins: HashMap<String, DiscoveredPlugin>,
    /// Catalog metadata
    metadata: CatalogMetadata,
}

/// Catalog metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetadata {
    /// Catalog creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update time
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Discovery configuration used
    #[serde(skip)]
    pub discovery_config: DiscoveryConfig,
    /// Total plugins discovered
    pub total_plugins: usize,
}

impl Default for CatalogMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            discovery_config: DiscoveryConfig::default(),
            total_plugins: 0,
        }
    }
}

impl PluginCatalog {
    /// Create a new plugin catalog
    pub fn new() -> Self {
        Self::default()
    }

    /// Add discovered plugins to the catalog
    pub fn add_plugins(&mut self, plugins: Vec<DiscoveredPlugin>) {
        for plugin in plugins {
            let key = plugin.discovery_id();
            self.plugins.insert(key, plugin);
        }

        self.metadata.updated_at = chrono::Utc::now();
        self.metadata.total_plugins = self.plugins.len();
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&DiscoveredPlugin> {
        self.plugins.values().find(|p| p.manifest.plugin.id == plugin_id)
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<&DiscoveredPlugin> {
        self.plugins.values().collect()
    }

    /// Search plugins by criteria
    pub fn search_plugins<F>(&self, predicate: F) -> Vec<&DiscoveredPlugin>
    where
        F: Fn(&DiscoveredPlugin) -> bool,
    {
        self.plugins.values().filter(|p| predicate(p)).collect()
    }

    /// Get catalog statistics
    pub fn stats(&self) -> CatalogStats {
        let mut stats = CatalogStats::default();

        for plugin in self.plugins.values() {
            stats.total_plugins += 1;
            stats.total_size_bytes += plugin.size_bytes;

            let plugin_type = &plugin.manifest.plugin.plugin_type;
            *stats.plugins_by_type.entry(plugin_type.to_string()).or_insert(0) += 1;
        }

        stats
    }

    /// Get catalog metadata
    pub fn metadata(&self) -> &CatalogMetadata {
        &self.metadata
    }

    /// Save catalog to file
    pub async fn save_to_file(&self, path: &Path) -> PluginResult<()> {
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Load catalog from file
    pub async fn load_from_file(path: &Path) -> PluginResult<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let catalog: PluginCatalog = serde_json::from_str(&content)?;
        Ok(catalog)
    }
}

/// Catalog statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogStats {
    /// Total number of plugins
    pub total_plugins: usize,
    /// Total size of all plugins in bytes
    pub total_size_bytes: u64,
    /// Number of plugins by type
    pub plugins_by_type: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PluginMetadata;
    use crate::types::{PluginType, PluginVersion};
    use tempfile::tempdir;

    fn create_test_manifest() -> PluginManifest {
        let metadata = PluginMetadata::new(
            "test-plugin",
            "Test Plugin",
            PluginVersion::new(1, 0, 0),
            "A test plugin",
            "Test Author",
            PluginType::Task,
        );
        PluginManifest::new(metadata)
    }

    #[tokio::test]
    async fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();

        assert!(!config.search_paths.is_empty());
        assert!(!config.file_patterns.is_empty());
        assert!(config.recursive);
        assert!(config.max_depth.is_some());
        assert!(!config.follow_symlinks);
        assert!(config.validate_during_discovery);
        assert!(!config.calculate_checksums);
    }

    #[tokio::test]
    async fn test_discovered_plugin() {
        let manifest = create_test_manifest();
        let path = PathBuf::from("/test/plugin.so");

        let discovered = DiscoveredPlugin::new(manifest, path.clone(), "test")
            .with_size(1024)
            .with_checksum("sha256", "abc123");

        assert_eq!(discovered.source_path, path);
        assert_eq!(discovered.size_bytes, 1024);
        assert_eq!(discovered.checksums.get("sha256"), Some(&"abc123".to_string()));
        assert_eq!(discovered.discovery_id(), "test-plugin:1.0.0");
    }

    #[tokio::test]
    async fn test_plugin_discovery_empty_path() {
        let discovery = PluginDiscovery::with_defaults();
        let temp_dir = tempdir().unwrap();

        // Empty directory should return no plugins
        let plugins = discovery.discover_in_path(temp_dir.path()).await.unwrap();
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_discovery_with_manifest() {
        let discovery = PluginDiscovery::with_defaults();
        let temp_dir = tempdir().unwrap();

        // Create a test manifest file
        let manifest = create_test_manifest();
        let manifest_path = temp_dir.path().join("plugin.json");
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        tokio::fs::write(&manifest_path, manifest_json).await.unwrap();

        // Discover plugins
        let plugins = discovery.discover_in_path(temp_dir.path()).await.unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.plugin.id, "test-plugin");
    }

    #[test]
    fn test_pattern_matching() {
        let discovery = PluginDiscovery::with_defaults();

        assert!(discovery.matches_pattern("plugin.so", "*.so"));
        assert!(discovery.matches_pattern("test.dll", "*.dll"));
        assert!(discovery.matches_pattern("manifest.json", "manifest.json"));
        assert!(!discovery.matches_pattern("readme.txt", "*.so"));
    }

    #[test]
    fn test_is_library_file() {
        let discovery = PluginDiscovery::with_defaults();

        assert!(discovery.is_library_file(Path::new("plugin.so")));
        assert!(discovery.is_library_file(Path::new("plugin.dll")));
        assert!(discovery.is_library_file(Path::new("plugin.dylib")));
        assert!(!discovery.is_library_file(Path::new("plugin.json")));
        assert!(!discovery.is_library_file(Path::new("plugin.txt")));
    }

    #[tokio::test]
    async fn test_plugin_catalog() {
        let mut catalog = PluginCatalog::new();

        // Add plugins
        let manifest1 = create_test_manifest();
        let plugin1 = DiscoveredPlugin::new(manifest1, PathBuf::from("/test/plugin1.so"), "test").with_size(1024);

        let mut manifest2 = create_test_manifest();
        manifest2.plugin.id = "test-plugin-2".to_string();
        let plugin2 = DiscoveredPlugin::new(manifest2, PathBuf::from("/test/plugin2.so"), "test").with_size(2048);

        catalog.add_plugins(vec![plugin1, plugin2]);

        // Test catalog functions
        assert_eq!(catalog.list_plugins().len(), 2);
        assert!(catalog.get_plugin("test-plugin").is_some());
        assert!(catalog.get_plugin("test-plugin-2").is_some());

        let stats = catalog.stats();
        assert_eq!(stats.total_plugins, 2);
        assert_eq!(stats.total_size_bytes, 3072);
    }

    #[tokio::test]
    async fn test_catalog_search() {
        let mut catalog = PluginCatalog::new();

        let manifest = create_test_manifest();
        let plugin = DiscoveredPlugin::new(manifest, PathBuf::from("/test/plugin.so"), "test").with_size(1024);

        catalog.add_plugins(vec![plugin]);

        // Search by plugin type
        let task_plugins = catalog.search_plugins(|p| matches!(p.manifest.plugin.plugin_type, PluginType::Task));
        assert_eq!(task_plugins.len(), 1);

        // Search by size
        let large_plugins = catalog.search_plugins(|p| p.size_bytes > 500);
        assert_eq!(large_plugins.len(), 1);

        let small_plugins = catalog.search_plugins(|p| p.size_bytes < 500);
        assert_eq!(small_plugins.len(), 0);
    }

    #[tokio::test]
    async fn test_deduplicate_plugins() {
        let discovery = PluginDiscovery::with_defaults();

        // Create duplicate plugins (same ID and version)
        let manifest = create_test_manifest();
        let plugin1 = DiscoveredPlugin::new(manifest.clone(), PathBuf::from("/test/plugin1.so"), "test");
        let plugin2 = DiscoveredPlugin::new(manifest, PathBuf::from("/test/plugin2.so"), "test");

        let plugins = vec![plugin1, plugin2];
        let deduplicated = discovery.deduplicate_plugins(plugins).unwrap();

        // Should only have one plugin after deduplication
        assert_eq!(deduplicated.len(), 1);
    }
}
