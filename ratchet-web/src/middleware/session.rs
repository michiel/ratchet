//! Session management middleware with automatic timeout and cleanup

use axum::{
    http::Request,
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, error};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::middleware::{AuthContext, AuditLogger, TracingAuditLogger, AuditEvent, AuditEventType, AuditSeverity};

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Enable session management
    pub enabled: bool,
    /// Session timeout duration (default: 24 hours)
    pub session_timeout: ChronoDuration,
    /// Cleanup interval for expired sessions (default: 1 hour)
    pub cleanup_interval: Duration,
    /// Maximum number of sessions per user (default: 5)
    pub max_sessions_per_user: usize,
    /// Maximum total sessions (default: 10000)
    pub max_total_sessions: usize,
    /// Enable session extension on activity
    pub extend_on_activity: bool,
    /// Minimum time between session extensions (default: 5 minutes)
    pub extension_threshold: ChronoDuration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            session_timeout: ChronoDuration::hours(24),
            cleanup_interval: Duration::from_secs(3600), // 1 hour
            max_sessions_per_user: 5,
            max_total_sessions: 10000,
            extend_on_activity: true,
            extension_threshold: ChronoDuration::minutes(5),
        }
    }
}

impl SessionConfig {
    /// Create a production configuration with shorter timeouts
    pub fn production() -> Self {
        Self {
            enabled: true,
            session_timeout: ChronoDuration::hours(8), // 8 hour timeout
            cleanup_interval: Duration::from_secs(1800), // 30 minutes
            max_sessions_per_user: 3,
            max_total_sessions: 50000,
            extend_on_activity: true,
            extension_threshold: ChronoDuration::minutes(10),
        }
    }
    
    /// Create a development configuration with longer timeouts
    pub fn development() -> Self {
        Self {
            enabled: true,
            session_timeout: ChronoDuration::hours(72), // 3 day timeout
            cleanup_interval: Duration::from_secs(7200), // 2 hours
            max_sessions_per_user: 10,
            max_total_sessions: 1000,
            extend_on_activity: true,
            extension_threshold: ChronoDuration::minutes(1),
        }
    }
    
    /// Create a strict configuration for high-security environments
    pub fn strict() -> Self {
        Self {
            enabled: true,
            session_timeout: ChronoDuration::hours(2), // 2 hour timeout
            cleanup_interval: Duration::from_secs(600), // 10 minutes
            max_sessions_per_user: 1, // Single session per user
            max_total_sessions: 10000,
            extend_on_activity: false, // No extension on activity
            extension_threshold: ChronoDuration::minutes(1),
        }
    }
    
    /// Disable session management
    pub fn disabled() -> Self {
        let mut config = Self::default();
        config.enabled = false;
        config
    }
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// User ID
    pub user_id: String,
    /// User role
    pub role: String,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// Session expiry time
    pub expires_at: DateTime<Utc>,
    /// IP address when session was created
    pub ip_address: Option<String>,
    /// User agent when session was created
    pub user_agent: Option<String>,
    /// Whether session is still active
    pub is_active: bool,
}

impl SessionInfo {
    /// Create a new session
    pub fn new(user_id: String, role: String, timeout: ChronoDuration) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            user_id,
            role,
            created_at: now,
            last_activity: now,
            expires_at: now + timeout,
            ip_address: None,
            user_agent: None,
            is_active: true,
        }
    }
    
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at || !self.is_active
    }
    
    /// Extend session expiry time
    pub fn extend(&mut self, timeout: ChronoDuration) {
        let now = Utc::now();
        self.last_activity = now;
        self.expires_at = now + timeout;
    }
    
    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }
    
    /// Invalidate session
    pub fn invalidate(&mut self) {
        self.is_active = false;
    }
    
    /// Get time until expiry
    pub fn time_until_expiry(&self) -> Option<ChronoDuration> {
        if self.is_expired() {
            None
        } else {
            Some(self.expires_at - Utc::now())
        }
    }
    
    /// Add request context to session
    pub fn with_request_context(mut self, ip_address: Option<String>, user_agent: Option<String>) -> Self {
        self.ip_address = ip_address;
        self.user_agent = user_agent;
        self
    }
}

