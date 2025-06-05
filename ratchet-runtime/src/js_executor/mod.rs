//! JavaScript execution engine for runtime tasks

#[cfg(feature = "javascript")]
pub mod error_handling;
#[cfg(feature = "javascript")]
pub mod execution;
#[cfg(feature = "javascript")]
pub mod http_integration;
#[cfg(feature = "javascript")]
pub mod conversion;

#[cfg(feature = "javascript")]
pub use execution::{execute_task, execute_task_with_context, execute_js_file};
#[cfg(feature = "javascript")]
pub use error_handling::{register_error_types, parse_js_error};
#[cfg(feature = "javascript")]
pub use conversion::{prepare_input_argument, convert_js_result_to_json};
#[cfg(feature = "javascript")]
pub use http_integration::{check_fetch_call, handle_fetch_processing};
#[cfg(feature = "javascript")]
pub use execution::call_js_function;