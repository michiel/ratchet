# Ratchet RBAC Configuration and Troubleshooting Guide

This guide covers RBAC system configuration, common issues, troubleshooting steps, and performance optimization for Ratchet's Role-Based Access Control system.

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [Authentication Configuration](#authentication-configuration)
3. [Authorization Configuration](#authorization-configuration)
4. [Database Configuration](#database-configuration)
5. [Performance Tuning](#performance-tuning)
6. [Security Hardening](#security-hardening)
7. [Troubleshooting](#troubleshooting)
8. [Monitoring and Logging](#monitoring-and-logging)

## Configuration Overview

RBAC configuration is managed through multiple configuration files and environment variables:

```
config/
├── server.yaml              # Main server configuration
├── rbac.yaml                # RBAC-specific settings
├── database.yaml            # Database and storage settings
├── security.yaml            # Security policies and settings
└── development.yaml         # Development mode overrides
```

### Environment Variables

Key environment variables for RBAC:

```bash
# Authentication
JWT_SECRET="your-secret-key-here"
API_KEY_ENCRYPTION_KEY="api-key-encryption-secret"
SESSION_SECRET="session-encryption-secret"

# Database
DATABASE_URL="postgresql://user:password@localhost/ratchet"
REDIS_URL="redis://localhost:6379/0"

# Development
RATCHET_DEV_MODE="false"
RATCHET_DISABLE_RBAC="false"

# External Services
OAUTH2_GOOGLE_CLIENT_ID="google-client-id"
OAUTH2_GOOGLE_CLIENT_SECRET="google-client-secret"
OAUTH2_GITHUB_CLIENT_ID="github-client-id"
OAUTH2_GITHUB_CLIENT_SECRET="github-client-secret"
```

## Authentication Configuration

### JWT Configuration

```yaml
# config/rbac.yaml
authentication:
  jwt:
    # Secret key for JWT signing (use environment variable)
    secret_key: "${JWT_SECRET}"
    
    # Token issuer
    issuer: "ratchet-server"
    
    # Token audience
    audience: "ratchet-api"
    
    # Token expiration time in seconds (default: 24 hours)
    expiration_seconds: 86400
    
    # Signing algorithm (HS256, HS384, HS512, RS256, RS384, RS512)
    algorithm: "HS256"
    
    # Allow token refresh
    allow_refresh: true
    
    # Refresh token expiration (default: 7 days)
    refresh_expiration_seconds: 604800
    
    # Clock skew tolerance in seconds
    clock_skew_seconds: 30
    
    # Required claims validation
    validate_claims:
      exp: true    # Expiration time
      iat: true    # Issued at
      nbf: true    # Not before
      iss: true    # Issuer
      aud: true    # Audience
```

### API Key Configuration

```yaml
api_keys:
  # Enable API key authentication
  enabled: true
  
  # Key generation settings
  key_length: 32
  prefix_length: 8
  prefix: "rk_"
  
  # Default expiration (null = no expiration)
  default_expiry_days: 90
  
  # Maximum keys per user
  max_keys_per_user: 10
  
  # Require IP restrictions for new keys
  require_ip_restriction: false
  
  # Hashing algorithm for storing keys
  hash_algorithm: "bcrypt"
  hash_cost: 12
  
  # Rate limiting for API keys
  rate_limiting:
    enabled: true
    requests_per_minute: 1000
    burst_allowance: 50
```

### Session Configuration

```yaml
sessions:
  # Enable session-based authentication
  enabled: true
  
  # Session timeout in minutes
  timeout_minutes: 480  # 8 hours
  
  # Extend session on activity
  extend_on_access: true
  
  # Maximum sessions per user
  max_sessions_per_user: 5
  
  # Cookie settings
  cookie:
    name: "ratchet_session"
    secure: true        # HTTPS only
    http_only: true     # No JavaScript access
    same_site: "strict" # CSRF protection
    domain: null        # Auto-detect
    path: "/"
  
  # CSRF protection
  csrf:
    enabled: true
    token_length: 32
    header_name: "X-CSRF-Token"
  
  # Session storage backend
  storage:
    type: "redis"       # redis, database, memory
    connection_string: "${REDIS_URL}"
    key_prefix: "session:"
    
  # Cleanup settings
  cleanup:
    interval_minutes: 60
    expired_session_retention_hours: 24
```

### OAuth2 Configuration

```yaml
oauth2:
  # Enable OAuth2 authentication
  enabled: false
  
  # Supported providers
  providers:
    google:
      enabled: true
      client_id: "${OAUTH2_GOOGLE_CLIENT_ID}"
      client_secret: "${OAUTH2_GOOGLE_CLIENT_SECRET}"
      redirect_uri: "https://your-domain.com/auth/google/callback"
      scopes: ["openid", "profile", "email"]
      
    github:
      enabled: true
      client_id: "${OAUTH2_GITHUB_CLIENT_ID}"
      client_secret: "${OAUTH2_GITHUB_CLIENT_SECRET}"
      redirect_uri: "https://your-domain.com/auth/github/callback"
      scopes: ["user:email"]
      
    microsoft:
      enabled: false
      client_id: "${OAUTH2_MICROSOFT_CLIENT_ID}"
      client_secret: "${OAUTH2_MICROSOFT_CLIENT_SECRET}"
      redirect_uri: "https://your-domain.com/auth/microsoft/callback"
      scopes: ["openid", "profile", "email"]
      tenant_id: "common"  # or specific tenant ID
  
  # OAuth2 settings
  settings:
    state_expiration_minutes: 10
    pkce_enabled: true
    auto_create_users: true
    auto_assign_role: "viewer"
    email_domain_restrictions: []  # Empty = allow all domains
```

## Authorization Configuration

### Casbin Configuration

```yaml
authorization:
  # Casbin model configuration
  casbin:
    # Model file path
    model_file: "config/casbin_model.conf"
    
    # Policy storage adapter
    adapter:
      type: "seaorm"      # seaorm, file, redis
      connection_string: "${DATABASE_URL}"
      table_name: "casbin_rules"
      
    # Caching settings
    cache:
      enabled: true
      ttl_seconds: 300    # 5 minutes
      max_entries: 10000
      
    # Auto-save policy changes
    auto_save: true
    
    # Enable policy watching (for distributed systems)
    enable_watcher: false
    watcher_config:
      type: "redis"
      connection_string: "${REDIS_URL}"
      channel: "casbin_policy_updates"
```

### Policy Configuration

```yaml
# Default policies applied to all tenants
default_policies:
  # Require authentication for all operations
  - policy: "p, anonymous, *, *, *, deny"
    priority: 1000
  
  # Platform admins have full access
  - policy: "g, platform_admin, admin, platform"
    priority: 900
  
  # Tenant admins have full access within their tenant
  - policy: "p, admin, *, *, tenant_*, allow"
    priority: 800

# Resource-specific policies
resource_policies:
  tasks:
    # Task owners can manage their own tasks
    - policy: "p, task_owner, tasks, *, *, allow"
      condition: "r.sub == r.obj.owner_id"
    
    # Developers can create and execute tasks
    - policy: "p, developer, tasks, create|read|update|execute, *, allow"
  
  executions:
    # Users can view executions of tasks they can access
    - policy: "p, user, executions, read, *, allow"
      condition: "has_task_access(r.sub, r.obj.task_id)"

# Time-based policies
temporal_policies:
  business_hours:
    condition: "time_in_range('09:00', '17:00') && day_of_week in ['mon', 'tue', 'wed', 'thu', 'fri']"
    policies:
      - policy: "p, operator, tasks, execute, production, allow"
  
  maintenance_window:
    condition: "time_in_range('02:00', '04:00')"
    policies:
      - policy: "p, *, tasks, execute, production, deny"
      - policy: "p, admin, tasks, execute, production, allow"
```

### Role Configuration

```yaml
# Built-in roles definition
roles:
  platform_roles:
    platform_admin:
      display_name: "Platform Administrator"
      description: "Full system access across all tenants"
      permissions: ["*:*"]
      inherits_from: []
      
    platform_operator:
      display_name: "Platform Operator"
      description: "Read-only monitoring across all tenants"
      permissions:
        - "metrics:read"
        - "configurations:read"
        - "tasks:read"
        - "executions:read"
      inherits_from: []
  
  tenant_roles:
    admin:
      display_name: "Administrator"
      description: "Full tenant administration"
      permissions:
        - "tasks:*"
        - "executions:*"
        - "jobs:*"
        - "schedules:*"
        - "users:*"
        - "roles:*"
      inherits_from: []
      
    developer:
      display_name: "Developer"
      description: "Task development and execution"
      permissions:
        - "tasks:create"
        - "tasks:read"
        - "tasks:update"
        - "tasks:execute"
        - "executions:read"
        - "executions:cancel"
        - "jobs:create"
        - "jobs:read"
        - "schedules:create"
        - "schedules:read"
      inherits_from: []
      
    operator:
      display_name: "Operator"
      description: "Task execution and monitoring"
      permissions:
        - "tasks:read"
        - "tasks:execute"
        - "executions:read"
        - "executions:cancel"
        - "jobs:read"
        - "schedules:read"
        - "metrics:read"
      inherits_from: []
      
    viewer:
      display_name: "Viewer"
      description: "Read-only access"
      permissions:
        - "tasks:read"
        - "executions:read"
        - "jobs:read"
        - "schedules:read"
        - "metrics:read"
      inherits_from: []

# Custom role templates
role_templates:
  data_scientist:
    display_name: "Data Scientist"
    description: "Data analysis and machine learning tasks"
    permissions:
      - "tasks:read"
      - "tasks:execute"
      - "executions:read"
      - "metrics:read"
    conditions:
      - "task.tags.includes('data-science')"
      - "task.type in ['python', 'r', 'jupyter']"
```

## Database Configuration

### Connection Settings

```yaml
database:
  # Primary database connection
  primary:
    url: "${DATABASE_URL}"
    pool:
      max_connections: 20
      min_connections: 5
      connection_timeout_seconds: 30
      idle_timeout_seconds: 600
      max_lifetime_seconds: 3600
    
    # SSL configuration
    ssl:
      mode: "require"       # disable, allow, prefer, require
      ca_cert_file: "/etc/ssl/certs/ca.pem"
      client_cert_file: "/etc/ssl/certs/client.pem"
      client_key_file: "/etc/ssl/private/client.key"
    
    # Migration settings
    migrations:
      auto_run: true
      timeout_seconds: 300
      backup_before_migration: true
  
  # Read replica for queries (optional)
  read_replica:
    url: "${READ_REPLICA_DATABASE_URL}"
    pool:
      max_connections: 10
      min_connections: 2
      connection_timeout_seconds: 30
    
    # Fallback to primary if replica unavailable
    fallback_to_primary: true
    fallback_threshold_ms: 5000

# Redis configuration for caching and sessions
redis:
  url: "${REDIS_URL}"
  pool:
    max_connections: 20
    min_connections: 5
    connection_timeout_seconds: 10
    command_timeout_seconds: 5
  
  # Key prefixes for different uses
  key_prefixes:
    sessions: "session:"
    cache: "cache:"
    rate_limit: "rate_limit:"
    policy_cache: "policy:"
  
  # TTL settings
  default_ttl_seconds: 3600
  session_ttl_seconds: 28800  # 8 hours
  cache_ttl_seconds: 1800     # 30 minutes
```

### Performance Optimization

```yaml
performance:
  # Query optimization
  database:
    # Enable query logging (development only)
    log_queries: false
    log_slow_queries: true
    slow_query_threshold_ms: 1000
    
    # Connection pooling
    pool_size: 20
    max_overflow: 30
    pool_recycle_seconds: 3600
    
    # Query caching
    query_cache:
      enabled: true
      ttl_seconds: 300
      max_entries: 10000
  
  # Authorization caching
  authorization:
    # Cache permission checks
    permission_cache:
      enabled: true
      ttl_seconds: 300
      max_entries: 50000
    
    # Cache role assignments
    role_cache:
      enabled: true
      ttl_seconds: 600
      max_entries: 10000
    
    # Cache policy evaluations
    policy_cache:
      enabled: true
      ttl_seconds: 180
      max_entries: 25000
  
  # Rate limiting
  rate_limiting:
    # Global rate limiting
    global:
      enabled: true
      requests_per_minute: 10000
      burst_allowance: 100
    
    # Per-user rate limiting
    per_user:
      enabled: true
      requests_per_minute: 1000
      burst_allowance: 50
    
    # Per-tenant rate limiting
    per_tenant:
      enabled: true
      requests_per_minute: 5000
      burst_allowance: 200
```

## Security Hardening

### Password Policies

```yaml
security:
  passwords:
    # Minimum password requirements
    min_length: 12
    max_length: 128
    require_uppercase: true
    require_lowercase: true
    require_numbers: true
    require_special_chars: true
    special_chars: "!@#$%^&*()_+-=[]{}|;:,.<>?"
    
    # Password history
    history_count: 5
    min_age_hours: 1
    max_age_days: 90
    
    # Account lockout
    max_failed_attempts: 5
    lockout_duration_minutes: 30
    lockout_escalation: true
    
    # Password hashing
    hash_algorithm: "bcrypt"
    hash_cost: 12
  
  # Two-factor authentication
  two_factor:
    enabled: true
    required_for_roles: ["platform_admin", "admin"]
    methods: ["totp", "sms", "email"]
    backup_codes_count: 10
    totp_window_size: 1
    sms_provider: "twilio"
    
  # Account security
  accounts:
    # Session security
    force_logout_on_password_change: true
    max_concurrent_sessions: 3
    session_fixation_protection: true
    
    # Login monitoring
    track_login_attempts: true
    suspicious_login_detection: true
    new_device_notification: true
    
    # Account recovery
    password_reset_token_ttl_hours: 24
    email_verification_token_ttl_hours: 72
    account_recovery_cooldown_hours: 1
```

### Network Security

```yaml
network:
  # IP filtering
  ip_filtering:
    enabled: false
    default_policy: "allow"  # allow, deny
    allowed_ranges: []
    blocked_ranges:
      - "169.254.0.0/16"     # Link-local
      - "127.0.0.0/8"        # Loopback (if not needed)
    
    # Trusted proxies for X-Forwarded-For
    trusted_proxies:
      - "10.0.0.0/8"
      - "172.16.0.0/12"
      - "192.168.0.0/16"
  
  # TLS configuration
  tls:
    enabled: true
    min_version: "1.2"
    max_version: "1.3"
    ciphers: [
      "TLS_AES_128_GCM_SHA256",
      "TLS_AES_256_GCM_SHA384",
      "TLS_CHACHA20_POLY1305_SHA256"
    ]
    
    # Certificate settings
    cert_file: "/etc/ssl/certs/ratchet.crt"
    key_file: "/etc/ssl/private/ratchet.key"
    ca_file: "/etc/ssl/certs/ca.crt"
    
    # HSTS settings
    hsts:
      enabled: true
      max_age_seconds: 31536000
      include_subdomains: true
      preload: true
  
  # CORS settings
  cors:
    enabled: true
    allowed_origins: []     # Empty = use default policy
    allowed_methods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
    allowed_headers: ["Authorization", "Content-Type", "X-API-Key", "X-CSRF-Token"]
    expose_headers: ["X-Request-ID", "X-Rate-Limit-Remaining"]
    allow_credentials: true
    max_age_seconds: 86400
```

## Troubleshooting

### Common Issues

#### 1. Authentication Failures

**Symptoms**: Users cannot log in, JWT token validation fails

**Diagnosis**:
```bash
# Check JWT token validity
curl -X POST http://localhost:8080/api/v1/auth/validate-token \
  -H "Content-Type: application/json" \
  -d '{"token": "your-jwt-token"}'

# Check server logs for authentication errors
tail -f /var/log/ratchet/auth.log | grep "AUTH_ERROR"

# Verify JWT secret configuration
ratchet config get authentication.jwt.secret_key
```

**Solutions**:
- Verify JWT secret is correctly set
- Check token expiration times
- Ensure clock synchronization between client and server
- Validate issuer and audience claims

#### 2. Authorization Failures

**Symptoms**: Users get 403 Forbidden errors despite having correct roles

**Diagnosis**:
```bash
# Check user permissions
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/auth/me

# Test specific permission
curl -X POST http://localhost:8080/api/v1/auth/check-permission \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "resource": "tasks",
    "action": "create",
    "tenant_id": 1
  }'

# Check Casbin policies
ratchet rbac debug-policies --user-id 123 --tenant-id 1
```

**Solutions**:
- Verify user has correct role assignments
- Check tenant membership
- Validate Casbin policy rules
- Clear permission cache if needed

#### 3. Database Connection Issues

**Symptoms**: RBAC operations fail with database errors

**Diagnosis**:
```bash
# Test database connectivity
ratchet db test-connection

# Check connection pool status
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/db/pool-status

# Check for connection leaks
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/db/connections
```

**Solutions**:
- Verify database URL and credentials
- Check connection pool settings
- Monitor for connection leaks
- Ensure database schema is up to date

#### 4. Performance Issues

**Symptoms**: Slow authentication/authorization responses

**Diagnosis**:
```bash
# Check cache hit rates
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/cache/stats

# Monitor query performance
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/db/slow-queries

# Check authorization metrics
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/metrics/authorization
```

**Solutions**:
- Enable caching for permissions and roles
- Optimize database queries and indexes
- Increase connection pool size
- Consider read replicas for query operations

### Debug Commands

#### Enable Debug Logging
```yaml
# config/logging.yaml
logging:
  level: debug
  loggers:
    ratchet_rbac: debug
    ratchet_auth: debug
    casbin: debug
    seaorm: debug
```

#### RBAC Debug CLI Commands
```bash
# Debug user permissions
ratchet rbac debug user --id 123 --tenant-id 1

# Debug role inheritance
ratchet rbac debug role --name developer --tenant-id 1

# Debug policy evaluation
ratchet rbac debug policy \
  --user-id 123 \
  --resource tasks \
  --action create \
  --tenant-id 1

# Export RBAC configuration
ratchet rbac export --format json --output rbac-config.json

# Validate RBAC configuration
ratchet rbac validate --config rbac-config.json
```

#### Database Debugging
```bash
# Check RBAC tables
ratchet db query "SELECT COUNT(*) FROM casbin_rules"
ratchet db query "SELECT * FROM user_roles WHERE user_id = 123"
ratchet db query "SELECT * FROM tenant_users WHERE tenant_id = 1"

# Check for orphaned records
ratchet rbac cleanup --dry-run

# Rebuild permission cache
ratchet rbac rebuild-cache --tenant-id 1
```

### Health Checks

#### RBAC System Health
```bash
# Overall RBAC health check
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/rbac/health

# Authentication system health
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/auth/health

# Database health for RBAC
curl -H "Authorization: Bearer <admin-token>" \
  http://localhost:8080/api/v1/admin/db/health
```

Response:
```json
{
  "success": true,
  "data": {
    "overall_status": "healthy",
    "components": {
      "authentication": {
        "status": "healthy",
        "jwt_validation": "ok",
        "api_key_store": "ok",
        "session_store": "ok"
      },
      "authorization": {
        "status": "healthy",
        "casbin_enforcer": "ok",
        "policy_cache": "ok",
        "permission_cache": "ok"
      },
      "database": {
        "status": "healthy",
        "connection_pool": "ok",
        "query_performance": "ok",
        "migrations": "up_to_date"
      }
    },
    "metrics": {
      "active_sessions": 125,
      "cache_hit_rate": 0.94,
      "avg_auth_latency_ms": 12.5,
      "avg_authz_latency_ms": 8.3
    }
  }
}
```

## Monitoring and Logging

### Metrics Configuration

```yaml
monitoring:
  # Prometheus metrics
  prometheus:
    enabled: true
    endpoint: "/metrics"
    
    # Custom metrics
    custom_metrics:
      # Authentication metrics
      - name: "ratchet_auth_requests_total"
        type: "counter"
        help: "Total authentication requests"
        labels: ["method", "status"]
      
      - name: "ratchet_auth_duration_seconds"
        type: "histogram"
        help: "Authentication request duration"
        buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
      
      # Authorization metrics
      - name: "ratchet_authz_requests_total"
        type: "counter"
        help: "Total authorization requests"
        labels: ["resource", "action", "result"]
      
      - name: "ratchet_authz_cache_hits_total"
        type: "counter"
        help: "Authorization cache hits"
        labels: ["cache_type"]
  
  # Health check endpoints
  health_checks:
    - name: "rbac_health"
      endpoint: "/health/rbac"
      timeout_seconds: 5
      interval_seconds: 30
    
    - name: "auth_health"
      endpoint: "/health/auth"
      timeout_seconds: 3
      interval_seconds: 15

# Audit logging configuration
audit:
  enabled: true
  
  # What to log
  events:
    authentication:
      login_success: true
      login_failure: true
      logout: true
      token_refresh: true
      password_change: true
    
    authorization:
      permission_granted: false   # Too verbose for production
      permission_denied: true
      role_assignment: true
      role_removal: true
    
    administration:
      user_creation: true
      user_deletion: true
      role_creation: true
      role_modification: true
      tenant_creation: true
      tenant_modification: true
  
  # Log format and destination
  format: "json"
  destinations:
    - type: "file"
      path: "/var/log/ratchet/audit.log"
      rotation:
        max_size_mb: 100
        max_files: 10
        compress: true
    
    - type: "syslog"
      facility: "local0"
      tag: "ratchet-audit"
    
    - type: "webhook"
      url: "https://audit.example.com/api/events"
      headers:
        "Authorization": "Bearer ${AUDIT_WEBHOOK_TOKEN}"
      timeout_seconds: 10
      retry_attempts: 3
  
  # Data retention
  retention:
    default_days: 90
    critical_events_days: 365
    cleanup_interval_hours: 24
```

### Log Analysis Examples

```bash
# Find failed authentication attempts
grep "AUTH_FAILURE" /var/log/ratchet/audit.log | jq '.user_id' | sort | uniq -c

# Analyze permission denials
grep "PERMISSION_DENIED" /var/log/ratchet/audit.log | \
  jq -r '.resource + ":" + .action' | sort | uniq -c

# Check for suspicious activity
grep "TENANT_ACCESS_DENIED" /var/log/ratchet/audit.log | \
  jq '.user_id' | sort | uniq -c | sort -nr

# Monitor role assignments
grep "ROLE_ASSIGNED" /var/log/ratchet/audit.log | \
  jq -r '.details.role_name' | sort | uniq -c
```

For advanced monitoring and alerting configurations, see the [RBAC Monitoring Guide](RBAC_MONITORING.md).