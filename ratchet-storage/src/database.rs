//! Database module - compatibility layer for migration from ratchet-lib
//!
//! This module provides the same interface as ratchet-lib's database module
//! but implemented using the new seaorm structure for easier migration.

#[cfg(feature = "seaorm")]
pub use crate::seaorm::*;

// For migration compatibility, re-export everything with the expected paths
#[cfg(feature = "seaorm")]
pub mod entities {
    pub use crate::seaorm::entities::*;
}

#[cfg(feature = "seaorm")]
pub mod repositories {
    pub use crate::seaorm::repositories::*;
}

#[cfg(feature = "seaorm")]
pub mod migrations {
    pub use crate::seaorm::migrations::*;
}

#[cfg(feature = "seaorm")]
pub use crate::seaorm::{
    connection::DatabaseConnection, connection::DatabaseError, filters::validation, filters::SafeFilterBuilder,
};
