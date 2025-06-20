//! Security middleware for HTTP headers and TLS configuration

use axum::{
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::time::Duration;

/// Security configuration for HTTP headers and TLS
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable HSTS (HTTP Strict Transport Security)
    pub enable_hsts: bool,
    /// HSTS max age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    pub hsts_preload: bool,

    /// Content Security Policy
    pub csp_policy: Option<String>,

    /// Enable X-Frame-Options
    pub enable_frame_options: bool,
    /// X-Frame-Options value (DENY, SAMEORIGIN, or ALLOW-FROM)
    pub frame_options: String,

    /// Enable X-Content-Type-Options
    pub enable_content_type_options: bool,

    /// Enable X-XSS-Protection
    pub enable_xss_protection: bool,

    /// Enable Referrer Policy
    pub enable_referrer_policy: bool,
    /// Referrer Policy value
    pub referrer_policy: String,

    /// Enable Permissions Policy
    pub enable_permissions_policy: bool,
    /// Permissions Policy value
    pub permissions_policy: Option<String>,

    /// Remove Server header
    pub remove_server_header: bool,

    /// Custom security headers
    pub custom_headers: Vec<(String, String)>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,

            csp_policy: Some(
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com; \
                 style-src 'self' 'unsafe-inline' https://unpkg.com; \
                 img-src 'self' data: https:; \
                 font-src 'self'; \
                 connect-src 'self'; \
                 frame-ancestors 'none'"
                    .to_string(),
            ),

            enable_frame_options: true,
            frame_options: "DENY".to_string(),

            enable_content_type_options: true,
            enable_xss_protection: true,

            enable_referrer_policy: true,
            referrer_policy: "strict-origin-when-cross-origin".to_string(),

            enable_permissions_policy: true,
            permissions_policy: Some(
                "geolocation=(), microphone=(), camera=(), \
                 payment=(), usb=(), magnetometer=(), gyroscope=()"
                    .to_string(),
            ),

            remove_server_header: true,
            custom_headers: vec![],
        }
    }
}

impl SecurityConfig {
    /// Create a permissive configuration for development
    pub fn development() -> Self {
        Self {
            enable_hsts: false, // Don't enforce HTTPS in development
            csp_policy: Some(
                "default-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                 script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com; \
                 style-src 'self' 'unsafe-inline' https://unpkg.com; \
                 img-src 'self' data: https: http:; \
                 connect-src 'self' ws: wss:"
                    .to_string(),
            ),
            frame_options: "SAMEORIGIN".to_string(),
            ..Default::default()
        }
    }

    /// Create a strict configuration for production
    pub fn production() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 63072000, // 2 years
            hsts_include_subdomains: true,
            hsts_preload: true,

            csp_policy: Some(
                "default-src 'self'; \
                 script-src 'self'; \
                 style-src 'self'; \
                 img-src 'self' data:; \
                 font-src 'self'; \
                 connect-src 'self'; \
                 frame-ancestors 'none'; \
                 base-uri 'self'; \
                 form-action 'self'"
                    .to_string(),
            ),

            frame_options: "DENY".to_string(),
            referrer_policy: "no-referrer".to_string(),

            permissions_policy: Some(
                "geolocation=(), microphone=(), camera=(), \
                 payment=(), usb=(), magnetometer=(), gyroscope=(), \
                 accelerometer=(), ambient-light-sensor=()"
                    .to_string(),
            ),

            ..Default::default()
        }
    }
}

