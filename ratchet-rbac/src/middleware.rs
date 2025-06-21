//! HTTP middleware for RBAC authorization

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::{
    auth::AuthContext,
    error::{RbacError, RbacResult},
    models::{ActionType, ResourceType},
    permissions::PermissionChecker,
};

/// RBAC middleware state
#[derive(Clone)]
pub struct RbacMiddleware {
    permission_checker: PermissionChecker,
}

impl RbacMiddleware {
    /// Create new RBAC middleware
    pub fn new(permission_checker: PermissionChecker) -> Self {
        Self { permission_checker }
    }

    /// Middleware function for route-level authorization
    pub async fn authorize_request(
        State(middleware): State<Arc<RbacMiddleware>>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // Extract auth context from request extensions
        let auth_context = request
            .extensions()
            .get::<AuthContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?
            .clone();

        // Extract required permission from request extensions
        let required_permission = request
            .extensions()
            .get::<RequiredPermission>()
            .cloned();

        if let Some(permission) = required_permission {
            // Check if user has required permission
            let has_permission = middleware
                .permission_checker
                .check(
                    &auth_context,
                    permission.resource,
                    permission.action,
                    permission.tenant_id,
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if !has_permission {
                return Err(StatusCode::FORBIDDEN);
            }
        }

        // Continue to next middleware/handler
        Ok(next.run(request).await)
    }

    /// Middleware for tenant-specific authorization
    pub async fn authorize_tenant_request(
        State(middleware): State<Arc<RbacMiddleware>>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let auth_context = request
            .extensions()
            .get::<AuthContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?
            .clone();

        let tenant_requirement = request
            .extensions()
            .get::<TenantRequirement>()
            .cloned();

        if let Some(requirement) = tenant_requirement {
            // Verify tenant access
            let result = middleware
                .permission_checker
                .verify_tenant_access(
                    &auth_context,
                    requirement.tenant_id,
                    requirement.resource,
                    requirement.action,
                )
                .await;

            match result {
                Ok(()) => {} // Access granted
                Err(RbacError::NotTenantMember { .. }) => return Err(StatusCode::FORBIDDEN),
                Err(RbacError::PermissionDenied { .. }) => return Err(StatusCode::FORBIDDEN),
                Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }

        Ok(next.run(request).await)
    }

    /// Middleware for platform-only operations
    pub async fn authorize_platform_request(
        State(middleware): State<Arc<RbacMiddleware>>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let auth_context = request
            .extensions()
            .get::<AuthContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?
            .clone();

        // Check if user has platform access
        let has_platform_access = middleware
            .permission_checker
            .can_access_platform(&auth_context)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !has_platform_access {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(next.run(request).await)
    }

    /// Middleware for admin-only operations
    pub async fn authorize_admin_request(
        State(middleware): State<Arc<RbacMiddleware>>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let auth_context = request
            .extensions()
            .get::<AuthContext>()
            .ok_or(StatusCode::UNAUTHORIZED)?
            .clone();

        // Check if user is admin (platform or tenant)
        let is_admin = middleware
            .permission_checker
            .is_admin(&auth_context)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !is_admin {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(next.run(request).await)
    }
}

/// Required permission for a route
#[derive(Debug, Clone)]
pub struct RequiredPermission {
    pub resource: ResourceType,
    pub action: ActionType,
    pub tenant_id: Option<i32>,
}

impl RequiredPermission {
    pub fn new(resource: ResourceType, action: ActionType) -> Self {
        Self {
            resource,
            action,
            tenant_id: None,
        }
    }

    pub fn with_tenant(resource: ResourceType, action: ActionType, tenant_id: i32) -> Self {
        Self {
            resource,
            action,
            tenant_id: Some(tenant_id),
        }
    }
}

/// Tenant requirement for a route
#[derive(Debug, Clone)]
pub struct TenantRequirement {
    pub tenant_id: i32,
    pub resource: ResourceType,
    pub action: ActionType,
}

impl TenantRequirement {
    pub fn new(tenant_id: i32, resource: ResourceType, action: ActionType) -> Self {
        Self {
            tenant_id,
            resource,
            action,
        }
    }
}

/// Helper functions for extracting context from HTTP headers
pub struct HttpAuthExtractor;

impl HttpAuthExtractor {
    /// Extract user ID from Authorization header
    pub fn extract_user_id(headers: &HeaderMap) -> Option<i32> {
        // This would implement JWT token parsing or API key validation
        // For now, return None - implement based on your auth system
        headers
            .get("X-User-Id")
            .and_then(|value| value.to_str().ok())
            .and_then(|id_str| id_str.parse().ok())
    }

    /// Extract tenant ID from headers or path
    pub fn extract_tenant_id(headers: &HeaderMap, path: &str) -> Option<i32> {
        // Try header first
        if let Some(tenant_id) = headers
            .get("X-Tenant-Id")
            .and_then(|value| value.to_str().ok())
            .and_then(|id_str| id_str.parse().ok())
        {
            return Some(tenant_id);
        }

        // Try to extract from path like /api/v1/tenants/123/...
        if let Some(captures) = regex::Regex::new(r"/tenants/(\d+)/")
            .ok()
            .and_then(|re| re.captures(path))
        {
            if let Some(tenant_match) = captures.get(1) {
                if let Ok(tenant_id) = tenant_match.as_str().parse::<i32>() {
                    return Some(tenant_id);
                }
            }
        }

        None
    }

    /// Extract session ID from headers
    pub fn extract_session_id(headers: &HeaderMap) -> Option<String> {
        headers
            .get("X-Session-Id")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_string())
    }

    /// Extract API key ID from headers
    pub fn extract_api_key_id(headers: &HeaderMap) -> Option<i32> {
        headers
            .get("X-API-Key-Id")
            .and_then(|value| value.to_str().ok())
            .and_then(|id_str| id_str.parse().ok())
    }
}

/// Convenience macros for common authorization patterns
#[macro_export]
macro_rules! require_permission_middleware {
    ($resource:expr, $action:expr) => {
        |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(
                RequiredPermission::new($resource, $action)
            );
            next.run(req).await
        }
    };
    ($resource:expr, $action:expr, $tenant_id:expr) => {
        |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(
                RequiredPermission::with_tenant($resource, $action, $tenant_id)
            );
            next.run(req).await
        }
    };
}

#[macro_export]
macro_rules! require_tenant_access_middleware {
    ($tenant_id:expr, $resource:expr, $action:expr) => {
        |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(
                TenantRequirement::new($tenant_id, $resource, $action)
            );
            next.run(req).await
        }
    };
}

