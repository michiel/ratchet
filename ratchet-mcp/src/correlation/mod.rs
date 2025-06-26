//! Request correlation and distributed tracing for MCP operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Configuration for request correlation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    /// Maximum number of active requests to track
    pub max_active_requests: usize,
    
    /// Maximum depth for correlation chains
    pub max_correlation_depth: usize,
    
    /// Whether to include correlation data in responses
    pub include_correlation_headers: bool,
    
    /// Timeout for cleaning up abandoned requests
    pub cleanup_timeout: Duration,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            max_active_requests: 10000,
            max_correlation_depth: 50,
            include_correlation_headers: true,
            cleanup_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Context for tracking request correlation and lifecycle
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique request identifier
    pub request_id: String,
    
    /// Parent request ID if this is a child request
    pub parent_id: Option<String>,
    
    /// Complete correlation chain from root to this request
    pub correlation_chain: Vec<String>,
    
    /// Request start time
    pub start_time: Instant,
    
    /// Client identifier
    pub client_id: String,
    
    /// Method being executed
    pub method: String,
    
    /// Additional context data
    pub metadata: HashMap<String, String>,
}

impl RequestContext {
    /// Create a new root request context
    pub fn new_root(client_id: String, method: String) -> Self {
        let request_id = format!("req-{}", Uuid::new_v4());
        
        Self {
            request_id: request_id.clone(),
            parent_id: None,
            correlation_chain: vec![request_id.clone()],
            start_time: Instant::now(),
            client_id,
            method,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a new child request context
    pub fn new_child(parent: &RequestContext, method: &str) -> Self {
        let request_id = format!("req-{}", Uuid::new_v4());
        let mut correlation_chain = parent.correlation_chain.clone();
        correlation_chain.push(request_id.clone());
        
        Self {
            request_id: request_id.clone(),
            parent_id: Some(parent.request_id.clone()),
            correlation_chain,
            start_time: Instant::now(),
            client_id: parent.client_id.clone(),
            method: method.to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to the request context
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Get the root request ID from the correlation chain
    pub fn root_request_id(&self) -> Option<&String> {
        self.correlation_chain.first()
    }
    
    /// Get correlation depth
    pub fn correlation_depth(&self) -> usize {
        self.correlation_chain.len()
    }
    
    /// Get elapsed time since request start
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Metrics collected for a completed request
#[derive(Debug, Clone, Serialize)]
pub struct RequestMetrics {
    /// Request identifier
    pub request_id: String,
    
    /// Parent request ID if applicable
    pub parent_id: Option<String>,
    
    /// Complete correlation chain
    pub correlation_chain: Vec<String>,
    
    /// Request duration
    pub duration: Duration,
    
    /// Client identifier
    pub client_id: String,
    
    /// Method executed
    pub method: String,
    
    /// Whether the request succeeded
    pub success: bool,
    
    /// Error code if failed
    pub error_code: Option<String>,
    
    /// Request metadata
    pub metadata: HashMap<String, String>,
    
    /// Completion timestamp
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Manager for request correlation and distributed tracing
pub struct CorrelationManager {
    /// Currently active request contexts
    active_requests: Arc<Mutex<HashMap<String, RequestContext>>>,
    
    /// Configuration
    config: CorrelationConfig,
    
    /// Metrics for completed requests (for analysis)
    completed_metrics: Arc<Mutex<Vec<RequestMetrics>>>,
}

impl CorrelationManager {
    /// Create a new correlation manager
    pub fn new(config: CorrelationConfig) -> Self {
        let manager = Self {
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            config,
            completed_metrics: Arc::new(Mutex::new(Vec::new())),
        };
        
        // Start cleanup task
        manager.start_cleanup_task();
        
        manager
    }
    
    /// Start a new root request
    pub async fn start_request(&self, client_id: String, method: String) -> String {
        let context = RequestContext::new_root(client_id.clone(), method.clone());
        let request_id = context.request_id.clone();
        
        let mut active = self.active_requests.lock().await;
        
        // Check if we're at capacity
        if active.len() >= self.config.max_active_requests {
            tracing::warn!(
                "Maximum active requests ({}) reached, may need to increase capacity",
                self.config.max_active_requests
            );
        }
        
        active.insert(request_id.clone(), context);
        
        tracing::debug!(
            request_id = %request_id,
            client_id = %client_id,
            method = %method,
            "Started new request"
        );
        
        request_id
    }
    
    /// Create a child request from an existing request
    pub async fn create_child_request(&self, parent_id: String, method: String) -> Result<String, String> {
        let mut active = self.active_requests.lock().await;
        
        // Get parent info first
        let (parent_depth, parent_client_id) = {
            let parent = active.get(&parent_id)
                .ok_or_else(|| format!("Parent request {} not found", parent_id))?;
            
            // Check correlation depth
            if parent.correlation_depth() >= self.config.max_correlation_depth {
                return Err(format!(
                    "Maximum correlation depth ({}) exceeded",
                    self.config.max_correlation_depth
                ));
            }
            
            (parent.correlation_depth(), parent.client_id.clone())
        };
        
        // Create child context using a cloned parent (to avoid borrowing issues)
        let parent_context = active.get(&parent_id).unwrap().clone();
        let context = RequestContext::new_child(&parent_context, &method);
        let request_id = context.request_id.clone();
        
        active.insert(request_id.clone(), context);
        
        tracing::debug!(
            request_id = %request_id,
            parent_id = %parent_id,
            method = %method,
            depth = parent_depth + 1,
            "Created child request"
        );
        
        Ok(request_id)
    }
    
    /// Complete a request and return its metrics
    pub async fn complete_request(&self, request_id: String, success: bool, error_code: Option<String>) -> Option<RequestMetrics> {
        let mut active = self.active_requests.lock().await;
        
        if let Some(context) = active.remove(&request_id) {
            let metrics = RequestMetrics {
                request_id: context.request_id.clone(),
                parent_id: context.parent_id.clone(),
                correlation_chain: context.correlation_chain.clone(),
                duration: context.elapsed_time(),
                client_id: context.client_id.clone(),
                method: context.method.clone(),
                success,
                error_code,
                metadata: context.metadata.clone(),
                completed_at: chrono::Utc::now(),
            };
            
            tracing::debug!(
                request_id = %request_id,
                duration_ms = metrics.duration.as_millis(),
                success = success,
                "Completed request"
            );
            
            // Store metrics for analysis
            let mut completed = self.completed_metrics.lock().await;
            completed.push(metrics.clone());
            
            // Keep only recent metrics (last 1000)
            if completed.len() > 1000 {
                let len = completed.len();
                completed.drain(0..(len - 1000));
            }
            
            Some(metrics)
        } else {
            tracing::warn!(
                request_id = %request_id,
                "Attempted to complete unknown request"
            );
            None
        }
    }
    
    /// Get request context by ID
    pub async fn get_request_context(&self, request_id: &str) -> Option<RequestContext> {
        self.active_requests.lock().await.get(request_id).cloned()
    }
    
    /// Add metadata to an active request
    pub async fn add_request_metadata(&self, request_id: &str, key: String, value: String) -> bool {
        let mut active = self.active_requests.lock().await;
        if let Some(context) = active.get_mut(request_id) {
            context.add_metadata(key, value);
            true
        } else {
            false
        }
    }
    
    /// Get count of active requests
    pub async fn active_request_count(&self) -> usize {
        self.active_requests.lock().await.len()
    }
    
    /// Get recent completed request metrics
    pub async fn get_recent_metrics(&self, limit: usize) -> Vec<RequestMetrics> {
        let completed = self.completed_metrics.lock().await;
        let start = if completed.len() > limit {
            completed.len() - limit
        } else {
            0
        };
        completed[start..].to_vec()
    }
    
    /// Get correlation chain for active request
    pub async fn get_correlation_chain(&self, request_id: &str) -> Option<Vec<String>> {
        self.active_requests.lock().await
            .get(request_id)
            .map(|ctx| ctx.correlation_chain.clone())
    }
    
    /// Start background cleanup task for abandoned requests
    fn start_cleanup_task(&self) {
        let active_requests = self.active_requests.clone();
        let cleanup_timeout = self.config.cleanup_timeout;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute
            
            loop {
                interval.tick().await;
                
                let mut active = active_requests.lock().await;
                let now = Instant::now();
                let mut to_remove = Vec::new();
                
                for (request_id, context) in active.iter() {
                    if now.duration_since(context.start_time) > cleanup_timeout {
                        to_remove.push(request_id.clone());
                    }
                }
                
                for request_id in to_remove {
                    active.remove(&request_id);
                    tracing::warn!(
                        request_id = %request_id,
                        timeout_seconds = cleanup_timeout.as_secs(),
                        "Cleaned up abandoned request"
                    );
                }
            }
        });
    }
}

impl Default for CorrelationManager {
    fn default() -> Self {
        Self::new(CorrelationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_request_correlation() {
        let manager = CorrelationManager::new(CorrelationConfig::default());
        
        // Start root request
        let root_id = manager.start_request("client-1".to_string(), "tools/list".to_string()).await;
        assert_eq!(manager.active_request_count().await, 1);
        
        // Create child request
        let child_id = manager.create_child_request(root_id.clone(), "tools/call".to_string()).await.unwrap();
        assert_eq!(manager.active_request_count().await, 2);
        
        // Verify correlation chain
        let chain = manager.get_correlation_chain(&child_id).await.unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], root_id);
        assert_eq!(chain[1], child_id);
        
        // Complete requests
        let child_metrics = manager.complete_request(child_id, true, None).await.unwrap();
        assert_eq!(child_metrics.success, true);
        assert_eq!(child_metrics.parent_id, Some(root_id.clone()));
        
        let root_metrics = manager.complete_request(root_id, true, None).await.unwrap();
        assert_eq!(root_metrics.success, true);
        assert_eq!(root_metrics.parent_id, None);
        
        assert_eq!(manager.active_request_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_correlation_depth_limit() {
        let config = CorrelationConfig {
            max_correlation_depth: 2,
            ..Default::default()
        };
        let manager = CorrelationManager::new(config);
        
        let root_id = manager.start_request("client-1".to_string(), "method1".to_string()).await;
        let child1_id = manager.create_child_request(root_id, "method2".to_string()).await.unwrap();
        
        // This should fail due to depth limit
        let result = manager.create_child_request(child1_id, "method3".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Maximum correlation depth"));
    }
    
    #[tokio::test]
    async fn test_request_metadata() {
        let manager = CorrelationManager::new(CorrelationConfig::default());
        
        let request_id = manager.start_request("client-1".to_string(), "test".to_string()).await;
        
        // Add metadata
        assert!(manager.add_request_metadata(&request_id, "tool_name".to_string(), "test_tool".to_string()).await);
        assert!(manager.add_request_metadata(&request_id, "input_size".to_string(), "1024".to_string()).await);
        
        // Verify metadata in completed metrics
        let metrics = manager.complete_request(request_id, true, None).await.unwrap();
        assert_eq!(metrics.metadata.get("tool_name"), Some(&"test_tool".to_string()));
        assert_eq!(metrics.metadata.get("input_size"), Some(&"1024".to_string()));
    }
    
    #[test]
    fn test_request_context() {
        let root = RequestContext::new_root("client-1".to_string(), "method1".to_string());
        assert_eq!(root.correlation_depth(), 1);
        assert_eq!(root.parent_id, None);
        assert_eq!(root.root_request_id(), Some(&root.request_id));
        
        let child = RequestContext::new_child(&root, "method2");
        assert_eq!(child.correlation_depth(), 2);
        assert_eq!(child.parent_id, Some(root.request_id.clone()));
        assert_eq!(child.root_request_id(), Some(&root.request_id));
    }
}