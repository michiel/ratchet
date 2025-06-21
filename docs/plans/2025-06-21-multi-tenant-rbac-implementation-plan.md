# Multi-Tenant RBAC Implementation Plan

## Executive Summary

The current Ratchet user model implements basic single-tenant RBAC with four fixed roles (Admin, User, ReadOnly, Service). To support multi-tenancy with platform operators and separate tenants, we need a comprehensive redesign of the authorization system that provides tenant isolation, configurable permissions, and role management via both API and YAML configuration.

**Recommendation**: Implement a hybrid approach using **Casbin-RS** for proven multi-tenant authorization combined with custom tenant management, providing flexible permission systems while maintaining security isolation between tenants.

## Current State Analysis

### Existing User Model Assessment

**Current Implementation**:
- Simple enum-based roles: `Admin`, `User`, `ReadOnly`, `Service`
- Basic permission checks with hardcoded role capabilities
- No tenant concept or data isolation
- Single-table user storage without tenant scoping

**Current Limitations**:
- No multi-tenancy support
- Fixed roles cannot be customized
- No granular permission system
- Platform vs tenant administration not distinguished
- No API-based role management
- No YAML configuration for permissions

### Current Permission Structure

```rust
// Current basic permission checks in users.rs
pub fn can_admin(&self) -> bool { matches!(self, UserRole::Admin) }
pub fn can_write(&self) -> bool { matches!(self, UserRole::Admin | UserRole::User | UserRole::Service) }
pub fn can_read(&self) -> bool { true } // All roles can read
pub fn can_manage_users(&self) -> bool { matches!(self, UserRole::Admin) }
pub fn can_execute_tasks(&self) -> bool { matches!(self, UserRole::Admin | UserRole::User | UserRole::Service) }
```

**Issues**:
- Binary permission model (can/cannot)
- No resource-specific permissions
- No tenant-scoped permissions
- No custom role support

## Multi-Tenant Architecture Requirements

### Platform vs Tenant Hierarchy

**Platform Level**:
- **Platform Operators**: Global system administration
- **Platform Users**: Cross-tenant monitoring, support
- **System Services**: Background processes, monitoring

**Tenant Level**:
- **Tenant Administrators**: Full control within tenant scope
- **Tenant Users**: Standard task execution and management
- **Tenant Viewers**: Read-only access to tenant resources
- **Custom Roles**: Tenant-defined roles with specific permissions

### Required Permission Granularity

**Resource Types**:
- `tasks` - Task definitions and management
- `executions` - Task execution monitoring and control
- `jobs` - Job queue management
- `schedules` - Schedule creation and management
- `users` - User management within tenant
- `roles` - Role and permission management
- `metrics` - System and tenant metrics
- `configurations` - Tenant settings and configurations

**Action Types**:
- `create` - Create new resources
- `read` - View and list resources
- `update` - Modify existing resources
- `delete` - Remove resources
- `execute` - Execute tasks or trigger operations
- `manage` - Administrative operations (user management, etc.)

**Scope Types**:
- `platform` - Platform-wide operations
- `tenant` - Tenant-scoped operations
- `self` - User's own resources only

## Implementation Options Analysis

### Option 1: Casbin-RS Integration (Recommended)

**Architecture**:
```
Platform Operator Domain: "platform"
├── Roles: platform_admin, platform_monitor, platform_service
├── Resources: all_tenants, system_metrics, platform_config
└── Permissions: create:tenant, read:all_metrics, manage:platform_config

Tenant Domain: "tenant_{tenant_id}"
├── Roles: tenant_admin, tenant_user, tenant_viewer, custom_roles
├── Resources: tenant_tasks, tenant_executions, tenant_users
└── Permissions: create:tasks, execute:jobs, read:schedules
```

**Benefits**:
- ✅ Proven multi-tenant authorization with domain separation
- ✅ Flexible policy configuration via YAML and API
- ✅ High performance with policy caching
- ✅ Extensive documentation and community support
- ✅ Supports complex permission inheritance
- ✅ Built-in role and permission management

