pub mod config;
pub mod context;
pub mod enrichment;
pub mod error_info;
pub mod event;
pub mod llm_export;
pub mod logger;
pub mod patterns;
pub mod sinks;

pub use config::{LoggingConfig, ConfigError};
pub use context::LogContext;
pub use enrichment::{Enricher, LogEnricher};
pub use error_info::{ErrorInfo, ErrorSuggestions, RelatedError};
pub use event::{LogEvent, LogLevel};
pub use llm_export::{LLMExporter, LLMExportConfig, LLMErrorReport, format_markdown_report};
pub use logger::{StructuredLogger, LoggerBuilder};
pub use patterns::{ErrorPattern, ErrorPatternMatcher, ErrorCategory, MatchingRule};

use std::sync::Arc;
use once_cell::sync::OnceCell;

static GLOBAL_LOGGER: OnceCell<Arc<dyn StructuredLogger>> = OnceCell::new();

/// Initialize the global logger
pub fn init_logger(logger: Arc<dyn StructuredLogger>) -> Result<(), &'static str> {
    GLOBAL_LOGGER.set(logger)
        .map_err(|_| "Logger already initialized")
}

/// Get the global logger
pub fn logger() -> Option<Arc<dyn StructuredLogger>> {
    GLOBAL_LOGGER.get().cloned()
}

/// Log an error with context
#[macro_export]
macro_rules! log_error {
    ($error:expr) => {
        if let Some(logger) = $crate::logging::logger() {
            logger.log($error.to_log_event(&$crate::logging::LogContext::current()));
        }
    };
    ($error:expr, $($key:expr => $value:expr),*) => {
        if let Some(logger) = $crate::logging::logger() {
            let mut context = $crate::logging::LogContext::current();
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
        if let Some(logger) = $crate::logging::logger() {
            logger.log($crate::logging::LogEvent::new($level, $message));
        }
    };
    ($level:expr, $message:expr, $($key:expr => $value:expr),*) => {
        if let Some(logger) = $crate::logging::logger() {
            let mut event = $crate::logging::LogEvent::new($level, $message);
            $(
                event = event.with_field($key, $value);
            )*
            logger.log(event);
        }
    };
}