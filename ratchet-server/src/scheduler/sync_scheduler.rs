//! Repository synchronization scheduler
//!
//! This module provides automated repository synchronization capabilities,
//! managing periodic sync operations and triggered sync events.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration as TokioDuration, Instant};
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use crate::repository_services::EnhancedRepositoryService;
use ratchet_storage::repositories::{SyncResult, RepositoryHealth};

/// Configuration for the sync scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSchedulerConfig {
    /// Default sync interval in minutes
    pub default_sync_interval_minutes: u32,
    /// Maximum number of concurrent sync operations
    pub max_concurrent_syncs: usize,
    /// Timeout for individual sync operations in seconds
    pub sync_timeout_seconds: u64,
    /// Whether to enable automatic retry of failed syncs
    pub enable_auto_retry: bool,
    /// Maximum number of retry attempts
    pub max_retry_attempts: u32,
    /// Base delay between retries in seconds
    pub retry_delay_seconds: u64,
    /// Whether to enable health checks before sync
    pub enable_health_checks: bool,
}

impl Default for SyncSchedulerConfig {
    fn default() -> Self {
        Self {
            default_sync_interval_minutes: 15,
            max_concurrent_syncs: 3,
            sync_timeout_seconds: 300, // 5 minutes
            enable_auto_retry: true,
            max_retry_attempts: 3,
            retry_delay_seconds: 60,
            enable_health_checks: true,
        }
    }
}

/// Sync schedule entry for a repository
#[derive(Debug, Clone)]
pub struct SyncSchedule {
    /// Repository ID
    pub repository_id: i32,
    /// Repository name
    pub repository_name: String,
    /// Sync interval in minutes
    pub interval_minutes: u32,
    /// Last sync timestamp
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Next scheduled sync timestamp
    pub next_sync_at: DateTime<Utc>,
    /// Whether this schedule is active
    pub enabled: bool,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Last sync result
    pub last_result: Option<ScheduledSyncResult>,
}

/// Result of a scheduled sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledSyncResult {
    /// Repository ID
    pub repository_id: i32,
    /// Sync timestamp
    pub sync_at: DateTime<Utc>,
    /// Whether the sync was successful
    pub success: bool,
    /// Sync result details
    pub sync_result: Option<SyncResult>,
    /// Error message if sync failed
    pub error: Option<String>,
    /// Duration of the sync operation
    pub duration_ms: u64,
    /// Repository health after sync
    pub health: Option<RepositoryHealth>,
}

/// Sync scheduler for managing repository synchronization
pub struct SyncScheduler {
    /// Configuration
    config: SyncSchedulerConfig,
    /// Repository service for sync operations
    repository_service: Arc<EnhancedRepositoryService>,
    /// Active sync schedules by repository ID
    schedules: Arc<RwLock<HashMap<i32, SyncSchedule>>>,
    /// Currently running sync operations
    active_syncs: Arc<RwLock<HashMap<i32, Instant>>>,
    /// Sync history for monitoring
    sync_history: Arc<Mutex<Vec<ScheduledSyncResult>>>,
    /// Whether the scheduler is running
    running: Arc<Mutex<bool>>,
    /// Shutdown signal
    shutdown_tx: Arc<Mutex<Option<tokio::sync::broadcast::Sender<()>>>>,
}

