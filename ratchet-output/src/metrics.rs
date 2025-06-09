//! Metrics collection for output delivery operations

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

/// Metrics for a specific destination
#[derive(Debug, Default)]
pub struct DestinationMetrics {
    pub total_deliveries: AtomicU64,
    pub successful_deliveries: AtomicU64,
    pub failed_deliveries: AtomicU64,
    pub total_bytes_delivered: AtomicU64,
    pub total_delivery_time: AtomicU64, // in milliseconds
    pub last_delivery_time: AtomicU64,  // timestamp in milliseconds
}

impl DestinationMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful delivery
    pub fn record_success(&self, delivery_time: Duration, bytes: u64) {
        self.total_deliveries.fetch_add(1, Ordering::Relaxed);
        self.successful_deliveries.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_delivered.fetch_add(bytes, Ordering::Relaxed);
        self.total_delivery_time.fetch_add(delivery_time.as_millis() as u64, Ordering::Relaxed);
        self.last_delivery_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            Ordering::Relaxed,
        );
    }

    /// Record a failed delivery
    pub fn record_failure(&self, delivery_time: Duration) {
        self.total_deliveries.fetch_add(1, Ordering::Relaxed);
        self.failed_deliveries.fetch_add(1, Ordering::Relaxed);
        self.total_delivery_time.fetch_add(delivery_time.as_millis() as u64, Ordering::Relaxed);
        self.last_delivery_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            Ordering::Relaxed,
        );
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.total_deliveries.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let successful = self.successful_deliveries.load(Ordering::Relaxed);
        (successful as f64 / total as f64) * 100.0
    }

    /// Get average delivery time
    pub fn average_delivery_time(&self) -> Duration {
        let total = self.total_deliveries.load(Ordering::Relaxed);
        if total == 0 {
            return Duration::from_millis(0);
        }
        let total_time = self.total_delivery_time.load(Ordering::Relaxed);
        Duration::from_millis(total_time / total)
    }

    /// Get total bytes delivered
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_delivered.load(Ordering::Relaxed)
    }

    /// Get total deliveries
    pub fn total_count(&self) -> u64 {
        self.total_deliveries.load(Ordering::Relaxed)
    }

    /// Get successful deliveries
    pub fn success_count(&self) -> u64 {
        self.successful_deliveries.load(Ordering::Relaxed)
    }

    /// Get failed deliveries
    pub fn failure_count(&self) -> u64 {
        self.failed_deliveries.load(Ordering::Relaxed)
    }

    /// Get last delivery timestamp
    pub fn last_delivery(&self) -> Option<std::time::SystemTime> {
        let timestamp = self.last_delivery_time.load(Ordering::Relaxed);
        if timestamp == 0 {
            None
        } else {
            Some(std::time::UNIX_EPOCH + Duration::from_millis(timestamp))
        }
    }
}

/// Metrics collection for all output destinations
#[derive(Debug)]
pub struct DeliveryMetrics {
    destination_metrics: Arc<RwLock<HashMap<String, Arc<DestinationMetrics>>>>,
}

impl DeliveryMetrics {
    pub fn new() -> Self {
        Self {
            destination_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a successful delivery for a destination
    pub fn record_success(&self, destination: &str, delivery_time: Duration, bytes: u64) {
        tokio::spawn({
            let destination = destination.to_string();
            let metrics = Arc::clone(&self.destination_metrics);
            async move {
                let mut metrics_map = metrics.write().await;
                let dest_metrics = metrics_map
                    .entry(destination)
                    .or_insert_with(|| Arc::new(DestinationMetrics::new()));
                dest_metrics.record_success(delivery_time, bytes);
            }
        });
    }

    /// Record a failed delivery for a destination
    pub fn record_failure(&self, destination: &str, delivery_time: Duration) {
        tokio::spawn({
            let destination = destination.to_string();
            let metrics = Arc::clone(&self.destination_metrics);
            async move {
                let mut metrics_map = metrics.write().await;
                let dest_metrics = metrics_map
                    .entry(destination)
                    .or_insert_with(|| Arc::new(DestinationMetrics::new()));
                dest_metrics.record_failure(delivery_time);
            }
        });
    }

    /// Get metrics for a specific destination
    pub async fn get_destination_metrics(&self, destination: &str) -> Option<Arc<DestinationMetrics>> {
        let metrics = self.destination_metrics.read().await;
        metrics.get(destination).cloned()
    }

    /// Get metrics for all destinations
    pub async fn get_all_metrics(&self) -> HashMap<String, Arc<DestinationMetrics>> {
        let metrics = self.destination_metrics.read().await;
        metrics.clone()
    }

    /// Get aggregate metrics across all destinations
    pub async fn get_aggregate_metrics(&self) -> AggregateMetrics {
        let metrics = self.destination_metrics.read().await;
        let mut aggregate = AggregateMetrics::default();

        for dest_metrics in metrics.values() {
            aggregate.total_deliveries += dest_metrics.total_count();
            aggregate.successful_deliveries += dest_metrics.success_count();
            aggregate.failed_deliveries += dest_metrics.failure_count();
            aggregate.total_bytes_delivered += dest_metrics.total_bytes();
        }

        aggregate
    }

    /// Reset metrics for a specific destination
    pub async fn reset_destination_metrics(&self, destination: &str) {
        let mut metrics = self.destination_metrics.write().await;
        metrics.remove(destination);
    }

    /// Reset all metrics
    pub async fn reset_all_metrics(&self) {
        let mut metrics = self.destination_metrics.write().await;
        metrics.clear();
    }
}

impl Default for DeliveryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate metrics across all destinations
#[derive(Debug, Default)]
pub struct AggregateMetrics {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub total_bytes_delivered: u64,
}

impl AggregateMetrics {
    /// Get overall success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_deliveries == 0 {
            return 0.0;
        }
        (self.successful_deliveries as f64 / self.total_deliveries as f64) * 100.0
    }

    /// Get failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.total_deliveries == 0 {
            return 0.0;
        }
        (self.failed_deliveries as f64 / self.total_deliveries as f64) * 100.0
    }
}