//! HTTP client implementation

use crate::config::HttpConfig;
use crate::errors::HttpError;
use crate::types::HttpMethod;
use anyhow::Result;
use chrono::Utc;
use reqwest::{
    self,
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, info, warn};

/// HTTP client trait for making HTTP requests
#[async_trait::async_trait]
pub trait HttpClient {
    async fn call_http(
        &self,
        url: &str,
        params: Option<&JsonValue>,
        body: Option<&JsonValue>,
    ) -> Result<JsonValue, HttpError>;
}

/// HTTP Manager for handling HTTP requests with mock support
#[derive(Debug, Clone)]
pub struct HttpManager {
    offline: bool,
    mocks: HashMap<String, JsonValue>,
    config: HttpConfig,
}

impl Default for HttpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpManager {
    /// Create a new HttpManager in online mode with default configuration
    pub fn new() -> Self {
        Self::with_config(HttpConfig::default())
    }

    /// Create a new HttpManager with specific configuration
    pub fn with_config(config: HttpConfig) -> Self {
        debug!(
            "Creating HttpManager with timeout: {}s",
            config.timeout.as_secs()
        );
        Self {
            offline: false,
            mocks: HashMap::new(),
            config,
        }
    }

    /// Set offline mode
    pub fn set_offline(&mut self) {
        self.offline = true;
        debug!("HttpManager set to offline mode");
    }

    /// Set online mode
    pub fn set_online(&mut self) {
        self.offline = false;
        debug!("HttpManager set to online mode");
    }

    /// Add HTTP mocks to the manager
    /// Key should be in format "METHOD:URL" (e.g., "GET:http://example.com")
    pub fn add_mocks(&mut self, mocks: HashMap<String, JsonValue>) {
        self.mocks.extend(mocks);
        debug!("Added {} HTTP mocks", self.mocks.len());
    }

    /// Add a single HTTP mock
    pub fn add_mock(&mut self, method: HttpMethod, url: &str, response: JsonValue) {
        let key = format!("{}:{}", method.as_str(), url);
        self.mocks.insert(key, response);
        debug!("Added HTTP mock for {} {}", method, url);
    }

    /// Add a single HTTP mock using string method (for backward compatibility)
    pub fn add_mock_str(
        &mut self,
        method: &str,
        url: &str,
        response: JsonValue,
    ) -> Result<(), crate::types::HttpMethodError> {
        let http_method: HttpMethod = method.parse()?;
        self.add_mock(http_method, url, response);
        Ok(())
    }

    /// Clear all mocks
    pub fn clear_mocks(&mut self) {
        self.mocks.clear();
        debug!("Cleared all HTTP mocks");
    }
}