/// Session statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub expired_sessions: usize,
    pub sessions_by_user: HashMap<String, usize>,
    pub average_session_duration: Option<ChronoDuration>,
    pub sessions_created_today: usize,
    pub sessions_expired_today: usize,
}

/// Session manager with automatic cleanup
pub struct SessionManager {
    config: SessionConfig,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    user_sessions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionConfig) -> Self {
        let sessions = Arc::new(RwLock::new(HashMap::new()));
        let user_sessions = Arc::new(RwLock::new(HashMap::new()));
        
        let mut manager = Self {
            config,
            sessions,
            user_sessions,
            cleanup_handle: None,
        };
        
        if manager.config.enabled {
            manager.start_cleanup_task();
        }
        
        manager
    }
    
    /// Start automatic cleanup task
    fn start_cleanup_task(&mut self) {
        let sessions = self.sessions.clone();
        let user_sessions = self.user_sessions.clone();
        let cleanup_interval = self.config.cleanup_interval;
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                let cleanup_result = Self::cleanup_expired_sessions(&sessions, &user_sessions).await;
                match cleanup_result {
                    Ok(cleaned_count) => {
                        if cleaned_count > 0 {
                            info!("Session cleanup completed: removed {} expired sessions", cleaned_count);
                        } else {
                            debug!("Session cleanup completed: no expired sessions found");
                        }
                    }
                    Err(e) => {
                        error!("Session cleanup failed: {}", e);
                    }
                }
            }
        });
        
        self.cleanup_handle = Some(handle);
    }
    
    /// Create a new session
    pub async fn create_session(
        &self,
        user_id: String,
        role: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SessionInfo, SessionError> {
        if !self.config.enabled {
            return Err(SessionError::SessionManagementDisabled);
        }
        
        // Check total session limit
        {
            let sessions = self.sessions.read().await;
            if sessions.len() >= self.config.max_total_sessions {
                return Err(SessionError::TotalSessionLimitExceeded);
            }
        }
        
        // Check per-user session limit and cleanup old sessions if needed
        {
            let mut user_sessions = self.user_sessions.write().await;
            let user_session_ids = user_sessions.entry(user_id.clone()).or_insert_with(Vec::new);
            
            if user_session_ids.len() >= self.config.max_sessions_per_user {
                // Remove oldest session for this user
                if !user_session_ids.is_empty() {
                    let old_session_id = user_session_ids.remove(0);
                    let mut sessions = self.sessions.write().await;
                    sessions.remove(&old_session_id);
                    info!("Removed oldest session for user {} to enforce limit", user_id);
                }
            }
        }
        
        // Create new session
        let session = SessionInfo::new(user_id.clone(), role, self.config.session_timeout)
            .with_request_context(ip_address, user_agent);
        
        let session_id = session.session_id.clone();
        
        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }
        
        // Update user session tracking
        {
            let mut user_sessions = self.user_sessions.write().await;
            let user_session_ids = user_sessions.entry(user_id).or_insert_with(Vec::new);
            user_session_ids.push(session_id);
        }
        
        info!("Created new session: {}", session.session_id);
        Ok(session)
    }
    
    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }
    
    /// Validate and update session
    pub async fn validate_session(&self, session_id: &str) -> Result<SessionInfo, SessionError> {
        if !self.config.enabled {
            return Err(SessionError::SessionManagementDisabled);
        }
        
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id)
            .ok_or(SessionError::SessionNotFound)?;
        
        if session.is_expired() {
            // Remove expired session
            sessions.remove(session_id);
            return Err(SessionError::SessionExpired);
        }
        
        // Update activity and extend if configured
        if self.config.extend_on_activity {
            let now = Utc::now();
            let time_since_extension = now - session.last_activity;
            
            if time_since_extension > self.config.extension_threshold {
                session.extend(self.config.session_timeout);
                debug!("Extended session: {}", session_id);
            } else {
                session.update_activity();
            }
        } else {
            session.update_activity();
        }
        
        Ok(session.clone())
    }
    
    /// Invalidate session
    pub async fn invalidate_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id)
            .ok_or(SessionError::SessionNotFound)?;
        
        session.invalidate();
        let user_id = session.user_id.clone();
        
        // Remove from session storage
        sessions.remove(session_id);
        
        // Remove from user session tracking
        let mut user_sessions = self.user_sessions.write().await;
        if let Some(user_session_ids) = user_sessions.get_mut(&user_id) {
            user_session_ids.retain(|id| id != session_id);
            if user_session_ids.is_empty() {
                user_sessions.remove(&user_id);
            }
        }
        
        info!("Invalidated session: {}", session_id);
        Ok(())
    }
    
    /// Get all sessions for a user
    pub async fn get_user_sessions(&self, user_id: &str) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let user_sessions = self.user_sessions.read().await;
        
        if let Some(session_ids) = user_sessions.get(user_id) {
            session_ids.iter()
                .filter_map(|id| sessions.get(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Invalidate all sessions for a user
    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<usize, SessionError> {
        let session_ids: Vec<String> = {
            let user_sessions = self.user_sessions.read().await;
            user_sessions.get(user_id)
                .map(|ids| ids.clone())
                .unwrap_or_default()
        };
        
        let mut count = 0;
        for session_id in session_ids {
            if self.invalidate_session(&session_id).await.is_ok() {
                count += 1;
            }
        }
        
        info!("Invalidated {} sessions for user: {}", count, user_id);
        Ok(count)
    }
    
    /// Get session statistics
    pub async fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.read().await;
        let user_sessions = self.user_sessions.read().await;
        
        let total_sessions = sessions.len();
        let active_sessions = sessions.values().filter(|s| !s.is_expired()).count();
        let expired_sessions = total_sessions - active_sessions;
        
        let sessions_by_user: HashMap<String, usize> = user_sessions.iter()
            .map(|(user_id, session_ids)| (user_id.clone(), session_ids.len()))
            .collect();
        
        let today = Utc::now().date_naive();
        let sessions_created_today = sessions.values()
            .filter(|s| s.created_at.date_naive() == today)
            .count();
        
        // Calculate average session duration for completed sessions
        let completed_sessions: Vec<_> = sessions.values()
            .filter(|s| s.is_expired())
            .collect();
        
        let average_session_duration = if !completed_sessions.is_empty() {
            let total_duration: ChronoDuration = completed_sessions.iter()
                .map(|s| s.last_activity - s.created_at)
                .sum();
            Some(total_duration / completed_sessions.len() as i32)
        } else {
            None
        };
        
        SessionStats {
            total_sessions,
            active_sessions,
            expired_sessions,
            sessions_by_user,
            average_session_duration,
            sessions_created_today,
            sessions_expired_today: expired_sessions,
        }
    }
    
    /// Manual cleanup of expired sessions
    pub async fn cleanup_expired(&self) -> Result<usize, SessionError> {
        Self::cleanup_expired_sessions(&self.sessions, &self.user_sessions).await
            .map_err(SessionError::CleanupFailed)
    }
    
    /// Internal cleanup implementation
    async fn cleanup_expired_sessions(
        sessions: &Arc<RwLock<HashMap<String, SessionInfo>>>,
        user_sessions: &Arc<RwLock<HashMap<String, Vec<String>>>>,
    ) -> Result<usize, String> {
        let expired_session_ids: Vec<String> = {
            let sessions_read = sessions.read().await;
            sessions_read.iter()
                .filter(|(_, session)| session.is_expired())
                .map(|(id, _)| id.clone())
                .collect()
        };
        
        let count = expired_session_ids.len();
        
        if count > 0 {
            // Remove expired sessions
            {
                let mut sessions_write = sessions.write().await;
                for session_id in &expired_session_ids {
                    sessions_write.remove(session_id);
                }
            }
            
            // Update user session tracking
            {
                let mut user_sessions_write = user_sessions.write().await;
                for session_id in &expired_session_ids {
                    // Find and remove session ID from user tracking
                    let mut users_to_remove = Vec::new();
                    for (user_id, session_ids) in user_sessions_write.iter_mut() {
                        session_ids.retain(|id| id != session_id);
                        if session_ids.is_empty() {
                            users_to_remove.push(user_id.clone());
                        }
                    }
                    // Remove users with no sessions
                    for user_id in users_to_remove {
                        user_sessions_write.remove(&user_id);
                    }
                }
            }
        }
        
        Ok(count)
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        if let Some(handle) = &self.cleanup_handle {
            handle.abort();
        }
    }
}

/// Session management errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session management is disabled")]
    SessionManagementDisabled,
    #[error("Session not found")]
    SessionNotFound,
    #[error("Session has expired")]
    SessionExpired,
    #[error("Total session limit exceeded")]
    TotalSessionLimitExceeded,
    #[error("Per-user session limit exceeded")]
    UserSessionLimitExceeded,
    #[error("Cleanup failed: {0}")]
    CleanupFailed(String),
}

