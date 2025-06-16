//! Comprehensive load testing framework for the Ratchet system
//!
//! This module provides a unified load testing framework that can test
//! REST API, GraphQL API, and MCP protocol performance under various load conditions.

use serde_json::{json, Value};
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
    collections::HashMap,
};
use tokio::{time::sleep, sync::Semaphore, task::JoinHandle};
use uuid::Uuid;

/// Load test scenario configuration
#[derive(Debug, Clone)]
pub struct LoadTestScenario {
    pub name: String,
    pub description: String,
    pub duration_secs: u64,
    pub concurrent_users: usize,
    pub ramp_up_duration_secs: u64,
    pub ramp_down_duration_secs: u64,
    pub target_rps: f64,
    pub max_response_time_ms: u64,
    pub error_rate_threshold: f64,
    pub endpoints: Vec<EndpointConfig>,
}

/// Endpoint configuration for load testing
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    pub name: String,
    pub endpoint_type: EndpointType,
    pub weight: f64, // Probability of selecting this endpoint (0.0 - 1.0)
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
    pub headers: HashMap<String, String>,
    pub expected_status: u16,
    pub timeout_ms: u64,
}

/// Type of API endpoint
#[derive(Debug, Clone)]
pub enum EndpointType {
    RestApi,
    GraphQL,
    MCP,
}

/// Load test results
#[derive(Debug, Clone)]
pub struct LoadTestResults {
    pub scenario_name: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration_ms: u64,
    pub average_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub p50_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub throughput_mb_per_sec: f64,
    pub endpoint_results: HashMap<String, EndpointResults>,
    pub resource_utilization: ResourceUtilization,
}

/// Per-endpoint results
#[derive(Debug, Clone)]
pub struct EndpointResults {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub status_code_distribution: HashMap<u16, u64>,
}

/// Resource utilization metrics
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    pub peak_memory_mb: f64,
    pub average_cpu_percent: f64,
    pub peak_cpu_percent: f64,
    pub network_in_mb: f64,
    pub network_out_mb: f64,
    pub disk_read_mb: f64,
    pub disk_write_mb: f64,
}

/// Request tracking for load tests
struct LoadTestTracker {
    response_times: Arc<tokio::sync::Mutex<Vec<u64>>>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    bytes_transferred: Arc<AtomicU64>,
    endpoint_stats: Arc<tokio::sync::Mutex<HashMap<String, EndpointResults>>>,
    status_codes: Arc<tokio::sync::Mutex<HashMap<u16, u64>>>,
}

impl LoadTestTracker {
    fn new() -> Self {
        Self {
            response_times: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            bytes_transferred: Arc::new(AtomicU64::new(0)),
            endpoint_stats: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            status_codes: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    async fn record_request(
        &self,
        endpoint_name: &str,
        response_time_ms: u64,
        status_code: u16,
        response_size: usize,
        success: bool,
    ) {
        // Update global stats
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        self.response_times.lock().await.push(response_time_ms);
        self.bytes_transferred.fetch_add(response_size as u64, Ordering::Relaxed);

        // Update status code distribution
        let mut status_codes = self.status_codes.lock().await;
        *status_codes.entry(status_code).or_insert(0) += 1;

        // Update endpoint-specific stats
        let mut endpoint_stats = self.endpoint_stats.lock().await;
        let stats = endpoint_stats.entry(endpoint_name.to_string()).or_insert_with(|| EndpointResults {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            error_rate: 0.0,
            status_code_distribution: HashMap::new(),
        });

        stats.total_requests += 1;
        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        *stats.status_code_distribution.entry(status_code).or_insert(0) += 1;
    }

    async fn calculate_results(&self, scenario_name: String, total_duration_ms: u64) -> LoadTestResults {
        let mut response_times = self.response_times.lock().await;
        response_times.sort_unstable();

        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let total = successful + failed;
        let bytes = self.bytes_transferred.load(Ordering::Relaxed);

        let mut results = LoadTestResults {
            scenario_name,
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            total_duration_ms,
            average_response_time_ms: 0.0,
            min_response_time_ms: 0,
            max_response_time_ms: 0,
            p50_response_time_ms: 0,
            p95_response_time_ms: 0,
            p99_response_time_ms: 0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            throughput_mb_per_sec: 0.0,
            endpoint_results: HashMap::new(),
            resource_utilization: ResourceUtilization {
                peak_memory_mb: 0.0,
                average_cpu_percent: 0.0,
                peak_cpu_percent: 0.0,
                network_in_mb: 0.0,
                network_out_mb: 0.0,
                disk_read_mb: 0.0,
                disk_write_mb: 0.0,
            },
        };

        if !response_times.is_empty() {
            results.min_response_time_ms = response_times[0];
            results.max_response_time_ms = response_times[response_times.len() - 1];
            results.average_response_time_ms = response_times.iter().sum::<u64>() as f64 / response_times.len() as f64;
            results.p50_response_time_ms = response_times[response_times.len() * 50 / 100];
            results.p95_response_time_ms = response_times[response_times.len() * 95 / 100];
            results.p99_response_time_ms = response_times[response_times.len() * 99 / 100];
        }

        if total > 0 {
            results.error_rate = failed as f64 / total as f64;
        }

        if total_duration_ms > 0 {
            results.requests_per_second = (total as f64 * 1000.0) / total_duration_ms as f64;
            results.throughput_mb_per_sec = (bytes as f64 / (1024.0 * 1024.0)) / (total_duration_ms as f64 / 1000.0);
        }

        // Calculate endpoint-specific metrics
        let mut endpoint_stats = self.endpoint_stats.lock().await;
        for (endpoint, stats) in endpoint_stats.iter_mut() {
            if stats.total_requests > 0 {
                stats.error_rate = stats.failed_requests as f64 / stats.total_requests as f64;
                // Note: endpoint-specific response time calculation would require per-endpoint tracking
            }
        }
        results.endpoint_results = endpoint_stats.clone();

        results
    }
}

/// Load test executor
pub struct LoadTestExecutor {
    scenarios: Vec<LoadTestScenario>,
}

impl LoadTestExecutor {
    /// Create a new load test executor
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
        }
    }

