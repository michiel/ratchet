//! Repository configuration management
//!
//! This module provides comprehensive configuration management for repository
//! operations, including security, performance, and environment-specific settings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository configuration profiles for different environments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigProfile {
    Development,
    Staging,
    Production,
    Enterprise,
    Custom(String),
}

impl Default for ConfigProfile {
    fn default() -> Self {
        Self::Development
    }
}

/// Repository-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// Repository ID
    pub repository_id: i32,
    /// Repository name for identification
    pub repository_name: String,
    /// Repository type (filesystem, git, http)
    pub repository_type: String,
    /// Repository URI
    pub uri: String,
    /// Configuration profile
    pub profile: ConfigProfile,
    /// Sync configuration
    pub sync: SyncConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Environment-specific settings
    pub environment: EnvironmentConfig,
    /// Custom configuration overrides
    pub custom: HashMap<String, serde_json::Value>,
    /// Configuration metadata
    pub metadata: ConfigMetadata,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            repository_id: 0,
            repository_name: "default".to_string(),
            repository_type: "filesystem".to_string(),
            uri: "./repositories/default".to_string(),
            profile: ConfigProfile::default(),
            sync: SyncConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
            environment: EnvironmentConfig::default(),
            custom: HashMap::new(),
            metadata: ConfigMetadata::default(),
        }
    }
}

/// Sync configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable automatic synchronization
    pub auto_sync: bool,
    /// Sync interval in minutes
    pub sync_interval_minutes: u32,
    /// Enable bidirectional sync
    pub bidirectional: bool,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolutionStrategy,
    /// Maximum sync timeout in seconds
    pub timeout_seconds: u32,
    /// Number of retry attempts
    pub retry_attempts: u32,
    /// Retry delay in seconds
    pub retry_delay_seconds: u32,
    /// Enable sync on file changes
    pub sync_on_change: bool,
    /// File change debounce delay in milliseconds
    pub change_debounce_ms: u64,
    /// Maximum concurrent sync operations
    pub max_concurrent_syncs: usize,
    /// Enable sync result caching
    pub enable_caching: bool,
    /// Cache TTL in minutes
    pub cache_ttl_minutes: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_interval_minutes: 15,
            bidirectional: true,
            conflict_resolution: ConflictResolutionStrategy::TakeLocal,
            timeout_seconds: 300,
            retry_attempts: 3,
            retry_delay_seconds: 5,
            sync_on_change: true,
            change_debounce_ms: 1000,
            max_concurrent_syncs: 2,
            enable_caching: true,
            cache_ttl_minutes: 30,
        }
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    TakeLocal,
    TakeRemote,
    Merge,
    Manual,
    Fail,
}

/// Security configuration for repository access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Encryption settings
    pub encryption: EncryptionConfig,
    /// Access control settings
    pub access_control: AccessControlConfig,
    /// Audit logging configuration
    pub audit: AuditConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            encryption: EncryptionConfig::default(),
            access_control: AccessControlConfig::default(),
            audit: AuditConfig::default(),
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,
    /// Authentication credentials (encrypted)
    pub credentials: HashMap<String, String>,
    /// Enable credential rotation
    pub rotate_credentials: bool,
    /// Credential rotation interval in days
    pub rotation_interval_days: u32,
    /// Enable MFA for sensitive operations
    pub require_mfa: bool,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            credentials: HashMap::new(),
            rotate_credentials: false,
            rotation_interval_days: 90,
            require_mfa: false,
            session_timeout_minutes: 60,
        }
    }
}

/// Authentication types supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    None,
    Token,
    Basic,
    ApiKey,
    SSH,
    OAuth2,
    Certificate,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable data-at-rest encryption
    pub encrypt_at_rest: bool,
    /// Enable data-in-transit encryption
    pub encrypt_in_transit: bool,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key rotation enabled
    pub key_rotation: bool,
    /// Key rotation interval in days
    pub key_rotation_days: u32,
    /// Backup encryption keys
    pub backup_keys: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            encrypt_at_rest: false,
            encrypt_in_transit: true,
            algorithm: EncryptionAlgorithm::AES256,
            key_rotation: false,
            key_rotation_days: 365,
            backup_keys: true,
        }
    }
}