/// Session middleware for automatic session management
pub async fn session_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Extract session manager from request extensions
    let session_manager = request.extensions().get::<Arc<SessionManager>>();
    
    if let Some(manager) = session_manager {
        // Try to extract session ID from Authorization header or cookies
        if let Some(session_id) = extract_session_id(&request) {
            match manager.validate_session(&session_id).await {
                Ok(session) => {
                    // Create auth context from session
                    let auth_context = AuthContext::authenticated(
                        session.user_id.clone(),
                        session.role.clone(),
                        session.session_id.clone(),
                    );
                    
                    // Add auth context to request
                    let mut request = request;
                    request.extensions_mut().insert(auth_context);
                    
                    // Log session activity if audit config is available
                    if let Some(audit_config) = request.extensions().get::<crate::middleware::AuditConfig>() {
                        let logger = TracingAuditLogger::new(audit_config.clone());
                        let event = AuditEvent::new(
                            AuditEventType::AccessGranted,
                            AuditSeverity::Info,
                            format!("Session validated: {}", session_id),
                        ).with_user(session.user_id, Some(session.session_id));
                        logger.log_event(event);
                    }
                    
                    return next.run(request).await;
                }
                Err(SessionError::SessionExpired) => {
                    // Log expired session attempt
                    if let Some(audit_config) = request.extensions().get::<crate::middleware::AuditConfig>() {
                        let logger = TracingAuditLogger::new(audit_config.clone());
                        let event = AuditEvent::new(
                            AuditEventType::AuthFailed,
                            AuditSeverity::Warning,
                            format!("Expired session used: {}", session_id),
                        );
                        logger.log_event(event);
                    }
                }
                Err(_) => {
                    // Log invalid session attempt
                    if let Some(audit_config) = request.extensions().get::<crate::middleware::AuditConfig>() {
                        let logger = TracingAuditLogger::new(audit_config.clone());
                        let event = AuditEvent::new(
                            AuditEventType::AuthFailed,
                            AuditSeverity::Warning,
                            format!("Invalid session used: {}", session_id),
                        );
                        logger.log_event(event);
                    }
                }
            }
        }
    }
    
    // No valid session found, continue without authentication
    next.run(request).await
}

