//! Webhook output destination implementation

use async_trait::async_trait;
use reqwest;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::output::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    errors::{DeliveryError, ValidationError},
    template::TemplateEngine,
    RetryPolicy, WebhookAuth,
};
use crate::types::HttpMethod;

/// Configuration for webhook destination
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub url_template: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub auth: Option<WebhookAuth>,
    pub content_type: Option<String>,
}

/// Webhook destination for sending HTTP requests
#[derive(Debug)]
pub struct WebhookDestination {
    config: WebhookConfig,
    client: reqwest::Client,
    template_engine: TemplateEngine,
}

impl WebhookDestination {
    pub fn new(
        config: WebhookConfig,
        client: reqwest::Client,
        template_engine: TemplateEngine,
    ) -> Self {
        Self {
            config,
            client,
            template_engine,
        }
    }

    /// Add authentication to the request
    fn add_auth(
        &self,
        mut request: reqwest::RequestBuilder,
        auth: &WebhookAuth,
        context: &DeliveryContext,
    ) -> Result<reqwest::RequestBuilder, DeliveryError> {
        match auth {
            WebhookAuth::Bearer { token } => {
                let rendered_token = self
                    .template_engine
                    .render(token, &context.template_variables)
                    .map_err(|e| DeliveryError::TemplateRender {
                        template: token.clone(),
                        error: e.to_string(),
                    })?;
                request = request.bearer_auth(rendered_token);
            }
            WebhookAuth::Basic { username, password } => {
                let rendered_username = self
                    .template_engine
                    .render(username, &context.template_variables)
                    .map_err(|e| DeliveryError::TemplateRender {
                        template: username.clone(),
                        error: e.to_string(),
                    })?;
                let rendered_password = self
                    .template_engine
                    .render(password, &context.template_variables)
                    .map_err(|e| DeliveryError::TemplateRender {
                        template: password.clone(),
                        error: e.to_string(),
                    })?;
                request = request.basic_auth(rendered_username, Some(rendered_password));
            }
            WebhookAuth::ApiKey { header, key } => {
                let rendered_key = self
                    .template_engine
                    .render(key, &context.template_variables)
                    .map_err(|e| DeliveryError::TemplateRender {
                        template: key.clone(),
                        error: e.to_string(),
                    })?;
                request = request.header(header, rendered_key);
            }
            WebhookAuth::Signature { secret, algorithm } => {
                // For now, we'll implement a basic HMAC-SHA256 signature
                // This can be extended for other algorithms
                if algorithm.to_lowercase() == "hmac-sha256" {
                    let rendered_secret = self
                        .template_engine
                        .render(secret, &context.template_variables)
                        .map_err(|e| DeliveryError::TemplateRender {
                            template: secret.clone(),
                            error: e.to_string(),
                        })?;

                    // Note: We'd need the request body to generate the signature
                    // For now, we'll add a placeholder - this should be implemented
                    // with the actual body content in a real implementation
                    let signature = format!("sha256={}", rendered_secret); // Placeholder
                    request = request.header("X-Hub-Signature-256", signature);
                } else {
                    return Err(DeliveryError::InvalidTemplateVariable {
                        variable: format!("Unsupported signature algorithm: {}", algorithm),
                    });
                }
            }
        }

        Ok(request)
    }

    /// Build the request body
    fn build_request_body(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<Vec<u8>, DeliveryError> {
        // Build comprehensive request body
        let body = serde_json::json!({
            "job_id": output.job_id,
            "task_id": output.task_id,
            "execution_id": output.execution_id,
            "task_name": context.task_name,
            "task_version": context.task_version,
            "completed_at": output.completed_at,
            "execution_duration_ms": output.execution_duration.as_millis(),
            "trace_id": context.trace_id,
            "output": output.output_data,
            "metadata": output.metadata,
            "environment": context.environment,
            "timestamp": context.timestamp,
        });

        serde_json::to_vec(&body).map_err(|e| DeliveryError::Serialization {
            format: "json".to_string(),
            error: e.to_string(),
        })
    }

    /// Check if HTTP status should trigger a retry
    fn should_retry_status(&self, status: u16) -> bool {
        self.config.retry_policy.retry_on_status.contains(&status) ||
            (500..600).contains(&status) || // Server errors
            status == 429 // Rate limited
    }

    /// Calculate retry delay with exponential backoff and jitter
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.retry_policy.initial_delay.as_millis() as f64;
        let multiplier = self.config.retry_policy.backoff_multiplier;
        let delay_ms = base_delay * multiplier.powi(attempt as i32 - 1);

        let delay = Duration::from_millis(delay_ms as u64).min(self.config.retry_policy.max_delay);

        if self.config.retry_policy.jitter {
            let jitter_range = delay.as_millis() / 4; // 25% jitter
            let jitter = Duration::from_millis(fastrand::u64(0..=jitter_range as u64));
            delay + jitter
        } else {
            delay
        }
    }
}

