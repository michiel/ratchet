//! Webhook output destination implementation

use async_trait::async_trait;
use reqwest;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::{
    destination::{DeliveryContext, DeliveryResult, OutputDestination, TaskOutput},
    errors::{DeliveryError, ValidationError},
    template::TemplateEngine,
    HttpMethod, RetryPolicy, WebhookAuth,
};

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
    pub fn new(config: WebhookConfig, client: reqwest::Client, template_engine: TemplateEngine) -> Self {
        Self {
            config,
            client,
            template_engine,
        }
    }

    /// Create a cross-platform HTTP client with appropriate TLS configuration
    pub fn create_default_client() -> Result<reqwest::Client, reqwest::Error> {
        reqwest::Client::builder()
            // Use rustls-tls for cross-platform TLS support
            .use_rustls_tls()
            // Set reasonable timeouts
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            // Enable connection pooling
            // Set a reasonable user agent
            .user_agent("ratchet-output/1.0")
            .build()
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
                let rendered_token = self.template_engine.render(token, &context.template_variables)?;
                request = request.bearer_auth(rendered_token);
            }
            WebhookAuth::Basic { username, password } => {
                let rendered_username = self.template_engine.render(username, &context.template_variables)?;
                let rendered_password = self.template_engine.render(password, &context.template_variables)?;
                request = request.basic_auth(rendered_username, Some(rendered_password));
            }
            WebhookAuth::ApiKey { header, key } => {
                let rendered_key = self.template_engine.render(key, &context.template_variables)?;
                request = request.header(header, rendered_key);
            }
            WebhookAuth::Signature {
                secret: _,
                algorithm: _,
            } => {
                // TODO: Implement HMAC signature
                return Err(DeliveryError::Network {
                    url: "webhook".to_string(),
                    error: "HMAC signature authentication not yet implemented".to_string(),
                });
            }
        }
        Ok(request)
    }

    /// Send HTTP request with retry logic
    async fn send_with_retry(
        &self,
        url: &str,
        payload: &serde_json::Value,
        context: &DeliveryContext,
    ) -> Result<(Duration, String), DeliveryError> {
        let mut attempt = 0;
        let mut delay = self.config.retry_policy.initial_delay;
        let start_time = Instant::now();

        loop {
            attempt += 1;

            // Build request
            let mut request = match self.config.method {
                HttpMethod::Get => self.client.get(url),
                HttpMethod::Post => self.client.post(url),
                HttpMethod::Put => self.client.put(url),
                HttpMethod::Patch => self.client.patch(url),
                HttpMethod::Delete => self.client.delete(url),
                HttpMethod::Head => self.client.head(url),
                HttpMethod::Options => self.client.request(reqwest::Method::OPTIONS, url),
            };

            // Add headers
            for (name, value_template) in &self.config.headers {
                let rendered_value = self
                    .template_engine
                    .render(value_template, &context.template_variables)?;
                request = request.header(name, rendered_value);
            }

            // Set content type
            if let Some(content_type) = &self.config.content_type {
                request = request.header("Content-Type", content_type);
            } else {
                request = request.header("Content-Type", "application/json");
            }

            // Add authentication
            if let Some(auth) = &self.config.auth {
                request = self.add_auth(request, auth, context)?;
            }

            // Add payload for non-GET requests
            if self.config.method != HttpMethod::Get {
                request = request.json(payload);
            }

            // Set timeout
            request = request.timeout(self.config.timeout);

            // Send request
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    let response_text = response.text().await.unwrap_or_default();

                    if status.is_success() {
                        let total_time = start_time.elapsed();
                        return Ok((total_time, response_text));
                    } else if self.config.retry_policy.retry_on_status.contains(&status.as_u16()) {
                        // Retry on specific status codes
                        if attempt >= self.config.retry_policy.max_attempts {
                            return Err(DeliveryError::MaxRetriesExceeded {
                                destination: "webhook".to_string(),
                                attempts: attempt,
                            });
                        }

                        tracing::warn!(
                            "Webhook request failed with status {}, attempt {}/{}, retrying in {:?}",
                            status,
                            attempt,
                            self.config.retry_policy.max_attempts,
                            delay
                        );
                    } else {
                        // Don't retry on other status codes
                        return Err(DeliveryError::WebhookFailed {
                            url: url.to_string(),
                            status: status.as_u16(),
                            response: response_text,
                        });
                    }
                }
                Err(e) => {
                    if attempt >= self.config.retry_policy.max_attempts {
                        return Err(DeliveryError::Network {
                            url: url.to_string(),
                            error: e.to_string(),
                        });
                    }

                    tracing::warn!(
                        "Webhook request failed with error: {}, attempt {}/{}, retrying in {:?}",
                        e,
                        attempt,
                        self.config.retry_policy.max_attempts,
                        delay
                    );
                }
            }

            // Wait before retry
            tokio::time::sleep(delay).await;

            // Calculate next delay with exponential backoff
            delay =
                Duration::from_millis((delay.as_millis() as f64 * self.config.retry_policy.backoff_multiplier) as u64);
            if delay > self.config.retry_policy.max_delay {
                delay = self.config.retry_policy.max_delay;
            }

            // Add jitter if enabled
            if self.config.retry_policy.jitter {
                use rand::Rng;
                let jitter_ms = rand::thread_rng().gen_range(0..=delay.as_millis() / 10) as u64;
                delay += Duration::from_millis(jitter_ms);
            }
        }
    }
}

#[async_trait]
impl OutputDestination for WebhookDestination {
    async fn deliver(&self, output: &TaskOutput, context: &DeliveryContext) -> Result<DeliveryResult, DeliveryError> {
        let _start_time = Instant::now();

        // Render the URL template
        let rendered_url = self
            .template_engine
            .render(&self.config.url_template, &context.template_variables)?;

        // Send the request
        let (delivery_time, response) = self
            .send_with_retry(&rendered_url, &output.output_data, context)
            .await?;

        let size_bytes = serde_json::to_vec(&output.output_data)
            .map(|v| v.len() as u64)
            .unwrap_or(0);

        Ok(DeliveryResult::success(
            "webhook".to_string(),
            delivery_time,
            size_bytes,
            Some(response),
        ))
    }

    fn validate_config(&self) -> Result<(), ValidationError> {
        if self.config.url_template.is_empty() {
            return Err(ValidationError::EmptyUrl);
        }

        // Validate URL template syntax
        self.template_engine
            .validate(&self.config.url_template)
            .map_err(|e| ValidationError::InvalidTemplate(e.to_string()))?;

        // Validate timeout
        if self.config.timeout.as_secs() == 0 || self.config.timeout.as_secs() > 300 {
            return Err(ValidationError::InvalidTimeout);
        }

        // Validate retry policy
        if self.config.retry_policy.max_attempts == 0 {
            return Err(ValidationError::InvalidRetryPolicy {
                reason: "max_attempts must be greater than 0".to_string(),
            });
        }

        if self.config.retry_policy.initial_delay.as_secs() == 0 {
            return Err(ValidationError::InvalidRetryPolicy {
                reason: "initial_delay must be greater than 0".to_string(),
            });
        }

        // Validate headers
        for name in self.config.headers.keys() {
            if name.is_empty() {
                return Err(ValidationError::EmptyHeaderName);
            }
        }

        Ok(())
    }

    fn destination_type(&self) -> &'static str {
        "webhook"
    }

    fn supports_retry(&self) -> bool {
        true
    }

    fn estimated_delivery_time(&self) -> Duration {
        self.config.timeout + self.config.retry_policy.max_delay
    }
}
