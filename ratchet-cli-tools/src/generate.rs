use anyhow::{Context, Result};
use serde_json::{json, Value as JsonValue};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};
use uuid::Uuid;

/// Configuration for generating a new task
#[derive(Debug, Clone)]
pub struct TaskGenerationConfig {
    pub path: PathBuf,
    pub label: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
}

impl TaskGenerationConfig {
    /// Create a new task generation configuration with required path
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            label: None,
            description: None,
            version: None,
        }
    }

    /// Set the task label
    pub fn with_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the task description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the task version
    pub fn with_version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = Some(version.into());
        self
    }
}

/// Information about a generated task
#[derive(Debug, Clone)]
pub struct GeneratedTaskInfo {
    pub path: PathBuf,
    pub uuid: Uuid,
    pub label: String,
    pub description: String,
    pub version: String,
    pub files_created: Vec<String>,
}

/// Generate a new task template with all required files
pub fn generate_task(config: TaskGenerationConfig) -> Result<GeneratedTaskInfo> {
    info!("Generating task template at: {:?}", config.path);

    // Check if directory already exists
    if config.path.exists() {
        return Err(anyhow::anyhow!("Directory already exists: {:?}", config.path));
    }

    // Create the task directory
    fs::create_dir_all(&config.path).context(format!("Failed to create task directory: {:?}", config.path))?;

    // Generate UUID for the task
    let task_uuid = Uuid::new_v4();

    // Use provided values or defaults
    let task_label = config.label.as_deref().unwrap_or("My Task");
    let task_description = config
        .description
        .as_deref()
        .unwrap_or("A task that performs a specific operation");
    let task_version = config.version.as_deref().unwrap_or("1.0.0");

    info!("Creating task files with UUID: {}", task_uuid);

    let mut files_created = Vec::new();

    // Create metadata.json
    let metadata = create_metadata_json(task_uuid, task_label, task_description, task_version)?;
    let metadata_path = config.path.join("metadata.json");
    write_json_file(&metadata_path, &metadata)?;
    files_created.push("metadata.json".to_string());

    // Create input.schema.json
    let input_schema = create_input_schema()?;
    let input_schema_path = config.path.join("input.schema.json");
    write_json_file(&input_schema_path, &input_schema)?;
    files_created.push("input.schema.json".to_string());

    // Create output.schema.json
    let output_schema = create_output_schema()?;
    let output_schema_path = config.path.join("output.schema.json");
    write_json_file(&output_schema_path, &output_schema)?;
    files_created.push("output.schema.json".to_string());

    // Create main.js
    let main_js_content = create_main_js_content();
    let main_js_path = config.path.join("main.js");
    fs::write(&main_js_path, main_js_content).context(format!("Failed to write main.js: {:?}", main_js_path))?;
    files_created.push("main.js".to_string());

    // Create tests directory with a sample test
    let tests_dir = config.path.join("tests");
    fs::create_dir_all(&tests_dir).context(format!("Failed to create tests directory: {:?}", tests_dir))?;

    // Create a sample test file
    let test_data = create_sample_test()?;
    let test_path = tests_dir.join("test-001.json");
    write_json_file(&test_path, &test_data)?;
    files_created.push("tests/test-001.json".to_string());

    info!("Task template generation completed successfully");

    Ok(GeneratedTaskInfo {
        path: config.path,
        uuid: task_uuid,
        label: task_label.to_string(),
        description: task_description.to_string(),
        version: task_version.to_string(),
        files_created,
    })
}

/// Create metadata.json content
fn create_metadata_json(uuid: Uuid, label: &str, description: &str, version: &str) -> Result<JsonValue> {
    debug!("Creating metadata.json with UUID: {}", uuid);
    Ok(json!({
        "uuid": uuid,
        "version": version,
        "label": label,
        "description": description
    }))
}

/// Create input.schema.json content
fn create_input_schema() -> Result<JsonValue> {
    debug!("Creating input.schema.json");
    Ok(json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "string",
                "description": "Input value for the task"
            }
        },
        "required": ["value"]
    }))
}

