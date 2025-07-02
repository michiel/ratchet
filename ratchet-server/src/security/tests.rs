//! Simple integration tests for security systems

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::config::*;

    /// Test security context creation
    #[test]
    fn test_security_context_creation() {
        let context = SecurityContext::new("test-correlation".to_string());
        assert_eq!(context.correlation_id, "test-correlation");
        assert!(context.user_id.is_none());
        assert!(context.roles.is_empty());
        
        let system_context = SecurityContext::system();
        assert_eq!(system_context.user_id.as_ref().unwrap(), "system");
        assert!(system_context.has_role("system"));
    }

    /// Test security event creation
    #[test]
    fn test_security_event_creation() {
        let context = SecurityContext::system();
        let event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecurityEventSeverity::Info,
            "Test event".to_string(),
            context,
        );
        
        assert_eq!(event.event_type, SecurityEventType::Authentication);
        assert_eq!(event.severity, SecurityEventSeverity::Info);
        assert_eq!(event.message, "Test event");
        assert!(event.repository_id.is_none());
    }

    /// Test configuration profiles
    #[test]
    fn test_configuration_profiles() {
        let dev_config = RepositoryConfig::new_with_profile(ConfigProfile::Development);
        assert_eq!(dev_config.profile, ConfigProfile::Development);
        assert_eq!(dev_config.security.auth.auth_type, AuthType::None);
        
        let prod_config = RepositoryConfig::new_with_profile(ConfigProfile::Production);
        assert_eq!(prod_config.profile, ConfigProfile::Production);
        assert_eq!(prod_config.security.auth.auth_type, AuthType::OAuth2);
        assert!(prod_config.security.auth.require_mfa);
        assert!(prod_config.security.encryption.encrypt_at_rest);
    }

    /// Test security policies
    #[test]
    fn test_security_policies() {
        let policies = SecurityPolicies::default();
        assert_eq!(policies.min_password_length, 8);
        assert!(!policies.require_mfa);
        assert_eq!(policies.session_timeout_minutes, 60);
        assert!(policies.enable_audit_logging);
    }
}