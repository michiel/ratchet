//! Batch request processing for MCP server

use crate::error::McpError;
use crate::protocol::{
    BatchExecutionMode, BatchItemResult, BatchParams, BatchProgressNotification, BatchRequest,
    BatchResult, BatchStats, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, span, warn, Instrument, Level};
use uuid::Uuid;

/// Type for batch request handler function
pub type BatchRequestHandler = dyn Fn(JsonRpcRequest) -> Pin<Box<dyn Future<Output = JsonRpcResponse> + Send>>
    + Send
    + Sync;

/// Type for progress notification callback
pub type ProgressCallback = dyn Fn(BatchProgressNotification) -> Pin<Box<dyn Future<Output = ()> + Send>>
    + Send
    + Sync;

/// Batch processor for handling batch operations
pub struct BatchProcessor {
    /// Maximum batch size allowed
    max_batch_size: u32,
    /// Maximum parallel executions
    max_parallel: u32,
    /// Default timeout for batch operations
    default_timeout: Duration,
    /// Request handler for individual requests
    request_handler: Arc<BatchRequestHandler>,
    /// Progress notification callback
    progress_callback: Option<Arc<ProgressCallback>>,
    /// Enable request deduplication
    enable_deduplication: bool,
    /// Enable result caching
    enable_caching: bool,
}

/// Execution context for a batch item
#[derive(Debug)]
struct BatchItemContext {
    id: String,
    request: JsonRpcRequest,
    dependencies: Vec<String>,
    timeout: Option<Duration>,
    priority: i32,
    start_time: Option<Instant>,
    completed: bool,
    result: Option<BatchItemResult>,
}

/// Dependency graph for batch execution
#[derive(Debug)]
struct DependencyGraph {
    nodes: HashMap<String, BatchItemContext>,
    edges: HashMap<String, Vec<String>>, // dependency -> dependents
    ready_queue: VecDeque<String>,       // items ready to execute
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(
        max_batch_size: u32,
        max_parallel: u32,
        default_timeout: Duration,
        request_handler: Arc<BatchRequestHandler>,
        progress_callback: Option<Arc<ProgressCallback>>,
    ) -> Self {
        Self {
            max_batch_size,
            max_parallel,
            default_timeout,
            request_handler,
            progress_callback,
            enable_deduplication: false,
            enable_caching: false,
        }
    }

    /// Create a new batch processor with optimizations enabled
    pub fn new_optimized(
        max_batch_size: u32,
        max_parallel: u32,
        default_timeout: Duration,
        request_handler: Arc<BatchRequestHandler>,
        progress_callback: Option<Arc<ProgressCallback>>,
        enable_deduplication: bool,
        enable_caching: bool,
    ) -> Self {
        Self {
            max_batch_size,
            max_parallel,
            default_timeout,
            request_handler,
            progress_callback,
            enable_deduplication,
            enable_caching,
        }
    }

    /// Process a batch request
    pub async fn process_batch(&self, params: BatchParams) -> Result<BatchResult, McpError> {
        self.process_batch_with_handler(params, &self.request_handler).await
    }
    
