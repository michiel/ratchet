//! HTTP-based task repository implementation
//!
//! This module provides a repository implementation that fetches and uploads
//! tasks via HTTP APIs, supporting various authentication methods and RESTful
//! operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use super::task_sync::{
    RepositoryHealth, RepositoryMetadata, RepositoryTask, TaskMetadata, TaskRepository,
};

/// HTTP authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAuth {
    /// Authentication type (bearer, basic, api_key, hmac)
    pub auth_type: String,
    /// Bearer token
    pub token: Option<String>,
    /// Basic auth username
    pub username: Option<String>,
    /// Basic auth password
    pub password: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// API key header name
    pub api_key_header: Option<String>,
    /// HMAC secret
    pub hmac_secret: Option<String>,
    /// Custom headers
    pub custom_headers: Option<HashMap<String, String>>,
}

/// HTTP repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRepositoryConfig {
    /// Base URL for the repository API
    pub base_url: String,
    /// Authentication configuration
    pub auth: Option<HttpAuth>,
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum retries for failed requests
    pub max_retries: Option<u32>,
    /// Custom headers to include with all requests
    pub default_headers: Option<HashMap<String, String>>,
}

/// HTTP-based task repository
pub struct HttpTaskRepository {
    /// Base URL for the repository API
    base_url: String,
    /// Authentication configuration
    auth_config: Option<HttpAuth>,
    /// HTTP client
    client: Client,
    /// Repository name
    name: String,
    /// Maximum retries for failed requests
    max_retries: u32,
}

/// HTTP API task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpApiTask {
    /// Task path/identifier
    pub path: String,
    /// Task name
    pub name: String,
    /// JavaScript source code
    pub source_code: String,
    /// Input schema
    pub input_schema: JsonValue,
    /// Output schema
    pub output_schema: JsonValue,
    /// Task metadata
    pub metadata: TaskMetadata,
    /// SHA256 checksum
    pub checksum: String,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// HTTP API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpApiResponse<T> {
    /// Response data
    pub data: T,
    /// Optional message
    pub message: Option<String>,
    /// Success flag
    pub success: bool,
}

/// HTTP API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpApiError {
    /// Error message
    pub error: String,
    /// Error code
    pub code: Option<String>,
    /// Additional error details
    pub details: Option<JsonValue>,
}

impl HttpTaskRepository {
    /// Create a new HTTP repository
    pub fn new(config: HttpRepositoryConfig, name: String) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        
        let mut client_builder = Client::builder()
            .timeout(timeout)
            .user_agent("ratchet-http-repository/1.0");

        // Add default headers if configured
        if let Some(headers) = &config.default_headers {
            let mut default_headers = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value)
                ) {
                    default_headers.insert(name, value);
                }
            }
            client_builder = client_builder.default_headers(default_headers);
        }

        let client = client_builder.build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url: config.base_url,
            auth_config: config.auth,
            client,
            name,
            max_retries: config.max_retries.unwrap_or(3),
        })
    }

    /// Apply authentication to a request builder
    fn apply_auth(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(auth) = &self.auth_config {
            match auth.auth_type.as_str() {
                "bearer" => {
                    if let Some(token) = &auth.token {
                        request = request.bearer_auth(token);
                    }
                }
                "basic" => {
                    if let Some(username) = &auth.username {
                        let password = auth.password.as_deref().unwrap_or("");
                        request = request.basic_auth(username, Some(password));
                    }
                }
                "api_key" => {
                    if let Some(api_key) = &auth.api_key {
                        let header_name = auth.api_key_header.as_deref().unwrap_or("X-API-Key");
                        request = request.header(header_name, api_key);
                    }
                }
                "hmac" => {
                    // HMAC authentication would require request signing
                    // Implementation would depend on specific HMAC requirements
                    debug!("HMAC authentication not yet implemented");
                }
                _ => {
                    warn!("Unknown authentication type: {}", auth.auth_type);
                }
            }

            // Apply custom headers
            if let Some(headers) = &auth.custom_headers {
                for (key, value) in headers {
                    request = request.header(key, value);
                }
            }
        }

        request
    }

    /// Execute HTTP request with retry logic
    async fn execute_with_retry<F, Fut>(&self, operation: F) -> Result<Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Response>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();
                        last_error = Some(anyhow!(
                            "HTTP request failed with status {}: {}",
                            status,
                            error_text
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }

            if attempt < self.max_retries {
                let delay = Duration::from_millis(100 * (2_u64.pow(attempt)));
                debug!("Retrying HTTP request in {:?} (attempt {}/{})", delay, attempt + 1, self.max_retries);
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("HTTP request failed after {} retries", self.max_retries)))
    }

    /// Convert internal RepositoryTask to HTTP API format
    fn to_http_task(&self, task: &RepositoryTask) -> HttpApiTask {
        HttpApiTask {
            path: task.path.clone(),
            name: task.name.clone(),
            source_code: task.source_code.clone(),
            input_schema: task.input_schema.clone(),
            output_schema: task.output_schema.clone(),
            metadata: task.metadata.clone(),
            checksum: task.checksum.clone(),
            modified_at: task.modified_at,
            created_at: task.created_at,
        }
    }

    /// Convert HTTP API task to internal RepositoryTask format
    fn from_http_task(&self, http_task: HttpApiTask) -> RepositoryTask {
        RepositoryTask {
            path: http_task.path,
            name: http_task.name,
            source_code: http_task.source_code,
            input_schema: http_task.input_schema,
            output_schema: http_task.output_schema,
            metadata: http_task.metadata,
            checksum: http_task.checksum,
            modified_at: http_task.modified_at,
            created_at: http_task.created_at,
        }
    }

    /// Parse error response from HTTP API
    async fn parse_error_response(&self, response: Response) -> String {
        let status = response.status();
        
        match response.text().await {
            Ok(text) => {
                // Try to parse as structured error
                if let Ok(error_response) = serde_json::from_str::<HttpApiError>(&text) {
                    format!("HTTP {} - {}", status, error_response.error)
                } else {
                    format!("HTTP {} - {}", status, text)
                }
            }
            Err(_) => format!("HTTP {} - Failed to read error response", status),
        }
    }
}

