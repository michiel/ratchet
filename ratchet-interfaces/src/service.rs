//! Service interface definitions
//!
//! Extracted from ratchet-lib/src/services/base.rs to break circular dependencies.
//! This provides the core service traits that all Ratchet services implement.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base service trait that all services should implement
///
/// This trait provides a common interface for service lifecycle management,
/// health monitoring, and metrics collection across all Ratchet services.
#[async_trait]
pub trait Service: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type Config: Send + Sync;

    /// Initialize the service with configuration
    async fn initialize(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Get the service name for logging and monitoring
    fn name(&self) -> &'static str;

    /// Perform health check
    async fn health_check(&self) -> Result<ServiceHealth, Self::Error>;

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<(), Self::Error>;

    /// Get service metrics
    fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }

    /// Get service configuration (optional)
    fn config(&self) -> Option<&Self::Config> {
        None
    }
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Service is operating normally
    Healthy,
    /// Service is operational but with reduced performance
    Degraded { reason: String },
    /// Service is not functioning properly
    Unhealthy { reason: String },
    /// Health status cannot be determined
    Unknown,
}

/// Service health information
///
/// Contains detailed information about a service's current health status,
/// including performance metrics and diagnostic metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_checked: DateTime<Utc>,
    pub latency_ms: Option<u64>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ServiceHealth {
    /// Create a healthy service status
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
            last_checked: Utc::now(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a degraded service status
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded { reason: reason.into() },
            message: None,
            last_checked: Utc::now(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an unhealthy service status
    pub fn unhealthy(reason: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy { reason: reason.into() },
            message: None,
            last_checked: Utc::now(),
            latency_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a descriptive message to the health status
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add latency information to the health status
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    /// Add metadata to the health status
    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }
}

/// Service metrics
///
/// Contains performance and operational metrics for a service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub requests_total: u64,
    pub requests_failed: u64,
    pub average_latency_ms: f64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: Option<u64>,
    pub custom_metrics: HashMap<String, f64>,
}

impl ServiceMetrics {
    /// Calculate the success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.requests_total == 0 {
            1.0
        } else {
            let successful = self.requests_total - self.requests_failed;
            successful as f64 / self.requests_total as f64
        }
    }

    /// Calculate the error rate (0.0 to 1.0)
    pub fn error_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }

    /// Add a custom metric
    pub fn with_custom_metric(mut self, name: &str, value: f64) -> Self {
        self.custom_metrics.insert(name.to_string(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_health_builder() {
        let health = ServiceHealth::healthy()
            .with_message("All systems operational")
            .with_latency(50)
            .with_metadata("version", "1.0.0");

        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.message, Some("All systems operational".to_string()));
        assert_eq!(health.latency_ms, Some(50));
        assert_eq!(health.metadata["version"], "1.0.0");
    }

    #[test]
    fn test_service_metrics_calculations() {
        let metrics = ServiceMetrics {
            requests_total: 100,
            requests_failed: 5,
            average_latency_ms: 25.0,
            uptime_seconds: 3600,
            memory_usage_bytes: Some(1024 * 1024),
            custom_metrics: HashMap::new(),
        };

        assert!((metrics.success_rate() - 0.95).abs() < 0.0001);
        assert!((metrics.error_rate() - 0.05).abs() < 0.0001);
    }

    #[test]
    fn test_health_status_variants() {
        let healthy = ServiceHealth::healthy();
        assert!(matches!(healthy.status, HealthStatus::Healthy));

        let degraded = ServiceHealth::degraded("High latency");
        assert!(matches!(degraded.status, HealthStatus::Degraded { .. }));

        let unhealthy = ServiceHealth::unhealthy("Database connection failed");
        assert!(matches!(unhealthy.status, HealthStatus::Unhealthy { .. }));
    }
}
