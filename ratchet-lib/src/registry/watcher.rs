use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use crate::errors::RatchetError;
use crate::registry::TaskRegistry;
use crate::services::TaskSyncService;
use crate::task::Task;

/// Configuration for the registry watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Whether to enable watching
    pub enabled: bool,
    /// Debounce period in milliseconds
    pub debounce_ms: u64,
    /// Patterns to ignore (glob patterns)
    pub ignore_patterns: Vec<String>,
    /// Maximum concurrent task reloads
    pub max_concurrent_reloads: usize,
    /// Whether to retry on errors
    pub retry_on_error: bool,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 500,
            ignore_patterns: vec![
                "*.tmp".to_string(),
                "*.swp".to_string(),
                ".git/**".to_string(),
                ".DS_Store".to_string(),
            ],
            max_concurrent_reloads: 5,
            retry_on_error: true,
            retry_delay_ms: 1000,
        }
    }
}

/// Events that can occur in the registry watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A task was added (new directory with metadata.json)
    TaskAdded(PathBuf),
    /// A task was modified (files within task directory changed)
    TaskModified(PathBuf),
    /// A task was removed (directory or metadata.json deleted)
    TaskRemoved(PathBuf),
    /// Multiple changes detected, batch reload
    BulkChange(Vec<PathBuf>),
}

/// Registry watcher that monitors filesystem changes
pub struct RegistryWatcher {
    /// The underlying notify watcher
    watcher: Option<RecommendedWatcher>,
    /// Reference to the registry
    registry: Arc<TaskRegistry>,
    /// Optional sync service for database synchronization
    sync_service: Option<Arc<TaskSyncService>>,
    /// Paths being watched with their recursive flag
    watch_paths: Vec<(PathBuf, bool)>,
    /// Channel for sending watch events
    event_tx: mpsc::UnboundedSender<WatchEvent>,
    /// Channel for receiving watch events
    event_rx: Option<mpsc::UnboundedReceiver<WatchEvent>>,
    /// Configuration
    config: WatcherConfig,
    /// Shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Task for processing events
    processor_handle: Option<tokio::task::JoinHandle<()>>,
}

