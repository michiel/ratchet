//! RBAC (Role-Based Access Control) implementation for Ratchet using Casbin
//! 
//! This crate provides multi-tenant role-based access control with support for:
//! - Platform operators and tenant administrators  
//! - Flexible permission systems with custom roles
//! - Tenant isolation and resource scoping
//! - API and YAML configuration support

pub mod adapter;
pub mod auth;
pub mod config;
pub mod enforcer;
pub mod error;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod policies;
pub mod roles;
pub mod tenant;

pub use auth::AuthContext;
pub use config::RbacConfig;
pub use enforcer::RbacEnforcer;
pub use error::{RbacError, RbacResult};
pub use models::{Permission, Role, Tenant, TenantUser};
pub use permissions::PermissionChecker;
pub use tenant::TenantManager;

/// Re-export commonly used types
pub use casbin::{Enforcer, Result as CasbinResult};
pub use ratchet_api_types::enums::UserRole;