pub mod conversion;
pub mod error_handling;
pub mod execution;
pub mod http_integration;

// Re-export main functions for backward compatibility
pub use conversion::{convert_js_result_to_json, prepare_input_argument};
pub use error_handling::{parse_js_error, register_error_types};
pub use execution::{execute_js_file, execute_task, execute_task_with_context};
pub use http_integration::{check_fetch_call, handle_fetch_processing};

// Re-export the main call_js_function from execution
pub use execution::call_js_function;