#[async_trait]
impl TaskRepository for HttpTaskRepository {
    async fn list_tasks(&self) -> Result<Vec<RepositoryTask>> {
        debug!("Fetching task list from HTTP repository: {}", self.base_url);

        let response = self.execute_with_retry(|| async {
            let request = self.client.get(&format!("{}/tasks", self.base_url));
            let request = self.apply_auth(request);
            request.send().await.context("Failed to send HTTP request")
        }).await?;

        if !response.status().is_success() {
            let error_msg = self.parse_error_response(response).await;
            return Err(anyhow!("Failed to list tasks: {}", error_msg));
        }

        let api_response: HttpApiResponse<Vec<HttpApiTask>> = response
            .json()
            .await
            .context("Failed to parse task list response")?;

        if !api_response.success {
            return Err(anyhow!("API request failed: {}", 
                api_response.message.unwrap_or("Unknown error".to_string())));
        }

        let tasks: Vec<RepositoryTask> = api_response.data
            .into_iter()
            .map(|http_task| self.from_http_task(http_task))
            .collect();

        info!("Successfully fetched {} tasks from HTTP repository", tasks.len());
        Ok(tasks)
    }

    async fn get_task(&self, path: &str) -> Result<Option<RepositoryTask>> {
        debug!("Fetching task from HTTP repository: {}/{}", self.base_url, path);

        let response = self.execute_with_retry(|| async {
            let url = format!("{}/tasks/{}", self.base_url, urlencoding::encode(path));
            let request = self.client.get(&url);
            let request = self.apply_auth(request);
            request.send().await.context("Failed to send HTTP request")
        }).await?;

        match response.status() {
            StatusCode::OK => {
                let api_response: HttpApiResponse<HttpApiTask> = response
                    .json()
                    .await
                    .context("Failed to parse task response")?;

                if api_response.success {
                    Ok(Some(self.from_http_task(api_response.data)))
                } else {
                    Err(anyhow!("API request failed: {}", 
                        api_response.message.unwrap_or("Unknown error".to_string())))
                }
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => {
                let error_msg = self.parse_error_response(response).await;
                Err(anyhow!("Failed to get task: {}", error_msg))
            }
        }
    }

    async fn put_task(&self, task: &RepositoryTask) -> Result<()> {
        debug!("Uploading task to HTTP repository: {}/{}", self.base_url, task.path);

        let http_task = self.to_http_task(task);
        
        let response = self.execute_with_retry(|| async {
            let url = format!("{}/tasks/{}", self.base_url, urlencoding::encode(&task.path));
            let request = self.client.put(&url);
            let request = self.apply_auth(request);
            request.json(&http_task)
                   .send()
                   .await
                   .context("Failed to send HTTP request")
        }).await?;

        if !response.status().is_success() {
            let error_msg = self.parse_error_response(response).await;
            return Err(anyhow!("Failed to upload task: {}", error_msg));
        }

        info!("Successfully uploaded task: {}", task.name);
        Ok(())
    }

    async fn delete_task(&self, path: &str) -> Result<()> {
        debug!("Deleting task from HTTP repository: {}/{}", self.base_url, path);

        let response = self.execute_with_retry(|| async {
            let url = format!("{}/tasks/{}", self.base_url, urlencoding::encode(path));
            let request = self.client.delete(&url);
            let request = self.apply_auth(request);
            request.send().await.context("Failed to send HTTP request")
        }).await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => {
                info!("Successfully deleted task: {}", path);
                Ok(())
            }
            StatusCode::NOT_FOUND => {
                warn!("Task not found for deletion: {}", path);
                Ok(()) // Consider deletion of non-existent task as success
            }
            _ => {
                let error_msg = self.parse_error_response(response).await;
                Err(anyhow!("Failed to delete task: {}", error_msg))
            }
        }
    }

