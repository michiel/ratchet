//! Simple integration tests for repository services with security

#[cfg(test)]
mod integration_tests {
    use crate::security::*;
    use crate::config::*;

    /// Test repository security context creation
    #[test]
    fn test_repository_security_context() {
        // Test security context creation
        let mut admin_context = SecurityContext::new("test-correlation".to_string());
        admin_context.user_id = Some("admin_user".to_string());
        admin_context.roles = vec!["admin".to_string()];

        assert_eq!(admin_context.correlation_id, "test-correlation");
        assert_eq!(admin_context.user_id.as_ref().unwrap(), "admin_user");
        assert!(admin_context.has_role("admin"));
        assert!(!admin_context.has_role("guest"));

        // Test system context
        let system_context = SecurityContext::system();
        assert_eq!(system_context.user_id.as_ref().unwrap(), "system");
        assert!(system_context.has_role("system"));
    }

    /// Test security event creation for repository operations
    #[test]
    fn test_repository_security_events() {
        let context = SecurityContext::system();
        
        // Test authentication event
        let auth_event = SecurityEvent::new(
            SecurityEventType::Authentication,
            SecurityEventSeverity::Info,
            "Repository authentication".to_string(),
            context.clone(),
        ).with_repository(1);
        
        assert_eq!(auth_event.event_type, SecurityEventType::Authentication);
        assert_eq!(auth_event.repository_id, Some(1));
        
        // Test authorization event
        let authz_event = SecurityEvent::new(
            SecurityEventType::Authorization,
            SecurityEventSeverity::Warning,
            "Repository access denied".to_string(),
            context,
        ).with_repository(2);
        
        assert_eq!(authz_event.event_type, SecurityEventType::Authorization);
        assert_eq!(authz_event.repository_id, Some(2));
        assert_eq!(authz_event.severity, SecurityEventSeverity::Warning);
    }

    /// Test security configuration for repositories
    #[test]
    fn test_repository_security_config() {
        let dev_config = RepositoryConfig::new_with_profile(ConfigProfile::Development);
        
        // Development should have minimal security
        assert_eq!(dev_config.security.auth.auth_type, AuthType::None);
        assert!(!dev_config.security.auth.require_mfa);
        assert!(!dev_config.security.encryption.encrypt_at_rest);
        
        let prod_config = RepositoryConfig::new_with_profile(ConfigProfile::Production);
        
        // Production should have enhanced security
        assert_eq!(prod_config.security.auth.auth_type, AuthType::OAuth2);
        assert!(prod_config.security.auth.require_mfa);
        assert!(prod_config.security.encryption.encrypt_at_rest);
        assert!(prod_config.security.audit.enabled);
    }

    /// Test repository permissions and roles
    #[test]
    fn test_repository_permissions() {
        let read_perm = Permission::Read;
        let write_perm = Permission::Write;
        let admin_perm = Permission::Admin;
        
        assert_eq!(read_perm, Permission::Read);
        assert_eq!(write_perm, Permission::Write);
        assert_eq!(admin_perm, Permission::Admin);
        
        let user_role = UserRole::User;
        let admin_role = UserRole::Admin;
        
        assert_eq!(user_role, UserRole::User);
        assert_eq!(admin_role, UserRole::Admin);
    }
}