/// Encryption algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EncryptionAlgorithm {
    AES128,
    AES256,
    ChaCha20,
    RSA2048,
    RSA4096,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    /// Enable role-based access control
    pub enable_rbac: bool,
    /// Default permissions for new users
    pub default_permissions: Vec<Permission>,
    /// Allowed user roles
    pub allowed_roles: Vec<UserRole>,
    /// IP whitelist for access
    pub ip_whitelist: Vec<String>,
    /// Enable time-based access restrictions
    pub time_restrictions: bool,
    /// Allowed access hours (24-hour format)
    pub allowed_hours: Option<(u8, u8)>,
    /// Maximum concurrent sessions per user
    pub max_sessions_per_user: u32,
}

impl Default for AccessControlConfig {
    fn default() -> Self {
        Self {
            enable_rbac: true,
            default_permissions: vec![Permission::Read],
            allowed_roles: vec![UserRole::User, UserRole::Admin],
            ip_whitelist: vec![],
            time_restrictions: false,
            allowed_hours: None,
            max_sessions_per_user: 5,
        }
    }
}

/// User permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Admin,
    Sync,
    Monitor,
}

/// User roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UserRole {
    Guest,
    User,
    Moderator,
    Admin,
    SuperAdmin,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log level for audit events
    pub log_level: AuditLogLevel,
    /// Audit log retention in days
    pub retention_days: u32,
    /// Enable real-time alerting
    pub realtime_alerts: bool,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Export audit logs
    pub export_enabled: bool,
    /// Export format
    pub export_format: AuditExportFormat,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: AuditLogLevel::Info,
            retention_days: 365,
            realtime_alerts: false,
            alert_thresholds: AlertThresholds::default(),
            export_enabled: false,
            export_format: AuditExportFormat::JSON,
        }
    }
}

/// Audit log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditLogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

/// Alert thresholds for security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Failed login attempts per minute
    pub failed_logins_per_minute: u32,
    /// Suspicious operations per hour
    pub suspicious_ops_per_hour: u32,
    /// Large data access threshold in MB
    pub large_data_access_mb: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            failed_logins_per_minute: 5,
            suspicious_ops_per_hour: 10,
            large_data_access_mb: 100,
        }
    }
}

/// Audit export formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditExportFormat {
    JSON,
    CSV,
    XML,
    Parquet,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Burst capacity
    pub burst_capacity: u32,
    /// Rate limit by IP
    pub limit_by_ip: bool,
    /// Rate limit by user
    pub limit_by_user: bool,
    /// Whitelist IPs exempt from rate limiting
    pub whitelist_ips: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 60,
            burst_capacity: 10,
            limit_by_ip: true,
            limit_by_user: true,
            whitelist_ips: vec!["127.0.0.1".to_string()],
        }
    }
}

/// Performance configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Connection pool settings
    pub connection_pool: ConnectionPoolConfig,
    /// Caching configuration
    pub caching: CachingConfig,
    /// Timeout configurations
    pub timeouts: TimeoutConfig,
    /// Concurrency settings
    pub concurrency: ConcurrencyConfig,
    /// Resource limits
    pub resource_limits: ResourceLimits,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            connection_pool: ConnectionPoolConfig::default(),
            caching: CachingConfig::default(),
            timeouts: TimeoutConfig::default(),
            concurrency: ConcurrencyConfig::default(),
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections
    pub max_connections: u32,
    /// Minimum number of connections
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u32,
    /// Idle timeout in seconds
    pub idle_timeout_seconds: u32,
    /// Enable connection validation
    pub validate_connections: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 300,
            validate_connections: true,
        }
    }
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache size in MB
    pub max_size_mb: u64,
    /// Cache TTL in minutes
    pub ttl_minutes: u32,
    /// Enable cache compression
    pub compression: bool,
    /// Cache eviction policy
    pub eviction_policy: CacheEvictionPolicy,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 256,
            ttl_minutes: 60,
            compression: true,
            eviction_policy: CacheEvictionPolicy::LRU,
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheEvictionPolicy {
    LRU,
    LFU,
    FIFO,
    TTL,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Request timeout in seconds
    pub request_timeout_seconds: u32,
    /// Operation timeout in seconds
    pub operation_timeout_seconds: u32,
    /// Health check timeout in seconds
    pub health_check_timeout_seconds: u32,
    /// Sync timeout in seconds
    pub sync_timeout_seconds: u32,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            request_timeout_seconds: 30,
            operation_timeout_seconds: 300,
            health_check_timeout_seconds: 10,
            sync_timeout_seconds: 600,
        }
    }
}

