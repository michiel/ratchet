//! Repository and sync health monitoring system
//!
//! This module provides comprehensive monitoring of repository health,
//! sync operations, and system performance metrics.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use crate::repository_services::EnhancedRepositoryService;
use crate::scheduler::{SyncScheduler, ScheduledSyncResult};
use crate::watchers::FilesystemWatcher;
use ratchet_storage::repositories::RepositoryHealth;

/// Configuration for sync health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHealthConfig {
    /// Interval for health checks in minutes
    pub health_check_interval_minutes: u32,
    /// Interval for metrics collection in minutes
    pub metrics_collection_interval_minutes: u32,
    /// Number of health check history entries to keep per repository
    pub health_history_size: usize,
    /// Number of sync result history entries to keep
    pub sync_history_size: usize,
    /// Threshold for marking repository as unhealthy (consecutive failures)
    pub unhealthy_threshold: u32,
    /// Threshold for alerting on sync failures (consecutive failures)
    pub alert_threshold: u32,
    /// Maximum age of sync results to consider for metrics (hours)
    pub metrics_window_hours: u32,
    /// Whether to enable detailed performance metrics
    pub enable_performance_metrics: bool,
    /// Whether to enable alert notifications
    pub enable_alerts: bool,
}

impl Default for SyncHealthConfig {
    fn default() -> Self {
        Self {
            health_check_interval_minutes: 5,
            metrics_collection_interval_minutes: 10,
            health_history_size: 100,
            sync_history_size: 1000,
            unhealthy_threshold: 3,
            alert_threshold: 5,
            metrics_window_hours: 24,
            enable_performance_metrics: true,
            enable_alerts: true,
        }
    }
}

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Repository health entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryHealthEntry {
    /// Repository ID
    pub repository_id: i32,
    /// Repository name
    pub repository_name: String,
    /// Timestamp of health check
    pub timestamp: DateTime<Utc>,
    /// Health status
    pub health: RepositoryHealth,
    /// Overall status
    pub status: HealthStatus,
    /// Additional notes
    pub notes: Option<String>,
}

/// Sync metrics for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySyncMetrics {
    /// Repository ID
    pub repository_id: i32,
    /// Repository name
    pub repository_name: String,
    /// Total number of sync operations
    pub total_syncs: u64,
    /// Number of successful syncs
    pub successful_syncs: u64,
    /// Number of failed syncs
    pub failed_syncs: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average sync duration in milliseconds
    pub avg_sync_duration_ms: u64,
    /// Minimum sync duration in milliseconds
    pub min_sync_duration_ms: u64,
    /// Maximum sync duration in milliseconds
    pub max_sync_duration_ms: u64,
    /// Last sync timestamp
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Last successful sync timestamp
    pub last_successful_sync_at: Option<DateTime<Utc>>,
    /// Consecutive failure count
    pub consecutive_failures: u32,
    /// Current health status
    pub health_status: HealthStatus,
}

/// Overall system sync metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    /// Timestamp of metrics collection
    pub timestamp: DateTime<Utc>,
    /// Total number of repositories
    pub total_repositories: u32,
    /// Number of healthy repositories
    pub healthy_repositories: u32,
    /// Number of repositories with warnings
    pub warning_repositories: u32,
    /// Number of critical repositories
    pub critical_repositories: u32,
    /// Overall system health status
    pub overall_health: HealthStatus,
    /// Per-repository metrics
    pub repository_metrics: Vec<RepositorySyncMetrics>,
    /// System-wide statistics
    pub total_syncs_last_24h: u64,
    pub successful_syncs_last_24h: u64,
    pub failed_syncs_last_24h: u64,
    pub avg_sync_duration_last_24h_ms: u64,
}

/// Health alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Alert ID
    pub id: String,
    /// Repository ID (if repository-specific)
    pub repository_id: Option<i32>,
    /// Repository name (if repository-specific)
    pub repository_name: Option<String>,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert type
    pub alert_type: AlertType,
    /// Alert message
    pub message: String,
    /// Timestamp when alert was raised
    pub raised_at: DateTime<Utc>,
    /// Whether alert is currently active
    pub active: bool,
    /// Timestamp when alert was resolved (if resolved)
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Types of health alerts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    RepositoryUnhealthy,
    SyncFailures,
    PerformanceDegradation,
    ConnectivityIssue,
    SystemOverload,
}

