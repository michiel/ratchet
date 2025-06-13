//! Input validation and error sanitization utilities
//!
//! This module provides comprehensive input validation and error sanitization
//! to prevent security vulnerabilities and information leakage.

pub mod input;
pub mod error_sanitization;

// Re-export commonly used types
pub use input::{InputValidator, ValidationError as InputValidationError};
pub use error_sanitization::{ErrorSanitizer, SanitizedError, ErrorSanitizationConfig};

// JSON schema validation utilities
//
// This module provides utilities for validating JSON data against JSON schemas,
// primarily used for task input/output validation.

use crate::error::{RatchetError, ValidationError};
use jsonschema::{Draft, JSONSchema};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, RatchetError>;

/// Validate JSON data against a schema
///
/// # Arguments
/// * `data` - The JSON data to validate
/// * `schema` - The JSON schema to validate against
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(RatchetError::Validation)` if validation fails
///
/// # Example
/// ```rust,ignore
/// use serde_json::json;
/// use ratchet_core::validation::validate_json;
///
/// let data = json!({"name": "test", "age": 25});
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "name": {"type": "string"},
///         "age": {"type": "number"}
///     },
///     "required": ["name"]
/// });
///
/// validate_json(&data, &schema)?;
/// ```
pub fn validate_json(data: &JsonValue, schema: &JsonValue) -> ValidationResult<()> {
    let compiled_schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|e| {
            RatchetError::Validation(ValidationError::SchemaValidation(format!(
                "Failed to compile schema: {}",
                e
            )))
        })?;

    compiled_schema.validate(data).map_err(|errs| {
        let error_msgs: Vec<String> = errs.map(|e| e.to_string()).collect();
        RatchetError::Validation(ValidationError::SchemaValidation(format!(
            "Schema validation failed: {}",
            error_msgs.join(", ")
        )))
    })?;

    Ok(())
}

/// Parse a JSON schema from a file
///
/// # Arguments
/// * `schema_path` - Path to the JSON schema file
///
/// # Returns
/// * `Ok(JsonValue)` containing the parsed schema
/// * `Err(RatchetError)` if file read or parsing fails
///
/// # Example
/// ```rust,ignore
/// use std::path::Path;
/// use ratchet_core::validation::parse_schema;
///
/// let schema = parse_schema(Path::new("task.schema.json"))?;
/// ```
pub fn parse_schema(schema_path: &Path) -> ValidationResult<JsonValue> {
    let schema_str = fs::read_to_string(schema_path).map_err(|e| {
        RatchetError::Validation(ValidationError::InvalidFormat(format!(
            "Failed to read schema file '{}': {}",
            schema_path.display(),
            e
        )))
    })?;

    serde_json::from_str(&schema_str).map_err(|e| {
        RatchetError::Validation(ValidationError::InvalidFormat(format!(
            "Failed to parse schema JSON from '{}': {}",
            schema_path.display(),
            e
        )))
    })
}