impl RegistryWatcher {
    /// Create a new registry watcher
    pub fn new(
        registry: Arc<TaskRegistry>,
        sync_service: Option<Arc<TaskSyncService>>,
        config: WatcherConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            watcher: None,
            registry,
            sync_service,
            watch_paths: Vec::new(),
            event_tx,
            event_rx: Some(event_rx),
            config,
            shutdown_tx: None,
            processor_handle: None,
        }
    }

    /// Add a path to watch
    pub fn add_watch_path(&mut self, path: PathBuf, recursive: bool) {
        self.watch_paths.push((path, recursive));
    }

    /// Start watching for changes
    pub async fn start(&mut self) -> Result<(), RatchetError> {
        if !self.config.enabled {
            info!("Registry watching is disabled");
            return Ok(());
        }

        info!("Starting registry watcher");

        // Create the notify watcher
        let event_tx = self.event_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = Self::handle_notify_event(event, &event_tx) {
                            error!("Failed to handle notify event: {}", e);
                        }
                    }
                    Err(e) => error!("Notify error: {}", e),
                }
            },
            Config::default(),
        ).map_err(|e| RatchetError::WatcherError(format!("Failed to create watcher: {}", e)))?;

        // Add all watch paths
        for (path, recursive) in &self.watch_paths {
            let mode = if *recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            
            watcher.watch(path, mode)
                .map_err(|e| RatchetError::WatcherError(
                    format!("Failed to watch path {:?}: {}", path, e)
                ))?;
            
            info!("Watching path: {:?} (recursive: {})", path, recursive);
        }

        self.watcher = Some(watcher);

        // Start the event processor
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        if let Some(event_rx) = self.event_rx.take() {
            let processor = EventProcessor {
                registry: self.registry.clone(),
                sync_service: self.sync_service.clone(),
                config: self.config.clone(),
            };

            let handle = tokio::spawn(async move {
                processor.run(event_rx, shutdown_rx).await;
            });

            self.processor_handle = Some(handle);
        }

        info!("Registry watcher started successfully");
        Ok(())
    }

    /// Stop watching for changes
    pub async fn stop(&mut self) -> Result<(), RatchetError> {
        info!("Stopping registry watcher");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for processor to finish
        if let Some(handle) = self.processor_handle.take() {
            let _ = timeout(Duration::from_secs(5), handle).await;
        }

        // Drop the watcher
        self.watcher = None;

        info!("Registry watcher stopped");
        Ok(())
    }

    /// Handle a notify event and convert to our event type
    fn handle_notify_event(
        event: Event,
        event_tx: &mpsc::UnboundedSender<WatchEvent>,
    ) -> Result<(), RatchetError> {
        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    if Self::is_task_directory(&path) {
                        debug!("Task added: {:?}", path);
                        event_tx.send(WatchEvent::TaskAdded(path))
                            .map_err(|e| RatchetError::WatcherError(e.to_string()))?;
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    if let Some(task_dir) = Self::find_task_directory(&path) {
                        debug!("Task modified: {:?}", task_dir);
                        event_tx.send(WatchEvent::TaskModified(task_dir))
                            .map_err(|e| RatchetError::WatcherError(e.to_string()))?;
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if Self::is_task_directory(&path) || path.ends_with("metadata.json") {
                        let task_dir = if path.ends_with("metadata.json") {
                            path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
                        } else {
                            path
                        };
                        debug!("Task removed: {:?}", task_dir);
                        event_tx.send(WatchEvent::TaskRemoved(task_dir))
                            .map_err(|e| RatchetError::WatcherError(e.to_string()))?;
                    }
                }
            }
            _ => {
                // Ignore other events like Access
            }
        }

        Ok(())
    }

    /// Check if a path is a task directory (contains metadata.json)
    fn is_task_directory(path: &Path) -> bool {
        path.join("metadata.json").exists()
    }

    /// Find the task directory for a given path
    fn find_task_directory(path: &Path) -> Option<PathBuf> {
        let mut current = path;
        
        // Walk up the directory tree looking for metadata.json
        while let Some(parent) = current.parent() {
            if parent.join("metadata.json").exists() {
                return Some(parent.to_path_buf());
            }
            current = parent;
        }
        
        // Check if the path itself is a task directory
        if path.join("metadata.json").exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

/// Event processor that handles watch events with debouncing
struct EventProcessor {
    registry: Arc<TaskRegistry>,
    sync_service: Option<Arc<TaskSyncService>>,
    config: WatcherConfig,
}

impl EventProcessor {
    /// Run the event processor
    async fn run(
        self,
        mut event_rx: mpsc::UnboundedReceiver<WatchEvent>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let mut pending_events: HashMap<PathBuf, WatchEvent> = HashMap::new();
        let mut debounce_interval = interval(Duration::from_millis(self.config.debounce_ms));

        loop {
            tokio::select! {
                // Process incoming events
                Some(event) = event_rx.recv() => {
                    self.buffer_event(&mut pending_events, event);
                }
                
                // Process buffered events after debounce period
                _ = debounce_interval.tick() => {
                    if !pending_events.is_empty() {
                        self.process_pending_events(&mut pending_events).await;
                    }
                }
                
                // Shutdown signal
                _ = &mut shutdown_rx => {
                    info!("Event processor shutting down");
                    break;
                }
            }
        }

        // Process any remaining events
        if !pending_events.is_empty() {
            self.process_pending_events(&mut pending_events).await;
        }
    }

    /// Buffer an event for debouncing
    fn buffer_event(&self, pending: &mut HashMap<PathBuf, WatchEvent>, event: WatchEvent) {
        match event {
            WatchEvent::TaskAdded(path) => {
                pending.insert(path.clone(), WatchEvent::TaskAdded(path));
            }
            WatchEvent::TaskModified(path) => {
                // If we already have an add event, keep it
                if !matches!(pending.get(&path), Some(WatchEvent::TaskAdded(_))) {
                    pending.insert(path.clone(), WatchEvent::TaskModified(path));
                }
            }
            WatchEvent::TaskRemoved(path) => {
                // Remove always takes precedence
                pending.insert(path.clone(), WatchEvent::TaskRemoved(path));
            }
            WatchEvent::BulkChange(paths) => {
                for path in paths {
                    self.buffer_event(pending, WatchEvent::TaskModified(path));
                }
            }
        }
    }

    /// Process all pending events
    async fn process_pending_events(&self, pending: &mut HashMap<PathBuf, WatchEvent>) {
        let events: Vec<_> = pending.drain().map(|(_, event)| event).collect();
        
        info!("Processing {} file system events", events.len());

        // Limit concurrent reloads
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_reloads));
        let mut handles = Vec::new();

        for event in events {
            let semaphore = semaphore.clone();
            let registry = self.registry.clone();
            let sync_service = self.sync_service.clone();
            let retry_on_error = self.config.retry_on_error;
            let retry_delay_ms = self.config.retry_delay_ms;

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                match event {
                    WatchEvent::TaskAdded(path) | WatchEvent::TaskModified(path) => {
                        if let Err(e) = Self::reload_task(
                            &path,
                            registry,
                            sync_service,
                            retry_on_error,
                            retry_delay_ms,
                        ).await {
                            error!("Failed to reload task at {:?}: {}", path, e);
                        }
                    }
                    WatchEvent::TaskRemoved(path) => {
                        if let Err(e) = Self::remove_task(&path, registry, sync_service).await {
                            error!("Failed to remove task at {:?}: {}", path, e);
                        }
                    }
                    WatchEvent::BulkChange(_) => {
                        // Should have been expanded in buffer_event
                        unreachable!("BulkChange should have been expanded");
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Reload a task from the filesystem
    async fn reload_task(
        path: &Path,
        registry: Arc<TaskRegistry>,
        sync_service: Option<Arc<TaskSyncService>>,
        retry_on_error: bool,
        retry_delay_ms: u64,
    ) -> Result<(), RatchetError> {
        let mut attempts = 0;
        let max_attempts = if retry_on_error { 3 } else { 1 };

        while attempts < max_attempts {
            attempts += 1;

            match Task::from_fs(path) {
                Ok(task) => {
                    info!("Reloading task: {} ({})", task.metadata.label, task.metadata.uuid);

                    // Update registry
                    if let Err(e) = registry.add_task(task.clone()).await {
                        error!("Failed to add task to registry: {}", e);
                        if attempts < max_attempts {
                            tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
                            continue;
                        }
                        return Err(RatchetError::LoadError(format!(
                            "Failed to add task to registry: {}",
                            e
                        )));
                    }

                    // Sync with database if available
                    if let Some(sync) = &sync_service {
                        if let Err(e) = sync.sync_task_to_db(&task).await {
                            error!("Failed to sync task to database: {}", e);
                            // Continue anyway - registry is updated
                        }
                    }

                    return Ok(());
                }
                Err(e) => {
                    if attempts < max_attempts {
                        warn!(
                            "Failed to reload task from {:?} (attempt {}/{}): {}",
                            path, attempts, max_attempts, e
                        );
                        tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
                    } else {
                        return Err(RatchetError::LoadError(format!(
                            "Failed to reload task after {} attempts: {}",
                            max_attempts, e
                        )));
                    }
                }
            }
        }

        unreachable!("Should have returned from loop");
    }

    /// Remove a task from the registry
    async fn remove_task(
        path: &Path,
        registry: Arc<TaskRegistry>,
        _sync_service: Option<Arc<TaskSyncService>>,
    ) -> Result<(), RatchetError> {
        // Extract task UUID from path
        // This is a simplification - in practice you might need to track path->UUID mapping
        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| RatchetError::LoadError("Invalid path".to_string()))?;

        info!("Removing task from directory: {}", dir_name);

        // Try to find the task by iterating through all tasks
        // In a real implementation, you'd want to maintain a path->UUID mapping
        let tasks = registry.list_tasks().await?;
        
        for task in tasks {
            // Check if this task's path matches
            // This is a heuristic - comparing directory names
            if let Some(task_dir) = path.file_name().and_then(|n| n.to_str()) {
                if task.metadata.uuid.to_string() == task_dir {
                    info!("Found task to remove: {} ({})", task.metadata.label, task.metadata.uuid);
                    
                    // Remove from registry
                    if let Err(e) = registry.remove_task(&task.metadata.uuid).await {
                        return Err(RatchetError::LoadError(format!(
                            "Failed to remove task from registry: {}",
                            e
                        )));
                    }
                    
                    // TODO: Sync removal with database if available
                    // The sync service doesn't currently have a remove method
                    // This would need to be added to properly sync deletions
                    
                    return Ok(());
                }
            }
        }
        
        warn!("Could not find task to remove for path: {:?}", path);
        Ok(())
    }
}