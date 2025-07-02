//! Simple integration tests for configuration management system

#[cfg(test)]
mod integration_tests {
    use crate::config::*;
    use std::collections::HashMap;

    /// Test repository configuration basic operations
    #[test]
    fn test_repository_config_basic() {
        // Create test repository config
        let repo_config = RepositoryConfig {
            repository_id: 1,
            repository_name: "test-repo".to_string(),
            repository_type: "filesystem".to_string(),
            uri: "./test-repo".to_string(),
            profile: ConfigProfile::Development,
            sync: SyncConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
            environment: EnvironmentConfig::default(),
            custom: HashMap::new(),
            metadata: ConfigMetadata::default(),
        };

        assert_eq!(repo_config.repository_id, 1);
        assert_eq!(repo_config.repository_name, "test-repo");
        assert_eq!(repo_config.profile, ConfigProfile::Development);
    }

    /// Test configuration profiles and templates
    #[test]
    fn test_configuration_profiles() {
        // Test development profile
        let dev_template = RepositoryConfig::new_with_profile(ConfigProfile::Development);
        assert_eq!(dev_template.profile, ConfigProfile::Development);
        assert_eq!(dev_template.security.auth.auth_type, AuthType::None);
        assert!(!dev_template.security.auth.require_mfa);
        assert!(!dev_template.security.encryption.encrypt_at_rest);
        assert!(dev_template.environment.debug_mode);

        // Test production profile
        let prod_template = RepositoryConfig::new_with_profile(ConfigProfile::Production);
        assert_eq!(prod_template.profile, ConfigProfile::Production);
        assert_eq!(prod_template.security.auth.auth_type, AuthType::OAuth2);
        assert!(prod_template.security.auth.require_mfa);
        assert!(prod_template.security.encryption.encrypt_at_rest);
        assert!(!prod_template.environment.debug_mode);
    }

    /// Test configuration validation
    #[test]
    fn test_configuration_validation() {
        // Valid configuration
        let valid_config = RepositoryConfig::new_with_profile(ConfigProfile::Development);
        let validation = valid_config.validate();
        assert!(validation.valid);
        assert!(validation.errors.is_empty());

        // Invalid configuration - empty name
        let mut invalid_config = valid_config.clone();
        invalid_config.repository_name = "".to_string();
        
        let validation = invalid_config.validate();
        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
        assert!(validation.errors.iter().any(|e| e.contains("name")));
    }

    /// Test auth types
    #[test]
    fn test_auth_types() {
        let auth_none = AuthType::None;
        let auth_oauth = AuthType::OAuth2;
        let auth_basic = AuthType::Basic;
        
        assert_eq!(auth_none, AuthType::None);
        assert_eq!(auth_oauth, AuthType::OAuth2);
        assert_eq!(auth_basic, AuthType::Basic);
    }

    /// Test encryption algorithms
    #[test]
    fn test_encryption_algorithms() {
        let aes256 = EncryptionAlgorithm::AES256;
        let chacha20 = EncryptionAlgorithm::ChaCha20;
        let rsa2048 = EncryptionAlgorithm::RSA2048;
        
        assert_eq!(aes256, EncryptionAlgorithm::AES256);
        assert_eq!(chacha20, EncryptionAlgorithm::ChaCha20);
        assert_eq!(rsa2048, EncryptionAlgorithm::RSA2048);
    }
}