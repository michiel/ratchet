//! Metrics collection for output delivery

use crate::output::errors::DeliveryError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Metrics for delivery operations
#[derive(Debug, Default)]
pub struct DeliveryMetrics {
    inner: Arc<Mutex<DeliveryMetricsInner>>,
}

#[derive(Debug, Default)]
struct DeliveryMetricsInner {
    /// Total deliveries by destination type
    total_deliveries: HashMap<String, u64>,
    /// Successful deliveries by destination type
    successful_deliveries: HashMap<String, u64>,
    /// Failed deliveries by destination type
    failed_deliveries: HashMap<String, u64>,
    /// Total bytes delivered by destination type
    bytes_delivered: HashMap<String, u64>,
    /// Total delivery time by destination type
    total_delivery_time: HashMap<String, Duration>,
    /// Error counts by error type
    error_counts: HashMap<String, u64>,
    /// Last error by destination type
    last_errors: HashMap<String, String>,
    /// Batch delivery statistics
    batch_stats: BatchStats,
}

#[derive(Debug, Default)]
struct BatchStats {
    total_batches: u64,
    total_destinations: u64,
    successful_destinations: u64,
    failed_destinations: u64,
    total_batch_time: Duration,
}

impl DeliveryMetrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DeliveryMetricsInner::default())),
        }
    }

    /// Record a successful delivery
    pub fn record_success(
        &self,
        destination_type: &str,
        delivery_time: Duration,
        bytes_delivered: u64,
    ) {
        let mut inner = self.inner.lock().unwrap();

        *inner
            .total_deliveries
            .entry(destination_type.to_string())
            .or_insert(0) += 1;
        *inner
            .successful_deliveries
            .entry(destination_type.to_string())
            .or_insert(0) += 1;
        *inner
            .bytes_delivered
            .entry(destination_type.to_string())
            .or_insert(0) += bytes_delivered;

        let total_time = inner
            .total_delivery_time
            .entry(destination_type.to_string())
            .or_insert(Duration::ZERO);
        *total_time += delivery_time;
    }

    /// Record a failed delivery
    pub fn record_failure(
        &self,
        destination_type: &str,
        delivery_time: Duration,
        error: &DeliveryError,
    ) {
        let mut inner = self.inner.lock().unwrap();

        *inner
            .total_deliveries
            .entry(destination_type.to_string())
            .or_insert(0) += 1;
        *inner
            .failed_deliveries
            .entry(destination_type.to_string())
            .or_insert(0) += 1;

        let total_time = inner
            .total_delivery_time
            .entry(destination_type.to_string())
            .or_insert(Duration::ZERO);
        *total_time += delivery_time;

        // Track error types
        let error_type = match error {
            DeliveryError::TemplateRender { .. } => "template_render",
            DeliveryError::Serialization { .. } => "serialization",
            DeliveryError::Filesystem { .. } => "filesystem",
            DeliveryError::FileExists { .. } => "file_exists",
            DeliveryError::WebhookFailed { .. } => "webhook_failed",
            DeliveryError::Network { .. } => "network",
            DeliveryError::RequestClone => "request_clone",
            DeliveryError::MaxRetriesExceeded { .. } => "max_retries_exceeded",
            DeliveryError::TaskJoin { .. } => "task_join",
            DeliveryError::InvalidTemplateVariable { .. } => "invalid_template_variable",
            DeliveryError::Database { .. } => "database",
            DeliveryError::S3 { .. } => "s3",
        };

        *inner
            .error_counts
            .entry(error_type.to_string())
            .or_insert(0) += 1;
        inner
            .last_errors
            .insert(destination_type.to_string(), error.to_string());
    }

    /// Record batch delivery statistics
    pub fn record_batch_delivery(
        &self,
        total_destinations: usize,
        successful_destinations: usize,
        failed_destinations: usize,
        batch_time: Duration,
    ) {
        let mut inner = self.inner.lock().unwrap();

        inner.batch_stats.total_batches += 1;
        inner.batch_stats.total_destinations += total_destinations as u64;
        inner.batch_stats.successful_destinations += successful_destinations as u64;
        inner.batch_stats.failed_destinations += failed_destinations as u64;
        inner.batch_stats.total_batch_time += batch_time;
    }

    /// Get success rate for a destination type
    pub fn success_rate(&self, destination_type: &str) -> f64 {
        let inner = self.inner.lock().unwrap();

        let total = inner
            .total_deliveries
            .get(destination_type)
            .copied()
            .unwrap_or(0);
        let successful = inner
            .successful_deliveries
            .get(destination_type)
            .copied()
            .unwrap_or(0);

        if total == 0 {
            0.0
        } else {
            successful as f64 / total as f64
        }
    }

    /// Get average delivery time for a destination type
    pub fn average_delivery_time(&self, destination_type: &str) -> Duration {
        let inner = self.inner.lock().unwrap();

        let total = inner
            .total_deliveries
            .get(destination_type)
            .copied()
            .unwrap_or(0);
        let total_time = inner
            .total_delivery_time
            .get(destination_type)
            .copied()
            .unwrap_or(Duration::ZERO);

        if total == 0 {
            Duration::ZERO
        } else {
            total_time / total as u32
        }
    }

    /// Get total bytes delivered for a destination type
    pub fn total_bytes_delivered(&self, destination_type: &str) -> u64 {
        let inner = self.inner.lock().unwrap();
        inner
            .bytes_delivered
            .get(destination_type)
            .copied()
            .unwrap_or(0)
    }

    /// Get all destination types that have been used
    pub fn destination_types(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        inner.total_deliveries.keys().cloned().collect()
    }

    /// Get summary statistics
    pub fn summary(&self) -> MetricsSummary {
        let inner = self.inner.lock().unwrap();

        let mut destinations = Vec::new();
        for dest_type in inner.total_deliveries.keys() {
            destinations.push(DestinationMetrics {
                destination_type: dest_type.clone(),
                total_deliveries: inner.total_deliveries.get(dest_type).copied().unwrap_or(0),
                successful_deliveries: inner
                    .successful_deliveries
                    .get(dest_type)
                    .copied()
                    .unwrap_or(0),
                failed_deliveries: inner.failed_deliveries.get(dest_type).copied().unwrap_or(0),
                bytes_delivered: inner.bytes_delivered.get(dest_type).copied().unwrap_or(0),
                average_delivery_time: self.average_delivery_time(dest_type),
                success_rate: self.success_rate(dest_type),
                last_error: inner.last_errors.get(dest_type).cloned(),
            });
        }

        MetricsSummary {
            destinations,
            error_counts: inner.error_counts.clone(),
            batch_stats: BatchStatsSummary {
                total_batches: inner.batch_stats.total_batches,
                total_destinations: inner.batch_stats.total_destinations,
                successful_destinations: inner.batch_stats.successful_destinations,
                failed_destinations: inner.batch_stats.failed_destinations,
                average_batch_time: if inner.batch_stats.total_batches > 0 {
                    inner.batch_stats.total_batch_time / inner.batch_stats.total_batches as u32
                } else {
                    Duration::ZERO
                },
                batch_success_rate: if inner.batch_stats.total_destinations > 0 {
                    inner.batch_stats.successful_destinations as f64
                        / inner.batch_stats.total_destinations as f64
                } else {
                    0.0
                },
            },
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        *inner = DeliveryMetricsInner::default();
    }
}

