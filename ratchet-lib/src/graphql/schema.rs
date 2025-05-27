use async_graphql::*;
use std::sync::Arc;

use crate::database::repositories::RepositoryFactory;
use crate::execution::{JobQueueManager, DatabaseTaskExecutor};
use crate::services::RatchetEngine;

use super::resolvers::{Query, Mutation, Subscription, GraphQLContext};

/// The main GraphQL schema type
pub type RatchetSchema = Schema<Query, Mutation, Subscription>;

/// Create a new GraphQL schema with all resolvers
pub fn create_schema(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    task_executor: Arc<DatabaseTaskExecutor>,
    engine: Arc<RatchetEngine>,
) -> RatchetSchema {
    let context = GraphQLContext {
        repositories,
        job_queue,
        task_executor,
        engine,
    };

    Schema::build(Query, Mutation, Subscription)
        .data(context)
        .finish()
}