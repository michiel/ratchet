//! Security middleware for headers, content validation, and request safety

use axum::{
    body::Body,
    http::{header, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::time::Duration;

use crate::errors::ApiError;

/// Security headers middleware
pub async fn security_headers(request: Request<Body>, next: Next<Body>) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // HSTS - Force HTTPS for 1 year
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    
    // Content Security Policy - Restrictive policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'"
        ),
    );
    
    // X-Frame-Options - Prevent clickjacking
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    
    // X-Content-Type-Options - Prevent MIME sniffing
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    
    // X-XSS-Protection - Enable XSS filtering
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    
    // Referrer Policy - Control referrer information
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    
    // Permissions Policy - Control browser features
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
    );
    
    // Cross-Origin-Embedder-Policy
    headers.insert(
        HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    );
    
    // Cross-Origin-Opener-Policy
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    
    // Cross-Origin-Resource-Policy
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    
    response
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_hsts: bool,
    pub hsts_max_age: Duration,
    pub csp_policy: Option<String>,
    pub enable_frame_options: bool,
    pub frame_options: FrameOptions,
    pub enable_content_type_options: bool,
    pub enable_xss_protection: bool,
    pub referrer_policy: ReferrerPolicy,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: Duration::from_secs(31536000), // 1 year
            csp_policy: Some(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
                    .to_string(),
            ),
            enable_frame_options: true,
            frame_options: FrameOptions::Deny,
            enable_content_type_options: true,
            enable_xss_protection: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FrameOptions {
    Deny,
    SameOrigin,
    AllowFrom(String),
}

impl FrameOptions {
    fn as_header_value(&self) -> HeaderValue {
        match self {
            FrameOptions::Deny => HeaderValue::from_static("DENY"),
            FrameOptions::SameOrigin => HeaderValue::from_static("SAMEORIGIN"),
            FrameOptions::AllowFrom(origin) => {
                HeaderValue::from_str(&format!("ALLOW-FROM {}", origin))
                    .unwrap_or_else(|_| HeaderValue::from_static("DENY"))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    fn as_header_value(&self) -> HeaderValue {
        let value = match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        };
        HeaderValue::from_static(value)
    }
}

/// Configurable security headers middleware
pub struct SecurityHeaders {
    config: SecurityConfig,
}

impl SecurityHeaders {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }
    
    pub fn default() -> Self {
        Self::new(SecurityConfig::default())
    }
    
    pub async fn middleware(&self, request: Request<Body>, next: Next<Body>) -> Response {
        let mut response = next.run(request).await;
        let headers = response.headers_mut();
        
        if self.config.enable_hsts {
            headers.insert(
                header::STRICT_TRANSPORT_SECURITY,
                HeaderValue::from_str(&format!(
                    "max-age={}; includeSubDomains",
                    self.config.hsts_max_age.as_secs()
                ))
                .unwrap_or_else(|_| HeaderValue::from_static("max-age=31536000; includeSubDomains")),
            );
        }
        
        if let Some(ref csp) = self.config.csp_policy {
            headers.insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_str(csp)
                    .unwrap_or_else(|_| HeaderValue::from_static("default-src 'self'")),
            );
        }
        
        if self.config.enable_frame_options {
            headers.insert(
                HeaderName::from_static("x-frame-options"),
                self.config.frame_options.as_header_value(),
            );
        }
        
        if self.config.enable_content_type_options {
            headers.insert(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            );
        }
        
        if self.config.enable_xss_protection {
            headers.insert(
                HeaderName::from_static("x-xss-protection"),
                HeaderValue::from_static("1; mode=block"),
            );
        }
        
        headers.insert(
            header::REFERRER_POLICY,
            self.config.referrer_policy.as_header_value(),
        );
        
        response
    }
}

/// Content validation middleware
pub async fn content_validator(request: Request<Body>, next: Next<Body>) -> Result<Response, ApiError> {
    // Check content length
    if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                const MAX_CONTENT_LENGTH: usize = 10 * 1024 * 1024; // 10MB
                if length > MAX_CONTENT_LENGTH {
                    return Err(ApiError::bad_request("Request body too large"));
                }
            }
        }
    }
    
    // Validate content type for POST/PUT requests
    let method = request.method();
    if method == axum::http::Method::POST || method == axum::http::Method::PUT {
        if let Some(content_type) = request.headers().get(header::CONTENT_TYPE) {
            let content_type_str = content_type.to_str()
                .map_err(|_| ApiError::bad_request("Invalid content-type header"))?;
            
            let allowed_types = [
                "application/json",
                "application/x-www-form-urlencoded",
                "multipart/form-data",
                "text/plain",
            ];
            
            if !allowed_types.iter().any(|&allowed| content_type_str.starts_with(allowed)) {
                return Err(ApiError::bad_request("Unsupported content type"));
            }
        } else {
            return Err(ApiError::bad_request("Content-Type header required"));
        }
    }
    
    Ok(next.run(request).await)
}

/// Rate limiting middleware (basic implementation)
pub struct RateLimiter {
    max_requests: u32,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
    
    pub async fn middleware(&self, request: Request<Body>, next: Next<Body>) -> Result<Response, ApiError> {
        // Basic rate limiting - in production, use Redis or similar
        // For now, just pass through
        // TODO: Implement proper rate limiting with storage backend
        
        Ok(next.run(request).await)
    }
}

/// Content validator utility struct
pub struct ContentValidator {
    max_body_size: usize,
    allowed_content_types: Vec<String>,
}

impl ContentValidator {
    pub fn new(max_body_size: usize, allowed_content_types: Vec<String>) -> Self {
        Self {
            max_body_size,
            allowed_content_types,
        }
    }
    
    pub fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB
            allowed_content_types: vec![
                "application/json".to_string(),
                "application/x-www-form-urlencoded".to_string(),
                "multipart/form-data".to_string(),
                "text/plain".to_string(),
            ],
        }
    }
    
    pub async fn middleware(&self, request: Request<Body>, next: Next<Body>) -> Result<Response, ApiError> {
        // Validate content length
        if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > self.max_body_size {
                        return Err(ApiError::bad_request(format!(
                            "Request body too large. Maximum size: {} bytes",
                            self.max_body_size
                        )));
                    }
                }
            }
        }
        
        // Validate content type for requests with bodies
        let method = request.method();
        if method == axum::http::Method::POST 
            || method == axum::http::Method::PUT 
            || method == axum::http::Method::PATCH 
        {
            if let Some(content_type) = request.headers().get(header::CONTENT_TYPE) {
                let content_type_str = content_type.to_str()
                    .map_err(|_| ApiError::bad_request("Invalid content-type header"))?;
                
                if !self.allowed_content_types.iter().any(|allowed| content_type_str.starts_with(allowed)) {
                    return Err(ApiError::bad_request(format!(
                        "Unsupported content type: {}. Allowed types: {}",
                        content_type_str,
                        self.allowed_content_types.join(", ")
                    )));
                }
            } else {
                return Err(ApiError::bad_request("Content-Type header required for requests with body"));
            }
        }
        
        Ok(next.run(request).await)
    }
}