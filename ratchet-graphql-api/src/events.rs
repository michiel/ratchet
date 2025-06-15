//! Event streaming system for GraphQL subscriptions
//!
//! This module provides a broadcasting mechanism for real-time events
//! that can be subscribed to via GraphQL subscriptions.

use async_graphql::SimpleObject;
use futures_util::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::types::{Execution, Job, Worker, GraphQLApiId};

/// Maximum number of events to buffer per subscription topic
const EVENT_BUFFER_SIZE: usize = 1000;

/// Event broadcaster for managing subscription streams
#[derive(Clone)]
pub struct EventBroadcaster {
    execution_tx: broadcast::Sender<Execution>,
    job_tx: broadcast::Sender<Job>,
    worker_tx: broadcast::Sender<Worker>,
}

impl EventBroadcaster {
    /// Create a new event broadcaster
    pub fn new() -> Self {
        let (execution_tx, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        let (job_tx, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        let (worker_tx, _) = broadcast::channel(EVENT_BUFFER_SIZE);

        Self {
            execution_tx,
            job_tx,
            worker_tx,
        }
    }

    /// Broadcast an execution event
    pub fn broadcast_execution(&self, execution: Execution) {
        match self.execution_tx.send(execution.clone()) {
            Ok(subscriber_count) => {
                debug!("Broadcasted execution event to {} subscribers: execution_id={:?}", 
                       subscriber_count, execution.id);
            }
            Err(_) => {
                debug!("No subscribers for execution events");
            }
        }
    }

    /// Broadcast a job event
    pub fn broadcast_job(&self, job: Job) {
        match self.job_tx.send(job.clone()) {
            Ok(subscriber_count) => {
                debug!("Broadcasted job event to {} subscribers: job_id={:?}", 
                       subscriber_count, job.id);
            }
            Err(_) => {
                debug!("No subscribers for job events");
            }
        }
    }

    /// Broadcast a worker event
    pub fn broadcast_worker(&self, worker: Worker) {
        match self.worker_tx.send(worker.clone()) {
            Ok(subscriber_count) => {
                debug!("Broadcasted worker event to {} subscribers: worker_id={}", 
                       subscriber_count, worker.id);
            }
            Err(_) => {
                debug!("No subscribers for worker events");
            }
        }
    }

    /// Subscribe to execution events with optional task filtering
    pub fn subscribe_executions(
        &self, 
        task_id_filter: Option<GraphQLApiId>
    ) -> Pin<Box<dyn Stream<Item = async_graphql::Result<Execution>> + Send>> {
        let rx = self.execution_tx.subscribe();
        let stream = ExecutionSubscriptionStream::new(rx, task_id_filter);
        Box::pin(stream)
    }

    /// Subscribe to job events with optional job filtering
    pub fn subscribe_jobs(
        &self, 
        job_id_filter: Option<GraphQLApiId>
    ) -> Pin<Box<dyn Stream<Item = async_graphql::Result<Job>> + Send>> {
        let rx = self.job_tx.subscribe();
        let stream = JobSubscriptionStream::new(rx, job_id_filter);
        Box::pin(stream)
    }

    /// Subscribe to worker events with optional worker filtering
    pub fn subscribe_workers(
        &self, 
        worker_id_filter: Option<String>
    ) -> Pin<Box<dyn Stream<Item = async_graphql::Result<Worker>> + Send>> {
        let rx = self.worker_tx.subscribe();
        let stream = WorkerSubscriptionStream::new(rx, worker_id_filter);
        Box::pin(stream)
    }

    /// Get subscriber count for executions
    pub fn execution_subscriber_count(&self) -> usize {
        self.execution_tx.receiver_count()
    }

    /// Get subscriber count for jobs
    pub fn job_subscriber_count(&self) -> usize {
        self.job_tx.receiver_count()
    }

    /// Get subscriber count for workers
    pub fn worker_subscriber_count(&self) -> usize {
        self.worker_tx.receiver_count()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom stream for execution subscriptions with filtering
pub struct ExecutionSubscriptionStream {
    rx: broadcast::Receiver<Execution>,
    task_id_filter: Option<GraphQLApiId>,
}

impl ExecutionSubscriptionStream {
    pub fn new(rx: broadcast::Receiver<Execution>, task_id_filter: Option<GraphQLApiId>) -> Self {
        Self { rx, task_id_filter }
    }
}

impl Stream for ExecutionSubscriptionStream {
    type Item = async_graphql::Result<Execution>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.try_recv() {
            Ok(execution) => {
                // Apply task ID filter if specified
                if let Some(ref filter_task_id) = self.task_id_filter {
                    if execution.task_id != filter_task_id.0 {
                        cx.waker().wake_by_ref(); // Schedule another poll
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Some(Ok(execution)))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!("Execution subscription lagged, skipped {} events", skipped);
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                debug!("Execution broadcast channel closed");
                Poll::Ready(None)
            }
        }
    }
}

/// Custom stream for job subscriptions with filtering
pub struct JobSubscriptionStream {
    rx: broadcast::Receiver<Job>,
    job_id_filter: Option<GraphQLApiId>,
}

impl JobSubscriptionStream {
    pub fn new(rx: broadcast::Receiver<Job>, job_id_filter: Option<GraphQLApiId>) -> Self {
        Self { rx, job_id_filter }
    }
}

impl Stream for JobSubscriptionStream {
    type Item = async_graphql::Result<Job>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.try_recv() {
            Ok(job) => {
                // Apply job ID filter if specified
                if let Some(ref filter_job_id) = self.job_id_filter {
                    if job.id != *filter_job_id {
                        cx.waker().wake_by_ref(); // Schedule another poll
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Some(Ok(job)))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!("Job subscription lagged, skipped {} events", skipped);
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                debug!("Job broadcast channel closed");
                Poll::Ready(None)
            }
        }
    }
}

/// Custom stream for worker subscriptions with filtering
pub struct WorkerSubscriptionStream {
    rx: broadcast::Receiver<Worker>,
    worker_id_filter: Option<String>,
}

impl WorkerSubscriptionStream {
    pub fn new(rx: broadcast::Receiver<Worker>, worker_id_filter: Option<String>) -> Self {
        Self { rx, worker_id_filter }
    }
}

impl Stream for WorkerSubscriptionStream {
    type Item = async_graphql::Result<Worker>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.try_recv() {
            Ok(worker) => {
                // Apply worker ID filter if specified
                if let Some(ref filter_worker_id) = self.worker_id_filter {
                    if worker.id != *filter_worker_id {
                        cx.waker().wake_by_ref(); // Schedule another poll
                        return Poll::Pending;
                    }
                }
                Poll::Ready(Some(Ok(worker)))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                warn!("Worker subscription lagged, skipped {} events", skipped);
                cx.waker().wake_by_ref(); // Schedule another poll
                Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                debug!("Worker broadcast channel closed");
                Poll::Ready(None)
            }
        }
    }
}

/// Helper struct for subscription health monitoring
#[derive(SimpleObject)]
pub struct SubscriptionStats {
    /// Number of active execution subscribers
    pub execution_subscribers: i32,
    /// Number of active job subscribers
    pub job_subscribers: i32,
    /// Number of active worker subscribers
    pub worker_subscribers: i32,
    /// Buffer size for each subscription type
    pub buffer_size: i32,
}

impl SubscriptionStats {
    /// Create subscription stats from broadcaster
    pub fn from_broadcaster(broadcaster: &EventBroadcaster) -> Self {
        Self {
            execution_subscribers: broadcaster.execution_subscriber_count() as i32,
            job_subscribers: broadcaster.job_subscriber_count() as i32,
            worker_subscribers: broadcaster.worker_subscriber_count() as i32,
            buffer_size: EVENT_BUFFER_SIZE as i32,
        }
    }
}