/// Response helpers for authorization errors
pub struct AuthResponse;

impl AuthResponse {
    /// Create unauthorized response
    pub fn unauthorized() -> Response<Body> {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"error":"Unauthorized","message":"Authentication required"}"#))
            .unwrap()
    }

    /// Create forbidden response
    pub fn forbidden() -> Response<Body> {
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"error":"Forbidden","message":"Insufficient permissions"}"#))
            .unwrap()
    }

    /// Create forbidden response with custom message
    pub fn forbidden_with_message(message: &str) -> Response<Body> {
        let body = format!(
            r#"{{"error":"Forbidden","message":"{}"}}"#,
            message.replace('"', "\\\"")
        );
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    /// Create tenant not found response
    pub fn tenant_not_found(tenant_id: i32) -> Response<Body> {
        let body = format!(
            r#"{{"error":"TenantNotFound","message":"Tenant {} not found or access denied"}}"#,
            tenant_id
        );
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }
}

/// Route protection helpers
pub struct RouteProtection;

impl RouteProtection {
    /// Protect task-related routes
    pub fn task_routes() -> Vec<(String, RequiredPermission)> {
        vec![
            ("GET /api/v1/tasks".to_string(), 
             RequiredPermission::new(ResourceType::Task, ActionType::Read)),
            ("POST /api/v1/tasks".to_string(), 
             RequiredPermission::new(ResourceType::Task, ActionType::Create)),
            ("PUT /api/v1/tasks/:id".to_string(), 
             RequiredPermission::new(ResourceType::Task, ActionType::Update)),
            ("DELETE /api/v1/tasks/:id".to_string(), 
             RequiredPermission::new(ResourceType::Task, ActionType::Delete)),
            ("POST /api/v1/tasks/:id/execute".to_string(), 
             RequiredPermission::new(ResourceType::Task, ActionType::Execute)),
        ]
    }

