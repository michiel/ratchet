//! Structured logging infrastructure for Ratchet
//!
//! This crate provides comprehensive logging capabilities including:
//! - Structured logging with enrichment
//! - Multiple output sinks (console, file, etc.)
//! - Error pattern matching and categorization
//! - LLM-optimized error reporting
//! - Distributed tracing context

pub mod config;
pub mod context;
pub mod enrichment;
pub mod error_info;
pub mod event;
pub mod init;
pub mod severity;

#[cfg(feature = "llm")]
pub mod llm_export;

pub mod logger;

#[cfg(feature = "patterns")]
pub mod patterns;

pub mod sinks;

// Re-export main types for convenience
pub use config::{ConfigError, LoggingConfig};
pub use context::LogContext;
pub use enrichment::{Enricher, LogEnricher};
pub use error_info::{ErrorInfo, ErrorSuggestions, RelatedError};
pub use event::{LogEvent, LogLevel};
pub use init::{init_hybrid_logging, init_logging_from_config, init_simple_tracing};
pub use logger::{LoggerBuilder, StructuredLogger};
pub use severity::ErrorSeverity;

#[cfg(feature = "llm")]
pub use llm_export::{format_markdown_report, LLMErrorReport, LLMExportConfig, LLMExporter};

#[cfg(feature = "patterns")]
pub use patterns::{ErrorCategory, ErrorPattern, ErrorPatternMatcher, MatchingRule};

use once_cell::sync::OnceCell;
use std::sync::Arc;

static GLOBAL_LOGGER: OnceCell<Arc<dyn StructuredLogger>> = OnceCell::new();

/// Initialize the global logger
pub fn init_logger(logger: Arc<dyn StructuredLogger>) -> Result<(), &'static str> {
    GLOBAL_LOGGER.set(logger).map_err(|_| "Logger already initialized")
}

/// Get the global logger
pub fn logger() -> Option<Arc<dyn StructuredLogger>> {
    GLOBAL_LOGGER.get().cloned()
}

/// Log an error with context
#[macro_export]
macro_rules! log_error {
    ($error:expr) => {
        if let Some(logger) = $crate::logger() {
            logger.log($error.to_log_event(&$crate::LogContext::current()));
        }
    };
    ($error:expr, $($key:expr => $value:expr),*) => {
        if let Some(logger) = $crate::logger() {
            let mut context = $crate::LogContext::current();
            $(
                context = context.with_field($key, $value);
            )*
            logger.log($error.to_log_event(&context));
        }
    };
}

/// Log a structured event
#[macro_export]
macro_rules! log_event {
    ($level:expr, $message:expr) => {
        if let Some(logger) = $crate::logger() {
            logger.log($crate::LogEvent::new($level, $message));
        }
    };
    ($level:expr, $message:expr, $($key:expr => $value:expr),*) => {
        if let Some(logger) = $crate::logger() {
            let mut event = $crate::LogEvent::new($level, $message);
            $(
                event = event.with_field($key, $value);
            )*
            logger.log(event);
        }
    };
}
