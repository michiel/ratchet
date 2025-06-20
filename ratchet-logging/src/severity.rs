//! Error severity levels for logging

use serde::{Deserialize, Serialize};

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum ErrorSeverity {
    Info,
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    pub fn should_alert(&self) -> bool {
        matches!(self, ErrorSeverity::High | ErrorSeverity::Critical)
    }

    pub fn should_retry(&self) -> bool {
        !matches!(self, ErrorSeverity::Critical)
    }
}
