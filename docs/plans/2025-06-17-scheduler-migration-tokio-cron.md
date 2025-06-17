# Scheduler Migration Plan: tokio-cron-scheduler Implementation

**Date:** 2025-06-17  
**Status:** Draft  
**Priority:** High  
**Estimated Timeline:** 3-4 weeks  

## Overview

Migrate the current polling-based scheduler implementation to use `tokio-cron-scheduler` for improved performance, precision, and maintainability. This migration will eliminate the 30-second polling overhead and provide sub-second scheduling accuracy while maintaining compatibility with existing repository patterns and removing hardcoded heartbeat references.

## Current State Analysis

### Current Implementation Issues
- **Performance**: 30-second polling creates unnecessary database load (120+ queries/hour minimum)
- **Precision**: ±30 second scheduling accuracy is insufficient for production use
- **Scalability**: Fixed polling interval doesn't scale with schedule density
- **Hardcoded Dependencies**: Heartbeat-specific logic embedded in scheduler core
- **Resource Inefficiency**: Continuous background activity regardless of schedule count

### Current Architecture
```
SchedulerService (polling) → SeaORM ScheduleRepository → SQLite
                          ↓
                   Job Creation → JobRepository → SQLite
```

## Target Architecture

### New Event-Driven Architecture
```
tokio-cron-scheduler → Repository Abstraction Layer → SeaORM Repositories → SQLite
                    ↓
            Unified Job Execution → JobRepository → SQLite
```

### Key Design Principles
1. **Repository Pattern Preservation**: All database access through existing repository interfaces
2. **Task Agnostic**: No hardcoded task-specific logic in scheduler core
3. **Event-Driven**: Eliminate polling in favor of precise event scheduling
4. **Backward Compatibility**: Maintain existing API contracts
5. **Configuration Driven**: All task scheduling controlled via configuration/database

## Migration Plan

### Phase 1: Foundation and Cleanup (Week 1)

#### 1.1 Remove Hardcoded Heartbeat References
**Files to Modify:**
- `ratchet-server/src/scheduler.rs` - Remove heartbeat-specific execution logic
- `ratchet-server/src/services.rs` - Remove heartbeat service initialization
- `ratchet-server/src/heartbeat.rs` - Simplify to configuration-only service
- `ratchet-server/src/startup.rs` - Remove heartbeat schedule creation

**Refactoring Tasks:**
```rust
// REMOVE: Hardcoded heartbeat execution
async fn execute_heartbeat_task(&self, job: &UnifiedJob, schedule: &Schedule) -> Result<()> {
    // This entire method should be removed
}

// REMOVE: Hardcoded heartbeat detection
if schedule.name.contains("heartbeat") {
    if let Err(e) = self.execute_heartbeat_task(&created_job, &schedule).await {
        error!("Failed to execute heartbeat task: {}", e);
    }
}
```

#### 1.2 Create Repository Abstraction Layer
**New File:** `ratchet-server/src/scheduler/repository_bridge.rs`
```rust
use ratchet_interfaces::RepositoryFactory;
use ratchet_api_types::{UnifiedSchedule, UnifiedJob, ApiId};

/// Bridge between tokio-cron-scheduler and repository layer
pub struct RepositoryBridge {
    repositories: Arc<dyn RepositoryFactory>,
}

impl RepositoryBridge {
    pub async fn load_all_schedules(&self) -> Result<Vec<ScheduleJobData>, SchedulerError> {
        // Load schedules from repository and convert to tokio-cron-scheduler format
    }
    
    pub async fn create_job_for_schedule(&self, schedule_id: ApiId, execution_time: DateTime<Utc>) -> Result<UnifiedJob, SchedulerError> {
        // Create job through repository pattern
    }
    
    pub async fn update_schedule_execution(&self, schedule_id: ApiId, last_run: DateTime<Utc>, next_run: Option<DateTime<Utc>>) -> Result<(), SchedulerError> {
        // Update schedule execution metadata
    }
}
```

#### 1.3 Define New Scheduler Interface
**New File:** `ratchet-server/src/scheduler/interface.rs`
```rust
#[async_trait]
pub trait SchedulerService: Send + Sync {
    async fn start(&self) -> Result<(), SchedulerError>;
    async fn stop(&self) -> Result<(), SchedulerError>;
    async fn add_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;
    async fn remove_schedule(&self, schedule_id: ApiId) -> Result<(), SchedulerError>;
    async fn update_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;
    async fn get_schedule_status(&self, schedule_id: ApiId) -> Result<ScheduleStatus, SchedulerError>;
}
```