#[async_trait]
impl OutputDestination for WebhookDestination {
    async fn deliver(
        &self,
        output: &TaskOutput,
        context: &DeliveryContext,
    ) -> Result<DeliveryResult, DeliveryError> {
        let start_time = Instant::now();

        // 1. Render URL template
        let url = self
            .template_engine
            .render(&self.config.url_template, &context.template_variables)
            .map_err(|e| DeliveryError::TemplateRender {
                template: self.config.url_template.clone(),
                error: e.to_string(),
            })?;

        // 2. Build request body
        let body = self.build_request_body(output, context)?;

        // 3. Build base request
        let method = match self.config.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        };

        let mut request = self
            .client
            .request(method, &url)
            .timeout(self.config.timeout);

        // 4. Add headers (with template rendering)
        for (key, value_template) in &self.config.headers {
            let value = self
                .template_engine
                .render(value_template, &context.template_variables)
                .map_err(|e| DeliveryError::TemplateRender {
                    template: value_template.clone(),
                    error: e.to_string(),
                })?;
            request = request.header(key, value);
        }

        // 5. Add authentication
        if let Some(auth) = &self.config.auth {
            request = self.add_auth(request, auth, context)?;
        }

        // 6. Set content type and body
        let content_type = self
            .config
            .content_type
            .as_deref()
            .unwrap_or("application/json");
        request = request.header("Content-Type", content_type);
        request = request.body(body.clone());

        // 7. Execute request with retry logic
        let mut last_error = None;

        for attempt in 1..=self.config.retry_policy.max_attempts {
            match request.try_clone() {
                Some(req) => {
                    match req.send().await {
                        Ok(response) => {
                            let status = response.status();
                            let response_text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Failed to read response".to_string());

                            if status.is_success() {
                                return Ok(DeliveryResult::success(
                                    format!("webhook:{}", url),
                                    start_time.elapsed(),
                                    body.len() as u64,
                                    Some(format!("HTTP {}: {}", status, response_text)),
                                ));
                            } else if self.should_retry_status(status.as_u16())
                                && attempt < self.config.retry_policy.max_attempts
                            {
                                last_error = Some(DeliveryError::WebhookFailed {
                                    url: url.clone(),
                                    status: status.as_u16(),
                                    response: response_text,
                                });

                                // Wait before retry
                                let delay = self.calculate_retry_delay(attempt);
                                tokio::time::sleep(delay).await;
                                continue;
                            } else {
                                return Err(DeliveryError::WebhookFailed {
                                    url,
                                    status: status.as_u16(),
                                    response: response_text,
                                });
                            }
                        }
                        Err(e) => {
                            last_error = Some(DeliveryError::Network {
                                url: url.clone(),
                                error: e.to_string(),
                            });

                            if attempt < self.config.retry_policy.max_attempts {
                                let delay = self.calculate_retry_delay(attempt);
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                        }
                    }
                }
                None => {
                    return Err(DeliveryError::RequestClone);
                }
            }
        }

        Err(last_error.unwrap_or(DeliveryError::MaxRetriesExceeded {
            destination: url,
            attempts: self.config.retry_policy.max_attempts,
        }))
    }