/// Create output.schema.json content
fn create_output_schema() -> Result<JsonValue> {
    debug!("Creating output.schema.json");
    Ok(json!({
        "type": "object",
        "properties": {
            "result": {
                "type": "string",
                "description": "Result of the task operation"
            }
        },
        "required": ["result"]
    }))
}

/// Create main.js content
fn create_main_js_content() -> String {
    debug!("Creating main.js content");
    r#"(function(input, context) {
    // Extract input parameters
    const { value } = input;
    
    // Access execution context if provided
    if (context) {
        console.log("Execution ID:", context.executionId);
        console.log("Task ID:", context.taskId);
        console.log("Task Version:", context.taskVersion);
        if (context.jobId) {
            console.log("Job ID:", context.jobId);
        }
    }
    
    // Validate input (schema validation happens automatically)
    if (!value || typeof value !== 'string') {
        throw new Error('Invalid input: value must be a string');
    }
    
    try {
        // Process the input
        const processedValue = `Processed: ${value}`;
        
        // Return the result matching the output schema
        return {
            result: processedValue
        };
    } catch (error) {
        // Handle any processing errors
        throw new Error(`Task processing failed: ${error.message}`);
    }
})
"#
    .to_string()
}

/// Create sample test content
fn create_sample_test() -> Result<JsonValue> {
    debug!("Creating sample test");
    Ok(json!({
        "input": {
            "value": "test input"
        },
        "expected_output": {
            "result": "Processed: test input"
        }
    }))
}

/// Write JSON content to a file with pretty formatting
fn write_json_file(path: &PathBuf, content: &JsonValue) -> Result<()> {
    debug!("Writing JSON file: {:?}", path);
    let json_content = serde_json::to_string_pretty(content)?;
    fs::write(path, json_content).context(format!("Failed to write JSON file: {:?}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_task_basic() {
        let temp_dir = tempdir().unwrap();
        let task_path = temp_dir.path().join("test-task");

        let config = TaskGenerationConfig::new(task_path.clone());
        let result = generate_task(config).unwrap();

        assert_eq!(result.path, task_path);
        assert_eq!(result.label, "My Task");
        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.files_created.len(), 5);

        // Check that files were created
        assert!(task_path.join("metadata.json").exists());
        assert!(task_path.join("input.schema.json").exists());
        assert!(task_path.join("output.schema.json").exists());
        assert!(task_path.join("main.js").exists());
        assert!(task_path.join("tests/test-001.json").exists());
    }

    #[test]
    fn test_generate_task_with_custom_metadata() {
        let temp_dir = tempdir().unwrap();
        let task_path = temp_dir.path().join("custom-task");

        let config = TaskGenerationConfig::new(task_path.clone())
            .with_label("Custom Task")
            .with_description("A custom task description")
            .with_version("2.5.0");

        let result = generate_task(config).unwrap();

        assert_eq!(result.label, "Custom Task");
        assert_eq!(result.description, "A custom task description");
        assert_eq!(result.version, "2.5.0");

        // Check metadata file content
        let metadata_content = fs::read_to_string(task_path.join("metadata.json")).unwrap();
        let metadata: JsonValue = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(metadata["label"], "Custom Task");
        assert_eq!(metadata["description"], "A custom task description");
        assert_eq!(metadata["version"], "2.5.0");
    }

    #[test]
    fn test_generate_task_existing_directory() {
        let temp_dir = tempdir().unwrap();
        let task_path = temp_dir.path().join("existing-task");

        // Create the directory first
        fs::create_dir_all(&task_path).unwrap();

        let config = TaskGenerationConfig::new(task_path);
        let result = generate_task(config);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Directory already exists"));
    }

    #[test]
    fn test_task_generation_config_builder() {
        let path = PathBuf::from("/test/path");
        let config = TaskGenerationConfig::new(path.clone())
            .with_label("Test Label")
            .with_description("Test Description")
            .with_version("1.2.3");

        assert_eq!(config.path, path);
        assert_eq!(config.label, Some("Test Label".to_string()));
        assert_eq!(config.description, Some("Test Description".to_string()));
        assert_eq!(config.version, Some("1.2.3".to_string()));
    }
}
