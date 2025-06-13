# RBAC (Role-Based Access Control) Implementation Plan

**Document Version**: 1.0  
**Date**: June 2025  
**Priority**: **CRITICAL** - Security Foundation  
**Estimated Timeline**: 6-8 weeks  
**Effort**: 1-2 senior developers  

## Executive Summary

This plan outlines the implementation of a comprehensive Role-Based Access Control (RBAC) system for Ratchet. Currently, all API endpoints are publicly accessible with no authentication or authorization mechanisms, creating significant security risks for production deployment.

**Goal**: Implement a secure, scalable RBAC system that provides fine-grained access control across all Ratchet services (REST API, GraphQL, MCP, Interactive Console) while maintaining operational flexibility and user experience.

---

## 🎯 Current State Analysis

### **Security Vulnerabilities**
1. **No authentication**: All API endpoints are publicly accessible
2. **No authorization**: No permission checking for any operations
3. **No user management**: No user accounts or identity system
4. **No audit trail**: No tracking of who performed which operations
5. **No session management**: No secure session handling

### **Code Locations**
```rust
// ratchet-mcp/src/security/auth.rs - Incomplete implementation
impl McpAuthManager {
    pub async fn authenticate(&self, request: &AuthRequest) -> Result<UserContext, AuthError> {
        match &request.auth_type {
            McpAuth::None => Ok(UserContext::anonymous()), // Always allows access
            McpAuth::ApiKey { key } => {
                // TODO: Implement API key validation
                Err(AuthError::NotImplemented)
            }
            McpAuth::JWT { token } => {
                // TODO: Implement JWT validation
                Err(AuthError::NotImplemented)
            }
        }
    }
}

// No middleware enforcement in REST/GraphQL APIs
// All endpoints currently unprotected
```

---

## 🏗️ RBAC Architecture Design

### **Core Components Overview**

```
┌─────────────────────────────────────────────────┐
│                  RBAC System                     │
├─────────────────────────────────────────────────┤
│  Authentication Layer                            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │     JWT     │ │  API Keys   │ │   OAuth2    ││
│  │   Tokens    │ │             │ │             ││
│  └─────────────┘ └─────────────┘ └─────────────┘│
├─────────────────────────────────────────────────┤
│  Authorization Layer                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │    Users    │ │    Roles    │ │ Permissions ││
│  │             │ │             │ │             ││
│  └─────────────┘ └─────────────┘ └─────────────┘│
├─────────────────────────────────────────────────┤
│  Policy Engine                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │   Access    │ │   Resource  │ │   Context   ││
│  │   Control   │ │   Policies  │ │   Rules     ││
│  └─────────────┘ └─────────────┘ └─────────────┘│
├─────────────────────────────────────────────────┤
│  Audit & Monitoring                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐│
│  │ Audit Logs  │ │   Security  │ │  Compliance ││
│  │             │ │   Events    │ │  Reporting  ││
│  └─────────────┘ └─────────────┘ └─────────────┘│
└─────────────────────────────────────────────────┘
```

### **Multi-Layer Security Model**

1. **Authentication Layer**: Verify identity through multiple methods
2. **Authorization Layer**: Determine permissions based on roles
3. **Policy Engine**: Enforce fine-grained access control rules
4. **Audit Layer**: Track and monitor all security events

---

## 🔧 Implementation Phases

### **Phase 1: Authentication Foundation (Week 1-2)**

#### 1.1 **Core Authentication Types**
```rust
// ratchet-auth/src/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    JWT {
        token: String,
        issuer: Option<String>,
    },
    ApiKey {
        key: String,
        prefix: Option<String>,
    },
    OAuth2 {
        provider: OAuth2Provider,
        token: String,
    },
    Session {
        session_id: String,
        csrf_token: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuth2Provider {
    Google,
    GitHub,
    Microsoft,
    Custom { endpoint: String },
}

// User identity after authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<Permission>,
    pub session_id: Option<String>,
    pub auth_method: AuthMethod,
    pub authenticated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

// Authentication request context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub request_id: String,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub requested_resource: String,
    pub requested_action: String,
    pub timestamp: DateTime<Utc>,
}
```

#### 1.2 **JWT Authentication Implementation**
```rust
// ratchet-auth/src/jwt.rs
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    issuer: String,
    audience: Option<String>,
    expiration_hours: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,                    // Subject (user ID)
    pub username: String,               // Username
    pub email: Option<String>,          // Email
    pub roles: Vec<String>,             // User roles
    pub permissions: Vec<String>,       // Direct permissions
    pub iss: String,                    // Issuer
    pub aud: Option<String>,            // Audience
    pub exp: i64,                       // Expiration time
    pub iat: i64,                       // Issued at
    pub jti: String,                    // JWT ID for revocation
    pub session_id: Option<String>,     // Session tracking
}

impl JwtManager {
    pub fn new(secret: &str, issuer: String) -> Result<Self, AuthError> {
        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            algorithm: Algorithm::HS256,
            issuer,
            audience: None,
            expiration_hours: 24,
        })
    }

    pub fn generate_token(&self, user: &User) -> Result<String, AuthError> {
        let now = Utc::now();
        let exp = now + chrono::Duration::hours(self.expiration_hours as i64);
        
        let claims = JwtClaims {
            sub: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            roles: user.roles.iter().map(|r| r.name.clone()).collect(),
            permissions: user.get_all_permissions().iter().map(|p| p.to_string()).collect(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            session_id: None,
        };

        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGeneration { source: e.into() })
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, AuthError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[self.issuer.clone()]);
        
        if let Some(audience) = &self.audience {
            validation.set_audience(&[audience.clone()]);
        }

        decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| AuthError::TokenValidation { source: e.into() })
    }

    pub async fn revoke_token(&self, jti: &str) -> Result<(), AuthError> {
        // Add to revocation list (Redis/Database)
        // Implementation depends on storage backend
        unimplemented!("Token revocation requires storage implementation")
    }
}
```