### Phase 2: tokio-cron-scheduler Integration (Week 2)

#### 2.1 Add Dependencies
**Update:** `ratchet-server/Cargo.toml`
```toml
[dependencies]
tokio-cron-scheduler = "0.13"
serde_json = "1.0"
```

#### 2.2 Implement SQLite Metadata Storage
**New File:** `ratchet-server/src/scheduler/sqlite_storage.rs`
```rust
use tokio_cron_scheduler::{JobStoredData, MetaDataStorage, JobSchedulerError};

pub struct SqliteMetadataStore {
    repository_bridge: Arc<RepositoryBridge>,
}

#[async_trait]
impl MetaDataStorage for SqliteMetadataStore {
    async fn add(&self, job: JobStoredData) -> Result<(), JobSchedulerError> {
        // Convert JobStoredData to UnifiedSchedule and store via repository
        let schedule = self.convert_job_to_schedule(job)?;
        self.repository_bridge.repositories.schedule_repository()
            .create(schedule).await
            .map_err(|e| JobSchedulerError::CantAdd)?;
        Ok(())
    }

    async fn delete(&self, id: &Uuid) -> Result<(), JobSchedulerError> {
        let schedule_id = ApiId::from_uuid(*id);
        self.repository_bridge.repositories.schedule_repository()
            .delete(schedule_id).await
            .map_err(|e| JobSchedulerError::CantRemove)?;
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Option<JobStoredData> {
        let schedule_id = ApiId::from_uuid(*id);
        if let Ok(Some(schedule)) = self.repository_bridge.repositories.schedule_repository()
            .find_by_id(schedule_id).await {
            Some(self.convert_schedule_to_job(schedule))
        } else {
            None
        }
    }

    async fn list(&self) -> Vec<JobStoredData> {
        match self.repository_bridge.load_all_schedules().await {
            Ok(schedules) => schedules,
            Err(_) => vec![],
        }
    }
}
```

#### 2.3 Implement Core Scheduler Service
**New File:** `ratchet-server/src/scheduler/tokio_scheduler.rs`
```rust
use tokio_cron_scheduler::{JobScheduler, Job, JobToRun};

pub struct TokioCronSchedulerService {
    scheduler: JobScheduler,
    repository_bridge: Arc<RepositoryBridge>,
    config: SchedulerConfig,
}

impl TokioCronSchedulerService {
    pub async fn new(
        repositories: Arc<dyn RepositoryFactory>,
        config: SchedulerConfig,
    ) -> Result<Self, SchedulerError> {
        let repository_bridge = Arc::new(RepositoryBridge::new(repositories));
        let metadata_storage = SqliteMetadataStore::new(repository_bridge.clone());
        
        let scheduler = JobScheduler::new_with_storage(Box::new(metadata_storage)).await?;
        
        Ok(Self {
            scheduler,
            repository_bridge,
            config,
        })
    }

    async fn create_job_execution_handler(&self) -> impl Fn(Uuid, JobContext) + Send + Sync {
        let bridge = self.repository_bridge.clone();
        
        move |job_id: Uuid, context: JobContext| {
            let bridge = bridge.clone();
            async move {
                if let Err(e) = Self::execute_scheduled_job(bridge, job_id, context).await {
                    error!("Failed to execute scheduled job {}: {}", job_id, e);
                }
            }
        }
    }

    async fn execute_scheduled_job(
        bridge: Arc<RepositoryBridge>,
        job_id: Uuid,
        context: JobContext,
    ) -> Result<(), SchedulerError> {
        let schedule_id = ApiId::from_uuid(job_id);
        let execution_time = Utc::now();
        
        // Create job through repository pattern
        let created_job = bridge.create_job_for_schedule(schedule_id, execution_time).await?;
        
        // Update schedule execution metadata
        let next_run = context.next_tick;
        bridge.update_schedule_execution(schedule_id, execution_time, next_run).await?;
        
        info!("Created job {} for schedule {}", created_job.id, schedule_id);
        Ok(())
    }
}

#[async_trait]
impl SchedulerService for TokioCronSchedulerService {
    async fn start(&self) -> Result<(), SchedulerError> {
        // Load existing schedules from database
        let schedules = self.repository_bridge.load_all_schedules().await?;
        
        for schedule_data in schedules {
            let job = Job::new_async_tz(
                schedule_data.cron_expression,
                schedule_data.timezone,
                self.create_job_execution_handler().await,
            )?;
            
            self.scheduler.add(job).await?;
        }
        
        self.scheduler.start().await?;
        info!("tokio-cron-scheduler started with {} schedules", schedules.len());
        Ok(())
    }

    async fn add_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError> {
        let job = Job::new_async(
            &schedule.cron_expression,
            self.create_job_execution_handler().await,
        )?;
        
        self.scheduler.add(job).await?;
        Ok(())
    }
    
    // ... implement remaining interface methods
}
```

