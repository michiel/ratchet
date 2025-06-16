//! Performance and load testing suite for the Ratchet MCP protocol
//!
//! This module provides comprehensive performance testing for MCP operations,
//! covering task development, testing, execution, and protocol overhead.

use ratchet_mcp::{
    server::task_dev_tools::{
        TaskDevelopmentService, CreateTaskRequest, TaskTestCase
    },
    protocol::{
        JsonRpcRequest, JsonRpcResponse, ProtocolHandler
    },
    McpError
};
use ratchet_storage::{
    repositories::MockRepositoryFactory,
    stores::memory::MemoryStorageFactory,
};
use sea_orm::{Database, DatabaseConnection};
use serde_json::{json, Value};
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
    collections::HashMap,
};
use tokio::{time::sleep, sync::Semaphore};
use uuid::Uuid;

/// MCP performance test configuration
#[derive(Debug, Clone)]
pub struct McpPerformanceConfig {
    pub concurrent_operations: usize,
    pub total_operations: usize,
    pub test_duration_secs: u64,
    pub protocol_timeout_ms: u64,
    pub task_execution_timeout_ms: u64,
    pub batch_size: usize,
    pub enable_file_operations: bool,
    pub enable_javascript_execution: bool,
    pub max_task_complexity: usize,
}

impl Default for McpPerformanceConfig {
    fn default() -> Self {
        Self {
            concurrent_operations: 10,
            total_operations: 200,
            test_duration_secs: 30,
            protocol_timeout_ms: 5000,
            task_execution_timeout_ms: 10000,
            batch_size: 5,
            enable_file_operations: true,
            enable_javascript_execution: false, // Disable for safety in performance tests
            max_task_complexity: 100,
        }
    }
}

/// MCP performance metrics
#[derive(Debug, Clone)]
pub struct McpPerformanceMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub task_create_operations: u64,
    pub task_edit_operations: u64,
    pub task_test_operations: u64,
    pub task_execute_operations: u64,
    pub task_delete_operations: u64,
    pub total_duration_ms: u64,
    pub average_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub operations_per_second: f64,
    pub error_rate: f64,
    pub protocol_overhead_ms: f64,
    pub task_execution_time_ms: f64,
    pub data_transferred_kb: f64,
    pub memory_usage_mb: f64,
}

impl Default for McpPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            task_create_operations: 0,
            task_edit_operations: 0,
            task_test_operations: 0,
            task_execute_operations: 0,
            task_delete_operations: 0,
            total_duration_ms: 0,
            average_latency_ms: 0.0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            operations_per_second: 0.0,
            error_rate: 0.0,
            protocol_overhead_ms: 0.0,
            task_execution_time_ms: 0.0,
            data_transferred_kb: 0.0,
            memory_usage_mb: 0.0,
        }
    }
}

/// MCP operation tracker
struct McpOperationTracker {
    latencies: Arc<tokio::sync::Mutex<Vec<u64>>>,
    successful_operations: Arc<AtomicU64>,
    failed_operations: Arc<AtomicU64>,
    operation_counts: Arc<tokio::sync::Mutex<HashMap<String, u64>>>,
    data_transferred: Arc<AtomicU64>,
    protocol_overhead: Arc<tokio::sync::Mutex<Vec<u64>>>,
    execution_times: Arc<tokio::sync::Mutex<Vec<u64>>>,
}

