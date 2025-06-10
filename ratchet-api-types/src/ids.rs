use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "graphql")]
use async_graphql::scalar;

/// Unified ID type that works consistently across both APIs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiId(pub String);

impl ApiId {
    /// Create from database integer ID
    pub fn from_i32(id: i32) -> Self {
        Self(id.to_string())
    }

    /// Create from UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid.to_string())
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get as string (always available)
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Try to parse as integer (for database IDs)
    pub fn as_i32(&self) -> Option<i32> {
        self.0.parse().ok()
    }

    /// Try to parse as UUID
    pub fn as_uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.0).ok()
    }
}

impl std::fmt::Display for ApiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for ApiId {
    fn from(id: i32) -> Self {
        Self::from_i32(id)
    }
}

impl From<Uuid> for ApiId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

impl From<String> for ApiId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ApiId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// GraphQL scalar implementation (only when graphql feature is enabled)
#[cfg(feature = "graphql")]
scalar!(
    ApiId,
    "ApiId",
    "A unified ID that accepts both strings and numbers"
);