//! Output delivery manager for coordinating delivery to multiple destinations

use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use crate::output::destinations::filesystem::FilesystemConfig;
use crate::output::destinations::webhook::WebhookConfig;
use crate::output::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    destinations::{FilesystemDestination, WebhookDestination},
    errors::{ConfigError, DeliveryError},
    metrics::DeliveryMetrics,
    template::TemplateEngine,
    JobContext, OutputDestinationConfig,
};

/// Manages delivery to multiple output destinations
pub struct OutputDeliveryManager {
    destinations: Vec<Arc<dyn OutputDestination>>,
    template_engine: TemplateEngine,
    metrics: Arc<DeliveryMetrics>,
    max_concurrent_deliveries: usize,
}

impl OutputDeliveryManager {
    /// Create a new delivery manager
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            destinations: Vec::new(),
            template_engine: TemplateEngine::new(),
            metrics: Arc::new(DeliveryMetrics::new()),
            max_concurrent_deliveries: max_concurrent,
        }
    }

    /// Create delivery manager from destination configurations
    pub fn from_configs(
        configs: &[OutputDestinationConfig],
        max_concurrent: usize,
    ) -> Result<Self, ConfigError> {
        let mut manager = Self::new(max_concurrent);

        for config in configs {
            let destination = manager.create_destination(config)?;
            destination
                .validate_config()
                .map_err(|e| ConfigError::InvalidDestination {
                    destination_type: destination.destination_type().to_string(),
                    error: e,
                })?;
            manager.destinations.push(destination);
        }

        Ok(manager)
    }

    /// Add a destination to the manager
    pub fn add_destination(
        &mut self,
        destination: Arc<dyn OutputDestination>,
    ) -> Result<(), ConfigError> {
        destination
            .validate_config()
            .map_err(|e| ConfigError::InvalidDestination {
                destination_type: destination.destination_type().to_string(),
                error: e,
            })?;
        self.destinations.push(destination);
        Ok(())
    }

    /// Get the number of configured destinations
    pub fn destination_count(&self) -> usize {
        self.destinations.len()
    }

    /// Get metrics for all deliveries
    pub fn metrics(&self) -> Arc<DeliveryMetrics> {
        self.metrics.clone()
    }

    /// Deliver output to all configured destinations
    pub async fn deliver_output(
        &self,
        output: TaskOutput,
        job_context: JobContext,
    ) -> Result<Vec<DeliveryResult>, DeliveryError> {
        if self.destinations.is_empty() {
            return Ok(Vec::new());
        }

        let context = self.build_delivery_context(&output, &job_context);
        let start_time = Instant::now();

        // Execute deliveries concurrently with limit
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_deliveries));
        let mut handles = Vec::new();

        for (idx, destination) in self.destinations.iter().enumerate() {
            let dest = destination.clone(); // Clone the Arc
            let output_clone = output.clone();
            let context_clone = context.clone();
            let metrics_clone = self.metrics.clone();
            let permit =
                semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| DeliveryError::TaskJoin {
                        error: e.to_string(),
                    })?;

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit for duration of task

                let delivery_start = Instant::now();
                let dest_type = dest.destination_type();

                let result = dest.deliver(&output_clone, &context_clone).await;

                // Record metrics
                let duration = delivery_start.elapsed();
                match &result {
                    Ok(delivery_result) => {
                        metrics_clone.record_success(
                            dest_type,
                            duration,
                            delivery_result.size_bytes,
                        );
                    }
                    Err(error) => {
                        metrics_clone.record_failure(dest_type, duration, error);
                    }
                }

                (idx, result)
            });

            handles.push(handle);
        }

        // Wait for all deliveries to complete
        let join_results = join_all(handles).await;
        let mut delivery_results = Vec::with_capacity(self.destinations.len());

        // Initialize results vector with the correct size
        for _ in 0..self.destinations.len() {
            delivery_results.push(Err(DeliveryError::TaskJoin {
                error: "Not executed".to_string(),
            }));
        }

        // Process results in order
        for join_result in join_results {
            match join_result {
                Ok((idx, delivery_result)) => {
                    delivery_results[idx] = delivery_result;
                }
                Err(join_error) => {
                    return Err(DeliveryError::TaskJoin {
                        error: join_error.to_string(),
                    });
                }
            }
        }

        // Record overall metrics
        let total_duration = start_time.elapsed();
        let success_count = delivery_results.iter().filter(|r| r.is_ok()).count();
        let failure_count = delivery_results.len() - success_count;

        self.metrics.record_batch_delivery(
            delivery_results.len(),
            success_count,
            failure_count,
            total_duration,
        );

        delivery_results.into_iter().collect::<Result<Vec<_>, _>>()
    }

    /// Test delivery configurations without actually delivering
    pub async fn test_configurations(
        configs: &[OutputDestinationConfig],
    ) -> Result<Vec<TestResult>, ConfigError> {
        let mut results = Vec::new();

        for (idx, config) in configs.iter().enumerate() {
            let template_engine = TemplateEngine::new();

            match Self::create_destination_from_config(config, &template_engine) {
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

    /// Create a destination from configuration
    fn create_destination(
        &self,
        config: &OutputDestinationConfig,
    ) -> Result<Arc<dyn OutputDestination>, ConfigError> {
        Self::create_destination_from_config(config, &self.template_engine)
    }

    /// Create a destination from configuration (static version)
    fn create_destination_from_config(
        config: &OutputDestinationConfig,
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
                let fs_config = FilesystemConfig {
                    path_template: path.clone(),
                    format: format.clone(),
                    permissions: *permissions,
                    create_dirs: *create_dirs,
                    overwrite: *overwrite,
                    backup_existing: *backup_existing,
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
                let webhook_config = WebhookConfig {
                    url_template: url.clone(),
                    method: *method,
                    headers: headers.clone(),
                    timeout: *timeout,
                    retry_policy: retry_policy.clone(),
                    auth: auth.clone(),
                    content_type: content_type.clone(),
                };

                let client = reqwest::Client::builder()
                    .timeout(webhook_config.timeout + Duration::from_secs(5)) // Add buffer
                    .build()
                    .map_err(|e| ConfigError::HttpClientCreate { source: e })?;

                Ok(Arc::new(WebhookDestination::new(
                    webhook_config,
                    client,
                    template_engine.clone(),
                )))
            }

            // Future implementations
            OutputDestinationConfig::Database { .. } => {
                Err(ConfigError::UnsupportedDestination("database".to_string()))
            }
            OutputDestinationConfig::S3 { .. } => {
                Err(ConfigError::UnsupportedDestination("s3".to_string()))
            }
        }
    }

    /// Build delivery context with template variables
    fn build_delivery_context(
        &self,
        output: &TaskOutput,
        job_context: &JobContext,
    ) -> DeliveryContext {
        let mut template_vars = HashMap::new();

        // Basic variables
        template_vars.insert("job_id".to_string(), output.job_id.to_string());
        template_vars.insert("task_id".to_string(), output.task_id.to_string());
        template_vars.insert("execution_id".to_string(), output.execution_id.to_string());
        template_vars.insert("task_name".to_string(), job_context.task_name.clone());
        template_vars.insert("task_version".to_string(), job_context.task_version.clone());
        template_vars.insert(
            "timestamp".to_string(),
            output.completed_at.format("%Y%m%d_%H%M%S").to_string(),
        );
        template_vars.insert(
            "iso_timestamp".to_string(),
            output.completed_at.to_rfc3339(),
        );
        template_vars.insert(
            "unix_timestamp".to_string(),
            output.completed_at.timestamp().to_string(),
        );
        template_vars.insert(
            "date".to_string(),
            output.completed_at.format("%Y-%m-%d").to_string(),
        );
        template_vars.insert(
            "time".to_string(),
            output.completed_at.format("%H:%M:%S").to_string(),
        );
        template_vars.insert(
            "year".to_string(),
            output.completed_at.format("%Y").to_string(),
        );
        template_vars.insert(
            "month".to_string(),
            output.completed_at.format("%m").to_string(),
        );
        template_vars.insert(
            "day".to_string(),
            output.completed_at.format("%d").to_string(),
        );
        template_vars.insert(
            "hour".to_string(),
            output.completed_at.format("%H").to_string(),
        );

        // Environment variables
        template_vars.insert(
            "env".to_string(),
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        );
        template_vars.insert(
            "hostname".to_string(),
            gethostname::gethostname().to_string_lossy().to_string(),
        );

        // Execution information
        template_vars.insert(
            "duration_ms".to_string(),
            output.execution_duration.as_millis().to_string(),
        );
        template_vars.insert(
            "duration_secs".to_string(),
            output.execution_duration.as_secs().to_string(),
        );

        // Custom metadata
        for (key, value) in &output.metadata {
            let safe_key = key.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
            if let Some(string_value) = value.as_str() {
                template_vars.insert(format!("meta_{}", safe_key), string_value.to_string());
            } else {
                template_vars.insert(format!("meta_{}", safe_key), value.to_string());
            }
        }

        // Environment variables with RATCHET_ prefix
        for (key, value) in std::env::vars() {
            if key.starts_with("RATCHET_") {
                let clean_key = key.strip_prefix("RATCHET_").unwrap_or(&key).to_lowercase();
                template_vars.insert(format!("env_{}", clean_key), value);
            }
        }

        DeliveryContext {
            job_id: output.job_id,
            task_name: job_context.task_name.clone(),
            task_version: job_context.task_version.clone(),
            timestamp: output.completed_at,
            environment: template_vars.get("env").unwrap().clone(),
            trace_id: format!(
                "{}-{}-{}",
                output.job_id,
                output.execution_id,
                output.completed_at.timestamp()
            ),
            template_variables: template_vars,
        }
    }
}

/// Result of testing a destination configuration
#[derive(Debug, Clone)]
pub struct TestResult {
    pub index: usize,
    pub destination_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub estimated_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{OutputFormat, RetryPolicy};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_output() -> TaskOutput {
        TaskOutput {
            job_id: 123,
            task_id: 456,
            execution_id: 789,
            output_data: serde_json::json!({
                "result": "success",
                "data": {"temperature": 20.5}
            }),
            metadata: {
                let mut map = HashMap::new();
                map.insert("user_id".to_string(), serde_json::json!("user123"));
                map.insert("priority".to_string(), serde_json::json!("high"));
                map
            },
            completed_at: chrono::Utc::now(),
            execution_duration: Duration::from_secs(5),
        }
    }

    fn create_test_job_context() -> JobContext {
        JobContext {
            job_uuid: "test-job-uuid".to_string(),
            task_name: "test-task".to_string(),
            task_version: "1.0.0".to_string(),
            schedule_id: None,
            priority: "normal".to_string(),
            environment: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_filesystem_delivery_manager() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("{{job_id}}_output.json");

        let configs = vec![OutputDestinationConfig::Filesystem {
            path: file_path.to_string_lossy().to_string(),
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }];

        let manager = OutputDeliveryManager::from_configs(&configs, 5).unwrap();
        assert_eq!(manager.destination_count(), 1);

        let output = create_test_output();
        let job_context = create_test_job_context();

        let results = manager.deliver_output(output, job_context).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);

        // Check file was created
        let expected_path = temp_dir.path().join("123_output.json");
        assert!(expected_path.exists());
    }

    #[tokio::test]
    async fn test_multiple_destinations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path1 = temp_dir.path().join("output1.json");
        let file_path2 = temp_dir.path().join("output2.yaml");

        let configs = vec![
            OutputDestinationConfig::Filesystem {
                path: file_path1.to_string_lossy().to_string(),
                format: OutputFormat::Json,
                permissions: 0o644,
                create_dirs: true,
                overwrite: true,
                backup_existing: false,
            },
            OutputDestinationConfig::Filesystem {
                path: file_path2.to_string_lossy().to_string(),
                format: OutputFormat::Yaml,
                permissions: 0o644,
                create_dirs: true,
                overwrite: true,
                backup_existing: false,
            },
        ];

        let manager = OutputDeliveryManager::from_configs(&configs, 5).unwrap();
        assert_eq!(manager.destination_count(), 2);

        let output = create_test_output();
        let job_context = create_test_job_context();

        let results = manager.deliver_output(output, job_context).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);

        // Check both files were created
        assert!(file_path1.exists());
        assert!(file_path2.exists());
    }

    #[tokio::test]
    async fn test_delivery_context_template_variables() {
        let manager = OutputDeliveryManager::new(5);
        let output = create_test_output();
        let job_context = create_test_job_context();

        let context = manager.build_delivery_context(&output, &job_context);

        assert_eq!(context.template_variables.get("job_id").unwrap(), "123");
        assert_eq!(
            context.template_variables.get("task_name").unwrap(),
            "test-task"
        );
        assert_eq!(
            context.template_variables.get("task_version").unwrap(),
            "1.0.0"
        );
        assert!(context.template_variables.contains_key("timestamp"));
        assert!(context.template_variables.contains_key("hostname"));
        assert_eq!(
            context.template_variables.get("meta_user_id").unwrap(),
            "user123"
        );
        assert_eq!(
            context.template_variables.get("meta_priority").unwrap(),
            "high"
        );
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        let configs = vec![
            OutputDestinationConfig::Filesystem {
                path: "/tmp/{{job_id}}.json".to_string(),
                format: OutputFormat::Json,
                permissions: 0o644,
                create_dirs: true,
                overwrite: true,
                backup_existing: false,
            },
            OutputDestinationConfig::Webhook {
                url: "https://example.com/webhook".to_string(),
                method: crate::types::HttpMethod::Post,
                headers: HashMap::new(),
                timeout: Duration::from_secs(30),
                retry_policy: RetryPolicy::default(),
                auth: None,
                content_type: None,
            },
        ];

        let test_results = OutputDeliveryManager::test_configurations(&configs)
            .await
            .unwrap();
        assert_eq!(test_results.len(), 2);
        assert!(test_results[0].success);
        assert!(test_results[1].success);
    }

    #[tokio::test]
    async fn test_invalid_configuration() {
        let configs = vec![OutputDestinationConfig::Filesystem {
            path: "".to_string(), // Invalid empty path
            format: OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }];

        let result = OutputDeliveryManager::from_configs(&configs, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_metrics_tracking() {
        let manager = OutputDeliveryManager::new(5);
        let metrics = manager.metrics();

        // Initially no metrics
        assert_eq!(metrics.destination_types().len(), 0);

        // Simulate some metrics
        metrics.record_success("filesystem", Duration::from_millis(100), 1024);
        metrics.record_success("webhook", Duration::from_millis(200), 512);

        assert_eq!(metrics.destination_types().len(), 2);
        assert_eq!(metrics.success_rate("filesystem"), 1.0);
        assert_eq!(metrics.total_bytes_delivered("filesystem"), 1024);
    }
}