/// Concurrency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// Maximum concurrent operations
    pub max_concurrent_operations: u32,
    /// Maximum concurrent syncs per repository
    pub max_concurrent_syncs: u32,
    /// Worker thread pool size
    pub worker_pool_size: u32,
    /// Enable async processing
    pub async_processing: bool,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 10,
            max_concurrent_syncs: 2,
            worker_pool_size: 4,
            async_processing: true,
        }
    }
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Maximum disk space in GB
    pub max_disk_gb: u64,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: u8,
    /// Maximum network bandwidth in Mbps
    pub max_bandwidth_mbps: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_disk_gb: 10,
            max_cpu_percent: 80,
            max_bandwidth_mbps: 100,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    /// Metrics collection configuration
    pub metrics: MetricsConfig,
    /// Alerting configuration
    pub alerting: AlertingConfig,
    /// Dashboard configuration
    pub dashboard: DashboardConfig,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            health_check: HealthCheckConfig::default(),
            metrics: MetricsConfig::default(),
            alerting: AlertingConfig::default(),
            dashboard: DashboardConfig::default(),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval in minutes
    pub interval_minutes: u32,
    /// Health check timeout in seconds
    pub timeout_seconds: u32,
    /// Enable detailed health checks
    pub detailed_checks: bool,
    /// Health check endpoints
    pub endpoints: Vec<String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_minutes: 5,
            timeout_seconds: 10,
            detailed_checks: true,
            endpoints: vec!["/health".to_string()],
        }
    }
}

/// Metrics collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub collection_interval_seconds: u32,
    /// Metrics retention in days
    pub retention_days: u32,
    /// Export metrics to external systems
    pub export_enabled: bool,
    /// Metrics export format
    pub export_format: MetricsExportFormat,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_seconds: 60,
            retention_days: 30,
            export_enabled: false,
            export_format: MetricsExportFormat::Prometheus,
        }
    }
}

/// Metrics export formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricsExportFormat {
    Prometheus,
    InfluxDB,
    Graphite,
    JSON,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Alert delivery methods
    pub delivery_methods: Vec<AlertDeliveryMethod>,
    /// Alert severity levels
    pub severity_levels: Vec<AlertSeverity>,
    /// Alert cooldown period in minutes
    pub cooldown_minutes: u32,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delivery_methods: vec![AlertDeliveryMethod::Log],
            severity_levels: vec![
                AlertSeverity::Info,
                AlertSeverity::Warning,
                AlertSeverity::Critical,
            ],
            cooldown_minutes: 15,
        }
    }
}

/// Alert delivery methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertDeliveryMethod {
    Log,
    Email,
    SMS,
    Webhook,
    Slack,
    PagerDuty,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable dashboard
    pub enabled: bool,
    /// Dashboard refresh interval in seconds
    pub refresh_interval_seconds: u32,
    /// Enable real-time updates
    pub realtime_updates: bool,
    /// Dashboard themes
    pub themes: Vec<DashboardTheme>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            refresh_interval_seconds: 30,
            realtime_updates: false,
            themes: vec![DashboardTheme::Light, DashboardTheme::Dark],
        }
    }
}

/// Dashboard themes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DashboardTheme {
    Light,
    Dark,
    Auto,
}

/// Environment-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name
    pub environment: String,
    /// Debug mode enabled
    pub debug_mode: bool,
    /// Log level override
    pub log_level: Option<String>,
    /// Feature flags
    pub feature_flags: HashMap<String, bool>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Development settings
    pub development: Option<DevelopmentConfig>,
    /// Production settings
    pub production: Option<ProductionConfig>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            environment: "development".to_string(),
            debug_mode: true,
            log_level: Some("debug".to_string()),
            feature_flags: HashMap::new(),
            env_vars: HashMap::new(),
            development: Some(DevelopmentConfig::default()),
            production: None,
        }
    }
}

/// Development environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentConfig {
    /// Enable hot reloading
    pub hot_reload: bool,
    /// Enable debug endpoints
    pub debug_endpoints: bool,
    /// Mock external services
    pub mock_services: bool,
    /// Test data generation
    pub generate_test_data: bool,
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        Self {
            hot_reload: true,
            debug_endpoints: true,
            mock_services: false,
            generate_test_data: false,
        }
    }
}

