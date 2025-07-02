//! Credential management for repository authentication
//!
//! This module provides secure credential storage, retrieval, and management
//! for repository access authentication.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{AuthType, RepositoryConfig};
use super::{EncryptionService, SecurityContext, SecurityEvent, SecurityEventType, SecurityEventSeverity};

/// Repository credentials for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryCredentials {
    /// Repository ID
    pub repository_id: i32,
    /// Authentication type
    pub auth_type: AuthType,
    /// Encrypted credentials
    pub credentials: HashMap<String, String>,
    /// Credential metadata
    pub metadata: CredentialMetadata,
}

/// Credential metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMetadata {
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,
    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Credential description
    pub description: String,
    /// Usage count
    pub usage_count: u64,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl Default for CredentialMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            last_used_at: None,
            expires_at: None,
            description: String::new(),
            usage_count: 0,
            tags: Vec::new(),
        }
    }
}

/// Credential validation result
#[derive(Debug, Clone)]
pub struct CredentialValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Credential expiry warning
    pub expires_soon: bool,
}

/// Credential manager for repository authentication
pub struct CredentialManager {
    /// Stored credentials by repository ID
    credentials: Arc<RwLock<HashMap<i32, RepositoryCredentials>>>,
    /// Encryption service for credential storage
    encryption_service: Arc<EncryptionService>,
    /// Credential rotation settings
    rotation_settings: Arc<RwLock<CredentialRotationSettings>>,
}

/// Credential rotation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRotationSettings {
    /// Enable automatic rotation
    pub auto_rotation: bool,
    /// Rotation interval in days
    pub rotation_interval_days: u32,
    /// Warning threshold in days before expiration
    pub warning_threshold_days: u32,
    /// Maximum credential age in days
    pub max_age_days: u32,
}

impl Default for CredentialRotationSettings {
    fn default() -> Self {
        Self {
            auto_rotation: false,
            rotation_interval_days: 90,
            warning_threshold_days: 7,
            max_age_days: 365,
        }
    }
}

