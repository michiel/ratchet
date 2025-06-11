//! GraphQL subscription resolvers

use async_graphql::{Subscription, Context, Result};
use futures_util::stream::{self, Stream};
use crate::{
    context::GraphQLContext,
    types::*,
};

/// Root subscription resolver
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to task execution events
    async fn task_executions(
        &self,
        ctx: &Context<'_>,
        _task_id: Option<GraphQLApiId>,
    ) -> Result<impl Stream<Item = Result<Execution>>> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // For now, return an empty stream as this is not yet implemented
        Ok(stream::empty())
    }

    /// Subscribe to job status changes
    async fn job_status(
        &self,
        ctx: &Context<'_>,
        _job_id: Option<GraphQLApiId>,
    ) -> Result<impl Stream<Item = Result<Job>>> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // For now, return an empty stream as this is not yet implemented
        Ok(stream::empty())
    }

    /// Subscribe to worker status changes
    async fn worker_status(
        &self,
        ctx: &Context<'_>,
        _worker_id: Option<String>,
    ) -> Result<impl Stream<Item = Result<Worker>>> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // For now, return an empty stream as this is not yet implemented
        Ok(stream::empty())
    }
}