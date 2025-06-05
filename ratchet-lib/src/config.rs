use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileReadError(#[from] std::io::Error),
    
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_yaml::Error),
    
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    
    #[error("Environment variable error: {0}")]
    EnvError(String),
}

/// Main Ratchet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct RatchetConfig {
    /// Task execution configuration
    #[serde(default)]
    pub execution: ExecutionConfig,
    
    /// HTTP client configuration
    #[serde(default)]
    pub http: HttpConfig,
    
    /// Caching configuration
    #[serde(default)]
    pub cache: CacheConfig,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: crate::logging::LoggingConfig,
    
    /// Output destinations configuration
    #[serde(default)]
    pub output: OutputConfig,
    
    /// Server configuration (optional, for future server mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfig>,
    
    /// MCP server configuration (optional, for LLM integration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpServerConfig>,
    
    /// Registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistryConfig>,
}

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// JavaScript variable names used for fetch operations
    #[serde(default)]
    pub fetch_variables: FetchVariables,
    
    /// Maximum execution time for JavaScript tasks
    #[serde(with = "serde_duration_seconds", default = "default_max_execution_duration")]
    pub max_execution_duration: Duration,
    
    /// Whether to validate schemas during execution
    #[serde(default = "default_true")]
    pub validate_schemas: bool,
}

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Request timeout
    #[serde(with = "serde_duration_seconds", default = "default_http_timeout")]
    pub timeout: Duration,
    
    /// Maximum number of redirects to follow
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,
    
    /// User agent string
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    
    /// Whether to verify SSL certificates
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// LRU cache size for task content
    #[serde(default = "default_cache_size")]
    pub task_content_cache_size: usize,
    
    /// Whether caching is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Output destinations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Maximum number of concurrent deliveries
    #[serde(default = "default_max_concurrent_deliveries")]
    pub max_concurrent_deliveries: usize,
    
    /// Default timeout for deliveries
    #[serde(with = "serde_duration_seconds", default = "default_delivery_timeout")]
    pub default_timeout: Duration,
    
    /// Whether to validate destination configurations on startup
    #[serde(default = "default_true")]
    pub validate_on_startup: bool,
    
    /// Global output destination templates
    #[serde(default)]
    pub global_destinations: Vec<OutputDestinationTemplate>,
    
    /// Default retry policy for failed deliveries
    #[serde(default)]
    pub default_retry_policy: RetryPolicyConfig,
}

/// Output destination template for reuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDestinationTemplate {
    /// Template name for reference
    pub name: String,
    
    /// Template description
    pub description: Option<String>,
    
    /// Destination configuration
    pub destination: OutputDestinationConfigTemplate,
}

/// Output destination configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutputDestinationConfigTemplate {
    Filesystem {
        /// Path template with variables
        path: String,
        /// Output format
        #[serde(default = "default_output_format")]
        format: String,
        /// File permissions (octal as string)
        #[serde(default = "default_file_permissions")]
        permissions: String,
        /// Whether to create directories
        #[serde(default = "default_true")]
        create_dirs: bool,
        /// Whether to overwrite existing files
        #[serde(default = "default_true")]
        overwrite: bool,
        /// Whether to backup existing files
        #[serde(default = "default_false")]
        backup_existing: bool,
    },
    Webhook {
        /// Webhook URL template
        url: String,
        /// HTTP method
        #[serde(default = "default_http_method")]
        method: String,
        /// HTTP headers
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        /// Request timeout in seconds
        #[serde(default = "default_webhook_timeout")]
        timeout_seconds: u64,
        /// Content type header
        content_type: Option<String>,
        /// Authentication configuration
        auth: Option<WebhookAuthConfig>,
    },
    Database {
        /// Database connection string
        connection_string: String,
        /// Target table name
        table_name: String,
        /// Column mappings
        column_mappings: std::collections::HashMap<String, String>,
    },
    S3 {
        /// S3 bucket name
        bucket: String,
        /// Object key template
        key_template: String,
        /// AWS region
        region: String,
        /// AWS access key ID (optional, can use environment)
        access_key_id: Option<String>,
        /// AWS secret access key (optional, can use environment)
        secret_access_key: Option<String>,
    },
}

/// Webhook authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WebhookAuthConfig {
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        header: String,
        value: String,
    },
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryPolicyConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: i32,
    
    /// Initial delay between retries in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    
    /// Backoff multiplier for exponential backoff
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}


/// Server configuration (for future server mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server bind address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    
    /// Authentication configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://ratchet.db")
    #[serde(default = "default_database_url")]
    pub url: String,
    
    /// Maximum number of database connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Connection timeout
    #[serde(with = "serde_duration_seconds", default = "default_connection_timeout")]
    pub connection_timeout: Duration,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,
    
    /// Token expiration time
    #[serde(with = "serde_duration_seconds")]
    pub token_expiration: Duration,
}

/// Enhanced MCP server configuration for LLM integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerConfig {
    /// Whether MCP server is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// Server settings
    #[serde(default)]
    pub server: McpServerSettings,
    
    /// Authentication configuration
    #[serde(default)]
    pub authentication: McpAuthenticationConfig,
    
    /// Security settings
    #[serde(default)]
    pub security: McpSecurityConfig,
    
    /// Performance settings
    #[serde(default)]
    pub performance: McpPerformanceConfig,
    
    /// Tool configuration
    #[serde(default)]
    pub tools: McpToolConfig,
    
    /// Audit and logging settings
    #[serde(default)]
    pub audit: McpAuditConfig,
}

/// MCP server settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerSettings {
    /// Transport type (stdio, sse, websocket)
    #[serde(default = "default_mcp_transport")]
    pub transport: String,
    
    /// Host for network transports (ignored for stdio)
    #[serde(default = "default_mcp_host")]
    pub host: String,
    
    /// Port for network transports (ignored for stdio)
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    
    /// Alternative ports for different services
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_port: Option<u16>,
    
    /// TLS configuration for secure connections
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<McpTlsConfig>,
    
    /// Whether to enable CORS for web-based connections
    #[serde(default = "default_false")]
    pub enable_cors: bool,
    
    /// Allowed origins for CORS
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

/// TLS configuration for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTlsConfig {
    /// Path to TLS certificate file
    pub cert_file: String,
    
    /// Path to TLS private key file
    pub key_file: String,
    
    /// Path to CA certificate file (for client authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<String>,
    
    /// Whether to require client certificates
    #[serde(default = "default_false")]
    pub require_client_cert: bool,
}

/// MCP authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpAuthenticationConfig {
    /// Primary authentication method
    #[serde(default = "default_auth_method")]
    pub method: String,
    
    /// API key authentication settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<McpApiKeyConfig>,
    
    /// JWT authentication settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt: Option<McpJwtConfig>,
    
    /// OAuth2 authentication settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<McpOAuth2Config>,
    
    /// Session configuration
    #[serde(default)]
    pub session: McpSessionConfig,
}

