use async_graphql::*;
use std::sync::Arc;

use crate::database::repositories::RepositoryFactory;
use crate::execution::{JobQueueManager, ProcessTaskExecutor};

use super::resolvers::{Query, Mutation, Subscription, GraphQLContext};

/// The main GraphQL schema type
pub type RatchetSchema = Schema<Query, Mutation, Subscription>;

/// Create a new GraphQL schema with all resolvers and Send+Sync compliance
pub fn create_schema(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    task_executor: Arc<ProcessTaskExecutor>,
) -> RatchetSchema {
    let context = GraphQLContext {
        repositories,
        job_queue,
        task_executor, // ✅ Send/Sync compliant via process separation
    };

    Schema::build(Query, Mutation, Subscription)
        .data(context)
        .finish()
}