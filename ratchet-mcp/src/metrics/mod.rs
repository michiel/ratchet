//! Performance metrics collection and monitoring for MCP operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether to collect detailed metrics
    pub enabled: bool,
    
    /// Maximum number of tool execution records to keep
    pub max_tool_records: usize,
    
    /// Window size for calculating rates (in seconds)
    pub rate_window_seconds: u64,
    
    /// Whether to include client-specific metrics
    pub track_per_client: bool,
    
    /// Histogram bucket boundaries for request durations (in seconds)
    pub duration_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tool_records: 1000,
            rate_window_seconds: 300, // 5 minutes
            track_per_client: true,
            duration_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
            ],
        }
    }
}

/// Basic metrics counters and gauges
#[derive(Debug)]
pub struct MetricCounters {
    /// Total number of requests processed
    pub total_requests: AtomicU64,
    
    /// Total number of successful requests
    pub successful_requests: AtomicU64,
    
    /// Total number of failed requests  
    pub failed_requests: AtomicU64,
    
    /// Current number of active connections
    pub active_connections: AtomicUsize,
    
    /// Current number of active requests
    pub active_requests: AtomicUsize,
    
    /// Total number of tool executions
    pub tool_executions: AtomicU64,
    
    /// Total duration of all requests (microseconds)
    pub total_request_duration_us: AtomicU64,
    
    /// Total duration of all tool executions (microseconds)
    pub total_tool_duration_us: AtomicU64,
}

impl MetricCounters {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            active_requests: AtomicUsize::new(0),
            tool_executions: AtomicU64::new(0),
            total_request_duration_us: AtomicU64::new(0),
            total_tool_duration_us: AtomicU64::new(0),
        }
    }
}

/// Histogram for tracking duration distributions
#[derive(Debug)]
pub struct Histogram {
    /// Bucket boundaries
    pub buckets: Vec<f64>,
    
    /// Counts for each bucket
    pub counts: Vec<AtomicU64>,
    
    /// Total count of observations
    pub total_count: AtomicU64,
    
    /// Sum of all observed values
    pub sum: AtomicU64, // stored as microseconds
}

impl Histogram {
    pub fn new(buckets: Vec<f64>) -> Self {
        let mut sorted_buckets = buckets;
        sorted_buckets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let counts = (0..sorted_buckets.len() + 1) // +1 for overflow bucket
            .map(|_| AtomicU64::new(0))
            .collect();
        
        Self {
            buckets: sorted_buckets,
            counts,
            total_count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }
    
    /// Record an observation
    pub fn observe(&self, value: f64) {
        let value_us = (value * 1_000_000.0) as u64;
        
        // Find the appropriate bucket
        let mut bucket_index = self.buckets.len(); // default to overflow bucket
        for (i, &boundary) in self.buckets.iter().enumerate() {
            if value <= boundary {
                bucket_index = i;
                break;
            }
        }
        
        self.counts[bucket_index].fetch_add(1, Ordering::Relaxed);
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value_us, Ordering::Relaxed);
    }
    
    /// Get snapshot of histogram data
    pub fn snapshot(&self) -> HistogramSnapshot {
        let buckets = self.buckets.clone();
        let counts: Vec<u64> = self.counts.iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect();
        let total_count = self.total_count.load(Ordering::Relaxed);
        let sum_us = self.sum.load(Ordering::Relaxed);
        
        HistogramSnapshot {
            buckets,
            counts,
            total_count,
            sum_seconds: sum_us as f64 / 1_000_000.0,
        }
    }
}

/// Snapshot of histogram data for serialization
#[derive(Debug, Clone, Serialize)]
pub struct HistogramSnapshot {
    pub buckets: Vec<f64>,
    pub counts: Vec<u64>,
    pub total_count: u64,
    pub sum_seconds: f64,
}

impl HistogramSnapshot {
    /// Calculate average duration
    pub fn average(&self) -> f64 {
        if self.total_count > 0 {
            self.sum_seconds / self.total_count as f64
        } else {
            0.0
        }
    }
    
    /// Calculate percentile (approximate based on buckets)
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        
        let target_count = (self.total_count as f64 * p / 100.0) as u64;
        let mut cumulative = 0;
        
        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target_count {
                return if i < self.buckets.len() {
                    self.buckets[i]
                } else {
                    // Overflow bucket - return last bucket boundary
                    self.buckets.last().copied().unwrap_or(0.0)
                };
            }
        }
        
        0.0
    }
}

