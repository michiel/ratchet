//! Output delivery manager for coordinating multiple destinations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    destinations::{FilesystemDestination, WebhookDestination},
    errors::{ConfigError, DeliveryError, ValidationError},
    metrics::DeliveryMetrics,
    template::TemplateEngine,
    OutputDestinationConfig,
};

/// Result of testing a destination configuration
#[derive(Debug, Clone)]
pub struct TestResult {
    pub index: usize,
    pub destination_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub estimated_time: Duration,
}

/// Manager for handling output delivery to multiple destinations
pub struct OutputDeliveryManager {
    destinations: Arc<RwLock<HashMap<String, Arc<dyn OutputDestination>>>>,
    template_engine: TemplateEngine,
    metrics: DeliveryMetrics,
}

impl OutputDeliveryManager {
    /// Create a new output delivery manager
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(RwLock::new(HashMap::new())),
            template_engine: TemplateEngine::new(),
            metrics: DeliveryMetrics::new(),
        }
    }

    /// Add a destination to the manager
    pub async fn add_destination(
        &self,
        name: String,
        config: OutputDestinationConfig,
    ) -> Result<(), ConfigError> {
        let destination = Self::create_destination_static(config, &self.template_engine)?;
        
        // Validate the destination configuration
        destination.validate_config()
            .map_err(|e| ConfigError::InvalidDestination {
                destination_type: destination.destination_type().to_string(),
                error: e,
            })?;

        let mut destinations = self.destinations.write().await;
        destinations.insert(name.clone(), destination);
        
        info!("Added output destination: {}", name);
        Ok(())
    }

    /// Remove a destination from the manager
    pub async fn remove_destination(&self, name: &str) -> bool {
        let mut destinations = self.destinations.write().await;
        let removed = destinations.remove(name).is_some();
        
        if removed {
            info!("Removed output destination: {}", name);
        } else {
            warn!("Attempted to remove non-existent destination: {}", name);
        }
        
        removed
    }

    /// Deliver output to a specific destination
    pub async fn deliver_output(
        &self,
        destination_name: &str,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<DeliveryResult, DeliveryError> {
        let start_time = Instant::now();
        
        debug!("Delivering output to destination: {}", destination_name);

        let destinations = self.destinations.read().await;
        let destination = destinations.get(destination_name)
            .ok_or_else(|| DeliveryError::Network {
                url: destination_name.to_string(),
                error: "Destination not found".to_string(),
            })?;

        let result = destination.deliver(output, context).await;
        
        // Record metrics
        match &result {
            Ok(delivery_result) => {
                self.metrics.record_success(
                    destination_name,
                    delivery_result.delivery_time,
                    delivery_result.size_bytes,
                );
                info!(
                    "Successfully delivered output to {} in {:?}",
                    destination_name,
                    delivery_result.delivery_time
                );
            }
            Err(e) => {
                self.metrics.record_failure(destination_name, start_time.elapsed());
                error!("Failed to deliver output to {}: {}", destination_name, e);
            }
        }

        result
    }

    /// Deliver output to all configured destinations
    pub async fn deliver_to_all(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Vec<(String, Result<DeliveryResult, DeliveryError>)> {
        let destinations = self.destinations.read().await;
        let destination_names: Vec<String> = destinations.keys().cloned().collect();
        drop(destinations); // Release the read lock

        let mut results = Vec::new();
        
        for name in destination_names {
            let result = self.deliver_output(&name, output, context).await;
            results.push((name, result));
        }

        results
    }

    /// Deliver output to multiple destinations concurrently
    pub async fn deliver_concurrent(
        &self,
        destination_names: &[String],
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Vec<(String, Result<DeliveryResult, DeliveryError>)> {
        let futures: Vec<_> = destination_names
            .iter()
            .map(|name| {
                let name = name.clone();
                let output = output.clone();
                let context = context.clone();
                async move {
                    let result = self.deliver_output(&name, &output, &context).await;
                    (name, result)
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }

    /// Get delivery metrics
    pub fn get_metrics(&self) -> &DeliveryMetrics {
        &self.metrics
    }

    /// List all configured destinations
    pub async fn list_destinations(&self) -> Vec<String> {
        let destinations = self.destinations.read().await;
        destinations.keys().cloned().collect()
    }

    /// Create a delivery manager from destination configurations
    pub fn from_configs(
        configs: &[OutputDestinationConfig],
        max_concurrent: usize,
    ) -> Result<Self, ConfigError> {
        let manager = Self::new();

        for (index, config) in configs.iter().enumerate() {
            let destination = Self::create_destination_static(config.clone(), &manager.template_engine)?;
            destination
                .validate_config()
                .map_err(|e| ConfigError::InvalidDestination {
                    destination_type: destination.destination_type().to_string(),
                    error: e,
                })?;
            
            // Use index as the destination name for now
            let destination_name = format!("destination_{}", index);
            tokio::runtime::Handle::current().block_on(async {
                let mut destinations = manager.destinations.write().await;
                destinations.insert(destination_name, destination);
            });
        }

        Ok(manager)
    }

    /// Test delivery configurations without actually delivering
    pub async fn test_configurations(
        configs: &[OutputDestinationConfig],
    ) -> Result<Vec<TestResult>, ConfigError> {
        let mut results = Vec::new();
        let template_engine = TemplateEngine::new();

        for (idx, config) in configs.iter().enumerate() {
            match Self::create_destination_static(config.clone(), &template_engine) {
                Ok(destination) => match destination.validate_config() {
                    Ok(()) => {
                        results.push(TestResult {
                            index: idx,
                            destination_type: destination.destination_type().to_string(),
                            success: true,
                            error: None,
                            estimated_time: destination.estimated_delivery_time(),
                        });
                    }
                    Err(e) => {
                        results.push(TestResult {
                            index: idx,
                            destination_type: destination.destination_type().to_string(),
                            success: false,
                            error: Some(e.to_string()),
                            estimated_time: Duration::ZERO,
                        });
                    }
                },
                Err(e) => {
                    results.push(TestResult {
                        index: idx,
                        destination_type: "unknown".to_string(),
                        success: false,
                        error: Some(e.to_string()),
                        estimated_time: Duration::ZERO,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Create a destination from configuration (static version)
    fn create_destination_static(
        config: OutputDestinationConfig,
        template_engine: &TemplateEngine,
    ) -> Result<Arc<dyn OutputDestination>, ConfigError> {
        match config {
            OutputDestinationConfig::Filesystem {
                path,
                format,
                permissions,
                create_dirs,
                overwrite,
                backup_existing,
            } => {
                let fs_config = crate::destinations::filesystem::FilesystemConfig {
                    path_template: path,
                    format,
                    permissions,
                    create_dirs,
                    overwrite,
                    backup_existing,
                };
                
                Ok(Arc::new(FilesystemDestination::new(
                    fs_config,
                    template_engine.clone(),
                )))
            }
            OutputDestinationConfig::Webhook {
                url,
                method,
                headers,
                timeout,
                retry_policy,
                auth,
                content_type,
            } => {
                let webhook_config = crate::destinations::webhook::WebhookConfig {
                    url_template: url,
                    method,
                    headers,
                    timeout,
                    retry_policy,
                    auth,
                    content_type,
                };

                let client = reqwest::Client::builder()
                    .timeout(timeout)
                    .build()
                    .map_err(|e| ConfigError::HttpClientCreate { source: e })?;

                Ok(Arc::new(WebhookDestination::new(
                    webhook_config,
                    client,
                    template_engine.clone(),
                )))
            }
            OutputDestinationConfig::Database { .. } => {
                Err(ConfigError::UnsupportedDestination("database".to_string()))
            }
            OutputDestinationConfig::S3 { .. } => {
                Err(ConfigError::UnsupportedDestination("s3".to_string()))
            }
        }
    }

    /// Create a destination from configuration (instance method)
    async fn create_destination(
        &self,
        config: OutputDestinationConfig,
    ) -> Result<Arc<dyn OutputDestination>, ConfigError> {
        Self::create_destination_static(config, &self.template_engine)
    }
}

impl Default for OutputDeliveryManager {
    fn default() -> Self {
        Self::new()
    }
}