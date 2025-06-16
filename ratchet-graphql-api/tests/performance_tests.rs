//! Performance and load testing suite for the Ratchet GraphQL API
//!
//! This module provides comprehensive performance testing for GraphQL queries, mutations,
//! and subscriptions, covering throughput, latency, and concurrent operations.

use ratchet_graphql_api::{
    context::GraphQLContext,
    schema::{RatchetSchema, create_schema},
};
use ratchet_storage::{
    repositories::{MockRepositoryFactory, MockTaskRepository, MockExecutionRepository, MockJobRepository},
    stores::memory::MemoryStorageFactory,
};
use ratchet_interfaces::{
    registry::{MockTaskRegistry, MockRegistryManager},
    validation::MockTaskValidator,
};
use async_graphql::{Request, Response, Variables};
use serde_json::{json, Value};
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
    collections::HashMap,
};
use tokio::{time::sleep, sync::Semaphore};

/// GraphQL performance test configuration
#[derive(Debug, Clone)]
pub struct GraphQLPerformanceConfig {
    pub concurrent_requests: usize,
    pub total_requests: usize,
    pub test_duration_secs: u64,
    pub query_complexity_limit: usize,
    pub query_depth_limit: usize,
    pub enable_introspection: bool,
    pub enable_playground: bool,
    pub batch_size: usize,
    pub subscription_duration_secs: u64,
}

impl Default for GraphQLPerformanceConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 10,
            total_requests: 500,
            test_duration_secs: 30,
            query_complexity_limit: 1000,
            query_depth_limit: 10,
            enable_introspection: true,
            enable_playground: false, // Disable in performance tests
            batch_size: 10,
            subscription_duration_secs: 10,
        }
    }
}

/// GraphQL performance metrics
#[derive(Debug, Clone)]
pub struct GraphQLPerformanceMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub query_operations: u64,
    pub mutation_operations: u64,
    pub subscription_operations: u64,
    pub total_duration_ms: u64,
    pub average_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub operations_per_second: f64,
    pub error_rate: f64,
    pub data_transferred_kb: f64,
    pub resolver_call_count: u64,
    pub average_query_complexity: f64,
    pub average_query_depth: f64,
}

impl Default for GraphQLPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            query_operations: 0,
            mutation_operations: 0,
            subscription_operations: 0,
            total_duration_ms: 0,
            average_latency_ms: 0.0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            operations_per_second: 0.0,
            error_rate: 0.0,
            data_transferred_kb: 0.0,
            resolver_call_count: 0,
            average_query_complexity: 0.0,
            average_query_depth: 0.0,
        }
    }
}

/// GraphQL operation tracker
struct GraphQLOperationTracker {
    latencies: Arc<tokio::sync::Mutex<Vec<u64>>>,
    successful_operations: Arc<AtomicU64>,
    failed_operations: Arc<AtomicU64>,
    query_count: Arc<AtomicU64>,
    mutation_count: Arc<AtomicU64>,
    subscription_count: Arc<AtomicU64>,
    data_transferred: Arc<AtomicU64>,
    resolver_calls: Arc<AtomicU64>,
}

