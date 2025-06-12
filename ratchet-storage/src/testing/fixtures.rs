//! Test fixtures for file-based testing
//!
//! This module provides utilities for creating temporary files, directories,
//! and task structures for testing purposes.

#[cfg(feature = "testing")]
use std::collections::HashMap;
#[cfg(feature = "testing")]
use std::path::PathBuf;
#[cfg(feature = "testing")]
use tempfile::TempDir;
#[cfg(feature = "testing")]
use std::io::Write;
#[cfg(feature = "testing")]
use serde_yaml;

/// Test fixtures for file-based testing
#[cfg(feature = "testing")]
pub struct TestFixtures {
    temp_dir: TempDir,
    files: HashMap<String, PathBuf>,
}

#[cfg(feature = "testing")]
impl TestFixtures {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = TempDir::new()?;
        Ok(Self {
            temp_dir,
            files: HashMap::new(),
        })
    }

    /// Create a task directory with all necessary files
    pub fn create_task_directory(&mut self, task_name: &str) -> Result<PathBuf, std::io::Error> {
        let task_dir = self.temp_dir.path().join(task_name);
        std::fs::create_dir_all(&task_dir)?;

        // Create metadata.json
        let metadata = serde_json::json!({
            "name": task_name,
            "version": "1.0.0",
            "description": format!("Test task: {}", task_name),
            "tags": ["test"],
            "timeout": 30000
        });
        self.create_json_file(&task_dir.join("metadata.json"), &metadata)?;

        // Create input.schema.json
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string"
                }
            },
            "required": ["input"]
        });
        self.create_json_file(&task_dir.join("input.schema.json"), &input_schema)?;

        // Create output.schema.json
        let output_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "output": {
                    "type": "string"
                }
            },
            "required": ["output"]
        });
        self.create_json_file(&task_dir.join("output.schema.json"), &output_schema)?;

        // Create main.js
        let main_js = r#"
function main(input) {
    if (!input || !input.input) {
        throw new Error("Missing required input");
    }
    
    return {
        output: "Processed: " + input.input
    };
}

