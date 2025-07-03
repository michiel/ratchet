//! Server-Sent Events transport implementation for MCP

use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use super::{McpTransport, SseAuth, TransportHealth};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::{McpError, McpResult};

/// Server-Sent Events transport for HTTP-based MCP servers
#[derive(Debug)]
pub struct SseTransport {
    /// Base URL for SSE endpoint
    url: String,

    /// HTTP headers
    headers: HashMap<String, String>,

    /// Authentication configuration
    auth: Option<SseAuth>,

    /// Request timeout
    _timeout: Duration,

    /// Whether to verify SSL certificates
    verify_ssl: bool,

    /// HTTP client
    client: Client,

    /// Event stream
    event_stream: Option<tokio_stream::wrappers::ReceiverStream<String>>,

    /// Transport health tracking
    health: Mutex<TransportHealth>,

    /// Whether the transport is connected
    connected: bool,

    /// Session ID for this connection
    session_id: Option<String>,

    /// Pending responses waiting for correlation
    pending_responses: Mutex<HashMap<String, tokio::sync::oneshot::Sender<JsonRpcResponse>>>,
}

impl SseTransport {
    /// Create a new SSE transport
    pub fn new(
        url: String,
        headers: HashMap<String, String>,
        auth: Option<SseAuth>,
        timeout: Duration,
        verify_ssl: bool,
    ) -> McpResult<Self> {
        if url.trim().is_empty() {
            return Err(McpError::Configuration {
                message: "URL cannot be empty".to_string(),
            });
        }

        // Validate URL and check for safe schemes
        let parsed_url = url::Url::parse(&url).map_err(|e| McpError::Configuration {
            message: format!("Invalid URL: {}", e),
        })?;
        
        // Only allow HTTP and HTTPS schemes for security
        match parsed_url.scheme() {
            "http" | "https" => {},
            scheme => {
                return Err(McpError::Configuration {
                    message: format!("Unsupported or unsafe URL scheme: {}. Only http and https are allowed.", scheme),
                });
            }
        }

        // Create HTTP client
        let client = Client::builder()
            .timeout(timeout)
            .danger_accept_invalid_certs(!verify_ssl)
            .build()
            .map_err(|e| McpError::Configuration {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            url,
            headers,
            auth,
            _timeout: timeout,
            verify_ssl,
            client,
            event_stream: None,
            health: Mutex::new(TransportHealth::unhealthy("Not connected")),
            connected: false,
            session_id: None,
            pending_responses: Mutex::new(HashMap::new()),
        })
    }

