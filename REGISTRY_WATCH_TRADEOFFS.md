# Registry File Watcher - Options & Tradeoffs Summary

## Quick Comparison Table

| Aspect | notify-rs | Polling | Hybrid |
|--------|-----------|---------|--------|
| **Performance** | ⭐⭐⭐⭐⭐ Native OS APIs | ⭐⭐ High CPU/IO usage | ⭐⭐⭐⭐ Platform-optimized |
| **Responsiveness** | ⭐⭐⭐⭐⭐ Real-time | ⭐⭐ Delayed (poll interval) | ⭐⭐⭐⭐⭐ Real-time where supported |
| **Cross-platform** | ⭐⭐⭐⭐ Good (some quirks) | ⭐⭐⭐⭐⭐ Consistent | ⭐⭐⭐⭐ Good |
| **Complexity** | ⭐⭐⭐ Moderate | ⭐⭐⭐⭐⭐ Simple | ⭐⭐ Complex |
| **Dependencies** | ⭐⭐⭐ One major dep | ⭐⭐⭐⭐⭐ None | ⭐⭐⭐ One major dep |
| **Reliability** | ⭐⭐⭐⭐ May miss events under load | ⭐⭐⭐⭐⭐ Won't miss changes | ⭐⭐⭐⭐ Platform-dependent |

## Key Tradeoffs

### 1. **Dependency vs Performance**
- **notify-rs**: Adds external dependency but provides excellent performance
- **Polling**: No dependencies but wastes resources
- **Decision Factor**: How important is minimizing dependencies vs resource efficiency?

### 2. **Complexity vs Consistency**
- **notify-rs**: Platform-specific behaviors require handling
- **Polling**: Same behavior everywhere but inefficient
- **Decision Factor**: Is consistent behavior worth the performance cost?

### 3. **Real-time vs Reliability**
- **notify-rs**: Instant updates but might miss events under extreme load
- **Polling**: Guaranteed to catch changes but with delay
- **Decision Factor**: Is real-time response critical or is eventual consistency OK?

### 4. **Implementation Time**
- **notify-rs**: 1-2 weeks for robust implementation
- **Polling**: 3-4 days for basic implementation
- **Hybrid**: 2-3 weeks due to dual code paths

## Platform-Specific Considerations

### Linux (inotify)
- **Limit**: Default 8192 watches per user
- **Impact**: Large task directories might hit limits
- **Mitigation**: Increase limits or watch selectively

### macOS (FSEvents)
- **Issue**: Events are coalesced, might batch multiple changes
- **Impact**: Fine-grained change detection harder
- **Mitigation**: Smart debouncing and change detection

### Windows (ReadDirectoryChangesW)
- **Issue**: Buffer overflows lose events
- **Impact**: Rapid changes might be missed
- **Mitigation**: Larger buffers, careful event handling

## Resource Impact Comparison

### Memory Usage
- **notify-rs**: ~1MB + OS buffers per watched directory
- **Polling**: ~100KB + cached file metadata
- **At 1000 tasks**: notify-rs uses 10-50MB, polling uses 5-10MB

### CPU Usage
- **notify-rs**: Near 0% idle, spikes on changes
- **Polling (1s interval)**: 1-5% constant
- **Polling (10s interval)**: 0.1-0.5% constant

### Disk I/O
- **notify-rs**: Only on actual changes
- **Polling**: Constant metadata reads

## Failure Modes & Recovery

### notify-rs Failures
1. **Watcher limit reached**: Fallback to polling specific dirs
2. **OS API errors**: Retry with exponential backoff
3. **Event buffer overflow**: Request manual refresh

### Polling Failures
1. **High disk latency**: Increase poll interval dynamically
2. **Permission errors**: Skip and log
3. **Too many files**: Implement progressive scanning

## Recommended Approach

**For Production**: notify-rs with careful error handling
- Best performance for typical use cases
- Well-tested in many production systems
- Good ecosystem support

**For Simple/Embedded**: Polling with 5-10 second interval
- Predictable resource usage
- No external dependencies
- Easier to debug

**Decision Tree**:
```
Is this for production use?
├─ Yes → How many tasks/files?
│  ├─ <100 → Either approach works
│  ├─ 100-1000 → notify-rs recommended
│  └─ >1000 → notify-rs required
└─ No → Polling is simpler to implement
```

## Implementation Priorities

### Must Have (Week 1)
1. Basic file watching (add/modify/delete detection)
2. Debouncing to prevent reload storms
3. Error handling without crashing
4. Database synchronization

### Should Have (Week 2)
1. Platform-specific optimizations
2. Configurable ignore patterns
3. Metrics/monitoring
4. GraphQL notifications

### Nice to Have (Future)
1. Hot reload running tasks
2. Dependency tracking
3. Git integration
4. Watch pause/resume API