//! Job processor service for processing queued jobs

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use chrono::Utc;
use ratchet_api_types::{ApiId, ExecutionStatus, UnifiedExecution, UnifiedOutputDestination};
use ratchet_interfaces::{DatabaseError, RepositoryFactory};
use ratchet_output::{DeliveryContext, OutputDeliveryManager, OutputDestinationConfig, TaskOutput};
use std::collections::HashMap;

/// Configuration for the job processor service
#[derive(Debug, Clone)]
pub struct JobProcessorConfig {
    /// Poll interval for checking new jobs (in seconds)
    pub poll_interval_seconds: u64,
    /// Maximum number of jobs to process per batch
    pub batch_size: u64,
    /// Enable automatic job processing
    pub enabled: bool,
}

impl Default for JobProcessorConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 5,
            batch_size: 10,
            enabled: true,
        }
    }
}

/// Job processor service that polls for queued jobs and creates executions
pub struct JobProcessorService {
    repositories: Arc<dyn RepositoryFactory>,
    output_manager: Arc<OutputDeliveryManager>,
    config: JobProcessorConfig,
    is_running: AtomicBool,
}

impl JobProcessorService {
    /// Create a new job processor service
    pub fn new(
        repositories: Arc<dyn RepositoryFactory>,
        output_manager: Arc<OutputDeliveryManager>,
        config: JobProcessorConfig,
    ) -> Self {
        Self {
            repositories,
            output_manager,
            config,
            is_running: AtomicBool::new(false),
        }
    }