**Implementation Effort**: **Medium** (2-3 sprints)

### Option 2: AWS Cedar Integration

**Architecture**:
```
Platform Context:
Principal: PlatformOperator
Action: "ManageTenant"
Resource: Tenant::*
Condition: hasRole("platform_admin")

Tenant Context:
Principal: User::{user_id}@Tenant::{tenant_id}
Action: "ExecuteTask"
Resource: Task::{task_id}@Tenant::{tenant_id}
Condition: hasPermission("execute:tasks") && tenantMember(tenant_id)
```

**Benefits**:
- ✅ Modern policy-as-code approach
- ✅ Formal verification of policies
- ✅ Excellent performance
- ✅ Native Rust implementation

**Drawbacks**:
- ❌ Steeper learning curve (Cedar policy language)
- ❌ Smaller ecosystem than Casbin
- ❌ More complex for simple RBAC scenarios

**Implementation Effort**: **High** (3-4 sprints)

### Option 3: Custom Implementation

**Architecture**:
```sql
-- Tenant isolation
tenants(id, name, created_at, settings)

-- User tenant membership
user_tenants(user_id, tenant_id, role_id, created_at)

-- Flexible role system
roles(id, name, tenant_id, is_platform_role, permissions)
permissions(id, resource, action, scope, conditions)
role_permissions(role_id, permission_id)
```

**Benefits**:
- ✅ Perfect fit for exact requirements
- ✅ No external dependencies
- ✅ Complete control over performance
- ✅ Tight integration with existing codebase

**Drawbacks**:
- ❌ High development and maintenance cost
- ❌ Security risks from custom implementation
- ❌ Need to handle complex authorization edge cases

**Implementation Effort**: **Very High** (5-6 sprints)

## Recommended Implementation Plan

### Phase 1: Foundation and Casbin Integration (Sprint 1-2)

**Database Schema Changes**:
```sql
-- Add tenant support
CREATE TABLE tenants (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    display_name VARCHAR(255),
    settings JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Platform vs tenant user roles
CREATE TABLE user_tenant_roles (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    tenant_id INTEGER REFERENCES tenants(id) ON DELETE CASCADE,
    role_name VARCHAR(100) NOT NULL,
    assigned_at TIMESTAMPTZ DEFAULT NOW(),
    assigned_by INTEGER REFERENCES users(id),
    UNIQUE(user_id, tenant_id, role_name)
);

-- Platform roles (tenant_id is NULL for platform roles)
CREATE TABLE platform_user_roles (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    role_name VARCHAR(100) NOT NULL,
    assigned_at TIMESTAMPTZ DEFAULT NOW(),
    assigned_by INTEGER REFERENCES users(id),
    UNIQUE(user_id, role_name)
);

-- Casbin policy storage
CREATE TABLE casbin_rules (
    id SERIAL PRIMARY KEY,
    ptype VARCHAR(100) NOT NULL,
    v0 VARCHAR(100),
    v1 VARCHAR(100),
    v2 VARCHAR(100),
    v3 VARCHAR(100),
    v4 VARCHAR(100),
    v5 VARCHAR(100)
);
```

**Standard Role Definitions**:
```yaml
# Platform roles
platform_roles:
  platform_admin:
    permissions:
      - "create:tenant"
      - "read:all_metrics" 
      - "manage:platform_config"
      - "manage:platform_users"
  
  platform_monitor:
    permissions:
      - "read:all_metrics"
      - "read:platform_logs"

# Tenant role templates
tenant_roles:
  tenant_admin:
    permissions:
      - "create:tasks"
      - "read:tasks"
      - "update:tasks"
      - "delete:tasks"
      - "execute:tasks"
      - "manage:users"
      - "manage:roles"
      - "read:metrics"
  
  tenant_user:
    permissions:
      - "create:tasks"
      - "read:tasks"
      - "update:own_tasks"
      - "execute:tasks"
      - "read:own_executions"
  
  tenant_viewer:
    permissions:
      - "read:tasks"
      - "read:executions"
      - "read:metrics"
```

