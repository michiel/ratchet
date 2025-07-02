//! Security and authentication management for repository operations
//!
//! This module provides comprehensive security features including credential
//! management, encryption, audit logging, and access control.

pub mod credential_manager;
pub mod encryption;
pub mod audit_logger;
pub mod access_control;

#[cfg(test)]
pub mod tests;

pub use credential_manager::*;
pub use encryption::*;
pub use audit_logger::*;
pub use access_control::*;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Security context for repository operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// User ID performing the operation
    pub user_id: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// IP address of the request
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Request correlation ID
    pub correlation_id: String,
    /// Operation timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional security attributes
    pub attributes: HashMap<String, String>,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(correlation_id: String) -> Self {
        Self {
            user_id: None,
            roles: Vec::new(),
            ip_address: None,
            user_agent: None,
            session_id: None,
            correlation_id,
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        }
    }

    /// Create security context for system operations
    pub fn system() -> Self {
        Self {
            user_id: Some("system".to_string()),
            roles: vec!["system".to_string()],
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("ratchet-server/1.0".to_string()),
            session_id: None,
            correlation_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        }
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Add attribute to security context
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityEventType {
    Authentication,
    Authorization,
    DataAccess,
    Configuration,
    AdminOperation,
    SuspiciousActivity,
    SecurityViolation,
}

/// Security event severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Hash)]
pub enum SecurityEventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Security event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Event severity
    pub severity: SecurityEventSeverity,
    /// Event message
    pub message: String,
    /// Security context
    pub context: SecurityContext,
    /// Repository ID (if applicable)
    pub repository_id: Option<i32>,
    /// Additional event data
    pub data: HashMap<String, serde_json::Value>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(
        event_type: SecurityEventType,
        severity: SecurityEventSeverity,
        message: String,
        context: SecurityContext,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            severity,
            message,
            context,
            repository_id: None,
            data: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add repository ID to the event
    pub fn with_repository(mut self, repository_id: i32) -> Self {
        self.repository_id = Some(repository_id);
        self
    }

    /// Add data to the event
    pub fn with_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.data.insert(key, value);
        self
    }
}

/// Security manager for repository operations
pub struct SecurityManager {
    /// Credential manager
    credential_manager: Arc<CredentialManager>,
    /// Encryption service
    encryption_service: Arc<dyn EncryptionService>,
    /// Audit logger
    audit_logger: Arc<AuditLogger>,
    /// Access control service
    access_control: Arc<AccessControlService>,
    /// Security policies
    policies: Arc<RwLock<SecurityPolicies>>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(
        credential_manager: Arc<CredentialManager>,
        encryption_service: Arc<dyn EncryptionService>,
        audit_logger: Arc<AuditLogger>,
        access_control: Arc<AccessControlService>,
    ) -> Self {
        Self {
            credential_manager,
            encryption_service,
            audit_logger,
            access_control,
            policies: Arc::new(RwLock::new(SecurityPolicies::default())),
        }
    }

    /// Authenticate repository access
    pub async fn authenticate_repository_access(
        &self,
        repository_id: i32,
        context: &SecurityContext,
    ) -> Result<bool> {
        // Log authentication attempt
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecurityEventSeverity::Info,
            format!("Repository authentication attempt for repository {}", repository_id),
            context.clone(),
        ).with_repository(repository_id);
        
        self.audit_logger.log_event(event).await?;

        // Get repository credentials
        let credentials = self.credential_manager
            .get_repository_credentials(repository_id)
            .await?;

        // Validate credentials based on type
        let authenticated = match credentials {
            Some(creds) => self.validate_credentials(&creds, context).await?,
            None => {
                // No credentials required
                true
            }
        };

