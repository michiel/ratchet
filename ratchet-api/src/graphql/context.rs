//! GraphQL context for dependency injection

/// GraphQL context containing shared application state
#[derive(Clone)]
pub struct GraphQLContext {
    // TODO: Add service dependencies here
    // pub task_service: Arc<dyn TaskService>,
    // pub execution_service: Arc<dyn ExecutionService>,
    // pub job_service: Arc<dyn JobService>,
    // pub schedule_service: Arc<dyn ScheduleService>,
}

impl GraphQLContext {
    /// Create a new GraphQL context
    pub fn new() -> Self {
        Self {
            // Initialize services here
        }
    }
}

impl Default for GraphQLContext {
    fn default() -> Self {
        Self::new()
    }
}