#### 1.3 **API Key Authentication**
```rust
// ratchet-auth/src/api_key.rs
#[derive(Debug, Clone)]
pub struct ApiKeyManager {
    storage: Arc<dyn ApiKeyStorage>,
    hasher: ApiKeyHasher,
    config: ApiKeyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,               // Hashed key (never store plaintext)
    pub prefix: String,                 // First 8 chars for identification
    pub user_id: String,                // Owner
    pub roles: Vec<String>,             // Associated roles
    pub permissions: Vec<Permission>,   // Direct permissions
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: u64,
    pub is_active: bool,
    pub allowed_ips: Vec<String>,       // IP restrictions
    pub rate_limit: Option<RateLimit>,  // Per-key rate limiting
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub key_length: usize,              // Default: 32 characters
    pub prefix_length: usize,           // Default: 8 characters
    pub default_expiry_days: Option<u32>, // None = no expiry
    pub max_keys_per_user: u32,         // Default: 10
    pub require_ip_restriction: bool,   // Default: false
}

impl ApiKeyManager {
    pub async fn create_key(
        &self,
        user_id: &str,
        name: &str,
        permissions: Vec<Permission>,
    ) -> Result<(String, ApiKey), AuthError> {
        // Generate secure random key
        let key = self.generate_secure_key()?;
        let prefix = key[..self.config.prefix_length].to_string();
        let key_hash = self.hasher.hash(&key)?;

        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            key_hash,
            prefix,
            user_id: user_id.to_string(),
            roles: vec![], // Set based on user roles
            permissions,
            created_at: Utc::now(),
            expires_at: self.config.default_expiry_days.map(|days| {
                Utc::now() + chrono::Duration::days(days as i64)
            }),
            last_used_at: None,
            usage_count: 0,
            is_active: true,
            allowed_ips: vec![],
            rate_limit: None,
        };

        self.storage.store_key(&api_key).await?;
        Ok((key, api_key))
    }

    pub async fn validate_key(&self, key: &str, client_ip: &str) -> Result<UserContext, AuthError> {
        // Extract prefix for efficient lookup
        if key.len() < self.config.prefix_length {
            return Err(AuthError::InvalidApiKey);
        }
        
        let prefix = &key[..self.config.prefix_length];
        let api_key = self.storage.find_by_prefix(prefix).await?
            .ok_or(AuthError::InvalidApiKey)?;

        // Verify key hash
        if !self.hasher.verify(key, &api_key.key_hash)? {
            return Err(AuthError::InvalidApiKey);
        }

        // Check if key is active and not expired
        if !api_key.is_active {
            return Err(AuthError::ApiKeyDisabled);
        }

        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::ApiKeyExpired);
            }
        }

        // Check IP restrictions
        if !api_key.allowed_ips.is_empty() && !api_key.allowed_ips.contains(&client_ip.to_string()) {
            return Err(AuthError::IpNotAllowed { ip: client_ip.to_string() });
        }

        // Update usage statistics
        self.storage.update_usage(&api_key.id).await?;

        // Create user context
        Ok(UserContext {
            user_id: api_key.user_id,
            username: format!("api-key:{}", api_key.name),
            email: None,
            roles: api_key.roles,
            permissions: api_key.permissions,
            session_id: None,
            auth_method: AuthMethod::ApiKey { 
                key: prefix.to_string(),
                prefix: Some(prefix.to_string()),
            },
            authenticated_at: Utc::now(),
            expires_at: api_key.expires_at,
            metadata: HashMap::new(),
        })
    }
}
```

### **Phase 2: Role and Permission System (Week 2-3)**