### Phase 3: Service Integration (Week 3)

#### 3.1 Update Service Container
**Modify:** `ratchet-server/src/services.rs`
```rust
pub struct ServiceContainer {
    pub repositories: Arc<dyn RepositoryFactory>,
    pub registry: Arc<dyn TaskRegistry>,
    pub registry_manager: Arc<dyn RegistryManager>,
    pub validator: Arc<dyn TaskValidator>,
    pub mcp_task_service: Option<Arc<TaskDevelopmentService>>,
    pub output_manager: Arc<OutputDeliveryManager>,
    // REMOVE: pub heartbeat_service: Option<Arc<HeartbeatService>>,
    pub scheduler_service: Arc<dyn SchedulerService>, // Change to trait object
}

impl ServiceContainer {
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        // ... existing setup ...

        // Create new scheduler service
        let scheduler_service: Arc<dyn SchedulerService> = Arc::new(
            TokioCronSchedulerService::new(repositories.clone(), scheduler_config).await?
        );

        Ok(Self {
            repositories,
            registry,
            registry_manager,
            validator,
            mcp_task_service,
            output_manager,
            scheduler_service,
        })
    }
}
```

#### 3.2 Update Startup Logic
**Modify:** `ratchet-server/src/startup.rs`
```rust
impl UnifiedServer {
    pub async fn start(self) -> Result<()> {
        // ... existing setup ...

        // Start scheduler service
        let scheduler = self.services.scheduler_service.clone();
        tokio::spawn(async move {
            if let Err(e) = scheduler.start().await {
                error!("Scheduler service failed: {}", e);
            }
        });
        
        // REMOVE: Heartbeat initialization logic
        // if let Some(heartbeat_service) = &self.services.heartbeat_service {
        //     heartbeat_service.initialize().await?;
        // }

        info!("Started scheduler service");
        
        // ... rest of startup ...
    }
}
```

#### 3.3 Implement Schedule Management APIs
**Modify:** `ratchet-rest-api/src/handlers/schedules.rs`
```rust
pub async fn create_schedule(
    State(context): State<TasksContext>,
    Json(create_request): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, AppError> {
    // Create schedule in database
    let schedule = context.repositories.schedule_repository()
        .create(create_request.into()).await?;
    
    // Add to running scheduler
    context.scheduler_service.add_schedule(schedule.clone()).await
        .map_err(|e| AppError::Internal(format!("Failed to add schedule to scheduler: {}", e)))?;
    
    Ok(Json(schedule.into()))
}

pub async fn delete_schedule(
    State(context): State<TasksContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let schedule_id = ApiId::parse(&id)?;
    
    // Remove from running scheduler first
    context.scheduler_service.remove_schedule(schedule_id).await
        .map_err(|e| AppError::Internal(format!("Failed to remove schedule from scheduler: {}", e)))?;
    
    // Remove from database
    context.repositories.schedule_repository()
        .delete(schedule_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}
```

### Phase 4: Configuration and Testing (Week 4)

#### 4.1 Update Configuration
**Modify:** `ratchet-server/src/config.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub max_concurrent_jobs: usize,
    pub job_timeout_seconds: u64,
    pub enable_notifications: bool,
    // REMOVE: heartbeat-specific configuration
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_jobs: 100,
            job_timeout_seconds: 3600,
            enable_notifications: false,
        }
    }
}

// REMOVE: HeartbeatConfig struct and related code
```

