pub mod errors;
pub mod generate;
pub mod http;
pub mod js_executor;
pub mod js_task;
pub mod recording;
pub mod task;
pub mod test;
pub mod types;
pub mod validation;

// Re-export commonly used types and functions for convenience
pub use errors::{JsErrorType, JsExecutionError};
pub use js_executor::{execute_task, execute_js_file};
pub use validation::{validate_json, parse_schema};

/// Legacy addition function (kept for compatibility)
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}