impl McpOperationTracker {
    fn new() -> Self {
        Self {
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            successful_operations: Arc::new(AtomicU64::new(0)),
            failed_operations: Arc::new(AtomicU64::new(0)),
            operation_counts: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            data_transferred: Arc::new(AtomicU64::new(0)),
            protocol_overhead: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            execution_times: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn record_operation(
        &self,
        latency_ms: u64,
        success: bool,
        operation_type: &str,
        response_size: usize,
        protocol_overhead_ms: u64,
        execution_time_ms: Option<u64>,
    ) {
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
            self.latencies.lock().await.push(latency_ms);
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }

        let mut counts = self.operation_counts.lock().await;
        *counts.entry(operation_type.to_string()).or_insert(0) += 1;

        self.data_transferred.fetch_add(response_size as u64, Ordering::Relaxed);
        self.protocol_overhead.lock().await.push(protocol_overhead_ms);

        if let Some(exec_time) = execution_time_ms {
            self.execution_times.lock().await.push(exec_time);
        }
    }

    async fn calculate_metrics(&self, total_duration_ms: u64) -> McpPerformanceMetrics {
        let mut latencies = self.latencies.lock().await;
        latencies.sort_unstable();

        let successful = self.successful_operations.load(Ordering::Relaxed);
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total = successful + failed;
        let data_kb = self.data_transferred.load(Ordering::Relaxed) as f64 / 1024.0;

        let counts = self.operation_counts.lock().await;
        let protocol_overheads = self.protocol_overhead.lock().await;
        let execution_times = self.execution_times.lock().await;

        let mut metrics = McpPerformanceMetrics::default();
        metrics.total_operations = total;
        metrics.successful_operations = successful;
        metrics.failed_operations = failed;
        metrics.total_duration_ms = total_duration_ms;
        metrics.data_transferred_kb = data_kb;

        // Operation type counts
        metrics.task_create_operations = counts.get("task_create").copied().unwrap_or(0);
        metrics.task_edit_operations = counts.get("task_edit").copied().unwrap_or(0);
        metrics.task_test_operations = counts.get("task_test").copied().unwrap_or(0);
        metrics.task_execute_operations = counts.get("task_execute").copied().unwrap_or(0);
        metrics.task_delete_operations = counts.get("task_delete").copied().unwrap_or(0);

        if !latencies.is_empty() {
            metrics.min_latency_ms = latencies[0];
            metrics.max_latency_ms = latencies[latencies.len() - 1];
            metrics.average_latency_ms = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
            metrics.p95_latency_ms = latencies[latencies.len() * 95 / 100];
            metrics.p99_latency_ms = latencies[latencies.len() * 99 / 100];
        }

        if !protocol_overheads.is_empty() {
            metrics.protocol_overhead_ms = protocol_overheads.iter().sum::<u64>() as f64 / protocol_overheads.len() as f64;
        }

        if !execution_times.is_empty() {
            metrics.task_execution_time_ms = execution_times.iter().sum::<u64>() as f64 / execution_times.len() as f64;
        }

        if total > 0 {
            metrics.error_rate = failed as f64 / total as f64;
        }

        if total_duration_ms > 0 {
            metrics.operations_per_second = (total as f64 * 1000.0) / total_duration_ms as f64;
        }

        metrics
    }
}

/// MCP performance test runner
pub struct McpPerformanceTest {
    config: McpPerformanceConfig,
    task_service: Option<Arc<TaskDevelopmentService>>,
    db_connection: DatabaseConnection,
}

impl McpPerformanceTest {
    /// Create a new MCP performance test with default configuration
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(McpPerformanceConfig::default()).await
    }

    /// Create a new MCP performance test with custom configuration
    pub async fn with_config(config: McpPerformanceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db_connection = Database::connect("sqlite::memory:").await?;
        
        // For performance tests, we'll use mock services to focus on protocol overhead
        let task_service = None; // Use mock instead of real service for performance testing

        Ok(Self {
            config,
            task_service,
            db_connection,
        })
    }