#### 2.1 **Role and Permission Definitions**
```rust
// ratchet-auth/src/rbac.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    // Task Management
    TaskRead,
    TaskWrite,
    TaskExecute,
    TaskDelete,
    
    // Job Management
    JobRead,
    JobWrite,
    JobCancel,
    JobRetry,
    
    // Execution Management
    ExecutionRead,
    ExecutionCancel,
    ExecutionLogs,
    
    // Repository Management
    RepositoryRead,
    RepositoryWrite,
    RepositoryRefresh,
    RepositoryDelete,
    
    // Server Management
    ServerStatus,
    ServerMetrics,
    ServerConfig,
    ServerRestart,
    
    // Database Management
    DatabaseRead,
    DatabaseWrite,
    DatabaseMigrate,
    DatabaseBackup,
    
    // User Management
    UserRead,
    UserWrite,
    UserDelete,
    UserRoles,
    
    // System Administration
    SystemConfig,
    SystemLogs,
    SystemHealth,
    SystemDebug,
    
    // Console Access
    ConsoleAccess,
    ConsoleAdmin,
    
    // MCP Access
    McpAccess,
    McpToolExecute,
    McpAdministration,
}

impl Permission {
    pub fn resource(&self) -> &'static str {
        match self {
            Permission::TaskRead | Permission::TaskWrite | Permission::TaskExecute | Permission::TaskDelete => "task",
            Permission::JobRead | Permission::JobWrite | Permission::JobCancel | Permission::JobRetry => "job",
            Permission::ExecutionRead | Permission::ExecutionCancel | Permission::ExecutionLogs => "execution",
            Permission::RepositoryRead | Permission::RepositoryWrite | Permission::RepositoryRefresh | Permission::RepositoryDelete => "repository",
            Permission::ServerStatus | Permission::ServerMetrics | Permission::ServerConfig | Permission::ServerRestart => "server",
            Permission::DatabaseRead | Permission::DatabaseWrite | Permission::DatabaseMigrate | Permission::DatabaseBackup => "database",
            Permission::UserRead | Permission::UserWrite | Permission::UserDelete | Permission::UserRoles => "user",
            Permission::SystemConfig | Permission::SystemLogs | Permission::SystemHealth | Permission::SystemDebug => "system",
            Permission::ConsoleAccess | Permission::ConsoleAdmin => "console",
            Permission::McpAccess | Permission::McpToolExecute | Permission::McpAdministration => "mcp",
        }
    }

    pub fn action(&self) -> &'static str {
        match self {
            Permission::TaskRead | Permission::JobRead | Permission::ExecutionRead | Permission::RepositoryRead 
            | Permission::DatabaseRead | Permission::UserRead => "read",
            Permission::TaskWrite | Permission::JobWrite | Permission::RepositoryWrite | Permission::DatabaseWrite 
            | Permission::UserWrite => "write",
            Permission::TaskExecute | Permission::McpToolExecute => "execute",
            Permission::TaskDelete | Permission::RepositoryDelete | Permission::UserDelete => "delete",
            Permission::JobCancel | Permission::ExecutionCancel => "cancel",
            Permission::JobRetry => "retry",
            Permission::RepositoryRefresh => "refresh",
            Permission::ServerStatus | Permission::ServerMetrics => "read",
            Permission::ServerConfig | Permission::SystemConfig => "config",
            Permission::ServerRestart => "restart",
            Permission::DatabaseMigrate => "migrate",
            Permission::DatabaseBackup => "backup",
            Permission::UserRoles => "roles",
            Permission::SystemLogs | Permission::ExecutionLogs => "logs",
            Permission::SystemHealth => "health",
            Permission::SystemDebug => "debug",
            Permission::ConsoleAccess | Permission::McpAccess => "access",
            Permission::ConsoleAdmin | Permission::McpAdministration => "admin",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub inherits_from: Vec<String>,     // Role inheritance
    pub is_system_role: bool,           // Cannot be deleted
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Predefined system roles
impl Role {
    pub fn admin() -> Self {
        Self {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full system access".to_string(),
            permissions: vec![
                // All permissions
                Permission::TaskRead, Permission::TaskWrite, Permission::TaskExecute, Permission::TaskDelete,
                Permission::JobRead, Permission::JobWrite, Permission::JobCancel, Permission::JobRetry,
                Permission::ExecutionRead, Permission::ExecutionCancel, Permission::ExecutionLogs,
                Permission::RepositoryRead, Permission::RepositoryWrite, Permission::RepositoryRefresh, Permission::RepositoryDelete,
                Permission::ServerStatus, Permission::ServerMetrics, Permission::ServerConfig, Permission::ServerRestart,
                Permission::DatabaseRead, Permission::DatabaseWrite, Permission::DatabaseMigrate, Permission::DatabaseBackup,
                Permission::UserRead, Permission::UserWrite, Permission::UserDelete, Permission::UserRoles,
                Permission::SystemConfig, Permission::SystemLogs, Permission::SystemHealth, Permission::SystemDebug,
                Permission::ConsoleAccess, Permission::ConsoleAdmin,
                Permission::McpAccess, Permission::McpToolExecute, Permission::McpAdministration,
            ],
            inherits_from: vec![],
            is_system_role: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn developer() -> Self {
        Self {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: "Task development and execution".to_string(),
            permissions: vec![
                Permission::TaskRead, Permission::TaskWrite, Permission::TaskExecute,
                Permission::JobRead, Permission::JobWrite,
                Permission::ExecutionRead, Permission::ExecutionLogs,
                Permission::RepositoryRead,
                Permission::ServerStatus, Permission::ServerMetrics,
                Permission::ConsoleAccess,
                Permission::McpAccess, Permission::McpToolExecute,
            ],
            inherits_from: vec![],
            is_system_role: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn operator() -> Self {
        Self {
            id: "operator".to_string(),
            name: "Operator".to_string(),
            description: "System monitoring and basic operations".to_string(),
            permissions: vec![
                Permission::TaskRead, Permission::TaskExecute,
                Permission::JobRead,
                Permission::ExecutionRead, Permission::ExecutionLogs,
                Permission::RepositoryRead,
                Permission::ServerStatus, Permission::ServerMetrics,
                Permission::SystemHealth,
                Permission::ConsoleAccess,
            ],
            inherits_from: vec![],
            is_system_role: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn readonly() -> Self {
        Self {
            id: "readonly".to_string(),
            name: "Read Only".to_string(),
            description: "Read-only access to system".to_string(),
            permissions: vec![
                Permission::TaskRead,
                Permission::JobRead,
                Permission::ExecutionRead,
                Permission::RepositoryRead,
                Permission::ServerStatus,
            ],
            inherits_from: vec![],
            is_system_role: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
```