/// Production environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    /// Enable performance optimizations
    pub performance_optimizations: bool,
    /// Enable security hardening
    pub security_hardening: bool,
    /// Enable comprehensive logging
    pub comprehensive_logging: bool,
    /// Enable monitoring and alerting
    pub monitoring_enabled: bool,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            performance_optimizations: true,
            security_hardening: true,
            comprehensive_logging: true,
            monitoring_enabled: true,
        }
    }
}

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// Configuration version
    pub version: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Configuration author
    pub author: String,
    /// Configuration description
    pub description: String,
    /// Configuration tags
    pub tags: Vec<String>,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
            author: "system".to_string(),
            description: "Default repository configuration".to_string(),
            tags: vec!["default".to_string()],
        }
    }
}

/// Configuration validation result
#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

impl RepositoryConfig {
    /// Create a new repository configuration with profile
    pub fn new_with_profile(profile: ConfigProfile) -> Self {
        let mut config = Self::default();
        config.profile = profile.clone();
        
        // Apply profile-specific defaults
        match profile {
            ConfigProfile::Development => {
                config.sync.sync_interval_minutes = 5;
                config.security.auth.auth_type = AuthType::None;
                config.monitoring.enabled = true;
                config.monitoring.health_check.detailed_checks = true;
                config.environment.debug_mode = true;
            }
            ConfigProfile::Staging => {
                config.sync.sync_interval_minutes = 10;
                config.security.auth.auth_type = AuthType::Token;
                config.monitoring.enabled = true;
                config.security.audit.enabled = true;
                config.environment.debug_mode = false;
            }
            ConfigProfile::Production => {
                config.sync.sync_interval_minutes = 15;
                config.security.auth.auth_type = AuthType::OAuth2;
                config.security.auth.require_mfa = true;
                config.security.encryption.encrypt_at_rest = true;
                config.monitoring.enabled = true;
                config.monitoring.alerting.enabled = true;
                config.security.audit.enabled = true;
                config.security.audit.realtime_alerts = true;
                config.environment.debug_mode = false;
                config.environment.production = Some(ProductionConfig::default());
                config.environment.development = None;
            }
            ConfigProfile::Enterprise => {
                config.sync.sync_interval_minutes = 30;
                config.security.auth.auth_type = AuthType::Certificate;
                config.security.auth.require_mfa = true;
                config.security.encryption.encrypt_at_rest = true;
                config.security.encryption.key_rotation = true;
                config.security.access_control.enable_rbac = true;
                config.monitoring.enabled = true;
                config.monitoring.alerting.enabled = true;
                config.monitoring.dashboard.enabled = true;
                config.security.audit.enabled = true;
                config.security.audit.realtime_alerts = true;
                config.performance.resource_limits.max_memory_mb = 4096;
                config.environment.debug_mode = false;
                config.environment.production = Some(ProductionConfig::default());
                config.environment.development = None;
            }
            ConfigProfile::Custom(_) => {
                // Keep defaults for custom profiles
            }
        }
        
        config
    }
    
    /// Validate configuration
    pub fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Validate repository configuration
        if self.repository_name.is_empty() {
            errors.push("Repository name cannot be empty".to_string());
        }
        
        if self.uri.is_empty() {
            errors.push("Repository URI cannot be empty".to_string());
        }
        
        // Validate sync configuration
        if self.sync.sync_interval_minutes == 0 {
            errors.push("Sync interval must be greater than 0".to_string());
        }
        
        if self.sync.timeout_seconds == 0 {
            errors.push("Sync timeout must be greater than 0".to_string());
        }
        
        // Validate security configuration
        if self.security.auth.auth_type != AuthType::None && self.security.auth.credentials.is_empty() {
            warnings.push("Authentication is enabled but no credentials are configured".to_string());
        }
        
        // Validate performance configuration
        if self.performance.connection_pool.max_connections == 0 {
            errors.push("Maximum connections must be greater than 0".to_string());
        }
        
        if self.performance.connection_pool.min_connections > self.performance.connection_pool.max_connections {
            errors.push("Minimum connections cannot be greater than maximum connections".to_string());
        }
        
