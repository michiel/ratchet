//! Shared error handling middleware for all Ratchet APIs
//!
//! This crate provides unified error handling middleware and utilities
//! that can be used across GraphQL, REST, and MCP APIs to ensure
//! consistent error sanitization and formatting.

pub mod middleware;
pub mod traits;
pub mod graphql;
pub mod rest;
pub mod mcp;
pub mod sanitization;

// Re-export commonly used types
pub use traits::{ToSanitizedApiError, ErrorSanitizationProvider};
pub use sanitization::SharedErrorSanitizer;
pub use ratchet_api_types::errors::ApiError;

#[cfg(feature = "graphql")]
pub use graphql::GraphQLErrorExtensions;