#### 2.2 **User and Role Management**
```rust
// ratchet-auth/src/user.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,  // For local accounts
    pub roles: Vec<Role>,
    pub direct_permissions: Vec<Permission>, // Permissions not from roles
    pub is_active: bool,
    pub is_system_user: bool,           // Cannot be deleted
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

impl User {
    pub fn get_all_permissions(&self) -> Vec<Permission> {
        let mut permissions = self.direct_permissions.clone();
        
        // Add permissions from roles
        for role in &self.roles {
            permissions.extend(role.permissions.clone());
        }
        
        // Remove duplicates
        permissions.sort();
        permissions.dedup();
        permissions
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.get_all_permissions().contains(permission)
    }

    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        let user_permissions = self.get_all_permissions();
        permissions.iter().any(|p| user_permissions.contains(p))
    }

    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        let user_permissions = self.get_all_permissions();
        permissions.iter().all(|p| user_permissions.contains(p))
    }

    pub fn can_access_resource(&self, resource: &str, action: &str) -> bool {
        self.get_all_permissions().iter().any(|p| {
            p.resource() == resource && p.action() == action
        })
    }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<User, AuthError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AuthError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AuthError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;
    async fn update_user(&self, user: &User) -> Result<User, AuthError>;
    async fn delete_user(&self, id: &str) -> Result<(), AuthError>;
    async fn list_users(&self, filters: UserFilters) -> Result<Vec<User>, AuthError>;
    async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<(), AuthError>;
    async fn remove_role(&self, user_id: &str, role_id: &str) -> Result<(), AuthError>;
    async fn update_last_login(&self, user_id: &str) -> Result<(), AuthError>;
    async fn increment_failed_login(&self, user_id: &str) -> Result<(), AuthError>;
    async fn reset_failed_login(&self, user_id: &str) -> Result<(), AuthError>;
    async fn lock_user(&self, user_id: &str, until: DateTime<Utc>) -> Result<(), AuthError>;
}
```

### **Phase 3: Policy Engine and Access Control (Week 3-4)**

#### 3.1 **Policy Engine Implementation**
```rust
// ratchet-auth/src/policy.rs
#[derive(Debug, Clone)]
pub struct PolicyEngine {
    policies: Vec<AccessPolicy>,
    context_providers: Vec<Box<dyn ContextProvider>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub priority: i32,                  // Higher priority = evaluated first
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub condition: PolicyCondition,
    pub effect: PolicyEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    Always,
    Never,
    HasPermission(Permission),
    HasRole(String),
    ResourceEquals(String),
    ActionEquals(String),
    IpInRange(String),                  // CIDR notation
    TimeInRange { start: String, end: String }, // HH:MM format
    UserEquals(String),
    And(Vec<PolicyCondition>),
    Or(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
    Require2FA,
    RequireApproval,
    RateLimit { requests_per_minute: u32 },
    AuditLog { level: AuditLevel },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLevel {
    Info,
    Warning,
    Critical,
}

impl PolicyEngine {
    pub async fn evaluate_access(
        &self,
        user: &UserContext,
        resource: &str,
        action: &str,
        context: &AccessContext,
    ) -> Result<AccessDecision, AuthError> {
        let mut decisions = Vec::new();
        
        // Evaluate all policies in priority order
        let mut sorted_policies = self.policies.clone();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for policy in sorted_policies {
            if !policy.is_active {
                continue;
            }
            
            for rule in policy.rules {
                if self.evaluate_condition(&rule.condition, user, resource, action, context).await? {
                    decisions.push(PolicyDecision {
                        policy_id: policy.id.clone(),
                        rule_effect: rule.effect.clone(),
                    });
                }
            }
        }
        
        // Determine final access decision
        self.resolve_decisions(decisions)
    }

    async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        user: &UserContext,
        resource: &str,
        action: &str,
        context: &AccessContext,
    ) -> Result<bool, AuthError> {
        match condition {
            PolicyCondition::Always => Ok(true),
            PolicyCondition::Never => Ok(false),
            PolicyCondition::HasPermission(permission) => {
                Ok(user.permissions.contains(permission))
            }
            PolicyCondition::HasRole(role) => {
                Ok(user.roles.contains(role))
            }
            PolicyCondition::ResourceEquals(res) => Ok(resource == res),
            PolicyCondition::ActionEquals(act) => Ok(action == act),
            PolicyCondition::IpInRange(cidr) => {
                self.check_ip_in_range(&context.client_ip, cidr)
            }
            PolicyCondition::TimeInRange { start, end } => {
                self.check_time_in_range(start, end)
            }
            PolicyCondition::UserEquals(user_id) => Ok(&user.user_id == user_id),
            PolicyCondition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, user, resource, action, context).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PolicyCondition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, user, resource, action, context).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PolicyCondition::Not(condition) => {
                Ok(!self.evaluate_condition(condition, user, resource, action, context).await?)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessContext {
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub resource_id: Option<String>,
    pub additional_context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AccessDecision {
    pub allowed: bool,
    pub requires_2fa: bool,
    pub requires_approval: bool,
    pub rate_limits: Vec<RateLimit>,
    pub audit_requirements: Vec<AuditLevel>,
    pub reasons: Vec<String>,
}
```