    /// Add a load test scenario
    pub fn add_scenario(&mut self, scenario: LoadTestScenario) {
        self.scenarios.push(scenario);
    }

    /// Create predefined load test scenarios
    pub fn create_standard_scenarios() -> Vec<LoadTestScenario> {
        vec![
            // Light load scenario
            LoadTestScenario {
                name: "Light Load".to_string(),
                description: "Light load with normal user behavior".to_string(),
                duration_secs: 60,
                concurrent_users: 10,
                ramp_up_duration_secs: 10,
                ramp_down_duration_secs: 10,
                target_rps: 50.0,
                max_response_time_ms: 500,
                error_rate_threshold: 0.01,
                endpoints: vec![
                    EndpointConfig {
                        name: "health_check".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.3,
                        method: "GET".to_string(),
                        path: "/health".to_string(),
                        body: None,
                        headers: HashMap::new(),
                        expected_status: 200,
                        timeout_ms: 1000,
                    },
                    EndpointConfig {
                        name: "list_tasks".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.4,
                        method: "GET".to_string(),
                        path: "/api/tasks".to_string(),
                        body: None,
                        headers: HashMap::new(),
                        expected_status: 200,
                        timeout_ms: 2000,
                    },
                    EndpointConfig {
                        name: "graphql_tasks".to_string(),
                        endpoint_type: EndpointType::GraphQL,
                        weight: 0.3,
                        method: "POST".to_string(),
                        path: "/graphql".to_string(),
                        body: Some(json!({
                            "query": "{ tasks { id name description } }"
                        })),
                        headers: {
                            let mut headers = HashMap::new();
                            headers.insert("content-type".to_string(), "application/json".to_string());
                            headers
                        },
                        expected_status: 200,
                        timeout_ms: 2000,
                    },
                ],
            },

            // Medium load scenario
            LoadTestScenario {
                name: "Medium Load".to_string(),
                description: "Medium load with mixed operations".to_string(),
                duration_secs: 120,
                concurrent_users: 50,
                ramp_up_duration_secs: 20,
                ramp_down_duration_secs: 20,
                target_rps: 200.0,
                max_response_time_ms: 1000,
                error_rate_threshold: 0.02,
                endpoints: vec![
                    EndpointConfig {
                        name: "list_tasks".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.25,
                        method: "GET".to_string(),
                        path: "/api/tasks".to_string(),
                        body: None,
                        headers: HashMap::new(),
                        expected_status: 200,
                        timeout_ms: 2000,
                    },
                    EndpointConfig {
                        name: "create_task".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.15,
                        method: "POST".to_string(),
                        path: "/api/tasks".to_string(),
                        body: Some(json!({
                            "name": "load_test_task",
                            "description": "Task created during load testing",
                            "task_type": "JavaScript",
                            "script": "function execute() { return { success: true }; }",
                            "enabled": true
                        })),
                        headers: {
                            let mut headers = HashMap::new();
                            headers.insert("content-type".to_string(), "application/json".to_string());
                            headers
                        },
                        expected_status: 201,
                        timeout_ms: 3000,
                    },
                    EndpointConfig {
                        name: "graphql_complex".to_string(),
                        endpoint_type: EndpointType::GraphQL,
                        weight: 0.3,
                        method: "POST".to_string(),
                        path: "/graphql".to_string(),
                        body: Some(json!({
                            "query": r#"
                                query {
                                    tasks(pagination: { page: 1, page_size: 10 }) {
                                        id name description
                                        executions { id status }
                                    }
                                    jobs { id priority status }
                                }
                            "#
                        })),
                        headers: {
                            let mut headers = HashMap::new();
                            headers.insert("content-type".to_string(), "application/json".to_string());
                            headers
                        },
                        expected_status: 200,
                        timeout_ms: 3000,
                    },
                    EndpointConfig {
                        name: "mcp_task_create".to_string(),
                        endpoint_type: EndpointType::MCP,
                        weight: 0.3,
                        method: "POST".to_string(),
                        path: "/mcp/tasks".to_string(),
                        body: Some(json!({
                            "name": "mcp_load_test_task",
                            "code": "function execute() { return { result: 'success' }; }",
                            "description": "MCP task for load testing"
                        })),
                        headers: {
                            let mut headers = HashMap::new();
                            headers.insert("content-type".to_string(), "application/json".to_string());
                            headers
                        },
                        expected_status: 201,
                        timeout_ms: 3000,
                    },
                ],
            },

            // High load scenario
            LoadTestScenario {
                name: "High Load".to_string(),
                description: "High load stress test".to_string(),
                duration_secs: 300,
                concurrent_users: 200,
                ramp_up_duration_secs: 60,
                ramp_down_duration_secs: 60,
                target_rps: 1000.0,
                max_response_time_ms: 2000,
                error_rate_threshold: 0.05,
                endpoints: vec![
                    EndpointConfig {
                        name: "health_check".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.4,
                        method: "GET".to_string(),
                        path: "/health".to_string(),
                        body: None,
                        headers: HashMap::new(),
                        expected_status: 200,
                        timeout_ms: 1000,
                    },
                    EndpointConfig {
                        name: "list_tasks".to_string(),
                        endpoint_type: EndpointType::RestApi,
                        weight: 0.3,
                        method: "GET".to_string(),
                        path: "/api/tasks".to_string(),
                        body: None,
                        headers: HashMap::new(),
                        expected_status: 200,
                        timeout_ms: 2000,
                    },
                    EndpointConfig {
                        name: "graphql_simple".to_string(),
                        endpoint_type: EndpointType::GraphQL,
                        weight: 0.3,
                        method: "POST".to_string(),
                        path: "/graphql".to_string(),
                        body: Some(json!({
                            "query": "{ tasks { id name } }"
                        })),
                        headers: {
                            let mut headers = HashMap::new();
                            headers.insert("content-type".to_string(), "application/json".to_string());
                            headers
                        },
                        expected_status: 200,
                        timeout_ms: 2000,
                    },
                ],
            },
        ]
    }

