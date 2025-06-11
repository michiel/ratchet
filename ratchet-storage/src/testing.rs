//! Testing utilities for ratchet-storage
//!
//! This module provides comprehensive testing infrastructure for the ratchet-storage crate,
//! including database testing, mock objects, test builders, and file fixtures.

#[cfg(feature = "seaorm")]
pub mod builders;
#[cfg(feature = "seaorm")]
pub mod database;
pub mod fixtures;
#[cfg(feature = "seaorm")]
pub mod mocks;

// Re-export commonly used testing utilities
#[cfg(feature = "seaorm")]
pub use builders::*;
#[cfg(feature = "seaorm")]
pub use database::*;
pub use fixtures::*;
#[cfg(feature = "seaorm")]
pub use mocks::*;