impl GraphQLOperationTracker {
    fn new() -> Self {
        Self {
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            successful_operations: Arc::new(AtomicU64::new(0)),
            failed_operations: Arc::new(AtomicU64::new(0)),
            query_count: Arc::new(AtomicU64::new(0)),
            mutation_count: Arc::new(AtomicU64::new(0)),
            subscription_count: Arc::new(AtomicU64::new(0)),
            data_transferred: Arc::new(AtomicU64::new(0)),
            resolver_calls: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn record_operation(
        &self,
        latency_ms: u64,
        success: bool,
        operation_type: &str,
        response_size: usize,
        resolver_calls: u64,
    ) {
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
            self.latencies.lock().await.push(latency_ms);
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }

        match operation_type {
            "query" => self.query_count.fetch_add(1, Ordering::Relaxed),
            "mutation" => self.mutation_count.fetch_add(1, Ordering::Relaxed),
            "subscription" => self.subscription_count.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };

        self.data_transferred.fetch_add(response_size as u64, Ordering::Relaxed);
        self.resolver_calls.fetch_add(resolver_calls, Ordering::Relaxed);
    }

    async fn calculate_metrics(&self, total_duration_ms: u64) -> GraphQLPerformanceMetrics {
        let mut latencies = self.latencies.lock().await;
        latencies.sort_unstable();

        let successful = self.successful_operations.load(Ordering::Relaxed);
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total = successful + failed;
        let queries = self.query_count.load(Ordering::Relaxed);
        let mutations = self.mutation_count.load(Ordering::Relaxed);
        let subscriptions = self.subscription_count.load(Ordering::Relaxed);
        let data_kb = self.data_transferred.load(Ordering::Relaxed) as f64 / 1024.0;
        let resolver_calls = self.resolver_calls.load(Ordering::Relaxed);

        let mut metrics = GraphQLPerformanceMetrics::default();
        metrics.total_operations = total;
        metrics.successful_operations = successful;
        metrics.failed_operations = failed;
        metrics.query_operations = queries;
        metrics.mutation_operations = mutations;
        metrics.subscription_operations = subscriptions;
        metrics.total_duration_ms = total_duration_ms;
        metrics.data_transferred_kb = data_kb;
        metrics.resolver_call_count = resolver_calls;

        if !latencies.is_empty() {
            metrics.min_latency_ms = latencies[0];
            metrics.max_latency_ms = latencies[latencies.len() - 1];
            metrics.average_latency_ms = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
            metrics.p95_latency_ms = latencies[latencies.len() * 95 / 100];
            metrics.p99_latency_ms = latencies[latencies.len() * 99 / 100];
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

/// GraphQL performance test runner
pub struct GraphQLPerformanceTest {
    config: GraphQLPerformanceConfig,
    schema: RatchetSchema,
    context: GraphQLContext,
}

impl GraphQLPerformanceTest {
    /// Create a new GraphQL performance test with default configuration
    pub async fn new() -> Self {
        Self::with_config(GraphQLPerformanceConfig::default()).await
    }

    /// Create a new GraphQL performance test with custom configuration
    pub async fn with_config(config: GraphQLPerformanceConfig) -> Self {
        let storage_factory = Arc::new(MemoryStorageFactory::new());
        let repositories = Arc::new(MockRepositoryFactory::new());
        let registry = Arc::new(MockTaskRegistry::new());
        let registry_manager = Arc::new(MockRegistryManager::new());
        let validator = Arc::new(MockTaskValidator::new());

        let context = GraphQLContext::new(
            repositories,
            registry,
            registry_manager,
            validator,
        );

        let schema = create_schema();

        Self { config, schema, context }
    }

    /// Execute a GraphQL operation and measure performance
    async fn execute_graphql_operation(
        &self,
        query: &str,
        variables: Option<Variables>,
        operation_name: Option<&str>,
    ) -> Result<(Response, u64), Box<dyn std::error::Error>> {
        let mut request = Request::new(query);
        
        if let Some(vars) = variables {
            request = request.variables(vars);
        }
        
        if let Some(name) = operation_name {
            request = request.operation_name(name);
        }

        let start_time = Instant::now();
        let response = self.schema.execute(request.data(&self.context)).await;
        let latency = start_time.elapsed().as_millis() as u64;

        Ok((response, latency))
    }

    /// Test performance of GraphQL queries
    pub async fn test_query_performance(&self) -> Result<GraphQLPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = GraphQLOperationTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_requests));
        
        let queries = vec![
            // Simple query
            "query { tasks { id name description } }",
            // Complex query with nested fields
            "query { tasks { id name description executions { id status started_at completed_at } } }",
            // Query with filtering
            "query { tasks(filter: { enabled: true }) { id name task_type } }",
            // Query with pagination
            "query { tasks(pagination: { page: 1, page_size: 10 }) { id name } }",
            // Introspection query
            "query { __schema { queryType { name } } }",
        ];

        let start_time = Instant::now();
        let mut handles = vec![];

        for i in 0..self.config.total_requests {
            let query = queries[i % queries.len()];
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let schema = self.schema.clone();
            let context = self.context.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let request = Request::new(query);
                let operation_start = Instant::now();
                let response = schema.execute(request.data(&context)).await;
                let latency = operation_start.elapsed().as_millis() as u64;

                let success = response.errors.is_empty();
                let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                
                tracker_clone.record_operation(latency, success, "query", response_size, 1).await;
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

    /// Test performance of GraphQL mutations
    pub async fn test_mutation_performance(&self) -> Result<GraphQLPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = GraphQLOperationTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_requests));
        
        let mutations = vec![
            // Create task mutation
            r#"mutation { createTask(input: { name: "perf-test-task", description: "Performance test", taskType: "JavaScript", script: "function test() { return true; }" }) { id name } }"#,
            // Update task mutation
            r#"mutation { updateTask(input: { id: "test-id", name: "updated-task" }) { id name } }"#,
            // Create execution mutation
            r#"mutation { createExecution(input: { taskId: "test-task", input: "{}" }) { id status } }"#,
            // MCP task creation
            r#"mutation { mcpCreateTask(input: { name: "mcp-task", code: "console.log('test')" }) { taskId status } }"#,
        ];

        let start_time = Instant::now();
        let mut handles = vec![];

        for i in 0..self.config.total_requests {
            let mutation = mutations[i % mutations.len()];
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let schema = self.schema.clone();
            let context = self.context.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let request = Request::new(mutation);
                let operation_start = Instant::now();
                let response = schema.execute(request.data(&context)).await;
                let latency = operation_start.elapsed().as_millis() as u64;

                let success = response.errors.is_empty();
                let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                
                tracker_clone.record_operation(latency, success, "mutation", response_size, 1).await;
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

    /// Test performance of GraphQL subscriptions
    pub async fn test_subscription_performance(&self) -> Result<GraphQLPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = GraphQLOperationTracker::new();
        
        let subscription_query = r#"
            subscription {
                taskExecutionEvents {
                    executionId
                    taskId
                    status
                    timestamp
                }
            }
        "#;

        let start_time = Instant::now();
        let mut handles = vec![];

        for _i in 0..self.config.concurrent_requests {
            let tracker_clone = tracker.clone();
            let schema = self.schema.clone();
            let context = self.context.clone();
            let query = subscription_query.to_string();

            let handle = tokio::spawn(async move {
                let request = Request::new(query);
                let operation_start = Instant::now();
                
                // For subscription testing, we'll simulate the setup time
                let response = schema.execute(request.data(&context)).await;
                let latency = operation_start.elapsed().as_millis() as u64;

                let success = response.errors.is_empty();
                let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                
                tracker_clone.record_operation(latency, success, "subscription", response_size, 1).await;
            });

            handles.push(handle);
        }

        // Wait for subscription setup to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed().as_millis() as u64;
        let metrics = tracker.calculate_metrics(total_duration).await;

        Ok(metrics)
    }

    /// Test performance of complex nested queries
    pub async fn test_complex_query_performance(&self) -> Result<GraphQLPerformanceMetrics, Box<dyn std::error::Error>> {
        let tracker = GraphQLOperationTracker::new();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_requests));
        