/// Sync health monitor
pub struct SyncHealthMonitor {
    /// Configuration
    config: SyncHealthConfig,
    /// Repository service for health checks
    repository_service: Arc<EnhancedRepositoryService>,
    /// Sync scheduler for metrics
    sync_scheduler: Arc<SyncScheduler>,
    /// Filesystem watcher for monitoring
    filesystem_watcher: Arc<FilesystemWatcher>,
    /// Repository health history
    health_history: Arc<RwLock<HashMap<i32, VecDeque<RepositoryHealthEntry>>>>,
    /// Sync metrics cache
    metrics_cache: Arc<RwLock<Option<SyncMetrics>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, HealthAlert>>>,
    /// Alert history
    alert_history: Arc<Mutex<VecDeque<HealthAlert>>>,
    /// Whether the monitor is running
    running: Arc<Mutex<bool>>,
}

impl SyncHealthMonitor {
    /// Create a new sync health monitor
    pub fn new(
        config: SyncHealthConfig,
        repository_service: Arc<EnhancedRepositoryService>,
        sync_scheduler: Arc<SyncScheduler>,
        filesystem_watcher: Arc<FilesystemWatcher>,
    ) -> Self {
        Self {
            config,
            repository_service,
            sync_scheduler,
            filesystem_watcher,
            health_history: Arc::new(RwLock::new(HashMap::new())),
            metrics_cache: Arc::new(RwLock::new(None)),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the health monitor
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            if *running {
                return Err(anyhow!("Health monitor is already running"));
            }
            *running = true;
        }

        info!("Starting sync health monitor with config: {:?}", self.config);

        // Start health check loop
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval_timer = interval(TokioDuration::from_secs(
                monitor.config.health_check_interval_minutes as u64 * 60
            ));

            loop {
                interval_timer.tick().await;
                if let Err(e) = monitor.perform_health_checks().await {
                    error!("Error performing health checks: {}", e);
                }
            }
        });

        // Start metrics collection loop
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval_timer = interval(TokioDuration::from_secs(
                monitor.config.metrics_collection_interval_minutes as u64 * 60
            ));

            loop {
                interval_timer.tick().await;
                if let Err(e) = monitor.collect_metrics().await {
                    error!("Error collecting metrics: {}", e);
                }
            }
        });

        info!("Sync health monitor started successfully");
        Ok(())
    }

    /// Stop the health monitor
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync health monitor");

        {
            let mut running = self.running.lock().await;
            *running = false;
        }

        info!("Sync health monitor stopped");
        Ok(())
    }

    /// Get current sync metrics
    pub async fn get_sync_metrics(&self) -> Option<SyncMetrics> {
        self.metrics_cache.read().await.clone()
    }

    /// Get repository health history
    pub async fn get_repository_health_history(&self, repository_id: i32) -> Vec<RepositoryHealthEntry> {
        let history = self.health_history.read().await;
        history.get(&repository_id)
            .map(|deque| deque.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<HealthAlert> {
        let alerts = self.active_alerts.read().await;
        alerts.values().cloned().collect()
    }

    /// Get alert history
    pub async fn get_alert_history(&self, limit: Option<usize>) -> Vec<HealthAlert> {
        let history = self.alert_history.lock().await;
        let limit = limit.unwrap_or(100);
        
        if history.len() <= limit {
            history.iter().cloned().collect()
        } else {
            history.iter().rev().take(limit).cloned().collect()
        }
    }

    /// Force immediate health check for all repositories
    pub async fn force_health_check(&self) -> Result<()> {
        info!("Forcing immediate health check for all repositories");
        self.perform_health_checks().await
    }

    /// Force immediate metrics collection
    pub async fn force_metrics_collection(&self) -> Result<SyncMetrics> {
        info!("Forcing immediate metrics collection");
        self.collect_metrics().await
    }

    /// Perform health checks for all repositories
    async fn perform_health_checks(&self) -> Result<()> {
        debug!("Performing health checks for all repositories");

        let repositories = self.repository_service.list_repositories().await
            .context("Failed to list repositories")?;

        for repository in repositories {
            if let Err(e) = self.check_repository_health(repository.id.as_i32().unwrap_or(0), &repository.name).await {
                error!("Health check failed for repository {}: {}", repository.name, e);
            }
        }

        debug!("Completed health checks for all repositories");
        Ok(())
    }

    /// Check health for a specific repository
    async fn check_repository_health(&self, repository_id: i32, repository_name: &str) -> Result<()> {
        let timestamp = Utc::now();

        // Get repository health
        let health = self.repository_service.get_repository_health(repository_id).await
            .context("Failed to get repository health")?;

        // Determine overall status
        let status = if !health.accessible {
            HealthStatus::Critical
        } else if !health.writable {
            HealthStatus::Warning
        } else if health.error_count > 0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        // Create health entry
        let health_entry = RepositoryHealthEntry {
            repository_id,
            repository_name: repository_name.to_string(),
            timestamp,
            health: health.clone(),
            status: status.clone(),
            notes: if health.error_count > 0 {
                Some(format!("Repository has {} errors: {}", health.error_count, health.message))
            } else {
                None
            },
        };

        // Store in health history
        {
            let mut history = self.health_history.write().await;
            let repo_history = history.entry(repository_id).or_insert_with(VecDeque::new);
            repo_history.push_back(health_entry.clone());
            
            // Limit history size
            while repo_history.len() > self.config.health_history_size {
                repo_history.pop_front();
            }
        }

        // Check for alerting conditions
        if self.config.enable_alerts {
            self.check_health_alerts(repository_id, repository_name, &status, &health).await?;
        }

        debug!("Health check completed for repository {}: {:?}", repository_name, status);
        Ok(())
    }

    /// Check for health alert conditions
    async fn check_health_alerts(&self, repository_id: i32, repository_name: &str, status: &HealthStatus, health: &RepositoryHealth) -> Result<()> {
        let alert_id = format!("health_{}", repository_id);

        match status {
            HealthStatus::Critical => {
                self.raise_alert(
                    alert_id,
                    Some(repository_id),
                    Some(repository_name.to_string()),
                    AlertSeverity::Critical,
                    AlertType::RepositoryUnhealthy,
                    format!("Repository {} is in critical state: {}", repository_name, health.message),
                ).await?;
            }
            HealthStatus::Warning => {
                self.raise_alert(
                    alert_id,
                    Some(repository_id),
                    Some(repository_name.to_string()),
                    AlertSeverity::Warning,
                    AlertType::RepositoryUnhealthy,
                    format!("Repository {} has warnings: {}", repository_name, health.message),
                ).await?;
            }
            HealthStatus::Healthy => {
                self.resolve_alert(alert_id).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Collect sync metrics
    async fn collect_metrics(&self) -> Result<SyncMetrics> {
        debug!("Collecting sync metrics");
        let timestamp = Utc::now();

        // Get all repositories
        let repositories = self.repository_service.list_repositories().await
            .context("Failed to list repositories")?;

        let mut repository_metrics = Vec::new();
        let mut total_repositories = 0u32;
        let mut healthy_repositories = 0u32;
        let mut warning_repositories = 0u32;
        let mut critical_repositories = 0u32;

        // Get sync history from scheduler
        let sync_history = self.sync_scheduler.get_sync_history(Some(self.config.sync_history_size)).await;

        for repository in repositories {
            let repo_id = repository.id.as_i32().unwrap_or(0);
            let metrics = self.calculate_repository_metrics(repo_id, &repository.name, &sync_history).await?;
            
            match metrics.health_status {
                HealthStatus::Healthy => healthy_repositories += 1,
                HealthStatus::Warning => warning_repositories += 1,
                HealthStatus::Critical => critical_repositories += 1,
                _ => {}
            }

            repository_metrics.push(metrics);
            total_repositories += 1;
        }

        // Calculate system-wide metrics
        let cutoff_time = timestamp - Duration::hours(24);
        let recent_syncs: Vec<&ScheduledSyncResult> = sync_history
            .iter()
            .filter(|result| result.sync_at >= cutoff_time)
            .collect();

        let total_syncs_last_24h = recent_syncs.len() as u64;
        let successful_syncs_last_24h = recent_syncs.iter().filter(|r| r.success).count() as u64;
        let failed_syncs_last_24h = total_syncs_last_24h - successful_syncs_last_24h;
        
        let avg_sync_duration_last_24h_ms = if !recent_syncs.is_empty() {
            recent_syncs.iter().map(|r| r.duration_ms).sum::<u64>() / total_syncs_last_24h
        } else {
            0
        };

        // Determine overall health
        let overall_health = if critical_repositories > 0 {
            HealthStatus::Critical
        } else if warning_repositories > 0 {
            HealthStatus::Warning
        } else if healthy_repositories > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };

        let metrics = SyncMetrics {
            timestamp,
            total_repositories,
            healthy_repositories,
            warning_repositories,
            critical_repositories,
            overall_health,
            repository_metrics,
            total_syncs_last_24h,
            successful_syncs_last_24h,
            failed_syncs_last_24h,
            avg_sync_duration_last_24h_ms,
        };

        // Cache metrics
        {
            let mut cache = self.metrics_cache.write().await;
            *cache = Some(metrics.clone());
        }

        debug!("Collected sync metrics: {} repositories, {} healthy, {} warning, {} critical", 
            total_repositories, healthy_repositories, warning_repositories, critical_repositories);

        Ok(metrics)
    }

    /// Calculate metrics for a specific repository
    async fn calculate_repository_metrics(&self, repository_id: i32, repository_name: &str, sync_history: &[ScheduledSyncResult]) -> Result<RepositorySyncMetrics> {
        // Filter sync results for this repository
        let repo_syncs: Vec<&ScheduledSyncResult> = sync_history
            .iter()
            .filter(|result| result.repository_id == repository_id)
            .collect();

        let total_syncs = repo_syncs.len() as u64;
        let successful_syncs = repo_syncs.iter().filter(|r| r.success).count() as u64;
        let failed_syncs = total_syncs - successful_syncs;
        
        let success_rate = if total_syncs > 0 {
            successful_syncs as f64 / total_syncs as f64
        } else {
            0.0
        };

        let (avg_sync_duration_ms, min_sync_duration_ms, max_sync_duration_ms) = if !repo_syncs.is_empty() {
            let durations: Vec<u64> = repo_syncs.iter().map(|r| r.duration_ms).collect();
            let avg = durations.iter().sum::<u64>() / durations.len() as u64;
            let min = *durations.iter().min().unwrap_or(&0);
            let max = *durations.iter().max().unwrap_or(&0);
            (avg, min, max)
        } else {
            (0, 0, 0)
        };

        let last_sync_at = repo_syncs.last().map(|r| r.sync_at);
        let last_successful_sync_at = repo_syncs
            .iter()
            .rev()
            .find(|r| r.success)
            .map(|r| r.sync_at);

        // Count consecutive failures
        let mut consecutive_failures = 0u32;
        for result in repo_syncs.iter().rev() {
            if result.success {
                break;
            }
            consecutive_failures += 1;
        }

        // Determine health status
        let health_status = if consecutive_failures >= self.config.alert_threshold {
            HealthStatus::Critical
        } else if consecutive_failures >= self.config.unhealthy_threshold {
            HealthStatus::Warning
        } else if success_rate < 0.8 && total_syncs > 5 {
            HealthStatus::Warning
        } else if total_syncs > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };

        Ok(RepositorySyncMetrics {
            repository_id,
            repository_name: repository_name.to_string(),
            total_syncs,
            successful_syncs,
            failed_syncs,
            success_rate,
            avg_sync_duration_ms,
            min_sync_duration_ms,
            max_sync_duration_ms,
            last_sync_at,
            last_successful_sync_at,
            consecutive_failures,
            health_status,
        })
    }

    /// Raise an alert
    async fn raise_alert(&self, alert_id: String, repository_id: Option<i32>, repository_name: Option<String>, severity: AlertSeverity, alert_type: AlertType, message: String) -> Result<()> {
        let now = Utc::now();

        // Check if alert already exists
        {
            let alerts = self.active_alerts.read().await;
            if alerts.contains_key(&alert_id) {
                return Ok(()); // Alert already active
            }
        }

        let alert = HealthAlert {
            id: alert_id.clone(),
            repository_id,
            repository_name,
            severity,
            alert_type,
            message: message.clone(),
            raised_at: now,
            active: true,
            resolved_at: None,
        };

        // Add to active alerts
        {
            let mut alerts = self.active_alerts.write().await;
            alerts.insert(alert_id, alert.clone());
        }

        // Add to alert history
        {
            let mut history = self.alert_history.lock().await;
            history.push_back(alert.clone());
            // Keep only last 1000 alerts
            while history.len() > 1000 {
                history.pop_front();
            }
        }

        warn!("ALERT RAISED: {:?} - {}", alert.severity, message);
        Ok(())
    }

    /// Resolve an alert
    async fn resolve_alert(&self, alert_id: String) -> Result<()> {
        let now = Utc::now();

        // Remove from active alerts and update in history
        {
            let mut alerts = self.active_alerts.write().await;
            if let Some(mut alert) = alerts.remove(&alert_id) {
                alert.active = false;
                alert.resolved_at = Some(now);

                // Update in history
                {
                    let mut history = self.alert_history.lock().await;
                    if let Some(hist_alert) = history.iter_mut().rev().find(|a| a.id == alert_id && a.active) {
                        hist_alert.active = false;
                        hist_alert.resolved_at = Some(now);
                    }
                }

                info!("ALERT RESOLVED: {}", alert.message);
            }
        }

        Ok(())
    }
}

// Implement Clone for SyncHealthMonitor to allow sharing between tasks
impl Clone for SyncHealthMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            repository_service: self.repository_service.clone(),
            sync_scheduler: self.sync_scheduler.clone(),
            filesystem_watcher: self.filesystem_watcher.clone(),
            health_history: self.health_history.clone(),
            metrics_cache: self.metrics_cache.clone(),
            active_alerts: self.active_alerts.clone(),
            alert_history: self.alert_history.clone(),
            running: self.running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    
    
    // TODO: Add comprehensive tests for sync health monitor
    // This would include:
    // - Health check processing
    // - Metrics calculation
    // - Alert generation and resolution
    // - Performance monitoring
    // - Error handling scenarios
}