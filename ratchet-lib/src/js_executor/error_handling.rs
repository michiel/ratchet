use crate::errors::{JsErrorType, JsExecutionError};
use boa_engine::{Context as BoaContext, Source};
use regex;

/// Configuration for JavaScript error types
#[derive(Debug, Clone)]
pub struct JsErrorConfig {
    pub name: &'static str,
    pub default_message: &'static str,
    pub has_status: bool,
}

/// Predefined JavaScript error types with their configurations
pub const JS_ERROR_CONFIGS: &[JsErrorConfig] = &[
    JsErrorConfig {
        name: "AuthenticationError",
        default_message: "Authentication failed",
        has_status: false,
    },
    JsErrorConfig {
        name: "AuthorizationError",
        default_message: "Authorization failed",
        has_status: false,
    },
    JsErrorConfig {
        name: "NetworkError",
        default_message: "Network error",
        has_status: false,
    },
    JsErrorConfig {
        name: "HttpError",
        default_message: "HTTP error",
        has_status: true,
    },
    JsErrorConfig {
        name: "ValidationError",
        default_message: "Validation error",
        has_status: false,
    },
    JsErrorConfig {
        name: "ConfigurationError",
        default_message: "Configuration error",
        has_status: false,
    },
    JsErrorConfig {
        name: "RateLimitError",
        default_message: "Rate limit exceeded",
        has_status: false,
    },
    JsErrorConfig {
        name: "ServiceUnavailableError",
        default_message: "Service unavailable",
        has_status: false,
    },
    JsErrorConfig {
        name: "TimeoutError",
        default_message: "Timeout error",
        has_status: false,
    },
    JsErrorConfig {
        name: "DataError",
        default_message: "Data error",
        has_status: false,
    },
];

/// Generate JavaScript error class definition for a single error type
pub fn generate_error_class(error_config: &JsErrorConfig) -> String {
    if error_config.has_status {
        // Special case for HttpError which takes status and message
        format!(
            r#"
        // {name}
        function {name}(status, message) {{
            this.name = "{name}";
            this.status = status;
            this.message = message || "{default_message}";
            this.stack = (new Error()).stack;
        }}
        {name}.prototype = Object.create(Error.prototype);
        {name}.prototype.constructor = {name};"#,
            name = error_config.name,
            default_message = error_config.default_message
        )
    } else {
        // Standard error type with just message
        format!(
            r#"
        // {name}
        function {name}(message) {{
            this.name = "{name}";
            this.message = message || "{default_message}";
            this.stack = (new Error()).stack;
        }}
        {name}.prototype = Object.create(Error.prototype);
        {name}.prototype.constructor = {name};"#,
            name = error_config.name,
            default_message = error_config.default_message
        )
    }
}

/// Generate all JavaScript error class definitions
pub fn generate_all_error_classes() -> String {
    JS_ERROR_CONFIGS
        .iter()
        .map(generate_error_class)
        .collect::<Vec<String>>()
        .join("\n")
}

/// Register custom error types in the JavaScript context
pub fn register_error_types(context: &mut BoaContext) -> Result<(), JsExecutionError> {
    let error_classes = generate_all_error_classes();

    context
        .eval(Source::from_bytes(&error_classes))
        .map_err(|e| {
            JsExecutionError::CompileError(format!("Failed to register error types: {}", e))
        })?;

    Ok(())
}

/// Parse JavaScript error and convert to JsErrorType
pub fn parse_js_error(error_message: &str) -> JsErrorType {
    // Try to extract error type and message from the error string
    if let Some(captures) = regex::Regex::new(r"(\w+Error): (.+)")
        .unwrap()
        .captures(error_message)
    {
        let error_type = &captures[1];
        let message = captures[2].to_string();

        match error_type {
            "AuthenticationError" => JsErrorType::AuthenticationError(message),
            "AuthorizationError" => JsErrorType::AuthorizationError(message),
            "NetworkError" => JsErrorType::NetworkError(message),
            "HttpError" => {
                // Try to extract status code from message
                if let Some(status_captures) =
                    regex::Regex::new(r"(\d+)").unwrap().captures(&message)
                {
                    if let Ok(status) = status_captures[1].parse::<u16>() {
                        return JsErrorType::HttpError { status, message };
                    }
                }
                JsErrorType::HttpError { status: 0, message }
            }
            "ValidationError" => JsErrorType::ValidationError(message),
            "ConfigurationError" => JsErrorType::ConfigurationError(message),
            "RateLimitError" => JsErrorType::RateLimitError(message),
            "ServiceUnavailableError" => JsErrorType::ServiceUnavailableError(message),
            "TimeoutError" => JsErrorType::TimeoutError(message),
            "DataError" => JsErrorType::DataError(message),
            _ => JsErrorType::UnknownError(message),
        }
    } else {
        JsErrorType::UnknownError(error_message.to_string())
    }
}
