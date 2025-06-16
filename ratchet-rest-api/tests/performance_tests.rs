//! Performance and load testing suite for the Ratchet REST API
//!
//! This module provides comprehensive performance testing for REST endpoints,
//! covering throughput, latency, concurrent operations, and resource utilization.

use ratchet_rest_api::{
    app::{create_app, AppConfig},
    context::TasksContext,
    handlers::auth::AuthRequest,
};
use ratchet_storage::testing::{MockFactory, TestDatabase};
use ratchet_interfaces::{
    TaskRegistry, RegistryManager, TaskValidator, RepositoryFactory,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};
use tokio::{time::sleep, sync::Semaphore};
use tower::ServiceExt;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    pub concurrent_requests: usize,
    pub total_requests: usize,
    pub test_duration_secs: u64,
    pub request_timeout_ms: u64,
    pub ramp_up_duration_secs: u64,
    pub enable_detailed_metrics: bool,
    pub target_percentile_latency_ms: u64,
    pub max_allowed_error_rate: f64,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 10,
            total_requests: 1000,
            test_duration_secs: 30,
            request_timeout_ms: 5000,
            ramp_up_duration_secs: 5,
            enable_detailed_metrics: true,
            target_percentile_latency_ms: 100, // 100ms for 95th percentile
            max_allowed_error_rate: 0.01, // 1% error rate
        }
    }
}

/// Performance metrics collection
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration_ms: u64,
    pub average_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub bytes_transferred: u64,
    pub memory_usage_mb: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_duration_ms: 0,
            average_latency_ms: 0.0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            bytes_transferred: 0,
            memory_usage_mb: 0.0,
        }
    }
}

/// Request latency tracker
struct LatencyTracker {
    latencies: Arc<tokio::sync::Mutex<Vec<u64>>>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    bytes_transferred: Arc<AtomicU64>,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            bytes_transferred: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn record_request(&self, latency_ms: u64, success: bool, response_size: usize) {
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
            self.latencies.lock().await.push(latency_ms);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.bytes_transferred.fetch_add(response_size as u64, Ordering::Relaxed);
    }

    async fn calculate_metrics(&self, total_duration_ms: u64) -> PerformanceMetrics {
        let mut latencies = self.latencies.lock().await;
        latencies.sort_unstable();
        
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let total = successful + failed;
        let bytes = self.bytes_transferred.load(Ordering::Relaxed);
        
        let mut metrics = PerformanceMetrics::default();
        metrics.total_requests = total;
        metrics.successful_requests = successful;
        metrics.failed_requests = failed;
        metrics.total_duration_ms = total_duration_ms;
        metrics.bytes_transferred = bytes;
        
        if !latencies.is_empty() {
            metrics.min_latency_ms = latencies[0];
            metrics.max_latency_ms = latencies[latencies.len() - 1];
            metrics.average_latency_ms = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
            
            // Calculate percentiles
            metrics.p50_latency_ms = latencies[latencies.len() * 50 / 100];
            metrics.p95_latency_ms = latencies[latencies.len() * 95 / 100];
            metrics.p99_latency_ms = latencies[latencies.len() * 99 / 100];
        }
        
        if total > 0 {
            metrics.error_rate = failed as f64 / total as f64;
        }
        
        if total_duration_ms > 0 {
            metrics.requests_per_second = (total as f64 * 1000.0) / total_duration_ms as f64;
        }
        
        metrics
    }
}

/// Performance test runner for REST API
pub struct RestApiPerformanceTest {
    config: PerformanceTestConfig,
    app_config: AppConfig,
}

impl RestApiPerformanceTest {
    /// Create a new performance test with default configuration
    pub fn new() -> Self {
        Self::with_config(PerformanceTestConfig::default())
    }
    
    /// Create a new performance test with custom configuration
    pub fn with_config(config: PerformanceTestConfig) -> Self {
        let app_config = AppConfig::default();
        Self { config, app_config }
    }
    
    /// Create test application with mock dependencies
    async fn create_test_app(&self) -> axum::Router {
        let storage_factory = Arc::new(MemoryStorageFactory::new());
        let repositories = Arc::new(MockRepositoryFactory::new());
        let registry = Arc::new(MockTaskRegistry::new());
        let registry_manager = Arc::new(MockRegistryManager::new());
        let validator = Arc::new(MockTaskValidator::new());
        
        let context = TasksContext::new(
            repositories,
            registry,
            registry_manager,
            validator,
        );
        
        create_app(self.app_config.clone(), context).await
    }
    
    /// Run performance test for a specific endpoint
    pub async fn test_endpoint_performance(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<PerformanceMetrics, Box<dyn std::error::Error>> {
        let app = self.create_test_app().await;
        let tracker = LatencyTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_requests));
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        // Ramp up phase
        let ramp_up_duration = Duration::from_secs(self.config.ramp_up_duration_secs);
        let request_interval = ramp_up_duration / self.config.total_requests as u32;
        