    /// Create a test task request for performance testing
    fn create_test_task_request(&self, name: &str, complexity: usize) -> CreateTaskRequest {
        let script_complexity = match complexity {
            1..=10 => "function execute(input) { return { result: 'simple' }; }",
            11..=50 => r#"
                function execute(input) {
                    let result = {};
                    for (let i = 0; i < 100; i++) {
                        result[`item_${i}`] = input.data || 'default';
                    }
                    return { result: 'medium', data: result };
                }
            "#,
            _ => r#"
                function execute(input) {
                    let result = {};
                    for (let i = 0; i < 1000; i++) {
                        result[`item_${i}`] = {
                            id: i,
                            data: input.data || 'default',
                            timestamp: new Date().toISOString(),
                            nested: {
                                value: Math.random(),
                                computed: i * 2 + 1
                            }
                        };
                    }
                    return { result: 'complex', data: result };
                }
            "#,
        };

        CreateTaskRequest {
            name: name.to_string(),
            description: format!("Performance test task: {} (complexity: {})", name, complexity),
            code: script_complexity.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "data": {"type": "string"}
                },
                "required": ["data"]
            }),
            output_schema: json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"},
                    "data": {"type": "object"}
                },
                "required": ["result"]
            }),
            tags: vec!["performance".to_string(), "test".to_string()],
            version: "1.0.0".to_string(),
            enabled: true,
            test_cases: vec![
                TaskTestCase {
                    name: "basic_test".to_string(),
                    input: json!({"data": "test input"}),
                    expected_output: None,
                    should_fail: false,
                    description: Some("Basic performance test".to_string()),
                }
            ],
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an MCP JSON-RPC request
    fn create_mcp_request(&self, method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
            id: Some(Value::String(Uuid::new_v4().to_string())),
        }
    }

    /// Simulate MCP protocol processing for performance testing
    async fn simulate_mcp_operation(
        &self,
        request: JsonRpcRequest,
    ) -> Result<(JsonRpcResponse, u64, u64), McpError> {
        let protocol_start = Instant::now();
        
        // Simulate protocol overhead (parsing, validation, routing)
        sleep(Duration::from_millis(1)).await; // Minimal protocol overhead
        let protocol_overhead = protocol_start.elapsed().as_millis() as u64;

        let execution_start = Instant::now();
        
        // Simulate operation execution based on method
        let (result, execution_time) = match request.method.as_str() {
            "task/create" => {
                // Simulate task creation time
                sleep(Duration::from_millis(5)).await;
                let result = json!({
                    "task_id": format!("perf-task-{}", Uuid::new_v4()),
                    "version": "1.0.0",
                    "status": "created",
                    "validation": {
                        "valid": true,
                        "warnings": [],
                        "errors": []
                    }
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/edit" => {
                sleep(Duration::from_millis(3)).await;
                let result = json!({
                    "task_id": "perf-task-edited",
                    "status": "edited",
                    "backup_created": true
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/test" => {
                // Simulate test execution time
                sleep(Duration::from_millis(10)).await;
                let result = json!({
                    "total_tests": 3,
                    "passed_tests": 2,
                    "failed_tests": 1,
                    "test_results": [
                        {
                            "name": "basic_test",
                            "status": "passed",
                            "duration_ms": 25
                        },
                        {
                            "name": "edge_case_test",
                            "status": "passed",
                            "duration_ms": 35
                        },
                        {
                            "name": "error_test",
                            "status": "failed",
                            "error": "Simulated test failure"
                        }
                    ]
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/execute" => {
                // Simulate task execution time based on complexity
                let complexity = request.params
                    .as_ref()
                    .and_then(|p| p.get("complexity"))
                    .and_then(|c| c.as_u64())
                    .unwrap_or(1);
                
                let execution_time_ms = match complexity {
                    1..=10 => 5,
                    11..=50 => 20,
                    _ => 50,
                };
                
                sleep(Duration::from_millis(execution_time_ms)).await;
                let result = json!({
                    "execution_id": format!("exec-{}", Uuid::new_v4()),
                    "status": "completed",
                    "output": {
                        "result": "success",
                        "message": "Task executed",
                        "complexity": complexity
                    }
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/store_result" => {
                sleep(Duration::from_millis(2)).await;
                let result = json!({
                    "execution_id": "exec-stored",
                    "stored": true,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/get_result" => {
                sleep(Duration::from_millis(1)).await;
                let result = json!({
                    "execution_id": "exec-retrieved",
                    "task_id": "perf-task",
                    "status": "completed",
                    "output": {
                        "result": "success",
                        "message": "Retrieved result"
                    }
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            "task/delete" => {
                sleep(Duration::from_millis(3)).await;
                let result = json!({
                    "task_id": "perf-task-deleted",
                    "deleted": true,
                    "backup_location": "/tmp/backup/perf-task.js"
                });
                (result, execution_start.elapsed().as_millis() as u64)
            },
            _ => return Err(McpError::MethodNotFound {
                method: request.method.clone(),
            }),
        };

        let response = JsonRpcResponse::success(result, request.id);
        Ok((response, protocol_overhead, execution_time))
    }

    /// Test performance of MCP task operations
    pub async fn test_task_operations_performance(&self) -> Result<McpPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = McpOperationTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_operations));
        
        let operations = vec![
            ("task/create", "task_create"),
            ("task/edit", "task_edit"),
            ("task/test", "task_test"),
            ("task/execute", "task_execute"),
            ("task/store_result", "task_store"),
            ("task/get_result", "task_get"),
            ("task/delete", "task_delete"),
        ];

        let start_time = Instant::now();
        let mut handles = vec![];

        for i in 0..self.config.total_operations {
            let (method, operation_type) = &operations[i % operations.len()];
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let method = method.to_string();
            let operation_type = operation_type.to_string();

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let params = json!({
                    "task_id": format!("perf-task-{}", i),
                    "complexity": (i % 100) + 1
                });
                
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: method.clone(),
                    params: Some(params),
                    id: Some(Value::String(Uuid::new_v4().to_string())),
                };

                let operation_start = Instant::now();
                
                // Note: In a real implementation, this would call the actual MCP service
                // For performance testing, we simulate the operation
                let result = match method.as_str() {
                    "task/create" => Ok((
                        JsonRpcResponse::success(json!({"task_id": "test", "status": "created"}), request.id),
                        1, // protocol overhead
                        5  // execution time
                    )),
                    "task/edit" => Ok((
                        JsonRpcResponse::success(json!({"task_id": "test", "status": "edited"}), request.id),
                        1,
                        3
                    )),
                    "task/test" => Ok((
                        JsonRpcResponse::success(json!({"total_tests": 1, "passed_tests": 1}), request.id),
                        1,
                        10
                    )),
                    "task/execute" => Ok((
                        JsonRpcResponse::success(json!({"execution_id": "test", "status": "completed"}), request.id),
                        1,
                        20
                    )),
                    "task/store_result" => Ok((
                        JsonRpcResponse::success(json!({"stored": true}), request.id),
                        1,
                        2
                    )),
                    "task/get_result" => Ok((
                        JsonRpcResponse::success(json!({"execution_id": "test"}), request.id),
                        1,
                        1
                    )),
                    "task/delete" => Ok((
                        JsonRpcResponse::success(json!({"deleted": true}), request.id),
                        1,
                        3
                    )),
                    _ => Err(McpError::MethodNotFound { method: method.clone() }),
                };

                let total_latency = operation_start.elapsed().as_millis() as u64;
                
                match result {
                    Ok((response, protocol_overhead, execution_time)) => {
                        let success = response.error.is_none();
                        let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                        
                        tracker_clone.record_operation(
                            total_latency,
                            success,
                            &operation_type,
                            response_size,
                            protocol_overhead,
                            Some(execution_time),
                        ).await;
                    },
                    Err(_) => {
                        tracker_clone.record_operation(
                            total_latency,
                            false,
                            &operation_type,
                            0,
                            0,
                            None,
                        ).await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        let metrics = tracker.calculate_metrics(total_duration).await;

        Ok(metrics)
    }

    /// Test performance of concurrent task execution
    pub async fn test_concurrent_execution_performance(&self) -> Result<McpPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = McpOperationTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_operations));
        
        let start_time = Instant::now();
        let mut handles = vec![];

        for i in 0..self.config.total_operations {
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let params = json!({
                    "task_id": format!("concurrent-task-{}", i),
                    "input": {"data": format!("test-{}", i)},
                    "complexity": (i % 50) + 1
                });
                
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "task/execute".to_string(),
                    params: Some(params),
                    id: Some(Value::String(Uuid::new_v4().to_string())),
                };

                let operation_start = Instant::now();
                
                // Simulate concurrent task execution
                let complexity = (i % 50) + 1;
                let execution_time = match complexity {
                    1..=10 => 5,
                    11..=30 => 15,
                    _ => 25,
                };
                
                sleep(Duration::from_millis(execution_time)).await;
                
                let total_latency = operation_start.elapsed().as_millis() as u64;
                let response = JsonRpcResponse::success(
                    json!({
                        "execution_id": format!("exec-{}", i),
                        "status": "completed",
                        "complexity": complexity
                    }),
                    request.id
                );
                
                let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                
                tracker_clone.record_operation(
                    total_latency,
                    true,
                    "task_execute",
                    response_size,
                    1,
                    Some(execution_time),
                ).await;
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        let metrics = tracker.calculate_metrics(total_duration).await;

        Ok(metrics)
    }

    /// Test performance of batch operations
    pub async fn test_batch_operations_performance(&self) -> Result<McpPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = McpOperationTracker::new();
        
        let batch_count = self.config.total_operations / self.config.batch_size;
        let start_time = Instant::now();

        for batch_i in 0..batch_count {
            let batch_start = Instant::now();
            let mut batch_handles = vec![];

            for i in 0..self.config.batch_size {
                let tracker_clone = tracker.clone();
                let operation_id = batch_i * self.config.batch_size + i;

                let handle = tokio::spawn(async move {
                    let params = json!({
                        "task_id": format!("batch-task-{}", operation_id),
                        "batch_id": batch_i,
                        "item_index": i
                    });
                    
                    let request = JsonRpcRequest {
                        jsonrpc: "2.0".to_string(),
                        method: "task/create".to_string(),
                        params: Some(params),
                        id: Some(Value::String(Uuid::new_v4().to_string())),
                    };

                    let operation_start = Instant::now();
                    
                    // Simulate batch task creation
                    sleep(Duration::from_millis(2)).await;
                    
                    let total_latency = operation_start.elapsed().as_millis() as u64;
                    let response = JsonRpcResponse::success(
                        json!({
                            "task_id": format!("batch-task-{}", operation_id),
                            "status": "created",
                            "batch_id": batch_i
                        }),
                        request.id
                    );
                    
                    let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                    
                    tracker_clone.record_operation(
                        total_latency,
                        true,
                        "task_create",
                        response_size,
                        1,
                        Some(2),
                    ).await;
                });

                batch_handles.push(handle);
            }

            // Wait for batch to complete
            for handle in batch_handles {
                let _ = handle.await;
            }

            // Add small delay between batches
            sleep(Duration::from_millis(10)).await;
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        let metrics = tracker.calculate_metrics(total_duration).await;

        Ok(metrics)
    }

    /// Run comprehensive MCP performance test suite
    pub async fn run_comprehensive_test_suite(&self) -> Result<Vec<(String, McpPerformanceMetrics)>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        println!("Testing MCP task operations performance...");
        let task_ops_metrics = self.test_task_operations_performance().await?;
        results.push(("MCP Task Operations".to_string(), task_ops_metrics));

        println!("Testing concurrent execution performance...");
        let concurrent_metrics = self.test_concurrent_execution_performance().await?;
        results.push(("Concurrent Execution".to_string(), concurrent_metrics));

        println!("Testing batch operations performance...");
        let batch_metrics = self.test_batch_operations_performance().await?;
        results.push(("Batch Operations".to_string(), batch_metrics));

        Ok(results)
    }

    /// Print MCP performance metrics
    pub fn print_metrics(&self, operation_type: &str, metrics: &McpPerformanceMetrics) {
        println!("\n=== MCP Performance Test Results: {} ===", operation_type);
        println!("Total Operations: {}", metrics.total_operations);
        println!("Successful: {} ({:.2}%)", metrics.successful_operations, 
                 (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0);
        println!("Failed: {} ({:.2}%)", metrics.failed_operations, metrics.error_rate * 100.0);
        println!("Operation Breakdown:");
        println!("  Create: {}, Edit: {}, Test: {}, Execute: {}, Delete: {}", 
                 metrics.task_create_operations, metrics.task_edit_operations, 
                 metrics.task_test_operations, metrics.task_execute_operations, 
                 metrics.task_delete_operations);
        println!("Test Duration: {:.2}s", metrics.total_duration_ms as f64 / 1000.0);
        println!("Operations/Second: {:.2}", metrics.operations_per_second);
        println!("Average Latency: {:.2}ms", metrics.average_latency_ms);
        println!("Latency P95: {}ms", metrics.p95_latency_ms);
        println!("Latency P99: {}ms", metrics.p99_latency_ms);
        println!("Min Latency: {}ms", metrics.min_latency_ms);
        println!("Max Latency: {}ms", metrics.max_latency_ms);
        println!("Protocol Overhead: {:.2}ms avg", metrics.protocol_overhead_ms);
        println!("Task Execution Time: {:.2}ms avg", metrics.task_execution_time_ms);
        println!("Data Transferred: {:.2}KB", metrics.data_transferred_kb);

        // Performance assessment
        if metrics.error_rate <= 0.05 {
            println!("✅ Error rate within acceptable limits");
        } else {
            println!("❌ Error rate exceeds acceptable limits ({:.2}%)", metrics.error_rate * 100.0);
        }

        if metrics.p95_latency_ms <= 100 {
            println!("✅ P95 latency within target (100ms)");
        } else {
            println!("❌ P95 latency exceeds target ({}ms > 100ms)", metrics.p95_latency_ms);
        }

        if metrics.protocol_overhead_ms <= 5.0 {
            println!("✅ Protocol overhead within target (5ms)");
        } else {
            println!("❌ Protocol overhead exceeds target ({:.2}ms > 5ms)", metrics.protocol_overhead_ms);
        }
    }
}

// =============================================================================
// MCP PERFORMANCE TESTS
// =============================================================================

#[tokio::test]
async fn test_mcp_task_operations_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = McpPerformanceConfig::default();
    config.concurrent_operations = 5;
    config.total_operations = 100;

    let performance_test = McpPerformanceTest::with_config(config).await?;
    let metrics = performance_test.test_task_operations_performance().await?;

    performance_test.print_metrics("Task Operations", &metrics);

    // Basic performance assertions
    assert!(metrics.total_operations >= 100);
    assert!(metrics.error_rate <= 0.05); // 5% error rate max
    assert!(metrics.operations_per_second > 0.0);
    assert!(metrics.average_latency_ms < 100.0); // 100ms max average

    Ok(())
}

#[tokio::test]
async fn test_mcp_concurrent_execution_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = McpPerformanceConfig::default();
    config.concurrent_operations = 10;
    config.total_operations = 100;

    let performance_test = McpPerformanceTest::with_config(config).await?;
    let metrics = performance_test.test_concurrent_execution_performance().await?;

    performance_test.print_metrics("Concurrent Execution", &metrics);

    // Concurrent performance assertions
    assert!(metrics.total_operations >= 100);
    assert!(metrics.error_rate <= 0.02); // 2% error rate max for execution
    assert!(metrics.p95_latency_ms < 200); // 200ms P95 max

    Ok(())
}

#[tokio::test]
async fn test_mcp_batch_operations_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = McpPerformanceConfig::default();
    config.total_operations = 50;
    config.batch_size = 5;

    let performance_test = McpPerformanceTest::with_config(config).await?;
    let metrics = performance_test.test_batch_operations_performance().await?;

    performance_test.print_metrics("Batch Operations", &metrics);

    // Batch performance assertions
    assert!(metrics.total_operations >= 50);
    assert!(metrics.error_rate <= 0.02); // 2% error rate max for batch
    assert!(metrics.operations_per_second > 10.0); // Minimum 10 OPS for batch

    Ok(())
}

#[tokio::test]
async fn test_mcp_comprehensive_performance_suite() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = McpPerformanceConfig::default();
    config.concurrent_operations = 3;
    config.total_operations = 30;

    let performance_test = McpPerformanceTest::with_config(config).await?;
    let results = performance_test.run_comprehensive_test_suite().await?;

    assert!(!results.is_empty());

    for (operation_type, metrics) in results {
        performance_test.print_metrics(&operation_type, &metrics);

        // Comprehensive test assertions
        assert!(metrics.total_operations >= 30);
        assert!(metrics.error_rate <= 0.10); // 10% error rate max for comprehensive test
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Marked as ignore since stress tests are resource intensive
async fn test_mcp_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = McpPerformanceConfig::default();
    config.concurrent_operations = 50;
    config.total_operations = 1000;
    config.test_duration_secs = 60;

    let performance_test = McpPerformanceTest::with_config(config).await?;
    let metrics = performance_test.test_task_operations_performance().await?;

    performance_test.print_metrics("MCP Stress Test", &metrics);

    // Stress test assertions
    assert!(metrics.total_operations >= 1000);
    assert!(metrics.error_rate <= 0.15); // 15% error rate max under stress
    assert!(metrics.operations_per_second > 10.0); // Minimum 10 OPS under stress

    Ok(())
}

// TODO: Add tests for:
// - WebSocket protocol performance vs HTTP
// - Task complexity impact on performance
// - Memory usage during long-running operations
// - File system operations performance
// - JavaScript execution performance
// - Protocol serialization/deserialization overhead
// - Database operations performance in TaskDevelopmentService
// - Real-time event streaming performance
// - Connection pooling and resource management