        // Validate monitoring configuration
        if self.monitoring.enabled && self.monitoring.health_check.interval_minutes == 0 {
            errors.push("Health check interval must be greater than 0 when monitoring is enabled".to_string());
        }
        
        ConfigValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
    
    /// Apply environment-specific overrides
    pub fn apply_environment_overrides(&mut self, env_vars: &HashMap<String, String>) {
        // Apply environment variable overrides
        for (key, value) in env_vars {
            self.environment.env_vars.insert(key.clone(), value.clone());
        }
        
        // Apply specific environment overrides
        if let Some(log_level) = env_vars.get("LOG_LEVEL") {
            self.environment.log_level = Some(log_level.clone());
        }
        
        if let Some(debug) = env_vars.get("DEBUG_MODE") {
            self.environment.debug_mode = debug.parse().unwrap_or(false);
        }
        
        if let Some(sync_interval) = env_vars.get("SYNC_INTERVAL_MINUTES") {
            if let Ok(interval) = sync_interval.parse::<u32>() {
                self.sync.sync_interval_minutes = interval;
            }
        }
    }
    
    /// Merge with another configuration (other takes precedence)
    pub fn merge_with(&mut self, other: &RepositoryConfig) {
        // Merge basic settings
        self.repository_name = other.repository_name.clone();
        self.repository_type = other.repository_type.clone();
        self.uri = other.uri.clone();
        self.profile = other.profile.clone();
        
        // Merge sync settings
        self.sync = other.sync.clone();
        
        // Merge security settings
        self.security = other.security.clone();
        
        // Merge performance settings
        self.performance = other.performance.clone();
        
        // Merge monitoring settings
        self.monitoring = other.monitoring.clone();
        
        // Merge environment settings
        self.environment = other.environment.clone();
        
        // Merge custom settings
        for (key, value) in &other.custom {
            self.custom.insert(key.clone(), value.clone());
        }
        
        // Update metadata
        self.metadata.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_config_default() {
        let config = RepositoryConfig::default();
        assert_eq!(config.repository_name, "default");
        assert_eq!(config.repository_type, "filesystem");
        assert_eq!(config.profile, ConfigProfile::Development);
        assert!(config.sync.auto_sync);
        assert_eq!(config.sync.sync_interval_minutes, 15);
    }

    #[test]
    fn test_repository_config_with_profile() {
        let config = RepositoryConfig::new_with_profile(ConfigProfile::Production);
        assert_eq!(config.profile, ConfigProfile::Production);
        assert!(config.security.auth.require_mfa);
        assert!(config.security.encryption.encrypt_at_rest);
        assert!(config.monitoring.alerting.enabled);
        assert!(!config.environment.debug_mode);
    }

    #[test]
    fn test_config_validation() {
        let mut config = RepositoryConfig::default();
        
        // Valid configuration
        let result = config.validate();
        assert!(result.valid);
        assert!(result.errors.is_empty());
        
        // Invalid configuration
        config.repository_name = "".to_string();
        config.sync.sync_interval_minutes = 0;
        
        let result = config.validate();
        assert!(!result.valid);
        assert!(result.errors.len() >= 2);
    }

    #[test]
    fn test_environment_overrides() {
        let mut config = RepositoryConfig::default();
        let mut env_vars = HashMap::new();
        env_vars.insert("LOG_LEVEL".to_string(), "error".to_string());
        env_vars.insert("DEBUG_MODE".to_string(), "false".to_string());
        env_vars.insert("SYNC_INTERVAL_MINUTES".to_string(), "60".to_string());
        
        config.apply_environment_overrides(&env_vars);
        
        assert_eq!(config.environment.log_level, Some("error".to_string()));
        assert!(!config.environment.debug_mode);
        assert_eq!(config.sync.sync_interval_minutes, 60);
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = RepositoryConfig::default();
        let mut other_config = RepositoryConfig::new_with_profile(ConfigProfile::Production);
        other_config.repository_name = "production-repo".to_string();
        other_config.sync.sync_interval_minutes = 30;
        
        base_config.merge_with(&other_config);
        
        assert_eq!(base_config.repository_name, "production-repo");
        assert_eq!(base_config.sync.sync_interval_minutes, 30);
        assert_eq!(base_config.profile, ConfigProfile::Production);
    }
}