#### 4.2 Registry-Based Task Loading
**Modify:** `ratchet-server/src/startup.rs`
```rust
impl UnifiedServer {
    async fn initialize_default_schedules(&self) -> Result<()> {
        // Load embedded tasks from registry
        let heartbeat_task_id = ApiId::parse("00000000-0000-0000-0000-000000000001")?;
        
        // Check if heartbeat task exists in registry
        if let Ok(Some(_task)) = self.services.registry.get_task_by_id(&heartbeat_task_id).await {
            // Check if schedule already exists
            let existing_schedules = self.services.repositories.schedule_repository()
                .find_with_filters(ScheduleFilters {
                    task_id: Some(heartbeat_task_id),
                    enabled: Some(true),
                    ..Default::default()
                }, PaginationInput::default()).await?;
            
            if existing_schedules.items.is_empty() {
                // Create default heartbeat schedule
                let heartbeat_schedule = UnifiedSchedule {
                    id: ApiId::new(),
                    task_id: heartbeat_task_id,
                    name: "system_heartbeat".to_string(),
                    description: Some("System health monitoring heartbeat".to_string()),
                    cron_expression: "0 */5 * * * *".to_string(), // Every 5 minutes
                    enabled: true,
                    next_run: None,
                    last_run: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    metadata: None,
                    output_destinations: Some(serde_json::json!({
                        "destinations": ["stdio"],
                        "format": "json"
                    })),
                };
                
                // Create through repository and add to scheduler
                let created_schedule = self.services.repositories.schedule_repository()
                    .create(heartbeat_schedule).await?;
                    
                self.services.scheduler_service.add_schedule(created_schedule).await?;
                info!("Created default heartbeat schedule");
            }
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Tests
- `SqliteMetadataStore` functionality
- `RepositoryBridge` data conversion
- Schedule CRUD operations through new scheduler

### Integration Tests
- End-to-end schedule creation and execution
- Database persistence across restarts
- API compatibility verification

### Performance Tests
- Compare scheduling precision (old vs new)
- Measure resource usage under load
- Validate concurrent execution limits

## Migration Steps

### Pre-Migration Checklist
- [ ] Backup existing schedule data
- [ ] Verify no production schedules exist
- [ ] Document current API contracts
- [ ] Prepare rollback plan

### Migration Execution
1. **Deploy Phase 1**: Remove hardcoded heartbeat logic
2. **Deploy Phase 2**: Add tokio-cron-scheduler with SQLite adapter
3. **Deploy Phase 3**: Update service integration
4. **Deploy Phase 4**: Final configuration and testing

### Post-Migration Validation
- [ ] All existing schedules function correctly
- [ ] API endpoints respond identically
- [ ] Performance metrics show improvement
- [ ] No hardcoded task references remain

## Risk Mitigation

### Technical Risks
| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Data migration issues | Medium | High | Comprehensive backup and rollback procedures |
| Performance regression | Low | Medium | Extensive performance testing |
| API breaking changes | Low | High | Maintain backward compatibility |
| Schedule execution failures | Medium | High | Thorough integration testing |

### Rollback Plan
1. Revert to previous scheduler implementation
2. Restore original service container structure
3. Re-enable hardcoded heartbeat initialization
4. Validate system functionality

## Success Criteria

### Performance Metrics
- **Scheduling Precision**: Sub-second accuracy (vs ±30 seconds)
- **Resource Usage**: 90% reduction in database queries
- **Concurrent Execution**: Support 100+ simultaneous jobs
- **Memory Usage**: No memory leaks under load

### Functional Requirements
- All existing API endpoints work identically
- Schedule persistence across server restarts
- Heartbeat task executes via registry lookup only
- No hardcoded task references in scheduler core

### Code Quality
- Clean separation of concerns
- Comprehensive test coverage (>80%)
- Clear documentation and examples
- Maintainable architecture

## Future Enhancements

### Post-Migration Opportunities
1. **Advanced Scheduling**: Timezone support, English syntax
2. **Job Notifications**: Webhook integration for job events
3. **Monitoring**: Metrics and observability improvements
4. **Scaling**: Distributed scheduler support

### Architecture Evolution
- Plugin-based task execution
- Event-driven job pipeline
- Advanced retry mechanisms
- Job dependency management

---

**Next Steps**: Begin Phase 1 implementation with removal of hardcoded heartbeat references and creation of repository abstraction layer.