/// Tool execution record for tracking performance
#[derive(Debug, Clone, Serialize)]
pub struct ToolExecutionRecord {
    pub tool_name: String,
    pub duration: Duration,
    pub success: bool,
    pub client_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

/// Main metrics collection system
pub struct McpMetrics {
    /// Configuration
    config: MetricsConfig,
    
    /// Basic counters
    counters: MetricCounters,
    
    /// Request duration histogram
    request_duration: Histogram,
    
    /// Tool execution duration histogram
    tool_duration: Histogram,
    
    /// Recent tool execution records
    tool_records: Arc<RwLock<Vec<ToolExecutionRecord>>>,
    
    /// Per-method request counts
    method_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    
    /// Per-client metrics (if enabled)
    client_metrics: Arc<RwLock<HashMap<String, ClientMetrics>>>,
    
    /// Start time for rate calculations
    start_time: Instant,
}

/// Per-client metrics
#[derive(Debug)]
struct ClientMetrics {
    request_count: AtomicU64,
    error_count: AtomicU64,
    total_duration_us: AtomicU64,
    last_activity: std::sync::Mutex<Instant>,
}

impl ClientMetrics {
    fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_duration_us: AtomicU64::new(0),
            last_activity: std::sync::Mutex::new(Instant::now()),
        }
    }
}