    fn validate_config(&self) -> Result<(), ValidationError> {
        // Validate URL template
        if self.config.url_template.is_empty() {
            return Err(ValidationError::EmptyUrl);
        }

        // Validate URL format (basic check)
        if !self.config.url_template.starts_with("http://")
            && !self.config.url_template.starts_with("https://")
            && !self
                .template_engine
                .has_variables(&self.config.url_template)
        {
            return Err(ValidationError::InvalidTemplate(
                "URL must start with http:// or https:// or contain template variables".into(),
            ));
        }

        // Validate template
        self.template_engine
            .validate(&self.config.url_template)
            .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;

        // Validate headers
        for (key, value_template) in &self.config.headers {
            if key.is_empty() {
                return Err(ValidationError::EmptyHeaderName);
            }
            self.template_engine
                .validate(value_template)
                .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
        }

        // Validate timeout
        if self.config.timeout < Duration::from_secs(1)
            || self.config.timeout > Duration::from_secs(300)
        {
            return Err(ValidationError::InvalidTimeout);
        }

        // Validate retry policy
        if self.config.retry_policy.max_attempts == 0 {
            return Err(ValidationError::InvalidRetryPolicy {
                reason: "max_attempts must be greater than 0".to_string(),
            });
        }

        if self.config.retry_policy.initial_delay > self.config.retry_policy.max_delay {
            return Err(ValidationError::InvalidRetryPolicy {
                reason: "initial_delay cannot be greater than max_delay".to_string(),
            });
        }

        if self.config.retry_policy.backoff_multiplier <= 0.0 {
            return Err(ValidationError::InvalidRetryPolicy {
                reason: "backoff_multiplier must be greater than 0".to_string(),
            });
        }

        // Validate auth templates if present
        if let Some(auth) = &self.config.auth {
            match auth {
                WebhookAuth::Bearer { token } => {
                    self.template_engine
                        .validate(token)
                        .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
                }
                WebhookAuth::Basic { username, password } => {
                    self.template_engine
                        .validate(username)
                        .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
                    self.template_engine
                        .validate(password)
                        .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
                }
                WebhookAuth::ApiKey { key, .. } => {
                    self.template_engine
                        .validate(key)
                        .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;
                }
                WebhookAuth::Signature { secret, algorithm } => {
                    self.template_engine
                        .validate(secret)
                        .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;

                    // Validate supported algorithms
                    let supported_algorithms = ["hmac-sha256", "hmac-sha1"];
                    if !supported_algorithms.contains(&algorithm.to_lowercase().as_str()) {
                        return Err(ValidationError::InvalidRetryPolicy {
                            reason: format!("Unsupported signature algorithm: {}", algorithm),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn destination_type(&self) -> &'static str {
        "webhook"
    }

    fn estimated_delivery_time(&self) -> Duration {
        self.config.timeout + Duration::from_millis(100) // Add small buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_output() -> TaskOutput {
        TaskOutput {
            job_id: 123,
            task_id: 456,
            execution_id: 789,
            output_data: serde_json::json!({
                "result": "success",
                "data": {"temperature": 20.5}
            }),
            metadata: HashMap::new(),
            completed_at: chrono::Utc::now(),
            execution_duration: Duration::from_secs(5),
        }
    }

    fn create_test_context() -> DeliveryContext {
        let mut template_vars = HashMap::new();
        template_vars.insert("job_id".to_string(), "123".to_string());
        template_vars.insert("env".to_string(), "test".to_string());

        DeliveryContext {
            job_id: 123,
            task_name: "test-task".to_string(),
            task_version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now(),
            environment: "test".to_string(),
            trace_id: "trace-123".to_string(),
            template_variables: template_vars,
        }
    }

    #[tokio::test]
    async fn test_successful_webhook_delivery() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&mock_server)
            .await;

        let config = WebhookConfig {
            url_template: format!("{}/webhook", mock_server.uri()),
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auth: None,
            content_type: Some("application/json".to_string()),
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(config, client, TemplateEngine::new());

        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();

        assert!(result.success);
        assert!(result.size_bytes > 0);
        assert!(result.response_info.is_some());
        assert!(result.response_info.unwrap().contains("HTTP 200"));
    }

    #[tokio::test]
    async fn test_webhook_with_bearer_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = WebhookConfig {
            url_template: format!("{}/webhook", mock_server.uri()),
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auth: Some(WebhookAuth::Bearer {
                token: "test-token".to_string(),
            }),
            content_type: None,
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(config, client, TemplateEngine::new());

        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_webhook_retry_on_500() {
        let mock_server = MockServer::start().await;

        // First call returns 500, second call returns 200
        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let mut retry_policy = RetryPolicy::default();
        retry_policy.max_attempts = 2;
        retry_policy.initial_delay = Duration::from_millis(10); // Fast retry for test

        let config = WebhookConfig {
            url_template: format!("{}/webhook", mock_server.uri()),
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy,
            auth: None,
            content_type: None,
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(config, client, TemplateEngine::new());

        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_webhook_template_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook/test/123"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = WebhookConfig {
            url_template: format!("{}/webhook/{{{{env}}}}/{{{{job_id}}}}", mock_server.uri()),
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auth: None,
            content_type: None,
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(config, client, TemplateEngine::new());

        let output = create_test_output();
        let context = create_test_context();

        let result = destination.deliver(&output, &context).await.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_webhook_validation() {
        let valid_config = WebhookConfig {
            url_template: "https://example.com/webhook".to_string(),
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auth: None,
            content_type: None,
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(valid_config, client, TemplateEngine::new());
        assert!(destination.validate_config().is_ok());

        let invalid_config = WebhookConfig {
            url_template: "".to_string(), // Empty URL
            method: HttpMethod::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            auth: None,
            content_type: None,
        };

        let client = reqwest::Client::new();
        let destination = WebhookDestination::new(invalid_config, client, TemplateEngine::new());
        assert!(destination.validate_config().is_err());
    }
}
