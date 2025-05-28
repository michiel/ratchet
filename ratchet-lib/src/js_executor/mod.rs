pub mod error_handling;
pub mod execution;
pub mod http_integration;
pub mod conversion;

// Re-export main functions for backward compatibility
pub use execution::{execute_task, execute_js_file};
pub use error_handling::{register_error_types, parse_js_error};
pub use conversion::{prepare_input_argument, convert_js_result_to_json};
pub use http_integration::{check_fetch_call, handle_fetch_processing};

// Re-export the main call_js_function from execution
pub use execution::call_js_function;