/// API key authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApiKeyConfig {
    /// List of valid API keys with metadata
    pub keys: std::collections::HashMap<String, McpApiKeyInfo>,
    
    /// Header name for API key (default: "Authorization")
    #[serde(default = "default_auth_header")]
    pub header_name: String,
    
    /// Prefix for API key (e.g., "Bearer", "ApiKey")
    #[serde(default = "default_auth_prefix")]
    pub prefix: String,
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApiKeyInfo {
    /// Human-readable name for this key
    pub name: String,
    
    /// Description of this key's purpose
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Client permissions
    pub permissions: McpClientPermissions,
    
    /// When this key was created (ISO 8601 format)
    pub created_at: String,
    
    /// When this key expires (ISO 8601 format, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    
    /// Whether this key is currently active
    #[serde(default = "default_true")]
    pub active: bool,
    
    /// IP address restrictions (CIDR notation)
    #[serde(default)]
    pub allowed_ips: Vec<String>,
}

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJwtConfig {
    /// JWT signing secret or path to public key file
    pub secret_or_key_file: String,
    
    /// JWT signing algorithm (HS256, RS256, etc.)
    #[serde(default = "default_jwt_algorithm")]
    pub algorithm: String,
    
    /// Token issuer to validate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    
    /// Audience to validate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    
    /// Token expiration time in seconds
    #[serde(default = "default_jwt_expiration")]
    pub expiration_seconds: u64,
    
    /// Clock skew tolerance in seconds
    #[serde(default = "default_jwt_clock_skew")]
    pub clock_skew_seconds: u64,
}

/// OAuth2 authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuth2Config {
    /// OAuth2 provider issuer URL
    pub issuer_url: String,
    
    /// Client ID for OAuth2
    pub client_id: String,
    
    /// Client secret for OAuth2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    
    /// Required scopes
    #[serde(default)]
    pub required_scopes: Vec<String>,
    
    /// JWKS URI for token validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_uri: Option<String>,
    
    /// Token introspection endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introspection_endpoint: Option<String>,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSessionConfig {
    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub timeout_seconds: u64,
    
    /// Maximum number of active sessions per client
    #[serde(default = "default_max_sessions_per_client")]
    pub max_sessions_per_client: u32,
    
    /// Session cleanup interval in seconds
    #[serde(default = "default_session_cleanup_interval")]
    pub cleanup_interval_seconds: u64,
    
    /// Whether to persist sessions across server restarts
    #[serde(default = "default_false")]
    pub persistent: bool,
}

/// MCP security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSecurityConfig {
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limiting: McpRateLimitConfig,
    
    /// Request size limits
    #[serde(default)]
    pub request_limits: McpRequestLimitsConfig,
    
    /// IP-based access control
    #[serde(default)]
    pub ip_filtering: McpIpFilterConfig,
    
    /// Security headers configuration
    #[serde(default)]
    pub headers: McpSecurityHeadersConfig,
    
    /// Input validation settings
    #[serde(default)]
    pub validation: McpValidationConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpRateLimitConfig {
    /// Overall requests per minute per client
    #[serde(default = "default_rate_limit_global")]
    pub global_per_minute: u32,
    
    /// Task execution requests per minute per client
    #[serde(default = "default_rate_limit_execute")]
    pub execute_task_per_minute: u32,
    
    /// Log reading requests per minute per client
    #[serde(default = "default_rate_limit_logs")]
    pub get_logs_per_minute: u32,
    
    /// Trace reading requests per minute per client
    #[serde(default = "default_rate_limit_traces")]
    pub get_traces_per_minute: u32,
    
    /// Rate limiting algorithm (token_bucket, sliding_window)
    #[serde(default = "default_rate_limit_algorithm")]
    pub algorithm: String,
    
    /// Burst allowance for rate limiting
    #[serde(default = "default_rate_limit_burst")]
    pub burst_allowance: u32,
}

/// Request size and complexity limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpRequestLimitsConfig {
    /// Maximum request size in bytes
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: u64,
    
    /// Maximum response size in bytes
    #[serde(default = "default_max_response_size")]
    pub max_response_size_bytes: u64,
    
    /// Maximum number of concurrent connections per IP
    #[serde(default = "default_max_connections_per_ip")]
    pub max_connections_per_ip: u32,
    
    /// Maximum number of concurrent executions per client
    #[serde(default = "default_max_concurrent_executions")]
    pub max_concurrent_executions_per_client: u32,
    
    /// Maximum execution time per task in seconds
    #[serde(default = "default_max_execution_time")]
    pub max_execution_time_seconds: u64,
}

/// IP filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpIpFilterConfig {
    /// Whether IP filtering is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// Default policy (allow, deny)
    #[serde(default = "default_ip_policy")]
    pub default_policy: String,
    
    /// Allowed IP ranges (CIDR notation)
    #[serde(default)]
    pub allowed_ranges: Vec<String>,
    
    /// Blocked IP ranges (CIDR notation)
    #[serde(default)]
    pub blocked_ranges: Vec<String>,
    
    /// Trusted proxy IPs for X-Forwarded-For handling
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSecurityHeadersConfig {
    /// Whether to add security headers
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Content Security Policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_security_policy: Option<String>,
    
    /// X-Frame-Options header value
    #[serde(default = "default_frame_options")]
    pub x_frame_options: String,
    
    /// X-Content-Type-Options header value
    #[serde(default = "default_content_type_options")]
    pub x_content_type_options: String,
    
    /// Strict-Transport-Security header value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_transport_security: Option<String>,
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpValidationConfig {
    /// Whether to validate JSON schemas strictly
    #[serde(default = "default_true")]
    pub strict_schema_validation: bool,
    
    /// Whether to sanitize string inputs
    #[serde(default = "default_true")]
    pub sanitize_strings: bool,
    
    /// Maximum string length for inputs
    #[serde(default = "default_max_string_length")]
    pub max_string_length: usize,
    
    /// Maximum array length for inputs
    #[serde(default = "default_max_array_length")]
    pub max_array_length: usize,
    
    /// Maximum object depth for nested inputs
    #[serde(default = "default_max_object_depth")]
    pub max_object_depth: usize,
}

/// MCP performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpPerformanceConfig {
    /// Connection pooling settings
    #[serde(default)]
    pub connection_pool: McpConnectionPoolConfig,
    
    /// Caching configuration
    #[serde(default)]
    pub caching: McpCachingConfig,
    
    /// Background task settings
    #[serde(default)]
    pub background_tasks: McpBackgroundTaskConfig,
    
    /// Resource monitoring
    #[serde(default)]
    pub monitoring: McpMonitoringConfig,
}

/// Connection pooling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConnectionPoolConfig {
    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Minimum number of idle connections to maintain
    #[serde(default = "default_min_idle_connections")]
    pub min_idle_connections: u32,
    
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout_seconds")]
    pub connection_timeout_seconds: u64,
    
    /// Idle connection timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_seconds: u64,
    
    /// Maximum connection lifetime in seconds
    #[serde(default = "default_max_connection_lifetime")]
    pub max_lifetime_seconds: u64,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpCachingConfig {
    /// Whether to enable response caching
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Cache size limit in megabytes
    #[serde(default = "default_cache_size_mb")]
    pub max_size_mb: u64,
    
    /// Default cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub default_ttl_seconds: u64,
    
    /// Whether to cache task execution results
    #[serde(default = "default_true")]
    pub cache_execution_results: bool,
    
    /// Whether to cache log queries
    #[serde(default = "default_true")]
    pub cache_log_queries: bool,
}