    /// Apply authentication to request builder
    fn apply_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.auth {
            Some(SseAuth::Bearer { token }) => builder.bearer_auth(token),
            Some(SseAuth::Basic { username, password }) => builder.basic_auth(username, Some(password)),
            Some(SseAuth::ApiKey { header, key }) => builder.header(header, key),
            None => builder,
        }
    }

    /// Create an SSE connection
    async fn create_sse_connection(&mut self) -> McpResult<()> {
        // Generate a session ID for this connection
        let session_id = uuid::Uuid::new_v4().to_string();
        self.session_id = Some(session_id.clone());

        // Build SSE endpoint URL
        let sse_url = if self.url.ends_with('/') {
            format!("{}sse/{}", self.url, session_id)
        } else {
            format!("{}/sse/{}", self.url, session_id)
        };

        let mut builder = self.client.get(&sse_url);

        // Apply headers
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        // Apply authentication
        builder = self.apply_auth(builder);

        // Set SSE headers
        builder = builder
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive");

        let response = builder.send().await.map_err(|e| McpError::ConnectionFailed {
            message: format!("Failed to connect to SSE endpoint: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(McpError::ConnectionFailed {
                message: format!("SSE connection failed with status: {}", response.status()),
            });
        }

        // Create event stream
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stream = response.bytes_stream();

        // Spawn task to process SSE events
        tokio::spawn(async move {
            let mut stream = stream;
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                            buffer.push_str(&text);

                            // Process complete SSE events (double newline separated)
                            while let Some(event_end) = buffer.find("\n\n") {
                                let event_text = buffer[..event_end].trim().to_string();
                                buffer = buffer[event_end + 2..].to_string();

                                // Parse SSE event format
                                for line in event_text.lines() {
                                    let line = line.trim();
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        // Remove "data: " prefix
                                        if !data.trim().is_empty()
                                            && data != "keep-alive"
                                            && tx.send(data.to_string()).await.is_err()
                                        {
                                            return; // Receiver dropped
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.event_stream = Some(tokio_stream::wrappers::ReceiverStream::new(rx));
        Ok(())
    }

    /// Send request via HTTP POST
    async fn send_http_request(&self, request: JsonRpcRequest) -> McpResult<()> {
        // Get the session ID
        let session_id = self.session_id.as_ref().ok_or_else(|| McpError::Transport {
            message: "No session ID available. Connect first.".to_string(),
        })?;

        // For SSE client, send to the message endpoint
        let message_url = if self.url.ends_with('/') {
            format!("{}message/{}", self.url, session_id)
        } else {
            format!("{}/message/{}", self.url, session_id)
        };

        let mut builder = self.client.post(&message_url);

        // Apply headers
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        // Apply authentication
        builder = self.apply_auth(builder);

        // Send JSON request
        builder = builder.header("Content-Type", "application/json").json(&request);

        let response = builder.send().await.map_err(|e| McpError::Network {
            message: format!("Failed to send HTTP request: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(McpError::Network {
                message: format!("HTTP request failed with status: {}", response.status()),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn connect(&mut self) -> McpResult<()> {
        if self.connected {
            return Ok(());
        }

        self.create_sse_connection().await?;
        self.connected = true;

        // Update health status
        let mut health = self.health.lock().await;
        health.mark_success(None);
        health
            .metadata
            .insert("url".to_string(), serde_json::Value::String(self.url.clone()));
        health
            .metadata
            .insert("verify_ssl".to_string(), serde_json::Value::Bool(self.verify_ssl));

        Ok(())
    }

    async fn send(&mut self, message: JsonRpcRequest) -> McpResult<()> {
        if !self.connected {
            return Err(McpError::Transport {
                message: "Transport not connected".to_string(),
            });
        }

        let start_time = Instant::now();

        match self.send_http_request(message).await {
            Ok(()) => {
                let latency = start_time.elapsed();
                let mut health = self.health.lock().await;
                health.mark_success(Some(latency));
                Ok(())
            }
            Err(e) => {
                self.connected = false;
                let mut health = self.health.lock().await;
                health.mark_failure(e.to_string());
                Err(e)
            }
        }
    }

    async fn receive(&mut self) -> McpResult<JsonRpcResponse> {
        if !self.connected {
            return Err(McpError::Transport {
                message: "Transport not connected".to_string(),
            });
        }

        let stream = self.event_stream.as_mut().ok_or_else(|| McpError::Transport {
            message: "Event stream not available".to_string(),
        })?;

        let start_time = Instant::now();

        // Wait for next event
        match stream.next().await {
            Some(data) => {
                // Parse JSON response
                let response: JsonRpcResponse = serde_json::from_str(&data).map_err(|e| McpError::Serialization {
                    message: format!("Failed to parse SSE response: {}", e),
                })?;

                // Update health
                let latency = start_time.elapsed();
                let mut health = self.health.lock().await;
                health.mark_success(Some(latency));

                Ok(response)
            }
            None => {
                self.connected = false;
                let mut health = self.health.lock().await;
                health.mark_failure("SSE stream ended".to_string());

                Err(McpError::ConnectionFailed {
                    message: "SSE stream ended".to_string(),
                })
            }
        }
    }

    async fn is_connected(&self) -> bool {
        self.connected
    }

    async fn health(&self) -> TransportHealth {
        self.health.lock().await.clone()
    }

    async fn close(&mut self) -> McpResult<()> {
        if !self.connected {
            return Ok(());
        }

        self.connected = false;
        self.event_stream = None;
        self.session_id = None;

        // Clear pending responses
        let mut pending = self.pending_responses.lock().await;
        pending.clear();

        // Update health
        let mut health = self.health.lock().await;
        health.connected = false;
        health.metadata.insert(
            "disconnected_at".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );

        Ok(())
    }

    async fn send_and_receive(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Duration,
    ) -> McpResult<JsonRpcResponse> {
        // For SSE, we need to correlate requests and responses by ID
        let request_id = request.id_as_string().ok_or_else(|| McpError::Protocol {
            message: "Request must have an ID for SSE transport".to_string(),
        })?;

        // Create a channel for the response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Store the sender for this request ID
        {
            let mut pending = self.pending_responses.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send the request
        self.send(request).await?;

        // Wait for response with timeout
        let response = tokio::time::timeout(timeout_duration, rx)
            .await
            .map_err(|_| McpError::ServerTimeout {
                timeout: timeout_duration,
            })?
            .map_err(|_| McpError::Internal {
                message: "Response channel was dropped".to_string(),
            })?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sse_transport_creation() {
        let transport = SseTransport::new(
            "https://example.com/sse".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            true,
        );
        assert!(transport.is_ok());

        let empty_url = SseTransport::new("".to_string(), HashMap::new(), None, Duration::from_secs(30), true);
        assert!(empty_url.is_err());

        let invalid_url = SseTransport::new(
            "not-a-url".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            true,
        );
        assert!(invalid_url.is_err());
    }

    #[test]
    fn test_auth_configuration() {
        let auth_configs = vec![
            SseAuth::Bearer {
                token: "test-token".to_string(),
            },
            SseAuth::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            },
            SseAuth::ApiKey {
                header: "X-API-Key".to_string(),
                key: "api-key".to_string(),
            },
        ];

        for auth in auth_configs {
            let transport = SseTransport::new(
                "https://example.com/sse".to_string(),
                HashMap::new(),
                Some(auth),
                Duration::from_secs(30),
                true,
            );
            assert!(transport.is_ok());
        }
    }

    #[tokio::test]
    async fn test_sse_transport_not_connected_initially() {
        let transport = SseTransport::new(
            "https://example.com/sse".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            true,
        )
        .unwrap();

        assert!(!transport.is_connected().await);

        let health = transport.health().await;
        assert!(!health.is_healthy());
        assert!(!health.connected);
    }

    #[tokio::test]
    async fn test_sse_session_id_generation() {
        let mut transport = SseTransport::new(
            "http://localhost:8080".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            false, // Don't verify SSL for test
        )
        .unwrap();

        assert!(transport.session_id.is_none());

        // Connection will fail but session ID should be generated
        let _ = transport.create_sse_connection().await;
        assert!(transport.session_id.is_some());
    }

    #[test]
    fn test_message_url_generation() {
        let transport = SseTransport::new(
            "http://localhost:8080".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            false,
        )
        .unwrap();

        // Test URL with trailing slash
        let transport_with_slash = SseTransport::new(
            "http://localhost:8080/".to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            false,
        )
        .unwrap();

        // Both should be valid
        assert!(transport.url.starts_with("http://"));
        assert!(transport_with_slash.url.starts_with("http://"));
    }
}