impl CredentialManager {
    /// Create a new credential manager
    pub fn new(encryption_service: Arc<EncryptionService>) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            encryption_service,
            rotation_settings: Arc::new(RwLock::new(CredentialRotationSettings::default())),
        }
    }

    /// Store credentials for a repository
    pub async fn store_credentials(
        &self,
        repository_id: i32,
        auth_type: AuthType,
        credentials: HashMap<String, String>,
        context: &SecurityContext,
    ) -> Result<()> {
        // Encrypt credentials before storage
        let mut encrypted_credentials = HashMap::new();
        for (key, value) in credentials {
            let encrypted_value = self.encryption_service
                .encrypt(value.as_bytes())
                .await
                .context("Failed to encrypt credential value")?;
            encrypted_credentials.insert(key, base64::encode(encrypted_value));
        }

        let repo_credentials = RepositoryCredentials {
            repository_id,
            auth_type,
            credentials: encrypted_credentials,
            metadata: CredentialMetadata::default(),
        };

        // Store credentials
        let mut creds = self.credentials.write().await;
        creds.insert(repository_id, repo_credentials);

        Ok(())
    }

    /// Retrieve credentials for a repository
    pub async fn get_repository_credentials(
        &self,
        repository_id: i32,
    ) -> Result<Option<RepositoryCredentials>> {
        let creds = self.credentials.read().await;
        let credentials = creds.get(&repository_id).cloned();

        if let Some(mut repo_creds) = credentials {
            // Decrypt credentials
            let mut decrypted_credentials = HashMap::new();
            for (key, encrypted_value) in repo_creds.credentials {
                let encrypted_bytes = base64::decode(&encrypted_value)
                    .context("Failed to decode base64 credential")?;
                let decrypted_bytes = self.encryption_service
                    .decrypt(&encrypted_bytes)
                    .await
                    .context("Failed to decrypt credential value")?;
                let decrypted_value = String::from_utf8(decrypted_bytes)
                    .context("Failed to convert decrypted bytes to string")?;
                decrypted_credentials.insert(key, decrypted_value);
            }
            repo_creds.credentials = decrypted_credentials;

            // Update last used timestamp
            self.update_last_used(repository_id).await?;

            Ok(Some(repo_creds))
        } else {
            Ok(None)
        }
    }

    /// Update credentials for a repository
    pub async fn update_credentials(
        &self,
        repository_id: i32,
        credentials: HashMap<String, String>,
        context: &SecurityContext,
    ) -> Result<()> {
        let mut creds = self.credentials.write().await;
        if let Some(repo_creds) = creds.get_mut(&repository_id) {
            // Encrypt new credentials
            let mut encrypted_credentials = HashMap::new();
            for (key, value) in credentials {
                let encrypted_value = self.encryption_service
                    .encrypt(value.as_bytes())
                    .await
                    .context("Failed to encrypt credential value")?;
                encrypted_credentials.insert(key, base64::encode(encrypted_value));
            }

            repo_creds.credentials = encrypted_credentials;
            repo_creds.metadata.updated_at = Utc::now();
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Repository credentials not found"))
        }
    }

    /// Remove credentials for a repository
    pub async fn remove_credentials(
        &self,
        repository_id: i32,
        context: &SecurityContext,
    ) -> Result<()> {
        let mut creds = self.credentials.write().await;
        creds.remove(&repository_id);
        Ok(())
    }

    /// List all repository IDs with stored credentials
    pub async fn list_repositories_with_credentials(&self) -> Result<Vec<i32>> {
        let creds = self.credentials.read().await;
        Ok(creds.keys().cloned().collect())
    }

    /// Validate repository credentials
    pub async fn validate_credentials(
        &self,
        repository_id: i32,
        context: &SecurityContext,
    ) -> Result<CredentialValidationResult> {
        let creds = self.credentials.read().await;
        
        if let Some(repo_creds) = creds.get(&repository_id) {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();
            let mut expires_soon = false;

            // Check credential completeness based on auth type
            match repo_creds.auth_type {
                AuthType::None => {
                    // No validation needed
                }
                AuthType::Token => {
                    if !repo_creds.credentials.contains_key("token") {
                        errors.push("Token authentication requires 'token' credential".to_string());
                    }
                }
                AuthType::Basic => {
                    if !repo_creds.credentials.contains_key("username") {
                        errors.push("Basic authentication requires 'username' credential".to_string());
                    }
                    if !repo_creds.credentials.contains_key("password") {
                        errors.push("Basic authentication requires 'password' credential".to_string());
                    }
                }
                AuthType::ApiKey => {
                    if !repo_creds.credentials.contains_key("api_key") {
                        errors.push("API key authentication requires 'api_key' credential".to_string());
                    }
                }
                AuthType::SSH => {
                    if !repo_creds.credentials.contains_key("private_key") {
                        errors.push("SSH authentication requires 'private_key' credential".to_string());
                    }
                }
                AuthType::OAuth2 => {
                    if !repo_creds.credentials.contains_key("access_token") {
                        errors.push("OAuth2 authentication requires 'access_token' credential".to_string());
                    }
                }
                AuthType::Certificate => {
                    if !repo_creds.credentials.contains_key("certificate") {
                        errors.push("Certificate authentication requires 'certificate' credential".to_string());
                    }
                    if !repo_creds.credentials.contains_key("private_key") {
                        errors.push("Certificate authentication requires 'private_key' credential".to_string());
                    }
                }
            }

            // Check expiration
            if let Some(expires_at) = repo_creds.metadata.expires_at {
                let now = Utc::now();
                let rotation_settings = self.rotation_settings.read().await;
                let warning_threshold = chrono::Duration::days(rotation_settings.warning_threshold_days as i64);
                
                if expires_at <= now {
                    errors.push("Credentials have expired".to_string());
                } else if expires_at <= now + warning_threshold {
                    warnings.push("Credentials will expire soon".to_string());
                    expires_soon = true;
                }
            }

            // Check credential age
            let rotation_settings = self.rotation_settings.read().await;
            let max_age = chrono::Duration::days(rotation_settings.max_age_days as i64);
            if repo_creds.metadata.created_at + max_age <= Utc::now() {
                warnings.push("Credentials are older than maximum allowed age".to_string());
            }

            Ok(CredentialValidationResult {
                valid: errors.is_empty(),
                errors,
                warnings,
                expires_soon,
            })
        } else {
            Ok(CredentialValidationResult {
                valid: false,
                errors: vec!["No credentials found for repository".to_string()],
                warnings: Vec::new(),
                expires_soon: false,
            })
        }
    }

    /// Rotate credentials for a repository
    pub async fn rotate_credentials(
        &self,
        repository_id: i32,
        new_credentials: HashMap<String, String>,
        context: &SecurityContext,
    ) -> Result<()> {
        // Update credentials with new values
        self.update_credentials(repository_id, new_credentials, context).await?;

        // Update rotation metadata
        let mut creds = self.credentials.write().await;
        if let Some(repo_creds) = creds.get_mut(&repository_id) {
            let rotation_settings = self.rotation_settings.read().await;
            let rotation_interval = chrono::Duration::days(rotation_settings.rotation_interval_days as i64);
            repo_creds.metadata.expires_at = Some(Utc::now() + rotation_interval);
        }

        Ok(())
    }

    /// Check if credentials need rotation
    pub async fn credentials_need_rotation(&self, repository_id: i32) -> Result<bool> {
        let creds = self.credentials.read().await;
        let rotation_settings = self.rotation_settings.read().await;
        
        if !rotation_settings.auto_rotation {
            return Ok(false);
        }

        if let Some(repo_creds) = creds.get(&repository_id) {
            let rotation_interval = chrono::Duration::days(rotation_settings.rotation_interval_days as i64);
            let next_rotation = repo_creds.metadata.updated_at + rotation_interval;
            Ok(Utc::now() >= next_rotation)
        } else {
            Ok(false)
        }
    }

    /// Get credential statistics
    pub async fn get_credential_stats(&self) -> Result<CredentialStats> {
        let creds = self.credentials.read().await;
        let rotation_settings = self.rotation_settings.read().await;
        
        let total_credentials = creds.len();
        let mut expired_count = 0;
        let mut expiring_soon_count = 0;
        let mut needs_rotation_count = 0;

        let now = Utc::now();
        let warning_threshold = chrono::Duration::days(rotation_settings.warning_threshold_days as i64);
        let rotation_interval = chrono::Duration::days(rotation_settings.rotation_interval_days as i64);

        for repo_creds in creds.values() {
            // Check expiration
            if let Some(expires_at) = repo_creds.metadata.expires_at {
                if expires_at <= now {
                    expired_count += 1;
                } else if expires_at <= now + warning_threshold {
                    expiring_soon_count += 1;
                }
            }

            // Check rotation need
            if rotation_settings.auto_rotation {
                let next_rotation = repo_creds.metadata.updated_at + rotation_interval;
                if now >= next_rotation {
                    needs_rotation_count += 1;
                }
            }
        }

        Ok(CredentialStats {
            total_credentials,
            expired_count,
            expiring_soon_count,
            needs_rotation_count,
        })
    }

    /// Update rotation settings
    pub async fn update_rotation_settings(&self, settings: CredentialRotationSettings) -> Result<()> {
        let mut rotation_settings = self.rotation_settings.write().await;
        *rotation_settings = settings;
        Ok(())
    }

    /// Get rotation settings
    pub async fn get_rotation_settings(&self) -> CredentialRotationSettings {
        self.rotation_settings.read().await.clone()
    }

    /// Update last used timestamp for credentials
    async fn update_last_used(&self, repository_id: i32) -> Result<()> {
        let mut creds = self.credentials.write().await;
        if let Some(repo_creds) = creds.get_mut(&repository_id) {
            repo_creds.metadata.last_used_at = Some(Utc::now());
            repo_creds.metadata.usage_count += 1;
        }
        Ok(())
    }
}

