use crate::error::Result;
use crate::types::{TaskDefinition, ValidationResult};

pub struct TaskValidator {
    #[allow(dead_code)]
    strict_mode: bool,
}

impl Default for TaskValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskValidator {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub async fn validate(&self, task: &TaskDefinition) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate basic metadata
        self.validate_metadata(task, &mut result);

        // Validate script
        self.validate_script(task, &mut result);

        // Validate schemas if present
        if let Some(ref input_schema) = task.input_schema {
            self.validate_json_schema(input_schema, "input_schema", &mut result);
        }

        if let Some(ref output_schema) = task.output_schema {
            self.validate_json_schema(output_schema, "output_schema", &mut result);
        }

        Ok(result)
    }

    fn validate_metadata(&self, task: &TaskDefinition, result: &mut ValidationResult) {
        // Check required fields
        if task.metadata.name.is_empty() {
            result.add_error(
                "metadata.name".to_string(),
                "Task name cannot be empty".to_string(),
                "EMPTY_NAME".to_string(),
            );
        }

        if task.metadata.version.is_empty() {
            result.add_error(
                "metadata.version".to_string(),
                "Task version cannot be empty".to_string(),
                "EMPTY_VERSION".to_string(),
            );
        }

        // Validate version format (basic semver check)
        if !self.is_valid_version(&task.metadata.version) {
            result.add_warning(
                "metadata.version".to_string(),
                "Version should follow semantic versioning (e.g., 1.0.0)".to_string(),
                "INVALID_VERSION_FORMAT".to_string(),
            );
        }

        // Validate name format
        if !self.is_valid_name(&task.metadata.name) {
            result.add_error(
                "metadata.name".to_string(),
                "Task name should contain only alphanumeric characters, hyphens, and underscores".to_string(),
                "INVALID_NAME_FORMAT".to_string(),
            );
        }
    }

    fn validate_script(&self, task: &TaskDefinition, result: &mut ValidationResult) {
        if task.script.is_empty() {
            result.add_error(
                "script".to_string(),
                "Task script cannot be empty".to_string(),
                "EMPTY_SCRIPT".to_string(),
            );
            return;
        }

        // Basic JavaScript syntax validation (very basic)
        if !task.script.contains("function") && !task.script.contains("const") && !task.script.contains("=>") {
            result.add_warning(
                "script".to_string(),
                "Script doesn't appear to contain any function definitions".to_string(),
                "NO_FUNCTIONS".to_string(),
            );
        }

        // Check for export (common pattern)
        if !task.script.contains("export") && !task.script.contains("module.exports") {
            result.add_warning(
                "script".to_string(),
                "Script doesn't appear to export anything".to_string(),
                "NO_EXPORTS".to_string(),
            );
        }
    }

    #[cfg(feature = "validation")]
    fn validate_json_schema(&self, schema: &serde_json::Value, field: &str, result: &mut ValidationResult) {
        // Try to compile the schema to check if it's valid
        match jsonschema::validator_for(schema) {
            Ok(_) => {
                // Schema is valid
            }
            Err(e) => {
                result.add_error(
                    field.to_string(),
                    format!("Invalid JSON Schema: {}", e),
                    "INVALID_SCHEMA".to_string(),
                );
            }
        }
    }

    #[cfg(not(feature = "validation"))]
    fn validate_json_schema(&self, _schema: &serde_json::Value, _field: &str, _result: &mut ValidationResult) {
        // Schema validation is disabled
    }

    fn is_valid_version(&self, version: &str) -> bool {
        // Basic semver regex: X.Y.Z or X.Y.Z-suffix
        let semver_regex = regex::Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9\.\-]+)?$").unwrap();
        semver_regex.is_match(version)
    }

    fn is_valid_name(&self, name: &str) -> bool {
        // Allow alphanumeric, hyphens, underscores
        let name_regex = regex::Regex::new(r"^[a-zA-Z0-9\-_]+$").unwrap();
        name_regex.is_match(name) && !name.is_empty()
    }
}