/// Extract session ID from request headers or cookies
fn extract_session_id(request: &Request<axum::body::Body>) -> Option<String> {
    // Try Authorization header first (Bearer token format)
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    
    // Try X-Session-ID header
    if let Some(session_header) = request.headers().get("x-session-id") {
        if let Ok(session_id) = session_header.to_str() {
            return Some(session_id.to_string());
        }
    }
    
    // Try cookie (simplified cookie parsing)
    if let Some(cookie_header) = request.headers().get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(session_id) = cookie.strip_prefix("session_id=") {
                    return Some(session_id.to_string());
                }
            }
        }
    }
    
    None
}

/// Create session management layer
pub fn session_layer(_config: SessionConfig) {
    // Config will be passed directly to the middleware when applied
    // This is a placeholder function for the public API
}

/// Create session manager for middleware
pub fn create_session_manager(config: SessionConfig) -> Arc<SessionManager> {
    Arc::new(SessionManager::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_session_creation() {
        let config = SessionConfig::development();
        let manager = SessionManager::new(config);
        
        let session = manager.create_session(
            "user123".to_string(),
            "user".to_string(),
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        ).await.unwrap();
        
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.role, "user");
        assert!(!session.is_expired());
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));
    }
    
    #[tokio::test]
    async fn test_session_validation() {
        let config = SessionConfig::development();
        let manager = SessionManager::new(config);
        
        let session = manager.create_session(
            "user123".to_string(),
            "user".to_string(),
            None,
            None,
        ).await.unwrap();
        
        let session_id = session.session_id.clone();
        
        // Validate session
        let validated = manager.validate_session(&session_id).await.unwrap();
        assert_eq!(validated.user_id, "user123");
        assert!(!validated.is_expired());
    }
    
    #[tokio::test]
    async fn test_session_expiry() {
        let mut config = SessionConfig::development();
        config.session_timeout = ChronoDuration::milliseconds(100);
        let manager = SessionManager::new(config);
        
        let session = manager.create_session(
            "user123".to_string(),
            "user".to_string(),
            None,
            None,
        ).await.unwrap();
        
        let session_id = session.session_id.clone();
        
        // Wait for session to expire
        sleep(TokioDuration::from_millis(150)).await;
        
        // Validation should fail
        let result = manager.validate_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::SessionExpired)));
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let mut config = SessionConfig::development();
        config.session_timeout = ChronoDuration::milliseconds(50);
        config.cleanup_interval = TokioDuration::from_millis(100);
        let manager = SessionManager::new(config);
        
        // Create a session that will expire
        let _session = manager.create_session(
            "user123".to_string(),
            "user".to_string(),
            None,
            None,
        ).await.unwrap();
        
        // Wait for expiry and cleanup
        sleep(TokioDuration::from_millis(200)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_sessions, 0); // Should be cleaned up
    }
    
    #[tokio::test]
    async fn test_user_session_limit() {
        let mut config = SessionConfig::development();
        config.max_sessions_per_user = 2;
        let manager = SessionManager::new(config);
        
        // Create maximum sessions for user
        let _session1 = manager.create_session("user123".to_string(), "user".to_string(), None, None).await.unwrap();
        let _session2 = manager.create_session("user123".to_string(), "user".to_string(), None, None).await.unwrap();
        
        // Third session should remove oldest
        let _session3 = manager.create_session("user123".to_string(), "user".to_string(), None, None).await.unwrap();
        
        let user_sessions = manager.get_user_sessions("user123").await;
        assert_eq!(user_sessions.len(), 2);
    }
    
    #[tokio::test]
    async fn test_session_invalidation() {
        let config = SessionConfig::development();
        let manager = SessionManager::new(config);
        
        let session = manager.create_session(
            "user123".to_string(),
            "user".to_string(),
            None,
            None,
        ).await.unwrap();
        
        let session_id = session.session_id.clone();
        
        // Invalidate session
        manager.invalidate_session(&session_id).await.unwrap();
        
        // Validation should fail
        let result = manager.validate_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::SessionNotFound)));
    }
}