/// Summary of all metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub destinations: Vec<DestinationMetrics>,
    pub error_counts: HashMap<String, u64>,
    pub batch_stats: BatchStatsSummary,
}

/// Metrics for a specific destination type
#[derive(Debug, Clone)]
pub struct DestinationMetrics {
    pub destination_type: String,
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub bytes_delivered: u64,
    pub average_delivery_time: Duration,
    pub success_rate: f64,
    pub last_error: Option<String>,
}

/// Batch delivery statistics summary
#[derive(Debug, Clone)]
pub struct BatchStatsSummary {
    pub total_batches: u64,
    pub total_destinations: u64,
    pub successful_destinations: u64,
    pub failed_destinations: u64,
    pub average_batch_time: Duration,
    pub batch_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::errors::DeliveryError;

    #[test]
    fn test_success_metrics() {
        let metrics = DeliveryMetrics::new();

        metrics.record_success("filesystem", Duration::from_millis(100), 1024);
        metrics.record_success("filesystem", Duration::from_millis(200), 2048);
        metrics.record_success("webhook", Duration::from_millis(300), 512);

        assert_eq!(metrics.success_rate("filesystem"), 1.0);
        assert_eq!(metrics.success_rate("webhook"), 1.0);
        assert_eq!(
            metrics.average_delivery_time("filesystem"),
            Duration::from_millis(150)
        );
        assert_eq!(metrics.total_bytes_delivered("filesystem"), 3072);
    }

    #[test]
    fn test_failure_metrics() {
        let metrics = DeliveryMetrics::new();
        let error = DeliveryError::FileExists {
            path: "/tmp/test".to_string(),
        };

        metrics.record_success("filesystem", Duration::from_millis(100), 1024);
        metrics.record_failure("filesystem", Duration::from_millis(50), &error);

        assert_eq!(metrics.success_rate("filesystem"), 0.5);
        assert_eq!(
            metrics.average_delivery_time("filesystem"),
            Duration::from_millis(75)
        );
    }

    #[test]
    fn test_batch_metrics() {
        let metrics = DeliveryMetrics::new();

        metrics.record_batch_delivery(5, 3, 2, Duration::from_millis(500));
        metrics.record_batch_delivery(3, 3, 0, Duration::from_millis(300));

        let summary = metrics.summary();
        assert_eq!(summary.batch_stats.total_batches, 2);
        assert_eq!(summary.batch_stats.total_destinations, 8);
        assert_eq!(summary.batch_stats.successful_destinations, 6);
        assert_eq!(summary.batch_stats.failed_destinations, 2);
        assert_eq!(summary.batch_stats.batch_success_rate, 0.75);
    }
}