#### 3.2 **Middleware Integration**
```rust
// ratchet-auth/src/middleware.rs
use axum::{extract::Request, middleware::Next, response::Response};

#[derive(Clone)]
pub struct AuthMiddleware {
    auth_manager: Arc<AuthManager>,
    policy_engine: Arc<PolicyEngine>,
    audit_logger: Arc<AuditLogger>,
}

impl AuthMiddleware {
    pub async fn authenticate_and_authorize(
        &self,
        mut req: Request,
        next: Next,
    ) -> Result<Response, AuthError> {
        let auth_context = self.extract_auth_context(&req)?;
        
        // 1. Authentication
        let user_context = match self.authenticate_request(&req).await {
            Ok(user) => user,
            Err(e) => {
                self.audit_logger.log_auth_failure(&auth_context, &e).await;
                return Err(e);
            }
        };

        // 2. Authorization
        let resource = self.extract_resource(&req);
        let action = self.extract_action(&req);
        
        let access_decision = self.policy_engine
            .evaluate_access(&user_context, &resource, &action, &auth_context)
            .await?;

        if !access_decision.allowed {
            self.audit_logger.log_access_denied(&user_context, &resource, &action).await;
            return Err(AuthError::AccessDenied);
        }

        // 3. Additional requirements
        if access_decision.requires_2fa {
            self.verify_2fa(&user_context, &req).await?;
        }

        if access_decision.requires_approval {
            self.check_approval(&user_context, &resource, &action).await?;
        }

        // 4. Rate limiting
        for rate_limit in access_decision.rate_limits {
            self.check_rate_limit(&user_context, &rate_limit).await?;
        }

        // 5. Audit logging
        for audit_level in access_decision.audit_requirements {
            self.audit_logger.log_access(&user_context, &resource, &action, audit_level).await;
        }

        // Add user context to request
        req.extensions_mut().insert(user_context);
        
        let response = next.run(req).await;
        Ok(response)
    }

    async fn authenticate_request(&self, req: &Request) -> Result<UserContext, AuthError> {
        // Try different authentication methods in order
        
        // 1. JWT Bearer token
        if let Some(auth_header) = req.headers().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    return self.auth_manager.validate_jwt(token).await;
                }
            }
        }

        // 2. API Key
        if let Some(api_key) = req.headers().get("x-api-key") {
            if let Ok(key_str) = api_key.to_str() {
                let client_ip = self.extract_client_ip(req);
                return self.auth_manager.validate_api_key(key_str, &client_ip).await;
            }
        }

        // 3. Session cookie
        if let Some(cookie_header) = req.headers().get("cookie") {
            if let Ok(cookie_str) = cookie_header.to_str() {
                if let Some(session_id) = self.extract_session_id(cookie_str) {
                    return self.auth_manager.validate_session(&session_id).await;
                }
            }
        }

        Err(AuthError::NoCredentials)
    }
}

// Usage in API setup
pub fn create_authenticated_router() -> Router {
    let auth_middleware = AuthMiddleware::new(auth_manager, policy_engine, audit_logger);
    
    Router::new()
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/:id", get(get_task).put(update_task).delete(delete_task))
        .route("/graphql", post(graphql_handler))
        .route("/mcp/*path", any(mcp_handler))
        .layer(middleware::from_fn_with_state(
            auth_middleware.clone(),
            |State(auth): State<AuthMiddleware>, req, next| async move {
                auth.authenticate_and_authorize(req, next).await
            }
        ))
}
```

### **Phase 4: Session Management and 2FA (Week 4-5)**

#### 4.1 **Session Management**
```rust
// ratchet-auth/src/session.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub is_active: bool,
    pub csrf_token: String,
    pub data: HashMap<String, String>, // Session data
}

#[derive(Clone)]
pub struct SessionManager {
    storage: Arc<dyn SessionStorage>,
    config: SessionConfig,
    csrf_manager: CsrfManager,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub timeout_minutes: u32,
    pub extend_on_access: bool,
    pub require_csrf: bool,
    pub secure_cookies: bool,
    pub same_site: SameSite,
}

impl SessionManager {
    pub async fn create_session(
        &self,
        user_id: &str,
        client_ip: &str,
        user_agent: Option<&str>,
    ) -> Result<Session, AuthError> {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::minutes(self.config.timeout_minutes as i64),
            last_accessed_at: Utc::now(),
            client_ip: client_ip.to_string(),
            user_agent: user_agent.map(|s| s.to_string()),
            is_active: true,
            csrf_token: self.csrf_manager.generate_token()?,
            data: HashMap::new(),
        };

        self.storage.store_session(&session).await?;
        Ok(session)
    }

    pub async fn validate_session(&self, session_id: &str) -> Result<Session, AuthError> {
        let mut session = self.storage.get_session(session_id).await?
            .ok_or(AuthError::InvalidSession)?;

        // Check if session is expired
        if Utc::now() > session.expires_at {
            self.storage.delete_session(session_id).await?;
            return Err(AuthError::SessionExpired);
        }

        // Check if session is active
        if !session.is_active {
            return Err(AuthError::SessionInactive);
        }

        // Extend session if configured
        if self.config.extend_on_access {
            session.expires_at = Utc::now() + chrono::Duration::minutes(self.config.timeout_minutes as i64);
            session.last_accessed_at = Utc::now();
            self.storage.update_session(&session).await?;
        }

        Ok(session)
    }

    pub async fn invalidate_session(&self, session_id: &str) -> Result<(), AuthError> {
        self.storage.delete_session(session_id).await
    }

    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<(), AuthError> {
        self.storage.delete_user_sessions(user_id).await
    }
}
```