// Export for testing
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { main };
}
"#;
        self.create_text_file(&task_dir.join("main.js"), main_js)?;

        // Create tests directory
        let tests_dir = task_dir.join("tests");
        std::fs::create_dir_all(&tests_dir)?;

        // Create test cases
        self.create_test_case(&tests_dir, "test-001", 
            &serde_json::json!({"input": "hello"}),
            &serde_json::json!({"output": "Processed: hello"})
        )?;

        self.create_test_case(&tests_dir, "test-002", 
            &serde_json::json!({"input": "world"}),
            &serde_json::json!({"output": "Processed: world"})
        )?;

        // Create failing test case
        self.create_failing_test_case(&tests_dir, "test-003-fail",
            &serde_json::json!({}),
            "Missing required input"
        )?;

        self.files.insert(task_name.to_string(), task_dir.clone());
        Ok(task_dir)
    }

    /// Create a simple task with minimal files
    pub fn create_simple_task(&mut self, task_name: &str, js_code: &str) -> Result<PathBuf, std::io::Error> {
        let task_dir = self.temp_dir.path().join(task_name);
        std::fs::create_dir_all(&task_dir)?;

        // Minimal metadata
        let metadata = serde_json::json!({
            "name": task_name,
            "version": "1.0.0",
            "description": "Simple test task"
        });
        self.create_json_file(&task_dir.join("metadata.json"), &metadata)?;

        // Simple schemas
        let schema = serde_json::json!({"type": "object"});
        self.create_json_file(&task_dir.join("input.schema.json"), &schema)?;
        self.create_json_file(&task_dir.join("output.schema.json"), &schema)?;

        // Custom JavaScript code
        self.create_text_file(&task_dir.join("main.js"), js_code)?;

        self.files.insert(task_name.to_string(), task_dir.clone());
        Ok(task_dir)
    }

    /// Create a task with HTTP fetch functionality
    pub fn create_http_task(&mut self, task_name: &str, mock_responses: Option<&str>) -> Result<PathBuf, std::io::Error> {
        let js_code = if let Some(responses) = mock_responses {
            format!(r#"
function main(input) {{
    // Mock HTTP responses for testing
    const mockResponses = {};
    
    // Override fetch for testing
    globalThis.fetch = function(url) {{
        if (mockResponses[url]) {{
            return Promise.resolve({{
                ok: true,
                json: () => Promise.resolve(mockResponses[url])
            }});
        }}
        throw new Error("No mock response for: " + url);
    }};
    
    const url = input.url || "https://api.example.com/data";
    return fetch(url)
        .then(response => response.json())
        .then(data => ({{ result: data }}));
}}
"#, responses)
        } else {
            r#"
function main(input) {
    const url = input.url || "https://httpbin.org/json";
    return fetch(url)
        .then(response => response.json())
        .then(data => ({ result: data }));
}
"#.to_string()
        };

        self.create_simple_task(task_name, &js_code)
    }

    /// Create an invalid task (missing required files)
    pub fn create_invalid_task(&mut self, task_name: &str) -> Result<PathBuf, std::io::Error> {
        let task_dir = self.temp_dir.path().join(task_name);
        std::fs::create_dir_all(&task_dir)?;

        // Only create main.js, missing other required files
        let js_code = "function main(input) { return {output: 'invalid'}; }";
        self.create_text_file(&task_dir.join("main.js"), js_code)?;

        self.files.insert(task_name.to_string(), task_dir.clone());
        Ok(task_dir)
    }

    /// Create a test case file
    fn create_test_case(
        &self,
        tests_dir: &PathBuf,
        test_name: &str,
        input: &serde_json::Value,
        expected_output: &serde_json::Value,
    ) -> Result<(), std::io::Error> {
        let test_case = serde_json::json!({
            "input": input,
            "expected_output": expected_output,
            "should_succeed": true
        });
        
        let test_file = tests_dir.join(format!("{}.json", test_name));
        self.create_json_file(&test_file, &test_case)
    }

    /// Create a failing test case
    fn create_failing_test_case(
        &self,
        tests_dir: &PathBuf,
        test_name: &str,
        input: &serde_json::Value,
        expected_error: &str,
    ) -> Result<(), std::io::Error> {
        let test_case = serde_json::json!({
            "input": input,
            "expected_error": expected_error,
            "should_succeed": false
        });
        
        let test_file = tests_dir.join(format!("{}.json", test_name));
        self.create_json_file(&test_file, &test_case)
    }

    /// Create a JSON file
    fn create_json_file(&self, path: &PathBuf, content: &serde_json::Value) -> Result<(), std::io::Error> {
        let json_string = serde_json::to_string_pretty(content)?;
        self.create_text_file(path, &json_string)
    }

    /// Create a text file
    fn create_text_file(&self, path: &PathBuf, content: &str) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Get the path to a created task
    pub fn get_task_path(&self, task_name: &str) -> Option<&PathBuf> {
        self.files.get(task_name)
    }

    /// Get the temp directory path
    pub fn temp_dir(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create a configuration file
    #[cfg(feature = "testing")]
    pub fn create_config_file(&mut self, config_name: &str, config: &serde_json::Value) -> Result<PathBuf, std::io::Error> {
        let config_path = self.temp_dir.path().join(format!("{}.yaml", config_name));
        let yaml_string = serde_yaml::to_string(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.create_text_file(&config_path, &yaml_string)?;
        Ok(config_path)
    }

    /// Create a temporary registry with multiple tasks
    pub fn create_task_registry(&mut self, tasks: &[&str]) -> Result<PathBuf, std::io::Error> {
        let registry_dir = self.temp_dir.path().join("registry");
        std::fs::create_dir_all(&registry_dir)?;

        for task_name in tasks {
            let task_path = registry_dir.join(task_name);
            self.create_task_directory_at(&task_path, task_name)?;
        }

        Ok(registry_dir)
    }

    /// Create a task directory at a specific path
    fn create_task_directory_at(&self, task_path: &PathBuf, task_name: &str) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(task_path)?;

        let metadata = serde_json::json!({
            "name": task_name,
            "version": "1.0.0",
            "description": format!("Registry task: {}", task_name)
        });
        self.create_json_file(&task_path.join("metadata.json"), &metadata)?;

        let schema = serde_json::json!({"type": "object"});
        self.create_json_file(&task_path.join("input.schema.json"), &schema)?;
        self.create_json_file(&task_path.join("output.schema.json"), &schema)?;

        let js_code = format!(r#"
function main(input) {{
    return {{
        task_name: "{}",
        input_received: input,
        timestamp: new Date().toISOString()
    }};
}}
"#, task_name);
        self.create_text_file(&task_path.join("main.js"), &js_code)?;

        Ok(())
    }

    /// Create a database configuration file for testing
    #[cfg(feature = "testing")]
    pub fn create_database_config(&mut self, config_name: &str, db_type: &str) -> Result<PathBuf, std::io::Error> {
        let db_config = match db_type {
            "sqlite" => serde_json::json!({
                "database": {
                    "url": "sqlite://test.db",
                    "max_connections": 10,
                    "connection_timeout": 30
                }
            }),
            "memory" => serde_json::json!({
                "database": {
                    "url": "sqlite::memory:",
                    "max_connections": 1,
                    "connection_timeout": 5
                }
            }),
            _ => serde_json::json!({
                "database": {
                    "url": format!("{}://localhost/test", db_type),
                    "max_connections": 10,
                    "connection_timeout": 30
                }
            })
        };

        self.create_config_file(config_name, &db_config)
    }

    /// Create a server configuration file for testing
    #[cfg(feature = "testing")]
    pub fn create_server_config(&mut self, config_name: &str, port: u16) -> Result<PathBuf, std::io::Error> {
        let server_config = serde_json::json!({
            "server": {
                "host": "127.0.0.1",
                "port": port,
                "cors": {
                    "enabled": true,
                    "allowed_origins": ["*"]
                }
            },
            "logging": {
                "level": "debug",
                "format": "json"
            }
        });

        self.create_config_file(config_name, &server_config)
    }
}

/// Convenient fixture builder for common test scenarios
#[cfg(feature = "testing")]
pub struct FixtureBuilder {
    fixtures: TestFixtures,
}

#[cfg(feature = "testing")]
impl FixtureBuilder {
    pub fn new() -> Result<Self, std::io::Error> {
        Ok(Self {
            fixtures: TestFixtures::new()?,
        })
    }

    /// Add a simple addition task
    pub fn with_addition_task(mut self) -> Result<Self, std::io::Error> {
        let js_code = r#"
function main(input) {
    if (typeof input.a !== 'number' || typeof input.b !== 'number') {
        throw new Error("Both 'a' and 'b' must be numbers");
    }
    return { result: input.a + input.b };
}
"#;
        self.fixtures.create_simple_task("addition", js_code)?;
        Ok(self)
    }

    /// Add a task that makes HTTP requests
    pub fn with_http_task(mut self) -> Result<Self, std::io::Error> {
        self.fixtures.create_http_task("http-test", Some(r#"
{
    "https://api.example.com/data": {"message": "Hello from API"},
    "https://httpbin.org/json": {"slideshow": {"title": "Sample Slide Show"}}
}
"#))?;
        Ok(self)
    }

    /// Add a task that always fails
    pub fn with_failing_task(mut self) -> Result<Self, std::io::Error> {
        let js_code = r#"
function main(input) {
    throw new Error("This task always fails for testing purposes");
}
"#;
        self.fixtures.create_simple_task("failing-task", js_code)?;
        Ok(self)
    }

    /// Add an invalid task for error testing
    pub fn with_invalid_task(mut self) -> Result<Self, std::io::Error> {
        self.fixtures.create_invalid_task("invalid-task")?;
        Ok(self)
    }

    /// Add a complex task with validation
    pub fn with_validation_task(mut self) -> Result<Self, std::io::Error> {
        self.fixtures.create_task_directory("validation-task")?;
        Ok(self)
    }

    /// Add a database configuration
    pub fn with_database_config(mut self, db_type: &str) -> Result<Self, std::io::Error> {
        self.fixtures.create_database_config("test-db", db_type)?;
        Ok(self)
    }

    /// Add a server configuration
    pub fn with_server_config(mut self, port: u16) -> Result<Self, std::io::Error> {
        self.fixtures.create_server_config("test-server", port)?;
        Ok(self)
    }

    /// Build the fixtures
    pub fn build(self) -> TestFixtures {
        self.fixtures
    }
}

#[cfg(feature = "testing")]
impl Default for FixtureBuilder {
    fn default() -> Self {
        Self::new().expect("Failed to create fixture builder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_directory() {
        let mut fixtures = TestFixtures::new().unwrap();
        let task_path = fixtures.create_task_directory("test-task").unwrap();

        // Verify all required files exist
        assert!(task_path.join("metadata.json").exists());
        assert!(task_path.join("input.schema.json").exists());
        assert!(task_path.join("output.schema.json").exists());
        assert!(task_path.join("main.js").exists());
        assert!(task_path.join("tests").exists());

        // Verify test files exist
        assert!(task_path.join("tests/test-001.json").exists());
        assert!(task_path.join("tests/test-002.json").exists());
        assert!(task_path.join("tests/test-003-fail.json").exists());
    }

    #[test]
    fn test_create_simple_task() {
        let mut fixtures = TestFixtures::new().unwrap();
        let js_code = "function main(input) { return {output: input}; }";
        let task_path = fixtures.create_simple_task("simple", js_code).unwrap();

        assert!(task_path.join("metadata.json").exists());
        assert!(task_path.join("main.js").exists());

        // Verify JS content
        let js_content = std::fs::read_to_string(task_path.join("main.js")).unwrap();
        assert!(js_content.contains("function main"));
    }

    #[test]
    fn test_create_http_task() {
        let mut fixtures = TestFixtures::new().unwrap();
        let task_path = fixtures.create_http_task("http", None).unwrap();

        assert!(task_path.join("main.js").exists());

        let js_content = std::fs::read_to_string(task_path.join("main.js")).unwrap();
        assert!(js_content.contains("fetch"));
    }

    #[test]
    fn test_fixture_builder() {
        let fixtures = FixtureBuilder::new()
            .unwrap()
            .with_addition_task()
            .unwrap()
            .with_http_task()
            .unwrap()
            .with_failing_task()
            .unwrap()
            .build();

        assert!(fixtures.get_task_path("addition").is_some());
        assert!(fixtures.get_task_path("http-test").is_some());
        assert!(fixtures.get_task_path("failing-task").is_some());
    }

    #[test]
    fn test_create_task_registry() {
        let mut fixtures = TestFixtures::new().unwrap();
        let registry_path = fixtures.create_task_registry(&["task1", "task2", "task3"]).unwrap();

        assert!(registry_path.join("task1/metadata.json").exists());
        assert!(registry_path.join("task2/metadata.json").exists());
        assert!(registry_path.join("task3/metadata.json").exists());
    }

    #[test]
    fn test_create_config_file() {
        let mut fixtures = TestFixtures::new().unwrap();
        let config = serde_json::json!({
            "database": {
                "url": "sqlite://test.db"
            },
            "server": {
                "port": 8080
            }
        });

        let config_path = fixtures.create_config_file("test-config", &config).unwrap();
        assert!(config_path.exists());

        let content = std::fs::read_to_string(config_path).unwrap();
        assert!(content.contains("database"));
        assert!(content.contains("sqlite://test.db"));
    }

    #[test]
    fn test_database_config_creation() {
        let mut fixtures = TestFixtures::new().unwrap();
        
        let sqlite_config = fixtures.create_database_config("sqlite", "sqlite").unwrap();
        assert!(sqlite_config.exists());
        
        let memory_config = fixtures.create_database_config("memory", "memory").unwrap();
        assert!(memory_config.exists());
    }

    #[test]
    fn test_server_config_creation() {
        let mut fixtures = TestFixtures::new().unwrap();
        
        let server_config = fixtures.create_server_config("test-server", 3000).unwrap();
        assert!(server_config.exists());
        
        let content = std::fs::read_to_string(server_config).unwrap();
        assert!(content.contains("3000"));
        assert!(content.contains("127.0.0.1"));
    }
}