    /// Process a batch request with a custom handler
    pub async fn process_batch_with_handler(
        &self, 
        params: BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<BatchResult, McpError> {
        let start_time = Instant::now();
        let correlation_token = params.correlation_token.clone();
        
        // Validate batch size
        if params.requests.len() > self.max_batch_size as usize {
            return Err(McpError::Validation {
                field: "batch_size".to_string(),
                message: format!(
                    "Batch size {} exceeds maximum allowed size {}",
                    params.requests.len(),
                    self.max_batch_size
                ),
            });
        }

        if params.requests.is_empty() {
            return Err(McpError::Validation {
                field: "batch_requests".to_string(),
                message: "Batch cannot be empty".to_string(),
            });
        }

        let span = span!(Level::INFO, "batch_processing", 
            batch_size = params.requests.len(),
            execution_mode = ?params.execution_mode,
            correlation_token = ?correlation_token
        );

        async move {
            info!("Starting batch processing");
            
            // Validate and build dependency graph
            let mut requests = params.requests.clone();
            
            // Apply deduplication if enabled
            if self.enable_deduplication {
                requests = self.deduplicate_requests(requests);
                info!("Deduplication reduced batch size from {} to {}", 
                    params.requests.len(), requests.len());
            }
            
            let mut graph = self.build_dependency_graph(requests).await?;
            
            // Apply timeout
            let batch_timeout = params.timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(self.default_timeout);

            let results = match timeout(batch_timeout, self.execute_batch(&mut graph, &params, handler)).await {
                Ok(results) => results?,
                Err(_) => {
                    error!("Batch execution timed out after {:?}", batch_timeout);
                    return Err(McpError::ServerTimeout { timeout: batch_timeout });
                }
            };

            let total_time = start_time.elapsed();
            let stats = self.calculate_stats(&results, total_time);

            info!(
                "Batch processing completed in {:?}, success: {}, failed: {}, skipped: {}",
                total_time, stats.successful_requests, stats.failed_requests, stats.skipped_requests
            );

            Ok(BatchResult {
                results,
                stats,
                correlation_token,
                metadata: HashMap::new(),
            })
        }.instrument(span).await
    }

    /// Deduplicate requests based on method and params
    fn deduplicate_requests(&self, requests: Vec<BatchRequest>) -> Vec<BatchRequest> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut seen_hashes: HashMap<u64, String> = HashMap::new();
        let mut deduped_requests = Vec::new();
        let mut duplicate_mapping: HashMap<String, String> = HashMap::new(); // original_id -> canonical_id

        for request in requests {
            // Create a hash of method and params for deduplication
            let mut hasher = DefaultHasher::new();
            request.method.hash(&mut hasher);
            if let Some(params) = &request.params {
                params.to_string().hash(&mut hasher);
            }
            let request_hash = hasher.finish();

            if let Some(canonical_id) = seen_hashes.get(&request_hash) {
                // This is a duplicate - map it to the canonical request
                duplicate_mapping.insert(request.id.clone(), canonical_id.clone());
                debug!("Deduplicated request {} -> {}", request.id, canonical_id);
            } else {
                // First time seeing this request - keep it
                seen_hashes.insert(request_hash, request.id.clone());
                deduped_requests.push(request);
            }
        }

        // Update dependencies to point to canonical IDs
        for request in &mut deduped_requests {
            for dep in &mut request.dependencies {
                if let Some(canonical_dep) = duplicate_mapping.get(dep) {
                    *dep = canonical_dep.clone();
                }
            }
        }

        deduped_requests
    }

    /// Build dependency graph from batch requests
    async fn build_dependency_graph(
        &self,
        requests: Vec<BatchRequest>,
    ) -> Result<DependencyGraph, McpError> {
        let mut nodes = HashMap::new();
        let mut edges: HashMap<String, Vec<String>> = HashMap::new();
        let mut id_set = HashSet::new();

        // First pass: collect all IDs and validate uniqueness
        for request in &requests {
            if id_set.contains(&request.id) {
                return Err(McpError::Validation {
                    field: "request_id".to_string(),
                    message: format!("Duplicate request ID: {}", request.id),
                });
            }
            id_set.insert(request.id.clone());
        }

        // Second pass: build nodes and validate dependencies
        for request in requests {
            // Validate dependencies exist
            for dep in &request.dependencies {
                if !id_set.contains(dep) {
                    return Err(McpError::Validation {
                        field: "dependencies".to_string(),
                        message: format!(
                            "Request {} depends on non-existent request {}",
                            request.id, dep
                        ),
                    });
                }
            }

            // Create JSON-RPC request
            let jsonrpc_request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: request.method.clone(),
                params: request.params.clone(),
                id: Some(Value::String(Uuid::new_v4().to_string())),
            };

            // Create context
            let context = BatchItemContext {
                id: request.id.clone(),
                request: jsonrpc_request,
                dependencies: request.dependencies.clone(),
                timeout: request.timeout_ms.map(Duration::from_millis),
                priority: request.priority,
                start_time: None,
                completed: false,
                result: None,
            };

            nodes.insert(request.id.clone(), context);

            // Build reverse dependency edges
            for dep in &request.dependencies {
                edges.entry(dep.clone())
                    .or_default()
                    .push(request.id.clone());
            }
        }

        // Check for circular dependencies
        self.detect_circular_dependencies(&nodes, &edges)?;

        // Build initial ready queue (items with no dependencies)
        let ready_queue: VecDeque<String> = nodes
            .iter()
            .filter(|(_, context)| context.dependencies.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        Ok(DependencyGraph {
            nodes,
            edges,
            ready_queue,
        })
    }

