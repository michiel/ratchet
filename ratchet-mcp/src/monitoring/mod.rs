//! Enhanced health monitoring and system observability for MCP operations

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use crate::correlation::{CorrelationManager, RequestMetrics};
use crate::metrics::{McpMetrics, MetricsSummary};

/// Configuration for health monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// How often to check system health
    pub check_interval: Duration,
    
    /// Threshold for marking system as unhealthy
    pub unhealthy_error_rate: f64,
    
    /// Minimum requests needed for health calculations
    pub min_requests_for_health: u64,
    
    /// Connection timeout threshold
    pub connection_timeout_threshold: Duration,
    
    /// Whether to include detailed metrics in health reports
    pub include_detailed_metrics: bool,
    
    /// Maximum age for correlation data in health reports
    pub max_correlation_age: Duration,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            unhealthy_error_rate: 0.1, // 10% error rate
            min_requests_for_health: 10,
            connection_timeout_threshold: Duration::from_secs(5),
            include_detailed_metrics: true,
            max_correlation_age: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Overall system health status
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum HealthStatus {
    /// System is operating normally
    Healthy,
    
    /// System is experiencing minor issues but operational
    Degraded,
    
    /// System is experiencing major issues
    Unhealthy,
    
    /// System status cannot be determined
    Unknown,
}

/// Transport layer health information
#[derive(Debug, Clone, Serialize)]
pub struct TransportHealth {
    /// Current connection status
    pub connection_status: ConnectionStatus,
    
    /// Number of active connections
    pub active_connections: usize,
    
    /// Average connection latency
    pub avg_latency: Option<Duration>,
    
    /// Recent connection failures
    pub recent_failures: u64,
    
    /// Last successful connection time
    pub last_successful_connection: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Transport-specific health data
    pub transport_specific: serde_json::Value,
}

/// Connection status details
#[derive(Debug, Clone, Serialize)]
pub enum ConnectionStatus {
    /// All connections are healthy
    AllHealthy,
    
    /// Some connections are experiencing issues
    PartiallyHealthy {
        healthy_count: usize,
        total_count: usize,
    },
    
    /// All connections are down
    AllDown,
    
    /// No connections established
    NoConnections,
}

/// Comprehensive health report
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    /// Overall system health status
    pub overall_status: HealthStatus,
    
    /// Transport layer health
    pub transport_health: TransportHealth,
    
    /// Number of active requests being processed
    pub active_requests: usize,
    
    /// Performance metrics summary
    pub metrics_summary: Option<MetricsSummary>,
    
    /// Recent correlation data
    pub correlation_summary: Option<CorrelationSummary>,
    
    /// System resource information
    pub system_resources: SystemResourceInfo,
    
    /// Detailed health checks
    pub health_checks: Vec<HealthCheck>,
    
    /// Report generation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Time taken to generate this report
    pub report_generation_time: Duration,
}

/// Summary of correlation/tracing data
#[derive(Debug, Clone, Serialize)]
pub struct CorrelationSummary {
    /// Number of active traced requests
    pub active_traced_requests: usize,
    
    /// Average request depth in correlation chains
    pub avg_correlation_depth: f64,
    
    /// Recent completed requests
    pub recent_completed_requests: Vec<RequestMetrics>,
    
    /// Top correlation chains by request count
    pub top_correlation_chains: Vec<CorrelationChainSummary>,
}

/// Summary of a correlation chain
#[derive(Debug, Clone, Serialize)]
pub struct CorrelationChainSummary {
    /// Root request ID
    pub root_request_id: String,
    
    /// Number of requests in this chain
    pub request_count: usize,
    
    /// Total duration of the chain
    pub total_duration: Duration,
    
    /// Whether the chain completed successfully
    pub success: bool,
}

/// System resource information
#[derive(Debug, Clone, Serialize)]
pub struct SystemResourceInfo {
    /// Memory usage information
    pub memory_usage: Option<MemoryUsage>,
    