#### 4.2 **Two-Factor Authentication**
```rust
// ratchet-auth/src/two_factor.rs
#[derive(Debug, Clone)]
pub struct TwoFactorManager {
    storage: Arc<dyn TwoFactorStorage>,
    config: TwoFactorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwoFactorMethod {
    TOTP {
        secret: String,
        backup_codes: Vec<String>,
    },
    SMS {
        phone_number: String,
        provider: SmsProvider,
    },
    Email {
        email: String,
    },
    WebAuthn {
        credential_id: String,
        public_key: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSetup {
    pub user_id: String,
    pub method: TwoFactorMethod,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_verified: bool,
}

impl TwoFactorManager {
    pub async fn setup_totp(&self, user_id: &str) -> Result<(String, Vec<String>), AuthError> {
        // Generate TOTP secret
        let secret = self.generate_totp_secret()?;
        
        // Generate backup codes
        let backup_codes = self.generate_backup_codes()?;
        
        let setup = TwoFactorSetup {
            user_id: user_id.to_string(),
            method: TwoFactorMethod::TOTP {
                secret: secret.clone(),
                backup_codes: backup_codes.clone(),
            },
            is_primary: true,
            created_at: Utc::now(),
            last_used_at: None,
            is_verified: false,
        };

        self.storage.store_setup(&setup).await?;
        Ok((secret, backup_codes))
    }

    pub async fn verify_totp(&self, user_id: &str, code: &str) -> Result<bool, AuthError> {
        let setup = self.storage.get_user_setup(user_id).await?
            .ok_or(AuthError::TwoFactorNotSetup)?;

        match &setup.method {
            TwoFactorMethod::TOTP { secret, backup_codes } => {
                // Verify TOTP code
                if self.verify_totp_code(secret, code)? {
                    self.storage.update_last_used(user_id).await?;
                    return Ok(true);
                }

                // Check backup codes
                if backup_codes.contains(&code.to_string()) {
                    self.storage.consume_backup_code(user_id, code).await?;
                    return Ok(true);
                }

                Ok(false)
            }
            _ => Err(AuthError::InvalidTwoFactorMethod),
        }
    }

    pub async fn require_2fa(&self, user: &UserContext) -> bool {
        // Check if user has 2FA enabled
        if let Ok(Some(_)) = self.storage.get_user_setup(&user.user_id).await {
            return true;
        }

        // Check if user role requires 2FA
        user.roles.iter().any(|role| {
            matches!(role.as_str(), "admin" | "operator")
        })
    }
}
```

### **Phase 5: Integration and Testing (Week 5-6)**

#### 5.1 **Complete Integration Example**
```rust
// Example: Protecting a REST API endpoint
#[axum::debug_handler]
pub async fn create_task(
    Extension(user): Extension<UserContext>,
    Json(task_data): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    // User is already authenticated and authorized by middleware
    
    // Additional permission check for specific operation
    if !user.has_permission(&Permission::TaskWrite) {
        return Err(ApiError::Forbidden);
    }

    // Resource-level authorization
    if task_data.is_system_task && !user.has_permission(&Permission::SystemConfig) {
        return Err(ApiError::Forbidden);
    }

    // Proceed with task creation
    let task = task_service.create_task(task_data, &user).await?;
    
    // Audit log the operation
    audit_logger.log_task_creation(&user, &task).await;
    
    Ok(Json(TaskResponse::from(task)))
}

// Example: Protecting GraphQL resolvers
impl Mutation {
    #[graphql(guard = "RequirePermission::new(Permission::TaskExecute)")]
    async fn execute_task(
        &self,
        ctx: &Context<'_>,
        task_id: String,
        input: String,
    ) -> Result<ExecutionResult> {
        let user = ctx.data::<UserContext>()?;
        
        // Additional checks
        let task = task_service.get_task(&task_id).await?;
        if task.requires_admin && !user.has_permission(&Permission::SystemConfig) {
            return Err("Admin permission required".into());
        }

        task_service.execute_task(&task_id, &input, user).await
    }
}

// Example: Protecting MCP tools
impl McpToolHandler {
    pub async fn handle_tool_call(
        &self,
        tool_name: &str,
        params: Value,
        user: &UserContext,
    ) -> Result<Value, McpError> {
        // Check MCP access permission
        if !user.has_permission(&Permission::McpToolExecute) {
            return Err(McpError::Forbidden);
        }

        // Tool-specific authorization
        match tool_name {
            "execute_task" => {
                if !user.has_permission(&Permission::TaskExecute) {
                    return Err(McpError::Forbidden);
                }
            }
            "get_execution_logs" => {
                if !user.has_permission(&Permission::ExecutionLogs) {
                    return Err(McpError::Forbidden);
                }
            }
            _ => {}
        }

        self.execute_tool(tool_name, params, user).await
    }
}
```