    /// Protect execution-related routes
    pub fn execution_routes() -> Vec<(String, RequiredPermission)> {
        vec![
            ("GET /api/v1/executions".to_string(), 
             RequiredPermission::new(ResourceType::Execution, ActionType::Read)),
            ("GET /api/v1/executions/:id".to_string(), 
             RequiredPermission::new(ResourceType::Execution, ActionType::Read)),
            ("POST /api/v1/executions/:id/cancel".to_string(), 
             RequiredPermission::new(ResourceType::Execution, ActionType::Update)),
        ]
    }

    /// Protect admin routes
    pub fn admin_routes() -> Vec<(String, RequiredPermission)> {
        vec![
            ("GET /api/v1/admin/users".to_string(), 
             RequiredPermission::new(ResourceType::User, ActionType::Read)),
            ("POST /api/v1/admin/users".to_string(), 
             RequiredPermission::new(ResourceType::User, ActionType::Create)),
            ("PUT /api/v1/admin/users/:id".to_string(), 
             RequiredPermission::new(ResourceType::User, ActionType::Update)),
            ("DELETE /api/v1/admin/users/:id".to_string(), 
             RequiredPermission::new(ResourceType::User, ActionType::Delete)),
            ("POST /api/v1/admin/users/:id/roles".to_string(), 
             RequiredPermission::new(ResourceType::User, ActionType::Manage)),
        ]
    }

    /// Protect tenant management routes
    pub fn tenant_routes() -> Vec<(String, RequiredPermission)> {
        vec![
            ("GET /api/v1/tenants".to_string(), 
             RequiredPermission::new(ResourceType::Tenant, ActionType::Read)),
            ("POST /api/v1/tenants".to_string(), 
             RequiredPermission::new(ResourceType::Tenant, ActionType::Create)),
            ("PUT /api/v1/tenants/:id".to_string(), 
             RequiredPermission::new(ResourceType::Tenant, ActionType::Update)),
            ("DELETE /api/v1/tenants/:id".to_string(), 
             RequiredPermission::new(ResourceType::Tenant, ActionType::Delete)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_permission_creation() {
        let perm = RequiredPermission::new(ResourceType::Task, ActionType::Read);
        assert!(matches!(perm.resource, ResourceType::Task));
        assert!(matches!(perm.action, ActionType::Read));
        assert!(perm.tenant_id.is_none());

        let perm_with_tenant = RequiredPermission::with_tenant(
            ResourceType::Task,
            ActionType::Create,
            100,
        );
        assert_eq!(perm_with_tenant.tenant_id, Some(100));
    }

    #[test]
    fn test_tenant_requirement_creation() {
        let req = TenantRequirement::new(100, ResourceType::Task, ActionType::Read);
        assert_eq!(req.tenant_id, 100);
        assert!(matches!(req.resource, ResourceType::Task));
        assert!(matches!(req.action, ActionType::Read));
    }

    #[test]
    fn test_http_auth_extractor() {
        let mut headers = HeaderMap::new();
        headers.insert("X-User-Id", "123".parse().unwrap());
        headers.insert("X-Tenant-Id", "456".parse().unwrap());
        headers.insert("X-Session-Id", "session_abc".parse().unwrap());

        assert_eq!(HttpAuthExtractor::extract_user_id(&headers), Some(123));
        assert_eq!(HttpAuthExtractor::extract_tenant_id(&headers, ""), Some(456));
        assert_eq!(
            HttpAuthExtractor::extract_session_id(&headers),
            Some("session_abc".to_string())
        );
    }

    #[test]
    fn test_tenant_id_extraction_from_path() {
        let headers = HeaderMap::new();
        
        // Test path extraction
        let path = "/api/v1/tenants/789/tasks";
        assert_eq!(
            HttpAuthExtractor::extract_tenant_id(&headers, path),
            Some(789)
        );

        let invalid_path = "/api/v1/tasks";
        assert_eq!(
            HttpAuthExtractor::extract_tenant_id(&headers, invalid_path),
            None
        );
    }

    #[test]
    fn test_route_protection_helpers() {
        let task_routes = RouteProtection::task_routes();
        assert!(!task_routes.is_empty());
        
        let admin_routes = RouteProtection::admin_routes();
        assert!(!admin_routes.is_empty());
        
        let tenant_routes = RouteProtection::tenant_routes();
        assert!(!tenant_routes.is_empty());
    }
}