    /// CPU usage percentage
    pub cpu_usage: Option<f64>,
    
    /// Open file descriptor count
    pub open_files: Option<usize>,
    
    /// Network connection count
    pub network_connections: Option<usize>,
    
    /// System uptime
    pub uptime: Duration,
}

/// Memory usage details
#[derive(Debug, Clone, Serialize)]
pub struct MemoryUsage {
    /// Current memory usage in bytes
    pub current_bytes: u64,
    
    /// Peak memory usage in bytes
    pub peak_bytes: u64,
    
    /// Available memory in bytes
    pub available_bytes: Option<u64>,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    /// Name of the health check
    pub name: String,
    
    /// Health check status
    pub status: HealthStatus,
    
    /// Description of the check result
    pub description: String,
    
    /// Time taken to perform this check
    pub duration: Duration,
    
    /// Additional data specific to this check
    pub data: serde_json::Value,
}

/// Enhanced health monitor that combines transport, metrics, and correlation data
pub struct EnhancedHealthMonitor {
    /// Configuration
    config: HealthMonitorConfig,
    
    /// Transport health information
    transport_health: Arc<Mutex<TransportHealth>>,
    
    /// Metrics system
    metrics: Arc<McpMetrics>,
    
    /// Correlation manager
    correlation_manager: Arc<CorrelationManager>,
    
    /// System start time for uptime calculation
    start_time: Instant,
    
    /// Last health report for comparison
    last_report: Arc<RwLock<Option<HealthReport>>>,
}