#### 5.2 **Configuration Management**
```yaml
# auth.yaml - Complete authentication configuration
authentication:
  # JWT Configuration
  jwt:
    secret_key: "${JWT_SECRET}"
    issuer: "ratchet-server"
    audience: "ratchet-api"
    expiration_hours: 24
    algorithm: "HS256"

  # API Key Configuration
  api_keys:
    enabled: true
    key_length: 32
    prefix_length: 8
    max_keys_per_user: 10
    default_expiry_days: 90
    require_ip_restriction: false

  # Session Configuration
  sessions:
    enabled: true
    timeout_minutes: 480  # 8 hours
    extend_on_access: true
    require_csrf: true
    secure_cookies: true
    same_site: "strict"

  # OAuth2 Configuration
  oauth2:
    enabled: false
    providers:
      google:
        client_id: "${GOOGLE_CLIENT_ID}"
        client_secret: "${GOOGLE_CLIENT_SECRET}"
        redirect_uri: "https://ratchet.example.com/auth/google/callback"
      github:
        client_id: "${GITHUB_CLIENT_ID}"
        client_secret: "${GITHUB_CLIENT_SECRET}"
        redirect_uri: "https://ratchet.example.com/auth/github/callback"

  # Two-Factor Authentication
  two_factor:
    enabled: true
    required_for_roles: ["admin", "operator"]
    backup_codes_count: 10
    totp_window: 1
    rate_limit_attempts: 5

authorization:
  # Default policies
  default_policies:
    - id: "require_auth"
      name: "Require Authentication"
      rules:
        - condition: "Always"
          effect: "RequireAuth"
      priority: 1000

    - id: "admin_only_system"
      name: "Admin Only System Operations"
      rules:
        - condition:
            And:
              - ResourceEquals: "system"
              - Not:
                  HasRole: "admin"
          effect: "Deny"
      priority: 900

    - id: "business_hours_only"
      name: "Restrict Operations to Business Hours"
      rules:
        - condition:
            And:
              - ActionEquals: "delete"
              - Not:
                  TimeInRange:
                    start: "09:00"
                    end: "17:00"
          effect: "RequireApproval"
      priority: 500

  # Rate limiting policies
  rate_limits:
    default_per_user: 1000  # requests per hour
    api_key_multiplier: 5   # API keys get 5x limit
    admin_multiplier: 10    # Admins get 10x limit

audit:
  enabled: true
  log_all_access: true
  log_failed_auth: true
  retention_days: 90
  export_format: "json"
  storage: "database"  # or "file" or "elasticsearch"
```

---

## 🧪 Testing Strategy