/// Background task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpBackgroundTaskConfig {
    /// Number of worker threads for background tasks
    #[serde(default = "default_worker_threads")]
    pub worker_threads: u32,
    
    /// Queue size for background tasks
    #[serde(default = "default_task_queue_size")]
    pub queue_size: u32,
    
    /// Health check interval in seconds
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_seconds: u64,
    
    /// Cleanup task interval in seconds
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_seconds: u64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpMonitoringConfig {
    /// Whether to enable metrics collection
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Metrics collection interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub collection_interval_seconds: u64,
    
    /// Whether to export metrics to external systems
    #[serde(default = "default_false")]
    pub export_enabled: bool,
    
    /// Metrics export endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_endpoint: Option<String>,
    
    /// Resource usage alerting thresholds
    #[serde(default)]
    pub alerts: McpAlertConfig,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpAlertConfig {
    /// CPU usage threshold (percentage)
    #[serde(default = "default_cpu_threshold")]
    pub cpu_threshold: f64,
    
    /// Memory usage threshold (percentage)
    #[serde(default = "default_memory_threshold")]
    pub memory_threshold: f64,
    
    /// Connection count threshold
    #[serde(default = "default_connection_threshold")]
    pub connection_threshold: u32,
    
    /// Error rate threshold (percentage)
    #[serde(default = "default_error_rate_threshold")]
    pub error_rate_threshold: f64,
}

/// MCP tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpToolConfig {
    /// Whether to enable task execution tools
    #[serde(default = "default_true")]
    pub enable_execution: bool,
    
    /// Whether to enable logging and monitoring tools
    #[serde(default = "default_true")]
    pub enable_logging: bool,
    
    /// Whether to enable system monitoring tools
    #[serde(default = "default_true")]
    pub enable_monitoring: bool,
    
    /// Whether to enable debugging tools
    #[serde(default = "default_false")]
    pub enable_debugging: bool,
    
    /// Whether to enable file system access tools
    #[serde(default = "default_false")]
    pub enable_filesystem: bool,
    
    /// Custom tool configurations
    #[serde(default)]
    pub custom_tools: std::collections::HashMap<String, serde_json::Value>,
    
    /// Tool-specific rate limits
    #[serde(default)]
    pub tool_rate_limits: std::collections::HashMap<String, u32>,
}

/// MCP audit and logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpAuditConfig {
    /// Whether to enable audit logging
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Audit log level (info, warn, error)
    #[serde(default = "default_audit_level")]
    pub level: String,
    
    /// Whether to log all requests and responses
    #[serde(default = "default_false")]
    pub log_all_requests: bool,
    
    /// Whether to log authentication events
    #[serde(default = "default_true")]
    pub log_auth_events: bool,
    
    /// Whether to log permission checks
    #[serde(default = "default_false")]
    pub log_permission_checks: bool,
    
    /// Whether to log performance metrics
    #[serde(default = "default_true")]
    pub log_performance: bool,
    
    /// Audit log rotation settings
    #[serde(default)]
    pub rotation: McpLogRotationConfig,
    
    /// External audit destinations
    #[serde(default)]
    pub external_destinations: Vec<McpAuditDestination>,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpLogRotationConfig {
    /// Maximum log file size in megabytes
    #[serde(default = "default_log_max_size")]
    pub max_size_mb: u64,
    
    /// Maximum number of rotated files to keep
    #[serde(default = "default_log_max_files")]
    pub max_files: u32,
    
    /// Whether to compress rotated logs
    #[serde(default = "default_true")]
    pub compress: bool,
}

/// External audit destination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpAuditDestination {
    #[serde(rename = "syslog")]
    Syslog {
        /// Syslog server address
        address: String,
        /// Syslog facility
        facility: String,
    },
    #[serde(rename = "webhook")]
    Webhook {
        /// Webhook URL
        url: String,
        /// HTTP headers
        headers: std::collections::HashMap<String, String>,
        /// Authentication
        auth: Option<WebhookAuthConfig>,
    },
    #[serde(rename = "database")]
    Database {
        /// Database connection string
        connection_string: String,
        /// Table name for audit logs
        table_name: String,
    },
}

/// Client permissions for MCP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpClientPermissions {
    /// Whether client can execute tasks
    #[serde(default = "default_true")]
    pub can_execute_tasks: bool,
    
    /// Whether client can read logs
    #[serde(default = "default_true")]
    pub can_read_logs: bool,
    
    /// Whether client can read execution traces
    #[serde(default = "default_false")]
    pub can_read_traces: bool,
    
    /// Whether client can access system information
    #[serde(default = "default_false")]
    pub can_access_system_info: bool,
    
    /// Task name patterns this client can execute (glob patterns)
    #[serde(default)]
    pub allowed_task_patterns: Vec<String>,
    
    /// Task name patterns this client cannot execute (glob patterns)
    #[serde(default)]
    pub denied_task_patterns: Vec<String>,
    
    /// Custom rate limits for this client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_rate_limits: Option<McpRateLimitConfig>,
    
    /// Resource quotas for this client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quotas: Option<McpRequestLimitsConfig>,
}

/// JavaScript fetch variables configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FetchVariables {
    /// Variable name for fetch URL
    #[serde(default = "default_url_var")]
    pub url_var: String,
    
    /// Variable name for fetch parameters
    #[serde(default = "default_params_var")]
    pub params_var: String,
    
    /// Variable name for fetch body
    #[serde(default = "default_body_var")]
    pub body_var: String,
    
    /// Variable name for HTTP result
    #[serde(default = "default_result_var")]
    pub result_var: String,
    
    /// Variable name for temporary result
    #[serde(default = "default_temp_result_var")]
    pub temp_result_var: String,
}

/// Serde helper for Duration serialization as seconds
mod serde_duration_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(seconds))
    }
}


impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            fetch_variables: FetchVariables::default(),
            max_execution_duration: Duration::from_secs(300), // 5 minutes
            validate_schemas: true,
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: "Ratchet/1.0".to_string(),
            verify_ssl: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            task_content_cache_size: 100,
            enabled: true,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_concurrent_deliveries: 10,
            default_timeout: Duration::from_secs(30),
            validate_on_startup: true,
            global_destinations: Vec::new(),
            default_retry_policy: RetryPolicyConfig::default(),
        }
    }
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}


impl Default for FetchVariables {
    fn default() -> Self {
        Self {
            url_var: "__fetch_url".to_string(),
            params_var: "__fetch_params".to_string(),
            body_var: "__fetch_body".to_string(),
            result_var: "__http_result".to_string(),
            temp_result_var: "__temp_result".to_string(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            database: DatabaseConfig::default(),
            auth: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server: McpServerSettings::default(),
            authentication: McpAuthenticationConfig::default(),
            security: McpSecurityConfig::default(),
            performance: McpPerformanceConfig::default(),
            tools: McpToolConfig::default(),
            audit: McpAuditConfig::default(),
        }
    }
}

impl Default for McpServerSettings {
    fn default() -> Self {
        Self {
            transport: default_mcp_transport(),
            host: default_mcp_host(),
            port: default_mcp_port(),
            metrics_port: None,
            tls: None,
            enable_cors: false,
            cors_origins: Vec::new(),
        }
    }
}

impl Default for McpAuthenticationConfig {
    fn default() -> Self {
        Self {
            method: default_auth_method(),
            api_key: None,
            jwt: None,
            oauth2: None,
            session: McpSessionConfig::default(),
        }
    }
}

impl Default for McpSessionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_session_timeout(),
            max_sessions_per_client: default_max_sessions_per_client(),
            cleanup_interval_seconds: default_session_cleanup_interval(),
            persistent: false,
        }
    }
}

