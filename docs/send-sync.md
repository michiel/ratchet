# Send+Sync Architecture Options

## Problem Overview

The current architecture has a fundamental incompatibility between the **Boa JavaScript engine** (which doesn't implement Send+Sync) and **GraphQL/Axum requirements** (which require Send+Sync for multi-threaded operation). This creates limitations in the GraphQL API layer where complex operations must use placeholder implementations.

## Current State

- ✅ Basic GraphQL endpoints working (`/graphql`, `/playground`, `/health`, `/version`)
- ❌ Task execution via GraphQL returns placeholder data
- ❌ Real-time operations limited due to Send+Sync constraints
- ❌ No true parallelism for JavaScript task execution

## Decision Options

### Option 1: Process Separation Architecture ⭐ (Recommended)

**Approach:** Move JS execution to separate worker processes, communicate via IPC

**Architecture:**
```
Main Server Process (Send+Sync compatible)
├── GraphQL API
├── Task Queue
├── Job Dispatcher
└── IPC Coordinator

Worker Processes (non-Send+Sync)
├── Worker Process 1: Boa JS Engine + Task Execution
├── Worker Process 2: Boa JS Engine + Task Execution
└── Worker Process N: Boa JS Engine + Task Execution
```

**✅ Pros:**
- Complete Send+Sync compatibility for web layer
- True parallel JS execution across CPU cores
- Fault isolation (worker crash doesn't kill server)
- Can scale workers independently
- Full GraphQL functionality restored
- Better resource management
- Follows industry patterns (Node.js cluster module)

**❌ Cons:**
- Increased complexity (IPC, process management)
- Higher memory overhead (multiple processes)
- Serialization costs for data transfer
- More complex debugging across processes
- Need to handle worker process lifecycle

**🔧 Implementation Effort:** High (2-3 weeks)

**Implementation Strategy:**
```rust
// Job Queue becomes IPC coordinator
pub struct ProcessJobQueue {
    worker_pool: Vec<WorkerProcess>,
    ipc_channels: Vec<Sender<JobMessage>>,
}

// Worker processes
fn worker_main() {
    let engine = RatchetEngine::new(); // Non-Send is OK here
    
    while let Ok(job) = ipc_receiver.recv() {
        let result = engine.execute_task(job.task_id, job.input).await;
        ipc_sender.send(JobResult { id: job.id, result }).await;
    }
}

// GraphQL layer becomes fully Send+Sync
pub struct GraphQLContext {
    pub repositories: RepositoryFactory,    // Send+Sync ✅
    pub job_queue: Arc<ProcessJobQueue>,    // Send+Sync ✅
    pub worker_manager: Arc<WorkerManager>, // Send+Sync ✅
}
```

---

### Option 2: Single-Threaded Server with Async Tasks

**Approach:** Run everything in single thread, use async for concurrency

**Architecture:**
```rust
// All operations in single thread
tokio::task::spawn_local(async {
    // JS execution happens here
    // No Send+Sync required
});
```

**✅ Pros:**
- Simplest to implement
- No serialization overhead
- Easy debugging
- Maintains current architecture
- Quick solution for immediate needs

**❌ Cons:**
- No true parallelism (single core utilization)
- One blocking JS task blocks everything
- Limited scalability under load
- Some GraphQL features may still be restricted
- Poor performance characteristics for CPU-intensive tasks

**🔧 Implementation Effort:** Low (1-2 days)

---

### Option 3: Alternative JS Engine

**Approach:** Replace Boa with a Send+Sync compatible engine

**Engine Options:**
- **Deno Core** (V8-based, more complex but Send+Sync)
- **QuickJS** (smaller, might have Send+Sync wrappers)
- **WASM-based** solutions (compile JS to WASM)

**✅ Pros:**
- Maintains multi-threaded architecture
- Better JS performance (especially V8)
- Full ecosystem compatibility
- Industry-standard JS engine (V8)
- No architectural compromises

**❌ Cons:**
- Major breaking change to existing codebase
- Potential compatibility issues with current tasks
- Larger dependencies (V8 is ~50MB+)
- Significant rewrite of JS execution layer
- Complex build requirements
- Learning curve for new engine APIs

**🔧 Implementation Effort:** Very High (4-6 weeks)

**Migration Considerations:**
- Need to audit all existing JS tasks for compatibility
- Different API surface than Boa
- Different performance characteristics
- May require changes to task format

---

### Option 4: Hybrid Architecture

**Approach:** Keep current for simple operations, add async boundaries for complex ones

**Architecture:**
```rust
// Simple operations: Direct access (current)
├── Task CRUD
├── Job Queue Management  
├── Statistics
└── Health Checks

// Complex operations: Async dispatch
├── JS Execution → Background Tasks → Results
├── Real-time subscriptions → Event streams
└── Bulk operations → Worker threads
```

**✅ Pros:**
- Incremental improvement path
- Best of both worlds approach
- Maintains backward compatibility
- Can migrate operations gradually
- Lower risk implementation

**❌ Cons:**
- Inconsistent API patterns
- Some latency for JS operations
- Still limited parallelism for JS
- More complex codebase to maintain
- Potential confusion about which operations are sync vs async

**🔧 Implementation Effort:** Medium (1-2 weeks)

---

### Option 5: Accept Current Limitations

**Approach:** Keep simplified GraphQL, focus development resources elsewhere

**✅ Pros:**
- Zero additional work required
- Stable current state
- Can focus resources on other features
- No risk of introducing new bugs
- Current functionality is sufficient for basic use cases

**❌ Cons:**
- GraphQL remains limited indefinitely
- No real-time task execution via API
- Reduced system capabilities
- Technical debt remains unaddressed
- May limit future feature development

**🔧 Implementation Effort:** None

---

## Detailed Comparison Matrix

| Criteria | Process Separation | Single-Threaded | Alternative Engine | Hybrid | Accept Limits |
|----------|-------------------|------------------|-------------------|--------|---------------|
| **Parallelism** | ✅ Full | ❌ None | ✅ Full | 🔶 Partial | ❌ None |
| **Complexity** | 🔶 High | ✅ Low | 🔴 Very High | 🔶 Medium | ✅ None |
| **Performance** | ✅ Excellent | 🔶 Limited | ✅ Excellent | 🔶 Good | 🔶 Current |
| **GraphQL Compatibility** | ✅ Full | 🔶 Partial | ✅ Full | ✅ Full | 🔶 Limited |
| **Implementation Timeline** | 2-3 weeks | 1-2 days | 4-6 weeks | 1-2 weeks | None |
| **Risk Level** | 🔶 Medium | ✅ Low | 🔴 High | 🔶 Medium | ✅ None |
| **Scalability** | ✅ Excellent | 🔴 Poor | ✅ Excellent | 🔶 Good | 🔴 Poor |
| **Maintenance Burden** | 🔶 Medium | ✅ Low | 🔶 Medium | 🔴 High | ✅ Low |
| **Future Flexibility** | ✅ High | 🔴 Low | ✅ High | 🔶 Medium | 🔴 Low |

## Recommendation: Option 1 - Process Separation Architecture

**Rationale:**
1. **Solves the fundamental problem** rather than working around it
2. **Enables true scalability** with parallel JS execution across cores
3. **Maintains full GraphQL capabilities** without architectural compromises
4. **Provides fault tolerance** through process isolation
5. **Follows proven industry patterns** (similar to Node.js cluster, PHP-FPM)
6. **Future-proof** - can evolve independently

**Next Steps:**
1. Design IPC protocol for job messages
2. Implement worker process lifecycle management
3. Create process-safe job queue coordinator
4. Migrate GraphQL context to use new architecture
5. Add monitoring and health checks for worker processes

## Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Design IPC message protocol
- [ ] Implement basic worker process spawning
- [ ] Create job message serialization/deserialization

### Phase 2: Integration (Week 2)
- [ ] Integrate worker processes with job queue
- [ ] Update GraphQL context to use process-based execution
- [ ] Implement worker health monitoring

### Phase 3: Polish (Week 3)
- [ ] Add worker process auto-restart
- [ ] Implement load balancing across workers
- [ ] Add comprehensive testing and documentation

## Alternative Considerations

If **Option 1** proves too complex initially, **Option 4 (Hybrid)** provides a good intermediate step that can later evolve into the full process separation architecture.

If development resources are extremely limited, **Option 2 (Single-Threaded)** provides the quickest path to restore basic GraphQL functionality, though with significant performance limitations.