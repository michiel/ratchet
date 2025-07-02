//! Configuration management for ratchet-server
//!
//! This module provides comprehensive configuration management for repository
//! operations, security settings, and environment-specific configurations.

pub mod repository_config;
pub mod server_config;

#[cfg(test)]
pub mod tests;

pub use repository_config::*;
pub use server_config::*;

// Re-export types needed for tests
pub use repository_config::{
    RepositoryConfig, ConfigProfile, SyncConfig, SecurityConfig, 
    PerformanceConfig, MonitoringConfig, EnvironmentConfig, 
    ConfigMetadata, AuthType, EncryptionAlgorithm, Permission, UserRole,
    ConfigValidationResult, ConflictResolutionStrategy
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Configuration manager for repository operations
pub struct ConfigManager {
    /// Loaded configurations by repository ID
    configurations: HashMap<i32, RepositoryConfig>,
    /// Global configuration defaults
    global_defaults: RepositoryConfig,
    /// Configuration file paths
    config_paths: Vec<String>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            configurations: HashMap::new(),
            global_defaults: RepositoryConfig::default(),
            config_paths: vec![
                "./config/repositories/".to_string(),
                "/etc/ratchet/repositories/".to_string(),
                "~/.ratchet/repositories/".to_string(),
            ],
        }
    }

    /// Create a configuration manager with custom paths
    pub fn with_paths(paths: Vec<String>) -> Self {
        Self {
            configurations: HashMap::new(),
            global_defaults: RepositoryConfig::default(),
            config_paths: paths,
        }
    }

    /// Load configuration for a repository
    pub async fn load_config(&mut self, repository_id: i32) -> Result<&RepositoryConfig> {
        if !self.configurations.contains_key(&repository_id) {
            let config = self.load_repository_config(repository_id).await?;
            self.configurations.insert(repository_id, config);
        }
        
        Ok(self.configurations.get(&repository_id).unwrap())
    }

    /// Get configuration for a repository (loads if not cached)
    pub async fn get_config(&mut self, repository_id: i32) -> Result<&RepositoryConfig> {
        self.load_config(repository_id).await
    }

    /// Save configuration for a repository
    pub async fn save_config(&mut self, repository_id: i32, config: RepositoryConfig) -> Result<()> {
        // Validate configuration before saving
        let validation = config.validate();
        if !validation.valid {
            return Err(anyhow::anyhow!(
                "Configuration validation failed: {}",
                validation.errors.join(", ")
            ));
        }

        // Save to file
        self.save_repository_config(repository_id, &config).await?;
        
        // Update cache
        self.configurations.insert(repository_id, config);
        
        Ok(())
    }

    /// Update configuration for a repository
    pub async fn update_config<F>(&mut self, repository_id: i32, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut RepositoryConfig),
    {
        let mut config = self.load_config(repository_id).await?.clone();
        update_fn(&mut config);
        self.save_config(repository_id, config).await
    }

    /// Remove configuration for a repository
    pub async fn remove_config(&mut self, repository_id: i32) -> Result<()> {
        // Remove from cache
        self.configurations.remove(&repository_id);
        
        // Remove configuration file
        for base_path in &self.config_paths {
            let config_path = format!("{}/repository-{}.yaml", base_path, repository_id);
            if Path::new(&config_path).exists() {
                fs::remove_file(&config_path).await
                    .context("Failed to remove configuration file")?;
                break;
            }
        }
        
        Ok(())
    }

    /// List all configured repository IDs
    pub async fn list_configured_repositories(&self) -> Result<Vec<i32>> {
        let mut repository_ids = Vec::new();
        
        for base_path in &self.config_paths {
            if let Ok(mut entries) = fs::read_dir(base_path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.starts_with("repository-") && file_name.ends_with(".yaml") {
                            if let Some(id_str) = file_name
                                .strip_prefix("repository-")
                                .and_then(|s| s.strip_suffix(".yaml"))
                            {
                                if let Ok(id) = id_str.parse::<i32>() {
                                    repository_ids.push(id);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        repository_ids.sort();
        repository_ids.dedup();
        Ok(repository_ids)
    }

    /// Set global defaults
    pub fn set_global_defaults(&mut self, defaults: RepositoryConfig) {
        self.global_defaults = defaults;
    }

    /// Get global defaults
    pub fn get_global_defaults(&self) -> &RepositoryConfig {
        &self.global_defaults
    }

    /// Validate all configurations
    pub async fn validate_all_configs(&mut self) -> Result<HashMap<i32, ConfigValidationResult>> {
        let mut results = HashMap::new();
        
        let repository_ids = self.list_configured_repositories().await?;
        for repository_id in repository_ids {
            let config = self.load_config(repository_id).await?;
            let validation = config.validate();
            results.insert(repository_id, validation);
        }
        
        Ok(results)
    }

    /// Create configuration template for a profile
    pub fn create_template(&self, profile: ConfigProfile) -> RepositoryConfig {
        RepositoryConfig::new_with_profile(profile)
    }

    /// Load repository configuration from file
    async fn load_repository_config(&self, repository_id: i32) -> Result<RepositoryConfig> {
        let config_file = format!("repository-{}.yaml", repository_id);
        
        for base_path in &self.config_paths {
            let config_path = format!("{}/{}", base_path, config_file);
            if Path::new(&config_path).exists() {
                let content = fs::read_to_string(&config_path).await
                    .context("Failed to read configuration file")?;
                
                let mut config: RepositoryConfig = serde_yaml::from_str(&content)
                    .context("Failed to parse configuration file")?;
                
                // Apply environment overrides
                let env_vars: HashMap<String, String> = std::env::vars().collect();
                config.apply_environment_overrides(&env_vars);
                
                return Ok(config);
            }
        }
        
        // If no configuration file found, return default with repository ID
        let mut config = self.global_defaults.clone();
        config.repository_id = repository_id;
        config.repository_name = format!("repository-{}", repository_id);
        
        Ok(config)
    }

    /// Save repository configuration to file
    async fn save_repository_config(&self, repository_id: i32, config: &RepositoryConfig) -> Result<()> {
        let config_file = format!("repository-{}.yaml", repository_id);
        let base_path = &self.config_paths[0]; // Use first path for writing
        
        // Ensure directory exists
        fs::create_dir_all(base_path).await
            .context("Failed to create configuration directory")?;
        
        let config_path = format!("{}/{}", base_path, config_file);
        let content = serde_yaml::to_string(config)
            .context("Failed to serialize configuration")?;
        
        fs::write(&config_path, content).await
            .context("Failed to write configuration file")?;
        
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration template generator
pub struct ConfigTemplateGenerator;

impl ConfigTemplateGenerator {
    /// Generate development configuration template
    pub fn development_template() -> RepositoryConfig {
        RepositoryConfig::new_with_profile(ConfigProfile::Development)
    }

    /// Generate staging configuration template
    pub fn staging_template() -> RepositoryConfig {
        RepositoryConfig::new_with_profile(ConfigProfile::Staging)
    }

    /// Generate production configuration template
    pub fn production_template() -> RepositoryConfig {
        RepositoryConfig::new_with_profile(ConfigProfile::Production)
    }

    /// Generate enterprise configuration template
    pub fn enterprise_template() -> RepositoryConfig {
        RepositoryConfig::new_with_profile(ConfigProfile::Enterprise)
    }

    /// Generate configuration template with custom settings
    pub fn custom_template(
        profile: ConfigProfile,
        customizations: HashMap<String, serde_json::Value>,
    ) -> RepositoryConfig {
        let mut config = RepositoryConfig::new_with_profile(profile);
        config.custom = customizations;
        config
    }
}

/// Environment-specific configuration loader
pub struct EnvironmentConfigLoader;

impl EnvironmentConfigLoader {
    /// Load configuration based on environment
    pub async fn load_for_environment(environment: &str) -> Result<RepositoryConfig> {
        let profile = match environment.to_lowercase().as_str() {
            "development" | "dev" => ConfigProfile::Development,
            "staging" | "stage" => ConfigProfile::Staging,
            "production" | "prod" => ConfigProfile::Production,
            "enterprise" | "ent" => ConfigProfile::Enterprise,
            custom => ConfigProfile::Custom(custom.to_string()),
        };

        let mut config = RepositoryConfig::new_with_profile(profile);
        
        // Apply environment variable overrides
        let env_vars: HashMap<String, String> = std::env::vars()
            .filter(|(key, _)| key.starts_with("RATCHET_"))
            .collect();
        
        config.apply_environment_overrides(&env_vars);
        
        Ok(config)
    }

    /// Detect current environment from environment variables
    pub fn detect_environment() -> String {
        std::env::var("ENVIRONMENT")
            .or_else(|_| std::env::var("ENV"))
            .or_else(|_| std::env::var("NODE_ENV"))
            .unwrap_or_else(|_| "development".to_string())
    }
}