        // Log authentication result
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            if authenticated { SecurityEventSeverity::Info } else { SecurityEventSeverity::Warning },
            format!(
                "Repository authentication {} for repository {}",
                if authenticated { "succeeded" } else { "failed" },
                repository_id
            ),
            context.clone(),
        ).with_repository(repository_id);
        
        self.audit_logger.log_event(event).await?;

        Ok(authenticated)
    }

    /// Authorize repository operation
    pub async fn authorize_repository_operation(
        &self,
        repository_id: i32,
        operation: &str,
        context: &SecurityContext,
    ) -> Result<bool> {
        // Log authorization attempt
        let event = SecurityEvent::new(
            SecurityEventType::Authorization,
            SecurityEventSeverity::Info,
            format!("Authorization check for operation '{}' on repository {}", operation, repository_id),
            context.clone(),
        ).with_repository(repository_id);
        
        self.audit_logger.log_event(event).await?;

        // Check access control permissions
        let authorized = self.access_control
            .check_permission(context, repository_id, operation)
            .await?;

        // Log authorization result
        let event = SecurityEvent::new(
            SecurityEventType::Authorization,
            if authorized { SecurityEventSeverity::Info } else { SecurityEventSeverity::Warning },
            format!(
                "Authorization {} for operation '{}' on repository {}",
                if authorized { "granted" } else { "denied" },
                operation,
                repository_id
            ),
            context.clone(),
        ).with_repository(repository_id);
        
        self.audit_logger.log_event(event).await?;

        Ok(authorized)
    }

    /// Encrypt sensitive data
    pub async fn encrypt_data(&self, data: &[u8], context: &SecurityContext) -> Result<Vec<u8>> {
        // Log data encryption
        let event = SecurityEvent::new(
            SecurityEventType::DataAccess,
            SecurityEventSeverity::Info,
            format!("Data encryption requested (size: {} bytes)", data.len()),
            context.clone(),
        );
        
        self.audit_logger.log_event(event).await?;

        self.encryption_service.encrypt(data).await
    }

    /// Decrypt sensitive data
    pub async fn decrypt_data(&self, encrypted_data: &[u8], context: &SecurityContext) -> Result<Vec<u8>> {
        // Log data decryption
        let event = SecurityEvent::new(
            SecurityEventType::DataAccess,
            SecurityEventSeverity::Info,
            format!("Data decryption requested (size: {} bytes)", encrypted_data.len()),
            context.clone(),
        );
        
        self.audit_logger.log_event(event).await?;

        self.encryption_service.decrypt(encrypted_data).await
    }

    /// Log security event
    pub async fn log_security_event(&self, event: SecurityEvent) -> Result<()> {
        self.audit_logger.log_event(event).await
    }

    /// Update security policies
    pub async fn update_policies(&self, policies: SecurityPolicies) -> Result<()> {
        let mut current_policies = self.policies.write().await;
        *current_policies = policies;
        Ok(())
    }

    /// Get current security policies
    pub async fn get_policies(&self) -> SecurityPolicies {
        self.policies.read().await.clone()
    }

    /// Validate credentials
    async fn validate_credentials(
        &self,
        credentials: &RepositoryCredentials,
        context: &SecurityContext,
    ) -> Result<bool> {
        // Implementation depends on credential type
        match &credentials.auth_type {
            crate::config::AuthType::None => Ok(true),
            crate::config::AuthType::Token => {
                // Validate token
                // This would integrate with external token validation service
                Ok(credentials.credentials.contains_key("token"))
            }
            crate::config::AuthType::Basic => {
                // Validate username/password
                Ok(credentials.credentials.contains_key("username") 
                   && credentials.credentials.contains_key("password"))
            }
            crate::config::AuthType::ApiKey => {
                // Validate API key
                Ok(credentials.credentials.contains_key("api_key"))
            }
            crate::config::AuthType::SSH => {
                // Validate SSH key
                Ok(credentials.credentials.contains_key("private_key"))
            }
            crate::config::AuthType::OAuth2 => {
                // Validate OAuth2 token
                Ok(credentials.credentials.contains_key("access_token"))
            }
            crate::config::AuthType::Certificate => {
                // Validate client certificate
                Ok(credentials.credentials.contains_key("certificate") 
                   && credentials.credentials.contains_key("private_key"))
            }
        }
    }
}

/// Security policies configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicies {
    /// Minimum password length
    pub min_password_length: u32,
    /// Require multi-factor authentication
    pub require_mfa: bool,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
    /// Maximum failed login attempts
    pub max_failed_logins: u32,
    /// Account lockout duration in minutes
    pub lockout_duration_minutes: u32,
    /// Require password rotation
    pub require_password_rotation: bool,
    /// Password rotation interval in days
    pub password_rotation_days: u32,
    /// Enable IP whitelist
    pub enable_ip_whitelist: bool,
    /// Allowed IP addresses
    pub allowed_ips: Vec<String>,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Audit log retention in days
    pub audit_retention_days: u32,
}

impl Default for SecurityPolicies {
    fn default() -> Self {
        Self {
            min_password_length: 8,
            require_mfa: false,
            session_timeout_minutes: 60,
            max_failed_logins: 5,
            lockout_duration_minutes: 30,
            require_password_rotation: false,
            password_rotation_days: 90,
            enable_ip_whitelist: false,
            allowed_ips: Vec::new(),
            enable_audit_logging: true,
            audit_retention_days: 365,
        }
    }
}