/// Validate JSON data against a schema loaded from file
///
/// This is a convenience function that combines `parse_schema` and `validate_json`.
///
/// # Arguments
/// * `data` - The JSON data to validate
/// * `schema_path` - Path to the JSON schema file
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(RatchetError)` if schema loading or validation fails
pub fn validate_json_with_schema_file(
    data: &JsonValue,
    schema_path: &Path,
) -> ValidationResult<()> {
    let schema = parse_schema(schema_path)?;
    validate_json(data, &schema)
}

/// Validate that a JSON value conforms to a basic type requirement
///
/// # Arguments
/// * `data` - The JSON data to validate
/// * `expected_type` - The expected JSON type ("object", "array", "string", "number", "boolean", "null")
///
/// # Returns
/// * `Ok(())` if the type matches
/// * `Err(RatchetError::Validation)` if the type doesn't match
pub fn validate_json_type(data: &JsonValue, expected_type: &str) -> ValidationResult<()> {
    let actual_type = match data {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    };

    if actual_type != expected_type {
        return Err(RatchetError::Validation(ValidationError::InvalidFormat(
            format!(
                "Expected JSON type '{}', but got '{}'",
                expected_type, actual_type
            ),
        )));
    }

    Ok(())
}

/// Check if a JSON object has all required fields
///
/// # Arguments
/// * `data` - The JSON object to check
/// * `required_fields` - List of field names that must be present
///
/// # Returns
/// * `Ok(())` if all required fields are present
/// * `Err(RatchetError::Validation)` if any required fields are missing
pub fn validate_required_fields(
    data: &JsonValue,
    required_fields: &[&str],
) -> ValidationResult<()> {
    let obj = data.as_object().ok_or_else(|| {
        RatchetError::Validation(ValidationError::InvalidFormat(
            "Expected JSON object for field validation".to_string(),
        ))
    })?;

    for field in required_fields {
        if !obj.contains_key(*field) {
            return Err(RatchetError::Validation(
                ValidationError::RequiredFieldMissing(field.to_string()),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_json_success() {
        let data = json!({"name": "test", "age": 25});
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        });

        assert!(validate_json(&data, &schema).is_ok());
    }

    #[test]
    fn test_validate_json_failure() {
        let data = json!({"age": "not a number"});
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        });

        let result = validate_json(&data, &schema);
        assert!(result.is_err());
        match result.unwrap_err() {
            RatchetError::Validation(ValidationError::SchemaValidation(_)) => {}
            _ => panic!("Expected ValidationError::SchemaValidation"),
        }
    }

    #[test]
    fn test_parse_schema_success() {
        let schema_content = json!({
            "type": "object",
            "properties": {
                "test": {"type": "string"}
            }
        });

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(
            temp_file.path(),
            serde_json::to_string(&schema_content).unwrap(),
        )
        .unwrap();

        let parsed = parse_schema(temp_file.path()).unwrap();
        assert_eq!(parsed, schema_content);
    }

    #[test]
    fn test_parse_schema_file_not_found() {
        let result = parse_schema(Path::new("nonexistent.json"));
        assert!(result.is_err());
        match result.unwrap_err() {
            RatchetError::Validation(ValidationError::InvalidFormat(_)) => {}
            _ => panic!("Expected ValidationError::InvalidFormat"),
        }
    }

    #[test]
    fn test_validate_json_type() {
        assert!(validate_json_type(&json!("test"), "string").is_ok());
        assert!(validate_json_type(&json!(42), "number").is_ok());
        assert!(validate_json_type(&json!(true), "boolean").is_ok());
        assert!(validate_json_type(&json!(null), "null").is_ok());
        assert!(validate_json_type(&json!({}), "object").is_ok());
        assert!(validate_json_type(&json!([]), "array").is_ok());

        // Test failures
        assert!(validate_json_type(&json!("test"), "number").is_err());
        assert!(validate_json_type(&json!(42), "string").is_err());
    }

    #[test]
    fn test_validate_required_fields() {
        let data = json!({"name": "test", "age": 25, "email": "test@example.com"});

        // All required fields present
        assert!(validate_required_fields(&data, &["name", "age"]).is_ok());

        // Missing required field
        let result = validate_required_fields(&data, &["name", "phone"]);
        assert!(result.is_err());
        match result.unwrap_err() {
            RatchetError::Validation(ValidationError::RequiredFieldMissing(field)) => {
                assert_eq!(field, "phone");
            }
            _ => panic!("Expected ValidationError::RequiredFieldMissing"),
        }

        // Non-object input
        let result = validate_required_fields(&json!("not an object"), &["name"]);
        assert!(result.is_err());
        match result.unwrap_err() {
            RatchetError::Validation(ValidationError::InvalidFormat(_)) => {}
            _ => panic!("Expected ValidationError::InvalidFormat"),
        }
    }
}