**Casbin Model Configuration**:
```ini
[request_definition]
r = sub, obj, act, tenant

[policy_definition]
p = sub, obj, act, tenant

[role_definition]
g = _, _, _
g2 = _, _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub, r.tenant) && r.obj == p.obj && r.act == p.act && (r.tenant == p.tenant || p.tenant == "*")
```

### Phase 2: API and Configuration Support (Sprint 2-3)

**Tenant Management API**:
```rust
// REST endpoints for tenant management
POST /api/v1/platform/tenants
GET /api/v1/platform/tenants
GET /api/v1/platform/tenants/{id}
PUT /api/v1/platform/tenants/{id}
DELETE /api/v1/platform/tenants/{id}

// Tenant user management
POST /api/v1/tenants/{tenant_id}/users
GET /api/v1/tenants/{tenant_id}/users
PUT /api/v1/tenants/{tenant_id}/users/{user_id}/roles
```

**Permission Management API**:
```rust
// Custom role management within tenants
POST /api/v1/tenants/{tenant_id}/roles
GET /api/v1/tenants/{tenant_id}/roles
PUT /api/v1/tenants/{tenant_id}/roles/{role_id}
DELETE /api/v1/tenants/{tenant_id}/roles/{role_id}

// Permission queries
GET /api/v1/permissions/check?resource={resource}&action={action}&tenant={tenant}
GET /api/v1/users/me/permissions
```

**YAML Configuration Support**:
```yaml
# In ratchet configuration
rbac:
  default_platform_roles:
    - platform_admin
    - platform_monitor
  
  default_tenant_roles:
    - tenant_admin
    - tenant_user
    - tenant_viewer
  
  custom_permissions:
    tenant_developer:
      permissions:
        - "create:tasks"
        - "execute:tasks"
        - "read:executions"
        - "update:own_tasks"
```

### Phase 3: Resource Isolation and Middleware (Sprint 3-4)

**Tenant-Aware Resource Access**:
```rust
// Modified repository interfaces
#[async_trait]
pub trait TenantAwareRepository<T> {
    async fn find_by_tenant(&self, tenant_id: TenantId, filters: Filters) -> Result<Vec<T>>;
    async fn create_for_tenant(&self, tenant_id: TenantId, item: T) -> Result<T>;
    async fn check_tenant_access(&self, tenant_id: TenantId, resource_id: ResourceId) -> Result<bool>;
}
```

**Authorization Middleware**:
```rust
// Request context with tenant information
pub struct AuthContext {
    pub user_id: UserId,
    pub tenant_id: Option<TenantId>,
    pub platform_roles: Vec<String>,
    pub tenant_roles: HashMap<TenantId, Vec<String>>,
    pub permissions: PermissionSet,
}

// Middleware for permission checking
pub async fn check_permission_middleware(
    req: Request,
    enforcer: Arc<Enforcer>,
) -> Result<Request, AuthError> {
    let auth_context = extract_auth_context(&req)?;
    let required_permission = extract_required_permission(&req)?;
    
    if !enforcer.enforce(&auth_context, &required_permission).await? {
        return Err(AuthError::InsufficientPermissions);
    }
    
    Ok(req)
}
```

### Phase 4: Migration and Testing (Sprint 4)

**Data Migration Strategy**:
1. **Backward Compatibility**: Maintain existing single-tenant mode during transition
2. **Default Tenant**: Create a default tenant for existing users
3. **Role Migration**: Map existing roles to new permission system
4. **Gradual Rollout**: Feature flag for multi-tenant mode

**Testing Strategy**:
- **Unit Tests**: Permission logic and role assignment
- **Integration Tests**: API endpoints with different role combinations
- **Security Tests**: Tenant isolation and privilege escalation prevention
- **Performance Tests**: Authorization decision performance under load

