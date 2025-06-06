pub mod api;
pub mod config;
pub mod database;
pub mod errors;
pub mod execution;
pub mod generate;
pub mod graphql;
pub mod http;
pub mod js_executor;
pub mod js_task;
pub mod logging;
pub mod output;
pub mod recording;
pub mod registry;
pub mod rest;
pub mod server;
pub mod services;
pub mod task;
pub mod test;
pub mod types;

// #[cfg(test)]
// pub mod testing;

// Re-export commonly used types and functions for convenience
pub use config::{ConfigError, RatchetConfig};
pub use errors::{JsErrorType, JsExecutionError};
pub use graphql::{create_schema, RatchetSchema};
pub use js_executor::{execute_js_file, execute_task};
pub use rest::create_rest_app;
pub use server::{create_app, ServerState};
pub use services::{RatchetEngine, ServiceError, ServiceProvider, ServiceResult};
// Re-export validation functions from ratchet-core for compatibility
pub use ratchet_core::validation::{parse_schema, validate_json};

// Legacy function removed as part of code cleanup
