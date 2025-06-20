//! GraphQL subscription resolvers

use crate::{context::GraphQLContext, types::*};
use async_graphql::{Context, Result, Subscription};
use futures_util::stream::Stream;
use std::pin::Pin;

/// Root subscription resolver
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to task execution events
    ///
    /// Subscribe to real-time execution status updates. Optionally filter by task_id.
    ///
    /// # Arguments
    /// * `task_id` - Optional filter to only receive events for a specific task
    ///
    /// # Returns
    /// Stream of execution objects as they change status
    async fn task_executions(
        &self,
        ctx: &Context<'_>,
        task_id: Option<GraphQLApiId>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Execution>> + Send>>> {
        let context = ctx.data::<GraphQLContext>()?;

        tracing::debug!("New execution subscription, task_id filter: {:?}", task_id);

        // Create filtered subscription stream
        let stream = context.event_broadcaster.subscribe_executions(task_id);

        Ok(stream)
    }

    /// Subscribe to job status changes
    ///
    /// Subscribe to real-time job status updates. Optionally filter by job_id.
    ///
    /// # Arguments
    /// * `job_id` - Optional filter to only receive events for a specific job
    ///
    /// # Returns
    /// Stream of job objects as they change status
    async fn job_status(
        &self,
        ctx: &Context<'_>,
        job_id: Option<GraphQLApiId>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Job>> + Send>>> {
        let context = ctx.data::<GraphQLContext>()?;

        tracing::debug!("New job subscription, job_id filter: {:?}", job_id);

        // Create filtered subscription stream
        let stream = context.event_broadcaster.subscribe_jobs(job_id);

        Ok(stream)
    }

    /// Subscribe to worker status changes
    ///
    /// Subscribe to real-time worker status updates. Optionally filter by worker_id.
    ///
    /// # Arguments
    /// * `worker_id` - Optional filter to only receive events for a specific worker
    ///
    /// # Returns
    /// Stream of worker objects as they change status
    async fn worker_status(
        &self,
        ctx: &Context<'_>,
        worker_id: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Worker>> + Send>>> {
        let context = ctx.data::<GraphQLContext>()?;

        tracing::debug!("New worker subscription, worker_id filter: {:?}", worker_id);

        // Create filtered subscription stream
        let stream = context.event_broadcaster.subscribe_workers(worker_id);

        Ok(stream)
    }
}
