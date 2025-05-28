# Registry File System Watcher Implementation Plan

## Overview
Implement a file system watcher for the registry that monitors task directories and automatically synchronizes changes (additions, modifications, deletions) with the internal registry and database.

## Core Requirements
1. Monitor filesystem sources when `config.watch: true`
2. Detect task additions, modifications, and deletions
3. Automatically reload affected tasks
4. Synchronize changes with database via TaskSyncService
5. Maintain system stability during reloads
6. Cross-platform compatibility (Linux, macOS, Windows)

## Implementation Options

### Option 1: notify-rs Based Implementation (Recommended)
**Library**: [notify](https://github.com/notify-rs/notify) v6.x

**Pros**:
- Mature, well-tested cross-platform solution
- Supports inotify (Linux), FSEvents (macOS), ReadDirectoryChangesW (Windows)
- Debouncing support to handle rapid changes
- Recursive directory watching
- Good performance with native OS APIs

**Cons**:
- Additional dependency
- Platform-specific quirks require handling
- Some events may be lost under high load

**Implementation approach**:
```rust
// Add to Cargo.toml
notify = { version = "6.1", features = ["serde"] }

// Create a RegistryWatcher component
pub struct RegistryWatcher {
    watcher: RecommendedWatcher,
    registry_service: Arc<RwLock<dyn RegistryService>>,
    debouncer: Debouncer,
}
```

### Option 2: tokio-based Polling Implementation
**Approach**: Use tokio intervals to poll directories for changes

**Pros**:
- No additional dependencies
- Consistent behavior across platforms
- Simple implementation
- Can control polling frequency

**Cons**:
- Higher resource usage (CPU/disk I/O)
- Delayed change detection (based on poll interval)
- May miss rapid changes between polls
- Not suitable for large directory trees

**Implementation approach**:
```rust
// Use tokio::time::interval
pub struct PollingWatcher {
    paths: Vec<PathBuf>,
    last_scan_state: HashMap<PathBuf, FileMetadata>,
    poll_interval: Duration,
}
```

### Option 3: Hybrid Approach
**Approach**: Use notify-rs where available, fall back to polling

**Pros**:
- Best of both worlds
- Graceful degradation
- Can optimize per platform

**Cons**:
- More complex implementation
- Two code paths to maintain
- Potential behavior differences

## Detailed Design (Recommended Option 1)

### 1. Registry Watcher Component

```rust
// ratchet-lib/src/registry/watcher.rs
use notify::{RecommendedWatcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;

pub struct RegistryWatcher {
    watcher: Option<RecommendedWatcher>,
    registry_service: Arc<RwLock<DefaultRegistryService>>,
    sync_service: Option<Arc<TaskSyncService>>,
    watch_paths: Vec<(PathBuf, bool)>, // (path, recursive)
    event_tx: mpsc::UnboundedSender<WatchEvent>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[derive(Debug)]
pub enum WatchEvent {
    TaskAdded(PathBuf),
    TaskModified(PathBuf),
    TaskRemoved(PathBuf),
    BulkChange(Vec<PathBuf>),
}

impl RegistryWatcher {
    pub async fn start(&mut self) -> Result<(), WatchError> {
        // Set up notify watcher
        // Start event processing task
        // Configure debouncing
    }
    
    async fn process_events(&self, mut event_rx: mpsc::UnboundedReceiver<WatchEvent>) {
        // Debounce events
        // Batch related changes
        // Trigger registry updates
    }
    
    async fn reload_task(&self, path: &Path) -> Result<(), RegistryError> {
        // Load task from filesystem
        // Update registry
        // Sync with database
    }
}
```

### 2. Integration Points

#### 2.1 Modify DefaultRegistryService
```rust
impl DefaultRegistryService {
    pub fn with_watcher(mut self, watcher_config: WatcherConfig) -> Self {
        self.watcher = Some(RegistryWatcher::new(watcher_config));
        self
    }
    
    pub async fn start_watching(&mut self) -> Result<(), ServiceError> {
        if let Some(watcher) = &mut self.watcher {
            watcher.start().await?;
        }
        Ok(())
    }
}
```

#### 2.2 Add GraphQL Subscriptions
```rust
// GraphQL subscription for registry changes
#[Subscription]
impl RegistrySubscription {
    async fn registry_changes(&self) -> impl Stream<Item = RegistryChangeEvent> {
        // Stream registry change events
    }
}
```

### 3. Change Detection Strategy

#### File System Event Mapping
- **Create/Add**: New `metadata.json` → Task addition
- **Modify**: Changes to any task file → Task reload
- **Delete**: Removed `metadata.json` → Task removal
- **Rename**: Treated as delete + add

#### Debouncing Strategy
- Collect events for 500ms before processing
- Batch changes to same task directory
- Ignore temporary files (`.tmp`, `.swp`, etc.)

### 4. Error Handling & Recovery

#### Graceful Degradation
1. If watcher fails to start → Log warning, continue without watching
2. If reload fails → Keep existing task version, log error
3. If sync fails → Retry with exponential backoff

#### Validation Before Reload
1. Validate task structure before replacing
2. Validate JavaScript syntax
3. Validate schemas
4. Keep previous version on validation failure

### 5. Performance Considerations

#### Resource Management
- Limit concurrent reloads (e.g., max 5)
- Use read-write locks for registry access
- Cache file metadata to detect actual changes

#### Optimization Strategies
1. **Lazy Loading**: Only reload when task is accessed
2. **Incremental Updates**: Only reload changed files
3. **Smart Caching**: Keep compiled JS in memory

## Implementation Phases

### Phase 1: Basic File Watching (2-3 days)
- [ ] Add notify-rs dependency
- [ ] Create RegistryWatcher component
- [ ] Implement basic file system monitoring
- [ ] Add debouncing logic

### Phase 2: Registry Integration (2-3 days)
- [ ] Integrate watcher with DefaultRegistryService
- [ ] Implement task reload logic
- [ ] Add configuration support
- [ ] Handle add/modify/delete events

### Phase 3: Database Synchronization (1-2 days)
- [ ] Update TaskSyncService for incremental updates
- [ ] Add transaction support for atomic updates
- [ ] Implement conflict resolution

### Phase 4: Error Handling & Testing (2-3 days)
- [ ] Add comprehensive error handling
- [ ] Implement retry mechanisms
- [ ] Add integration tests
- [ ] Test on all platforms

### Phase 5: Advanced Features (Optional, 3-4 days)
- [ ] GraphQL subscriptions for changes
- [ ] Web UI notifications
- [ ] Metrics and monitoring
- [ ] Hot reload for running tasks

## Configuration Schema

```yaml
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./tasks"
      config:
        watch: true
        watch_options:
          debounce_ms: 500
          ignore_patterns:
            - "*.tmp"
            - ".git/**"
          max_concurrent_reloads: 5
          retry_on_error: true
          retry_delay_ms: 1000
```

## Testing Strategy

### Unit Tests
- Mock file system events
- Test debouncing logic
- Test event mapping
- Test error scenarios

### Integration Tests
- Create temporary directories
- Simulate file operations
- Verify registry updates
- Test database synchronization

### Platform Tests
- Linux: inotify limits
- macOS: FSEvents quirks
- Windows: Path handling

## Security Considerations

1. **Path Traversal**: Validate all paths are within configured directories
2. **Symbolic Links**: Decide whether to follow (security vs functionality)
3. **File Permissions**: Handle permission errors gracefully
4. **Resource Limits**: Prevent DoS through rapid file changes

## Alternatives Considered

### 1. Git-based Watching
- Watch `.git` directory for changes
- Pro: Version control integration
- Con: Requires git, doesn't work for non-git directories

### 2. External Watcher Process
- Separate process/service for watching
- Pro: Isolation, language agnostic
- Con: IPC complexity, deployment complexity

### 3. Database Triggers
- Store tasks in database, use triggers
- Pro: Transactional consistency
- Con: Database-specific, not file-system based

## Recommendation

Implement **Option 1 (notify-rs)** with the following priorities:
1. Start with basic watching for a single directory
2. Add debouncing and error handling
3. Integrate with registry and database
4. Add advanced features incrementally

This provides the best balance of:
- Performance (native OS APIs)
- Maintainability (single well-tested dependency)
- User experience (near real-time updates)
- Cross-platform support

## Open Questions

1. Should we support watching ZIP files for changes?
2. How to handle tasks that are temporarily invalid during editing?
3. Should we version tasks automatically on each change?
4. What metrics should we expose for monitoring?
5. Should watch mode be toggleable at runtime via API?