    async fn get_metadata(&self) -> Result<RepositoryMetadata> {
        debug!("Fetching repository metadata from: {}", self.base_url);

        let response = self.execute_with_retry(|| async {
            let request = self.client.get(&format!("{}/metadata", self.base_url));
            let request = self.apply_auth(request);
            request.send().await.context("Failed to send HTTP request")
        }).await?;

        let mut metadata = HashMap::new();
        metadata.insert("base_url".to_string(), JsonValue::String(self.base_url.clone()));
        
        if let Some(auth) = &self.auth_config {
            metadata.insert("auth_type".to_string(), JsonValue::String(auth.auth_type.clone()));
        }

        // Try to get additional metadata from the API
        if response.status().is_success() {
            if let Ok(api_metadata) = response.json::<JsonValue>().await {
                if let Some(obj) = api_metadata.as_object() {
                    for (key, value) in obj {
                        metadata.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        Ok(RepositoryMetadata {
            name: self.name.clone(),
            repository_type: "http".to_string(),
            uri: self.base_url.clone(),
            branch: None,
            commit: None,
            is_writable: true, // Assume writable if we can authenticate
            metadata,
        })
    }

    async fn is_writable(&self) -> bool {
        // HTTP repositories are considered writable if we have authentication
        self.auth_config.is_some()
    }

    async fn test_connection(&self) -> Result<bool> {
        debug!("Testing HTTP repository connection: {}", self.base_url);

        match self.execute_with_retry(|| async {
            let request = self.client.get(&format!("{}/health", self.base_url));
            let request = self.apply_auth(request);
            request.send().await.context("Failed to send health check request")
        }).await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                debug!("HTTP connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn health_check(&self) -> Result<RepositoryHealth> {
        let accessible = self.test_connection().await.unwrap_or(false);
        let writable = accessible && self.is_writable().await;
        
        let message = if !accessible {
            "Cannot connect to HTTP repository".to_string()
        } else if !writable {
            "HTTP repository is read-only (no authentication)".to_string()
        } else {
            "HTTP repository is healthy".to_string()
        };

        Ok(RepositoryHealth {
            accessible,
            writable,
            last_success: if accessible { Some(Utc::now()) } else { None },
            error_count: if accessible { 0 } else { 1 },
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_http_repository_creation() {
        let config = HttpRepositoryConfig {
            base_url: "https://api.example.com/tasks".to_string(),
            auth: None,
            timeout_seconds: Some(60),
            max_retries: Some(5),
            default_headers: None,
        };

        let repo = HttpTaskRepository::new(config, "test-http-repo".to_string()).unwrap();
        assert_eq!(repo.name, "test-http-repo");
        assert_eq!(repo.base_url, "https://api.example.com/tasks");
        assert_eq!(repo.max_retries, 5);
    }

    #[test]
    fn test_http_auth_configuration() {
        let auth = HttpAuth {
            auth_type: "bearer".to_string(),
            token: Some("token123".to_string()),
            username: None,
            password: None,
            api_key: None,
            api_key_header: None,
            hmac_secret: None,
            custom_headers: None,
        };

        let config = HttpRepositoryConfig {
            base_url: "https://api.example.com/tasks".to_string(),
            auth: Some(auth),
            timeout_seconds: None,
            max_retries: None,
            default_headers: None,
        };

        let repo = HttpTaskRepository::new(config, "test-repo".to_string()).unwrap();
        assert!(repo.auth_config.is_some());
        assert_eq!(repo.auth_config.unwrap().auth_type, "bearer");
    }

    #[test]
    fn test_task_conversion() {
        let config = HttpRepositoryConfig {
            base_url: "https://api.example.com".to_string(),
            auth: None,
            timeout_seconds: None,
            max_retries: None,
            default_headers: None,
        };

        let repo = HttpTaskRepository::new(config, "test".to_string()).unwrap();

        let task_metadata = TaskMetadata::minimal("1.0.0".to_string());
        let task = RepositoryTask::new(
            "test/task.js".to_string(),
            "test_task".to_string(),
            "function test() { return 'hello'; }".to_string(),
            json!({"type": "object"}),
            json!({"type": "string"}),
            task_metadata,
        );

        let http_task = repo.to_http_task(&task);
        let converted_back = repo.from_http_task(http_task);

        assert_eq!(task.name, converted_back.name);
        assert_eq!(task.source_code, converted_back.source_code);
        assert_eq!(task.path, converted_back.path);
    }
}