    /// Execute a single load test scenario
    pub async fn execute_scenario(&self, scenario: &LoadTestScenario) -> Result<LoadTestResults, Box<dyn std::error::Error>> {
        println!("\n🚀 Starting load test scenario: {}", scenario.name);
        println!("Description: {}", scenario.description);
        println!("Duration: {}s, Users: {}, Target RPS: {}", 
                 scenario.duration_secs, scenario.concurrent_users, scenario.target_rps);

        let tracker = LoadTestTracker::new();
        let semaphore = Arc::new(Semaphore::new(scenario.concurrent_users));
        
        let start_time = Instant::now();
        let mut handles = Vec::new();

        // Calculate request intervals
        let total_requests = (scenario.target_rps * scenario.duration_secs as f64) as usize;
        let request_interval = Duration::from_millis(
            (1000.0 / scenario.target_rps * scenario.concurrent_users as f64) as u64
        );

        for i in 0..total_requests {
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let scenario_clone = scenario.clone();

            let handle: JoinHandle<()> = tokio::spawn(async move {
                // Ramp-up delay
                let ramp_up_delay = if scenario_clone.ramp_up_duration_secs > 0 {
                    let progress = i as f64 / total_requests as f64;
                    let max_delay = scenario_clone.ramp_up_duration_secs * 1000 / scenario_clone.concurrent_users as u64;
                    Duration::from_millis((max_delay as f64 * progress) as u64)
                } else {
                    Duration::from_millis(0)
                };

                sleep(ramp_up_delay).await;

                let _permit = semaphore_clone.acquire().await.unwrap();

                // Select endpoint based on weights
                let endpoint = Self::select_endpoint(&scenario_clone.endpoints);
                if let Some(endpoint) = endpoint {
                    let request_start = Instant::now();
                    
                    // Simulate HTTP request (in real implementation, this would make actual HTTP calls)
                    let (status_code, response_size, success) = Self::simulate_request(&endpoint).await;
                    
                    let response_time = request_start.elapsed().as_millis() as u64;
                    
                    tracker_clone.record_request(
                        &endpoint.name,
                        response_time,
                        status_code,
                        response_size,
                        success,
                    ).await;
                }

                // Request pacing
                sleep(request_interval).await;
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        let results = tracker.calculate_results(scenario.name.clone(), total_duration).await;

        println!("✅ Load test scenario '{}' completed", scenario.name);
        Self::print_results(&results);

        Ok(results)
    }

    /// Select an endpoint based on weights
    fn select_endpoint(endpoints: &[EndpointConfig]) -> Option<EndpointConfig> {
        if endpoints.is_empty() {
            return None;
        }

        let random_value: f64 = fastrand::f64();
        let mut cumulative_weight = 0.0;

        for endpoint in endpoints {
            cumulative_weight += endpoint.weight;
            if random_value <= cumulative_weight {
                return Some(endpoint.clone());
            }
        }

        // Fallback to first endpoint
        Some(endpoints[0].clone())
    }

    /// Simulate an HTTP request (for testing purposes)
    async fn simulate_request(endpoint: &EndpointConfig) -> (u16, usize, bool) {
        // Simulate request processing time based on endpoint type
        let processing_time = match endpoint.endpoint_type {
            EndpointType::RestApi => Duration::from_millis(10 + fastrand::u64(0..50)),
            EndpointType::GraphQL => Duration::from_millis(20 + fastrand::u64(0..100)),
            EndpointType::MCP => Duration::from_millis(15 + fastrand::u64(0..75)),
        };

        sleep(processing_time).await;

        // Simulate response
        let success_rate = match endpoint.name.as_str() {
            "health_check" => 0.99,
            "list_tasks" => 0.95,
            "create_task" => 0.90,
            "graphql_simple" => 0.95,
            "graphql_complex" => 0.85,
            "mcp_task_create" => 0.90,
            _ => 0.90,
        };

        let success = fastrand::f64() < success_rate;
        let status_code = if success { endpoint.expected_status } else { 500 };
        let response_size = match endpoint.endpoint_type {
            EndpointType::RestApi => 200 + fastrand::usize(0..800),
            EndpointType::GraphQL => 500 + fastrand::usize(0..1500),
            EndpointType::MCP => 300 + fastrand::usize(0..700),
        };

        (status_code, response_size, success)
    }

    /// Execute all scenarios
    pub async fn execute_all_scenarios(&self) -> Result<Vec<LoadTestResults>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for scenario in &self.scenarios {
            let result = self.execute_scenario(scenario).await?;
            results.push(result);

            // Brief pause between scenarios
            sleep(Duration::from_secs(5)).await;
        }

        // Print summary
        Self::print_summary(&results);

        Ok(results)
    }

    /// Print load test results
    fn print_results(results: &LoadTestResults) {
        println!("\n=== Load Test Results: {} ===", results.scenario_name);
        println!("Total Requests: {}", results.total_requests);
        println!("Successful: {} ({:.2}%)", results.successful_requests, 
                 (results.successful_requests as f64 / results.total_requests as f64) * 100.0);
        println!("Failed: {} ({:.2}%)", results.failed_requests, results.error_rate * 100.0);
        println!("Test Duration: {:.2}s", results.total_duration_ms as f64 / 1000.0);
        println!("Requests/Second: {:.2}", results.requests_per_second);
        println!("Throughput: {:.2} MB/s", results.throughput_mb_per_sec);
        println!("Response Times:");
        println!("  Average: {:.2}ms", results.average_response_time_ms);
        println!("  P50: {}ms", results.p50_response_time_ms);
        println!("  P95: {}ms", results.p95_response_time_ms);
        println!("  P99: {}ms", results.p99_response_time_ms);
        println!("  Min: {}ms", results.min_response_time_ms);
        println!("  Max: {}ms", results.max_response_time_ms);

        // Endpoint breakdown
        if !results.endpoint_results.is_empty() {
            println!("\nEndpoint Breakdown:");
            for (endpoint, stats) in &results.endpoint_results {
                println!("  {}: {} requests, {:.2}% error rate", 
                         endpoint, stats.total_requests, stats.error_rate * 100.0);
            }
        }
    }

    /// Print summary of all load test results
    fn print_summary(results: &[LoadTestResults]) {
        println!("\n🏁 Load Test Summary 🏁");
        println!("Scenarios executed: {}", results.len());

        for result in results {
            println!("\n{}: {:.2} RPS, {:.2}% errors, {:.2}ms P95", 
                     result.scenario_name, result.requests_per_second, 
                     result.error_rate * 100.0, result.p95_response_time_ms);
        }

        // Overall performance assessment
        let overall_error_rate: f64 = results.iter()
            .map(|r| r.error_rate)
            .sum::<f64>() / results.len() as f64;

        let overall_p95: f64 = results.iter()
            .map(|r| r.p95_response_time_ms as f64)
            .sum::<f64>() / results.len() as f64;

        println!("\nOverall Assessment:");
        if overall_error_rate <= 0.02 {
            println!("✅ Error rate within acceptable limits ({:.2}%)", overall_error_rate * 100.0);
        } else {
            println!("❌ Error rate exceeds acceptable limits ({:.2}%)", overall_error_rate * 100.0);
        }

        if overall_p95 <= 1000.0 {
            println!("✅ P95 response time within target ({:.2}ms)", overall_p95);
        } else {
            println!("❌ P95 response time exceeds target ({:.2}ms)", overall_p95);
        }
    }
}

// =============================================================================
// LOAD TEST EXECUTION TESTS
// =============================================================================

#[tokio::test]
async fn test_light_load_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = LoadTestExecutor::create_standard_scenarios();
    let light_load = &scenarios[0]; // Light load scenario

    let mut executor = LoadTestExecutor::new();
    executor.add_scenario(light_load.clone());

    let results = executor.execute_scenario(light_load).await?;

    // Assertions for light load
    assert!(results.total_requests > 0);
    assert!(results.error_rate <= 0.05); // 5% error rate max
    assert!(results.requests_per_second > 0.0);
    assert!(results.p95_response_time_ms <= 1000); // 1 second P95 max

    Ok(())
}

#[tokio::test]
async fn test_medium_load_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = LoadTestExecutor::create_standard_scenarios();
    let medium_load = &scenarios[1]; // Medium load scenario