        for i in 0..self.config.total_requests {
            let app_clone = app.clone();
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let method = method.to_string();
            let path = path.to_string();
            let body = body.clone();
            
            let handle = tokio::spawn(async move {
                // Ramp up delay
                if ramp_up_duration.as_millis() > 0 {
                    sleep(request_interval * i as u32).await;
                }
                
                let _permit = semaphore_clone.acquire().await.unwrap();
                let request_start = Instant::now();
                
                let mut request_builder = Request::builder()
                    .method(method.as_str())
                    .uri(&path);
                
                let request = if let Some(body_json) = body {
                    request_builder = request_builder.header("content-type", "application/json");
                    request_builder.body(Body::from(body_json.to_string())).unwrap()
                } else {
                    request_builder.body(Body::empty()).unwrap()
                };
                
                let response = app_clone.oneshot(request).await;
                let latency = request_start.elapsed().as_millis() as u64;
                
                match response {
                    Ok(resp) => {
                        let status = resp.status();
                        let body_bytes = to_bytes(resp.into_body()).await.unwrap_or_default();
                        let success = status.is_success();
                        
                        tracker_clone.record_request(latency, success, body_bytes.len()).await;
                    }
                    Err(_) => {
                        tracker_clone.record_request(latency, false, 0).await;
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all requests to complete
        for handle in handles {
            let _ = handle.await;
        }
        
        let total_duration = start_time.elapsed().as_millis() as u64;
        let metrics = tracker.calculate_metrics(total_duration).await;
        
        Ok(metrics)
    }
    
    /// Run comprehensive performance test suite
    pub async fn run_comprehensive_test_suite(&self) -> Result<Vec<(String, PerformanceMetrics)>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        // Test GET endpoints
        println!("Testing GET /api/tasks...");
        let tasks_metrics = self.test_endpoint_performance("GET", "/api/tasks", None).await?;
        results.push(("GET /api/tasks".to_string(), tasks_metrics));
        
        println!("Testing GET /api/executions...");
        let executions_metrics = self.test_endpoint_performance("GET", "/api/executions", None).await?;
        results.push(("GET /api/executions".to_string(), executions_metrics));
        
        println!("Testing GET /api/jobs...");
        let jobs_metrics = self.test_endpoint_performance("GET", "/api/jobs", None).await?;
        results.push(("GET /api/jobs".to_string(), jobs_metrics));
        
        println!("Testing GET /api/schedules...");
        let schedules_metrics = self.test_endpoint_performance("GET", "/api/schedules", None).await?;
        results.push(("GET /api/schedules".to_string(), schedules_metrics));
        
        println!("Testing GET /api/workers...");
        let workers_metrics = self.test_endpoint_performance("GET", "/api/workers", None).await?;
        results.push(("GET /api/workers".to_string(), workers_metrics));
        
        // Test POST endpoints
        println!("Testing POST /api/tasks...");
        let create_task_body = json!({
            "name": "performance-test-task",
            "description": "Task created during performance testing",
            "task_type": "JavaScript",
            "script": "function execute() { return { success: true }; }",
            "enabled": true,
            "retry_policy": {
                "max_retries": 3,
                "base_delay_ms": 1000
            }
        });
        let create_task_metrics = self.test_endpoint_performance("POST", "/api/tasks", Some(create_task_body)).await?;
        results.push(("POST /api/tasks".to_string(), create_task_metrics));
        
        // Test authentication endpoints
        println!("Testing POST /api/auth/login...");
        let login_body = json!({
            "username": "test_user",
            "password": "test_password"
        });
        let login_metrics = self.test_endpoint_performance("POST", "/api/auth/login", Some(login_body)).await?;
        results.push(("POST /api/auth/login".to_string(), login_metrics));
        
        // Test health endpoint
        println!("Testing GET /health...");
        let health_metrics = self.test_endpoint_performance("GET", "/health", None).await?;
        results.push(("GET /health".to_string(), health_metrics));
        
        Ok(results)
    }
    
    /// Run stress test with high concurrency
    pub async fn run_stress_test(&self) -> Result<PerformanceMetrics, Box<dyn std::error::Error>> {
        let mut stress_config = self.config.clone();
        stress_config.concurrent_requests = 100;
        stress_config.total_requests = 5000;
        stress_config.test_duration_secs = 60;
        
        let stress_test = Self::with_config(stress_config);
        stress_test.test_endpoint_performance("GET", "/api/tasks", None).await
    }
    
    /// Print performance metrics in a readable format
    pub fn print_metrics(&self, endpoint: &str, metrics: &PerformanceMetrics) {
        println!("\n=== Performance Test Results: {} ===", endpoint);
        println!("Total Requests: {}", metrics.total_requests);
        println!("Successful: {} ({:.2}%)", metrics.successful_requests, 
                 (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0);
        println!("Failed: {} ({:.2}%)", metrics.failed_requests, metrics.error_rate * 100.0);
        println!("Test Duration: {:.2}s", metrics.total_duration_ms as f64 / 1000.0);
        println!("Requests/Second: {:.2}", metrics.requests_per_second);
        println!("Average Latency: {:.2}ms", metrics.average_latency_ms);
        println!("Latency P50: {}ms", metrics.p50_latency_ms);
        println!("Latency P95: {}ms", metrics.p95_latency_ms);
        println!("Latency P99: {}ms", metrics.p99_latency_ms);
        println!("Min Latency: {}ms", metrics.min_latency_ms);
        println!("Max Latency: {}ms", metrics.max_latency_ms);
        println!("Data Transferred: {:.2}MB", metrics.bytes_transferred as f64 / (1024.0 * 1024.0));
        
        // Performance assessment
        if metrics.error_rate <= self.config.max_allowed_error_rate {
            println!("✅ Error rate within acceptable limits");
        } else {
            println!("❌ Error rate exceeds acceptable limits ({:.2}% > {:.2}%)", 
                     metrics.error_rate * 100.0, self.config.max_allowed_error_rate * 100.0);
        }
        
        if metrics.p95_latency_ms <= self.config.target_percentile_latency_ms {
            println!("✅ P95 latency within target");
        } else {
            println!("❌ P95 latency exceeds target ({}ms > {}ms)", 
                     metrics.p95_latency_ms, self.config.target_percentile_latency_ms);
        }
    }
}

// =============================================================================
// PERFORMANCE TESTS
// =============================================================================

#[tokio::test]
async fn test_rest_api_basic_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PerformanceTestConfig::default();
    config.concurrent_requests = 5;
    config.total_requests = 100;
    config.test_duration_secs = 10;
    
    let performance_test = RestApiPerformanceTest::with_config(config);
    let metrics = performance_test.test_endpoint_performance("GET", "/api/tasks", None).await?;
    
    performance_test.print_metrics("GET /api/tasks", &metrics);
    
    // Basic performance assertions
    assert!(metrics.total_requests >= 100);
    assert!(metrics.error_rate <= 0.05); // 5% error rate max
    assert!(metrics.requests_per_second > 0.0);
    assert!(metrics.average_latency_ms < 1000.0); // 1 second max average
    
    Ok(())
}

#[tokio::test]
async fn test_rest_api_concurrent_requests() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PerformanceTestConfig::default();
    config.concurrent_requests = 20;
    config.total_requests = 200;
    
    let performance_test = RestApiPerformanceTest::with_config(config);
    let metrics = performance_test.test_endpoint_performance("GET", "/health", None).await?;
    
    performance_test.print_metrics("GET /health (concurrent)", &metrics);
    
    // Concurrent performance assertions
    assert!(metrics.total_requests >= 200);
    assert!(metrics.error_rate <= 0.02); // 2% error rate max for health endpoint
    assert!(metrics.p95_latency_ms < 500); // 500ms P95 max
    
    Ok(())
}

#[tokio::test]
async fn test_rest_api_comprehensive_suite() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PerformanceTestConfig::default();
    config.concurrent_requests = 3;
    config.total_requests = 50;
    
    let performance_test = RestApiPerformanceTest::with_config(config);
    let results = performance_test.run_comprehensive_test_suite().await?;
    
    assert!(!results.is_empty());
    
    for (endpoint, metrics) in results {
        performance_test.print_metrics(&endpoint, &metrics);
        
        // Comprehensive test assertions
        assert!(metrics.total_requests >= 50);
        assert!(metrics.error_rate <= 0.10); // 10% error rate max for comprehensive test
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rest_api_post_request_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PerformanceTestConfig::default();
    config.concurrent_requests = 5;
    config.total_requests = 50;
    
    let performance_test = RestApiPerformanceTest::with_config(config);
    
    let task_body = json!({
        "name": "perf-test-task",
        "description": "Performance test task",
        "task_type": "JavaScript",
        "script": "function execute() { return { result: 'success' }; }",
        "enabled": true
    });
    
    let metrics = performance_test.test_endpoint_performance("POST", "/api/tasks", Some(task_body)).await?;
    
    performance_test.print_metrics("POST /api/tasks", &metrics);
    
    // POST request performance assertions
    assert!(metrics.total_requests >= 50);
    assert!(metrics.error_rate <= 0.15); // 15% error rate max for POST (some may fail validation)
    assert!(metrics.average_latency_ms < 2000.0); // 2 seconds max average for POST
    
    Ok(())
}

#[tokio::test]
#[ignore] // Marked as ignore since stress tests are resource intensive
async fn test_rest_api_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    let performance_test = RestApiPerformanceTest::new();
    let metrics = performance_test.run_stress_test().await?;
    
    performance_test.print_metrics("GET /api/tasks (stress test)", &metrics);
    
    // Stress test assertions
    assert!(metrics.total_requests >= 5000);
    assert!(metrics.error_rate <= 0.05); // 5% error rate max under stress
    assert!(metrics.requests_per_second > 50.0); // Minimum 50 RPS under stress
    
    Ok(())
}

// TODO: Add tests for:
// - Memory usage monitoring during performance tests
// - Database connection pool performance under load
// - WebSocket connection performance
// - File upload/download performance
// - Authentication performance with JWT validation
// - Rate limiting performance impact
// - Long-running request handling
// - Graceful degradation under resource constraints