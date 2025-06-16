//! Audit logging middleware for security events

use axum::{
    extract::ConnectInfo,
    http::{Method, Request, Uri},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;
use tracing::{error, info, warn};

use crate::middleware::AuthContext;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication events
    AuthLogin,
    AuthLogout,
    AuthFailed,
    AuthTokenRefresh,
    
    /// Authorization events  
    AccessGranted,
    AccessDenied,
    PermissionEscalation,
    
    /// Resource access events
    ResourceCreated,
    ResourceRead,
    ResourceUpdated,
    ResourceDeleted,
    
    /// Security events
    SecurityViolation,
    RateLimitExceeded,
    SuspiciousActivity,
    DataExfiltration,
    
    /// Administrative events
    ConfigChanged,
    UserCreated,
    UserDeleted,
    RoleChanged,
    
    /// System events
    SystemStart,
    SystemShutdown,
    ErrorOccurred,
}

/// Audit event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// Event severity
    pub severity: AuditSeverity,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// HTTP method
    pub method: Option<String>,
    /// Request URI
    pub uri: Option<String>,
    /// Response status code
    pub status_code: Option<u16>,
    /// Request duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Event description
    pub description: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        description: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            severity,
            user_id: None,
            session_id: None,
            client_ip: None,
            user_agent: None,
            method: None,
            uri: None,
            status_code: None,
            duration_ms: None,
            description,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
    
    /// Add user context to the event
    pub fn with_user(mut self, user_id: String, session_id: Option<String>) -> Self {
        self.user_id = Some(user_id);
        self.session_id = session_id;
        self
    }
    
    /// Add request context to the event
    pub fn with_request(
        mut self,
        method: &Method,
        uri: &Uri,
        client_ip: Option<SocketAddr>,
        user_agent: Option<&str>,
    ) -> Self {
        self.method = Some(method.to_string());
        self.uri = Some(uri.to_string());
        self.client_ip = client_ip.map(|addr| addr.ip().to_string());
        self.user_agent = user_agent.map(|ua| ua.to_string());
        self
    }
    
    /// Add response context to the event
    pub fn with_response(mut self, status_code: u16, duration: Instant) -> Self {
        self.status_code = Some(status_code);
        self.duration_ms = Some(duration.elapsed().as_millis() as u64);
        self
    }
    
    /// Add metadata to the event
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), value);
        }
        self
    }
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log all requests (not just security events)
    pub log_all_requests: bool,
    /// Log request bodies (be careful with sensitive data)
    pub log_request_bodies: bool,
    /// Log response bodies (be careful with sensitive data)
    pub log_response_bodies: bool,
    /// Minimum severity level to log
    pub min_severity: AuditSeverity,
    /// Maximum log entry size in bytes
    pub max_entry_size: usize,
    /// Audit log retention in days
    pub retention_days: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_requests: false,
            log_request_bodies: false,
            log_response_bodies: false,
            min_severity: AuditSeverity::Info,
            max_entry_size: 8192, // 8KB
            retention_days: 90,
        }
    }
}

impl AuditConfig {
    /// Create a production audit configuration
    pub fn production() -> Self {
        Self {
            enabled: true,
            log_all_requests: true,
            log_request_bodies: false, // Don't log bodies in production
            log_response_bodies: false,
            min_severity: AuditSeverity::Info,
            max_entry_size: 4096, // 4KB
            retention_days: 365, // 1 year
        }
    }
    
    /// Create a development audit configuration
    pub fn development() -> Self {
        Self {
            enabled: true,
            log_all_requests: false,
            log_request_bodies: true,
            log_response_bodies: true,
            min_severity: AuditSeverity::Info,
            max_entry_size: 16384, // 16KB
            retention_days: 30,
        }
    }
}

/// Audit logger trait
pub trait AuditLogger: Send + Sync {
    /// Log an audit event
    fn log_event(&self, event: AuditEvent);
    
    /// Log an authentication event
    fn log_auth_event(&self, event_type: AuditEventType, user_id: Option<&str>, description: &str) {
        let mut event = AuditEvent::new(event_type, AuditSeverity::Info, description.to_string());
        if let Some(uid) = user_id {
            event = event.with_user(uid.to_string(), None);
        }
        self.log_event(event);
    }
    
    /// Log a security violation
    fn log_security_violation(&self, description: &str, metadata: serde_json::Value) {
        let event = AuditEvent::new(
            AuditEventType::SecurityViolation,
            AuditSeverity::Warning,
            description.to_string(),
        ).with_metadata("violation_details", metadata);
        self.log_event(event);
    }
}

/// Default audit logger that uses tracing
#[derive(Debug, Clone)]
pub struct TracingAuditLogger {
    config: AuditConfig,
}