/// Credential statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStats {
    /// Total number of credentials
    pub total_credentials: usize,
    /// Number of expired credentials
    pub expired_count: usize,
    /// Number of credentials expiring soon
    pub expiring_soon_count: usize,
    /// Number of credentials needing rotation
    pub needs_rotation_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::encryption::MockEncryptionService;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_credential_storage_and_retrieval() {
        let encryption_service = Arc::new(MockEncryptionService::new());
        let manager = CredentialManager::new(encryption_service);
        let context = SecurityContext::system();

        let mut credentials = HashMap::new();
        credentials.insert("username".to_string(), "test_user".to_string());
        credentials.insert("password".to_string(), "test_pass".to_string());

        // Store credentials
        manager.store_credentials(1, AuthType::Basic, credentials.clone(), &context).await.unwrap();

        // Retrieve credentials
        let retrieved = manager.get_repository_credentials(1).await.unwrap();
        assert!(retrieved.is_some());
        
        let repo_creds = retrieved.unwrap();
        assert_eq!(repo_creds.repository_id, 1);
        assert_eq!(repo_creds.auth_type, AuthType::Basic);
        assert!(repo_creds.credentials.contains_key("username"));
        assert!(repo_creds.credentials.contains_key("password"));
    }

    #[tokio::test]
    async fn test_credential_validation() {
        let encryption_service = Arc::new(MockEncryptionService::new());
        let manager = CredentialManager::new(encryption_service);
        let context = SecurityContext::system();

        // Store incomplete basic auth credentials
        let mut credentials = HashMap::new();
        credentials.insert("username".to_string(), "test_user".to_string());
        // Missing password

        manager.store_credentials(1, AuthType::Basic, credentials, &context).await.unwrap();

        // Validate credentials
        let validation = manager.validate_credentials(1, &context).await.unwrap();
        assert!(!validation.valid);
        assert!(validation.errors.iter().any(|e| e.contains("password")));
    }

    #[tokio::test]
    async fn test_credential_rotation_check() {
        let encryption_service = Arc::new(MockEncryptionService::new());
        let manager = CredentialManager::new(encryption_service);

        // Enable auto rotation with short interval for testing
        let mut settings = CredentialRotationSettings::default();
        settings.auto_rotation = true;
        settings.rotation_interval_days = 0; // Immediate rotation needed
        manager.update_rotation_settings(settings).await.unwrap();

        let context = SecurityContext::system();
        let mut credentials = HashMap::new();
        credentials.insert("token".to_string(), "test_token".to_string());

        manager.store_credentials(1, AuthType::Token, credentials, &context).await.unwrap();

        // Check if rotation is needed
        let needs_rotation = manager.credentials_need_rotation(1).await.unwrap();
        assert!(needs_rotation);
    }
}

/// Mock encryption service for testing
#[cfg(test)]
mod mock_encryption {
    use super::*;

    pub struct MockEncryptionService;

    impl MockEncryptionService {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl super::super::EncryptionService for MockEncryptionService {
        async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
            // Simple mock: just return the data as-is
            Ok(data.to_vec())
        }

        async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
            // Simple mock: just return the data as-is
            Ok(encrypted_data.to_vec())
        }

        async fn generate_key(&self) -> Result<Vec<u8>> {
            Ok(vec![0u8; 32])
        }

        async fn rotate_key(&self) -> Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
use mock_encryption::MockEncryptionService;