impl SyncScheduler {
    /// Create a new sync scheduler
    pub fn new(
        config: SyncSchedulerConfig,
        repository_service: Arc<EnhancedRepositoryService>,
    ) -> Self {
        Self {
            config,
            repository_service,
            schedules: Arc::new(RwLock::new(HashMap::new())),
            active_syncs: Arc::new(RwLock::new(HashMap::new())),
            sync_history: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the sync scheduler
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            if *running {
                return Err(anyhow!("Sync scheduler is already running"));
            }
            *running = true;
        }

        info!("Starting repository sync scheduler with config: {:?}", self.config);

        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
        {
            let mut tx_guard = self.shutdown_tx.lock().await;
            *tx_guard = Some(shutdown_tx.clone());
        }

        // Initialize repository schedules
        self.initialize_schedules().await?;

        // Start the main scheduler loop
        let scheduler = self.clone();
        let mut shutdown_rx = shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut interval_timer = interval(TokioDuration::from_secs(60)); // Check every minute

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        if let Err(e) = scheduler.process_scheduled_syncs().await {
                            error!("Error processing scheduled syncs: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Sync scheduler received shutdown signal");
                        break;
                    }
                }
            }

            // Clean up active syncs
            {
                let mut running = scheduler.running.lock().await;
                *running = false;
            }

            info!("Sync scheduler stopped");
        });

        info!("Sync scheduler started successfully");
        Ok(())
    }

    /// Stop the sync scheduler
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync scheduler");

        {
            let running = self.running.lock().await;
            if !*running {
                return Ok(());
            }
        }

        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_tx.lock().await.as_ref() {
            if let Err(_) = shutdown_tx.send(()) {
                warn!("No receivers for shutdown signal");
            }
        }

        // Wait for active syncs to complete
        let timeout = TokioDuration::from_secs(30);
        let start = Instant::now();

        while start.elapsed() < timeout {
            let active_count = self.active_syncs.read().await.len();
            if active_count == 0 {
                break;
            }
            tokio::time::sleep(TokioDuration::from_millis(100)).await;
        }

        info!("Sync scheduler stopped");
        Ok(())
    }

    /// Add or update a sync schedule for a repository
    pub async fn schedule_repository_sync(&self, repository_id: i32, interval_minutes: u32) -> Result<()> {
        info!("Scheduling repository {} for sync every {} minutes", repository_id, interval_minutes);

        // Get repository information
        let repository = self.repository_service.get_repository(repository_id).await
            .context("Failed to get repository")?
            .ok_or_else(|| anyhow!("Repository {} not found", repository_id))?;

        // Calculate next sync time
        let now = Utc::now();
        let next_sync_at = now + Duration::minutes(interval_minutes as i64);

        let schedule = SyncSchedule {
            repository_id,
            repository_name: repository.name,
            interval_minutes,
            last_sync_at: None,
            next_sync_at,
            enabled: true,
            failure_count: 0,
            last_result: None,
        };

        let mut schedules = self.schedules.write().await;
        schedules.insert(repository_id, schedule);

        info!("Repository {} scheduled for sync at {}", repository_id, next_sync_at);
        Ok(())
    }

    /// Remove a sync schedule for a repository
    pub async fn unschedule_repository_sync(&self, repository_id: i32) -> Result<()> {
        info!("Unscheduling repository {} from sync", repository_id);

        let mut schedules = self.schedules.write().await;
        schedules.remove(&repository_id);

        info!("Repository {} unscheduled from sync", repository_id);
        Ok(())
    }

    /// Trigger an immediate sync for a repository
    pub async fn trigger_immediate_sync(&self, repository_id: i32) -> Result<ScheduledSyncResult> {
        info!("Triggering immediate sync for repository {}", repository_id);

        // Check if sync is already running
        {
            let active_syncs = self.active_syncs.read().await;
            if active_syncs.contains_key(&repository_id) {
                return Err(anyhow!("Sync already in progress for repository {}", repository_id));
            }
        }

        // Perform the sync
        self.perform_sync(repository_id).await
    }

    /// Get sync status for all repositories
    pub async fn get_sync_status(&self) -> HashMap<i32, SyncSchedule> {
        self.schedules.read().await.clone()
    }

    /// Get sync history
    pub async fn get_sync_history(&self, limit: Option<usize>) -> Vec<ScheduledSyncResult> {
        let history = self.sync_history.lock().await;
        let limit = limit.unwrap_or(100);
        
        if history.len() <= limit {
            history.clone()
        } else {
            history[history.len() - limit..].to_vec()
        }
    }

    /// Initialize repository schedules from database
    async fn initialize_schedules(&self) -> Result<()> {
        info!("Initializing repository sync schedules");

        let repositories = self.repository_service.list_repositories().await
            .context("Failed to list repositories")?;

        let mut initialized_count = 0;

        for repository in repositories {
            if repository.sync_enabled {
                let interval_minutes = repository.sync_interval_minutes
                    .unwrap_or(self.config.default_sync_interval_minutes as i32) as u32;

                if let Err(e) = self.schedule_repository_sync(repository.id.as_i32().unwrap_or(0), interval_minutes).await {
                    warn!("Failed to schedule repository {}: {}", repository.name, e);
                } else {
                    initialized_count += 1;
                }
            }
        }

        info!("Initialized {} repository sync schedules", initialized_count);
        Ok(())
    }

    /// Process scheduled syncs
    async fn process_scheduled_syncs(&self) -> Result<()> {
        let now = Utc::now();
        let mut due_syncs = Vec::new();

        // Find schedules that are due
        {
            let schedules = self.schedules.read().await;
            for (repo_id, schedule) in schedules.iter() {
                if schedule.enabled && schedule.next_sync_at <= now {
                    // Check if sync is not already running
                    let active_syncs = self.active_syncs.read().await;
                    if !active_syncs.contains_key(repo_id) {
                        due_syncs.push(*repo_id);
                    }
                }
            }
        }

        if due_syncs.is_empty() {
            return Ok(());
        }

        debug!("Found {} repositories due for sync", due_syncs.len());

        // Limit concurrent syncs
        let max_concurrent = self.config.max_concurrent_syncs;
        let active_count = self.active_syncs.read().await.len();
        let available_slots = max_concurrent.saturating_sub(active_count);

        let syncs_to_run = due_syncs.into_iter().take(available_slots).collect::<Vec<_>>();

        // Start syncs
        for repository_id in syncs_to_run {
            let scheduler = self.clone();
            tokio::spawn(async move {
                if let Err(e) = scheduler.perform_scheduled_sync(repository_id).await {
                    error!("Scheduled sync failed for repository {}: {}", repository_id, e);
                }
            });
        }

        Ok(())
    }

    /// Perform a scheduled sync for a repository
    async fn perform_scheduled_sync(&self, repository_id: i32) -> Result<()> {
        let result = self.perform_sync(repository_id).await?;

        // Update schedule based on result
        {
            let mut schedules = self.schedules.write().await;
            if let Some(schedule) = schedules.get_mut(&repository_id) {
                schedule.last_sync_at = Some(result.sync_at);
                schedule.last_result = Some(result.clone());

                if result.success {
                    // Reset failure count and calculate next sync time
                    schedule.failure_count = 0;
                    schedule.next_sync_at = result.sync_at + Duration::minutes(schedule.interval_minutes as i64);
                } else {
                    // Increment failure count and apply backoff
                    schedule.failure_count += 1;
                    let backoff_minutes = schedule.interval_minutes * (2_u32.pow(schedule.failure_count.min(3)));
                    schedule.next_sync_at = result.sync_at + Duration::minutes(backoff_minutes as i64);

                    // Disable schedule if too many failures
                    if schedule.failure_count >= self.config.max_retry_attempts {
                        warn!("Disabling sync schedule for repository {} after {} failures", 
                            repository_id, schedule.failure_count);
                        schedule.enabled = false;
                    }
                }
            }
        }

        // Store result in history
        {
            let mut history = self.sync_history.lock().await;
            history.push(result);
            // Keep only last 1000 results
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Perform a sync operation for a repository
    async fn perform_sync(&self, repository_id: i32) -> Result<ScheduledSyncResult> {
        let start_time = Instant::now();
        let sync_at = Utc::now();

        // Mark sync as active
        {
            let mut active_syncs = self.active_syncs.write().await;
            active_syncs.insert(repository_id, start_time);
        }

        // Perform health check if enabled
        let mut health = None;
        if self.config.enable_health_checks {
            match self.repository_service.get_repository_health(repository_id).await {
                Ok(repo_health) => {
                    health = Some(repo_health.clone());
                    if !repo_health.accessible {
                        // Remove from active syncs
                        {
                            let mut active_syncs = self.active_syncs.write().await;
                            active_syncs.remove(&repository_id);
                        }

                        return Ok(ScheduledSyncResult {
                            repository_id,
                            sync_at,
                            success: false,
                            sync_result: None,
                            error: Some(format!("Repository not accessible: {}", repo_health.message)),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            health,
                        });
                    }
                }
                Err(e) => {
                    warn!("Health check failed for repository {}: {}", repository_id, e);
                }
            }
        }

        // Perform the sync with timeout
        let sync_future = self.repository_service.sync_repository(repository_id);
        let timeout_duration = TokioDuration::from_secs(self.config.sync_timeout_seconds);

        let sync_result = match tokio::time::timeout(timeout_duration, sync_future).await {
            Ok(Ok(result)) => {
                info!("Sync completed for repository {}: Added: {}, Updated: {}, Deleted: {}", 
                    repository_id, result.tasks_added, result.tasks_updated, result.tasks_deleted);
                
                let success = result.errors.is_empty();
                let error = if result.errors.is_empty() {
                    None
                } else {
                    let error_strings: Vec<String> = result.errors.iter().map(|e| e.to_string()).collect();
                    Some(error_strings.join("; "))
                };

                ScheduledSyncResult {
                    repository_id,
                    sync_at,
                    success,
                    sync_result: Some(result),
                    error,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    health,
                }
            }
            Ok(Err(e)) => {
                error!("Sync failed for repository {}: {}", repository_id, e);
                ScheduledSyncResult {
                    repository_id,
                    sync_at,
                    success: false,
                    sync_result: None,
                    error: Some(e.to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    health,
                }
            }
            Err(_) => {
                error!("Sync timed out for repository {} after {} seconds", 
                    repository_id, self.config.sync_timeout_seconds);
                ScheduledSyncResult {
                    repository_id,
                    sync_at,
                    success: false,
                    sync_result: None,
                    error: Some("Sync operation timed out".to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    health,
                }
            }
        };

        // Remove from active syncs
        {
            let mut active_syncs = self.active_syncs.write().await;
            active_syncs.remove(&repository_id);
        }

        Ok(sync_result)
    }
}

// Implement Clone for SyncScheduler to allow sharing between tasks
impl Clone for SyncScheduler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            repository_service: self.repository_service.clone(),
            schedules: self.schedules.clone(),
            active_syncs: self.active_syncs.clone(),
            sync_history: self.sync_history.clone(),
            running: self.running.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: Add comprehensive tests for sync scheduler
    // This would include:
    // - Schedule management
    // - Sync execution
    // - Timeout handling
    // - Health check integration
    // - Error handling and retry logic
}