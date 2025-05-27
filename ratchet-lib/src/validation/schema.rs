use crate::errors::JsExecutionError;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;

/// Validate JSON data against a schema
pub fn validate_json(data: &JsonValue, schema: &JsonValue) -> Result<(), JsExecutionError> {
    let compiled_schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|e| JsExecutionError::SchemaValidationError(e.to_string()))?;

    compiled_schema.validate(data).map_err(|errs| {
        let error_msgs: Vec<String> = errs.map(|e| e.to_string()).collect();
        JsExecutionError::SchemaValidationError(error_msgs.join(", "))
    })?;

    Ok(())
}

/// Parse a JSON schema from a file
pub fn parse_schema(schema_path: &Path) -> Result<JsonValue, JsExecutionError> {
    let schema_str =
        fs::read_to_string(schema_path).map_err(JsExecutionError::FileReadError)?;

    serde_json::from_str(&schema_str)
        .map_err(|e| JsExecutionError::InvalidInputSchema(e.to_string()))
}