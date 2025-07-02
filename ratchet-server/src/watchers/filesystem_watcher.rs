//! File system watcher for real-time repository monitoring
//!
//! This module provides real-time file system monitoring for repository changes,
//! triggering automatic sync operations when files are modified.

use chrono::{DateTime, Utc};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock, Mutex};
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use crate::repository_services::EnhancedRepositoryService;
use crate::scheduler::SyncScheduler;

/// Configuration for filesystem watcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemWatcherConfig {
    /// Debounce delay in milliseconds to avoid excessive sync triggers
    pub debounce_delay_ms: u64,
    /// Maximum number of concurrent sync operations triggered by file changes
    pub max_concurrent_syncs: usize,
    /// Whether to enable recursive watching of subdirectories
    pub recursive_watching: bool,
    /// File patterns to watch (e.g., ["*.js", "*.json", "*.yaml"])
    pub watch_patterns: Vec<String>,
    /// File patterns to ignore (e.g., [".git/**", "node_modules/**", "*.tmp"])
    pub ignore_patterns: Vec<String>,
    /// Minimum interval between syncs for the same repository in seconds
    pub min_sync_interval_seconds: u64,
    /// Whether to batch multiple file changes into single sync operations
    pub enable_batching: bool,
    /// Batching window in milliseconds
    pub batch_window_ms: u64,
}

impl Default for FilesystemWatcherConfig {
    fn default() -> Self {
        Self {
            debounce_delay_ms: 1000,    // 1 second
            max_concurrent_syncs: 2,
            recursive_watching: true,
            watch_patterns: vec![
                "*.js".to_string(),
                "*.json".to_string(),
                "*.yaml".to_string(),
                "*.yml".to_string(),
                "*.md".to_string(),
            ],
            ignore_patterns: vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
                "*~".to_string(),
            ],
            min_sync_interval_seconds: 30,
            enable_batching: true,
            batch_window_ms: 2000,      // 2 seconds
        }
    }
}

/// File system watch event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    /// Repository ID associated with this event
    pub repository_id: i32,
    /// Repository name
    pub repository_name: String,
    /// File path that changed
    pub file_path: PathBuf,
    /// Type of change (created, modified, deleted)
    pub event_type: WatchEventType,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Whether this event triggered a sync
    pub triggered_sync: bool,
}

/// Type of file system event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WatchEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Repository watch entry
#[derive(Debug, Clone)]
struct RepositoryWatch {
    repository_id: i32,
    repository_name: String,
    watch_path: PathBuf,
    patterns: Vec<String>,
    ignore_patterns: Vec<String>,
    last_sync_at: Option<SystemTime>,
}

/// Pending sync operation
#[derive(Debug, Clone)]
struct PendingSync {
    repository_id: i32,
    repository_name: String,
    triggered_at: SystemTime,
    events: Vec<WatchEvent>,
}

/// File system watcher for repository monitoring
pub struct FilesystemWatcher {
    /// Configuration
    config: FilesystemWatcherConfig,
    /// Repository service for sync operations
    repository_service: Arc<EnhancedRepositoryService>,
    /// Sync scheduler for triggering syncs
    sync_scheduler: Arc<SyncScheduler>,
    /// File system watcher
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    /// Repository watches by path
    repository_watches: Arc<RwLock<HashMap<PathBuf, RepositoryWatch>>>,
    /// Currently active sync operations
    active_syncs: Arc<RwLock<HashSet<i32>>>,
    /// Pending sync operations (for batching)
    pending_syncs: Arc<RwLock<HashMap<i32, PendingSync>>>,
    /// Event history for monitoring
    event_history: Arc<Mutex<Vec<WatchEvent>>>,
    /// Event sender channel
    event_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Event>>>>,
    /// Whether the watcher is running
    running: Arc<Mutex<bool>>,
}