#[async_trait::async_trait]
impl HttpClient for HttpManager {
    /// Perform an HTTP request similar to the JavaScript fetch API
    async fn call_http(
        &self,
        url: &str,
        params: Option<&JsonValue>,
        body: Option<&JsonValue>,
    ) -> Result<JsonValue, HttpError> {
        let start_time = Utc::now();

        info!("Making HTTP request to: {}", url);
        debug!("Request params: {:?}", params);
        debug!("Request body: {:?}", body);

        // Extract method from params or default to GET
        let method = if let Some(params) = params {
            if let Some(method_str) = params.get("method").and_then(|m| m.as_str()) {
                method_str.parse().unwrap_or(HttpMethod::Get)
            } else {
                HttpMethod::Get
            }
        } else {
            HttpMethod::Get
        };

        // Check if we're in offline mode and return mock data if available
        if self.offline {
            debug!("Offline mode enabled, checking for mock response");
            let mock_key = format!("{}:{}", method.as_str(), url);

            if let Some(mock_response) = self.mocks.get(&mock_key) {
                debug!("Found matching mock response for {} {}", method, url);
                // Create a response object from the mock data
                let response = json!({
                    "ok": true,
                    "status": 200,
                    "statusText": "OK",
                    "headers": {},
                    "body": mock_response
                });
                return Ok(response);
            } else {
                // Check for partial URL matches
                for (key, response) in &self.mocks {
                    if let Some((mock_method, mock_url)) = key.split_once(':') {
                        if mock_method.eq_ignore_ascii_case(method.as_str())
                            && (url.contains(mock_url) || mock_url.contains(url))
                        {
                            debug!(
                                "Found partial matching mock response for {} {}",
                                method, url
                            );
                            let response = json!({
                                "ok": true,
                                "status": 200,
                                "statusText": "OK",
                                "headers": {},
                                "body": response
                            });
                            return Ok(response);
                        }
                    }
                }

                debug!("No matching mock response found for {} {}", method, url);
                return Err(HttpError::InvalidUrl(
                    "No mock response available in offline mode".to_string(),
                ));
            }
        }

        // If no mock data or mock doesn't match, perform a real HTTP request
        debug!(
            "Creating HTTP client with {}s timeout",
            self.config.timeout.as_secs()
        );
        // Create a client with configured settings
        let client = Client::builder()
            .timeout(self.config.timeout)
            .user_agent(&self.config.user_agent)
            .danger_accept_invalid_certs(!self.config.verify_ssl)
            .redirect(reqwest::redirect::Policy::limited(
                self.config.max_redirects as usize,
            ))
            .build()?;

        // Extract headers for recording
        let request_headers: Option<HashMap<String, String>> = if let Some(params) = params {
            params
                .get("headers")
                .and_then(|h| h.as_object())
                .map(|headers| {
                    headers
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
        } else {
            None
        };

        // Convert body to string for recording
        let request_body_str = body.map(|b| {
            if let Some(s) = b.as_str() {
                s.to_string()
            } else {
                serde_json::to_string(b).unwrap_or_default()
            }
        });

        let reqwest_method = reqwest::Method::from(method);

        debug!("Building {} request to {}", method, url);
        // Build the request
        let mut request = client.request(reqwest_method, url);

        // Add headers if provided
        if let Some(params) = params {
            if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
                debug!("Adding {} custom headers", headers.len());
                let mut header_map = HeaderMap::new();
                for (key, value) in headers {
                    if let Some(value_str) = value.as_str() {
                        let header_name = HeaderName::from_str(key).map_err(|_| {
                            HttpError::InvalidHeaderName(key.to_string())
                        })?;

                        if let Ok(header_value) = HeaderValue::from_str(value_str) {
                            header_map.insert(header_name, header_value);
                        }
                    }
                }
                request = request.headers(header_map);
            }
        }

        // Add body if provided
        if let Some(body) = body {
            // Check if the Content-Type header indicates form data
            let is_form_data = if let Some(params) = params {
                if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
                    headers
                        .get("Content-Type")
                        .and_then(|ct| ct.as_str())
                        .map(|ct| ct.contains("application/x-www-form-urlencoded"))
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            };

            if is_form_data {
                // Send as form data if body is a string
                if let Some(body_str) = body.as_str() {
                    debug!("Adding form-encoded body to request");
                    request = request.body(body_str.to_string());
                } else {
                    debug!(
                        "Adding JSON body to request (form data expected but body is not string)"
                    );
                    request = request.json(body);
                }
            } else {
                debug!("Adding JSON body to request");
                request = request.json(body);
            }
        }

        // Send the request and get the response
        debug!("Sending HTTP request");
        let response = request.send().await?;

        // Get status info
        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("Unknown Status");

        info!("HTTP response received: {} {}", status_code, status_text);

        // Collect response headers for recording
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.to_string(), v.to_string()))
            })
            .collect();

        // Try to parse the response as JSON, fall back to text if it fails
        debug!("Parsing response body");
        let response_body = match response.json::<JsonValue>().await {
            Ok(json_data) => {
                debug!("Successfully parsed response as JSON");
                json_data
            }
            Err(_) => {
                warn!("Failed to parse response as JSON, falling back to text");
                // Fall back to text - we need to send a new request since json() consumes the response
                let text_response = client
                    .request(reqwest::Method::from(method), url)
                    .send()
                    .await?;
                let text = text_response.text().await?;
                debug!("Response parsed as text: {} bytes", text.len());
                json!(text)
            }
        };

        // Record the HTTP request if recording is enabled
        #[cfg(feature = "recording")]
        {
            let end_time = Utc::now();
            let duration_ms = (end_time - start_time).num_milliseconds() as u64;

            if crate::recording::is_recording() {
                let response_body_str = serde_json::to_string(&response_body).unwrap_or_default();
                if let Err(e) = crate::recording::record_http_request(
                    url,
                    method.as_str(),
                    request_headers.as_ref(),
                    request_body_str.as_deref(),
                    status_code,
                    Some(&response_headers),
                    &response_body_str,
                    start_time,
                    duration_ms,
                ) {
                    warn!("Failed to record HTTP request: {}", e);
                }
            }
        }

        // Construct a response object similar to JavaScript's Response
        debug!("Constructing response object");
        let result = json!({
            "ok": status.is_success(),
            "status": status_code,
            "statusText": status_text,
            "headers": response_headers,
            "body": response_body
        });

        debug!("HTTP call completed successfully");
        Ok(result)
    }
}

/// Create a default HttpManager instance for backward compatibility
pub fn create_http_manager() -> HttpManager {
    HttpManager::new()
}