//! Core output destination trait and data structures

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::errors::{DeliveryError, ValidationError};

/// Trait for output destinations that can receive task results
#[async_trait]
pub trait OutputDestination: Send + Sync {
    /// Deliver output to this destination
    async fn deliver(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<DeliveryResult, DeliveryError>;

    /// Validate destination configuration
    fn validate_config(&self) -> Result<(), ValidationError>;

    /// Get destination type for metrics/logging
    fn destination_type(&self) -> &'static str;

    /// Check if destination supports retries
    fn supports_retry(&self) -> bool {
        true
    }

    /// Get estimated delivery time (for scheduling)
    fn estimated_delivery_time(&self) -> Duration {
        Duration::from_secs(5) // Default 5 seconds
    }
}

/// Task output data to be delivered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub job_id: i32,
    pub task_id: i32,
    pub execution_id: i32,
    pub output_data: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
    pub completed_at: DateTime<Utc>,
    pub execution_duration: Duration,
}

/// Context information for delivery
#[derive(Debug, Clone)]
pub struct DeliveryContext {
    pub job_id: i32,
    pub task_name: String,
    pub task_version: String,
    pub timestamp: DateTime<Utc>,
    pub environment: String,
    pub trace_id: String,
    pub template_variables: HashMap<String, String>,
}

/// Result of a delivery attempt
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub success: bool,
    pub destination_id: String,
    pub delivery_time: Duration,
    pub size_bytes: u64,
    pub response_info: Option<String>, // Response from webhook, file path, etc.
    pub error: Option<DeliveryError>,
}

impl DeliveryResult {
    pub fn success(
        destination_id: String,
        delivery_time: Duration,
        size_bytes: u64,
        response_info: Option<String>,
    ) -> Self {
        Self {
            success: true,
            destination_id,
            delivery_time,
            size_bytes,
            response_info,
            error: None,
        }
    }

    pub fn failure(destination_id: String, delivery_time: Duration, error: DeliveryError) -> Self {
        Self {
            success: false,
            destination_id,
            delivery_time,
            size_bytes: 0,
            response_info: None,
            error: Some(error),
        }
    }
}
