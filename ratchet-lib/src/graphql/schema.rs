use async_graphql::*;
use std::sync::Arc;

use crate::database::repositories::RepositoryFactory;
use crate::execution::{JobQueueManager, DatabaseTaskExecutor};
use crate::services::RatchetEngine;

use super::resolvers::{Query, Mutation, Subscription, GraphQLContext};

/// The main GraphQL schema type
pub type RatchetSchema = Schema<Query, Mutation, Subscription>;

/// Create a new GraphQL schema with all resolvers (simplified for Send+Sync)
pub fn create_schema(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    _task_executor: Arc<DatabaseTaskExecutor>, // unused due to Send+Sync constraints
    _engine: Arc<RatchetEngine>, // unused due to Send+Sync constraints
) -> RatchetSchema {
    let context = GraphQLContext {
        repositories,
        job_queue,
        // Note: task_executor and engine removed due to Send+Sync constraints
    };

    Schema::build(Query, Mutation, Subscription)
        .data(context)
        .finish()
}