/// Security headers middleware
pub async fn security_headers_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    // Get security config from request extensions
    let config = request
        .extensions()
        .get::<SecurityConfig>()
        .cloned()
        .unwrap_or_default();

    // Process the request
    let mut response = next.run(request).await;

    // Add security headers to response
    let headers = response.headers_mut();

    // HSTS (HTTP Strict Transport Security)
    if config.enable_hsts {
        let mut hsts_value = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts_value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            hsts_value.push_str("; preload");
        }
        if let Ok(header_value) = HeaderValue::from_str(&hsts_value) {
            headers.insert("strict-transport-security", header_value);
        }
    }

    // Content Security Policy
    if let Some(ref csp) = config.csp_policy {
        if let Ok(header_value) = HeaderValue::from_str(csp) {
            headers.insert("content-security-policy", header_value);
        }
    }

    // X-Frame-Options
    if config.enable_frame_options {
        if let Ok(header_value) = HeaderValue::from_str(&config.frame_options) {
            headers.insert("x-frame-options", header_value);
        }
    }

    // X-Content-Type-Options
    if config.enable_content_type_options {
        headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    }

    // X-XSS-Protection
    if config.enable_xss_protection {
        headers.insert("x-xss-protection", HeaderValue::from_static("1; mode=block"));
    }

    // Referrer Policy
    if config.enable_referrer_policy {
        if let Ok(header_value) = HeaderValue::from_str(&config.referrer_policy) {
            headers.insert("referrer-policy", header_value);
        }
    }

    // Permissions Policy
    if config.enable_permissions_policy {
        if let Some(ref policy) = config.permissions_policy {
            if let Ok(header_value) = HeaderValue::from_str(policy) {
                headers.insert("permissions-policy", header_value);
            }
        }
    }

    // Remove Server header
    if config.remove_server_header {
        headers.remove("server");
    }

    // Add custom headers
    for (name, value) in &config.custom_headers {
        if let (Ok(header_name), Ok(header_value)) =
            (HeaderName::from_bytes(name.as_bytes()), HeaderValue::from_str(value))
        {
            headers.insert(header_name, header_value);
        }
    }

    response
}

/// TLS configuration for HTTPS
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: String,
    /// Path to private key file
    pub key_path: String,
    /// TLS protocol versions to support
    pub protocols: Vec<TlsProtocol>,
    /// Cipher suites to allow (None = use defaults)
    pub cipher_suites: Option<Vec<String>>,
    /// Require SNI (Server Name Indication)
    pub require_sni: bool,
    /// Session timeout in seconds
    pub session_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsProtocol {
    TlsV12,
    TlsV13,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "/etc/ssl/certs/server.crt".to_string(),
            key_path: "/etc/ssl/private/server.key".to_string(),
            protocols: vec![TlsProtocol::TlsV12, TlsProtocol::TlsV13],
            cipher_suites: None, // Use rustls defaults
            require_sni: false,
            session_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl TlsConfig {
    /// Create a modern TLS configuration (TLS 1.3 only)
    pub fn modern() -> Self {
        Self {
            protocols: vec![TlsProtocol::TlsV13],
            require_sni: true,
            session_timeout: Duration::from_secs(600), // 10 minutes
            ..Default::default()
        }
    }

    /// Create a compatible TLS configuration (TLS 1.2+)
    pub fn compatible() -> Self {
        Self {
            protocols: vec![TlsProtocol::TlsV12, TlsProtocol::TlsV13],
            require_sni: false,
            session_timeout: Duration::from_secs(300), // 5 minutes
            ..Default::default()
        }
    }
}

/// Create security headers layer
/// Note: This is a helper function - actual layer application is done inline in app.rs
pub fn security_headers_layer(_config: SecurityConfig) {
    // Config will be passed directly to the middleware when applied
    // This is a placeholder function for the public API
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "test response"
    }

    #[tokio::test]
    async fn test_security_headers_default() {
        let config = SecurityConfig::default();
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(
                move |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                    let config = config.clone();
                    async move {
                        req.extensions_mut().insert(config);
                        security_headers_middleware(req, next).await
                    }
                },
            ));

        let request = axum::http::Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let headers = response.headers();

        // Check that security headers are present
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-xss-protection"));
        assert!(headers.contains_key("referrer-policy"));
        assert!(headers.contains_key("permissions-policy"));
    }

    #[tokio::test]
    async fn test_security_headers_development() {
        let config = SecurityConfig::development();
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(
                move |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                    let config = config.clone();
                    async move {
                        req.extensions_mut().insert(config);
                        security_headers_middleware(req, next).await
                    }
                },
            ));

        let request = axum::http::Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let headers = response.headers();

        // HSTS should be disabled in development
        assert!(!headers.contains_key("strict-transport-security"));

        // But other headers should still be present
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("x-frame-options"));
    }

    #[test]
    fn test_tls_config_modern() {
        let config = TlsConfig::modern();
        assert_eq!(config.protocols, vec![TlsProtocol::TlsV13]);
        assert!(config.require_sni);
    }

    #[test]
    fn test_tls_config_compatible() {
        let config = TlsConfig::compatible();
        assert_eq!(config.protocols, vec![TlsProtocol::TlsV12, TlsProtocol::TlsV13]);
        assert!(!config.require_sni);
    }
}