impl Default for McpSecurityConfig {
    fn default() -> Self {
        Self {
            rate_limiting: McpRateLimitConfig::default(),
            request_limits: McpRequestLimitsConfig::default(),
            ip_filtering: McpIpFilterConfig::default(),
            headers: McpSecurityHeadersConfig::default(),
            validation: McpValidationConfig::default(),
        }
    }
}

impl Default for McpRateLimitConfig {
    fn default() -> Self {
        Self {
            global_per_minute: default_rate_limit_global(),
            execute_task_per_minute: default_rate_limit_execute(),
            get_logs_per_minute: default_rate_limit_logs(),
            get_traces_per_minute: default_rate_limit_traces(),
            algorithm: default_rate_limit_algorithm(),
            burst_allowance: default_rate_limit_burst(),
        }
    }
}

impl Default for McpRequestLimitsConfig {
    fn default() -> Self {
        Self {
            max_request_size_bytes: default_max_request_size(),
            max_response_size_bytes: default_max_response_size(),
            max_connections_per_ip: default_max_connections_per_ip(),
            max_concurrent_executions_per_client: default_max_concurrent_executions(),
            max_execution_time_seconds: default_max_execution_time(),
        }
    }
}

impl Default for McpIpFilterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_policy: default_ip_policy(),
            allowed_ranges: Vec::new(),
            blocked_ranges: Vec::new(),
            trusted_proxies: Vec::new(),
        }
    }
}

impl Default for McpSecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            content_security_policy: None,
            x_frame_options: default_frame_options(),
            x_content_type_options: default_content_type_options(),
            strict_transport_security: None,
        }
    }
}

impl Default for McpValidationConfig {
    fn default() -> Self {
        Self {
            strict_schema_validation: true,
            sanitize_strings: true,
            max_string_length: default_max_string_length(),
            max_array_length: default_max_array_length(),
            max_object_depth: default_max_object_depth(),
        }
    }
}

impl Default for McpPerformanceConfig {
    fn default() -> Self {
        Self {
            connection_pool: McpConnectionPoolConfig::default(),
            caching: McpCachingConfig::default(),
            background_tasks: McpBackgroundTaskConfig::default(),
            monitoring: McpMonitoringConfig::default(),
        }
    }
}

impl Default for McpConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            min_idle_connections: default_min_idle_connections(),
            connection_timeout_seconds: default_connection_timeout_seconds(),
            idle_timeout_seconds: default_idle_timeout(),
            max_lifetime_seconds: default_max_connection_lifetime(),
        }
    }
}

impl Default for McpCachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: default_cache_size_mb(),
            default_ttl_seconds: default_cache_ttl(),
            cache_execution_results: true,
            cache_log_queries: true,
        }
    }
}

impl Default for McpBackgroundTaskConfig {
    fn default() -> Self {
        Self {
            worker_threads: default_worker_threads(),
            queue_size: default_task_queue_size(),
            health_check_interval_seconds: default_health_check_interval(),
            cleanup_interval_seconds: default_cleanup_interval(),
        }
    }
}

impl Default for McpMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval_seconds: default_metrics_interval(),
            export_enabled: false,
            export_endpoint: None,
            alerts: McpAlertConfig::default(),
        }
    }
}

impl Default for McpAlertConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: default_cpu_threshold(),
            memory_threshold: default_memory_threshold(),
            connection_threshold: default_connection_threshold(),
            error_rate_threshold: default_error_rate_threshold(),
        }
    }
}

impl Default for McpToolConfig {
    fn default() -> Self {
        Self {
            enable_execution: true,
            enable_logging: true,
            enable_monitoring: true,
            enable_debugging: false,
            enable_filesystem: false,
            custom_tools: std::collections::HashMap::new(),
            tool_rate_limits: std::collections::HashMap::new(),
        }
    }
}

impl Default for McpAuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: default_audit_level(),
            log_all_requests: false,
            log_auth_events: true,
            log_permission_checks: false,
            log_performance: true,
            rotation: McpLogRotationConfig::default(),
            external_destinations: Vec::new(),
        }
    }
}

impl Default for McpLogRotationConfig {
    fn default() -> Self {
        Self {
            max_size_mb: default_log_max_size(),
            max_files: default_log_max_files(),
            compress: true,
        }
    }
}

impl Default for McpClientPermissions {
    fn default() -> Self {
        Self {
            can_execute_tasks: true,
            can_read_logs: true,
            can_read_traces: false,
            can_access_system_info: false,
            allowed_task_patterns: vec!["*".to_string()],
            denied_task_patterns: Vec::new(),
            custom_rate_limits: None,
            resource_quotas: None,
        }
    }
}

impl RatchetConfig {
    /// Load configuration from a YAML file with environment variable overrides
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: RatchetConfig = serde_yaml::from_str(&content)?;
        
        // Apply environment variable overrides
        config.apply_env_overrides()?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from environment variables only
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = RatchetConfig::default();
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // HTTP configuration overrides
        if let Ok(timeout) = std::env::var("RATCHET_HTTP_TIMEOUT") {
            let seconds: u64 = timeout.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_HTTP_TIMEOUT: {}", e)))?;
            self.http.timeout = Duration::from_secs(seconds);
        }
        
        if let Ok(user_agent) = std::env::var("RATCHET_HTTP_USER_AGENT") {
            self.http.user_agent = user_agent;
        }
        
        // Cache configuration overrides
        if let Ok(cache_size) = std::env::var("RATCHET_CACHE_SIZE") {
            let size: usize = cache_size.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_CACHE_SIZE: {}", e)))?;
            self.cache.task_content_cache_size = size;
        }
        
        // Logging configuration overrides
        if let Ok(log_level_str) = std::env::var("RATCHET_LOG_LEVEL") {
            use std::str::FromStr;
            if let Ok(log_level) = crate::logging::LogLevel::from_str(&log_level_str) {
                self.logging.level = log_level;
            } else {
                return Err(ConfigError::EnvError(format!("Invalid RATCHET_LOG_LEVEL: {}", log_level_str)));
            }
        }
        