impl FilesystemWatcher {
    /// Create a new filesystem watcher
    pub fn new(
        config: FilesystemWatcherConfig,
        repository_service: Arc<EnhancedRepositoryService>,
        sync_scheduler: Arc<SyncScheduler>,
    ) -> Self {
        Self {
            config,
            repository_service,
            sync_scheduler,
            watcher: Arc::new(Mutex::new(None)),
            repository_watches: Arc::new(RwLock::new(HashMap::new())),
            active_syncs: Arc::new(RwLock::new(HashSet::new())),
            pending_syncs: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(Mutex::new(Vec::new())),
            event_tx: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the filesystem watcher
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            if *running {
                return Err(anyhow!("Filesystem watcher is already running"));
            }
            *running = true;
        }

        info!("Starting filesystem watcher with config: {:?}", self.config);

        // Create event channel
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();
        {
            let mut tx_guard = self.event_tx.lock().await;
            *tx_guard = Some(event_tx.clone());
        }

        // Create file system watcher
        let watcher_config = Config::default()
            .with_poll_interval(Duration::from_millis(500))
            .with_compare_contents(true);

        let mut watcher = RecommendedWatcher::new(
            move |result: NotifyResult<Event>| {
                match result {
                    Ok(event) => {
                        if let Err(e) = event_tx.send(event) {
                            error!("Failed to send watch event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {}", e);
                    }
                }
            },
            watcher_config,
        ).context("Failed to create filesystem watcher")?;

        // Initialize repository watches
        self.initialize_repository_watches(&mut watcher).await?;

        // Store watcher
        {
            let mut watcher_guard = self.watcher.lock().await;
            *watcher_guard = Some(watcher);
        }

        // Start event processing loop
        let filesystem_watcher = self.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Err(e) = filesystem_watcher.handle_filesystem_event(event).await {
                    error!("Error handling filesystem event: {}", e);
                }
            }
        });

        // Start batch processing timer
        if self.config.enable_batching {
            let filesystem_watcher = self.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(
                    tokio::time::Duration::from_millis(filesystem_watcher.config.batch_window_ms)
                );
                
                loop {
                    interval.tick().await;
                    if let Err(e) = filesystem_watcher.process_pending_syncs().await {
                        error!("Error processing pending syncs: {}", e);
                    }
                }
            });
        }

        info!("Filesystem watcher started successfully");
        Ok(())
    }

    /// Stop the filesystem watcher
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping filesystem watcher");

        {
            let running = self.running.lock().await;
            if !*running {
                return Ok(());
            }
        }

        // Drop the watcher to stop watching
        {
            let mut watcher_guard = self.watcher.lock().await;
            *watcher_guard = None;
        }

        // Clear event channel
        {
            let mut tx_guard = self.event_tx.lock().await;
            *tx_guard = None;
        }

        {
            let mut running = self.running.lock().await;
            *running = false;
        }

        info!("Filesystem watcher stopped");
        Ok(())
    }

    /// Add a repository to watch
    pub async fn watch_repository(&self, repository_id: i32) -> Result<()> {
        let repository = self.repository_service.get_repository(repository_id).await
            .context("Failed to get repository")?
            .ok_or_else(|| anyhow!("Repository {} not found", repository_id))?;

        // Only watch filesystem repositories
        if repository.repository_type != "filesystem" {
            debug!("Skipping non-filesystem repository: {} ({})", repository.name, repository.repository_type);
            return Ok(());
        }

        let watch_path = PathBuf::from(&repository.uri);
        if !watch_path.exists() {
            warn!("Repository path does not exist: {}", watch_path.display());
            return Err(anyhow!("Repository path does not exist: {}", watch_path.display()));
        }

        info!("Adding filesystem watch for repository {} at path: {}", repository.name, watch_path.display());

        let repository_watch = RepositoryWatch {
            repository_id,
            repository_name: repository.name.clone(),
            watch_path: watch_path.clone(),
            patterns: repository.watch_patterns.clone(),
            ignore_patterns: repository.ignore_patterns.clone(),
            last_sync_at: None,
        };

        // Add to repository watches
        {
            let mut watches = self.repository_watches.write().await;
            watches.insert(watch_path.clone(), repository_watch);
        }

        // Add to file system watcher if running
        if let Some(watcher) = self.watcher.lock().await.as_mut() {
            let recursive_mode = if self.config.recursive_watching {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };

            watcher.watch(&watch_path, recursive_mode)
                .context("Failed to add path to filesystem watcher")?;

            info!("Successfully added filesystem watch for repository {} at path: {}", 
                repository.name, watch_path.display());
        }

        Ok(())
    }

    /// Remove a repository from watching
    pub async fn unwatch_repository(&self, repository_id: i32) -> Result<()> {
        let mut path_to_remove = None;

        {
            let watches = self.repository_watches.read().await;
            for (path, watch) in watches.iter() {
                if watch.repository_id == repository_id {
                    path_to_remove = Some(path.clone());
                    break;
                }
            }
        }

        if let Some(path) = path_to_remove {
            info!("Removing filesystem watch for repository {} at path: {}", repository_id, path.display());

            // Remove from repository watches
            {
                let mut watches = self.repository_watches.write().await;
                watches.remove(&path);
            }

            // Remove from file system watcher if running
            if let Some(watcher) = self.watcher.lock().await.as_mut() {
                watcher.unwatch(&path)
                    .context("Failed to remove path from filesystem watcher")?;
            }

            info!("Successfully removed filesystem watch for repository {}", repository_id);
        }

        Ok(())
    }

    /// Get watch events history
    pub async fn get_event_history(&self, limit: Option<usize>) -> Vec<WatchEvent> {
        let history = self.event_history.lock().await;
        let limit = limit.unwrap_or(100);
        
        if history.len() <= limit {
            history.clone()
        } else {
            history[history.len() - limit..].to_vec()
        }
    }

    /// Get current watch status
    pub async fn get_watch_status(&self) -> HashMap<i32, (String, PathBuf)> {
        let watches = self.repository_watches.read().await;
        watches.iter()
            .map(|(path, watch)| (watch.repository_id, (watch.repository_name.clone(), path.clone())))
            .collect()
    }

    /// Initialize repository watches from database
    async fn initialize_repository_watches(&self, watcher: &mut RecommendedWatcher) -> Result<()> {
        info!("Initializing repository filesystem watches");

        let repositories = self.repository_service.list_repositories().await
            .context("Failed to list repositories")?;

        let mut initialized_count = 0;

        for repository in repositories {
            if repository.repository_type == "filesystem" {
                let watch_path = PathBuf::from(&repository.uri);
                
                if watch_path.exists() {
                    let repository_watch = RepositoryWatch {
                        repository_id: repository.id.as_i32().unwrap_or(0),
                        repository_name: repository.name.clone(),
                        watch_path: watch_path.clone(),
                        patterns: repository.watch_patterns.clone(),
                        ignore_patterns: repository.ignore_patterns.clone(),
                        last_sync_at: None,
                    };

                    // Add to repository watches
                    {
                        let mut watches = self.repository_watches.write().await;
                        watches.insert(watch_path.clone(), repository_watch);
                    }

                    // Add to file system watcher
                    let recursive_mode = if self.config.recursive_watching {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };

                    if let Err(e) = watcher.watch(&watch_path, recursive_mode) {
                        warn!("Failed to watch repository {} at path {}: {}", 
                            repository.name, watch_path.display(), e);
                    } else {
                        initialized_count += 1;
                        debug!("Added filesystem watch for repository {} at path: {}", 
                            repository.name, watch_path.display());
                    }
                } else {
                    warn!("Repository path does not exist: {} for repository {}", 
                        watch_path.display(), repository.name);
                }
            }
        }

        info!("Initialized {} filesystem repository watches", initialized_count);
        Ok(())
    }

    /// Handle a filesystem event
    async fn handle_filesystem_event(&self, event: Event) -> Result<()> {
        // Filter events by kind - check specific modify types for renames
        let event_type = match event.kind {
            EventKind::Create(_) => WatchEventType::Created,
            EventKind::Modify(modify_kind) => {
                // Check if this is a rename by looking at modify kind
                match modify_kind {
                    notify::event::ModifyKind::Name(_) => WatchEventType::Renamed,
                    _ => WatchEventType::Modified,
                }
            },
            EventKind::Remove(_) => WatchEventType::Deleted,
            EventKind::Other => WatchEventType::Modified, // Treat Other as generic modification
            _ => return Ok(()), // Ignore access events and other types
        };

        // Process each path in the event
        for path in event.paths {
            if let Err(e) = self.handle_path_event(&path, event_type.clone()).await {
                error!("Error handling path event for {}: {}", path.display(), e);
            }
        }

        Ok(())
    }

    /// Handle an event for a specific path
    async fn handle_path_event(&self, path: &Path, event_type: WatchEventType) -> Result<()> {
        // Find the repository watch for this path
        let repository_watch = {
            let watches = self.repository_watches.read().await;
            watches.iter()
                .find(|(watch_path, _)| path.starts_with(watch_path))
                .map(|(_, watch)| watch.clone())
        };

        let repository_watch = match repository_watch {
            Some(watch) => watch,
            None => {
                debug!("No repository watch found for path: {}", path.display());
                return Ok(());
            }
        };

        // Check if file matches patterns
        if !self.should_watch_file(path, &repository_watch.patterns, &repository_watch.ignore_patterns) {
            debug!("File {} does not match watch patterns", path.display());
            return Ok(());
        }

        debug!("Filesystem event: {:?} for file {} in repository {}", 
            event_type, path.display(), repository_watch.repository_name);

        // Create watch event
        let watch_event = WatchEvent {
            repository_id: repository_watch.repository_id,
            repository_name: repository_watch.repository_name.clone(),
            file_path: path.to_path_buf(),
            event_type,
            timestamp: Utc::now(),
            triggered_sync: false,
        };

        // Store in history
        {
            let mut history = self.event_history.lock().await;
            history.push(watch_event.clone());
            // Keep only last 1000 events
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        // Handle sync triggering
        if self.config.enable_batching {
            self.add_to_pending_sync(watch_event).await?;
        } else {
            self.trigger_immediate_sync(watch_event).await?;
        }

        Ok(())
    }

    /// Check if a file should be watched based on patterns
    fn should_watch_file(&self, path: &Path, watch_patterns: &[String], ignore_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        // Check ignore patterns first
        for pattern in ignore_patterns {
            if glob_match::glob_match(pattern, &path_str) {
                return false;
            }
        }

        // Check watch patterns
        for pattern in watch_patterns {
            if glob_match::glob_match(pattern, &path_str) {
                return true;
            }
        }

        // If no specific patterns, use config patterns
        for pattern in &self.config.ignore_patterns {
            if glob_match::glob_match(pattern, &path_str) {
                return false;
            }
        }

        for pattern in &self.config.watch_patterns {
            if glob_match::glob_match(pattern, &path_str) {
                return true;
            }
        }

        false
    }

    /// Add event to pending sync batch
    async fn add_to_pending_sync(&self, mut watch_event: WatchEvent) -> Result<()> {
        let now = SystemTime::now();
        
        {
            let mut pending = self.pending_syncs.write().await;
            
            if let Some(pending_sync) = pending.get_mut(&watch_event.repository_id) {
                // Add to existing pending sync
                pending_sync.events.push(watch_event);
            } else {
                // Create new pending sync
                watch_event.triggered_sync = true;
                let pending_sync = PendingSync {
                    repository_id: watch_event.repository_id,
                    repository_name: watch_event.repository_name.clone(),
                    triggered_at: now,
                    events: vec![watch_event],
                };
                pending.insert(pending_sync.repository_id, pending_sync);
            }
        }

        Ok(())
    }

    /// Process pending sync operations
    async fn process_pending_syncs(&self) -> Result<()> {
        let now = SystemTime::now();
        let batch_window = Duration::from_millis(self.config.batch_window_ms);
        
        let mut syncs_to_trigger = Vec::new();

        {
            let mut pending = self.pending_syncs.write().await;
            let mut to_remove = Vec::new();

            for (repo_id, pending_sync) in pending.iter() {
                if now.duration_since(pending_sync.triggered_at).unwrap_or_default() >= batch_window {
                    syncs_to_trigger.push(pending_sync.clone());
                    to_remove.push(*repo_id);
                }
            }

            for repo_id in to_remove {
                pending.remove(&repo_id);
            }
        }

        // Trigger syncs
        for pending_sync in syncs_to_trigger {
            if let Err(e) = self.trigger_repository_sync(pending_sync.repository_id, pending_sync.events).await {
                error!("Failed to trigger sync for repository {}: {}", pending_sync.repository_id, e);
            }
        }

        Ok(())
    }

    /// Trigger immediate sync for a watch event
    async fn trigger_immediate_sync(&self, mut watch_event: WatchEvent) -> Result<()> {
        watch_event.triggered_sync = true;
        self.trigger_repository_sync(watch_event.repository_id, vec![watch_event]).await
    }

    /// Trigger sync for a repository
    async fn trigger_repository_sync(&self, repository_id: i32, events: Vec<WatchEvent>) -> Result<()> {
        // Check minimum sync interval
        {
            let watches = self.repository_watches.read().await;
            if let Some(watch) = watches.values().find(|w| w.repository_id == repository_id) {
                if let Some(last_sync) = watch.last_sync_at {
                    let min_interval = Duration::from_secs(self.config.min_sync_interval_seconds);
                    if SystemTime::now().duration_since(last_sync).unwrap_or_default() < min_interval {
                        debug!("Skipping sync for repository {} due to minimum interval", repository_id);
                        return Ok(());
                    }
                }
            }
        }

        // Check if sync is already active
        {
            let active_syncs = self.active_syncs.read().await;
            if active_syncs.contains(&repository_id) {
                debug!("Sync already active for repository {}", repository_id);
                return Ok(());
            }
        }

        // Check concurrent sync limit
        {
            let active_syncs = self.active_syncs.read().await;
            if active_syncs.len() >= self.config.max_concurrent_syncs {
                debug!("Maximum concurrent syncs reached, skipping repository {}", repository_id);
                return Ok(());
            }
        }

        info!("Triggering filesystem-initiated sync for repository {} due to {} file changes", 
            repository_id, events.len());

        // Mark sync as active
        {
            let mut active_syncs = self.active_syncs.write().await;
            active_syncs.insert(repository_id);
        }

        // Update last sync time
        {
            let mut watches = self.repository_watches.write().await;
            for watch in watches.values_mut() {
                if watch.repository_id == repository_id {
                    watch.last_sync_at = Some(SystemTime::now());
                    break;
                }
            }
        }

        // Trigger sync via scheduler
        let filesystem_watcher = self.clone();
        tokio::spawn(async move {
            let result = filesystem_watcher.sync_scheduler.trigger_immediate_sync(repository_id).await;
            
            // Remove from active syncs
            {
                let mut active_syncs = filesystem_watcher.active_syncs.write().await;
                active_syncs.remove(&repository_id);
            }

            match result {
                Ok(_) => {
                    info!("Filesystem-initiated sync completed for repository {}", repository_id);
                }
                Err(e) => {
                    error!("Filesystem-initiated sync failed for repository {}: {}", repository_id, e);
                }
            }
        });

        Ok(())
    }
}

// Implement Clone for FilesystemWatcher to allow sharing between tasks
impl Clone for FilesystemWatcher {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            repository_service: self.repository_service.clone(),
            sync_scheduler: self.sync_scheduler.clone(),
            watcher: self.watcher.clone(),
            repository_watches: self.repository_watches.clone(),
            active_syncs: self.active_syncs.clone(),
            pending_syncs: self.pending_syncs.clone(),
            event_history: self.event_history.clone(),
            event_tx: self.event_tx.clone(),
            running: self.running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    
    
    // TODO: Add comprehensive tests for filesystem watcher
    // This would include:
    // - File pattern matching
    // - Event handling and debouncing
    // - Sync triggering logic
    // - Batch processing
    // - Error handling scenarios
}