impl TracingAuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self { config }
    }
}

impl AuditLogger for TracingAuditLogger {
    fn log_event(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }
        
        // Filter by minimum severity
        let should_log = match (&event.severity, &self.config.min_severity) {
            (AuditSeverity::Critical, _) => true,
            (AuditSeverity::Error, AuditSeverity::Critical) => false,
            (AuditSeverity::Error, _) => true,
            (AuditSeverity::Warning, AuditSeverity::Critical | AuditSeverity::Error) => false,
            (AuditSeverity::Warning, _) => true,
            (AuditSeverity::Info, AuditSeverity::Info) => true,
            (AuditSeverity::Info, _) => false,
        };
        
        if !should_log {
            return;
        }
        
        // Serialize event to JSON
        match serde_json::to_string(&event) {
            Ok(mut json) => {
                // Truncate if too large
                if json.len() > self.config.max_entry_size {
                    json.truncate(self.config.max_entry_size - 3);
                    json.push_str("...");
                }
                
                // Log at appropriate level
                match event.severity {
                    AuditSeverity::Critical => error!(target: "audit", "{}", json),
                    AuditSeverity::Error => error!(target: "audit", "{}", json),
                    AuditSeverity::Warning => warn!(target: "audit", "{}", json),
                    AuditSeverity::Info => info!(target: "audit", "{}", json),
                }
            }
            Err(e) => {
                error!("Failed to serialize audit event: {}", e);
            }
        }
    }
}

/// Audit middleware for request/response logging
pub async fn audit_middleware(
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    
    // Extract request information
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_agent = request.headers().get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let client_ip = connect_info.map(|info| info.0);
    
    // Get audit config and auth context
    let config = request.extensions().get::<AuditConfig>().cloned()
        .unwrap_or_default();
    let auth_context = request.extensions().get::<AuthContext>().cloned();
    
    // Process request
    let response = next.run(request).await;
    let status = response.status();
    
    // Log request if configured
    if config.enabled && (config.log_all_requests || is_security_relevant(&method, &uri, status.as_u16())) {
        let logger = TracingAuditLogger::new(config);
        
        let mut event = AuditEvent::new(
            if status.is_success() { AuditEventType::AccessGranted } else { AuditEventType::AccessDenied },
            if status.is_client_error() || status.is_server_error() { AuditSeverity::Warning } else { AuditSeverity::Info },
            format!("{} {} - {}", method, uri, status),
        )
        .with_request(&method, &uri, client_ip, user_agent.as_deref())
        .with_response(status.as_u16(), start_time);
        
        // Add user context if available
        if let Some(auth) = auth_context {
            if auth.is_authenticated {
                event = event.with_user(auth.user_id, Some(auth.session_id));
            }
        }
        
        logger.log_event(event);
    }
    
    response
}

/// Check if a request is security-relevant
fn is_security_relevant(method: &Method, uri: &Uri, status_code: u16) -> bool {
    // Authentication endpoints
    if uri.path().starts_with("/auth/") {
        return true;
    }
    
    // Admin endpoints
    if uri.path().starts_with("/admin/") {
        return true;
    }
    
    // Mutating operations
    if matches!(method, &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE) {
        return true;
    }
    
    // Failed requests
    if status_code >= 400 {
        return true;
    }
    
    false
}

/// Create audit logging layer
/// Note: This is a helper function - actual layer application is done inline in app.rs
pub fn audit_layer(_config: AuditConfig) {
    // Config will be passed directly to the middleware when applied
    // This is a placeholder function for the public API
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use std::net::{IpAddr, Ipv4Addr};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "test response"
    }

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::AuthLogin,
            AuditSeverity::Info,
            "User logged in".to_string(),
        )
        .with_user("user123".to_string(), Some("session456".to_string()))
        .with_metadata("ip", serde_json::json!("192.168.1.1"));
        
        assert_eq!(event.event_type, AuditEventType::AuthLogin);
        assert_eq!(event.severity, AuditSeverity::Info);
        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.session_id, Some("session456".to_string()));
        assert!(!event.id.is_empty());
    }

    #[tokio::test]
    async fn test_audit_middleware() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(audit_layer(AuditConfig::development()));

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let request = axum::http::Request::builder()
            .uri("/test")
            .extension(ConnectInfo(addr))
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn test_is_security_relevant() {
        assert!(is_security_relevant(&Method::POST, &"/api/v1/tasks".parse().unwrap(), 200));
        assert!(is_security_relevant(&Method::GET, &"/auth/login".parse().unwrap(), 200));
        assert!(is_security_relevant(&Method::GET, &"/api/v1/tasks".parse().unwrap(), 401));
        assert!(!is_security_relevant(&Method::GET, &"/api/v1/tasks".parse().unwrap(), 200));
    }
}