        // Execution configuration overrides
        if let Ok(max_exec) = std::env::var("RATCHET_MAX_EXECUTION_SECONDS") {
            let seconds: u64 = max_exec.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_MAX_EXECUTION_SECONDS: {}", e)))?;
            self.execution.max_execution_duration = Duration::from_secs(seconds);
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Log level is now an enum, so it's always valid
        
        // Validate cache size
        if self.cache.task_content_cache_size == 0 {
            return Err(ConfigError::ValidationError(
                "Cache size must be greater than 0".to_string()
            ));
        }
        
        // Validate timeout values
        if self.http.timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "HTTP timeout must be greater than 0 seconds".to_string()
            ));
        }
        
        if self.execution.max_execution_duration.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "Max execution duration must be greater than 0 seconds".to_string()
            ));
        }
        
        // Validate fetch variable names are not empty
        let fetch_vars = &self.execution.fetch_variables;
        if fetch_vars.url_var.is_empty() || fetch_vars.params_var.is_empty() || 
           fetch_vars.body_var.is_empty() || fetch_vars.result_var.is_empty() || 
           fetch_vars.temp_result_var.is_empty() {
            return Err(ConfigError::ValidationError(
                "Fetch variable names cannot be empty".to_string()
            ));
        }
        
        // Validate output configuration
        self.validate_output_config()?;
        
        // Validate MCP configuration
        if let Some(mcp) = &self.mcp {
            self.validate_mcp_config(mcp)?;
        }
        
        Ok(())
    }
    
    /// Validate output configuration
    fn validate_output_config(&self) -> Result<(), ConfigError> {
        let output = &self.output;
        
        // Validate max concurrent deliveries
        if output.max_concurrent_deliveries == 0 {
            return Err(ConfigError::ValidationError(
                "Max concurrent deliveries must be greater than 0".to_string()
            ));
        }
        
        // Validate default timeout
        if output.default_timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "Default delivery timeout must be greater than 0 seconds".to_string()
            ));
        }
        
        // Validate retry policy
        let retry = &output.default_retry_policy;
        if retry.max_attempts <= 0 {
            return Err(ConfigError::ValidationError(
                "Max retry attempts must be greater than 0".to_string()
            ));
        }
        
        if retry.initial_delay_ms == 0 {
            return Err(ConfigError::ValidationError(
                "Initial retry delay must be greater than 0 milliseconds".to_string()
            ));
        }
        
        if retry.max_delay_ms < retry.initial_delay_ms {
            return Err(ConfigError::ValidationError(
                "Max retry delay must be greater than or equal to initial delay".to_string()
            ));
        }
        
        if retry.backoff_multiplier <= 1.0 {
            return Err(ConfigError::ValidationError(
                "Backoff multiplier must be greater than 1.0".to_string()
            ));
        }
        
        // Validate global destination templates
        for (index, template) in output.global_destinations.iter().enumerate() {
            if template.name.is_empty() {
                return Err(ConfigError::ValidationError(
                    format!("Global destination template {} has empty name", index)
                ));
            }
            
            self.validate_destination_template(&template.destination, &template.name)?;
        }
        
        Ok(())
    }
    
    /// Validate a destination template configuration
    fn validate_destination_template(&self, template: &OutputDestinationConfigTemplate, name: &str) -> Result<(), ConfigError> {
        match template {
            OutputDestinationConfigTemplate::Filesystem { path, format, .. } => {
                if path.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Filesystem destination '{}' has empty path", name)
                    ));
                }
                
                let valid_formats = ["json", "json_compact", "yaml", "csv", "raw", "template"];
                if !valid_formats.contains(&format.as_str()) {
                    return Err(ConfigError::ValidationError(
                        format!("Filesystem destination '{}' has invalid format '{}'. Valid formats: {}", 
                            name, format, valid_formats.join(", "))
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::Webhook { url, method, .. } => {
                if url.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has empty URL", name)
                    ));
                }
                
                // Basic URL validation
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has invalid URL format", name)
                    ));
                }
                
                let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
                if !valid_methods.contains(&method.to_uppercase().as_str()) {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has invalid HTTP method '{}'. Valid methods: {}", 
                            name, method, valid_methods.join(", "))
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::Database { connection_string, table_name, .. } => {
                if connection_string.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Database destination '{}' has empty connection string", name)
                    ));
                }
                
                if table_name.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Database destination '{}' has empty table name", name)
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::S3 { bucket, key_template, region, .. } => {
                if bucket.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty bucket name", name)
                    ));
                }
                
                if key_template.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty key template", name)
                    ));
                }
                
                if region.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty region", name)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate MCP configuration
    fn validate_mcp_config(&self, mcp: &McpServerConfig) -> Result<(), ConfigError> {
        if !mcp.enabled {
            return Ok(());
        }
        
        // Validate server settings
        self.validate_mcp_server_settings(&mcp.server)?;
        
        // Validate authentication configuration
        self.validate_mcp_authentication(&mcp.authentication)?;
        
        // Validate security configuration
        self.validate_mcp_security(&mcp.security)?;
        
        // Validate performance configuration
        self.validate_mcp_performance(&mcp.performance)?;
        
        // Validate audit configuration
        self.validate_mcp_audit(&mcp.audit)?;
        
        Ok(())
    }
    
    /// Validate MCP server settings
    fn validate_mcp_server_settings(&self, settings: &McpServerSettings) -> Result<(), ConfigError> {
        // Validate transport type
        let valid_transports = ["stdio", "sse", "websocket"];
        if !valid_transports.contains(&settings.transport.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("Invalid MCP transport '{}'. Valid options: {}", 
                    settings.transport, valid_transports.join(", "))
            ));
        }
        
        // Validate port for network transports
        if settings.transport != "stdio" && settings.port == 0 {
            return Err(ConfigError::ValidationError(
                "MCP port cannot be 0 for network transports".to_string()
            ));
        }
        
        // Validate host for network transports
        if settings.transport != "stdio" && settings.host.is_empty() {
            return Err(ConfigError::ValidationError(
                "MCP host cannot be empty for network transports".to_string()
            ));
        }
        
        // Validate TLS configuration if present
        if let Some(tls) = &settings.tls {
            if tls.cert_file.is_empty() {
                return Err(ConfigError::ValidationError(
                    "TLS certificate file path cannot be empty".to_string()
                ));
            }
            if tls.key_file.is_empty() {
                return Err(ConfigError::ValidationError(
                    "TLS private key file path cannot be empty".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate MCP authentication configuration
    fn validate_mcp_authentication(&self, auth: &McpAuthenticationConfig) -> Result<(), ConfigError> {
        let valid_methods = ["none", "api_key", "jwt", "oauth2", "certificate"];
        if !valid_methods.contains(&auth.method.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("Invalid MCP authentication method '{}'. Valid options: {}", 
                    auth.method, valid_methods.join(", "))
            ));
        }
        
        // Validate configuration based on method
        match auth.method.as_str() {
            "api_key" => {
                if auth.api_key.is_none() {
                    return Err(ConfigError::ValidationError(
                        "API key configuration required when auth method is 'api_key'".to_string()
                    ));
                }
                if let Some(api_key_config) = &auth.api_key {
                    if api_key_config.keys.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "At least one API key must be configured".to_string()
                        ));
                    }
                    // Validate each API key
                    for (key, info) in &api_key_config.keys {
                        if key.len() < 16 {
                            return Err(ConfigError::ValidationError(
                                "API keys must be at least 16 characters long".to_string()
                            ));
                        }
                        if info.name.is_empty() {
                            return Err(ConfigError::ValidationError(
                                "API key name cannot be empty".to_string()
                            ));
                        }
                        // Validate date formats if provided
                        if let Some(expires_at) = &info.expires_at {
                            if chrono::DateTime::parse_from_rfc3339(expires_at).is_err() {
                                return Err(ConfigError::ValidationError(
                                    format!("Invalid expiration date format for API key '{}'. Use ISO 8601 format.", info.name)
                                ));
                            }
                        }
                    }
                }
            }
            "jwt" => {
                if auth.jwt.is_none() {
                    return Err(ConfigError::ValidationError(
                        "JWT configuration required when auth method is 'jwt'".to_string()
                    ));
                }
                if let Some(jwt_config) = &auth.jwt {
                    if jwt_config.secret_or_key_file.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "JWT secret or key file path cannot be empty".to_string()
                        ));
                    }
                    let valid_algorithms = ["HS256", "HS384", "HS512", "RS256", "RS384", "RS512", "ES256", "ES384", "ES512"];
                    if !valid_algorithms.contains(&jwt_config.algorithm.as_str()) {
                        return Err(ConfigError::ValidationError(
                            format!("Invalid JWT algorithm '{}'. Valid options: {}", 
                                jwt_config.algorithm, valid_algorithms.join(", "))
                        ));
                    }
                }
            }
            "oauth2" => {
                if auth.oauth2.is_none() {
                    return Err(ConfigError::ValidationError(
                        "OAuth2 configuration required when auth method is 'oauth2'".to_string()
                    ));
                }
                if let Some(oauth2_config) = &auth.oauth2 {
                    if oauth2_config.issuer_url.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "OAuth2 issuer URL cannot be empty".to_string()
                        ));
                    }
                    if oauth2_config.client_id.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "OAuth2 client ID cannot be empty".to_string()
                        ));
                    }
                    // Basic URL validation
                    if !oauth2_config.issuer_url.starts_with("https://") {
                        return Err(ConfigError::ValidationError(
                            "OAuth2 issuer URL must use HTTPS".to_string()
                        ));
                    }
                }
            }
            _ => {} // "none" and "certificate" don't require additional validation
        }
        
        // Validate session configuration
        if auth.session.timeout_seconds == 0 {
            return Err(ConfigError::ValidationError(
                "Session timeout must be greater than 0".to_string()
            ));
        }
        
        if auth.session.max_sessions_per_client == 0 {
            return Err(ConfigError::ValidationError(
                "Max sessions per client must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate MCP security configuration
    fn validate_mcp_security(&self, security: &McpSecurityConfig) -> Result<(), ConfigError> {
        // Validate rate limiting
        let rate_limits = &security.rate_limiting;
        if rate_limits.global_per_minute == 0 {
            return Err(ConfigError::ValidationError(
                "Global rate limit must be greater than 0".to_string()
            ));
        }
        
        let valid_algorithms = ["token_bucket", "sliding_window", "fixed_window"];
        if !valid_algorithms.contains(&rate_limits.algorithm.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("Invalid rate limiting algorithm '{}'. Valid options: {}", 
                    rate_limits.algorithm, valid_algorithms.join(", "))
            ));
        }
        
        // Validate request limits
        let limits = &security.request_limits;
        if limits.max_request_size_bytes == 0 {
            return Err(ConfigError::ValidationError(
                "Max request size must be greater than 0".to_string()
            ));
        }
        
        if limits.max_response_size_bytes == 0 {
            return Err(ConfigError::ValidationError(
                "Max response size must be greater than 0".to_string()
            ));
        }
        
        if limits.max_connections_per_ip == 0 {
            return Err(ConfigError::ValidationError(
                "Max connections per IP must be greater than 0".to_string()
            ));
        }
        
        // Validate IP filtering if enabled
        if security.ip_filtering.enabled {
            let valid_policies = ["allow", "deny"];
            if !valid_policies.contains(&security.ip_filtering.default_policy.as_str()) {
                return Err(ConfigError::ValidationError(
                    format!("Invalid IP filtering policy '{}'. Valid options: {}", 
                        security.ip_filtering.default_policy, valid_policies.join(", "))
                ));
            }
        }
        
        // Validate input validation settings
        let validation = &security.validation;
        if validation.max_string_length == 0 {
            return Err(ConfigError::ValidationError(
                "Max string length must be greater than 0".to_string()
            ));
        }
        
        if validation.max_array_length == 0 {
            return Err(ConfigError::ValidationError(
                "Max array length must be greater than 0".to_string()
            ));
        }
        
        if validation.max_object_depth == 0 {
            return Err(ConfigError::ValidationError(
                "Max object depth must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate MCP performance configuration
    fn validate_mcp_performance(&self, performance: &McpPerformanceConfig) -> Result<(), ConfigError> {
        // Validate connection pool
        let pool = &performance.connection_pool;
        if pool.max_connections == 0 {
            return Err(ConfigError::ValidationError(
                "Max connections must be greater than 0".to_string()
            ));
        }
        
        if pool.min_idle_connections > pool.max_connections {
            return Err(ConfigError::ValidationError(
                "Min idle connections cannot exceed max connections".to_string()
            ));
        }
        
        if pool.connection_timeout_seconds == 0 {
            return Err(ConfigError::ValidationError(
                "Connection timeout must be greater than 0".to_string()
            ));
        }
        
        // Validate caching
        let caching = &performance.caching;
        if caching.enabled && caching.max_size_mb == 0 {
            return Err(ConfigError::ValidationError(
                "Cache size must be greater than 0 when caching is enabled".to_string()
            ));
        }
        
        if caching.enabled && caching.default_ttl_seconds == 0 {
            return Err(ConfigError::ValidationError(
                "Cache TTL must be greater than 0 when caching is enabled".to_string()
            ));
        }
        
        // Validate background tasks
        let bg_tasks = &performance.background_tasks;
        if bg_tasks.worker_threads == 0 {
            return Err(ConfigError::ValidationError(
                "Worker threads must be greater than 0".to_string()
            ));
        }
        
        if bg_tasks.queue_size == 0 {
            return Err(ConfigError::ValidationError(
                "Task queue size must be greater than 0".to_string()
            ));
        }
        
        // Validate monitoring thresholds
        let alerts = &performance.monitoring.alerts;
        if alerts.cpu_threshold <= 0.0 || alerts.cpu_threshold > 100.0 {
            return Err(ConfigError::ValidationError(
                "CPU threshold must be between 0 and 100".to_string()
            ));
        }
        
        if alerts.memory_threshold <= 0.0 || alerts.memory_threshold > 100.0 {
            return Err(ConfigError::ValidationError(
                "Memory threshold must be between 0 and 100".to_string()
            ));
        }
        
        if alerts.error_rate_threshold < 0.0 || alerts.error_rate_threshold > 100.0 {
            return Err(ConfigError::ValidationError(
                "Error rate threshold must be between 0 and 100".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate MCP audit configuration
    fn validate_mcp_audit(&self, audit: &McpAuditConfig) -> Result<(), ConfigError> {
        if !audit.enabled {
            return Ok(());
        }
        
        let valid_levels = ["debug", "info", "warn", "error"];
        if !valid_levels.contains(&audit.level.as_str()) {
            return Err(ConfigError::ValidationError(
                format!("Invalid audit log level '{}'. Valid options: {}", 
                    audit.level, valid_levels.join(", "))
            ));
        }
        
        // Validate log rotation
        if audit.rotation.max_size_mb == 0 {
            return Err(ConfigError::ValidationError(
                "Log rotation max size must be greater than 0".to_string()
            ));
        }
        
        if audit.rotation.max_files == 0 {
            return Err(ConfigError::ValidationError(
                "Log rotation max files must be greater than 0".to_string()
            ));
        }
        
        // Validate external destinations
        for destination in &audit.external_destinations {
            match destination {
                McpAuditDestination::Syslog { address, facility } => {
                    if address.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "Syslog server address cannot be empty".to_string()
                        ));
                    }
                    if facility.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "Syslog facility cannot be empty".to_string()
                        ));
                    }
                }
                McpAuditDestination::Webhook { url, .. } => {
                    if url.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "Webhook URL cannot be empty".to_string()
                        ));
                    }
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        return Err(ConfigError::ValidationError(
                            "Webhook URL must be a valid HTTP/HTTPS URL".to_string()
                        ));
                    }
                }
                McpAuditDestination::Database { connection_string, table_name } => {
                    if connection_string.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "Database connection string cannot be empty".to_string()
                        ));
                    }
                    if table_name.is_empty() {
                        return Err(ConfigError::ValidationError(
                            "Database table name cannot be empty".to_string()
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate a sample configuration file
    pub fn generate_sample() -> String {
        let config = RatchetConfig::default();
        serde_yaml::to_string(&config).unwrap_or_else(|_| "# Failed to generate sample config".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = RatchetConfig::default();
        
        assert_eq!(config.http.timeout, Duration::from_secs(30));
        assert_eq!(config.cache.task_content_cache_size, 100);
        assert_eq!(config.logging.level, crate::logging::LogLevel::Info);
        assert_eq!(config.execution.fetch_variables.url_var, "__fetch_url");
        assert!(config.execution.validate_schemas);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = RatchetConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Test zero cache size
        config = RatchetConfig::default();
        config.cache.task_content_cache_size = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_env_overrides() {
        std::env::set_var("RATCHET_HTTP_TIMEOUT", "60");
        std::env::set_var("RATCHET_CACHE_SIZE", "200");
        std::env::set_var("RATCHET_LOG_LEVEL", "debug");
        
        let config = RatchetConfig::from_env().unwrap();
        
        assert_eq!(config.http.timeout, Duration::from_secs(60));
        assert_eq!(config.cache.task_content_cache_size, 200);
        assert_eq!(config.logging.level, crate::logging::LogLevel::Debug);
        
        // Clean up
        std::env::remove_var("RATCHET_HTTP_TIMEOUT");
        std::env::remove_var("RATCHET_CACHE_SIZE");
        std::env::remove_var("RATCHET_LOG_LEVEL");
    }
    
    #[test]
    fn test_fetch_variables() {
        let config = RatchetConfig::default();
        let vars = &config.execution.fetch_variables;
        
        assert_eq!(vars.url_var, "__fetch_url");
        assert_eq!(vars.params_var, "__fetch_params");
        assert_eq!(vars.body_var, "__fetch_body");
        assert_eq!(vars.result_var, "__http_result");
        assert_eq!(vars.temp_result_var, "__temp_result");
    }
    
    #[test]
    fn test_partial_config_loading() {
        // Test that a partial config loads with defaults
        let yaml = r#"
logging:
  level: debug
"#;
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        
        // Check that defaults were applied
        assert_eq!(config.logging.level, crate::logging::LogLevel::Debug);  // Our override
        assert_eq!(config.http.timeout, Duration::from_secs(30));  // Default
        assert_eq!(config.cache.task_content_cache_size, 100);  // Default
        assert!(config.execution.validate_schemas);  // Default
    }
    
    #[test]
    fn test_empty_config_loading() {
        // Test that an empty config loads with all defaults
        let yaml = "{}";
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        
        // Check that all defaults were applied
        assert_eq!(config.logging.level, crate::logging::LogLevel::Info);
        assert_eq!(config.http.timeout, Duration::from_secs(30));
        assert_eq!(config.cache.task_content_cache_size, 100);
        assert!(config.execution.validate_schemas);
    }
    
    #[test]
    fn test_mcp_config_defaults() {
        let config = RatchetConfig::default();
        
        // MCP should be None by default
        assert!(config.mcp.is_none());
        
        // Test MCP config with defaults
        let mcp_config = McpServerConfig::default();
        assert!(!mcp_config.enabled);
        assert_eq!(mcp_config.server.transport, "stdio");
        assert_eq!(mcp_config.authentication.method, "none");
        assert!(mcp_config.tools.enable_execution);
        assert!(!mcp_config.tools.enable_debugging);
    }
    
    #[test]
    fn test_mcp_config_validation() {
        let mut config = RatchetConfig::default();
        
        // Test with disabled MCP - should validate
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = false;
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_ok());
        
        // Test with enabled MCP and valid stdio transport
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        mcp_config.server.transport = "stdio".to_string();
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_ok());
        
        // Test with invalid transport
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        mcp_config.server.transport = "invalid".to_string();
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_mcp_auth_validation() {
        let mut config = RatchetConfig::default();
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        
        // Test invalid auth method
        mcp_config.authentication.method = "invalid".to_string();
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test API key auth without keys
        mcp_config.authentication.method = "api_key".to_string();
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test JWT auth without config
        mcp_config.authentication.method = "jwt".to_string();
        mcp_config.authentication.api_key = None;
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test OAuth2 auth without config
        mcp_config.authentication.method = "oauth2".to_string();
        mcp_config.authentication.jwt = None;
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_mcp_security_validation() {
        let mut config = RatchetConfig::default();
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        
        // Test invalid rate limiting algorithm
        mcp_config.security.rate_limiting.algorithm = "invalid".to_string();
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test zero rate limit
        mcp_config.security.rate_limiting.algorithm = "token_bucket".to_string();
        mcp_config.security.rate_limiting.global_per_minute = 0;
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test zero request size limit
        mcp_config.security.rate_limiting.global_per_minute = 100;
        mcp_config.security.request_limits.max_request_size_bytes = 0;
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_mcp_performance_validation() {
        let mut config = RatchetConfig::default();
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        
        // Test invalid connection pool settings
        mcp_config.performance.connection_pool.max_connections = 0;
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test min_idle > max_connections
        mcp_config.performance.connection_pool.max_connections = 10;
        mcp_config.performance.connection_pool.min_idle_connections = 20;
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test invalid cache settings when enabled
        mcp_config.performance.connection_pool.min_idle_connections = 5;
        mcp_config.performance.caching.enabled = true;
        mcp_config.performance.caching.max_size_mb = 0;
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_mcp_audit_validation() {
        let mut config = RatchetConfig::default();
        let mut mcp_config = McpServerConfig::default();
        mcp_config.enabled = true;
        mcp_config.audit.enabled = true;
        
        // Test invalid audit level
        mcp_config.audit.level = "invalid".to_string();
        config.mcp = Some(mcp_config.clone());
        assert!(config.validate().is_err());
        
        // Test invalid log rotation settings
        mcp_config.audit.level = "info".to_string();
        mcp_config.audit.rotation.max_size_mb = 0;
        config.mcp = Some(mcp_config);
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_mcp_config_serialization() {
        // Test that MCP config can be serialized and deserialized
        let mcp_config = McpServerConfig::default();
        let yaml = serde_yaml::to_string(&mcp_config).unwrap();
        let deserialized: McpServerConfig = serde_yaml::from_str(&yaml).unwrap();
        
        assert_eq!(mcp_config.enabled, deserialized.enabled);
        assert_eq!(mcp_config.server.transport, deserialized.server.transport);
        assert_eq!(mcp_config.authentication.method, deserialized.authentication.method);
    }
    
    #[test]
    fn test_mcp_config_with_api_keys() {
        let yaml = r#"
mcp:
  enabled: true
  server:
    transport: "stdio"
  authentication:
    method: "api_key"
    api_key:
      keys:
        "test-key-1234567890123456":
          name: "Test Key"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            allowed_task_patterns:
              - "test-*"
          created_at: "2024-01-01T00:00:00Z"
          active: true
"#;
        
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        
        let mcp = config.mcp.unwrap();
        assert!(mcp.enabled);
        assert_eq!(mcp.authentication.method, "api_key");
        assert!(mcp.authentication.api_key.is_some());
        
        let api_key_config = mcp.authentication.api_key.unwrap();
        assert!(!api_key_config.keys.is_empty());
        assert!(api_key_config.keys.contains_key("test-key-1234567890123456"));
    }
    
    #[test]
    fn test_mcp_config_production_example() {
        // Test a production-like configuration
        let yaml = r#"
mcp:
  enabled: true
  server:
    transport: "sse"
    host: "0.0.0.0"
    port: 8443
    enable_cors: true
    cors_origins:
      - "https://example.com"
  authentication:
    method: "api_key"
    api_key:
      keys:
        "prod-key-abcdef1234567890abcdef12":
          name: "Production Client"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            can_read_traces: false
            allowed_task_patterns:
              - "approved-*"
          created_at: "2024-01-01T00:00:00Z"
          active: true
          allowed_ips:
            - "10.0.0.0/8"
  security:
    rate_limiting:
      global_per_minute: 1000
      algorithm: "sliding_window"
    ip_filtering:
      enabled: true
      default_policy: "deny"
      allowed_ranges:
        - "10.0.0.0/8"
  performance:
    connection_pool:
      max_connections: 100
    caching:
      enabled: true
      max_size_mb: 512
  tools:
    enable_execution: true
    enable_debugging: false
  audit:
    enabled: true
    level: "info"
"#;
        
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        
        let mcp = config.mcp.unwrap();
        assert!(mcp.enabled);
        assert_eq!(mcp.server.transport, "sse");
        assert_eq!(mcp.server.port, 8443);
        assert!(mcp.security.ip_filtering.enabled);
        assert!(!mcp.tools.enable_debugging);
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// List of registry sources
    pub sources: Vec<RegistrySourceConfig>,
}

/// Registry source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySourceConfig {
    /// Source name for identification
    pub name: String,
    
    /// Source URI (e.g., "file://./tasks" or "https://registry.example.com")
    pub uri: String,
    
    /// Additional source-specific configuration
    pub config: Option<serde_json::Value>,
}

// Default value functions for serde
fn default_true() -> bool {
    true
}


fn default_cache_size() -> usize {
    100
}

fn default_http_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_redirects() -> u32 {
    10
}

fn default_user_agent() -> String {
    "Ratchet/1.0".to_string()
}

fn default_max_execution_duration() -> Duration {
    Duration::from_secs(300)
}

fn default_url_var() -> String {
    "__fetch_url".to_string()
}

fn default_params_var() -> String {
    "__fetch_params".to_string()
}

fn default_body_var() -> String {
    "__fetch_body".to_string()
}

fn default_result_var() -> String {
    "__http_result".to_string()
}

fn default_temp_result_var() -> String {
    "__temp_result".to_string()
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_database_url() -> String {
    "sqlite::memory:".to_string()
}

fn default_max_connections() -> u32 {
    10
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

// Output configuration defaults
fn default_max_concurrent_deliveries() -> usize {
    10
}

fn default_delivery_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_output_format() -> String {
    "json".to_string()
}

fn default_file_permissions() -> String {
    "644".to_string()
}

fn default_false() -> bool {
    false
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn default_webhook_timeout() -> u64 {
    30
}

fn default_max_retries() -> i32 {
    3
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

// MCP server default functions
fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}

fn default_mcp_port() -> u16 {
    3000
}

fn default_mcp_max_connections() -> u32 {
    100
}

fn default_mcp_timeout() -> u64 {
    30
}

fn default_mcp_rate_limit() -> u32 {
    1000
}

// Enhanced MCP configuration default functions

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

fn default_auth_method() -> String {
    "none".to_string()
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_auth_prefix() -> String {
    "Bearer".to_string()
}

fn default_jwt_algorithm() -> String {
    "HS256".to_string()
}

fn default_jwt_expiration() -> u64 {
    3600 // 1 hour
}

fn default_jwt_clock_skew() -> u64 {
    60 // 1 minute
}

fn default_session_timeout() -> u64 {
    3600 // 1 hour
}

fn default_max_sessions_per_client() -> u32 {
    10
}

fn default_session_cleanup_interval() -> u64 {
    300 // 5 minutes
}

fn default_rate_limit_global() -> u32 {
    1000
}

fn default_rate_limit_execute() -> u32 {
    100
}

fn default_rate_limit_logs() -> u32 {
    500
}

fn default_rate_limit_traces() -> u32 {
    200
}

fn default_rate_limit_algorithm() -> String {
    "token_bucket".to_string()
}

fn default_rate_limit_burst() -> u32 {
    50
}

fn default_max_request_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_response_size() -> u64 {
    50 * 1024 * 1024 // 50MB
}

fn default_max_connections_per_ip() -> u32 {
    100
}

fn default_max_concurrent_executions() -> u32 {
    10
}

fn default_max_execution_time() -> u64 {
    300 // 5 minutes
}

fn default_ip_policy() -> String {
    "allow".to_string()
}

fn default_frame_options() -> String {
    "DENY".to_string()
}

fn default_content_type_options() -> String {
    "nosniff".to_string()
}

fn default_max_string_length() -> usize {
    1024 * 1024 // 1MB
}

fn default_max_array_length() -> usize {
    10000
}

fn default_max_object_depth() -> usize {
    32
}

fn default_min_idle_connections() -> u32 {
    5
}

fn default_connection_timeout_seconds() -> u64 {
    30
}

fn default_idle_timeout() -> u64 {
    300 // 5 minutes
}

fn default_max_connection_lifetime() -> u64 {
    3600 // 1 hour
}

fn default_cache_size_mb() -> u64 {
    256
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_worker_threads() -> u32 {
    4
}

fn default_task_queue_size() -> u32 {
    10000
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_cleanup_interval() -> u64 {
    300 // 5 minutes
}

fn default_metrics_interval() -> u64 {
    60 // 1 minute
}

fn default_cpu_threshold() -> f64 {
    80.0
}

fn default_memory_threshold() -> f64 {
    90.0
}

fn default_connection_threshold() -> u32 {
    1000
}

fn default_error_rate_threshold() -> f64 {
    5.0
}

fn default_audit_level() -> String {
    "info".to_string()
}

fn default_log_max_size() -> u64 {
    100 // 100MB
}

fn default_log_max_files() -> u32 {
    10
}