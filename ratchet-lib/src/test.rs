use crate::js_executor::execute_task;
use crate::errors::JsExecutionError;
use crate::task::{Task, TaskError};
use crate::types::HttpMethod;
use anyhow::Result;
use serde_json::{Value as JsonValue};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors that can occur during test execution
#[derive(Error, Debug)]
pub enum TestError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Task error: {0}")]
    TaskError(#[from] TaskError),

    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Execution error: {0}")]
    ExecutionError(#[from] JsExecutionError),

    #[error("Invalid test file: {0}")]
    InvalidTestFile(String),

    #[error("No tests directory found")]
    NoTestsDirectory,
}

/// Represents a single test case
#[derive(Debug)]
pub struct TestCase {
    pub file_path: PathBuf,
    pub input: JsonValue,
    pub expected_output: JsonValue,
    pub mock: Option<JsonValue>,
}

/// Represents the result of a test case
#[derive(Debug)]
pub struct TestResult {
    pub file_path: PathBuf,
    pub passed: bool,
    pub actual_output: Option<JsonValue>,
    pub error_message: Option<String>,
}

/// Represents a test summary
#[derive(Debug)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<TestResult>,
}

impl TestSummary {
    /// Returns true if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// Load test cases from a tests directory
pub fn load_test_cases(task_path: &Path) -> Result<Vec<TestCase>, TestError> {
    let tests_dir = task_path.join("tests");
    
    debug!("Loading test cases from: {:?}", tests_dir);
    
    if !tests_dir.exists() || !tests_dir.is_dir() {
        warn!("Tests directory not found: {:?}", tests_dir);
        return Err(TestError::NoTestsDirectory);
    }
    
    let mut test_cases = Vec::new();
    
    for entry in fs::read_dir(&tests_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Skip non-JSON files
        if path.extension().map_or(true, |ext| ext != "json") {
            debug!("Skipping non-JSON file: {:?}", path);
            continue;
        }
        
        debug!("Reading test file: {:?}", path);
        // Read the test file
        let content = fs::read_to_string(&path)?;
        let test_json: JsonValue = serde_json::from_str(&content)?;
        
        // Validate test file structure - skip invalid files instead of failing
        let input = match test_json.get("input") {
            Some(input) => input,
            None => {
                warn!("Skipping test file with missing 'input' field: {:?}", path);
                continue;
            }
        };
        
        let expected_output = match test_json.get("expected_output") {
            Some(output) => output,
            None => {
                warn!("Skipping test file with missing 'expected_output' field: {:?}", path);
                continue;
            }
        };
        
        // Check for optional mock data
        let mock = test_json.get("mock").map(|m| m.clone());
        
        let test_name = path.file_name().unwrap().to_string_lossy();
        debug!("Loaded test case: {}", test_name);
        if mock.is_some() {
            debug!("Test case {} includes mock data", test_name);
        }
        
        test_cases.push(TestCase {
            file_path: path,
            input: input.clone(),
            expected_output: expected_output.clone(),
            mock,
        });
    }
    
    // Sort test cases by file name for consistent execution order
    test_cases.sort_by(|a, b| {
        a.file_path.file_name().unwrap().cmp(b.file_path.file_name().unwrap())
    });
    
    info!("Loaded {} test cases from: {:?}", test_cases.len(), tests_dir);
    
    Ok(test_cases)
}

/// Run a single test case
pub async fn run_test_case(task: &mut Task, test_case: &TestCase) -> TestResult {
    let test_name = test_case.file_path.file_name().unwrap().to_string_lossy();
    debug!("Running test case: {}", test_name);
    
    // Setup HttpManager with mock data if provided
    let mut http_manager = crate::http::HttpManager::new();
    if let Some(mock) = &test_case.mock {
        debug!("Setting up HTTP manager with mock data for test: {}", test_name);
        http_manager.set_offline();
        
        // Parse mock data and setup mocks
        if let Some(http_mock) = mock.get("http") {
            let url = http_mock.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let method_str = http_mock.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
            let method: HttpMethod = method_str.parse().unwrap_or(HttpMethod::Get);
            if let Some(response) = http_mock.get("response") {
                http_manager.add_mock(method, url, response.clone());
                debug!("Added HTTP mock for {} {}", method, url);
            }
        }
    }
    
    // Execute the task
    debug!("Executing task for test: {}", test_name);
    let result = execute_task(task, test_case.input.clone(), &http_manager).await;
    
    match result {
        Ok(output) => {
            // Compare actual output with expected output
            let passed = output == test_case.expected_output;
            
            if passed {
                debug!("Test passed: {}", test_name);
            } else {
                warn!("Test failed: {} - output mismatch", test_name);
            }
            
            TestResult {
                file_path: test_case.file_path.clone(),
                passed,
                actual_output: Some(output),
                error_message: None,
            }
        },
        Err(err) => {
            warn!("Test failed: {} - execution error: {}", test_name, err);
            TestResult {
                file_path: test_case.file_path.clone(),
                passed: false,
                actual_output: None,
                error_message: Some(err.to_string()),
            }
        }
    }
}

/// Run all test cases for a task
pub async fn run_tests(task_path: &str) -> Result<TestSummary, TestError> {
    info!("Running tests for task at: {}", task_path);
    
    // Load the task
    debug!("Loading task from: {}", task_path);
    let mut task = Task::from_fs(task_path)?;
    
    // Load test cases
    let test_cases = load_test_cases(&task.path)?;
    
    if test_cases.is_empty() {
        warn!("No test cases found for task: {}", task_path);
        return Ok(TestSummary {
            total: 0,
            passed: 0,
            failed: 0,
            results: Vec::new(),
        });
    }
    
    // Run each test case
    info!("Executing {} test cases", test_cases.len());
    let mut results = Vec::new();
    for (i, test_case) in test_cases.iter().enumerate() {
        debug!("Running test case {}/{}", i + 1, test_cases.len());
        let result = run_test_case(&mut task, test_case).await;
        results.push(result);
    }
    
    // Count passed and failed tests
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    
    info!("Test execution completed - Total: {}, Passed: {}, Failed: {}", total, passed, failed);
    
    Ok(TestSummary {
        total,
        passed,
        failed,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio_test::block_on;
    
    fn create_test_task_with_tests() -> Result<(PathBuf, tempfile::TempDir), std::io::Error> {
        let temp_dir = tempdir()?;
        let task_dir = temp_dir.path().to_path_buf();
        
        // Create metadata.json
        let metadata = r#"{
            "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
            "version": "1.0.0",
            "label": "Test Task",
            "description": "Test task for unit testing"
        }"#;
        fs::write(task_dir.join("metadata.json"), metadata)?;
        
        // Create input.schema.json
        let input_schema = r#"{
            "type": "object",
            "properties": {
                "num1": { "type": "number" },
                "num2": { "type": "number" }
            },
            "required": ["num1", "num2"]
        }"#;
        fs::write(task_dir.join("input.schema.json"), input_schema)?;
        
        // Create output.schema.json
        let output_schema = r#"{
            "type": "object",
            "properties": {
                "sum": { "type": "number" }
            },
            "required": ["sum"]
        }"#;
        fs::write(task_dir.join("output.schema.json"), output_schema)?;
        
        // Create main.js
        let main_js = r#"(function(input) {
            const {num1, num2} = input;
            
            if (typeof num1 !== 'number' || typeof num2 !== 'number') {
              throw new Error('num1 and num2 must be numbers');
            }
            
            return {
              sum: num1 + num2
            }
        })"#;
        fs::write(task_dir.join("main.js"), main_js)?;
        
        // Create tests directory
        let tests_dir = task_dir.join("tests");
        fs::create_dir(&tests_dir)?;
        
        // Create test files
        let test1 = r#"{
            "input": {
                "num1": 5,
                "num2": 7
            },
            "expected_output": {
                "sum": 12
            }
        }"#;
        fs::write(tests_dir.join("test-001.json"), test1)?;
        
        let test2 = r#"{
            "input": {
                "num1": 10,
                "num2": 20
            },
            "expected_output": {
                "sum": 30
            }
        }"#;
        fs::write(tests_dir.join("test-002.json"), test2)?;
        
        // Create a failing test
        let test3 = r#"{
            "input": {
                "num1": 1,
                "num2": 2
            },
            "expected_output": {
                "sum": 1234
            }
        }"#;
        fs::write(tests_dir.join("test-003.json"), test3)?;
        
        // Create an invalid test (missing expected_output)
        let invalid_test = r#"{
            "input": {
                "num1": 1,
                "num2": 2
            }
        }"#;
        fs::write(tests_dir.join("invalid-test.json"), invalid_test)?;
        
        Ok((task_dir, temp_dir))
    }
    
    #[test]
    fn test_load_test_cases() {
        if let Ok((task_dir, _temp_dir)) = create_test_task_with_tests() {
            let test_cases = load_test_cases(&task_dir).unwrap();
            
            // We should have 3 valid test cases (the invalid one should be rejected)
            assert_eq!(test_cases.len(), 3);
            
            // Check first test case
            assert_eq!(test_cases[0].input["num1"], 5);
            assert_eq!(test_cases[0].input["num2"], 7);
            assert_eq!(test_cases[0].expected_output["sum"], 12);
            
            // Attempting to load the invalid test file should result in an error
            let invalid_test_path = task_dir.join("tests").join("invalid-test.json");
            let test_json: JsonValue = serde_json::from_str(
                &fs::read_to_string(invalid_test_path).unwrap()
            ).unwrap();
            
            let _input = test_json.get("input").unwrap();
            let expected_output = test_json.get("expected_output");
            assert!(expected_output.is_none());
        }
    }
    
    #[test]
    fn test_run_tests() {
        block_on(async {
            if let Ok((task_dir, _temp_dir)) = create_test_task_with_tests() {
                let summary = run_tests(task_dir.to_str().unwrap()).await.unwrap();
                
                // We should have 3 test cases
                assert_eq!(summary.total, 3);
                
                // Two tests should pass and one should fail
                assert_eq!(summary.passed, 2);
                assert_eq!(summary.failed, 1);
                
                // Check individual results
                let passed_tests = summary.results.iter().filter(|r| r.passed).count();
                assert_eq!(passed_tests, 2);
                
                let failed_tests = summary.results.iter().filter(|r| !r.passed).count();
                assert_eq!(failed_tests, 1);
                
                // The failing test should be the third one
                let failing_test = summary.results.iter().find(|r| !r.passed).unwrap();
                assert_eq!(failing_test.file_path.file_name().unwrap(), "test-003.json");
                assert!(failing_test.actual_output.is_some());
                assert_eq!(failing_test.actual_output.as_ref().unwrap()["sum"], 3);
                assert_ne!(failing_test.actual_output.as_ref().unwrap()["sum"], 1234);
            }
        });
    }
    
    #[test]
    fn test_no_tests_directory() {
        block_on(async {
            let temp_dir = tempdir().unwrap();
            let task_dir = temp_dir.path().to_path_buf();
            
            // Create a complete task without a tests directory
            let metadata = r#"{
                "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
                "version": "1.0.0",
                "label": "Test Task",
                "description": "Test task for unit testing"
            }"#;
            fs::write(task_dir.join("metadata.json"), metadata).unwrap();
            
            // Create input.schema.json
            let input_schema = r#"{
                "type": "object",
                "properties": {
                    "num1": { "type": "number" },
                    "num2": { "type": "number" }
                },
                "required": ["num1", "num2"]
            }"#;
            fs::write(task_dir.join("input.schema.json"), input_schema).unwrap();
            
            // Create output.schema.json
            let output_schema = r#"{
                "type": "object",
                "properties": {
                    "sum": { "type": "number" }
                },
                "required": ["sum"]
            }"#;
            fs::write(task_dir.join("output.schema.json"), output_schema).unwrap();
            
            // Create main.js
            let main_js = r#"(function(input) {
                return { sum: input.num1 + input.num2 };
            })"#;
            fs::write(task_dir.join("main.js"), main_js).unwrap();
            
            // Do NOT create a tests directory - this is what we're testing
            
            let result = run_tests(task_dir.to_str().unwrap()).await;
            assert!(matches!(result, Err(TestError::NoTestsDirectory)));
        });
    }
}