impl EnhancedHealthMonitor {
    /// Create a new enhanced health monitor
    pub fn new(
        config: HealthMonitorConfig,
        metrics: Arc<McpMetrics>,
        correlation_manager: Arc<CorrelationManager>,
    ) -> Self {
        let transport_health = Arc::new(Mutex::new(TransportHealth {
            connection_status: ConnectionStatus::NoConnections,
            active_connections: 0,
            avg_latency: None,
            recent_failures: 0,
            last_successful_connection: None,
            transport_specific: serde_json::Value::Null,
        }));
        
        Self {
            config,
            transport_health,
            metrics,
            correlation_manager,
            start_time: Instant::now(),
            last_report: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Update transport health information
    pub async fn update_transport_health(&self, health: TransportHealth) {
        let mut transport_health = self.transport_health.lock().await;
        *transport_health = health;
    }
    
    /// Update connection count
    pub async fn update_connection_count(&self, count: usize) {
        let mut transport_health = self.transport_health.lock().await;
        transport_health.active_connections = count;
        
        // Update metrics
        self.metrics.set_active_connections(count);
    }
    
    /// Record a connection failure
    pub async fn record_connection_failure(&self) {
        let mut transport_health = self.transport_health.lock().await;
        transport_health.recent_failures += 1;
    }
    
    /// Record a successful connection
    pub async fn record_successful_connection(&self) {
        let mut transport_health = self.transport_health.lock().await;
        transport_health.last_successful_connection = Some(chrono::Utc::now());
        transport_health.recent_failures = 0; // Reset failure count on success
    }
    
    /// Generate a comprehensive health report
    pub async fn get_comprehensive_health(&self) -> HealthReport {
        let report_start = Instant::now();
        
        // Gather transport health
        let transport_health = self.transport_health.lock().await.clone();
        
        // Get metrics summary
        let metrics_summary = if self.config.include_detailed_metrics {
            Some(self.metrics.get_summary().await)
        } else {
            None
        };
        
        // Get correlation summary
        let correlation_summary = self.build_correlation_summary().await;
        
        // Get active request count
        let active_requests = self.correlation_manager.active_request_count().await;
        self.metrics.set_active_requests(active_requests);
        
        // Perform health checks
        let health_checks = self.perform_health_checks(&transport_health, &metrics_summary).await;
        
        // Calculate overall status
        let overall_status = self.calculate_overall_status(&transport_health, &metrics_summary, &health_checks);
        
        // Gather system resource information
        let system_resources = self.gather_system_resources();
        
        let report = HealthReport {
            overall_status,
            transport_health,
            active_requests,
            metrics_summary,
            correlation_summary,
            system_resources,
            health_checks,
            timestamp: chrono::Utc::now(),
            report_generation_time: report_start.elapsed(),
        };
        
        // Store for comparison
        let mut last_report = self.last_report.write().await;
        *last_report = Some(report.clone());
        
        report
    }
    
    /// Build correlation summary from recent data
    async fn build_correlation_summary(&self) -> Option<CorrelationSummary> {
        let active_traced_requests = self.correlation_manager.active_request_count().await;
        
        // Get recent completed requests
        let recent_metrics = self.correlation_manager.get_recent_metrics(100).await;
        
        // Filter recent requests within the configured age
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(self.config.max_correlation_age).unwrap_or(chrono::Duration::zero());
        let recent_completed_requests: Vec<RequestMetrics> = recent_metrics
            .into_iter()
            .filter(|m| m.completed_at > cutoff_time)
            .collect();
        
        // Calculate average correlation depth
        let avg_correlation_depth = if !recent_completed_requests.is_empty() {
            recent_completed_requests.iter()
                .map(|m| m.correlation_chain.len() as f64)
                .sum::<f64>() / recent_completed_requests.len() as f64
        } else {
            0.0
        };
        
        // Build top correlation chains
        let mut chain_summaries: std::collections::HashMap<String, CorrelationChainSummary> = std::collections::HashMap::new();
        
        for metrics in &recent_completed_requests {
            if let Some(root_id) = metrics.correlation_chain.first() {
                let entry = chain_summaries.entry(root_id.clone()).or_insert_with(|| {
                    CorrelationChainSummary {
                        root_request_id: root_id.clone(),
                        request_count: 0,
                        total_duration: Duration::ZERO,
                        success: true,
                    }
                });
                
                entry.request_count += 1;
                entry.total_duration += metrics.duration;
                entry.success &= metrics.success;
            }
        }
        
        let mut top_correlation_chains: Vec<CorrelationChainSummary> = chain_summaries.into_values().collect();
        top_correlation_chains.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        top_correlation_chains.truncate(10); // Keep top 10
        
        Some(CorrelationSummary {
            active_traced_requests,
            avg_correlation_depth,
            recent_completed_requests,
            top_correlation_chains,
        })
    }
    
    /// Perform individual health checks
    async fn perform_health_checks(
        &self,
        transport_health: &TransportHealth,
        metrics_summary: &Option<MetricsSummary>,
    ) -> Vec<HealthCheck> {
        let mut checks = Vec::new();
        
        // Connection health check
        let connection_check = self.check_connection_health(transport_health).await;
        checks.push(connection_check);
        
        // Error rate health check
        if let Some(metrics) = metrics_summary {
            let error_rate_check = self.check_error_rate(metrics);
            checks.push(error_rate_check);
            
            // Response time check
            let response_time_check = self.check_response_times(metrics);
            checks.push(response_time_check);
        }
        
        // Resource usage check
        let resource_check = self.check_resource_usage();
        checks.push(resource_check);
        
        checks
    }
    
    /// Check connection health
    async fn check_connection_health(&self, transport_health: &TransportHealth) -> HealthCheck {
        let start = Instant::now();
        
        let (status, description) = match &transport_health.connection_status {
            ConnectionStatus::AllHealthy => (HealthStatus::Healthy, "All connections are healthy".to_string()),
            ConnectionStatus::PartiallyHealthy { healthy_count, total_count } => {
                let ratio = *healthy_count as f64 / *total_count as f64;
                if ratio >= 0.8 {
                    (HealthStatus::Degraded, format!("{}/{} connections healthy", healthy_count, total_count))
                } else {
                    (HealthStatus::Unhealthy, format!("Only {}/{} connections healthy", healthy_count, total_count))
                }
            },
            ConnectionStatus::AllDown => (HealthStatus::Unhealthy, "All connections are down".to_string()),
            ConnectionStatus::NoConnections => (HealthStatus::Unknown, "No connections established".to_string()),
        };
        
        HealthCheck {
            name: "connection_health".to_string(),
            status,
            description,
            duration: start.elapsed(),
            data: serde_json::json!({
                "active_connections": transport_health.active_connections,
                "recent_failures": transport_health.recent_failures,
                "avg_latency_ms": transport_health.avg_latency.map(|d| d.as_millis())
            }),
        }
    }
    
    /// Check error rate
    fn check_error_rate(&self, metrics: &MetricsSummary) -> HealthCheck {
        let start = Instant::now();
        
        let (status, description) = if metrics.total_requests < self.config.min_requests_for_health {
            (HealthStatus::Unknown, "Insufficient data for error rate assessment".to_string())
        } else if metrics.error_rate <= self.config.unhealthy_error_rate {
            (HealthStatus::Healthy, format!("Error rate: {:.2}%", metrics.error_rate * 100.0))
        } else if metrics.error_rate <= self.config.unhealthy_error_rate * 2.0 {
            (HealthStatus::Degraded, format!("Elevated error rate: {:.2}%", metrics.error_rate * 100.0))
        } else {
            (HealthStatus::Unhealthy, format!("High error rate: {:.2}%", metrics.error_rate * 100.0))
        };
        
        HealthCheck {
            name: "error_rate".to_string(),
            status,
            description,
            duration: start.elapsed(),
            data: serde_json::json!({
                "error_rate": metrics.error_rate,
                "total_requests": metrics.total_requests,
                "failed_requests": metrics.failed_requests
            }),
        }
    }
    
    /// Check response times
    fn check_response_times(&self, metrics: &MetricsSummary) -> HealthCheck {
        let start = Instant::now();
        
        let avg_duration_ms = metrics.avg_request_duration.as_millis() as f64;
        let p95_duration_ms = metrics.request_duration_histogram.percentile(95.0) * 1000.0;
        
        let (status, description) = if avg_duration_ms <= 100.0 && p95_duration_ms <= 1000.0 {
            (HealthStatus::Healthy, format!("Avg: {:.0}ms, P95: {:.0}ms", avg_duration_ms, p95_duration_ms))
        } else if avg_duration_ms <= 500.0 && p95_duration_ms <= 5000.0 {
            (HealthStatus::Degraded, format!("Elevated latency - Avg: {:.0}ms, P95: {:.0}ms", avg_duration_ms, p95_duration_ms))
        } else {
            (HealthStatus::Unhealthy, format!("High latency - Avg: {:.0}ms, P95: {:.0}ms", avg_duration_ms, p95_duration_ms))
        };
        
        HealthCheck {
            name: "response_times".to_string(),
            status,
            description,
            duration: start.elapsed(),
            data: serde_json::json!({
                "avg_duration_ms": avg_duration_ms,
                "p50_duration_ms": metrics.request_duration_histogram.percentile(50.0) * 1000.0,
                "p95_duration_ms": p95_duration_ms,
                "p99_duration_ms": metrics.request_duration_histogram.percentile(99.0) * 1000.0
            }),
        }
    }
    
    /// Check resource usage
    fn check_resource_usage(&self) -> HealthCheck {
        let start = Instant::now();
        
        // Basic resource check - in a real implementation this would check actual system resources
        let (status, description) = (HealthStatus::Healthy, "Resource usage within normal limits".to_string());
        
        HealthCheck {
            name: "resource_usage".to_string(),
            status,
            description,
            duration: start.elapsed(),
            data: serde_json::json!({
                "uptime_seconds": self.start_time.elapsed().as_secs()
            }),
        }
    }
    
    /// Calculate overall system status
    fn calculate_overall_status(
        &self,
        _transport_health: &TransportHealth,
        _metrics_summary: &Option<MetricsSummary>,
        health_checks: &[HealthCheck],
    ) -> HealthStatus {
        let check_statuses: Vec<&HealthStatus> = health_checks.iter().map(|c| &c.status).collect();
        
        // If any check is unhealthy, system is unhealthy
        if check_statuses.iter().any(|&s| matches!(s, HealthStatus::Unhealthy)) {
            return HealthStatus::Unhealthy;
        }
        
        // If any check is degraded, system is degraded
        if check_statuses.iter().any(|&s| matches!(s, HealthStatus::Degraded)) {
            return HealthStatus::Degraded;
        }
        
        // If all checks are healthy, system is healthy
        if check_statuses.iter().all(|&s| matches!(s, HealthStatus::Healthy)) {
            return HealthStatus::Healthy;
        }
        
        // Otherwise unknown
        HealthStatus::Unknown
    }
    
    /// Gather system resource information
    fn gather_system_resources(&self) -> SystemResourceInfo {
        SystemResourceInfo {
            memory_usage: None, // Would implement actual memory monitoring
            cpu_usage: None,    // Would implement actual CPU monitoring
            open_files: None,   // Would implement file descriptor monitoring
            network_connections: None, // Would implement network monitoring
            uptime: self.start_time.elapsed(),
        }
    }
    
    /// Start background health monitoring
    pub fn start_background_monitoring(self: Arc<Self>) {
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitor.config.check_interval);
            
            loop {
                interval.tick().await;
                
                let health_report = monitor.get_comprehensive_health().await;
                
                // Log health status changes
                if let Some(last_report) = monitor.last_report.read().await.as_ref() {
                    if health_report.overall_status != last_report.overall_status {
                        match health_report.overall_status {
                            HealthStatus::Healthy => {
                                tracing::info!("System health recovered to healthy status");
                            },
                            HealthStatus::Degraded => {
                                tracing::warn!("System health degraded");
                            },
                            HealthStatus::Unhealthy => {
                                tracing::error!("System health is unhealthy");
                            },
                            HealthStatus::Unknown => {
                                tracing::warn!("System health status is unknown");
                            },
                        }
                    }
                }
                
                // Log periodic health summary
                tracing::debug!(
                    status = ?health_report.overall_status,
                    active_requests = health_report.active_requests,
                    active_connections = health_report.transport_health.active_connections,
                    "Health check completed"
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation::CorrelationConfig;
    use crate::metrics::MetricsConfig;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthMonitorConfig::default();
        let metrics = Arc::new(McpMetrics::new(MetricsConfig::default()));
        let correlation_manager = Arc::new(CorrelationManager::new(CorrelationConfig::default()));
        
        let monitor = EnhancedHealthMonitor::new(config, metrics, correlation_manager);
        
        let health_report = monitor.get_comprehensive_health().await;
        assert_eq!(health_report.overall_status, HealthStatus::Unknown);
        assert_eq!(health_report.active_requests, 0);
    }
    
    #[tokio::test]
    async fn test_health_report_generation() {
        let config = HealthMonitorConfig::default();
        let metrics = Arc::new(McpMetrics::new(MetricsConfig::default()));
        let correlation_manager = Arc::new(CorrelationManager::new(CorrelationConfig::default()));
        
        let monitor = EnhancedHealthMonitor::new(config, metrics.clone(), correlation_manager.clone());
        
        // Add some metrics
        metrics.record_request("tools/list", "client-1", Duration::from_millis(100), true).await;
        
        // Start a request for correlation
        correlation_manager.start_request("client-1".to_string(), "tools/list".to_string()).await;
        
        let health_report = monitor.get_comprehensive_health().await;
        
        assert!(health_report.metrics_summary.is_some());
        assert!(health_report.correlation_summary.is_some());
        assert_eq!(health_report.active_requests, 1);
        assert!(!health_report.health_checks.is_empty());
    }
}