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
pub mod recording;
pub mod registry;
pub mod rest;
pub mod server;
pub mod services;
pub mod task;
pub mod test;
pub mod types;
pub mod validation;

// #[cfg(test)]
// pub mod testing;

// Re-export commonly used types and functions for convenience
pub use config::{RatchetConfig, ConfigError};
pub use errors::{JsErrorType, JsExecutionError};
pub use graphql::{RatchetSchema, create_schema};
pub use js_executor::{execute_task, execute_js_file};
pub use rest::create_rest_app;
pub use server::{create_app, ServerState};
pub use services::{RatchetEngine, ServiceProvider, ServiceError, ServiceResult};
pub use validation::{validate_json, parse_schema};

// Legacy function removed as part of code cleanup