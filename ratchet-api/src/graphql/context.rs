//! GraphQL context for dependency injection

#[cfg(feature = "auth")]
use crate::middleware::{AuthUser, ApiKeyUser};

/// GraphQL context containing shared application state and user information
#[derive(Clone)]
pub struct GraphQLContext {
    /// Authenticated user (JWT-based)
    #[cfg(feature = "auth")]
    pub user: Option<AuthUser>,
    
    /// Authenticated API key user
    #[cfg(feature = "auth")]
    pub api_user: Option<ApiKeyUser>,
    
    // TODO: Add service dependencies here
    // pub task_service: Arc<dyn TaskService>,
    // pub execution_service: Arc<dyn ExecutionService>,
    // pub job_service: Arc<dyn JobService>,
    // pub schedule_service: Arc<dyn ScheduleService>,
}

impl GraphQLContext {
    /// Create a new GraphQL context
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "auth")]
            user: None,
            #[cfg(feature = "auth")]
            api_user: None,
            // Initialize services here
        }
    }
    
    /// Create a context with JWT user authentication
    #[cfg(feature = "auth")]
    pub fn with_user(user: AuthUser) -> Self {
        Self {
            user: Some(user),
            api_user: None,
        }
    }
    
    /// Create a context with API key authentication
    #[cfg(feature = "auth")]
    pub fn with_api_user(api_user: ApiKeyUser) -> Self {
        Self {
            user: None,
            api_user: Some(api_user),
        }
    }
    
    /// Check if the context has any form of authentication
    #[cfg(feature = "auth")]
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some() || self.api_user.is_some()
    }
    
    /// Get the authenticated user ID (regardless of auth type)
    #[cfg(feature = "auth")]
    pub fn user_id(&self) -> Option<&str> {
        if let Some(ref user) = self.user {
            Some(&user.user_id)
        } else if let Some(ref api_user) = self.api_user {
            Some(&api_user.info.name)
        } else {
            None
        }
    }
    
    /// Check if the authenticated entity has a specific permission/role
    #[cfg(feature = "auth")]
    pub fn has_permission(&self, permission: &str) -> bool {
        if let Some(ref user) = self.user {
            user.has_role(permission)
        } else if let Some(ref api_user) = self.api_user {
            api_user.has_permission(permission)
        } else {
            false
        }
    }
    
    /// Require authentication, returning an error if not authenticated
    #[cfg(feature = "auth")]
    pub fn require_auth(&self) -> Result<(), async_graphql::Error> {
        if self.is_authenticated() {
            Ok(())
        } else {
            Err(async_graphql::Error::new("Authentication required"))
        }
    }
    
    /// Require a specific permission, returning an error if not authorized
    #[cfg(feature = "auth")]
    pub fn require_permission(&self, permission: &str) -> Result<(), async_graphql::Error> {
        self.require_auth()?;
        
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(async_graphql::Error::new(format!(
                "Permission '{}' required",
                permission
            )))
        }
    }
}

impl Default for GraphQLContext {
    fn default() -> Self {
        Self::new()
    }
}