## Security Considerations

### Tenant Isolation

**Database Level**:
- All tenant-specific resources include `tenant_id` foreign key
- Row-level security policies where supported
- Mandatory tenant filtering in all queries

**Application Level**:
- Request context always includes tenant information
- Middleware validates tenant access before resource operations
- No cross-tenant data leakage in API responses

### Permission Validation

**Defense in Depth**:
- API layer permission checks
- Service layer validation
- Repository layer tenant filtering
- Database constraints and policies

**Audit and Monitoring**:
- Log all permission checks and failures
- Monitor for unusual permission patterns
- Track role assignments and modifications

## Performance Considerations

### Casbin Optimization

**Policy Caching**:
- In-memory policy cache with TTL
- Redis-backed policy cache for multi-instance deployments
- Efficient policy reloading on changes

**Query Optimization**:
- Index casbin_rules table for fast policy lookups
- Batch permission checks where possible
- Cache user permissions for request duration

### Database Performance

**Indexing Strategy**:
```sql
-- Tenant-aware indexes
CREATE INDEX idx_tasks_tenant_id ON tasks(tenant_id);
CREATE INDEX idx_executions_tenant_id ON executions(tenant_id);
CREATE INDEX idx_user_tenant_roles_lookup ON user_tenant_roles(user_id, tenant_id);
CREATE INDEX idx_casbin_rules_lookup ON casbin_rules(ptype, v0, v1, v2);
```

## Implementation Timeline

### Sprint 1 (Weeks 1-2): Foundation
- Database schema design and migration
- Basic Casbin integration
- Platform vs tenant role concept implementation

### Sprint 2 (Weeks 3-4): Core Authorization
- Casbin policy configuration
- Basic permission checking middleware
- Standard role definitions

### Sprint 3 (Weeks 5-6): API Development
- Tenant management API endpoints
- Role and permission management API
- YAML configuration loading

### Sprint 4 (Weeks 7-8): Integration and Migration
- Resource isolation implementation
- Existing data migration
- Comprehensive testing

### Sprint 5 (Weeks 9-10): Refinement and Documentation
- Performance optimization
- Security auditing
- Documentation and examples

## Risk Assessment and Mitigation

### High Risks

**Security Vulnerabilities**:
- *Risk*: Tenant data leakage, privilege escalation
- *Mitigation*: Comprehensive security testing, code review, penetration testing

**Performance Impact**:
- *Risk*: Authorization checks slow down API requests
- *Mitigation*: Aggressive caching, performance testing, optimization

**Migration Complexity**:
- *Risk*: Data loss or corruption during migration
- *Mitigation*: Thorough testing, rollback plans, gradual migration

### Medium Risks

**Casbin Learning Curve**:
- *Risk*: Team unfamiliar with Casbin concepts
- *Mitigation*: Training, documentation, pair programming

**Configuration Complexity**:
- *Risk*: Complex permission configurations lead to errors
- *Mitigation*: Validation tooling, clear documentation, examples

## Success Metrics

### Functional Metrics
- ✅ Complete tenant isolation (no cross-tenant data access)
- ✅ Configurable roles via API and YAML
- ✅ Sub-100ms authorization decision latency
- ✅ Zero security vulnerabilities in penetration testing

### Operational Metrics
- ✅ Successful migration of existing users without data loss
- ✅ Platform operators can manage multiple tenants efficiently
- ✅ Tenant administrators can manage their users and permissions
- ✅ API endpoints support both single-tenant (backward compatibility) and multi-tenant modes

## Conclusion

The recommended approach using **Casbin-RS** provides a robust foundation for multi-tenant RBAC while balancing implementation complexity with feature completeness. The phased implementation plan ensures gradual migration with minimal risk to existing functionality while establishing a scalable permission system suitable for platform and tenant-level administration.

The hybrid approach combines proven authorization technology (Casbin) with custom tenant management, providing the flexibility needed for API and YAML configuration while maintaining security isolation and performance requirements.