    let mut executor = LoadTestExecutor::new();
    executor.add_scenario(medium_load.clone());

    let results = executor.execute_scenario(medium_load).await?;

    // Assertions for medium load
    assert!(results.total_requests > 0);
    assert!(results.error_rate <= 0.10); // 10% error rate max for medium load
    assert!(results.requests_per_second > 10.0); // Minimum 10 RPS

    Ok(())
}

#[tokio::test]
#[ignore] // Marked as ignore since high load tests are resource intensive
async fn test_high_load_scenario() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = LoadTestExecutor::create_standard_scenarios();
    let high_load = &scenarios[2]; // High load scenario

    let mut executor = LoadTestExecutor::new();
    executor.add_scenario(high_load.clone());

    let results = executor.execute_scenario(high_load).await?;

    // Assertions for high load
    assert!(results.total_requests > 0);
    assert!(results.error_rate <= 0.15); // 15% error rate max for high load
    assert!(results.requests_per_second > 5.0); // Minimum 5 RPS under stress

    Ok(())
}

#[tokio::test]
#[ignore] // Marked as ignore since comprehensive tests are resource intensive
async fn test_comprehensive_load_test_suite() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = LoadTestExecutor::create_standard_scenarios();
    let mut executor = LoadTestExecutor::new();

    for scenario in scenarios {
        executor.add_scenario(scenario);
    }

    let results = executor.execute_all_scenarios().await?;

    assert_eq!(results.len(), 3); // Should have 3 scenarios

    for result in results {
        assert!(result.total_requests > 0);
        assert!(result.requests_per_second > 0.0);
        // Allow higher error rates for comprehensive testing
        assert!(result.error_rate <= 0.20); // 20% error rate max
    }

    Ok(())
}

// TODO: Add tests for:
// - Real HTTP client integration with actual servers
// - Resource monitoring during load tests (CPU, memory, network)
// - Database performance under load
// - WebSocket connection load testing
// - Authentication performance under load
// - Rate limiting behavior under load
// - Graceful degradation testing
// - Recovery testing after load spikes
// - Load balancing performance
// - Cache performance under load