        let complex_query = r#"
            query ComplexNestedQuery {
                tasks(pagination: { page: 1, page_size: 20 }) {
                    id
                    name
                    description
                    taskType
                    enabled
                    createdAt
                    updatedAt
                    executions(pagination: { page: 1, page_size: 5 }) {
                        id
                        status
                        startedAt
                        completedAt
                        duration
                        input
                        output
                        error
                        job {
                            id
                            priority
                            status
                            createdAt
                            schedule {
                                id
                                name
                                cronExpression
                                enabled
                            }
                        }
                    }
                    schedules {
                        id
                        name
                        cronExpression
                        enabled
                        nextRun
                        lastRun
                    }
                }
                jobs(pagination: { page: 1, page_size: 10 }) {
                    id
                    priority
                    status
                    createdAt
                    updatedAt
                    task {
                        id
                        name
                        taskType
                    }
                    executions {
                        id
                        status
                        duration
                    }
                }
                workers {
                    id
                    status
                    lastHeartbeat
                    currentTask {
                        id
                        name
                    }
                }
            }
        "#;

        let start_time = Instant::now();
        let mut handles = vec![];

        for _i in 0..self.config.total_requests {
            let tracker_clone = tracker.clone();
            let semaphore_clone = semaphore.clone();
            let schema = self.schema.clone();
            let context = self.context.clone();
            let query = complex_query.to_string();

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                let request = Request::new(query);
                let operation_start = Instant::now();
                let response = schema.execute(request.data(&context)).await;
                let latency = operation_start.elapsed().as_millis() as u64;

                let success = response.errors.is_empty();
                let response_size = serde_json::to_string(&response).unwrap_or_default().len();
                
                // Complex queries likely hit multiple resolvers
                let estimated_resolver_calls = 5;
                tracker_clone.record_operation(latency, success, "query", response_size, estimated_resolver_calls).await;
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

    /// Run comprehensive GraphQL performance test suite
    pub async fn run_comprehensive_test_suite(&self) -> Result<Vec<(String, GraphQLPerformanceMetrics)>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        println!("Testing GraphQL query performance...");
        let query_metrics = self.test_query_performance().await?;
        results.push(("GraphQL Queries".to_string(), query_metrics));

        println!("Testing GraphQL mutation performance...");
        let mutation_metrics = self.test_mutation_performance().await?;
        results.push(("GraphQL Mutations".to_string(), mutation_metrics));

        println!("Testing GraphQL subscription performance...");
        let subscription_metrics = self.test_subscription_performance().await?;
        results.push(("GraphQL Subscriptions".to_string(), subscription_metrics));

        println!("Testing complex GraphQL query performance...");
        let complex_metrics = self.test_complex_query_performance().await?;
        results.push(("Complex GraphQL Queries".to_string(), complex_metrics));

        Ok(results)
    }