impl McpMetrics {
    /// Create a new metrics system
    pub fn new(config: MetricsConfig) -> Self {
        let request_duration = Histogram::new(config.duration_buckets.clone());
        let tool_duration = Histogram::new(config.duration_buckets.clone());
        
        Self {
            config,
            counters: MetricCounters::new(),
            request_duration,
            tool_duration,
            tool_records: Arc::new(RwLock::new(Vec::new())),
            method_counts: Arc::new(RwLock::new(HashMap::new())),
            client_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }
    
    /// Record a completed request
    pub async fn record_request(&self, method: &str, client_id: &str, duration: Duration, success: bool) {
        if !self.config.enabled {
            return;
        }
        
        let duration_secs = duration.as_secs_f64();
        
        // Update basic counters
        self.counters.total_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.counters.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.counters.total_request_duration_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        // Record in histogram
        self.request_duration.observe(duration_secs);
        
        // Update method counts
        {
            let mut method_counts = self.method_counts.write().await;
            method_counts.entry(method.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
        
        // Update client metrics if enabled
        if self.config.track_per_client {
            let mut client_metrics = self.client_metrics.write().await;
            let metrics = client_metrics.entry(client_id.to_string())
                .or_insert_with(ClientMetrics::new);
            
            metrics.request_count.fetch_add(1, Ordering::Relaxed);
            metrics.total_duration_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
            if !success {
                metrics.error_count.fetch_add(1, Ordering::Relaxed);
            }
            
            if let Ok(mut last_activity) = metrics.last_activity.lock() {
                *last_activity = Instant::now();
            };
        }
    }
    
    /// Record a tool execution
    pub async fn record_tool_execution(
        &self,
        tool_name: &str,
        client_id: &str,
        duration: Duration,
        success: bool,
        request_id: Option<String>,
    ) {
        if !self.config.enabled {
            return;
        }
        
        let duration_secs = duration.as_secs_f64();
        
        // Update counters
        self.counters.tool_executions.fetch_add(1, Ordering::Relaxed);
        self.counters.total_tool_duration_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        // Record in histogram
        self.tool_duration.observe(duration_secs);
        
        // Add to tool records
        let record = ToolExecutionRecord {
            tool_name: tool_name.to_string(),
            duration,
            success,
            client_id: client_id.to_string(),
            timestamp: chrono::Utc::now(),
            request_id,
        };
        
        let mut tool_records = self.tool_records.write().await;
        tool_records.push(record);
        
        // Keep only recent records
        if tool_records.len() > self.config.max_tool_records {
            let len = tool_records.len();
            tool_records.drain(0..(len - self.config.max_tool_records));
        }
    }
    
    /// Update active connection count
    pub fn set_active_connections(&self, count: usize) {
        self.counters.active_connections.store(count, Ordering::Relaxed);
    }
    
    /// Update active request count
    pub fn set_active_requests(&self, count: usize) {
        self.counters.active_requests.store(count, Ordering::Relaxed);
    }
    
    /// Get comprehensive metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        let uptime = self.start_time.elapsed();
        let uptime_seconds = uptime.as_secs_f64();
        
        // Basic metrics
        let total_requests = self.counters.total_requests.load(Ordering::Relaxed);
        let successful_requests = self.counters.successful_requests.load(Ordering::Relaxed);
        let failed_requests = self.counters.failed_requests.load(Ordering::Relaxed);
        let active_connections = self.counters.active_connections.load(Ordering::Relaxed);
        let active_requests = self.counters.active_requests.load(Ordering::Relaxed);
        let tool_executions = self.counters.tool_executions.load(Ordering::Relaxed);
        
        // Request rates
        let request_rate = if uptime_seconds > 0.0 {
            total_requests as f64 / uptime_seconds
        } else {
            0.0
        };
        
        let error_rate = if total_requests > 0 {
            failed_requests as f64 / total_requests as f64
        } else {
            0.0
        };
        
        // Average durations
        let avg_request_duration = if total_requests > 0 {
            Duration::from_micros(
                self.counters.total_request_duration_us.load(Ordering::Relaxed) / total_requests
            )
        } else {
            Duration::ZERO
        };
        
        let avg_tool_duration = if tool_executions > 0 {
            Duration::from_micros(
                self.counters.total_tool_duration_us.load(Ordering::Relaxed) / tool_executions
            )
        } else {
            Duration::ZERO
        };
        
        // Method counts
        let method_counts = self.method_counts.read().await;
        let method_stats: HashMap<String, u64> = method_counts.iter()
            .map(|(method, counter)| (method.clone(), counter.load(Ordering::Relaxed)))
            .collect();
        
        // Histogram snapshots
        let request_duration_histogram = self.request_duration.snapshot();
        let tool_duration_histogram = self.tool_duration.snapshot();
        
        // Recent tool executions
        let recent_tools = self.tool_records.read().await;
        let recent_tool_executions = recent_tools.clone();
        
        MetricsSummary {
            uptime,
            total_requests,
            successful_requests,
            failed_requests,
            active_connections,
            active_requests,
            tool_executions,
            request_rate,
            error_rate,
            avg_request_duration,
            avg_tool_duration,
            method_stats,
            request_duration_histogram,
            tool_duration_histogram,
            recent_tool_executions,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Get client-specific metrics
    pub async fn get_client_metrics(&self, client_id: &str) -> Option<ClientMetricsSummary> {
        if !self.config.track_per_client {
            return None;
        }
        
        let client_metrics = self.client_metrics.read().await;
        client_metrics.get(client_id).map(|metrics| {
            let request_count = metrics.request_count.load(Ordering::Relaxed);
            let error_count = metrics.error_count.load(Ordering::Relaxed);
            let total_duration_us = metrics.total_duration_us.load(Ordering::Relaxed);
            
            let avg_duration = if request_count > 0 {
                Duration::from_micros(total_duration_us / request_count)
            } else {
                Duration::ZERO
            };
            
            let error_rate = if request_count > 0 {
                error_count as f64 / request_count as f64
            } else {
                0.0
            };
            
            ClientMetricsSummary {
                client_id: client_id.to_string(),
                request_count,
                error_count,
                error_rate,
                avg_duration,
                last_activity: metrics.last_activity.lock().ok().map(|la| *la),
            }
        })
    }
    
    /// Get tool execution statistics
    pub async fn get_tool_stats(&self) -> HashMap<String, ToolStats> {
        let tool_records = self.tool_records.read().await;
        let mut stats: HashMap<String, ToolStats> = HashMap::new();
        
        for record in tool_records.iter() {
            let entry = stats.entry(record.tool_name.clone()).or_insert_with(|| ToolStats {
                tool_name: record.tool_name.clone(),
                execution_count: 0,
                success_count: 0,
                total_duration: Duration::ZERO,
                min_duration: Duration::MAX,
                max_duration: Duration::ZERO,
            });
            
            entry.execution_count += 1;
            if record.success {
                entry.success_count += 1;
            }
            entry.total_duration += record.duration;
            entry.min_duration = entry.min_duration.min(record.duration);
            entry.max_duration = entry.max_duration.max(record.duration);
        }
        
        stats
    }
}

/// Comprehensive metrics summary
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSummary {
    pub uptime: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub active_connections: usize,
    pub active_requests: usize,
    pub tool_executions: u64,
    pub request_rate: f64, // requests per second
    pub error_rate: f64,   // error percentage
    pub avg_request_duration: Duration,
    pub avg_tool_duration: Duration,
    pub method_stats: HashMap<String, u64>,
    pub request_duration_histogram: HistogramSnapshot,
    pub tool_duration_histogram: HistogramSnapshot,
    pub recent_tool_executions: Vec<ToolExecutionRecord>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Client-specific metrics summary
#[derive(Debug, Clone, Serialize)]
pub struct ClientMetricsSummary {
    pub client_id: String,
    pub request_count: u64,
    pub error_count: u64,
    pub error_rate: f64,
    pub avg_duration: Duration,
    #[serde(skip)]
    pub last_activity: Option<Instant>,
}

/// Statistics for a specific tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolStats {
    pub tool_name: String,
    pub execution_count: u64,
    pub success_count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
}

impl ToolStats {
    pub fn success_rate(&self) -> f64 {
        if self.execution_count > 0 {
            self.success_count as f64 / self.execution_count as f64
        } else {
            0.0
        }
    }
    
    pub fn avg_duration(&self) -> Duration {
        if self.execution_count > 0 {
            self.total_duration / self.execution_count as u32
        } else {
            Duration::ZERO
        }
    }
}

impl Default for McpMetrics {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_basic_metrics() {
        let metrics = McpMetrics::new(MetricsConfig::default());
        
        // Record some requests
        metrics.record_request("tools/list", "client-1", Duration::from_millis(100), true).await;
        metrics.record_request("tools/call", "client-1", Duration::from_millis(200), false).await;
        metrics.record_request("tools/list", "client-2", Duration::from_millis(150), true).await;
        
        let summary = metrics.get_summary().await;
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.successful_requests, 2);
        assert_eq!(summary.failed_requests, 1);
        assert_eq!(summary.error_rate, 1.0 / 3.0);
        
        // Check method stats
        assert_eq!(summary.method_stats.get("tools/list"), Some(&2));
        assert_eq!(summary.method_stats.get("tools/call"), Some(&1));
    }
    
    #[tokio::test]
    async fn test_tool_metrics() {
        let metrics = McpMetrics::new(MetricsConfig::default());
        
        // Record tool executions
        metrics.record_tool_execution("test_tool", "client-1", Duration::from_millis(50), true, Some("req-1".to_string())).await;
        metrics.record_tool_execution("test_tool", "client-1", Duration::from_millis(75), true, Some("req-2".to_string())).await;
        metrics.record_tool_execution("other_tool", "client-2", Duration::from_millis(100), false, None).await;
        
        let tool_stats = metrics.get_tool_stats().await;
        
        let test_tool_stats = tool_stats.get("test_tool").unwrap();
        assert_eq!(test_tool_stats.execution_count, 2);
        assert_eq!(test_tool_stats.success_count, 2);
        assert_eq!(test_tool_stats.success_rate(), 1.0);
        
        let other_tool_stats = tool_stats.get("other_tool").unwrap();
        assert_eq!(other_tool_stats.execution_count, 1);
        assert_eq!(other_tool_stats.success_count, 0);
        assert_eq!(other_tool_stats.success_rate(), 0.0);
    }
    
    #[tokio::test]
    async fn test_client_metrics() {
        let config = MetricsConfig {
            track_per_client: true,
            ..Default::default()
        };
        let metrics = McpMetrics::new(config);
        
        // Record requests for different clients
        metrics.record_request("tools/list", "client-1", Duration::from_millis(100), true).await;
        metrics.record_request("tools/call", "client-1", Duration::from_millis(200), false).await;
        metrics.record_request("tools/list", "client-2", Duration::from_millis(150), true).await;
        
        let client1_metrics = metrics.get_client_metrics("client-1").await.unwrap();
        assert_eq!(client1_metrics.request_count, 2);
        assert_eq!(client1_metrics.error_count, 1);
        assert_eq!(client1_metrics.error_rate, 0.5);
        
        let client2_metrics = metrics.get_client_metrics("client-2").await.unwrap();
        assert_eq!(client2_metrics.request_count, 1);
        assert_eq!(client2_metrics.error_count, 0);
        assert_eq!(client2_metrics.error_rate, 0.0);
    }
    
    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(vec![0.1, 0.5, 1.0, 5.0]);
        
        // Record some observations
        histogram.observe(0.05);   // bucket 0
        histogram.observe(0.3);    // bucket 1
        histogram.observe(0.8);    // bucket 2
        histogram.observe(2.0);    // bucket 3
        histogram.observe(10.0);   // overflow bucket
        
        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.total_count, 5);
        assert_eq!(snapshot.counts[0], 1); // 0.05
        assert_eq!(snapshot.counts[1], 1); // 0.3
        assert_eq!(snapshot.counts[2], 1); // 0.8
        assert_eq!(snapshot.counts[3], 1); // 2.0
        assert_eq!(snapshot.counts[4], 1); // 10.0 (overflow)
        
        // Test percentiles
        assert_eq!(snapshot.percentile(50.0), 1.0); // 50th percentile should be in 1.0 bucket
    }
}