    /// Detect circular dependencies using DFS
    fn detect_circular_dependencies(
        &self,
        nodes: &HashMap<String, BatchItemContext>,
        edges: &HashMap<String, Vec<String>>,
    ) -> Result<(), McpError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in nodes.keys() {
            if !visited.contains(node_id) && self.has_cycle_dfs(node_id, nodes, edges, &mut visited, &mut rec_stack) {
                return Err(McpError::Validation {
                    field: "dependencies".to_string(),
                    message: "Circular dependency detected in batch requests".to_string(),
                });
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node_id: &str,
        nodes: &HashMap<String, BatchItemContext>,
        edges: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        if let Some(context) = nodes.get(node_id) {
            for dep in &context.dependencies {
                if !visited.contains(dep) {
                    if self.has_cycle_dfs(dep, nodes, edges, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(node_id);
        false
    }

    /// Execute batch based on execution mode
    async fn execute_batch(
        &self,
        graph: &mut DependencyGraph,
        params: &BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<Vec<BatchItemResult>, McpError> {
        match params.execution_mode {
            BatchExecutionMode::Parallel => self.execute_parallel(graph, params, handler).await,
            BatchExecutionMode::Sequential => self.execute_sequential(graph, params, handler).await,
            BatchExecutionMode::Dependency => self.execute_dependency_based(graph, params, handler).await,
            BatchExecutionMode::PriorityDependency => {
                self.execute_priority_dependency_based(graph, params, handler).await
            }
        }
    }

    /// Execute requests in parallel
    async fn execute_parallel(
        &self,
        graph: &mut DependencyGraph,
        params: &BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<Vec<BatchItemResult>, McpError> {
        let max_parallel = params.max_parallel.unwrap_or(self.max_parallel).min(self.max_parallel);
        let semaphore = Arc::new(Semaphore::new(max_parallel as usize));
        let results = Arc::new(RwLock::new(HashMap::new()));
        let mut handles = Vec::new();

        let total_requests = graph.nodes.len();
        let completed_count = Arc::new(RwLock::new(0));

        for (id, context) in graph.nodes.iter_mut() {
            let id = id.clone();
            let request = context.request.clone();
            let timeout_duration = context.timeout.unwrap_or(self.default_timeout);
            let handler_clone = handler.clone();
            let semaphore = semaphore.clone();
            let results = results.clone();
            let completed_count = completed_count.clone();
            let progress_callback = self.progress_callback.clone();
            let correlation_token = params.correlation_token.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let start_time = Instant::now();

                debug!("Starting execution of request: {}", id);

                let response = match timeout(timeout_duration, handler_clone(request)).await {
                    Ok(response) => response,
                    Err(_) => {
                        warn!("Request {} timed out after {:?}", id, timeout_duration);
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError::internal_error("Request timeout")),
                            id: Some(Value::String(id.clone())),
                        }
                    }
                };

                let execution_time = start_time.elapsed();
                let result = BatchItemResult {
                    id: id.clone(),
                    result: response.result,
                    error: response.error,
                    execution_time_ms: execution_time.as_millis() as u64,
                    skipped: false,
                    metadata: HashMap::new(),
                };

                results.write().await.insert(id.clone(), result);
                
                // Update progress
                let completed = {
                    let mut count = completed_count.write().await;
                    *count += 1;
                    *count
                };

                if let (Some(callback), Some(token)) = (&progress_callback, &correlation_token) {
                    let notification = BatchProgressNotification {
                        correlation_token: token.clone(),
                        completed_requests: completed as u32,
                        total_requests: total_requests as u32,
                        executing_requests: vec![], // Could be enhanced to track this
                        timestamp: Utc::now().to_rfc3339(),
                        data: None,
                    };
                    callback(notification).await;
                }

                debug!("Completed execution of request: {} in {:?}", id, execution_time);
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task execution failed: {}", e);
            }
        }

        // Collect results in original order
        let results_map = results.read().await;
        let mut ordered_results = Vec::new();
        for id in graph.nodes.keys() {
            if let Some(result) = results_map.get(id) {
                ordered_results.push(result.clone());
            }
        }

        Ok(ordered_results)
    }

    /// Execute requests sequentially
    async fn execute_sequential(
        &self,
        graph: &mut DependencyGraph,
        params: &BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<Vec<BatchItemResult>, McpError> {
        let mut results = Vec::new();
        let total_requests = graph.nodes.len();

        for (index, (id, context)) in graph.nodes.iter_mut().enumerate() {
            let start_time = Instant::now();
            let timeout_duration = context.timeout.unwrap_or(self.default_timeout);

            debug!("Starting sequential execution of request: {}", id);

            let response = match timeout(timeout_duration, handler(context.request.clone())).await {
                Ok(response) => response,
                Err(_) => {
                    warn!("Request {} timed out after {:?}", id, timeout_duration);
                    if params.stop_on_error {
                        return Err(McpError::ServerTimeout { timeout: timeout_duration });
                    }
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError::internal_error("Request timeout")),
                        id: Some(Value::String(id.clone())),
                    }
                }
            };

            let execution_time = start_time.elapsed();
            let result = BatchItemResult {
                id: id.clone(),
                result: response.result.clone(),
                error: response.error.clone(),
                execution_time_ms: execution_time.as_millis() as u64,
                skipped: false,
                metadata: HashMap::new(),
            };

            results.push(result);

            // Check for error and stop if required
            if params.stop_on_error && response.error.is_some() {
                warn!("Request {} failed, stopping batch execution", id);
                // Mark remaining requests as skipped
                for (remaining_id, _) in graph.nodes.iter().skip(index + 1) {
                    results.push(BatchItemResult {
                        id: remaining_id.clone(),
                        result: None,
                        error: Some(JsonRpcError::internal_error("Skipped due to previous error")),
                        execution_time_ms: 0,
                        skipped: true,
                        metadata: HashMap::new(),
                    });
                }
                break;
            }

            // Send progress notification
            if let (Some(callback), Some(token)) = (&self.progress_callback, &params.correlation_token) {
                let notification = BatchProgressNotification {
                    correlation_token: token.clone(),
                    completed_requests: (index + 1) as u32,
                    total_requests: total_requests as u32,
                    executing_requests: vec![],
                    timestamp: Utc::now().to_rfc3339(),
                    data: None,
                };
                callback(notification).await;
            }

            debug!("Completed sequential execution of request: {} in {:?}", id, execution_time);
        }

        Ok(results)
    }

    /// Execute based on dependency graph
    async fn execute_dependency_based(
        &self,
        graph: &mut DependencyGraph,
        params: &BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<Vec<BatchItemResult>, McpError> {
        let max_parallel = params.max_parallel.unwrap_or(self.max_parallel).min(self.max_parallel);
        let semaphore = Arc::new(Semaphore::new(max_parallel as usize));
        let results = Arc::new(RwLock::new(HashMap::new()));
        let completed = Arc::new(RwLock::new(HashSet::new()));
        let mut executing = HashSet::new();
        let mut handles = Vec::new();

        while !graph.ready_queue.is_empty() || !executing.is_empty() {
            // Start as many ready requests as possible
            while !graph.ready_queue.is_empty() && executing.len() < max_parallel as usize {
                if let Some(id) = graph.ready_queue.pop_front() {
                    if let Some(context) = graph.nodes.get(&id) {
                        executing.insert(id.clone());
                        
                        let request = context.request.clone();
                        let timeout_duration = context.timeout.unwrap_or(self.default_timeout);
                        let handler_clone = handler.clone();
                        let semaphore = semaphore.clone();
                        let results = results.clone();
                        let completed = completed.clone();
                        let id_clone = id.clone();

                        let handle = tokio::spawn(async move {
                            let _permit = semaphore.acquire().await.unwrap();
                            let start_time = Instant::now();

                            debug!("Starting dependency-based execution of request: {}", id_clone);

                            let response = match timeout(timeout_duration, handler_clone(request)).await {
                                Ok(response) => response,
                                Err(_) => {
                                    warn!("Request {} timed out after {:?}", id_clone, timeout_duration);
                                    JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        result: None,
                                        error: Some(JsonRpcError::internal_error("Request timeout")),
                                        id: Some(Value::String(id_clone.clone())),
                                    }
                                }
                            };

                            let execution_time = start_time.elapsed();
                            let result = BatchItemResult {
                                id: id_clone.clone(),
                                result: response.result,
                                error: response.error,
                                execution_time_ms: execution_time.as_millis() as u64,
                                skipped: false,
                                metadata: HashMap::new(),
                            };

                            results.write().await.insert(id_clone.clone(), result);
                            completed.write().await.insert(id_clone.clone());

                            debug!("Completed dependency-based execution of request: {} in {:?}", id_clone, execution_time);
                            id_clone
                        });

                        handles.push(handle);
                    }
                }
            }

            // Wait for at least one to complete
            if !handles.is_empty() {
                let (completed_id, _remaining_index, remaining_handles) = {
                    // Use a simpler approach with tokio::select!
                    let mut completed_id = None;
                    let mut remaining_handles = Vec::new();
                    
                    // For simplicity, await the first handle
                    if let Some(handle) = handles.pop() {
                        completed_id = Some(handle.await);
                        remaining_handles = handles;
                    }
                    
                    (completed_id.unwrap(), 0, remaining_handles)
                };
                handles = remaining_handles;

                match completed_id {
                    Ok(id) => {
                        executing.remove(&id);
                        
                        // Add dependents to ready queue if all their dependencies are complete
                        if let Some(dependents) = graph.edges.get(&id) {
                            let completed_set = completed.read().await;
                            for dependent in dependents {
                                if let Some(dependent_context) = graph.nodes.get(dependent) {
                                    let all_deps_complete = dependent_context.dependencies
                                        .iter()
                                        .all(|dep| completed_set.contains(dep));
                                    
                                    if all_deps_complete && !graph.ready_queue.contains(dependent) {
                                        graph.ready_queue.push_back(dependent.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Task execution failed: {}", e);
                    }
                }
            }
        }

        // Wait for any remaining handles
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task execution failed: {}", e);
            }
        }

        // Collect results in original order
        let results_map = results.read().await;
        let mut ordered_results = Vec::new();
        for id in graph.nodes.keys() {
            if let Some(result) = results_map.get(id) {
                ordered_results.push(result.clone());
            }
        }

        Ok(ordered_results)
    }

    /// Execute based on priority and dependencies
    async fn execute_priority_dependency_based(
        &self,
        graph: &mut DependencyGraph,
        params: &BatchParams,
        handler: &Arc<BatchRequestHandler>,
    ) -> Result<Vec<BatchItemResult>, McpError> {
        // Sort ready queue by priority (higher priority first)
        let mut priority_queue: Vec<String> = graph.ready_queue.drain(..).collect();
        priority_queue.sort_by(|a, b| {
            let priority_a = graph.nodes.get(a).map(|c| c.priority).unwrap_or(0);
            let priority_b = graph.nodes.get(b).map(|c| c.priority).unwrap_or(0);
            priority_b.cmp(&priority_a) // Reverse for descending order
        });
        graph.ready_queue = priority_queue.into();

        // Use dependency-based execution with prioritized queue
        self.execute_dependency_based(graph, params, handler).await
    }

    /// Calculate batch execution statistics
    fn calculate_stats(&self, results: &[BatchItemResult], total_time: Duration) -> BatchStats {
        let total_requests = results.len() as u32;
        let successful_requests = results.iter().filter(|r| r.error.is_none() && !r.skipped).count() as u32;
        let failed_requests = results.iter().filter(|r| r.error.is_some() && !r.skipped).count() as u32;
        let skipped_requests = results.iter().filter(|r| r.skipped).count() as u32;

        let total_execution_time_ms = total_time.as_millis() as u64;
        let average_execution_time_ms = if total_requests > 0 {
            results.iter()
                .filter(|r| !r.skipped)
                .map(|r| r.execution_time_ms as f64)
                .sum::<f64>() / (total_requests - skipped_requests) as f64
        } else {
            0.0
        };

        // This is simplified - in a real implementation, you'd track max parallel during execution
        let max_parallel_executed = self.max_parallel.min(total_requests);

        BatchStats {
            total_requests,
            successful_requests,
            failed_requests,
            skipped_requests,
            total_execution_time_ms,
            average_execution_time_ms,
            max_parallel_executed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::BatchExecutionMode;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn create_test_processor() -> BatchProcessor {
        let counter = Arc::new(AtomicU32::new(0));
        let handler = Arc::new(move |request: JsonRpcRequest| {
            let counter = counter.clone();
            Box::pin(async move {
                // Simulate some work
                tokio::time::sleep(Duration::from_millis(10)).await;
                let count = counter.fetch_add(1, Ordering::SeqCst);
                
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({"count": count, "method": request.method})),
                    error: None,
                    id: request.id,
                }
            }) as Pin<Box<dyn Future<Output = JsonRpcResponse> + Send>>
        });

        BatchProcessor::new(
            100,
            10,
            Duration::from_secs(30),
            handler,
            None,
        )
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let processor = create_test_processor();
        
        let params = BatchParams {
            requests: vec![
                BatchRequest {
                    id: "req1".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 1})),
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req2".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 2})),
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
            ],
            execution_mode: BatchExecutionMode::Parallel,
            max_parallel: Some(2),
            timeout_ms: None,
            stop_on_error: false,
            correlation_token: Some("test-token".to_string()),
            metadata: HashMap::new(),
        };

        let result = processor.process_batch(params).await.unwrap();
        
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.stats.total_requests, 2);
        assert_eq!(result.stats.successful_requests, 2);
        assert_eq!(result.stats.failed_requests, 0);
        assert_eq!(result.correlation_token, Some("test-token".to_string()));
    }

    #[tokio::test]
    async fn test_dependency_execution() {
        let processor = create_test_processor();
        
        let params = BatchParams {
            requests: vec![
                BatchRequest {
                    id: "req1".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 1})),
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req2".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 2})),
                    dependencies: vec!["req1".to_string()],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
            ],
            execution_mode: BatchExecutionMode::Dependency,
            max_parallel: Some(2),
            timeout_ms: None,
            stop_on_error: false,
            correlation_token: None,
            metadata: HashMap::new(),
        };

        let result = processor.process_batch(params).await.unwrap();
        
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.stats.successful_requests, 2);
        
        // Verify execution order - req1 should complete before req2
        let req1_result = result.results.iter().find(|r| r.id == "req1").unwrap();
        let req2_result = result.results.iter().find(|r| r.id == "req2").unwrap();
        
        if let (Some(req1_count), Some(req2_count)) = (
            req1_result.result.as_ref().and_then(|r| r.get("count")),
            req2_result.result.as_ref().and_then(|r| r.get("count"))
        ) {
            assert!(req1_count.as_u64().unwrap() < req2_count.as_u64().unwrap());
        }
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let processor = create_test_processor();
        
        let params = BatchParams {
            requests: vec![
                BatchRequest {
                    id: "req1".to_string(),
                    method: "test_method".to_string(),
                    params: None,
                    dependencies: vec!["req2".to_string()],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req2".to_string(),
                    method: "test_method".to_string(),
                    params: None,
                    dependencies: vec!["req1".to_string()],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
            ],
            execution_mode: BatchExecutionMode::Dependency,
            max_parallel: Some(2),
            timeout_ms: None,
            stop_on_error: false,
            correlation_token: None,
            metadata: HashMap::new(),
        };

        let result = processor.process_batch(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[tokio::test]
    async fn test_batch_size_validation() {
        let processor = BatchProcessor::new(
            2, // max_batch_size = 2
            10,
            Duration::from_secs(30),
            Arc::new(|_| Box::pin(async { JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({})),
                error: None,
                id: None,
            } })),
            None,
        );
        
        let params = BatchParams {
            requests: vec![
                BatchRequest {
                    id: "req1".to_string(),
                    method: "test".to_string(),
                    params: None,
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req2".to_string(),
                    method: "test".to_string(),
                    params: None,
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req3".to_string(),
                    method: "test".to_string(),
                    params: None,
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
            ],
            execution_mode: BatchExecutionMode::Parallel,
            max_parallel: None,
            timeout_ms: None,
            stop_on_error: false,
            correlation_token: None,
            metadata: HashMap::new(),
        };

        let result = processor.process_batch(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum allowed size"));
    }

    #[tokio::test]
    async fn test_request_deduplication() {
        let processor = BatchProcessor::new_optimized(
            100,
            10,
            Duration::from_secs(30),
            Arc::new(|_| Box::pin(async { JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({"executed": true})),
                error: None,
                id: None,
            } })),
            None,
            true, // enable_deduplication
            false,
        );
        
        let params = BatchParams {
            requests: vec![
                BatchRequest {
                    id: "req1".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 42})),
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req2".to_string(),
                    method: "test_method".to_string(), // Same method
                    params: Some(json!({"value": 42})), // Same params  
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
                BatchRequest {
                    id: "req3".to_string(),
                    method: "test_method".to_string(),
                    params: Some(json!({"value": 24})), // Different params
                    dependencies: vec![],
                    timeout_ms: None,
                    priority: 0,
                    metadata: HashMap::new(),
                },
            ],
            execution_mode: BatchExecutionMode::Parallel,
            max_parallel: None,
            timeout_ms: None,
            stop_on_error: false,
            correlation_token: Some("test-dedup".to_string()),
            metadata: HashMap::new(),
        };

        let result = processor.process_batch(params).await.unwrap();
        
        // Should have 2 unique results (req1 and req3, req2 was deduplicated)
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.correlation_token, Some("test-dedup".to_string()));
        assert_eq!(result.stats.total_requests, 2);
    }
}