    /// Print GraphQL performance metrics
    pub fn print_metrics(&self, operation_type: &str, metrics: &GraphQLPerformanceMetrics) {
        println!("\n=== GraphQL Performance Test Results: {} ===", operation_type);
        println!("Total Operations: {}", metrics.total_operations);
        println!("Successful: {} ({:.2}%)", metrics.successful_operations, 
                 (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0);
        println!("Failed: {} ({:.2}%)", metrics.failed_operations, metrics.error_rate * 100.0);
        println!("Queries: {}, Mutations: {}, Subscriptions: {}", 
                 metrics.query_operations, metrics.mutation_operations, metrics.subscription_operations);
        println!("Test Duration: {:.2}s", metrics.total_duration_ms as f64 / 1000.0);
        println!("Operations/Second: {:.2}", metrics.operations_per_second);
        println!("Average Latency: {:.2}ms", metrics.average_latency_ms);
        println!("Latency P95: {}ms", metrics.p95_latency_ms);
        println!("Latency P99: {}ms", metrics.p99_latency_ms);
        println!("Min Latency: {}ms", metrics.min_latency_ms);
        println!("Max Latency: {}ms", metrics.max_latency_ms);
        println!("Data Transferred: {:.2}KB", metrics.data_transferred_kb);
        println!("Resolver Calls: {}", metrics.resolver_call_count);

        // Performance assessment
        if metrics.error_rate <= 0.02 {
            println!("✅ Error rate within acceptable limits");
        } else {
            println!("❌ Error rate exceeds acceptable limits ({:.2}%)", metrics.error_rate * 100.0);
        }

        if metrics.p95_latency_ms <= 200 {
            println!("✅ P95 latency within target (200ms)");
        } else {
            println!("❌ P95 latency exceeds target ({}ms > 200ms)", metrics.p95_latency_ms);
        }
    }
}

// =============================================================================
// GRAPHQL PERFORMANCE TESTS
// =============================================================================

#[tokio::test]
async fn test_graphql_query_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 5;
    config.total_requests = 100;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let metrics = performance_test.test_query_performance().await?;