    /// Start the job processor service
    pub async fn start(&self) -> Result<(), DatabaseError> {
        if !self.config.enabled {
            info!("Job processor service is disabled");
            return Ok(());
        }

        if self.is_running.load(Ordering::Relaxed) {
            warn!("Job processor service is already running");
            return Ok(());
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!(
            "Starting job processor service with {} second poll interval",
            self.config.poll_interval_seconds
        );

        // Main processing loop
        while self.is_running.load(Ordering::Relaxed) {
            if let Err(e) = self.process_batch().await {
                error!("Error processing job batch: {}", e);
            }

            // Sleep between polls
            sleep(Duration::from_secs(self.config.poll_interval_seconds)).await;
        }

        info!("Job processor service stopped");
        Ok(())
    }

    /// Stop the job processor service
    pub async fn stop(&self) {
        info!("Stopping job processor service");
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Process a batch of ready jobs
    async fn process_batch(&self) -> Result<(), DatabaseError> {
        debug!("Checking for ready jobs to process");

        // Get ready jobs from the repository
        let jobs = self
            .repositories
            .job_repository()
            .find_ready_for_processing(self.config.batch_size)
            .await?;

        if jobs.is_empty() {
            debug!("No jobs ready for processing");
            return Ok(());
        }

        info!("Found {} jobs ready for processing", jobs.len());

        // Process each job
        for job in jobs {
            let job_id_copy = job.id.clone();
            if let Err(e) = self.process_job(&job.id).await {
                error!("Failed to process job {}: {}", job_id_copy, e);

                // Mark job as failed
                if let Err(mark_err) = self
                    .repositories
                    .job_repository()
                    .mark_failed(job.id, e.to_string(), None)
                    .await
                {
                    error!("Failed to mark job {} as failed: {}", job_id_copy, mark_err);
                }
            }
        }

        Ok(())
    }

    /// Process a single job by creating an execution and marking it as completed
    /// For now, this is a simplified implementation that doesn't actually execute tasks
    async fn process_job(&self, job_id: &ApiId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing job {}", job_id);

        // Get the job details
        let job = self
            .repositories
            .job_repository()
            .find_by_id(job_id.as_i32().ok_or("Invalid job ID")?)
            .await?
            .ok_or("Job not found")?;

        // Create an execution for this job
        let execution = UnifiedExecution {
            id: ApiId::from_uuid(uuid::Uuid::new_v4()),
            uuid: uuid::Uuid::new_v4(),
            task_id: job.task_id.clone(),
            status: ExecutionStatus::Pending,
            input: serde_json::json!({}), // TODO: Get input from job metadata
            output: None,
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            http_requests: None,
            recording_path: None,
            progress: None,
            can_retry: false,
            can_cancel: false,
        };

        // Create the execution in the repository
        let created_execution = self.repositories.execution_repository().create(execution).await?;

        // Store IDs before they get moved
        let execution_id = created_execution.id.clone();
        let job_id_for_processing = job.id.clone();

        // Mark job as processing and link to execution
        self.repositories
            .job_repository()
            .mark_processing(job_id_for_processing, execution_id.clone())
            .await?;

        info!("Created execution {} for job {}", execution_id, job_id);

        // For now, we'll simulate task execution with a simple success
        // In a full implementation, this would delegate to a task executor
        // TODO: Integrate with actual task execution system

        // Mark execution as started
        self.repositories
            .execution_repository()
            .mark_started(execution_id.clone())
            .await
            .map_err(|e| error!("Failed to mark execution {} as started: {}", execution_id, e))
            .ok();

        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(100)).await;

        // For heartbeat tasks, create a simple success response
        let output = if job.task_id.to_string().contains("heartbeat") {
            serde_json::json!({
                "status": "success",
                "message": "Heartbeat completed successfully",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "system_healthy": true
            })
        } else {
            serde_json::json!({
                "status": "success",
                "message": "Job completed successfully",
                "job_id": job_id.to_string()
            })
        };

        // Mark execution as completed
        if let Err(e) = self
            .repositories
            .execution_repository()
            .mark_completed(
                execution_id.clone(),
                output.clone(),
                Some(100), // 100ms duration
            )
            .await
        {
            error!("Failed to mark execution {} as completed: {}", execution_id, e);
        }

        // Mark job as completed
        if let Err(e) = self.repositories.job_repository().mark_completed(job.id.clone()).await {
            error!("Failed to mark job {} as completed: {}", job_id, e);
        }

        // Process output destinations if any are configured
        if let Some(ref output_destinations) = job.output_destinations {
            self.deliver_job_output(job_id.clone(), execution_id.clone(), output, output_destinations)
                .await;
        }

        info!("Successfully processed job {} with execution {}", job_id, execution_id);
        Ok(())
    }

    /// Deliver job output to configured destinations
    async fn deliver_job_output(
        &self,
        job_id: ApiId,
        execution_id: ApiId,
        output: serde_json::Value,
        destinations: &[UnifiedOutputDestination],
    ) {
        debug!(
            "Delivering output for job {} to {} destinations",
            job_id,
            destinations.len()
        );

        // Create task output for delivery
        let task_output = TaskOutput {
            job_id: job_id.as_i32().unwrap_or(0),
            task_id: 0, // Would need to get from job/execution
            execution_id: execution_id.as_i32().unwrap_or(0),
            output_data: output,
            metadata: HashMap::new(),
            completed_at: Utc::now(),
            execution_duration: std::time::Duration::from_millis(100), // Default duration
        };

        let delivery_context = DeliveryContext::default();

        // Process each destination
        for (index, destination) in destinations.iter().enumerate() {
            let destination_id = format!("job_{}_dest_{}", job_id, index);

            // Convert UnifiedOutputDestination to OutputDestinationConfig
            if let Ok(config) = self.convert_unified_to_output_config(destination) {
                // Add destination to output manager
                if let Err(e) = self
                    .output_manager
                    .add_destination(destination_id.clone(), config)
                    .await
                {
                    error!("Failed to add destination {} for job {}: {}", destination_id, job_id, e);
                    continue;
                }

                // Deliver output
                match self
                    .output_manager
                    .deliver_output(&destination_id, &task_output, &delivery_context)
                    .await
                {
                    Ok(_) => {
                        info!(
                            "Successfully delivered output for job {} to destination {}",
                            job_id, destination_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "Failed to deliver output for job {} to destination {}: {}",
                            job_id, destination_id, e
                        );
                    }
                }

                // Clean up destination after delivery
                if !self.output_manager.remove_destination(&destination_id).await {
                    warn!("Failed to clean up destination {} for job {}", destination_id, job_id);
                }
            } else {
                error!(
                    "Failed to convert output destination for job {}: unsupported destination type {}",
                    job_id, destination.destination_type
                );
            }
        }
    }

    /// Convert between API types and output manager types
    fn convert_output_format(format: &ratchet_api_types::OutputFormat) -> ratchet_output::OutputFormat {
        match format {
            ratchet_api_types::OutputFormat::Json => ratchet_output::OutputFormat::Json,
            ratchet_api_types::OutputFormat::Yaml => ratchet_output::OutputFormat::Yaml,
            ratchet_api_types::OutputFormat::Csv => ratchet_output::OutputFormat::Csv,
            ratchet_api_types::OutputFormat::Xml => ratchet_output::OutputFormat::Raw, // Map XML to Raw since it's not available
        }
    }

    fn convert_http_method(method: &ratchet_api_types::HttpMethod) -> ratchet_http::HttpMethod {
        match method {
            ratchet_api_types::HttpMethod::Get => ratchet_http::HttpMethod::Get,
            ratchet_api_types::HttpMethod::Post => ratchet_http::HttpMethod::Post,
            ratchet_api_types::HttpMethod::Put => ratchet_http::HttpMethod::Put,
            ratchet_api_types::HttpMethod::Patch => ratchet_http::HttpMethod::Patch,
            ratchet_api_types::HttpMethod::Delete => ratchet_http::HttpMethod::Delete,
        }
    }

    /// Convert UnifiedOutputDestination to OutputDestinationConfig
    fn convert_unified_to_output_config(
        &self,
        destination: &UnifiedOutputDestination,
    ) -> Result<OutputDestinationConfig, String> {
        match destination.destination_type.as_str() {
            "stdio" => {
                if let Some(ref stdio_config) = destination.stdio {
                    Ok(OutputDestinationConfig::Stdio {
                        stream: stdio_config.stream.clone(),
                        format: Self::convert_output_format(&stdio_config.format),
                        include_metadata: stdio_config.include_metadata,
                        line_buffered: stdio_config.line_buffered,
                        prefix: stdio_config.prefix.clone(),
                    })
                } else {
                    Err("stdio destination missing configuration".to_string())
                }
            }
            "webhook" => {
                if let Some(ref webhook_config) = destination.webhook {
                    // Convert webhook configuration
                    let mut headers = HashMap::new();

                    // Convert authentication to headers
                    if let Some(ref auth) = webhook_config.authentication {
                        match auth.auth_type.as_str() {
                            "bearer" => {
                                if let Some(ref bearer) = auth.bearer {
                                    headers.insert("Authorization".to_string(), format!("Bearer {}", bearer.token));
                                }
                            }
                            "basic" => {
                                if let Some(ref basic) = auth.basic {
                                    use base64::Engine as _;
                                    let encoded = base64::engine::general_purpose::STANDARD
                                        .encode(format!("{}:{}", basic.username, basic.password));
                                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                                }
                            }
                            "api_key" => {
                                if let Some(ref api_key) = auth.api_key {
                                    headers.insert(api_key.header_name.clone(), api_key.key.clone());
                                }
                            }
                            _ => {}
                        }
                    }

                    // Convert retry policy
                    let retry_policy = if let Some(ref retry) = webhook_config.retry_policy {
                        ratchet_output::RetryPolicy {
                            max_attempts: retry.max_attempts as u32,
                            initial_delay: std::time::Duration::from_secs(retry.initial_delay_seconds as u64),
                            max_delay: std::time::Duration::from_secs(retry.max_delay_seconds as u64),
                            backoff_multiplier: retry.backoff_multiplier,
                            jitter: true,
                            retry_on_status: vec![429, 500, 502, 503, 504],
                        }
                    } else {
                        ratchet_output::RetryPolicy::default()
                    };

                    Ok(OutputDestinationConfig::Webhook {
                        url: webhook_config.url.clone(),
                        method: Self::convert_http_method(&webhook_config.method),
                        headers,
                        timeout: std::time::Duration::from_secs(webhook_config.timeout_seconds as u64),
                        retry_policy,
                        auth: None, // Already converted to headers
                        content_type: webhook_config.content_type.clone(),
                    })
                } else {
                    Err("webhook destination missing configuration".to_string())
                }
            }
            "filesystem" => {
                if let Some(ref fs_config) = destination.filesystem {
                    Ok(OutputDestinationConfig::Filesystem {
                        path: fs_config.path.clone(),
                        format: Self::convert_output_format(&fs_config.format),
                        permissions: if let Some(ref perms) = fs_config.permissions {
                            u32::from_str_radix(perms, 8).unwrap_or(0o644)
                        } else {
                            0o644
                        },
                        create_dirs: true,
                        overwrite: false,
                        backup_existing: false,
                    })
                } else {
                    Err("filesystem destination missing configuration".to_string())
                }
            }
            _ => Err(format!(
                "unsupported destination type: {}",
                destination.destination_type
            )),
        }
    }
}

/// Job processor service trait for dependency injection
#[async_trait]
pub trait JobProcessor: Send + Sync {
    /// Start the job processor
    async fn start(&self) -> Result<(), DatabaseError>;

    /// Stop the job processor
    async fn stop(&self);

    /// Check if running
    fn is_running(&self) -> bool;
}

#[async_trait]
impl JobProcessor for JobProcessorService {
    async fn start(&self) -> Result<(), DatabaseError> {
        JobProcessorService::start(self).await
    }

    async fn stop(&self) {
        JobProcessorService::stop(self).await
    }

    fn is_running(&self) -> bool {
        JobProcessorService::is_running(self)
    }
}