### **Unit Testing**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_generation_and_validation() {
        let jwt_manager = JwtManager::new("test-secret", "test-issuer".to_string()).unwrap();
        let user = create_test_user();
        
        let token = jwt_manager.generate_token(&user).unwrap();
        let claims = jwt_manager.validate_token(&token).unwrap();
        
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, user.username);
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let user = create_test_user_with_role("developer");
        
        assert!(user.has_permission(&Permission::TaskRead));
        assert!(user.has_permission(&Permission::TaskExecute));
        assert!(!user.has_permission(&Permission::UserDelete));
    }

    #[tokio::test]
    async fn test_policy_evaluation() {
        let policy_engine = create_test_policy_engine();
        let user = create_test_user();
        let context = create_test_context();
        
        let decision = policy_engine
            .evaluate_access(&user, "task", "read", &context)
            .await
            .unwrap();
        
        assert!(decision.allowed);
    }

    #[tokio::test]
    async fn test_api_key_validation() {
        let api_key_manager = create_test_api_key_manager().await;
        let (key, api_key_info) = api_key_manager
            .create_key("user123", "test-key", vec![Permission::TaskRead])
            .await
            .unwrap();
        
        let user_context = api_key_manager
            .validate_key(&key, "192.168.1.1")
            .await
            .unwrap();
        
        assert_eq!(user_context.user_id, "user123");
        assert!(user_context.permissions.contains(&Permission::TaskRead));
    }
}
```

### **Integration Testing**
```rust
#[tokio::test]
async fn test_end_to_end_authentication() {
    let app = create_test_app().await;
    
    // Test unauthenticated request
    let response = app
        .oneshot(Request::builder()
            .uri("/api/v1/tasks")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    // Test authenticated request
    let token = create_test_jwt_token("user123", vec!["developer"]);
    let response = app
        .oneshot(Request::builder()
            .uri("/api/v1/tasks")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_graphql_authorization() {
    let app = create_test_app().await;
    
    let query = r#"
        mutation {
            executeTask(taskId: "test-task", input: "{}") {
                success
                output
            }
        }
    "#;
    
    // Test with insufficient permissions
    let token = create_test_jwt_token("user123", vec!["readonly"]);
    let response = app
        .oneshot(Request::builder()
            .uri("/graphql")
            .method("POST")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::json!({ "query": query }).to_string()))
            .unwrap())
        .await
        .unwrap();
    
    let body = response.into_body();
    let response_data: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(response_data["errors"].as_array().unwrap().len() > 0);
}
```

---

## 📊 Performance Considerations

### **Authentication Performance**
- **JWT validation**: ~1-2ms per request (includes signature verification)
- **API key lookup**: ~2-5ms per request (database lookup with caching)
- **Session validation**: ~3-7ms per request (depends on storage backend)
- **Policy evaluation**: ~5-15ms per request (complex policies)

### **Optimization Strategies**
1. **Caching**: Cache user contexts, roles, and policy decisions
2. **Connection pooling**: Efficient database connections for auth data
3. **Async processing**: Non-blocking authentication and authorization
4. **Batch operations**: Bulk permission checks for multiple resources

### **Scalability Metrics**
```rust
pub struct AuthMetrics {
    pub auth_requests_per_second: Counter,
    pub auth_latency: Histogram,
    pub auth_failures: Counter,
    pub policy_evaluations: Counter,
    pub cache_hit_rate: Gauge,
}
```

---

## 🚀 Migration Strategy

### **Phased Rollout**
1. **Phase 1**: Enable authentication with permissive policies
2. **Phase 2**: Add role-based authorization with granular permissions
3. **Phase 3**: Implement advanced policies and 2FA
4. **Phase 4**: Full security hardening and monitoring

### **Backward Compatibility**
```rust
// Support both authenticated and legacy access during migration
pub enum AuthMode {
    Disabled,     // Legacy mode - no auth required
    Optional,     // Auth preferred but not required
    Required,     // Auth required for all operations
    Strict,       // Auth + authorization required
}

impl AuthMiddleware {
    pub async fn handle_request_with_mode(
        &self,
        req: Request,
        next: Next,
        mode: AuthMode,
    ) -> Result<Response, AuthError> {
        match mode {
            AuthMode::Disabled => next.run(req).await,
            AuthMode::Optional => {
                match self.authenticate_request(&req).await {
                    Ok(user) => {
                        req.extensions_mut().insert(user);
                        next.run(req).await
                    }
                    Err(_) => {
                        // Continue without authentication
                        req.extensions_mut().insert(UserContext::anonymous());
                        next.run(req).await
                    }
                }
            }
            AuthMode::Required | AuthMode::Strict => {
                self.authenticate_and_authorize(req, next).await
            }
        }
    }
}
```

---

## 📋 Implementation Checklist

### **Phase 1: Authentication Foundation (Week 1-2)**
- [ ] JWT authentication implementation
- [ ] API key authentication system
- [ ] Basic user management
- [ ] Authentication middleware
- [ ] Database schema and migrations
- [ ] Configuration management

### **Phase 2: Role and Permission System (Week 2-3)**
- [ ] Role and permission definitions
- [ ] User-role assignment system
- [ ] Permission checking utilities
- [ ] System role creation (admin, developer, operator, readonly)
- [ ] Role inheritance mechanism
- [ ] Permission caching system

### **Phase 3: Policy Engine (Week 3-4)**
- [ ] Policy definition language
- [ ] Policy evaluation engine
- [ ] Context providers implementation
- [ ] Access decision logic
- [ ] Policy storage and management
- [ ] Policy testing framework

### **Phase 4: Advanced Features (Week 4-5)**
- [ ] Session management system
- [ ] Two-factor authentication (TOTP)
- [ ] OAuth2 integration framework
- [ ] Rate limiting integration
- [ ] Audit logging system
- [ ] Security monitoring

### **Phase 5: Integration (Week 5-6)**
- [ ] REST API middleware integration
- [ ] GraphQL authorization guards
- [ ] MCP authentication integration
- [ ] Console authentication
- [ ] Configuration management
- [ ] Migration utilities

### **Phase 6: Testing and Documentation (Week 6-8)**
- [ ] Unit test suite (>90% coverage)
- [ ] Integration test suite
- [ ] Performance benchmarks
- [ ] Security testing
- [ ] Documentation and examples
- [ ] Migration guides

---

## 🎯 Success Criteria

### **Security Objectives**
- [ ] **Zero unauthorized access** to any API endpoints
- [ ] **Comprehensive audit trail** for all security events
- [ ] **Fine-grained permission control** with role-based access
- [ ] **Multi-factor authentication** for privileged accounts
- [ ] **Session security** with proper timeout and invalidation

### **Performance Objectives**
- [ ] **Authentication latency** < 10ms for 95th percentile
- [ ] **Authorization overhead** < 15ms per request
- [ ] **System throughput** maintains 95% of current performance
- [ ] **Cache hit rate** > 90% for frequently accessed permissions

### **Operational Objectives**
- [ ] **Zero-downtime migration** from unauthenticated to authenticated
- [ ] **Self-service user management** for administrators
- [ ] **Comprehensive monitoring** of authentication and authorization
- [ ] **Clear troubleshooting procedures** for access issues

---

**Document Approval**: Security Team, Architecture Team, Product Team  
**Implementation Owner**: Security Team  
**Review Schedule**: Weekly during implementation, bi-weekly post-deployment  
**Security Review**: Required before production deployment