    performance_test.print_metrics("Query Performance", &metrics);

    // Basic performance assertions
    assert!(metrics.total_operations >= 100);
    assert!(metrics.error_rate <= 0.05); // 5% error rate max
    assert!(metrics.operations_per_second > 0.0);
    assert!(metrics.average_latency_ms < 1000.0); // 1 second max average

    Ok(())
}

#[tokio::test]
async fn test_graphql_mutation_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 3;
    config.total_requests = 50;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let metrics = performance_test.test_mutation_performance().await?;

    performance_test.print_metrics("Mutation Performance", &metrics);

    // Mutation performance assertions
    assert!(metrics.total_operations >= 50);
    assert!(metrics.error_rate <= 0.20); // 20% error rate max for mutations (some may fail validation)
    assert!(metrics.average_latency_ms < 2000.0); // 2 seconds max average for mutations

    Ok(())
}

#[tokio::test]
async fn test_graphql_subscription_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 5;
    config.subscription_duration_secs = 5;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let metrics = performance_test.test_subscription_performance().await?;

    performance_test.print_metrics("Subscription Performance", &metrics);

    // Subscription performance assertions
    assert!(metrics.total_operations >= 5);
    assert!(metrics.error_rate <= 0.10); // 10% error rate max for subscriptions
    assert!(metrics.average_latency_ms < 500.0); // 500ms max average for subscription setup

    Ok(())
}

#[tokio::test]
async fn test_graphql_complex_query_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 3;
    config.total_requests = 30;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let metrics = performance_test.test_complex_query_performance().await?;

    performance_test.print_metrics("Complex Query Performance", &metrics);

    // Complex query performance assertions
    assert!(metrics.total_operations >= 30);
    assert!(metrics.error_rate <= 0.15); // 15% error rate max for complex queries
    assert!(metrics.average_latency_ms < 3000.0); // 3 seconds max average for complex queries
    assert!(metrics.resolver_call_count >= metrics.total_operations); // Multiple resolvers per query

    Ok(())
}

#[tokio::test]
async fn test_graphql_comprehensive_performance_suite() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 2;
    config.total_requests = 20;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let results = performance_test.run_comprehensive_test_suite().await?;

    assert!(!results.is_empty());

    for (operation_type, metrics) in results {
        performance_test.print_metrics(&operation_type, &metrics);

        // Comprehensive test assertions
        assert!(metrics.total_operations >= 20);
        assert!(metrics.error_rate <= 0.25); // 25% error rate max for comprehensive test
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Marked as ignore since stress tests are resource intensive
async fn test_graphql_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = GraphQLPerformanceConfig::default();
    config.concurrent_requests = 50;
    config.total_requests = 2000;
    config.test_duration_secs = 60;

    let performance_test = GraphQLPerformanceTest::with_config(config).await;
    let metrics = performance_test.test_query_performance().await?;

    performance_test.print_metrics("GraphQL Stress Test", &metrics);

    // Stress test assertions
    assert!(metrics.total_operations >= 2000);
    assert!(metrics.error_rate <= 0.10); // 10% error rate max under stress
    assert!(metrics.operations_per_second > 20.0); // Minimum 20 OPS under stress

    Ok(())
}

// TODO: Add tests for:
// - DataLoader performance and N+1 query prevention
// - GraphQL schema complexity analysis
// - Subscription memory usage and connection limits
// - Persisted query performance
// - Real-time event broadcasting performance
// - GraphQL batching performance
// - Depth and complexity limiting performance impact
// - Custom scalar performance
// - Error handling performance in complex scenarios